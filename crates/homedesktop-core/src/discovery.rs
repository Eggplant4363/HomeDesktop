//! 插件发现：扫描插件目录、解析 manifest（含 provider 子插件展平）。

use crate::schema::*;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

// ---------- 插件发现 ----------

/// 扫描单个插件目录，将合法 manifest 加入 out（按 id 去重，后者优先级更高）
pub fn scan_plugin_dir(dir: &Path, seen: &mut HashSet<String>, out: &mut Vec<PluginInfo>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let manifest_path = entry.path().join("manifest.json");
        if !manifest_path.is_file() {
            continue;
        }
        match fs::read_to_string(&manifest_path) {
            Ok(text) => match serde_json::from_str::<Manifest>(&text) {
                Ok(m) => {
                    let dir = entry.path().to_string_lossy().into_owned();
                    if !m.plugins.is_empty() {
                        // 提供商容器：本身作为 plugin_type=provider 的条目（前端用于二级菜单分组 + 共享设置），
                        // 子插件逐个展平并带上 provider_id/provider_name
                        if seen.insert(m.id.clone()) {
                            out.push(PluginInfo {
                                id: m.id.clone(),
                                name: m.name.clone(),
                                version: m.version.clone(),
                                plugin_type: "provider".into(),
                                emoji: m.emoji.clone(),
                                actions: Vec::new(),
                                widget_component: None,
                                widget_file: None,
                                widget_element: None,
                                sizes: Vec::new(),
                                settings: m.settings.clone(),
                                builtin: false,
                                provider_id: None,
                                provider_name: None,
                                domain: None,
                                dir: Some(dir.clone()),
                            });
                        }
                        for sub in m.plugins {
                            if seen.insert(sub.id.clone()) {
                                out.push(PluginInfo {
                                    id: sub.id,
                                    name: sub.name,
                                    version: m.version.clone(),
                                    plugin_type: sub.plugin_type,
                                    emoji: sub.emoji,
                                    actions: sub.actions,
                                    widget_component: sub.widget_component,
                                    widget_file: sub.widget_file,
                                    widget_element: sub.widget_element,
                                    sizes: sub.sizes,
                                    settings: sub.settings,
                                    builtin: false,
                                    provider_id: Some(m.id.clone()),
                                    provider_name: Some(m.name.clone()),
                                    domain: sub.domain,
                                    dir: Some(dir.clone()),
                                });
                            }
                        }
                    } else if seen.insert(m.id.clone()) {
                        out.push(PluginInfo {
                            id: m.id,
                            name: m.name,
                            version: m.version,
                            plugin_type: m.plugin_type,
                            emoji: m.emoji,
                            actions: m.actions,
                            widget_component: m.widget_component,
                            widget_file: m.widget_file,
                            widget_element: m.widget_element,
                            sizes: m.sizes,
                            settings: m.settings,
                            builtin: false,
                            provider_id: None,
                            provider_name: None,
                            domain: m.domain,
                            dir: Some(dir),
                        });
                    }
                }
                Err(e) => eprintln!(
                    "[homedesktop] invalid plugin manifest {}: {e}",
                    manifest_path.display()
                ),
            },
            Err(e) => eprintln!(
                "[homedesktop] read manifest {} failed: {e}",
                manifest_path.display()
            ),
        }
    }
}

/// 按优先级顺序扫描多个插件目录（纯函数，便于单测）
pub fn collect_plugins(dirs: &[PathBuf]) -> Vec<PluginInfo> {
    collect_plugins_with_builtin(dirs)
        .into_iter()
        .map(|(p, _)| p)
        .collect()
}

/// 同 collect_plugins，但返回 (PluginInfo, 是否内置)。
/// 内置 = 来自非用户数据目录（索引 > 0）的插件（M11：内置不可卸载）。
pub fn collect_plugins_with_builtin(dirs: &[PathBuf]) -> Vec<(PluginInfo, bool)> {
    let mut plugins: Vec<PluginInfo> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut builtins: Vec<bool> = Vec::new();
    for (i, dir) in dirs.iter().enumerate() {
        if dir.is_dir() {
            let before = plugins.len();
            scan_plugin_dir(dir, &mut seen, &mut plugins);
            for _ in before..plugins.len() {
                builtins.push(i != 0);
            }
        }
    }
    plugins.into_iter().zip(builtins).collect()
}

