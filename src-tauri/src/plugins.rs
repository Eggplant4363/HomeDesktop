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

/// 按单元格 id 启动：优先用图标自带动作（应用抽屉），否则回退到插件动作
#[tauri::command]
pub fn launch_cell(app: AppHandle, cell_id: String) -> Result<(), String> {
    crate::log::info(&format!("launch_cell: {cell_id}"));
    use homedesktop_core::{execute_action, Cell};
    let layout = layout_load(app.clone()).unwrap_or_default();
    for page in &layout.pages {
        for cell in page {
            if let Cell::Icon {
                id,
                plugin_id,
                action,
                ..
            } = cell
            {
                if id == &cell_id {
                    if let Some(action) = action {
                        return execute_action(action);
                    }
                    let plugin = plugins_list(app.clone())
                        .into_iter()
                        .find(|p| &p.id == plugin_id)
                        .ok_or_else(|| format!("plugin not found: {plugin_id}"))?;
                    return launch_plugin_action(&plugin);
                }
            }
        }
    }
    Err("cell not found".into())
}

/// 插件市场 MVP：从本地 zip 安装插件包（zip 内含 manifest.json + 可选资源）
/// 解压到用户数据目录 plugins/<id>/，随后可被插件注册表发现
#[tauri::command]
pub fn plugins_install(app: AppHandle, zip_path: String) -> Result<PluginInfo, String> {
    use homedesktop_core::parse_manifest_info;
    use std::io::Read;

    let file = std::fs::File::open(&zip_path).map_err(|e| format!("打开 zip 失败: {e}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("zip 解析失败: {e}"))?;

    // 1) 校验根目录 manifest.json（独立作用域，读取后立即释放 zip 借用）
    let text = {
        let mut manifest_file = archive
            .by_name("manifest.json")
            .map_err(|_| "zip 中缺少 manifest.json".to_string())?;
        let mut t = String::new();
        manifest_file
            .read_to_string(&mut t)
            .map_err(|e| format!("读取 manifest 失败: {e}"))?;
        t
    };
    let info = parse_manifest_info(&text)?;

    // 2) 解压到 plugins/<id>/
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let plugin_dir = data_dir.join("plugins").join(&info.id);
    if plugin_dir.exists() {
        std::fs::remove_dir_all(&plugin_dir).map_err(|e| format!("清理旧版本失败: {e}"))?;
    }
    std::fs::create_dir_all(&plugin_dir).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().replace('\\', "/");
        let name = name.trim_start_matches('/').to_string();
        // 防目录穿越
        if name.is_empty() || name.contains("..") {
            continue;
        }
        let out_path = plugin_dir.join(&name);
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = std::fs::File::create(&out_path).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
    }

    // 3) 返回安装后的插件信息
    plugins_list(app)
        .into_iter()
        .find(|p| p.id == info.id)
        .ok_or_else(|| "安装完成但插件未被发现".to_string())
}

// ---------- 插件市场目录（本地市场：把 zip 放进 market/ 即可浏览安装） ----------

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketItem {
    pub file: String,
    pub id: String,
    pub name: String,
    pub version: String,
    pub plugin_type: String,
    pub emoji: Option<String>,
    /// 是否已安装（用户数据 plugins/<id> 存在）
    pub installed: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketScan {
    pub dir: String,
    pub items: Vec<MarketItem>,
}

/// 扫描市场目录（app 数据目录 market/*.zip）中的插件包
#[tauri::command]
pub fn market_scan(app: AppHandle) -> Result<MarketScan, String> {
    use homedesktop_core::parse_manifest_info;
    use std::io::Read;

    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let market_dir = data_dir.join("market");
    std::fs::create_dir_all(&market_dir).map_err(|e| e.to_string())?;

    let mut items = Vec::new();
    let Ok(entries) = std::fs::read_dir(&market_dir) else {
        return Ok(MarketScan {
            dir: market_dir.to_string_lossy().into_owned(),
            items,
        });
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e.to_string_lossy().to_lowercase()) != Some("zip".into()) {
            continue;
        }
        let Ok(file) = std::fs::File::open(&path) else {
            continue;
        };
        let Ok(mut archive) = zip::ZipArchive::new(file) else {
            continue;
        };
        let Ok(text) = (|| -> Result<String, String> {
            let mut mf = archive.by_name("manifest.json").map_err(|_| "无 manifest".to_string())?;
            let mut t = String::new();
            mf.read_to_string(&mut t).map_err(|e| e.to_string())?;
            Ok(t)
        })()
        else {
            continue;
        };
        let Ok(info) = parse_manifest_info(&text) else {
            continue;
        };
        let installed = data_dir.join("plugins").join(&info.id).is_dir();
        items.push(MarketItem {
            file: path.to_string_lossy().into_owned(),
            id: info.id,
            name: info.name,
            version: info.version,
            plugin_type: info.plugin_type,
            emoji: info.emoji,
            installed,
        });
    }
    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(MarketScan {
        dir: market_dir.to_string_lossy().into_owned(),
        items,
    })
}
