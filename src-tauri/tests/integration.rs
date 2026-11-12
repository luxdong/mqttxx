//! 集成测试：
//! 1. PEM 三证书解析（目录由环境变量 `MQTTXX_CERT_DIR` 提供，未设置则跳过）
//! 2. 对 broker.emqx.io 的真实回环测试（v4 / v5 / TLS / WS / WSS）
//!
//! 回环测试依赖外网，网络不可用时会失败——这是有意的验收信号。

use std::sync::Arc;
use std::time::{Duration, Instant};

use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS, TlsConfiguration, Transport};

const HOST: &str = "broker.emqx.io";

fn unique_id(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::new_v4().simple())
}

// ---------------------------------------------------------------- TLS 配置构建

#[test]
fn tls_config_from_cert_dir() {
    // 证书目录只从环境变量读取，不在仓库中保存任何本地路径或证书内容
    let Ok(dir) = std::env::var("MQTTXX_CERT_DIR") else {
        eprintln!("skip: MQTTXX_CERT_DIR not set");
        return;
    };
    let dir = std::path::PathBuf::from(&dir);
    if !dir.exists() {
        eprintln!("skip: cert dir not found: {}", dir.display());
        return;
    }
    let ca = std::fs::read_to_string(dir.join("ca-cert.pem")).expect("read ca-cert.pem");
    let cert = std::fs::read_to_string(dir.join("client-cert.pem")).expect("read client-cert.pem");
    let key = std::fs::read_to_string(dir.join("client-key.pem")).expect("read client-key.pem");

    // 三证书双向认证
    let cfg = mqttxx_lib::mqtt_manager::build_tls_config(&ca, &cert, &key, false);
    assert!(cfg.is_ok(), "mutual tls config failed: {:?}", cfg.err());

    // 仅 CA
    assert!(
        mqttxx_lib::mqtt_manager::build_tls_config(&ca, "", "", false).is_ok(),
        "CA-only config failed"
    );

    // 无 CA → 系统根证书
    assert!(
        mqttxx_lib::mqtt_manager::build_tls_config("", "", "", false).is_ok(),
        "native-roots config failed"
    );

    // 跳过校验
    assert!(
        mqttxx_lib::mqtt_manager::build_tls_config("", "", "", true).is_ok(),
        "insecure config failed"
    );

    // 无效 CA 内容必须报错，而不是静默通过
    assert!(
        mqttxx_lib::mqtt_manager::build_tls_config("not a pem", "", "", false).is_err(),
        "invalid CA must be rejected"
    );

    // 证书/私钥必须成对出现
    assert!(
        mqttxx_lib::mqtt_manager::build_tls_config("", &cert, "", false).is_err(),
        "cert without key must be rejected"
    );
}

// ---------------------------------------------------------------- 真实回环

async fn v4_echo(mut opts: MqttOptions) -> Result<(), String> {
    let topic = unique_id("mqttxx-test");
    let payload = format!("hello-{}", topic);

    let (client, mut eventloop) = AsyncClient::new(opts, 64);
    client
        .subscribe(&topic, QoS::AtLeastOnce)
        .await
        .map_err(|e| e.to_string())?;

    let deadline = Instant::now() + Duration::from_secs(45);
    let mut published = false;
    loop {
        if Instant::now() > deadline {
            return Err("timeout waiting for echo".into());
        }
        let ev = tokio::time::timeout(Duration::from_secs(15), eventloop.poll())
            .await
            .map_err(|_| "poll timeout".to_string())?
            .map_err(|e| e.to_string())?;
        match ev {
            Event::Incoming(Incoming::ConnAck(_)) => {
                client
                    .publish(&topic, QoS::AtLeastOnce, false, payload.clone())
                    .await
                    .map_err(|e| e.to_string())?;
            }
            Event::Incoming(Incoming::SubAck(_)) => {
                if !published {
                    client
                        .publish(&topic, QoS::AtLeastOnce, false, payload.clone())
                        .await
                        .map_err(|e| e.to_string())?;
                    published = true;
                }
            }
            Event::Incoming(Incoming::Publish(p)) => {
                if p.topic == topic && p.payload.as_ref() == payload.as_bytes() {
                    client.disconnect().await.map_err(|e| e.to_string())?;
                    return Ok(());
                }
            }
            _ => {}
        }
    }
}

async fn v5_echo(mut opts: rumqttc::v5::MqttOptions) -> Result<(), String> {
    use rumqttc::v5::mqttbytes::QoS as Q5;

    let topic = unique_id("mqttxx-test");
    let payload = format!("hello-{}", topic);

    let (client, mut eventloop) = rumqttc::v5::AsyncClient::new(opts, 64);
    client
        .subscribe(&topic, Q5::AtLeastOnce)
        .await
        .map_err(|e| e.to_string())?;

    let deadline = Instant::now() + Duration::from_secs(45);
    let mut published = false;
    loop {
        if Instant::now() > deadline {
            return Err("timeout waiting for echo".into());
        }
        let ev = tokio::time::timeout(Duration::from_secs(15), eventloop.poll())
            .await
            .map_err(|_| "poll timeout".to_string())?
            .map_err(|e| e.to_string())?;
        match ev {
            rumqttc::v5::Event::Incoming(rumqttc::v5::Incoming::ConnAck(_)) => {
                if !published {
                    client
                        .publish(&topic, Q5::AtLeastOnce, false, payload.clone())
                        .await
                        .map_err(|e| e.to_string())?;
                    published = true;
                }
            }
            rumqttc::v5::Event::Incoming(rumqttc::v5::Incoming::Publish(p)) => {
                if p.topic == topic && p.payload.as_ref() == payload.as_bytes() {
                    client.disconnect().await.map_err(|e| e.to_string())?;
                    return Ok(());
                }
            }
            _ => {}
        }
    }
}

fn tls_config_native() -> TlsConfiguration {
    let config = mqttxx_lib::mqtt_manager::build_tls_config("", "", "", false)
        .expect("native tls config");
    TlsConfiguration::Rustls(Arc::new(config))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn echo_v4_tcp() {
    let mut opts = MqttOptions::new(unique_id("mqttxx-t"), HOST, 1883);
    opts.set_keep_alive(Duration::from_secs(30));
    v4_echo(opts).await.expect("v4 tcp echo");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn echo_v5_tcp() {
    let mut opts = rumqttc::v5::MqttOptions::new(unique_id("mqttxx-t"), HOST, 1883);
    opts.set_keep_alive(Duration::from_secs(30));
    v5_echo(opts).await.expect("v5 tcp echo");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn echo_v4_tls() {
    let mut opts = MqttOptions::new(unique_id("mqttxx-t"), HOST, 8883);
    opts.set_keep_alive(Duration::from_secs(30));
    opts.set_transport(Transport::tls_with_config(tls_config_native()));
    v4_echo(opts).await.expect("v4 tls echo");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn echo_v4_ws() {
    let mut opts = MqttOptions::new(unique_id("mqttxx-t"), format!("ws://{HOST}:8083/mqtt"), 8083);
    opts.set_keep_alive(Duration::from_secs(30));
    opts.set_transport(Transport::ws());
    v4_echo(opts).await.expect("v4 ws echo");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn echo_v4_wss() {
    let mut opts = MqttOptions::new(
        unique_id("mqttxx-t"),
        format!("wss://{HOST}:8084/mqtt"),
        8084,
    );
    opts.set_keep_alive(Duration::from_secs(30));
    opts.set_transport(Transport::wss_with_config(tls_config_native()));
    v4_echo(opts).await.expect("v4 wss echo");
}
