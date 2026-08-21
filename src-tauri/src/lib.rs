mod apps;
mod ime;
#[cfg(windows)]
mod media;
#[cfg(not(windows))]
mod media {
    // 非 Windows 平台无 SMTC：命令返回不支持提示
    use serde::Serialize;
    #[derive(Debug, Clone, Default, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct MediaInfo {
        pub title: String,
        pub artist: String,
        pub album: String,
        pub app: String,
        pub state: String,
        pub thumbnail: Option<String>,
        pub position: f64,
        pub duration: f64,
    }
    #[tauri::command]
    pub fn media_now_playing() -> Result<MediaInfo, String> {
        Err("当前平台不支持媒体控制".into())
    }
    #[tauri::command]
    pub fn media_control(_action: String) -> Result<(), String> {
        Err("当前平台不支持媒体控制".into())
    }
}
mod backup;
mod config;
mod log;
mod notify;
mod plugins;
mod shortcuts;
mod stats;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri::{
        menu::{Menu, MenuItem},
        tray::TrayIconBuilder,
    };
    log::debug("run() 启动");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        // 全局快捷键（M6/M7）：不在 builder 里写死，setup 时按配置动态注册
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        // 系统通知（插件通知能力）：timer 等插件到点后右下角弹 toast
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            log::init(app.handle());
            // 插件目录加入 asset 作用域（M16：插件自带 JS 用 asset:// 加载）
            plugins::allow_asset_scope(app.handle());
            // 可配置快捷键：默认 Alt+Space（Pad 开关）
            shortcuts::register_current(app.handle());
            // 系统托盘图标：显示/隐藏 + 退出
            let toggle = MenuItem::with_id(app, "toggle", "显示 / 隐藏", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&toggle, &quit])?;
            let mut builder = TrayIconBuilder::new()
                .menu(&menu)
                // 菜单只在右键显示；左键只触发点击事件（显示/隐藏窗口）
                .show_menu_on_left_click(false)
                .tooltip("HomeDesktop")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "toggle" => {
                        log::info("托盘菜单: 显示/隐藏");
                        toggle_window(app);
                    }
                    "quit" => {
                        log::info("托盘菜单: 退出");
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    use tauri::tray::MouseButtonState;
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        log::debug("托盘左键单击: 切换窗口");
                        toggle_window(tray.app_handle());
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                builder = builder.icon(icon.clone());
            }
            builder.build(app)?;
            // 任务栏不显示图标（全屏启动器用托盘控制，避免显示/隐藏时任务栏按钮抖动）：
            // 直接设置 WS_EX_TOOLWINDOW 确保生效（tauri skipTaskbar 配置/API 在 wry 上未实际改样式）
            #[cfg(windows)]
            {
                use tauri::Manager;
                if let Some(win) = app.get_webview_window("main") {
                    if let Ok(hwnd) = win.hwnd() {
                        use windows::Win32::Foundation::HWND;
                        use windows::Win32::UI::WindowsAndMessaging::{
                            GetWindowLongW, SetWindowLongW, GWL_EXSTYLE, WS_EX_TOOLWINDOW,
                        };
                        unsafe {
                            const WS_EX_APPWINDOW: i32 = 0x00040000;
                            let hw = HWND(hwnd.0);
                            let ex = GetWindowLongW(hw, GWL_EXSTYLE);
                            // 清除 WS_EX_APPWINDOW（强制任务栏按钮）再设 WS_EX_TOOLWINDOW：
                            // 两者并存时 APPWINDOW 优先，任务栏按钮仍会出现
                            let ex = (ex & !WS_EX_APPWINDOW) | WS_EX_TOOLWINDOW.0 as i32;
                            SetWindowLongW(hw, GWL_EXSTYLE, ex);
                        }
                    }
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            plugins::plugins_list,
            plugins::layout_load,
            plugins::layout_save,
            plugins::launch_action,
            plugins::launch_cell,
            plugins::plugins_install,
            plugins::plugins_uninstall,
            plugins::market_scan,
            plugins::market_remote_list,
            plugins::market_remote_install,
            plugins::web_fetch_title,
            plugins::web_fetch_icon,
            config::config_get,
            config::config_set,
            config::set_wallpaper,
            apps::apps_scan,
            apps::app_icon,
            apps::launch_path,
            media::media_now_playing,
            media::media_control,
            ime::ime_save,
            ime::ime_restore,
            stats::sys_stats,
            backup::backup_export,
            backup::backup_import,
            shortcuts::shortcuts_set,
            notify::app_notify,
            log::log_write
        ])
        .run(tauri::generate_context!())
        .expect("error while running HomeDesktop");
}

/// Launchpad Pad 开关（M6）：隐藏 → 显示并全屏；全屏中 → 隐藏；窗口模式 → 全屏
/// 隐藏/显示由前端播淡出/淡入动画（emit "hide-window"/"show-window" 事件），保持苹果风格过渡
pub(crate) fn toggle_pad(app: &tauri::AppHandle) {
    use tauri::{Emitter, Manager};
    if let Some(win) = app.get_webview_window("main") {
        match win.is_visible() {
            Ok(true) => match win.is_fullscreen() {
                Ok(true) => {
                    log::debug("Pad 开关: 全屏中 → 隐藏（前端动画）");
                    let _ = app.emit("hide-window", ());
                }
                _ => {
                    log::debug("Pad 开关: 窗口模式 → 全屏");
                    let _ = win.set_fullscreen(true);
                    let _ = win.set_focus();
                }
            },
            _ => {
                log::debug("Pad 开关: 隐藏 → 显示全屏");
                let _ = win.show();
                let _ = win.set_fullscreen(true);
                let _ = win.set_focus();
                let _ = app.emit("show-window", ());
            }
        }
    }
}

fn toggle_window(app: &tauri::AppHandle) {
    use tauri::{Emitter, Manager};
    if let Some(win) = app.get_webview_window("main") {
        match win.is_visible() {
            Ok(true) => {
                log::debug("托盘: 隐藏（前端动画）");
                let _ = app.emit("hide-window", ());
            }
            _ => {
                let _ = win.show();
                let _ = win.set_focus();
                let _ = app.emit("show-window", ());
            }
        }
    }
}
