//! 输入法（IME）状态保存/恢复：应用显示/隐藏时窗口焦点来回切换，
//! Windows 会重置 IME 的开/关与转换模式（中/英），导致"输入法转变"。
//! 对策：隐藏前 ime_save 保存，显示后 ime_restore 恢复。

use std::sync::Mutex;
use std::sync::OnceLock;

/// (open, conversion, sentence)
static SAVED_IME: OnceLock<Mutex<Option<(bool, u32, u32)>>> = OnceLock::new();

fn ime_slot() -> &'static Mutex<Option<(bool, u32, u32)>> {
    SAVED_IME.get_or_init(|| Mutex::new(None))
}

#[cfg(windows)]
fn main_hwnd(app: &tauri::AppHandle) -> Option<windows::Win32::Foundation::HWND> {
    use tauri::Manager;
    app.get_webview_window("main")
        .and_then(|w| w.hwnd().ok())
        .map(|h| windows::Win32::Foundation::HWND(h.0))
}

/// 保存当前输入法状态（隐藏前调用）
#[tauri::command]
pub fn ime_save(app: tauri::AppHandle) {
    #[cfg(windows)]
    {
        use windows::Win32::UI::Input::Ime::{
            ImmGetContext, ImmGetConversionStatus, ImmGetOpenStatus, ImmReleaseContext,
        };
        if let Some(hwnd) = main_hwnd(&app) {
            unsafe {
                let hime = ImmGetContext(hwnd);
                if !hime.is_invalid() {
                    let open = ImmGetOpenStatus(hime).as_bool();
                    let mut conv: u32 = 0;
                    let mut sent: u32 = 0;
                    let _ = ImmGetConversionStatus(
                        hime,
                        Some(&mut conv as *mut u32 as _),
                        Some(&mut sent as *mut u32 as _),
                    );
                    if let Ok(mut slot) = ime_slot().lock() {
                        *slot = Some((open, conv, sent));
                    }
                    let _ = ImmReleaseContext(hwnd, hime);
                }
            }
        }
    }
}

/// 恢复之前保存的输入法状态（显示后调用）
#[tauri::command]
pub fn ime_restore(app: tauri::AppHandle) {
    #[cfg(windows)]
    {
        use windows::Win32::UI::Input::Ime::{
            ImmGetContext, ImmReleaseContext, ImmSetConversionStatus, ImmSetOpenStatus,
        };
        if let Some(hwnd) = main_hwnd(&app) {
            if let Ok(slot) = ime_slot().lock() {
                if let Some((open, conv, sent)) = *slot {
                    unsafe {
                        let hime = ImmGetContext(hwnd);
                        if !hime.is_invalid() {
                            let _ = ImmSetConversionStatus(
                                hime,
                                windows::Win32::UI::Input::Ime::IME_CONVERSION_MODE(conv),
                                windows::Win32::UI::Input::Ime::IME_SENTENCE_MODE(sent),
                            );
                            let _ = ImmSetOpenStatus(hime, open);
                            let _ = ImmReleaseContext(hwnd, hime);
                        }
                    }
                }
            }
        }
    }
}
