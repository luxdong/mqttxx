//! 更新中心（Gitee Releases）
//!
//! 流程：check_update 拉 Gitee 最新 Release → 比较版本 → 命中则前端展示新版本卡片
//! → download_update 流式下载（`update-progress` 事件推进度）→ install_update
//! 按扩展名静默安装（NSIS `/S` 或 `msiexec /quiet`）后退出应用。

use serde::{Deserialize, Serialize};
use std::io::Write;
use tauri::{AppHandle, Emitter, Manager};

use crate::db::Db;
use crate::settings;

const RELEASE_API: &str = "https://gitee.com/api/v5/repos/oslu/mqttxx/releases/latest";

#[derive(Debug, Deserialize)]
struct GiteeRelease {
    tag_name: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    assets: Vec<GiteeAsset>,
}

#[derive(Debug, Deserialize)]
struct GiteeAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub notes: String,
    pub asset_name: String,
    pub download_url: String,
    pub asset_size: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadProgress {
    received: u64,
    total: u64,
}

/// "v1.2.3" → "1.2.3"
fn parse_version(tag: &str) -> String {
    tag.trim().trim_start_matches(['v', 'V']).to_string()
}

/// 分段数值比较（各段缺失按 0 处理）：1.2.10 > 1.2.9
fn is_newer(latest: &str, current: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.')
            .map(|p| p.trim().parse::<u64>().unwrap_or(0))
            .collect()
    };
    let l = parse(latest);
    let c = parse(current);
    for i in 0..l.len().max(c.len()) {
        let li = l.get(i).copied().unwrap_or(0);
        let ci = c.get(i).copied().unwrap_or(0);
        if li != ci {
            return li > ci;
        }
    }
    false
}

/// 检查 Gitee 最新 Release。返回 None 表示已是最新；Err 为网络/数据问题。
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    let current = app.package_info().version.to_string();

    // 可选的 Gitee 私人令牌：带认证调用解除匿名限流
    let mut url = RELEASE_API.to_string();
    let token = {
        let state = app.state::<Db>();
        let conn = state.0.lock().unwrap();
        settings::get(&conn).gitee_token
    };
    if !token.trim().is_empty() {
        url.push_str(&format!("?access_token={}", token.trim()));
    }

    let resp = reqwest::get(&url)
        .await
        .map_err(|e| format!("network: {e}"))?;
    if !resp.status().is_success() {
        match resp.status().as_u16() {
            403 => {
                return Err(
                    "Gitee 匿名请求被限流(403)：请稍后再试，或在设置中配置 Gitee 私人令牌".into(),
                )
            }
            404 => {
                return Err(
                    "gitee API 404: repository is private or has no release yet (make the repo public on gitee.com)"
                        .into(),
                )
            }
            s => return Err(format!("gitee api status: {s}")),
        }
    }
    let rel: GiteeRelease = resp.json().await.map_err(|e| format!("parse: {e}"))?;
    let latest = rel.tag_name.as_deref().map(parse_version).unwrap_or_default();
    if latest.is_empty() || !is_newer(&latest, &current) {
        return Ok(None);
    }
    // 优先 NSIS setup.exe，其次任意 .exe，最后 .msi
    let pick = rel
        .assets
        .iter()
        .find(|a| a.name.ends_with("-setup.exe"))
        .or_else(|| rel.assets.iter().find(|a| a.name.ends_with(".exe")))
        .or_else(|| rel.assets.iter().find(|a| a.name.ends_with(".msi")));
    match pick {
        Some(a) => Ok(Some(UpdateInfo {
            current_version: current,
            latest_version: latest,
            notes: rel.body.unwrap_or_default(),
            asset_name: a.name.clone(),
            download_url: a.browser_download_url.clone(),
            asset_size: a.size,
        })),
        None => Err("release has no Windows installer asset".into()),
    }
}

/// 流式下载安装包到缓存目录，进度经 `update-progress` 事件推送，返回安装包路径。
#[tauri::command]
pub async fn download_update(app: AppHandle, info: UpdateInfo) -> Result<String, String> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("update");
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| e.to_string())?;
    let file_path = dir.join(&info.asset_name);

    let mut resp = reqwest::Client::new()
        .get(&info.download_url)
        .send()
        .await
        .map_err(|e| format!("download: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("download status: {}", resp.status()));
    }
    let total = resp
        .content_length()
        .filter(|&n| n > 0)
        .unwrap_or(info.asset_size.max(0) as u64);

    let mut file = std::fs::File::create(&file_path)
        .map_err(|e| format!("create file: {e}"))?;
    let mut received: u64 = 0;
    let mut last_percent: u64 = 0;
    while let Some(chunk) = resp.chunk().await.map_err(|e| format!("download: {e}"))? {
        file.write_all(&chunk).map_err(|e| format!("write: {e}"))?;
        received += chunk.len() as u64;
        if total > 0 {
            let percent = (received * 100 / total).min(100);
            if percent >= last_percent + 2 || percent == 100 {
                last_percent = percent;
                let _ = app.emit(
                    "update-progress",
                    DownloadProgress {
                        received,
                        total,
                    },
                );
            }
        }
    }
    file.flush().ok();
    let _ = app.emit(
        "update-progress",
        DownloadProgress {
            received,
            total,
        },
    );
    Ok(file_path.to_string_lossy().to_string())
}

/// 运行安装包（静默），随后退出应用让安装器接管。
///
/// 采用延迟脚本方案避免"应用尚未退出安装器就抢文件锁"的竞态：
/// 等 2s（应用退出）→ 静默安装 → 等 3s（安装完成）→ 自动重启新版本。
#[tauri::command]
pub async fn install_update(app: AppHandle, file_path: String) -> Result<(), String> {
    let path = std::path::PathBuf::from(&file_path);
    if !path.exists() {
        return Err("installer file not found".into());
    }
    let is_msi = path
        .extension()
        .map(|e| e.eq_ignore_ascii_case("msi"))
        .unwrap_or(false);
    let relaunch = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .to_string();

    let installer = path.to_string_lossy().to_string();
    let script = if is_msi {
        format!(
            "@echo off\r\n timeout /t 2 /nobreak >nul\r\n msiexec /i \"{installer}\" /quiet\r\n timeout /t 3 /nobreak >nul\r\n start \"\" \"{relaunch}\"\r\n"
        )
    } else {
        format!(
            "@echo off\r\n timeout /t 2 /nobreak >nul\r\n \"{installer}\" /S\r\n timeout /t 3 /nobreak >nul\r\n start \"\" \"{relaunch}\"\r\n"
        )
    };

    let bat = path
        .parent()
        .ok_or("no parent dir")?
        .join("install-update.bat");
    std::fs::write(&bat, script).map_err(|e| format!("write script: {e}"))?;

    // CREATE_NO_WINDOW：不弹黑色命令行窗口
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new("cmd")
        .args(["/C", &bat.to_string_lossy()])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("launch installer: {e}"))?;

    // 给脚本一点启动时间，再退出应用释放文件锁
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    app.exit(0);
    Ok(())
}
