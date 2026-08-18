//! 通用键值配置持久化（app 数据目录 config.json）
//! 用于小组件配置（天气城市等）与后续外观设置。

use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn config_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("config.json"))
}

fn load_map(path: &PathBuf) -> BTreeMap<String, Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

#[tauri::command]
pub fn config_get(app: AppHandle, key: String) -> Option<Value> {
    let path = config_path(&app)?;
    load_map(&path).get(&key).cloned()
}

#[tauri::command]
pub fn config_set(app: AppHandle, key: String, value: Value) -> Result<(), String> {
    let path = config_path(&app).ok_or("app data dir unavailable")?;
    let mut map = load_map(&path);
    map.insert(key, value);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(&map).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

/// 把壁纸图片拷贝到 app 数据目录（wallpaper.<ext>），返回存储后的绝对路径
/// （拷贝到数据目录后，asset 协议作用域只需覆盖 $APPDATA）
#[tauri::command]
pub fn set_wallpaper(app: AppHandle, src: String) -> Result<String, String> {
    const ALLOWED: [&str; 6] = ["png", "jpg", "jpeg", "webp", "gif", "bmp"];
    let ext = std::path::Path::new(&src)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .ok_or_else(|| "文件没有扩展名".to_string())?;
    if !ALLOWED.contains(&ext.as_str()) {
        return Err(format!("不支持的图片格式: .{ext}"));
    }
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    let dest = data_dir.join(format!("wallpaper.{ext}"));
    fs::copy(&src, &dest).map_err(|e| format!("拷贝失败: {e}"))?;
    Ok(dest.to_string_lossy().into_owned())
}

// ---------- 内部字符串读写（供其他模块复用，如快捷键配置） ----------

pub(crate) fn get_str(app: &AppHandle, key: &str) -> Option<String> {
    config_get(app.clone(), key.to_string()).and_then(|v| v.as_str().map(|s| s.to_string()))
}

pub(crate) fn set_str(app: &AppHandle, key: &str, value: &str) -> Result<(), String> {
    config_set(app.clone(), key.to_string(), Value::String(value.to_string()))
}
