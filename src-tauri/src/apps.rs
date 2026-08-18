//! 应用抽屉：扫描系统已安装应用（注册表 App Paths + 开始菜单快捷方式 .lnk）

use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tauri::Manager;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub path: String,
}

#[tauri::command]
pub fn apps_scan() -> Vec<AppInfo> {
    let mut apps: Vec<AppInfo> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    scan_registry_app_paths(&mut apps, &mut seen);

    for dir in start_menu_dirs() {
        scan_lnk_dir(&dir, &mut apps, &mut seen);
    }

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

/// 注册表 App Paths：HKLM + HKCU（winreg 仅 Windows；非 Windows 平台无注册表扫描）
#[cfg(windows)]
fn scan_registry_app_paths(out: &mut Vec<AppInfo>, seen: &mut HashSet<String>) {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    let roots = [
        (
            HKEY_CURRENT_USER,
            "Software\\Microsoft\\Windows\\CurrentVersion\\App Paths",
        ),
        (
            HKEY_LOCAL_MACHINE,
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\App Paths",
        ),
    ];
    for (root, sub) in roots {
        let Ok(key) = RegKey::predef(root).open_subkey(sub) else {
            continue;
        };
        for entry in key.enum_keys().flatten() {
            let Ok(sub_key) = key.open_subkey(&entry) else {
                continue;
            };
            let Ok(path) = sub_key.get_value::<String, _>("") else {
                continue;
            };
            let p = path.trim().trim_matches('"').to_string();
            if p.is_empty() || !seen.insert(p.to_lowercase()) {
                continue;
            }
            out.push(AppInfo {
                name: display_name(&entry, &p),
                path: p,
            });
        }
    }
}

/// 非 Windows 平台：无注册表扫描（空实现）
#[cfg(not(windows))]
fn scan_registry_app_paths(_out: &mut Vec<AppInfo>, _seen: &mut HashSet<String>) {}

/// "chrome.exe" → "Chrome"；退化用 exe 文件名
fn display_name(entry: &str, path: &str) -> String {
    let stem = entry
        .rsplit('.')
        .nth(1)
        .unwrap_or(entry)
        .replace(['_', '-'], " ");
    if stem.trim().is_empty() {
        Path::new(path)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| entry.to_string())
    } else {
        stem
    }
}

fn start_menu_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(appdata) = std::env::var_os("APPDATA") {
        dirs.push(
            PathBuf::from(appdata).join("Microsoft\\Windows\\Start Menu\\Programs"),
        );
    }
    if let Some(programdata) = std::env::var_os("PROGRAMDATA") {
        dirs.push(
            PathBuf::from(programdata).join("Microsoft\\Windows\\Start Menu\\Programs"),
        );
    }
    dirs
}

