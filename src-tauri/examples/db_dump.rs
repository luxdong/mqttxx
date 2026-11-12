//! 开发辅助：打印应用数据库的 user_version 与 connections 表结构
//! 用法：cargo run --example db_dump
use rusqlite::Connection;

fn main() {
    let dir = std::env::var("APPDATA").expect("APPDATA not set");
    let path = std::path::Path::new(&dir)
        .join("com.mqttxx.desktop")
        .join("mqttxx.db");
    let conn = Connection::open_with_flags(
        &path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("open db");
    let ver: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
    println!("db: {}", path.display());
    println!("user_version: {ver}");
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap();
    let tables: Vec<String> = stmt
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    println!("tables: {tables:?}");
    let mut stmt = conn.prepare("PRAGMA table_info(connections)").unwrap();
    let cols: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(1))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    println!("connections columns ({})", cols.len());
    println!("{cols:?}");
}
