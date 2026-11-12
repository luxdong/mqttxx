use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub theme: String,
    pub lang: String,
    pub max_messages_in_memory: i64,
    pub auto_resub: bool,
    pub auto_check_update: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "light".into(),
            lang: "zh".into(),
            max_messages_in_memory: 1000,
            auto_resub: true,
            auto_check_update: true,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    pub theme: Option<String>,
    pub lang: Option<String>,
    pub max_messages_in_memory: Option<i64>,
    pub auto_resub: Option<bool>,
    pub auto_check_update: Option<bool>,
}

fn read_key(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
        .ok()
}

fn write_key(conn: &Connection, key: &str, value: &str) {
    let _ = conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
        [key, value],
    );
}

pub fn get(conn: &Connection) -> AppSettings {
    let mut s = AppSettings::default();
    if let Some(v) = read_key(conn, "theme") {
        s.theme = v;
    }
    if let Some(v) = read_key(conn, "lang") {
        s.lang = v;
    }
    if let Some(v) = read_key(conn, "max_messages_in_memory").and_then(|v| v.parse().ok()) {
        s.max_messages_in_memory = v;
    }
    if let Some(v) = read_key(conn, "auto_resub") {
        s.auto_resub = v == "true";
    }
    if let Some(v) = read_key(conn, "auto_check_update") {
        s.auto_check_update = v == "true";
    }
    s
}

pub fn set(conn: &Connection, patch: SettingsPatch) -> AppSettings {
    let mut current = get(conn);
    if let Some(v) = patch.theme {
        current.theme = v;
    }
    if let Some(v) = patch.lang {
        current.lang = v;
    }
    if let Some(v) = patch.max_messages_in_memory {
        current.max_messages_in_memory = v.clamp(100, 100_000);
    }
    if let Some(v) = patch.auto_resub {
        current.auto_resub = v;
    }
    if let Some(v) = patch.auto_check_update {
        current.auto_check_update = v;
    }
    write_key(conn, "theme", &current.theme);
    write_key(conn, "lang", &current.lang);
    write_key(conn, "max_messages_in_memory", &current.max_messages_in_memory.to_string());
    write_key(conn, "auto_resub", if current.auto_resub { "true" } else { "false" });
    write_key(conn, "auto_check_update", if current.auto_check_update { "true" } else { "false" });
    current
}
