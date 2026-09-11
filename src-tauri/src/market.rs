//! 插件市场：本地 market/ 扫描安装 + 远程 index.json 拉取与下载安装（阶段4 从 plugins.rs 拆出）。

use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use homedesktop_core::{collect_plugins_with_builtin, PluginInfo};
use crate::plugins::plugins_list;
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

// ---------- 在线市场（远程仓库）：拉取 index.json + 下载 zip 安装 ----------

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteMarketItem {
    pub id: String,
    pub name: String,
    pub version: String,
    pub plugin_type: String,
    pub emoji: Option<String>,
    /// zip 文件名（相对 base 下载）
    pub file: String,
    /// 字节数（索引提供时显示）
    pub size: Option<u64>,
    pub description: Option<String>,
    /// 是否已安装
    pub installed: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteMarket {
    pub base: String,
    pub items: Vec<RemoteMarketItem>,
}

/// 在线市场索引 JSON 结构（仓库 market/index.json）
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteIndex {
    /// zip 下载基础 URL（末尾带 /）
    base: String,
    #[serde(default)]
    plugins: Vec<RemoteIndexItem>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteIndexItem {
    id: String,
    name: String,
    version: String,
    #[serde(rename = "pluginType", default)]
    plugin_type: String,
    #[serde(default)]
    emoji: Option<String>,
    file: String,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    description: Option<String>,
}

/// 下载进度（经 IPC Channel 推送给前端，用于在线安装进度条）
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    /// 插件包文件名
    pub file: String,
    /// 已下载字节数
    pub received: u64,
    /// 总字节数（无 Content-Length 时为 None，前端显示不确定进度）
    pub total: Option<u64>,
}

/// 拉取在线市场索引（异步：下载在阻塞线程池执行，不卡 UI；自动应用代理设置）
#[tauri::command]
pub async fn market_remote_list(app: AppHandle, url: String) -> Result<RemoteMarket, String> {
    let agent = market_agent(&app)?;
    let body = tauri::async_runtime::spawn_blocking(move || fetch_text(&agent, &url))
        .await
        .map_err(|e| format!("索引拉取任务失败: {e}"))??;
    let index: RemoteIndex =
        serde_json::from_str(&body).map_err(|e| format!("索引 JSON 解析失败: {e}"))?;
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let items = index
        .plugins
        .into_iter()
        .map(|p| RemoteMarketItem {
            installed: data_dir.join("plugins").join(&p.id).is_dir(),
            id: p.id,
            name: p.name,
            version: p.version,
            plugin_type: p.plugin_type,
            emoji: p.emoji,
            file: p.file,
            size: p.size,
            description: p.description,
        })
        .collect();
    Ok(RemoteMarket {
        base: index.base,
        items,
    })
}

/// 从在线市场下载 zip 并安装（异步 + 进度推送；下载在阻塞线程池执行，不卡 UI）
#[tauri::command]
pub async fn market_remote_install(
    app: AppHandle,
    base: String,
    file: String,
    on_progress: tauri::ipc::Channel<DownloadProgress>,
) -> Result<PluginInfo, String> {
    // 文件名防穿越：只允许普通文件名
    let file_name = std::path::Path::new(&file)
        .file_name()
        .and_then(|s| s.to_str())
        .filter(|s| s.ends_with(".zip") && !s.contains('/') && !s.contains('\\'))
        .ok_or_else(|| "非法的插件包文件名".to_string())?
        .to_string();

    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let market_dir = data_dir.join("market");
    std::fs::create_dir_all(&market_dir).map_err(|e| e.to_string())?;
    let zip_path = market_dir.join(&file_name);

    // 修复：base 可能带/不带结尾斜杠，统一保证只有一个分隔符
    let url = format!("{}/{}", base.trim_end_matches('/'), file_name);
    crate::log::info(&format!("在线市场下载: {url}"));

    let agent = market_agent(&app)?;
    let result = tauri::async_runtime::spawn_blocking(move || -> Result<PluginInfo, String> {
        download_to_with_progress(&agent, &url, &zip_path, &on_progress)?;
        crate::log::info(&format!(
            "下载完成: {} ({} bytes)",
            file_name,
            zip_path.metadata().map(|m| m.len()).unwrap_or(0)
        ));
        // 复用本地安装逻辑（校验 manifest + 解压到 plugins/<id>/）
        plugins_install(app, zip_path.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| format!("下载任务失败: {e}"))??;

    crate::log::info(&format!("在线市场安装成功: {} v{}", result.name, result.version));
    Ok(result)
}

/// 按配置构造在线市场 HTTP agent（代理设置：proxy.mode=none|http|socks5 + host/port/username/password）
fn market_agent(app: &AppHandle) -> Result<ureq::Agent, String> {
    let mode = crate::config::get_str(app, "proxy.mode").unwrap_or_else(|| "none".into());
    if mode.is_empty() || mode == "none" {
        return Ok(ureq::AgentBuilder::new().build());
    }
    let host = crate::config::get_str(app, "proxy.host").unwrap_or_default();
    let port = crate::config::get_str(app, "proxy.port").unwrap_or_default();
    if host.trim().is_empty() || port.trim().is_empty() {
        return Err("代理未配置完整（缺少地址或端口）".to_string());
    }
    let user = crate::config::get_str(app, "proxy.username").unwrap_or_default();
    let pass = crate::config::get_str(app, "proxy.password").unwrap_or_default();
    let auth = if !user.is_empty() {
        format!("{user}:{pass}@")
    } else {
        String::new()
    };
    let scheme = if mode == "socks5" { "socks5" } else { "http" };
    let url = format!("{scheme}://{auth}{}:{}", host.trim(), port.trim());
    crate::log::info(&format!("使用代理: {scheme}://{host}:{port}"));
    let proxy = ureq::Proxy::new(&url).map_err(|e| format!("代理配置无效: {e}"))?;
    Ok(ureq::AgentBuilder::new().proxy(proxy).build())
}

/// GET 文本（超时 30s）
fn fetch_text(agent: &ureq::Agent, url: &str) -> Result<String, String> {
    let resp = agent
        .get(url)
        .timeout(std::time::Duration::from_secs(30))
        .call()
        .map_err(|e| format!("请求失败: {e}"))?;
    resp.into_string().map_err(|e| format!("读取响应失败: {e}"))
}

/// 下载到本地文件并推送进度（最多 64MB，防异常大包）
fn download_to_with_progress(
    agent: &ureq::Agent,
    url: &str,
    dest: &std::path::Path,
    progress: &tauri::ipc::Channel<DownloadProgress>,
) -> Result<(), String> {
    use std::io::Read;
    use std::io::Write;
    const MAX: u64 = 64 * 1024 * 1024;
    const CHUNK: usize = 64 * 1024;
    let file_name = dest
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    let resp = agent
        .get(url)
        .timeout(std::time::Duration::from_secs(60))
        .call()
        .map_err(|e| format!("下载失败: {e}"))?;
    let total = resp
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok());

    let mut reader = resp.into_reader().take(MAX + 1);
    let mut out = std::fs::File::create(dest).map_err(|e| format!("写文件失败: {e}"))?;
    let mut received: u64 = 0;
    let mut buf = vec![0u8; CHUNK];
    loop {
        let n = reader.read(&mut buf).map_err(|e| format!("下载中断: {e}"))?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n]).map_err(|e| format!("写文件失败: {e}"))?;
        received += n as u64;
        // 推送进度（发送失败不影响下载）
        let _ = progress.send(DownloadProgress {
            file: file_name.clone(),
            received,
            total,
        });
    }
    if received > MAX {
        let _ = std::fs::remove_file(dest);
        return Err("插件包超过 64MB 限制".to_string());
    }
    Ok(())
}