fn scan_lnk_dir(dir: &Path, out: &mut Vec<AppInfo>, seen: &mut HashSet<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_lnk_dir(&path, out, seen);
            continue;
        }
        if path.extension().map(|e| e.to_string_lossy().to_lowercase()) != Some("lnk".into()) {
            continue;
        }
        if let Some(target) = resolve_lnk(&path) {
            if seen.insert(target.to_lowercase()) {
                let name = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if !name.trim().is_empty() {
                    out.push(AppInfo {
                        name,
                        path: target,
                    });
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn resolve_lnk(path: &Path) -> Option<String> {
    use windows::core::{GUID, Interface, PCWSTR};
    use windows::Win32::{
        Storage::FileSystem::WIN32_FIND_DATAW,
        System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER, IPersistFile, STGM_READ},
        UI::Shell::IShellLinkW,
    };

    // CLSID_ShellLink {00021401-0000-0000-C000-000000000046}（0.61 未生成常量，手动定义）
    const CLSID_SHELL_LINK: GUID = GUID::from_u128(0x00021401_0000_0000_C000_000000000046);

    unsafe {
        let shell_link: IShellLinkW =
            CoCreateInstance(&CLSID_SHELL_LINK, None, CLSCTX_INPROC_SERVER).ok()?;
        let persist: IPersistFile = shell_link.cast().ok()?;
        let wide: Vec<u16> = path
            .to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        persist.Load(PCWSTR(wide.as_ptr()), STGM_READ).ok()?;
        let mut buf = [0u16; 1024];
        let mut fd = std::mem::MaybeUninit::<WIN32_FIND_DATAW>::uninit();
        shell_link
            .GetPath(&mut buf, fd.as_mut_ptr(), 0)
            .ok()?;
        let target = String::from_utf16_lossy(&buf);
        let target = target.trim_end_matches('\0').to_string();
        if target.is_empty() {
            None
        } else {
            Some(target)
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn resolve_lnk(_path: &Path) -> Option<String> {
    None
}

// ---------- 系统应用图标 ----------

/// PNG → base64 data URL
fn to_data_url(png: Vec<u8>) -> String {
    use base64::Engine;
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    )
}

/// 图标磁盘缓存路径（app 数据目录 icons/<path hash>.png）
fn icon_cache_path(app: &tauri::AppHandle, path: &str) -> Option<PathBuf> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let dir = app.path().app_data_dir().ok()?.join("icons");
    let mut h = DefaultHasher::new();
    path.hash(&mut h);
    Some(dir.join(format!("{:016x}.png", h.finish())))
}

/// 获取应用图标的 data URL（PNG base64，可直接用于 <img src>）。
/// 非 Windows 平台或提取失败时返回 None（前端回退首字母头像）。
/// async：GDI 提取放阻塞线程池，不卡 UI；磁盘缓存避免重启后重复提取。
#[tauri::command]
pub async fn app_icon(app: tauri::AppHandle, path: String) -> Option<String> {
    crate::log::debug(&format!("app_icon: {path}"));

    // 1) 磁盘缓存命中：直接读文件（秒出）
    let cache_path = icon_cache_path(&app, &path)?;
    if cache_path.is_file() {
        return to_data_url(std::fs::read(&cache_path).ok()?).into();
    }

    // 2) 未命中：阻塞线程池提取（不占 UI 主线程），成功后写缓存
    let png = tauri::async_runtime::spawn_blocking(move || extract_icon_png(&path))
        .await
        .ok()??;
    if let Some(parent) = cache_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&cache_path, &png);
    Some(to_data_url(png))
}

/// 直接启动一个可执行文件/URL（M10 全局搜索面板"系统应用"结果用；不经过桌面布局）
#[tauri::command]
pub fn launch_path(path: String) -> Result<(), String> {
    use homedesktop_core::{execute_action, ActionKind, ActionSpec};
    crate::log::info(&format!("launch_path: {path}"));
    execute_action(&ActionSpec {
        kind: ActionKind::App,
        path: Some(path),
        cmd: None,
    })
}

/// Windows：SHGetFileInfo 取文件关联图标（HICON）→ GetDIBits 取 32bpp BGRA 像素
/// （+ 1bpp 掩码补 alpha）→ homedesktop-core 编码 PNG。
#[cfg(target_os = "windows")]
fn extract_icon_png(path: &str) -> Option<Vec<u8>> {
    use homedesktop_core::encode_rgba_png;
    use windows::core::PCWSTR;
    use windows::Win32::Graphics::Gdi::{
        BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC,
        DeleteObject, GetDIBits, HGDIOBJ, HDC, HBITMAP,
    };
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

    unsafe {
        // 1) 取文件关联的大图标（32×32）
        let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let mut sfi = std::mem::zeroed::<SHFILEINFOW>();
        if SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut sfi as *mut SHFILEINFOW),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        ) == 0
        {
            return None;
        }
        let icon: HICON = sfi.hIcon;
        if icon.0.is_null() {
            return None;
        }

        // 2) 取颜色位图 + 掩码位图
        let mut ii = std::mem::zeroed::<ICONINFO>();
        if GetIconInfo(icon, &mut ii).is_err() {
            let _ = DestroyIcon(icon);
            return None;
        }
        let hbm_color: HBITMAP = ii.hbmColor;
        let hbm_mask: HBITMAP = ii.hbmMask;

        let hdc: HDC = CreateCompatibleDC(None);
        if hdc.0.is_null() {
            let _ = DestroyIcon(icon);
            return None;
        }

        let mut result: Option<Vec<u8>> = None;

        // 3) 先查询位图尺寸（lpvBits = None）
        let mut bmi = std::mem::zeroed::<BITMAPINFO>();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        if GetDIBits(hdc, hbm_color, 0, 0, None, &mut bmi, DIB_RGB_COLORS) != 0 {
            let w = bmi.bmiHeader.biWidth.max(0) as u32;
            let h = bmi.bmiHeader.biHeight.abs() as u32;
            if w > 0 && h > 0 && w <= 512 && h <= 512 {
                // 4) 取 32bpp 顶向下 BGRA 像素
                let mut color_buf = vec![0u8; (w * h * 4) as usize];
                bmi.bmiHeader.biHeight = -(h as i32);
                bmi.bmiHeader.biPlanes = 1;
                bmi.bmiHeader.biBitCount = 32;
                bmi.bmiHeader.biCompression = BI_RGB.0;
                bmi.bmiHeader.biSizeImage = 0;
                if GetDIBits(
                    hdc,
                    hbm_color,
                    0,
                    h,
                    Some(color_buf.as_mut_ptr() as *mut _),
                    &mut bmi,
                    DIB_RGB_COLORS,
                ) != 0
                {
                    // 5) 颜色位图自身带 alpha（Vista+ 图标）则直接用，否则用 1bpp 掩码补 alpha
                    let has_alpha = color_buf.chunks_exact(4).any(|p| p[3] != 0);
                    let mut mask_buf: Vec<u8> = Vec::new();
                    let mut mask_ok = false;
                    if !has_alpha {
                        let mask_row = ((w + 7) / 8) as usize;
                        mask_buf = vec![0u8; mask_row * h as usize];
                        let mut bmi_mask = std::mem::zeroed::<BITMAPINFO>();
                        bmi_mask.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                        bmi_mask.bmiHeader.biWidth = w as i32;
                        bmi_mask.bmiHeader.biHeight = -(h as i32);
                        bmi_mask.bmiHeader.biPlanes = 1;
                        bmi_mask.bmiHeader.biBitCount = 1;
                        bmi_mask.bmiHeader.biCompression = BI_RGB.0;
                        mask_ok = GetDIBits(
                            hdc,
                            hbm_mask,
                            0,
                            h,
                            Some(mask_buf.as_mut_ptr() as *mut _),
                            &mut bmi_mask,
                            DIB_RGB_COLORS,
                        ) != 0;
                    }

                    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
                    for y in 0..h as usize {
                        for x in 0..w as usize {
                            let idx = (y * w as usize + x) * 4;
                            let alpha = if has_alpha {
                                color_buf[idx + 3]
                            } else if mask_ok {
                                let byte = mask_buf[y * (((w + 7) / 8) as usize) + (x >> 3)];
                                if (byte >> (7 - (x & 7))) & 1 == 1 {
                                    0
                                } else {
                                    255
                                }
                            } else {
                                255
                            };
                            // BGRA → RGBA
                            rgba.extend_from_slice(&[
                                color_buf[idx + 2],
                                color_buf[idx + 1],
                                color_buf[idx],
                                alpha,
                            ]);
                        }
                    }
                    result = encode_rgba_png(w, h, &rgba).ok();
                }
            }
        }

        // 6) 清理句柄
        let _ = DeleteDC(hdc);
        let _ = DeleteObject(HGDIOBJ(hbm_color.0));
        let _ = DeleteObject(HGDIOBJ(hbm_mask.0));
        let _ = DestroyIcon(icon);
        result
    }
}

#[cfg(not(target_os = "windows"))]
fn extract_icon_png(_path: &str) -> Option<Vec<u8>> {
    None
}
