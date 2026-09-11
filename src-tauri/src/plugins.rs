//! Tauri 壳：将 homedesktop-core 的纯逻辑暴露为 tauri 命令
//! （插件发现、布局持久化、动作执行的核心实现见 crates/homedesktop-core）

use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use homedesktop_core::{
    collect_plugins_with_builtin, launch_plugin_action, read_layout_from, write_layout_to, Layout,
    PluginInfo,
};

/// 插件目录优先级：用户数据目录 > 资源目录 > 开发期向上查找（exe 位置 / cwd 的祖先目录）
fn plugin_dirs(app: &AppHandle) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(d) = app.path().app_data_dir() {
        dirs.push(d.join("plugins"));
    }
    if let Ok(d) = app.path().resource_dir() {
        dirs.push(d.join("plugins"));
    }

    // 开发期：tauri dev 启动的进程 cwd 不固定（可能是 src-tauri / 项目根 / target 等），
    // 因此从 exe 所在目录与 cwd 分别向上查找最多 6 层，收集存在的 <dir>/plugins
    let mut anchors: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            anchors.push(dir.to_path_buf());
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        anchors.push(cwd);
    }
    for anchor in anchors {
        let mut cur = Some(anchor.as_path());
        for _ in 0..6 {
            if let Some(c) = cur {
                dirs.push(c.join("plugins"));
                cur = c.parent();
            }
        }
    }

    // 去重（保持顺序）
    let mut seen = std::collections::HashSet::new();
    dirs.retain(|d| seen.insert(d.clone()));
    eprintln!("[homedesktop] plugin dirs: {dirs:?}");
    dirs
}

#[tauri::command]
pub fn plugins_list(app: AppHandle) -> Vec<PluginInfo> {
    collect_plugins_with_builtin(&plugin_dirs(&app))
        .into_iter()
        .map(|(mut p, builtin)| {
            p.builtin = builtin;
            p
        })
        .collect()
}

/// 把插件目录动态加入 asset 协议作用域（M16：插件自带 JS 用 asset:// 加载）。
/// 默认 scope 只覆盖 $APPDATA/**，dev 模式的项目根 plugins/ 不在其内，需运行时放行。
pub fn allow_asset_scope(app: &tauri::AppHandle) {
    for dir in plugin_dirs(app) {
        if dir.is_dir() {
            let _ = app.asset_protocol_scope().allow_directory(&dir, true);
            crate::log::debug(&format!("asset 作用域放行插件目录: {}", dir.display()));
        }
    }
}

/// 卸载用户安装的插件（删除用户数据目录 plugins/<id>/；内置插件不可卸载）
#[tauri::command]
pub fn plugins_uninstall(app: AppHandle, plugin_id: String) -> Result<(), String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let dir = data_dir.join("plugins").join(&plugin_id);
    if !dir.is_dir() {
        return Err("该插件为内置插件或未安装到用户目录，不可卸载".to_string());
    }
    std::fs::remove_dir_all(&dir).map_err(|e| format!("卸载失败: {e}"))?;
    crate::log::info(&format!("插件已卸载: {plugin_id}"));
    Ok(())
}

