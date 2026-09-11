//! 布局持久化与单元格启动（阶段4 从 plugins.rs 拆出）。

use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use homedesktop_core::{
    launch_plugin_action, read_layout_from, write_layout_to, Layout, PluginInfo,
};

use crate::plugins::plugins_list;
fn layout_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("layout.json"))
}

#[tauri::command]
pub fn layout_load(app: AppHandle) -> Option<Layout> {
    read_layout_from(&layout_path(&app)?)
}

#[tauri::command]
pub fn layout_save(app: AppHandle, layout: Layout) -> Result<(), String> {
    crate::log::debug(&format!("layout_save: {} 页", layout.pages.len()));
    let path = layout_path(&app).ok_or("app data dir unavailable")?;
    // 单槽位备份：写前把旧文件复制为 layout.json.bak（防误删页等意外丢失数据，可手动恢复）
    if path.is_file() {
        let _ = std::fs::copy(&path, path.with_extension("json.bak"));
    }
    write_layout_to(&path, &layout)
}

#[tauri::command]
pub fn launch_action(app: AppHandle, plugin_id: String) -> Result<(), String> {
    crate::log::info(&format!("launch_action: {plugin_id}"));
    let plugin = plugins_list(app)
        .into_iter()
        .find(|p| p.id == plugin_id)
        .ok_or_else(|| format!("plugin not found: {plugin_id}"))?;
    launch_plugin_action(&plugin)
}

/// 按单元格 id 启动：优先用图标自带动作（应用抽屉），否则回退到插件动作。
/// 覆盖页面顶层图标与**文件夹内图标**（移入文件夹后仍可启动）。
#[tauri::command]
pub fn launch_cell(app: AppHandle, cell_id: String) -> Result<(), String> {
    crate::log::info(&format!("launch_cell: {cell_id}"));
    use homedesktop_core::{execute_action, ActionSpec, Cell};
    let layout = layout_load(app.clone()).unwrap_or_default();

    /// id 匹配则启动（action 优先，回退插件动作并解析 `{设置键}` 占位符）；不匹配返回 None
    fn try_launch(
        app: &AppHandle,
        id: &str,
        plugin_id: &str,
        action: &Option<ActionSpec>,
        target: &str,
    ) -> Option<Result<(), String>> {
        if id != target {
            return None;
        }
        Some(launch_icon(app, plugin_id, action, id))
    }

    fn launch_icon(
        app: &AppHandle,
        plugin_id: &str,
        action: &Option<ActionSpec>,
        cell_id: &str,
    ) -> Result<(), String> {
        if let Some(action) = action {
            return execute_action(action);
        }
        let plugin = plugins_list(app.clone())
            .into_iter()
            .find(|p| &p.id == plugin_id)
            .ok_or_else(|| format!("plugin not found: {plugin_id}"))?;
        // 动作支持 `{设置键}` 占位符（如 {url}）：用实例设置替换，缺省回退 manifest 默认值
        let mut resolved = plugin.clone();
        if let Some(action) = resolved.actions.first_mut() {
            if let Some(path) = action.path.as_mut() {
                *path = resolve_placeholders(app, path, cell_id, &plugin);
            }
            if let Some(cmd) = action.cmd.as_mut() {
                *cmd = resolve_placeholders(app, cmd, cell_id, &plugin);
            }
        }
        launch_plugin_action(&resolved)
    }

    for page in &layout.pages {
        for cell in page {
            match cell {
                Cell::Icon {
                    id,
                    plugin_id,
                    action,
                    ..
                } => {
                    if let Some(res) = try_launch(&app, id, plugin_id, action, &cell_id) {
                        return res;
                    }
                }
                Cell::Folder { items, .. } => {
                    for item in items {
                        if let Some(res) =
                            try_launch(&app, &item.id, &item.plugin_id, &item.action, &cell_id)
                        {
                            return res;
                        }
                    }
                }
            }
        }
    }
    Err("cell not found".into())
}

/// 动作命令/路径中的 `{设置键}` 占位符 → 实例设置（`cell.<cellId>.<key>`），
/// 未配置时回退 manifest 默认值（如 `{url}` 网页插件）。
fn resolve_placeholders(
    app: &AppHandle,
    text: &str,
    cell_id: &str,
    plugin: &PluginInfo,
) -> String {
    let mut out = text.to_string();
    for setting in &plugin.settings {
        let key = &setting.key;
        let ph = format!("{{{key}}}");
        if !out.contains(&ph) {
            continue;
        }
        let value = crate::config::config_get(app.clone(), format!("cell.{cell_id}.{key}"))
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .filter(|s| !s.is_empty())
            .or_else(|| {
                setting
                    .default
                    .as_ref()
                    .and_then(|d| d.as_str().map(|s| s.to_string()))
            })
            .unwrap_or_default();
        out = out.replace(&ph, &value);
    }
    out
}

