//! 剪贴板历史（全局监听，跨应用自动收录）
//! - 随 app 启动自动起后台 watcher：每 ~500ms 读一次系统剪贴板文本，
//!   内容变化且非空 → 历史去重（即同文置顶）并广播事件 "cbpush"。
//! - 提供命令 clip_state / clip_clear / clip_paste（写回主题剪贴板）。

use copypasta::{ClipboardContext, ClipboardProvider};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

static MAX: usize = 80;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClipEntry {
    id: u64,
    text: String,
    at: u64,
}

#[derive(Default)]
pub struct ClipState {
    items: Mutex<VecDeque<ClipEntry>>,
    seq: AtomicU64,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 启动后台监听线程（每 350ms 读一次剪贴板）。出错则静默退出该线程。
pub fn start_watcher(app: AppHandle) {
    std::thread::spawn(move || {
        let mut ctx = match ClipboardContext::new() {
            Ok(c) => c,
            Err(_) => return,
        };
        let mut last: Option<String> = None;
        loop {
            let cur = ctx.get_contents().unwrap_or_default();
            let trimmed = cur.trim().to_string();
            let changed = last.as_deref() != Some(trimmed.as_str());
            if !changed {
                std::thread::sleep(std::time::Duration::from_millis(300));
                continue;
            }
            last = Some(trimmed.clone());
            if trimmed.is_empty() {
                std::thread::sleep(std::time::Duration::from_millis(300));
                continue;
            }
            // 追加历史（去重置顶）
            let st = app.state::<ClipState>();
            let id = st.seq.fetch_add(1, Ordering::SeqCst) + 1;
            let entry = ClipEntry { id, text: trimmed.clone(), at: now_ms() };
            {
                let mut g = st.items.lock().unwrap();
                if let Some(pos) = g.iter().position(|e| e.text == trimmed) {
                    g.remove(pos);
                }
                g.push_front(entry.clone());
                while g.len() > MAX {
                    g.pop_back();
                }
            }
            let _ = app.emit("cbpush", &entry);
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
    });
}

#[tauri::command]
pub fn clip_state(state: State<'_, ClipState>) -> Vec<ClipEntry> {
    state.items.lock().unwrap().iter().cloned().collect()
}

/// 把文本写回系统剪贴板（点击历史条目"一键粘贴"对应用）
#[tauri::command]
pub fn clip_paste(text: String) -> Result<(), String> {
    let mut ctx = ClipboardContext::new().map_err(|e| e.to_string())?;
    ctx.set_contents(text).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clip_clear(state: State<'_, ClipState>) {
    state.items.lock().unwrap().clear();
}
