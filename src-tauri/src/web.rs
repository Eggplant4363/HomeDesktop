//! 网页抓取：标题与站点图标（失败返回 None，前端回退 emoji）。
//! 阶段4 从 plugins.rs 拆出。

use std::path::PathBuf;
use tauri::{AppHandle, Manager};
#[tauri::command]
pub async fn web_fetch_title(url: String) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let resp = fetch_agent()
            .get(&url)
            .set("User-Agent", FETCH_UA)
            .timeout(std::time::Duration::from_secs(8))
            .call()
            .map_err(|e| format!("请求失败: {e}"))?;
        let mut body = resp.into_string().map_err(|e| e.to_string())?;
        truncate_utf8(&mut body, 512 * 1024); // 只解析前 512KB（字符边界安全截断）
        Ok(extract_html_title(&body))
    })
    .await
    .map_err(|e| format!("线程错误: {e}"))?
}

/// 按字节上限截断字符串（保证不落在 UTF-8 字符中间，避免 truncate panic）
fn truncate_utf8(s: &mut String, max_bytes: usize) {
    if s.len() > max_bytes {
        let cut = s.floor_char_boundary(max_bytes);
        s.truncate(cut);
    }
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

/// 获取网页图标为 **data URL**（WebView 加载远程图不跳过证书校验，自签证书站点
/// 的图标必须在 Rust 侧免校验抓取后转 data: 才能显示）。
/// **磁盘缓存**：按网址哈希存 `icons/web/<hash>.txt`，抓一次永久生效（不再每次联网）。
/// 失败返回 None → 前端回退 emoji。
/// async + spawn_blocking：避免同步阻塞 Tauri 主线程导致 UI 卡住。
#[tauri::command]
pub async fn web_fetch_icon(app: AppHandle, url: String) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
    use std::io::Read;
    // 上限 4MB（部分站点声明的图标是大图，如 cchong.cc 2.4MB）
    const MAX_ICON: usize = 4 * 1024 * 1024;

    // 0) 磁盘缓存命中：直接返回（"存一次"，无需重新抓取）
    if let Some(cache) = web_icon_cache_path(&app, &url) {
        if cache.is_file() {
            if let Ok(data) = std::fs::read_to_string(&cache) {
                return Ok(Some(data));
            }
        }
    }

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
        truncate_utf8(&mut body, 512 * 1024);
        extract_html_icon(&body, &url)
            .or_else(|| resolve_url(&url, "/favicon.ico"))
            .or_else(|| resolve_url(&url, "/favicon.png"))
    };
    let Some(icon_url) = icon_url else {
        crate::log::warn(&format!("网站图标: 未找到图标来源 ({url})"));
        return Ok(None);
    };

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
    if bytes.is_empty() {
        return Ok(None);
    }
    if bytes.len() > MAX_ICON {
        crate::log::warn(&format!(
            "网站图标过大已跳过: {} ({} bytes > {}KB)",
            icon_url,
            bytes.len(),
            MAX_ICON / 1024
        ));
        return Ok(None);
    }
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let data_url = format!("data:{ctype};base64,{b64}");
    crate::log::info(&format!(
        "网站图标: {} ({ctype}, {} bytes)，已缓存",
        icon_url,
        bytes.len()
    ));
    if let Some(cache) = web_icon_cache_path(&app, &url) {
        if let Some(parent) = cache.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&cache, &data_url);
    }
    Ok(Some(data_url))
    })
    .await
    .map_err(|e| format!("线程错误: {e}"))?
}

/// 网站图标磁盘缓存路径（icons/web/<url hash>.txt）
fn web_icon_cache_path(app: &AppHandle, url: &str) -> Option<std::path::PathBuf> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let dir = app.path().app_data_dir().ok()?.join("icons").join("web");
    let mut h = DefaultHasher::new();
    url.hash(&mut h);
    Some(dir.join(format!("{:016x}.txt", h.finish())))
}

/// 抓取用的浏览器 UA（部分站点对非浏览器 UA 返回 403）
pub(crate) const FETCH_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36";

/// 标题/图标抓取 agent：跳过证书校验（自签证书站点如 OpenWrt 路由器可访问）
pub(crate) fn fetch_agent() -> ureq::Agent {
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

