//! 分级日志系统：写入 app 数据目录 homedesktop.log
//! - 级别：debug < info < warn < error < off
//! - 开发构建默认 debug（记录全部操作）；release 构建默认 off（用户要求关闭）
//! - 可用环境变量 HOMEDESKTOP_LOG_LEVEL 覆盖（debug|info|warn|error|off）

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
    Off,
}

static LEVEL: Mutex<Option<Level>> = Mutex::new(None);
static FILE: Mutex<Option<PathBuf>> = Mutex::new(None);

fn parse_level(s: &str) -> Level {
    match s.trim().to_lowercase().as_str() {
        "debug" => Level::Debug,
        "info" => Level::Info,
        "warn" => Level::Warn,
        "error" => Level::Error,
        _ => Level::Off,
    }
}

pub fn level_name(l: Level) -> &'static str {
    match l {
        Level::Debug => "DEBUG",
        Level::Info => "INFO",
        Level::Warn => "WARN",
        Level::Error => "ERROR",
        Level::Off => "OFF",
    }
}

/// 初始化日志：决定级别、清空旧日志文件
pub fn init(app: &AppHandle) {
    let level = match std::env::var("HOMEDESKTOP_LOG_LEVEL") {
        Ok(s) => parse_level(&s),
        Err(_) => {
            #[cfg(debug_assertions)]
            {
                Level::Debug
            }
            #[cfg(not(debug_assertions))]
            {
                Level::Off
            }
        }
    };
    *LEVEL.lock().unwrap() = Some(level);

    if let Ok(data_dir) = app.path().app_data_dir() {
        let path = data_dir.join("homedesktop.log");
        let _ = std::fs::remove_file(&path);
        *FILE.lock().unwrap() = Some(path);
    }
    log(Level::Info, "==== HomeDesktop 日志开始 ====");
    log(Level::Info, &format!("日志级别: {}", level_name(level)));
}

/// 写入一条日志（级别低于当前级别时丢弃）
pub fn log(level: Level, msg: &str) {
    let cur = *LEVEL.lock().unwrap();
    let cur = cur.unwrap_or(Level::Off);
    if level < cur {
        return;
    }

    let ts = format_ts();
    let line = format!("{ts} [{:5}] {msg}", level_name(level));

    if let Some(path) = FILE.lock().unwrap().clone() {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(f, "{line}");
        }
    }
    // 开发时同时输出到控制台，方便 tauri dev 实时查看
    #[cfg(debug_assertions)]
    {
        eprintln!("[homedesktop] {line}");
    }
}

pub fn debug(msg: &str) {
    log(Level::Debug, msg);
}
pub fn info(msg: &str) {
    log(Level::Info, msg);
}
#[allow(dead_code)]
pub fn warn(msg: &str) {
    log(Level::Warn, msg);
}
#[allow(dead_code)]
pub fn error(msg: &str) {
    log(Level::Error, msg);
}

/// 前端通过此命令写入日志（fire-and-forget）
#[tauri::command]
pub fn log_write(level: String, message: String) {
    log(parse_level(&level), &message);
}

/// 简易时间戳（UTC 时:分:秒.毫秒 + 自 epoch 起的秒数），避免引入时间库
fn format_ts() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let total = now.as_secs();
    let h = (total % 86400) / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    let ms = now.subsec_millis();
    format!("{h:02}:{m:02}:{s:02}.{ms:03} (UTC, t+{total}s)")
}
