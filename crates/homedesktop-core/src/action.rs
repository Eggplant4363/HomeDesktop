//! 动作执行：启动应用 / 执行命令（按平台分发）。

use crate::schema::*;

// ---------- 动作执行 ----------

/// 执行单个动作（纯函数，便于单测错误路径）
pub fn execute_action(action: &ActionSpec) -> Result<(), String> {
    match action.kind {
        ActionKind::App => {
            let path = action.path.clone().ok_or("app action missing path")?;
            launch_app(&path)
        }
        ActionKind::Command => {
            let cmd = action.cmd.clone().ok_or("command action missing cmd")?;
            run_command(&cmd)
        }
        // 系统应用面板是纯前端行为（打开应用列表），后端不执行
        ActionKind::SystemApps => Err("system_apps 动作由前端处理".to_string()),
    }
}

/// 根据插件动作分发执行（纯函数，便于单测错误路径）
pub fn launch_plugin_action(plugin: &PluginInfo) -> Result<(), String> {
    let action = plugin
        .actions
        .first()
        .ok_or_else(|| "plugin has no actions".to_string())?;
    execute_action(action)
}

fn launch_app(path: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // CREATE_NO_WINDOW(0x08000000)：不弹出黑色 cmd 窗口
        use std::os::windows::process::CommandExt;
        std::process::Command::new("cmd")
            .args(["/C", "start", "", path])
            .creation_flags(0x08000000)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

fn run_command(cmd: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // CREATE_NO_WINDOW(0x08000000)：不弹出黑色 cmd 窗口
        use std::os::windows::process::CommandExt;
        std::process::Command::new("cmd")
            .args(["/C", cmd])
            .creation_flags(0x08000000)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

