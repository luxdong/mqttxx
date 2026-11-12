//! Tauri 命令层：前端 `invoke()` 的全部入口

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::{
    self, ConnectionInput, ConnectionProfile, Db, MessageRecord, PublishRecord, Subscription,
};
use crate::mqtt_manager::{ConnCmd, MqttManager};
use crate::settings::{self, AppSettings, SettingsPatch};

fn lock_db<'a>(db: &'a State<'a, Db>) -> std::sync::MutexGuard<'a, rusqlite::Connection> {
    db.0.lock().unwrap()
}

// ---------------------------------------------------------------- connections

#[tauri::command]
pub fn connection_list(db: State<Db>) -> Result<Vec<ConnectionProfile>, String> {
    db::list_connections(&lock_db(&db)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn connection_save(db: State<Db>, input: ConnectionInput) -> Result<ConnectionProfile, String> {
    db::save_connection(&lock_db(&db), input).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn connection_delete(
    db: State<'_, Db>,
    mgr: State<'_, MqttManager>,
    id: String,
) -> Result<(), String> {
    if mgr.is_running(&id) {
        let _ = mgr.send(&id, ConnCmd::Disconnect).await;
    }
    db::delete_connection(&lock_db(&db), &id).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- subscriptions

#[tauri::command]
pub fn subscription_list(db: State<Db>, connection_id: String) -> Result<Vec<Subscription>, String> {
    db::list_subscriptions(&lock_db(&db), &connection_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn subscription_add(
    db: State<Db>,
    connection_id: String,
    topic: String,
    qos: u8,
    color: String,
) -> Result<Subscription, String> {
    if topic.trim().is_empty() {
        return Err("topic is required".into());
    }
    db::add_subscription(&lock_db(&db), &connection_id, topic.trim(), qos, &color)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn subscription_delete(db: State<Db>, id: String) -> Result<(), String> {
    db::delete_subscription(&lock_db(&db), &id).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- runtime

#[tauri::command]
pub async fn connect(
    db: State<'_, Db>,
    mgr: State<'_, MqttManager>,
    id: String,
) -> Result<(), String> {
    let (profile, subs) = {
        let conn = lock_db(&db);
        let profile = db::get_connection(&conn, &id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "connection not found".to_string())?;
        let subs = db::list_subscriptions(&conn, &id).map_err(|e| e.to_string())?;
        (profile, subs)
    };
    mgr.connect(profile, subs).await
}

#[tauri::command]
pub async fn disconnect(mgr: State<'_, MqttManager>, id: String) -> Result<(), String> {
    if !mgr.is_running(&id) {
        return Ok(());
    }
    mgr.send(&id, ConnCmd::Disconnect).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishInput {
    pub connection_id: String,
    pub topic: String,
    pub qos: u8,
    pub retain: bool,
    pub payload: String,
}

/// 已连接状态下在线添加订阅（同时由调用方负责持久化）
#[tauri::command]
pub async fn subscribe_live(
    mgr: State<'_, MqttManager>,
    connection_id: String,
    topic: String,
    qos: u8,
) -> Result<(), String> {
    mgr.send(&connection_id, ConnCmd::Subscribe { topic, qos }).await
}

/// 已连接状态下在线退订
#[tauri::command]
pub async fn unsubscribe_live(
    mgr: State<'_, MqttManager>,
    connection_id: String,
    topic: String,
) -> Result<(), String> {
    mgr.send(&connection_id, ConnCmd::Unsubscribe { topic }).await
}

#[tauri::command]
pub async fn publish(db: State<'_, Db>, mgr: State<'_, MqttManager>, input: PublishInput) -> Result<(), String> {
    if input.topic.trim().is_empty() {
        return Err("topic is required".into());
    }
    let topic = input.topic.trim().to_string();
    mgr.send(
        &input.connection_id,
        ConnCmd::Publish {
            topic: topic.clone(),
            qos: input.qos,
            retain: input.retain,
            payload: input.payload.clone().into_bytes(),
        },
    )
    .await?;
    // 记录发布历史（LRU 去重，独立于消息表）
    db::insert_publish_history(&lock_db(&db), &input.connection_id, &topic, input.qos, input.retain, &input.payload)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn publish_history_list(
    db: State<Db>,
    connection_id: String,
    limit: Option<i64>,
) -> Result<Vec<PublishRecord>, String> {
    db::list_publish_history(&lock_db(&db), &connection_id, limit.unwrap_or(50))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn publish_history_set_pinned(db: State<Db>, id: i64, pinned: bool) -> Result<(), String> {
    db::set_publish_history_pinned(&lock_db(&db), id, pinned).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn publish_history_clear(db: State<Db>, connection_id: String) -> Result<(), String> {
    db::clear_publish_history(&lock_db(&db), &connection_id).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- messages

#[tauri::command]
pub fn message_list(
    db: State<Db>,
    connection_id: String,
    before: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<MessageRecord>, String> {
    // 按 ts DESC 查询，前端反转后即为时间正序
    db::list_messages(&lock_db(&db), &connection_id, before, limit.unwrap_or(200))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn message_clear(db: State<Db>, connection_id: String) -> Result<(), String> {
    db::clear_messages(&lock_db(&db), &connection_id).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- certificate files

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertPick {
    /// 文件名（仅用于界面展示）
    pub name: String,
    /// PEM 文本内容，存入数据库
    pub content: String,
}

/// 弹出系统文件选择框，读取证书文件内容（CA / 客户端证书 / 客户端私钥共用）
#[tauri::command]
pub async fn pick_cert_file() -> Result<Option<CertPick>, String> {
    tokio::task::spawn_blocking(|| {
        let picked = rfd::FileDialog::new()
            .set_title("Select certificate file")
            .add_filter("Certificate / Key (PEM)", &["pem", "crt", "cer", "key", "txt"])
            .add_filter("All files", &["*"])
            .pick_file();
        match picked {
            Some(path) => {
                let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "certificate".into());
                Ok(Some(CertPick { name, content }))
            }
            None => Ok(None),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

// ---------------------------------------------------------------- settings

#[tauri::command]
pub fn settings_get(db: State<Db>) -> Result<AppSettings, String> {
    Ok(settings::get(&lock_db(&db)))
}

#[tauri::command]
pub fn settings_set(db: State<Db>, patch: SettingsPatch) -> Result<AppSettings, String> {
    Ok(settings::set(&lock_db(&db), patch))
}
