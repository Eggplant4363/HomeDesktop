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
                    // 动作支持 `{设置键}` 占位符（如 {url}）：用实例设置替换，缺省回退 manifest 默认值
                    let mut resolved = plugin.clone();
                    if let Some(action) = resolved.actions.first_mut() {
                        if let Some(path) = action.path.as_mut() {
                            *path = resolve_placeholders(&app, path, &cell_id, &plugin);
                        }
                        if let Some(cmd) = action.cmd.as_mut() {
                            *cmd = resolve_placeholders(&app, cmd, &cell_id, &plugin);
                        }
                    }
                    return launch_plugin_action(&resolved);
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

/// 获取网页标题（网页快捷方式：图标标签自动显示站点标题）。失败/无标题返回 None。
#[tauri::command]
pub fn web_fetch_title(url: String) -> Result<Option<String>, String> {
    let resp = fetch_agent()
        .get(&url)
        .set("User-Agent", FETCH_UA)
        .timeout(std::time::Duration::from_secs(8))
        .call()
        .map_err(|e| format!("请求失败: {e}"))?;
    let mut body = resp.into_string().map_err(|e| e.to_string())?;
    body.truncate(512 * 1024); // 只解析前 512KB
    Ok(extract_html_title(&body))
}

/// 从 HTML 提取 <title>（大小写不敏感、跨行、简单实体解码、折叠空白、限长 60 字符）
fn extract_html_title(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let start = lower.find("<title")?;
    let gt = lower[start..].find('>')? + start + 1;
    let end = lower[gt..].find("</title")? + gt;
    let mut t = html[gt..end].trim().to_string();
    for (from, to) in [
        ("&amp;", "&"),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&#39;", "'"),
        ("&nbsp;", " "),
    ] {
        t = t.replace(from, to);
    }
    let collapsed: String = t.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return None;
    }
    Some(collapsed.chars().take(60).collect())
}

/// 获取网页声明的图标 URL（解析 `<link rel="icon|shortcut icon|apple-touch-icon">` 并转绝对地址）。
/// 获取网页图标为 **data URL**（WebView 加载远程图不跳过证书校验，自签证书站点
/// 的图标必须在 Rust 侧免校验抓取后转 data: 才能显示）。失败返回 None → 前端回退 emoji。
#[tauri::command]
pub fn web_fetch_icon(url: String) -> Result<Option<String>, String> {
    use std::io::Read;
    const MAX_ICON: usize = 300 * 1024;

    let agent = fetch_agent();
    // 1) 拉页面 → 找声明的图标 URL（优先 link rel=icon，其次 /favicon.ico、/favicon.png）
    let icon_url = {
        let resp = agent
            .get(&url)
            .set("User-Agent", FETCH_UA)
            .timeout(std::time::Duration::from_secs(8))
            .call()
            .map_err(|e| format!("请求失败: {e}"))?;
        let mut body = resp.into_string().map_err(|e| e.to_string())?;
        body.truncate(512 * 1024);
        extract_html_icon(&body, &url)
            .or_else(|| resolve_url(&url, "/favicon.ico"))
            .or_else(|| resolve_url(&url, "/favicon.png"))
    };
    let Some(icon_url) = icon_url else { return Ok(None) };

    // 2) 抓取图标字节（免证书校验）→ data URL
    let resp = agent
        .get(&icon_url)
        .set("User-Agent", FETCH_UA)
        .timeout(std::time::Duration::from_secs(8))
        .call()
        .map_err(|e| format!("请求失败: {e}"))?;
    let ctype = resp
        .header("Content-Type")
        .unwrap_or("image/x-icon")
        .split(';')
        .next()
        .unwrap_or("image/x-icon")
        .trim()
        .to_string();
    if !ctype.starts_with("image/") {
        return Ok(None);
    }
    let mut bytes: Vec<u8> = Vec::new();
    resp.into_reader()
        .take((MAX_ICON + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("图标读取失败: {e}"))?;
    if bytes.is_empty() || bytes.len() > MAX_ICON {
        return Ok(None);
    }
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(Some(format!("data:{ctype};base64,{b64}")))
}

/// 抓取用的浏览器 UA（部分站点对非浏览器 UA 返回 403）
const FETCH_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36";

/// 标题/图标抓取 agent：跳过证书校验（自签证书站点如 OpenWrt 路由器可访问）
fn fetch_agent() -> ureq::Agent {
    // 显式指定 ring provider（避免与 aws-lc-rs 同时启用导致进程级 provider 二义性 panic）
    let provider = rustls::crypto::ring::default_provider();
    let config = rustls::ClientConfig::builder_with_provider(std::sync::Arc::new(provider))
        .with_safe_default_protocol_versions()
        .expect("默认 TLS 版本可用")
        .dangerous()
        .with_custom_certificate_verifier(std::sync::Arc::new(AcceptAllVerifier))
        .with_no_client_auth();
    ureq::AgentBuilder::new()
        .tls_config(std::sync::Arc::new(config))
        .build()
}

/// 接受任意证书的验证器（仅用于标题/图标抓取）
#[derive(Debug)]
struct AcceptAllVerifier;

impl rustls::client::danger::ServerCertVerifier for AcceptAllVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// 从 HTML 提取声明的图标地址（优先 apple-touch-icon > shortcut icon > icon），转绝对 URL
fn extract_html_icon(html: &str, page_url: &str) -> Option<String> {
    let lower = html.to_lowercase();
    for rel in ["apple-touch-icon", "shortcut icon", "icon"] {
        if let Some(href) = find_link_href(&lower, html, rel) {
            if let Some(abs) = resolve_url(page_url, &href) {
                return Some(abs);
            }
        }
    }
    None
}

/// 在 HTML 里找第一个包含指定 rel 的 `<link>` 标签，提取其 href 值（大小写不敏感）
fn find_link_href(lower: &str, html: &str, rel: &str) -> Option<String> {
    let mut from = 0;
    while let Some(li) = lower[from..].find("<link") {
        let start = from + li;
        let tag_end = lower[start..].find('>').map(|i| start + i).unwrap_or(lower.len());
        if lower[start..tag_end].contains(rel) {
            let tag = &html[start..tag_end];
            if let Some(hi) = lower[start..tag_end].find("href=") {
                let after = &tag[hi + 5..];
                let first = after.chars().next();
                let value: String = if first == Some('"') {
                    let end = after[1..].find('"')?;
                    after[1..=end].to_string()
                } else if first == Some('\'') {
                    let end = after[1..].find('\'')?;
                    after[1..=end].to_string()
                } else {
                    after.split_whitespace().next()?.to_string()
                };
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
        from = tag_end;
    }
    None
}

/// 相对 href → 绝对 URL（支持 // 协议相对、/ 根相对、裸相对）
fn resolve_url(base: &str, href: &str) -> Option<String> {
    let href = href.trim();
    if href.starts_with("http://") || href.starts_with("https://") {
        return Some(href.to_string());
    }
    let (scheme, rest) = base.split_once("://")?;
    let host = rest.split(['/', '?', '#']).next()?;
    let origin = format!("{scheme}://{host}");
    if let Some(p) = href.strip_prefix("//") {
        return Some(format!("{scheme}:{p}"));
    }
    if let Some(p) = href.strip_prefix('/') {
        return Some(format!("{origin}/{p}"));
    }
    Some(format!("{origin}/{href}"))
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
