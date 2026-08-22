//! 可配置全局快捷键（M7 基础 + M10 扩展）：
//! - togglePad：显示/隐藏 Pad（全屏），默认 alt+space
//! - search：唤起全局搜索面板，默认 ctrl+space（由前端监听事件打开面板）
//! 配置存 config.json（shortcuts.<action>）；修改时动态 注册新键（失败则保持旧键）→ 注销旧键 → 持久化。

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use serde::Serialize;

/// 快捷键注册结果（供前端展示冲突警告，不影响启动）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutStatus {
    pub action: String,
    pub spec: String,
    pub ok: bool,
    pub error: Option<String>,
}

fn status_slot() -> &'static Mutex<HashMap<String, ShortcutStatus>> {
    static S: OnceLock<Mutex<HashMap<String, ShortcutStatus>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashMap::new()))
}

fn record(action: &str, spec: &str, result: Result<(), String>) {
    if let Ok(mut m) = status_slot().lock() {
        m.insert(
            action.to_string(),
            ShortcutStatus {
                action: action.to_string(),
                spec: spec.to_string(),
                ok: result.is_ok(),
                error: result.err(),
            },
        );
    }
}

/// 查询各快捷键注册状态（前端设置页展示冲突警告）
#[tauri::command]
pub fn shortcuts_status() -> Vec<ShortcutStatus> {
    let mut v: Vec<ShortcutStatus> = Vec::new();
    if let Ok(m) = status_slot().lock() {
        v = m.values().cloned().collect();
    }
    v.sort_by(|a, b| a.action.cmp(&b.action));
    v
}

pub const DEFAULT_TOGGLE: &str = "alt+space";
pub const DEFAULT_SEARCH: &str = "ctrl+space";
const TOGGLE_KEY: &str = "shortcuts.togglePad";
const SEARCH_KEY: &str = "shortcuts.search";
/// 前端监听此事件打开搜索面板
pub const SEARCH_EVENT: &str = "homedesktop:search";

/// 读取配置中的快捷键（缺失/非法 → 默认）
fn load(app: &AppHandle, key: &str, default: &str) -> String {
    match crate::config::get_str(app, key) {
        Some(v) if !v.trim().is_empty() => {
            let v = v.trim().to_lowercase();
            if v.parse::<Shortcut>().is_ok() {
                v
            } else {
                crate::log::warn(&format!("配置的快捷键非法，回退默认: {v}"));
                default.to_string()
            }
        }
        _ => default.to_string(),
    }
}

/// 注册 Pad 开关快捷键（Pressed → toggle_pad）
fn register_toggle(app: &AppHandle, spec: &str) -> Result<(), String> {
    app.global_shortcut()
        .on_shortcut(spec, |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                crate::toggle_pad(app);
            }
        })
        .map_err(|e| format!("注册快捷键失败（可能被其他应用占用）: {e}"))
}

/// 注册搜索唤起快捷键（Pressed → 通知前端打开搜索面板）
fn register_search(app: &AppHandle, spec: &str) -> Result<(), String> {
    app.global_shortcut()
        .on_shortcut(spec, |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                crate::log::debug("搜索快捷键按下");
                let _ = app.emit(SEARCH_EVENT, ());
            }
        })
        .map_err(|e| format!("注册快捷键失败（可能被其他应用占用）: {e}"))
}

/// setup 时注册配置的快捷键
pub fn register_current(app: &AppHandle) {
    let toggle = load(app, TOGGLE_KEY, DEFAULT_TOGGLE);
    crate::log::info(&format!("注册全局快捷键: togglePad={toggle}"));
    let r1 = register_toggle(app, &toggle);
    record("togglePad", &toggle, r1.clone());
    if let Err(e) = r1 {
        crate::log::error(&e);
    }
    let search = load(app, SEARCH_KEY, DEFAULT_SEARCH);
    crate::log::info(&format!("注册全局快捷键: search={search}"));
    let r2 = register_search(app, &search);
    record("search", &search, r2.clone());
    if let Err(e) = r2 {
        crate::log::error(&e);
    }
}

/// 动作 → (配置键, 默认值)
fn action_spec(action: &str) -> Result<(&str, &str), String> {
    match action {
        "togglePad" => Ok((TOGGLE_KEY, DEFAULT_TOGGLE)),
        "search" => Ok((SEARCH_KEY, DEFAULT_SEARCH)),
        _ => Err(format!("未知快捷键动作: {action}")),
    }
}

/// 修改快捷键：先注册新键（失败则保持旧键）→ 注销旧键 → 持久化
pub fn set_shortcut(app: &AppHandle, action: &str, spec: &str) -> Result<String, String> {
    let (key, default) = action_spec(action)?;
    let spec = spec.trim().to_lowercase();
    if spec.is_empty() {
        return Err("快捷键不能为空".to_string());
    }
    let old = load(app, key, default);
    if spec == old {
        return Ok(spec);
    }
    // 先注册新键：失败说明被其他应用占用，旧键保持不变；无论成败都记录状态供前端展示
    match action {
        "togglePad" => {
            let r = register_toggle(app, &spec);
            record("togglePad", &spec, r.clone());
            r?;
        }
        "search" => {
            let r = register_search(app, &spec);
            record("search", &spec, r.clone());
            r?;
        }
        _ => unreachable!(),
    }
    let _ = app.global_shortcut().unregister(old.as_str());
    crate::config::set_str(app, key, &spec)?;
    crate::log::info(&format!("快捷键已修改: {action} {old} -> {spec}"));
    Ok(spec)
}

#[tauri::command]
pub fn shortcuts_set(app: AppHandle, action: String, value: String) -> Result<String, String> {
    set_shortcut(&app, &action, &value)
}
