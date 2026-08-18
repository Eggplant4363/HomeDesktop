//! 系统通知（插件通知能力）：插件通过桥接调用 app_notify，在桌面右下角弹系统 toast。
//!
//! Windows 上走 tauri-plugin-notification（WinRT 通知，无需额外权限申请），
//! 由宿主窗口 fetch -> postMessage 桥把 title/body 传过来。

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// 发送系统通知。title/body 来自插件桥的 `notify(title, body)`。
#[tauri::command]
pub fn app_notify(app: AppHandle, title: String, body: String) -> Result<(), String> {
    let t = title.trim();
    let b = body.trim();
    if t.is_empty() && b.is_empty() {
        let msg = "通知内容为空（title 与 body 均未提供）";
        crate::log::warn(msg);
        return Err(msg.into());
    }
    match app
        .notification()
        .builder()
        .title(t)
        .body(b)
        .show()
    {
        Ok(()) => {
            crate::log::info(&format!("系统通知: {t} | {b}"));
            Ok(())
        }
        Err(e) => {
            crate::log::error(&format!("系统通知失败: {e}"));
            Err(format!("系统通知失败: {e}"))
        }
    }
}
