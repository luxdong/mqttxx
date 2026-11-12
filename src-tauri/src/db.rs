use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

pub struct Db(pub std::sync::Mutex<Connection>);

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------- schema

/// 最新建表语句（新库直接建到最新版并标记 user_version = 2）
const SCHEMA_LATEST: &str = r#"
CREATE TABLE IF NOT EXISTS connections (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  client_id TEXT NOT NULL DEFAULT '',
  host TEXT NOT NULL,
  port INTEGER NOT NULL DEFAULT 1883,
  username TEXT,
  password TEXT,
  protocol TEXT NOT NULL DEFAULT 'mqtt',
  ws_path TEXT NOT NULL DEFAULT '/mqtt',
  protocol_version INTEGER NOT NULL DEFAULT 4,
  ssl INTEGER NOT NULL DEFAULT 0,
  ssl_insecure INTEGER NOT NULL DEFAULT 0,
  ca TEXT NOT NULL DEFAULT '',
  client_cert TEXT NOT NULL DEFAULT '',
  client_key TEXT NOT NULL DEFAULT '',
  will_topic TEXT NOT NULL DEFAULT '',
  will_payload TEXT NOT NULL DEFAULT '',
  will_qos INTEGER NOT NULL DEFAULT 0,
  will_retain INTEGER NOT NULL DEFAULT 0,
  keepalive INTEGER NOT NULL DEFAULT 60,
  clean_session INTEGER NOT NULL DEFAULT 1,
  connect_timeout INTEGER NOT NULL DEFAULT 30,
  auto_resub INTEGER NOT NULL DEFAULT 1,
  description TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS subscriptions (
  id TEXT PRIMARY KEY,
  connection_id TEXT NOT NULL,
  topic TEXT NOT NULL,
  qos INTEGER NOT NULL DEFAULT 0,
  color TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_subscriptions_conn ON subscriptions(connection_id);
CREATE TABLE IF NOT EXISTS messages (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  connection_id TEXT NOT NULL,
  direction INTEGER NOT NULL,
  topic TEXT NOT NULL,
  qos INTEGER NOT NULL DEFAULT 0,
  retain INTEGER NOT NULL DEFAULT 0,
  payload TEXT NOT NULL DEFAULT '',
  payload_binary INTEGER NOT NULL DEFAULT 0,
  ts INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_messages_conn_ts ON messages(connection_id, ts DESC);
CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS publish_history (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  connection_id TEXT NOT NULL,
  topic TEXT NOT NULL,
  qos INTEGER NOT NULL DEFAULT 0,
  retain INTEGER NOT NULL DEFAULT 0,
  payload TEXT NOT NULL DEFAULT '',
  ts INTEGER NOT NULL,
  pinned INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_publish_history_conn ON publish_history(connection_id, ts DESC);
"#;

/// v1 → v2：补齐 protocol / ws_path / 双向证书 / 遗嘱消息列
const MIGRATE_V2: &str = r#"
ALTER TABLE connections ADD COLUMN protocol TEXT NOT NULL DEFAULT 'mqtt';
ALTER TABLE connections ADD COLUMN ws_path TEXT NOT NULL DEFAULT '/mqtt';
ALTER TABLE connections ADD COLUMN client_cert TEXT NOT NULL DEFAULT '';
ALTER TABLE connections ADD COLUMN client_key TEXT NOT NULL DEFAULT '';
ALTER TABLE connections ADD COLUMN will_topic TEXT NOT NULL DEFAULT '';
ALTER TABLE connections ADD COLUMN will_payload TEXT NOT NULL DEFAULT '';
ALTER TABLE connections ADD COLUMN will_qos INTEGER NOT NULL DEFAULT 0;
ALTER TABLE connections ADD COLUMN will_retain INTEGER NOT NULL DEFAULT 0;
"#;

/// v2 → v3：发布记录表（独立于 messages，清空消息不影响发布历史）
const MIGRATE_V3: &str = r#"
CREATE TABLE IF NOT EXISTS publish_history (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  connection_id TEXT NOT NULL,
  topic TEXT NOT NULL,
  qos INTEGER NOT NULL DEFAULT 0,
  retain INTEGER NOT NULL DEFAULT 0,
  payload TEXT NOT NULL DEFAULT '',
  ts INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_publish_history_conn ON publish_history(connection_id, ts DESC);
"#;

/// v3 → v4：收藏标记（收藏记录不参与容量淘汰）
const MIGRATE_V4: &str = r#"
ALTER TABLE publish_history ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
"#;

pub fn open(path: &std::path::Path) -> Result<Connection, rusqlite::Error> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<(), rusqlite::Error> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    match version {
        0 => {
            conn.execute_batch(SCHEMA_LATEST)?;
            conn.pragma_update(None, "user_version", 4)?;
        }
        1 => {
            conn.execute_batch(MIGRATE_V2)?;
            conn.execute_batch(MIGRATE_V3)?;
            conn.execute_batch(MIGRATE_V4)?;
            conn.pragma_update(None, "user_version", 4)?;
        }
        2 => {
            conn.execute_batch(MIGRATE_V3)?;
            conn.execute_batch(MIGRATE_V4)?;
            conn.pragma_update(None, "user_version", 4)?;
        }
        3 => {
            conn.execute_batch(MIGRATE_V4)?;
            conn.pragma_update(None, "user_version", 4)?;
        }
        _ => {}
    }
    Ok(())
}

// ---------------------------------------------------------------- models

/// 合法传输协议（与前端协议下拉一致）
pub const PROTOCOLS: [&str; 4] = ["mqtt", "mqtts", "ws", "wss"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfile {
    pub id: String,
    pub name: String,
    pub client_id: String,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    /// mqtt | mqtts | ws | wss
    pub protocol: String,
    /// WebSocket 路径（ws/wss 用），默认 /mqtt
    pub ws_path: String,
    /// 4 = MQTT 3.1.1, 5 = MQTT 5.0
    pub protocol_version: u8,
    /// 派生字段：protocol 是否为 TLS（mqtts/wss），落库时同步
    pub ssl: bool,
    pub ssl_insecure: bool,
    /// CA 证书 PEM 内容（可选，空则用系统根证书）
    pub ca: String,
    /// 客户端证书 PEM 内容（双向认证，可选）
    pub client_cert: String,
    /// 客户端私钥 PEM 内容（双向认证，可选）
    pub client_key: String,
    pub will_topic: String,
    pub will_payload: String,
    pub will_qos: u8,
    pub will_retain: bool,
    pub keepalive: u32,
    pub clean_session: bool,
    pub connect_timeout: u64,
    pub auto_resub: bool,
    pub description: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl ConnectionProfile {
    pub fn is_tls(&self) -> bool {
        matches!(self.protocol.as_str(), "mqtts" | "wss")
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInput {
    pub id: Option<String>,
    pub name: String,
    pub client_id: Option<String>,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub protocol: Option<String>,
    pub ws_path: Option<String>,
    pub protocol_version: Option<u8>,
    pub ssl_insecure: Option<bool>,
    pub ca: Option<String>,
    pub client_cert: Option<String>,
    pub client_key: Option<String>,
    pub will_topic: Option<String>,
    pub will_payload: Option<String>,
    pub will_qos: Option<u8>,
    pub will_retain: Option<bool>,
    pub keepalive: Option<u32>,
    pub clean_session: Option<bool>,
    pub connect_timeout: Option<u64>,
    pub auto_resub: Option<bool>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    pub id: String,
    pub connection_id: String,
    pub topic: String,
    pub qos: u8,
    pub color: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageRecord {
    pub id: i64,
    pub connection_id: String,
    /// 0 = received, 1 = published by this client
    pub direction: u8,
    pub topic: String,
    pub qos: u8,
    pub retain: bool,
    pub payload: String,
    pub payload_binary: bool,
    pub ts: i64,
}

/// 发布记录（每连接独立的发布历史，LRU 去重，容量受控；收藏不参与淘汰）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishRecord {
    pub id: i64,
    pub topic: String,
    pub qos: u8,
    pub retain: bool,
    pub payload: String,
    pub ts: i64,
    pub pinned: bool,
}

// ---------------------------------------------------------------- connections

const CONN_COLS: &str = "id, name, client_id, host, port, username, password, protocol, ws_path, \
protocol_version, ssl, ssl_insecure, ca, client_cert, client_key, will_topic, will_payload, will_qos, will_retain, \
keepalive, clean_session, connect_timeout, auto_resub, description, created_at, updated_at";

fn connection_from_row(row: &rusqlite::Row) -> rusqlite::Result<ConnectionProfile> {
    Ok(ConnectionProfile {
        id: row.get(0)?,
        name: row.get(1)?,
        client_id: row.get(2)?,
        host: row.get(3)?,
        port: row.get::<_, i64>(4)? as u16,
        username: row.get(5)?,
        password: row.get(6)?,
        protocol: row.get(7)?,
        ws_path: row.get(8)?,
        protocol_version: row.get::<_, i64>(9)? as u8,
        ssl: row.get::<_, i64>(10)? != 0,
        ssl_insecure: row.get::<_, i64>(11)? != 0,
        ca: row.get(12)?,
        client_cert: row.get(13)?,
        client_key: row.get(14)?,
        will_topic: row.get(15)?,
        will_payload: row.get(16)?,
        will_qos: row.get::<_, i64>(17)? as u8,
        will_retain: row.get::<_, i64>(18)? != 0,
        keepalive: row.get::<_, i64>(19)? as u32,
        clean_session: row.get::<_, i64>(20)? != 0,
        connect_timeout: row.get::<_, i64>(21)? as u64,
        auto_resub: row.get::<_, i64>(22)? != 0,
        description: row.get(23)?,
        created_at: row.get(24)?,
        updated_at: row.get(25)?,
    })
}

pub fn list_connections(conn: &Connection) -> Result<Vec<ConnectionProfile>, rusqlite::Error> {
    let mut stmt = conn.prepare(&format!("SELECT {CONN_COLS} FROM connections ORDER BY created_at DESC"))?;
    let rows = stmt.query_map([], connection_from_row)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn get_connection(conn: &Connection, id: &str) -> Result<Option<ConnectionProfile>, rusqlite::Error> {
    let mut stmt = conn.prepare(&format!("SELECT {CONN_COLS} FROM connections WHERE id = ?1"))?;
    let mut rows = stmt.query_map(params![id], connection_from_row)?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn gen_client_id() -> String {
    format!("mqttxx-{}", &uuid::Uuid::new_v4().simple().to_string()[..12])
}

pub fn save_connection(conn: &Connection, input: ConnectionInput) -> Result<ConnectionProfile, rusqlite::Error> {
    let now = now_ms();
    let protocol = input
        .protocol
        .filter(|p| PROTOCOLS.contains(&p.as_str()))
        .unwrap_or_else(|| "mqtt".to_string());
    let ssl = matches!(protocol.as_str(), "mqtts" | "wss");

    match input.id {
        Some(id) => {
            conn.execute(
                "UPDATE connections SET name=?2, client_id=?3, host=?4, port=?5, username=?6, password=?7, \
                 protocol=?8, ws_path=?9, protocol_version=?10, ssl=?11, ssl_insecure=?12, ca=?13, \
                 client_cert=?14, client_key=?15, will_topic=?16, will_payload=?17, will_qos=?18, will_retain=?19, \
                 keepalive=?20, clean_session=?21, connect_timeout=?22, auto_resub=?23, description=?24, updated_at=?25 \
                 WHERE id=?1",
                params![
                    id,
                    input.name,
                    input.client_id.unwrap_or_default(),
                    input.host,
                    input.port as i64,
                    input.username,
                    input.password,
                    protocol,
                    input.ws_path.unwrap_or_else(|| "/mqtt".into()),
                    input.protocol_version.unwrap_or(4) as i64,
                    ssl as i64,
                    input.ssl_insecure.unwrap_or(false) as i64,
                    input.ca.unwrap_or_default(),
                    input.client_cert.unwrap_or_default(),
                    input.client_key.unwrap_or_default(),
                    input.will_topic.unwrap_or_default(),
                    input.will_payload.unwrap_or_default(),
                    input.will_qos.unwrap_or(0) as i64,
                    input.will_retain.unwrap_or(false) as i64,
                    input.keepalive.unwrap_or(60) as i64,
                    input.clean_session.unwrap_or(true) as i64,
                    input.connect_timeout.unwrap_or(30) as i64,
                    input.auto_resub.unwrap_or(true) as i64,
                    input.description.unwrap_or_default(),
                    now,
                ],
            )?;
            get_connection(conn, &id)?
                .ok_or(rusqlite::Error::QueryReturnedNoRows)
        }
        None => {
            let id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO connections (id, name, client_id, host, port, username, password, protocol, ws_path, \
                 protocol_version, ssl, ssl_insecure, ca, client_cert, client_key, will_topic, will_payload, will_qos, will_retain, \
                 keepalive, clean_session, connect_timeout, auto_resub, description, created_at, updated_at) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26)",
                params![
                    id,
                    input.name,
                    input.client_id.unwrap_or_else(|| gen_client_id()),
                    input.host,
                    input.port as i64,
                    input.username,
                    input.password,
                    protocol,
                    input.ws_path.unwrap_or_else(|| "/mqtt".into()),
                    input.protocol_version.unwrap_or(4) as i64,
                    ssl as i64,
                    input.ssl_insecure.unwrap_or(false) as i64,
                    input.ca.unwrap_or_default(),
                    input.client_cert.unwrap_or_default(),
                    input.client_key.unwrap_or_default(),
                    input.will_topic.unwrap_or_default(),
                    input.will_payload.unwrap_or_default(),
                    input.will_qos.unwrap_or(0) as i64,
                    input.will_retain.unwrap_or(false) as i64,
                    input.keepalive.unwrap_or(60) as i64,
                    input.clean_session.unwrap_or(true) as i64,
                    input.connect_timeout.unwrap_or(30) as i64,
                    input.auto_resub.unwrap_or(true) as i64,
                    input.description.unwrap_or_default(),
                    now,
                    now,
                ],
            )?;
            Ok(get_connection(conn, &id)?.expect("row just inserted"))
        }
    }
}

pub fn delete_connection(conn: &Connection, id: &str) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM subscriptions WHERE connection_id = ?1", params![id])?;
    conn.execute("DELETE FROM messages WHERE connection_id = ?1", params![id])?;
    conn.execute("DELETE FROM publish_history WHERE connection_id = ?1", params![id])?;
    conn.execute("DELETE FROM connections WHERE id = ?1", params![id])?;
    Ok(())
}

// ---------------------------------------------------------------- subscriptions

pub fn list_subscriptions(conn: &Connection, connection_id: &str) -> Result<Vec<Subscription>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, connection_id, topic, qos, color, created_at FROM subscriptions WHERE connection_id = ?1 ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(params![connection_id], |row| {
        Ok(Subscription {
            id: row.get(0)?,
            connection_id: row.get(1)?,
            topic: row.get(2)?,
            qos: row.get::<_, i64>(3)? as u8,
            color: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn add_subscription(
    conn: &Connection,
    connection_id: &str,
    topic: &str,
    qos: u8,
    color: &str,
) -> Result<Subscription, rusqlite::Error> {
    let sub = Subscription {
        id: uuid::Uuid::new_v4().to_string(),
        connection_id: connection_id.to_string(),
        topic: topic.to_string(),
        qos,
        color: color.to_string(),
        created_at: now_ms(),
    };
    conn.execute(
        "INSERT INTO subscriptions (id, connection_id, topic, qos, color, created_at) VALUES (?1,?2,?3,?4,?5,?6)",
        params![sub.id, sub.connection_id, sub.topic, sub.qos as i64, sub.color, sub.created_at],
    )?;
    Ok(sub)
}

pub fn delete_subscription(conn: &Connection, id: &str) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM subscriptions WHERE id = ?1", params![id])?;
    Ok(())
}

// ---------------------------------------------------------------- messages

pub fn insert_messages(conn: &Connection, messages: &mut [MessageRecord]) -> Result<(), rusqlite::Error> {
    let tx = conn.unchecked_transaction()?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO messages (connection_id, direction, topic, qos, retain, payload, payload_binary, ts) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        )?;
        for m in messages.iter_mut() {
            stmt.execute(params![
                m.connection_id,
                m.direction as i64,
                m.topic,
                m.qos as i64,
                m.retain as i64,
                m.payload,
                m.payload_binary as i64,
                m.ts,
            ])?;
            m.id = tx.last_insert_rowid();
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn list_messages(
    conn: &Connection,
    connection_id: &str,
    before: Option<i64>,
    limit: i64,
) -> Result<Vec<MessageRecord>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, connection_id, direction, topic, qos, retain, payload, payload_binary, ts FROM messages \
         WHERE connection_id = ?1 AND (?2 IS NULL OR ts < ?2) ORDER BY ts DESC, id DESC LIMIT ?3",
    )?;
    let rows = stmt.query_map(params![connection_id, before, limit], |row| {
        Ok(MessageRecord {
            id: row.get(0)?,
            connection_id: row.get(1)?,
            direction: row.get::<_, i64>(2)? as u8,
            topic: row.get(3)?,
            qos: row.get::<_, i64>(4)? as u8,
            retain: row.get::<_, i64>(5)? != 0,
            payload: row.get(6)?,
            payload_binary: row.get::<_, i64>(7)? != 0,
            ts: row.get(8)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn clear_messages(conn: &Connection, connection_id: &str) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM messages WHERE connection_id = ?1", params![connection_id])?;
    Ok(())
}

// ---------------------------------------------------------------- publish history

/// 每连接发布历史上限（LRU 淘汰，收藏记录不参与淘汰）
const PUBLISH_HISTORY_MAX: i64 = 200;

/// 记录一次发布：同 (topic, qos, retain, payload) 命中则只刷新时间戳，否则插入；
/// 超过容量上限时删除最旧的未收藏记录。
pub fn insert_publish_history(
    conn: &Connection,
    connection_id: &str,
    topic: &str,
    qos: u8,
    retain: bool,
    payload: &str,
) -> Result<(), rusqlite::Error> {
    let now = now_ms();
    let matched: Option<i64> = conn
        .query_row(
            "SELECT id FROM publish_history WHERE connection_id = ?1 AND topic = ?2 AND qos = ?3 \
             AND retain = ?4 AND payload = ?5",
            params![connection_id, topic, qos as i64, retain as i64, payload],
            |r| r.get(0),
        )
        .ok();
    match matched {
        Some(id) => {
            conn.execute(
                "UPDATE publish_history SET ts = ?2 WHERE id = ?1",
                params![id, now],
            )?;
        }
        None => {
            conn.execute(
                "INSERT INTO publish_history (connection_id, topic, qos, retain, payload, ts) \
                 VALUES (?1,?2,?3,?4,?5,?6)",
                params![connection_id, topic, qos as i64, retain as i64, payload, now],
            )?;
        }
    }
    // 容量淘汰：删除超出上限的最旧未收藏记录
    conn.execute(
        "DELETE FROM publish_history WHERE connection_id = ?1 AND pinned = 0 AND id NOT IN (\
           SELECT id FROM publish_history WHERE connection_id = ?1 AND pinned = 0 \
           ORDER BY ts DESC, id DESC LIMIT ?2\
         )",
        params![connection_id, PUBLISH_HISTORY_MAX],
    )?;
    Ok(())
}

pub fn set_publish_history_pinned(conn: &Connection, id: i64, pinned: bool) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE publish_history SET pinned = ?2 WHERE id = ?1",
        params![id, pinned as i64],
    )?;
    Ok(())
}

pub fn list_publish_history(
    conn: &Connection,
    connection_id: &str,
    limit: i64,
) -> Result<Vec<PublishRecord>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, topic, qos, retain, payload, ts, pinned FROM publish_history \
         WHERE connection_id = ?1 ORDER BY ts DESC, id DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![connection_id, limit], |row| {
        Ok(PublishRecord {
            id: row.get(0)?,
            topic: row.get(1)?,
            qos: row.get::<_, i64>(2)? as u8,
            retain: row.get::<_, i64>(3)? != 0,
            payload: row.get(4)?,
            ts: row.get(5)?,
            pinned: row.get::<_, i64>(6)? != 0,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn clear_publish_history(conn: &Connection, connection_id: &str) -> Result<(), rusqlite::Error> {
    // 清空历史保留收藏
    conn.execute(
        "DELETE FROM publish_history WHERE connection_id = ?1 AND pinned = 0",
        params![connection_id],
    )?;
    Ok(())
}
