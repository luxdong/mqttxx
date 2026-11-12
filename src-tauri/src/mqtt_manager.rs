//! MQTT 连接管理器
//!
//! 每条连接由一个 tokio 任务承载 rumqttc EventLoop：
//! - 命令（publish/subscribe/unsubscribe/disconnect）经 mpsc 通道下发
//! - 收到的消息进入缓冲区，攒批（>=FLUSH_AT 条或 150ms）后事务写入 SQLite 并向前端推送事件
//! - 连接状态变化以 `conn-status` 事件广播给前端
//! - 网络错误时 rumqttc 自带重连，这里限制"从未连上过"的失败次数避免无限重试
//!
//! 传输协议对齐 MQTTX：mqtt / mqtts / ws / wss
//! - mqtts/wss：rustls，支持 CA + 双向认证（客户端证书/私钥）、跳过校验、系统根证书兜底
//! - ws/wss：rumqttc 要求 broker_addr 传完整 ws:// 或 wss:// URL（含路径）

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rumqttc::{
    AsyncClient, Event, EventLoop, Incoming, LastWill, MqttOptions, QoS, TlsConfiguration, Transport,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;

use crate::db::{self, ConnectionProfile, Db, MessageRecord, Subscription};

const EVENT_STATUS: &str = "conn-status";
const EVENT_MESSAGES: &str = "messages";
/// 单批消息达到该条数立即落库/推送
const FLUSH_AT: usize = 100;
const MAX_PACKET_SIZE: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnStatusEvent {
    pub id: String,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageBatchEvent {
    pub connection_id: String,
    pub items: Vec<MessageRecord>,
}

pub enum ConnCmd {
    Publish {
        topic: String,
        qos: u8,
        retain: bool,
        payload: Vec<u8>,
    },
    Subscribe {
        topic: String,
        qos: u8,
    },
    Unsubscribe {
        topic: String,
    },
    Disconnect,
}

pub struct MqttManager {
    app: AppHandle,
    conns: Mutex<HashMap<String, mpsc::Sender<ConnCmd>>>,
}

impl MqttManager {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            conns: Mutex::new(HashMap::new()),
        }
    }

    pub async fn connect(&self, profile: ConnectionProfile, subs: Vec<Subscription>) -> Result<(), String> {
        // 连接重入：先优雅断开旧连接并等待其退出（对齐 MQTTX 重新连接语义）
        let existing = self.conns.lock().unwrap().get(&profile.id).cloned();
        if let Some(tx) = existing {
            let _ = tx.send(ConnCmd::Disconnect).await;
            let deadline = std::time::Instant::now() + Duration::from_secs(3);
            while self.is_running(&profile.id) {
                if std::time::Instant::now() > deadline {
                    return Err("previous connection is still shutting down".into());
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }

        self.emit_status(&profile.id, "connecting", None);
        let pair = match build_client(&profile) {
            Ok(p) => p,
            Err(e) => {
                self.emit_status(&profile.id, "error", Some(e.clone()));
                return Err(e);
            }
        };
        let (tx, rx) = mpsc::channel::<ConnCmd>(256);
        self.conns.lock().unwrap().insert(profile.id.clone(), tx);
        let app = self.app.clone();
        let id = profile.id.clone();
        tokio::spawn(async move {
            run_connection(app, id, profile, subs, pair, rx).await;
        });
        Ok(())
    }

    pub async fn send(&self, id: &str, cmd: ConnCmd) -> Result<(), String> {
        let tx = {
            let conns = self.conns.lock().unwrap();
            conns
                .get(id)
                .cloned()
                .ok_or_else(|| "connection is not running".to_string())?
        };
        tx.send(cmd).await.map_err(|_| "connection has stopped".to_string())
    }

    pub fn is_running(&self, id: &str) -> bool {
        self.conns.lock().unwrap().contains_key(id)
    }

    fn remove_handle(&self, id: &str) {
        self.conns.lock().unwrap().remove(id);
    }

    fn emit_status(&self, id: &str, status: &str, error: Option<String>) {
        let _ = self.app.emit(
            EVENT_STATUS,
            ConnStatusEvent {
                id: id.into(),
                status: status.into(),
                error,
            },
        );
    }
}

// ---------------------------------------------------------------- tls

/// 构建 rustls 客户端配置（供管理器与集成测试共用）：
/// - `ca_pem` 非空：用它做服务端校验；为空：回退系统根证书
/// - `cert_pem`/`key_pem` 非空：启用双向认证（对齐阿里云等需要三证书的场景）
/// - `insecure`：跳过服务端证书校验（自签名场景）
pub fn build_tls_config(
    ca_pem: &str,
    cert_pem: &str,
    key_pem: &str,
    insecure: bool,
) -> Result<rustls::ClientConfig, String> {
    let mut roots = rustls::RootCertStore::empty();
    if !ca_pem.trim().is_empty() {
        let mut reader = std::io::BufReader::new(ca_pem.as_bytes());
        for cert in rustls_pemfile::certs(&mut reader) {
            let cert = cert.map_err(|e| format!("parse CA certificate: {e}"))?;
            roots
                .add(cert)
                .map_err(|e| format!("add CA certificate: {e}"))?;
        }
        if roots.is_empty() {
            return Err("CA certificate file contains no certificate".into());
        }
    } else {
        // 未提供 CA：使用操作系统根证书
        let native = rustls_native_certs::load_native_certs()
            .map_err(|e| format!("load system certificates: {e}"))?;
        for cert in native {
            let _ = roots.add(cert);
        }
        if roots.is_empty() {
            return Err("no system certificates available; provide a CA certificate".into());
        }
    }

    if cert_pem.trim().is_empty() != key_pem.trim().is_empty() {
        return Err("client certificate and private key must be provided together".into());
    }

    let builder = rustls::ClientConfig::builder().with_root_certificates(roots);
    let mut config = if !cert_pem.trim().is_empty() && !key_pem.trim().is_empty() {
        let mut cert_reader = std::io::BufReader::new(cert_pem.as_bytes());
        let certs: Vec<_> = rustls_pemfile::certs(&mut cert_reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("parse client certificate: {e}"))?;
        if certs.is_empty() {
            return Err("client certificate file contains no certificate".into());
        }
        let mut key_reader = std::io::BufReader::new(key_pem.as_bytes());
        let key = rustls_pemfile::private_key(&mut key_reader)
            .map_err(|e| format!("parse client private key: {e}"))?
            .ok_or_else(|| "client key file contains no private key".to_string())?;
        builder
            .with_client_auth_cert(certs, key)
            .map_err(|e| format!("invalid client certificate/key pair: {e}"))?
    } else {
        builder.with_no_client_auth()
    };

    if insecure {
        config
            .dangerous()
            .set_certificate_verifier(Arc::new(NoVerify));
    }
    Ok(config)
}

// ---------------------------------------------------------------- client

enum ClientPair {
    V4(AsyncClient, EventLoop),
    V5(rumqttc::v5::AsyncClient, rumqttc::v5::EventLoop),
}

fn qos4(q: u8) -> QoS {
    match q {
        1 => QoS::AtLeastOnce,
        2 => QoS::ExactlyOnce,
        _ => QoS::AtMostOnce,
    }
}

fn qos5(q: u8) -> rumqttc::v5::mqttbytes::QoS {
    match q {
        1 => rumqttc::v5::mqttbytes::QoS::AtLeastOnce,
        2 => rumqttc::v5::mqttbytes::QoS::ExactlyOnce,
        _ => rumqttc::v5::mqttbytes::QoS::AtMostOnce,
    }
}

fn qos_u8(q: rumqttc::v5::mqttbytes::QoS) -> u8 {
    match q {
        rumqttc::v5::mqttbytes::QoS::AtLeastOnce => 1,
        rumqttc::v5::mqttbytes::QoS::ExactlyOnce => 2,
        _ => 0,
    }
}

fn tls_configuration(profile: &ConnectionProfile) -> Result<TlsConfiguration, String> {
    let config = build_tls_config(
        &profile.ca,
        &profile.client_cert,
        &profile.client_key,
        profile.ssl_insecure,
    )?;
    Ok(TlsConfiguration::Rustls(Arc::new(config)))
}

fn ws_broker_addr(profile: &ConnectionProfile) -> String {
    let scheme = if profile.protocol == "wss" { "wss" } else { "ws" };
    let mut path = profile.ws_path.trim().to_string();
    if path.is_empty() {
        path = "/mqtt".into();
    }
    if !path.starts_with('/') {
        path = format!("/{path}");
    }
    format!("{}://{}:{}{}", scheme, profile.host, profile.port, path)
}

fn build_client(profile: &ConnectionProfile) -> Result<ClientPair, String> {
    let transport = match profile.protocol.as_str() {
        "mqtts" => Some(Transport::tls_with_config(tls_configuration(profile)?)),
        "wss" => Some(Transport::wss_with_config(tls_configuration(profile)?)),
        "ws" => Some(Transport::ws()),
        _ => None, // mqtt: tcp
    };

    let creds = profile
        .username
        .clone()
        .map(|u| (u, profile.password.clone().unwrap_or_default()));

    // ws/wss 时 rumqttc 要求 broker_addr 为完整 URL（域名与端口从 URL 解析）
    let is_ws = matches!(profile.protocol.as_str(), "ws" | "wss");
    let addr = if is_ws {
        ws_broker_addr(profile)
    } else {
        profile.host.clone()
    };

    let will = (!profile.will_topic.trim().is_empty()).then(|| {
        (
            profile.will_topic.trim().to_string(),
            profile.will_payload.as_bytes().to_vec(),
            profile.will_qos,
            profile.will_retain,
        )
    });

    if profile.protocol_version == 5 {
        let mut opts = rumqttc::v5::MqttOptions::new(&profile.client_id, addr, profile.port);
        opts.set_keep_alive(Duration::from_secs(profile.keepalive as u64));
        opts.set_clean_start(profile.clean_session);
        opts.set_connection_timeout(profile.connect_timeout);
        opts.set_max_packet_size(Some(MAX_PACKET_SIZE as u32));
        if let Some((u, p)) = creds {
            opts.set_credentials(u, p);
        }
        if let Some((topic, message, qos, retain)) = will {
            opts.set_last_will(rumqttc::v5::mqttbytes::v5::LastWill {
                topic: topic.into_bytes().into(),
                message: message.into(),
                qos: qos5(qos),
                retain,
                properties: None,
            });
        }
        if let Some(t) = transport {
            opts.set_transport(t);
        }
        let (client, eventloop) = rumqttc::v5::AsyncClient::new(opts, 256);
        Ok(ClientPair::V5(client, eventloop))
    } else {
        let mut opts = MqttOptions::new(&profile.client_id, addr, profile.port);
        opts.set_keep_alive(Duration::from_secs(profile.keepalive as u64));
        opts.set_clean_session(profile.clean_session);
        opts.set_max_packet_size(MAX_PACKET_SIZE, MAX_PACKET_SIZE);
        if let Some((u, p)) = creds {
            opts.set_credentials(u, p);
        }
        if let Some((topic, message, qos, retain)) = will {
            opts.set_last_will(LastWill {
                topic,
                message: message.into(),
                qos: qos4(qos),
                retain,
            });
        }
        if let Some(t) = transport {
            opts.set_transport(t);
        }
        let (client, mut eventloop) = AsyncClient::new(opts, 256);
        // v4 的连接超时位于 EventLoop 的 NetworkOptions
        let mut network = rumqttc::NetworkOptions::new();
        network.set_connection_timeout(profile.connect_timeout);
        eventloop.set_network_options(network);
        Ok(ClientPair::V4(client, eventloop))
    }
}

// ---------------------------------------------------------------- event loop

enum NetEvent {
    Connected,
    Publish {
        topic: String,
        qos: u8,
        retain: bool,
        payload: Vec<u8>,
    },
    BrokerDisconnect,
}

async fn poll_pair(pair: &mut ClientPair) -> Result<Option<NetEvent>, String> {
    match pair {
        ClientPair::V4(_, el) => match el.poll().await {
            Ok(Event::Incoming(incoming)) => Ok(v4_incoming(incoming)),
            Ok(_) => Ok(None),
            Err(e) => Err(e.to_string()),
        },
        ClientPair::V5(_, el) => match el.poll().await {
            Ok(rumqttc::v5::Event::Incoming(incoming)) => Ok(v5_incoming(incoming)),
            Ok(_) => Ok(None),
            Err(e) => Err(e.to_string()),
        },
    }
}

fn v4_incoming(incoming: Incoming) -> Option<NetEvent> {
    match incoming {
        Incoming::ConnAck(_) => Some(NetEvent::Connected),
        Incoming::Publish(p) => Some(NetEvent::Publish {
            topic: p.topic,
            qos: match p.qos {
                QoS::AtLeastOnce => 1,
                QoS::ExactlyOnce => 2,
                _ => 0,
            },
            retain: p.retain,
            payload: p.payload.to_vec(),
        }),
        Incoming::Disconnect => Some(NetEvent::BrokerDisconnect),
        _ => None,
    }
}

fn v5_incoming(incoming: rumqttc::v5::Incoming) -> Option<NetEvent> {
    match incoming {
        rumqttc::v5::Incoming::ConnAck(_) => Some(NetEvent::Connected),
        rumqttc::v5::Incoming::Publish(p) => Some(NetEvent::Publish {
            topic: String::from_utf8_lossy(&p.topic).to_string(),
            qos: qos_u8(p.qos),
            retain: p.retain,
            payload: p.payload.to_vec(),
        }),
        rumqttc::v5::Incoming::Disconnect(_) => Some(NetEvent::BrokerDisconnect),
        _ => None,
    }
}

async fn send_publish(
    pair: &mut ClientPair,
    topic: String,
    qos: u8,
    retain: bool,
    payload: Vec<u8>,
) -> Result<(), String> {
    match pair {
        ClientPair::V4(c, _) => {
            c.publish(topic, qos4(qos), retain, payload)
                .await
                .map_err(|e| e.to_string())
        }
        ClientPair::V5(c, _) => {
            c.publish(topic, qos5(qos), retain, payload)
                .await
                .map_err(|e| e.to_string())
        }
    }
}

async fn send_subscribe(pair: &mut ClientPair, topic: &str, qos: u8) -> Result<(), String> {
    match pair {
        ClientPair::V4(c, _) => c.subscribe(topic, qos4(qos)).await.map_err(|e| e.to_string()),
        ClientPair::V5(c, _) => c.subscribe(topic, qos5(qos)).await.map_err(|e| e.to_string()),
    }
}

async fn send_unsubscribe(pair: &mut ClientPair, topic: &str) -> Result<(), String> {
    match pair {
        ClientPair::V4(c, _) => c.unsubscribe(topic).await.map_err(|e| e.to_string()),
        ClientPair::V5(c, _) => c.unsubscribe(topic).await.map_err(|e| e.to_string()),
    }
}

async fn send_disconnect(pair: &mut ClientPair) -> Result<(), String> {
    match pair {
        ClientPair::V4(c, _) => c.disconnect().await.map_err(|e| e.to_string()),
        ClientPair::V5(c, _) => c.disconnect().await.map_err(|e| e.to_string()),
    }
}

fn decode_payload(bytes: &[u8]) -> (String, bool) {
    match std::str::from_utf8(bytes) {
        Ok(s) => (s.to_string(), false),
        Err(_) => (BASE64.encode(bytes), true),
    }
}

fn flush_buffer(app: &AppHandle, id: &str, buffer: &mut Vec<MessageRecord>) {
    if buffer.is_empty() {
        return;
    }
    let state = app.state::<Db>();
    let conn = state.0.lock().unwrap();
    let result = db::insert_messages(&conn, buffer);
    drop(conn);
    match result {
        Ok(()) => {
            let batch = MessageBatchEvent {
                connection_id: id.to_string(),
                items: std::mem::take(buffer),
            };
            let _ = app.emit(EVENT_MESSAGES, batch);
        }
        Err(e) => {
            eprintln!("[mqtt:{id}] failed to persist messages: {e}");
            buffer.clear();
        }
    }
}

async fn run_connection(
    app: AppHandle,
    id: String,
    profile: ConnectionProfile,
    subs: Vec<Subscription>,
    pair: ClientPair,
    mut rx: mpsc::Receiver<ConnCmd>,
) {
    let mgr = app.state::<MqttManager>();
    let mut pair = pair;
    let mut buffer: Vec<MessageRecord> = Vec::new();
    let mut flush = tokio::time::interval(Duration::from_millis(150));
    flush.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut err_streak = 0u32;
    let mut ever_connected = false;
    let mut terminal_status = "disconnected";

    loop {
        tokio::select! {
            ev = poll_pair(&mut pair) => match ev {
                Ok(Some(NetEvent::Connected)) => {
                    err_streak = 0;
                    mgr.emit_status(&id, "connected", None);
                    if profile.auto_resub {
                        for s in &subs {
                            if let Err(e) = send_subscribe(&mut pair, &s.topic, s.qos).await {
                                eprintln!("[mqtt:{id}] resubscribe {} failed: {e}", s.topic);
                            }
                        }
                    }
                    ever_connected = true;
                }
                Ok(Some(NetEvent::Publish { topic, qos, retain, payload })) => {
                    let (text, binary) = decode_payload(&payload);
                    buffer.push(MessageRecord {
                        id: 0,
                        connection_id: id.clone(),
                        direction: 0,
                        topic,
                        qos,
                        retain,
                        payload: text,
                        payload_binary: binary,
                        ts: db::now_ms(),
                    });
                    if buffer.len() >= FLUSH_AT {
                        flush_buffer(&app, &id, &mut buffer);
                    }
                }
                Ok(Some(NetEvent::BrokerDisconnect)) => {
                    mgr.emit_status(&id, "reconnecting", Some("broker disconnected".into()));
                }
                Ok(None) => {}
                Err(e) => {
                    err_streak += 1;
                    if err_streak == 1 || err_streak % 20 == 0 {
                        mgr.emit_status(&id, "reconnecting", Some(e.clone()));
                    }
                    if err_streak >= 30 && !ever_connected {
                        terminal_status = "error";
                        mgr.emit_status(&id, "error", Some(format!("connection failed: {e}")));
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            },
            cmd = rx.recv() => {
                let Some(cmd) = cmd else { break };
                match cmd {
                    ConnCmd::Publish { topic, qos, retain, payload } => {
                        let (text, binary) = decode_payload(&payload);
                        buffer.push(MessageRecord {
                            id: 0,
                            connection_id: id.clone(),
                            direction: 1,
                            topic: topic.clone(),
                            qos,
                            retain,
                            payload: text,
                            payload_binary: binary,
                            ts: db::now_ms(),
                        });
                        if let Err(e) = send_publish(&mut pair, topic, qos, retain, payload).await {
                            mgr.emit_status(&id, "error", Some(format!("publish failed: {e}")));
                        }
                        if buffer.len() >= FLUSH_AT {
                            flush_buffer(&app, &id, &mut buffer);
                        }
                    }
                    ConnCmd::Subscribe { topic, qos } => {
                        if let Err(e) = send_subscribe(&mut pair, &topic, qos).await {
                            mgr.emit_status(&id, "error", Some(format!("subscribe failed: {e}")));
                        }
                    }
                    ConnCmd::Unsubscribe { topic } => {
                        if let Err(e) = send_unsubscribe(&mut pair, &topic).await {
                            mgr.emit_status(&id, "error", Some(format!("unsubscribe failed: {e}")));
                        }
                    }
                    ConnCmd::Disconnect => {
                        let _ = send_disconnect(&mut pair).await;
                        break;
                    }
                }
            }
            _ = flush.tick() => {
                if !buffer.is_empty() {
                    flush_buffer(&app, &id, &mut buffer);
                }
            }
        }
    }

    flush_buffer(&app, &id, &mut buffer);
    mgr.remove_handle(&id);
    mgr.emit_status(&id, terminal_status, None);
}

// ---------------------------------------------------------------- tls insecure verifier

#[derive(Debug)]
struct NoVerify;

impl rustls::client::danger::ServerCertVerifier for NoVerify {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
        ]
    }
}
