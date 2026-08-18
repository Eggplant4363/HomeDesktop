//! 布局/配置备份（M13）：导出 layout.json + config.json 为单个备份文件，支持导入恢复

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use tauri::{AppHandle, Manager};

const BACKUP_APP: &str = "homedesktop";
const BACKUP_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupFile {
    app: String,
    version: u32,
    exported_at: String,
    layout: Value,
    config: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSummary {
    pub pages: usize,
    pub cells: usize,
    pub config_keys: usize,
}

fn app_data(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path().app_data_dir().map_err(|e| e.to_string())
}

fn read_json_or_default(path: &Path) -> Value {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(Value::Null)
}

/// 导出：把 layout.json + config.json 合并写成一个备份文件
#[tauri::command]
pub fn backup_export(app: AppHandle, dest: String) -> Result<(), String> {
    let dir = app_data(&app)?;
    let layout = read_json_or_default(&dir.join("layout.json"));
    let config = read_json_or_default(&dir.join("config.json"));
    let backup = BackupFile {
        app: BACKUP_APP.into(),
        version: BACKUP_VERSION,
        exported_at: chrono_now(),
        layout,
        config,
    };
    let text = serde_json::to_string_pretty(&backup).map_err(|e| e.to_string())?;
    std::fs::write(&dest, text).map_err(|e| format!("写入备份文件失败: {e}"))?;
    crate::log::info(&format!("备份已导出: {dest}"));
    Ok(())
}

/// 导入：校验备份文件 → 写回 layout.json + config.json（先备份当前文件为 .bak）。
/// mode: "merge" = 合并（保留当前布局，追加备份中不存在的图标；配置按键覆盖），其余 = 覆盖。
#[tauri::command]
pub fn backup_import(app: AppHandle, src: String, mode: String) -> Result<BackupSummary, String> {
    let text = std::fs::read_to_string(&src).map_err(|e| format!("读取备份文件失败: {e}"))?;
    let backup: BackupFile =
        serde_json::from_str(&text).map_err(|e| format!("备份文件格式错误: {e}"))?;
    if backup.app != BACKUP_APP || backup.layout.is_null() {
        return Err("不是有效的 HomeDesktop 备份文件".to_string());
    }

    let is_merge = mode == "merge";
    let dir = app_data(&app)?;
    let layout_path = dir.join("layout.json");
    let config_path = dir.join("config.json");

    // layout 走 v1→v2 迁移（兼容旧备份）
    let mut layout = backup.layout;
    homedesktop_core::migrate_layout(&mut layout);
    let config = backup.config;

    // 合并模式：保留当前布局 + 追加备份独有的图标；配置按键合并（备份胜出）
    let (layout_out, config_out) = if is_merge {
        let current_layout = read_json_or_default(&layout_path);
        let current_config = read_json_or_default(&config_path);
        (
            homedesktop_core::merge_layout(&current_layout, &layout),
            homedesktop_core::merge_config(&current_config, &config),
        )
    } else {
        (layout, config)
    };

    // 导入前备份当前文件
    for p in [&layout_path, &config_path] {
        if p.is_file() {
            let _ = std::fs::copy(p, p.with_extension("json.bak"));
        }
    }
    std::fs::write(
        &layout_path,
        serde_json::to_string_pretty(&layout_out).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("写入布局失败: {e}"))?;
    std::fs::write(
        &config_path,
        serde_json::to_string_pretty(&config_out).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("写入配置失败: {e}"))?;

    let pages = layout_out
        .get("pages")
        .and_then(|p| p.as_array())
        .map(|p| p.len())
        .unwrap_or(0);
    let cells = layout_out
        .get("pages")
        .and_then(|p| p.as_array())
        .map(|pages| pages.iter().map(|pg| pg.as_array().map(|a| a.len()).unwrap_or(0)).sum())
        .unwrap_or(0);
    let config_keys = config_out
        .as_object()
        .map(|m| m.len())
        .unwrap_or(0);
    crate::log::info(&format!(
        "备份已导入({}): {src}（{pages} 页 {cells} 单元，配置 {config_keys} 键）",
        if is_merge { "合并" } else { "覆盖" }
    ));
    Ok(BackupSummary {
        pages,
        cells,
        config_keys,
    })
}

/// 简易时间戳（避免引入 chrono 依赖）
fn chrono_now() -> String {
    #[cfg(target_os = "windows")]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        // 距 2020-01-01 的秒数（大致时间，仅用于文件名可读性）
        format!("t{}", secs)
    }
    #[cfg(not(target_os = "windows"))]
    {
        String::new()
    }
}
