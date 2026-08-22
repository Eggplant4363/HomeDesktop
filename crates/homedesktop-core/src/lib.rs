//! HomeDesktop 纯逻辑核心：插件 manifest 解析、插件扫描、布局序列化、动作分发。
//!
//! 本 crate 不依赖 tauri / WebView2，保证可以在任意环境直接 `cargo test`。

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

// ---------- 插件 manifest ----------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActionSpec {
    pub kind: ActionKind,
    pub path: Option<String>,
    pub cmd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ActionKind {
    App,
    Command,
    /// 系统应用面板（框架内置「系统应用」插件使用）：由前端处理，后端不执行
    #[serde(rename = "system_apps")]
    SystemApps,
}

/// 提供商容器的子插件清单（二级菜单：一个提供商可提供多个插件，如 HomeAssistant 提供灯/开关/传感器）
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct SubManifest {
    id: String,
    name: String,
    #[serde(rename = "type")]
    plugin_type: String,
    #[serde(default)]
    emoji: Option<String>,
    #[serde(default)]
    actions: Vec<ActionSpec>,
    #[serde(default)]
    widget_component: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    widget_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    widget_element: Option<String>,
    #[serde(default)]
    sizes: Vec<Size>,
    #[serde(default)]
    settings: Vec<SettingSpec>,
    /// 子插件专属字段（如 HA 的实体域：light/switch/sensor）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    domain: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    id: String,
    name: String,
    version: String,
    #[serde(rename = "type")]
    plugin_type: String,
    emoji: Option<String>,
    /// 提供商容器无顶层 actions（子插件各自带），故允许缺省
    #[serde(default)]
    actions: Vec<ActionSpec>,
    /// 提供商容器：子插件数组（type=provider 时有效）
    #[serde(default)]
    plugins: Vec<SubManifest>,
    /// 实体域（如 HA 子插件：light/switch/sensor）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    domain: Option<String>,
    /// type=widget 时前端渲染哪个小组件（"clock" | "weather" …；"__plugin__"=插件自带组件）
    widget_component: Option<String>,
    /// 插件自带小组件（M16）：插件目录内的 JS 文件（自定义元素定义）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    widget_file: Option<String>,
    /// 插件自带小组件（M16）：自定义元素标签名
    #[serde(default, skip_serializing_if = "Option::is_none")]
    widget_element: Option<String>,
    /// 插件声明支持的尺寸集合（小米/安卓 widget 设计：切换大小只能选这些档）
    #[serde(default)]
    sizes: Vec<Size>,
    /// 插件声明可配置项（框架提供统一设置菜单）
    #[serde(default)]
    settings: Vec<SettingSpec>,
}

/// 插件设置项声明（框架统一菜单渲染用）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SettingSpec {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub setting_type: SettingType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<SettingOption>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SettingType {
    Text,
    Number,
    Select,
    Toggle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SettingOption {
    pub label: String,
    pub value: serde_json::Value,
}

/// 暴露给前端的插件信息（camelCase 序列化）
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub plugin_type: String,
    pub emoji: Option<String>,
    pub actions: Vec<ActionSpec>,
    /// type=widget 时前端渲染哪个小组件（"clock" | "weather" …；"__plugin__"=插件自带组件）
    pub widget_component: Option<String>,
    /// 插件自带小组件（M16）：插件目录内的 JS 文件（自定义元素定义）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub widget_file: Option<String>,
    /// 插件自带小组件（M16）：自定义元素标签名
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub widget_element: Option<String>,
    /// 插件支持的尺寸集合（空 = 未声明，前端回退默认）
    pub sizes: Vec<Size>,
    /// 插件可配置项声明（统一设置菜单）
    pub settings: Vec<SettingSpec>,
    /// 是否内置插件（非用户数据目录安装，不可卸载；M11）
    #[serde(default, skip_serializing_if = "is_false")]
    pub builtin: bool,
    /// 提供商 id（二级菜单分组；provider 插件自身的子插件带此字段）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    /// 提供商名称
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_name: Option<String>,
    /// 实体域（HomeAssistant 等子插件专用）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// 插件目录绝对路径（M16：加载插件自带 JS 用 asset 协议）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// 插件清单校验信息（插件市场安装前校验用）
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ManifestInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub plugin_type: String,
    pub widget_component: Option<String>,
    pub emoji: Option<String>,
}

/// 解析并校验 manifest 文本
pub fn parse_manifest_info(text: &str) -> Result<ManifestInfo, String> {
    let m: Manifest = serde_json::from_str(text).map_err(|e| format!("manifest 解析失败: {e}"))?;
    if m.id.trim().is_empty() || m.name.trim().is_empty() {
        return Err("manifest 缺少 id/name".to_string());
    }
    Ok(ManifestInfo {
        id: m.id,
        name: m.name,
        version: m.version,
        plugin_type: m.plugin_type,
        widget_component: m.widget_component,
        emoji: m.emoji,
    })
}

// ---------- 布局（schema v2：单元格 = 图标 | 文件夹） ----------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Size {
    pub w: u32,
    pub h: u32,
}

/// 文件夹内的图标条目
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IconItem {
    pub id: String,
    pub plugin_id: String,
    pub title: String,
    pub size: Size,
    /// 图标自带的启动动作（如"应用抽屉"扫描出的应用）；缺省时按 plugin_id 查插件
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<ActionSpec>,
    /// 自定义显示（M9）：覆盖插件 emoji / 背景色 / 借用系统应用图标
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_path: Option<String>,
    /// 自由摆放（v3）：文件夹内网格坐标（缺省由迁移分配）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<u16>,
}

/// 网格单元格（tagged enum：kind 字段区分）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Cell {
    #[serde(rename_all = "camelCase")]
    Icon {
        id: String,
        plugin_id: String,
        title: String,
        size: Size,
        /// 图标自带的启动动作（如"应用抽屉"扫描出的应用）
        #[serde(default, skip_serializing_if = "Option::is_none")]
        action: Option<ActionSpec>,
        /// 自定义显示（M9）：覆盖插件 emoji / 背景色 / 借用系统应用图标
        #[serde(default, skip_serializing_if = "Option::is_none")]
        emoji: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        icon_path: Option<String>,
        /// 自由摆放（v3）：页面网格坐标（缺省由迁移分配）
        #[serde(default, skip_serializing_if = "Option::is_none")]
        x: Option<u16>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        y: Option<u16>,
    },
    #[serde(rename_all = "camelCase")]
    Folder {
        id: String,
        name: String,
        emoji: String,
        items: Vec<IconItem>,
        /// 自由摆放（v3）：页面网格坐标（缺省由迁移分配；文件夹占 1×1）
        #[serde(default, skip_serializing_if = "Option::is_none")]
        x: Option<u16>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        y: Option<u16>,
    },
}

impl Cell {
    pub fn id(&self) -> &str {
        match self {
            Cell::Icon { id, .. } => id,
            Cell::Folder { id, .. } => id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    #[serde(default = "default_layout_version")]
    pub version: u32,
    pub pages: Vec<Vec<Cell>>,
}

fn default_layout_version() -> u32 {
    3
}

/// 自由摆放（v3）：主页面虚拟列数 / 文件夹内虚拟列数 / 最大行数
pub const PAGE_COLS: u16 = 12;
pub const FOLDER_COLS: u16 = 6;
const MAX_ROWS: u16 = 500;

fn rects_overlap(a: (u16, u16, u16, u16), b: (u16, u16, u16, u16)) -> bool {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;
    ax < bx + bw && bx < ax + aw && ay < by + bh && by < ay + ah
}

/// 在 occupied 矩形集中找 w×h 的首个空位（行优先扫描；x+w 超出列宽则换行）
fn find_free_slot(
    occupied: &[(u16, u16, u16, u16)],
    cols: u16,
    w: u16,
    h: u16,
) -> Option<(u16, u16)> {
    for y in 0..MAX_ROWS {
        for x in 0..cols {
            if x + w > cols {
                continue;
            }
            let rect = (x, y, w, h);
            if !occupied.iter().any(|o| rects_overlap(rect, *o)) {
                return Some((x, y));
            }
        }
    }
    None
}

fn cell_size_from_json(obj: &serde_json::Map<String, serde_json::Value>) -> (u16, u16) {
    let size = obj.get("size");
    let w = size
        .and_then(|s| s.get("w"))
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .max(1) as u16;
    let h = size
        .and_then(|s| s.get("h"))
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .max(1) as u16;
    (w, h)
}

/// 给缺 x/y 的单元分配坐标（cols 列虚拟网格，行优先找空位）；幂等
fn assign_positions(cells: &mut [serde_json::Value], cols: u16) {
    let mut occupied: Vec<(u16, u16, u16, u16)> = Vec::new();
    for cell in cells {
        let Some(obj) = cell.as_object_mut() else {
            continue;
        };
        let (w, h) = cell_size_from_json(obj);
        match (
            obj.get("x").and_then(|v| v.as_u64()),
            obj.get("y").and_then(|v| v.as_u64()),
        ) {
            (Some(x), Some(y)) => {
                occupied.push((x as u16, y as u16, w, h));
            }
            _ => {
                if let Some((x, y)) = find_free_slot(&occupied, cols, w, h) {
                    obj.insert("x".into(), serde_json::json!(x));
                    obj.insert("y".into(), serde_json::json!(y));
                    occupied.push((x, y, w, h));
                }
            }
        }
        // 文件夹内部图标同样分配坐标（FOLDER_COLS 列）
        if let Some(items) = obj.get_mut("items").and_then(|i| i.as_array_mut()) {
            let mut focc: Vec<(u16, u16, u16, u16)> = Vec::new();
            for item in items {
                let Some(iobj) = item.as_object_mut() else {
                    continue;
                };
                let (iw, ih) = cell_size_from_json(iobj);
                match (
                    iobj.get("x").and_then(|v| v.as_u64()),
                    iobj.get("y").and_then(|v| v.as_u64()),
                ) {
                    (Some(x), Some(y)) => {
                        focc.push((x as u16, y as u16, iw, ih));
                    }
                    _ => {
                        if let Some((x, y)) = find_free_slot(&focc, FOLDER_COLS, iw, ih) {
                            iobj.insert("x".into(), serde_json::json!(x));
                            iobj.insert("y".into(), serde_json::json!(y));
                            focc.push((x, y, iw, ih));
                        }
                    }
                }
            }
        }
    }
}

/// v1 → v2 → v3 迁移：v1 图标项无 `kind`（补 `kind:"icon"`）；v3 起为自由摆放，
/// 给缺 `x`/`y` 的单元分配虚拟网格坐标。幂等：对任意输入执行都是安全的。
pub fn migrate_layout(value: &mut serde_json::Value) {
    if let Some(pages) = value.get_mut("pages").and_then(|p| p.as_array_mut()) {
        for page in pages {
            if let Some(cells) = page.as_array_mut() {
                // v1 → v2：kind 补全
                for cell in cells.iter_mut() {
                    if let Some(obj) = cell.as_object_mut() {
                        if !obj.contains_key("kind") {
                            obj.insert("kind".into(), serde_json::json!("icon"));
                        }
                    }
                }
                // v2 → v3：自由摆放坐标
                assign_positions(cells, PAGE_COLS);
            }
        }
    }
    let version = value.get("version").and_then(|x| x.as_u64()).unwrap_or(0);
    if version < 3 {
        value["version"] = serde_json::json!(3);
    }
}

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

// ---------- 布局持久化 ----------

pub fn read_layout_from(path: &Path) -> Option<Layout> {
    let text = fs::read_to_string(path).ok()?;
    let mut value: serde_json::Value = serde_json::from_str(&text).ok()?;
    migrate_layout(&mut value);
    serde_json::from_value(value).ok()
}

pub fn write_layout_to(path: &Path, layout: &Layout) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(layout).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

/// 合并布局（M13 合并导入）：保留当前布局，把备份中"当前不存在（按 id 去重，含文件夹内）"的单元
/// 追加到对应页；备份页数超出当前页数时追加为新页。纯函数，便于单测。
pub fn merge_layout(current: &serde_json::Value, backup: &serde_json::Value) -> serde_json::Value {
    use serde_json::Value;

    let cur_pages = current
        .get("pages")
        .and_then(|p| p.as_array())
        .cloned()
        .unwrap_or_default();
    let bak_pages = backup
        .get("pages")
        .and_then(|p| p.as_array())
        .cloned()
        .unwrap_or_default();

    let mut out: Vec<Value> = cur_pages;
    let mut existing: HashSet<String> = HashSet::new();
    for page in &out {
        collect_cell_ids(page, &mut existing);
    }

    for (i, bpage) in bak_pages.iter().enumerate() {
        let new_cells: Vec<Value> = bpage
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter(|c| {
                        let id = c.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        !id.is_empty() && existing.insert(id.to_string())
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        if i < out.len() {
            if let Some(page) = out[i].as_array_mut() {
                page.extend(new_cells);
            }
        } else if !new_cells.is_empty() {
            out.push(Value::Array(new_cells));
        }
    }

    let mut result = current.clone();
    result["pages"] = Value::Array(out);
    result
}

/// 收集单元 id（页面图标 + 文件夹内图标）到集合
fn collect_cell_ids(page: &serde_json::Value, out: &mut HashSet<String>) {
    let Some(cells) = page.as_array() else { return };
    for cell in cells {
        if let Some(id) = cell.get("id").and_then(|v| v.as_str()) {
            out.insert(id.to_string());
        }
        if let Some(items) = cell.get("items").and_then(|v| v.as_array()) {
            for item in items {
                if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                    out.insert(id.to_string());
                }
            }
        }
    }
}

/// 合并配置（M13 合并导入）：以备份键覆盖，保留当前独有键
pub fn merge_config(current: &serde_json::Value, backup: &serde_json::Value) -> serde_json::Value {
    let mut out = current.as_object().cloned().unwrap_or_default();
    if let Some(b) = backup.as_object() {
        for (k, v) in b {
            out.insert(k.clone(), v.clone());
        }
    }
    serde_json::Value::Object(out)
}

// ---------- 图像编码 ----------

/// 把 RGBA 像素数据编码为 PNG 字节（用于系统应用图标）。
/// `rgba` 长度必须等于 `width * height * 4`（纯函数，便于单测）。
pub fn encode_rgba_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or("尺寸溢出")?;
    if rgba.len() != expected {
        return Err(format!(
            "RGBA 数据长度不匹配: 期望 {expected} 字节, 实际 {}",
            rgba.len()
        ));
    }
    let img = image::RgbaImage::from_raw(width, height, rgba.to_vec()).ok_or("创建图像失败")?;
    let dyn_img = image::DynamicImage::ImageRgba8(img);
    let mut out: Vec<u8> = Vec::new();
    dyn_img
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| format!("PNG 编码失败: {e}"))?;
    Ok(out)
}

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
        std::process::Command::new("cmd")
            .args(["/C", "start", "", path])
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
        std::process::Command::new("cmd")
            .args(["/C", cmd])
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

// ---------- 单元测试 ----------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_DIR_SEQ: AtomicU32 = AtomicU32::new(0);

    fn temp_dir(name: &str) -> PathBuf {
        let seq = TEST_DIR_SEQ.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "homedesktop-test-{}-{}-{}",
            name,
            std::process::id(),
            seq
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn sample_manifest() -> &'static str {
        r#"{
            "id": "dev.homedesktop.demo",
            "name": "Demo",
            "version": "0.1.0",
            "type": "icon",
            "emoji": "🧪",
            "actions": [{ "kind": "app", "path": "C:\\Windows\\notepad.exe" }]
        }"#
    }

    #[test]
    fn manifest_parses_valid_json() {
        let m: Manifest = serde_json::from_str(sample_manifest()).expect("valid manifest");
        assert_eq!(m.id, "dev.homedesktop.demo");
        assert_eq!(m.plugin_type, "icon");
        assert_eq!(m.emoji.as_deref(), Some("🧪"));
        assert_eq!(m.actions.len(), 1);
        assert_eq!(m.actions[0].kind, ActionKind::App);
        assert_eq!(
            m.actions[0].path.as_deref(),
            Some(r"C:\Windows\notepad.exe")
        );
    }

    #[test]
    fn manifest_rejects_invalid_json() {
        assert!(serde_json::from_str::<Manifest>("{ not json").is_err());
        // 缺少必填字段
        assert!(serde_json::from_str::<Manifest>(r#"{"id":"x"}"#).is_err());
    }

    #[test]
    fn scan_plugin_dir_skips_invalid_and_non_manifest() {
        let dir = temp_dir("scan");
        fs::create_dir_all(dir.join("good")).unwrap();
        fs::create_dir_all(dir.join("bad")).unwrap();
        fs::create_dir_all(dir.join("no-manifest")).unwrap();
        fs::write(dir.join("good/manifest.json"), sample_manifest()).unwrap();
        fs::write(dir.join("bad/manifest.json"), "{ broken").unwrap();
        // no-manifest 目录没有 manifest.json，应被跳过

        let mut seen = HashSet::new();
        let mut out = Vec::new();
        scan_plugin_dir(&dir, &mut seen, &mut out);

        assert_eq!(out.len(), 1, "只有合法 manifest 被收集");
        assert_eq!(out[0].id, "dev.homedesktop.demo");
    }

    #[test]
    fn collect_plugins_dedupes_by_id_with_later_priority() {
        let low = temp_dir("low");
        let high = temp_dir("high");
        fs::create_dir_all(low.join("p1")).unwrap();
        fs::create_dir_all(high.join("p1")).unwrap();
        // 同一 id，高优先级目录（列表靠前）版本号更高
        fs::write(
            high.join("p1/manifest.json"),
            sample_manifest().replace("0.1.0", "0.2.0-high"),
        )
        .unwrap();
        fs::write(
            low.join("p1/manifest.json"),
            sample_manifest().replace("0.1.0", "0.1.0-low"),
        )
        .unwrap();

        // 列表顺序 = 优先级：靠前的目录先扫描、先到先得
        let plugins = collect_plugins(&[high.clone(), low.clone()]);
        assert_eq!(plugins.len(), 1, "重复 id 只保留一个");
        assert_eq!(plugins[0].version, "0.2.0-high", "高优先级目录（靠前）胜出");
    }

    #[test]
    fn collect_plugins_marks_builtin_by_dir_index() {
        use std::collections::HashMap;
        let user = temp_dir("u");
        let res = temp_dir("r");
        fs::create_dir_all(user.join("p1")).unwrap();
        fs::create_dir_all(res.join("p1")).unwrap();
        fs::create_dir_all(res.join("p2")).unwrap();
        fs::write(user.join("p1/manifest.json"), sample_manifest()).unwrap();
        fs::write(res.join("p1/manifest.json"), sample_manifest()).unwrap();
        fs::write(
            res.join("p2/manifest.json"),
            sample_manifest().replace("dev.homedesktop.demo", "dev.homedesktop.demo2"),
        )
        .unwrap();

        // 索引 0 = 用户数据目录（非内置）；索引 1+ = 资源/内置目录（内置）
        let list = collect_plugins_with_builtin(&[user, res]);
        let m: HashMap<String, bool> = list.into_iter().map(|(p, b)| (p.id, b)).collect();
        assert_eq!(m.get("dev.homedesktop.demo"), Some(&false), "用户目录插件非内置");
        assert_eq!(m.get("dev.homedesktop.demo2"), Some(&true), "资源目录插件为内置");
    }

    #[test]
    fn layout_roundtrip_via_file() {
        let dir = temp_dir("layout");
        let path = dir.join("layout.json");
        let layout = Layout {
            version: 3,
            pages: vec![vec![
                Cell::Icon {
                    id: "item-1".into(),
                    plugin_id: "dev.homedesktop.demo".into(),
                    title: "Demo".into(),
                    size: Size { w: 1, h: 1 },
                    action: None,
                    emoji: None,
                    color: None,
                    icon_path: None,
                    x: Some(0),
                    y: Some(0),
                },
                Cell::Folder {
                    id: "folder-1".into(),
                    name: "工具".into(),
                    emoji: "📁".into(),
                    items: vec![IconItem {
                        id: "item-2".into(),
                        plugin_id: "dev.homedesktop.demo".into(),
                        title: "内部图标".into(),
                        size: Size { w: 1, h: 1 },
                        action: None,
                        emoji: None,
                        color: None,
                        icon_path: None,
                        x: Some(0),
                        y: Some(0),
                    }],
                    x: Some(1),
                    y: Some(0),
                },
            ]],
        };

        write_layout_to(&path, &layout).expect("write ok");
        let loaded = read_layout_from(&path).expect("read ok");
        assert_eq!(loaded, layout, "序列化往返一致（含文件夹）");
    }

    #[test]
    fn layout_v1_auto_migrates_to_v3() {
        // v1 布局：图标项没有 kind 字段、无 version
        let v1 = r#"{
            "pages": [[
                { "id": "a", "pluginId": "p1", "title": "A", "size": { "w": 1, "h": 1 } }
            ]]
        }"#;
        let mut value: serde_json::Value = serde_json::from_str(v1).unwrap();
        migrate_layout(&mut value);
        assert_eq!(value["version"], 3);
        assert_eq!(value["pages"][0][0]["kind"], "icon");
        // v3：自动分配自由摆放坐标
        assert_eq!(value["pages"][0][0]["x"], 0);
        assert_eq!(value["pages"][0][0]["y"], 0);

        // 迁移后能正常反序列化为 Layout v3
        let layout: Layout = serde_json::from_value(value).unwrap();
        assert_eq!(layout.version, 3);
        match &layout.pages[0][0] {
            Cell::Icon { id, .. } => assert_eq!(id, "a"),
            _ => panic!("应为图标单元"),
        }
    }

    #[test]
    fn migrate_layout_is_idempotent_and_assigns_positions() {
        // v2：已有 kind、无坐标；两个 1×1 图标 + 一个 2×1 小组件
        let v2 = r#"{
            "version": 2,
            "pages": [[
                { "kind": "icon", "id": "a", "pluginId": "p1", "title": "A", "size": { "w": 1, "h": 1 } },
                { "kind": "icon", "id": "b", "pluginId": "p1", "title": "B", "size": { "w": 2, "h": 1 } }
            ]]
        }"#;
        let mut value: serde_json::Value = serde_json::from_str(v2).unwrap();
        migrate_layout(&mut value);
        assert_eq!(value["version"], 3);
        assert_eq!(value["pages"][0][0]["kind"], "icon");
        // a 占 (0,0) 1×1；b 为 2×1 → 起点 (1,0)（a 后一个槽）
        assert_eq!(value["pages"][0][0]["x"], 0);
        assert_eq!(value["pages"][0][0]["y"], 0);
        assert_eq!(value["pages"][0][1]["x"], 1);
        assert_eq!(value["pages"][0][1]["y"], 0);
        // 幂等：再跑一次结果不变
        let snapshot = value.clone();
        migrate_layout(&mut value);
        assert_eq!(value, snapshot, "幂等");
    }

    #[test]
    fn migrate_assigns_folder_item_positions() {
        let v2 = r#"{
            "version": 2,
            "pages": [[
                { "kind": "folder", "id": "f", "name": "F", "emoji": "📁", "items": [
                    { "id": "i1", "pluginId": "p", "title": "1", "size": { "w": 1, "h": 1 } },
                    { "id": "i2", "pluginId": "p", "title": "2", "size": { "w": 1, "h": 1 } }
                ] }
            ]]
        }"#;
        let mut value: serde_json::Value = serde_json::from_str(v2).unwrap();
        migrate_layout(&mut value);
        assert_eq!(value["pages"][0][0]["x"], 0);
        assert_eq!(value["pages"][0][0]["y"], 0);
        // 文件夹内图标按 FOLDER_COLS(6) 列分配
        assert_eq!(value["pages"][0][0]["items"][0]["x"], 0);
        assert_eq!(value["pages"][0][0]["items"][0]["y"], 0);
        assert_eq!(value["pages"][0][0]["items"][1]["x"], 1);
        assert_eq!(value["pages"][0][0]["items"][1]["y"], 0);
    }

    #[test]
    fn cell_serializes_with_kind_tag() {
        let cell = Cell::Icon {
            id: "x".into(),
            plugin_id: "p".into(),
            title: "X".into(),
            size: Size { w: 1, h: 1 },
            action: None,
            emoji: None,
            color: None,
            icon_path: None,
                    x: None,
                    y: None,
        };
        let json = serde_json::to_value(&cell).unwrap();
        assert_eq!(json["kind"], "icon");
        assert_eq!(json["pluginId"], "p");

        // 图标自带动作时序列化包含 action；无动作时不输出该字段
        let with_action = Cell::Icon {
            id: "y".into(),
            plugin_id: "builtin.app".into(),
            title: "Y".into(),
            size: Size { w: 1, h: 1 },
            action: Some(ActionSpec {
                kind: ActionKind::App,
                path: Some("C:\\app.exe".into()),
                cmd: None,
            }),
            emoji: None,
            color: None,
            icon_path: None,
                    x: None,
                    y: None,
        };
        let json = serde_json::to_value(&with_action).unwrap();
        assert_eq!(json["action"]["kind"], "app");
        assert!(!json["action"].get("path").is_none());

        // 自定义字段（M9）往返不丢：emoji / color / iconPath
        let custom = Cell::Icon {
            id: "z".into(),
            plugin_id: "p".into(),
            title: "Z".into(),
            size: Size { w: 1, h: 1 },
            action: None,
            emoji: Some("🎮".into()),
            color: Some("#e53935".into()),
            icon_path: Some("C:\\game.exe".into()),
            x: None,
            y: None,
        };
        let json = serde_json::to_value(&custom).unwrap();
        assert_eq!(json["emoji"], "🎮");
        assert_eq!(json["color"], "#e53935");
        assert_eq!(json["iconPath"], "C:\\game.exe");
        let back: Cell = serde_json::from_value(json).unwrap();
        assert_eq!(back, custom, "自定义字段序列化往返一致");

        let folder = Cell::Folder {
            id: "f".into(),
            name: "F".into(),
            emoji: "📁".into(),
            items: vec![],
            x: None,
            y: None,
        };
        let json = serde_json::to_value(&folder).unwrap();
        assert_eq!(json["kind"], "folder");
        assert_eq!(json["emoji"], "📁");
    }

    #[test]
    fn execute_action_dispatches_and_errors() {
        // app 类型缺 path → 报错（不实际启动）
        let bad_app = ActionSpec {
            kind: ActionKind::App,
            path: None,
            cmd: None,
        };
        assert!(execute_action(&bad_app).is_err());

        // command 类型缺 cmd → 报错
        let bad_cmd = ActionSpec {
            kind: ActionKind::Command,
            path: None,
            cmd: None,
        };
        assert!(execute_action(&bad_cmd).is_err());
    }

    #[test]
    fn system_apps_action_is_frontend_handled() {
        // 序列化：kind → "system_apps"
        let spec = ActionSpec {
            kind: ActionKind::SystemApps,
            path: None,
            cmd: None,
        };
        let json = serde_json::to_value(&spec).unwrap();
        assert_eq!(json["kind"], "system_apps");

        // 反序列化：manifest 里的 {"kind":"system_apps"} 可解析
        let parsed: ActionSpec = serde_json::from_str(r#"{"kind":"system_apps"}"#).unwrap();
        assert_eq!(parsed.kind, ActionKind::SystemApps);

        // 后端不执行（由前端处理）
        assert!(execute_action(&spec).is_err());
    }

    #[test]
    fn encode_rgba_png_roundtrip() {
        let w = 4u32;
        let h = 3u32;
        let mut rgba = Vec::new();
        for y in 0..h {
            for x in 0..w {
                rgba.extend_from_slice(&[
                    (x * 60) as u8,
                    (y * 80) as u8,
                    200,
                    if (x + y) % 2 == 0 { 255 } else { 128 },
                ]);
            }
        }
        let png = encode_rgba_png(w, h, &rgba).expect("encode ok");
        // PNG 魔数
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);

        // 解码往返验证像素一致
        let decoded = image::load_from_memory(&png).expect("decode ok").to_rgba8();
        assert_eq!(decoded.dimensions(), (w, h));
        assert_eq!(decoded.as_raw(), &rgba, "PNG 往返像素一致");
    }

    #[test]
    fn encode_rgba_png_rejects_wrong_size() {
        assert!(encode_rgba_png(2, 2, &[0u8; 8]).is_err(), "长度不足应报错");
        assert!(encode_rgba_png(0, 0, &[]).is_err(), "0 尺寸应报错");
        assert!(encode_rgba_png(10, 10, &[0u8; 400]).is_ok());
    }

    #[test]
    fn layout_load_missing_file_returns_none() {
        let dir = temp_dir("layout-missing");
        assert!(read_layout_from(&dir.join("nope.json")).is_none());
    }

    #[test]
    fn launch_plugin_action_errors() {
        // 无 actions
        let no_action = PluginInfo {
            id: "a".into(),
            name: "a".into(),
            version: "0".into(),
            plugin_type: "icon".into(),
            emoji: None,
            actions: vec![],
            widget_component: None,
            sizes: vec![],
            settings: vec![],
            builtin: false,
            provider_id: None,
            provider_name: None,
            domain: None,
            widget_file: None,
            widget_element: None,
            dir: None,
        };
        assert!(launch_plugin_action(&no_action).is_err());

        // app 类型缺 path
        let no_path = PluginInfo {
            id: "b".into(),
            name: "b".into(),
            version: "0".into(),
            plugin_type: "icon".into(),
            emoji: None,
            actions: vec![ActionSpec {
                kind: ActionKind::App,
                path: None,
                cmd: None,
            }],
            widget_component: None,
            sizes: vec![],
            settings: vec![],
            builtin: false,
            provider_id: None,
            provider_name: None,
            domain: None,
            widget_file: None,
            widget_element: None,
            dir: None,
        };
        assert!(launch_plugin_action(&no_path).is_err());
    }

    #[test]
    fn plugin_info_serializes_camel_case() {
        let info = PluginInfo {
            id: "x".into(),
            name: "X".into(),
            version: "1".into(),
            plugin_type: "widget".into(),
            emoji: Some("🕐".into()),
            actions: vec![ActionSpec {
                kind: ActionKind::Command,
                path: None,
                cmd: Some("echo hi".into()),
            }],
            widget_component: Some("clock".into()),
            sizes: vec![Size { w: 2, h: 1 }, Size { w: 2, h: 2 }],
            settings: vec![SettingSpec {
                key: "showSeconds".into(),
                label: "显示秒".into(),
                setting_type: SettingType::Toggle,
                options: None,
                default: Some(serde_json::json!(true)),
            }],
            builtin: false,
            provider_id: None,
            provider_name: None,
            domain: None,
            widget_file: Some("widget.js".into()),
            widget_element: Some("hd-x-widget".into()),
            dir: Some("C:\\plugins\\x".into()),
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["pluginType"], "widget");
        assert_eq!(json["actions"][0]["kind"], "command");
        assert_eq!(json["widgetComponent"], "clock");
        assert_eq!(json["sizes"][1]["w"], 2);
        assert_eq!(json["settings"][0]["type"], "toggle");
        assert_eq!(json["settings"][0]["default"], true);
        assert_eq!(json["widgetFile"], "widget.js");
        assert_eq!(json["widgetElement"], "hd-x-widget");
        assert_eq!(json["dir"], "C:\\plugins\\x");

        // builtin 标记：true 序列化、false 省略
        let builtin = PluginInfo {
            builtin: true,
            provider_id: None,
            provider_name: None,
            domain: None,
            ..info
        };
        let json = serde_json::to_value(&builtin).unwrap();
        assert_eq!(json["builtin"], true);
        assert!(!json.get("builtin").is_none());
    }

    #[test]
    fn widget_manifest_parses() {
        let manifest = r#"{
            "id": "dev.homedesktop.clock",
            "name": "时钟",
            "version": "0.1.0",
            "type": "widget",
            "emoji": "🕐",
            "actions": [],
            "widgetComponent": "clock"
        }"#;
        let m: Manifest = serde_json::from_str(manifest).expect("widget manifest valid");
        assert_eq!(m.plugin_type, "widget");
        assert_eq!(m.widget_component.as_deref(), Some("clock"));
        // 旧 manifest（无 widgetComponent）仍可解析
        let old = serde_json::from_str::<Manifest>(
            r#"{"id":"a","name":"A","version":"1","type":"icon","emoji":"📦","actions":[]}"#,
        )
        .expect("old manifest still parses");
        assert_eq!(old.widget_component, None);
    }

    #[test]
    fn merge_layout_keeps_new_and_appends_backup_missing() {
        // 当前布局：页1 有 A(新增)、B
        let current = serde_json::json!({
            "version": 2,
            "pages": [[
                { "kind": "icon", "id": "A", "pluginId": "p", "title": "A", "size": { "w": 1, "h": 1 } },
                { "kind": "icon", "id": "B", "pluginId": "p", "title": "B", "size": { "w": 1, "h": 1 } }
            ]]
        });
        // 备份：页1 有 A(重复)、C；页2 有 D
        let backup = serde_json::json!({
            "version": 2,
            "pages": [
                [{ "kind": "icon", "id": "A", "pluginId": "p", "title": "A", "size": { "w": 1, "h": 1 } },
                 { "kind": "icon", "id": "C", "pluginId": "p", "title": "C", "size": { "w": 1, "h": 1 } }],
                [{ "kind": "icon", "id": "D", "pluginId": "p", "title": "D", "size": { "w": 1, "h": 1 } }]
            ]
        });
        let merged = merge_layout(&current, &backup);
        let pages = merged["pages"].as_array().unwrap();
        assert_eq!(pages.len(), 2, "页1 合并 + 页2 新增");
        let p1: Vec<&str> = pages[0]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["id"].as_str().unwrap())
            .collect();
        assert_eq!(p1, vec!["A", "B", "C"], "保留新增 A/B，追加备份独有的 C");
        assert_eq!(pages[1][0]["id"], "D");
    }

    #[test]
    fn merge_layout_dedupes_by_id_inside_folders() {
        let current = serde_json::json!({
            "version": 2,
            "pages": [[
                { "kind": "folder", "id": "f", "name": "F", "emoji": "📁",
                  "items": [{ "kind": "icon", "id": "inner", "pluginId": "p", "title": "x", "size": { "w": 1, "h": 1 } }] }
            ]]
        });
        let backup = serde_json::json!({
            "version": 2,
            "pages": [[
                { "kind": "icon", "id": "inner", "pluginId": "p", "title": "x", "size": { "w": 1, "h": 1 } },
                { "kind": "icon", "id": "new", "pluginId": "p", "title": "n", "size": { "w": 1, "h": 1 } }
            ]]
        });
        let merged = merge_layout(&current, &backup);
        let p1 = merged["pages"][0].as_array().unwrap();
        let ids: Vec<&str> = p1.iter().map(|c| c["id"].as_str().unwrap()).collect();
        assert_eq!(ids, vec!["f", "new"], "文件夹内的 inner 已存在则跳过");
    }

    #[test]
    fn merge_config_backup_wins_and_keeps_others() {
        let current = serde_json::json!({ "plugin.a.k": "old", "plugin.keep.k": 1 });
        let backup = serde_json::json!({ "plugin.a.k": "new", "plugin.b.k": true });
        let merged = merge_config(&current, &backup);
        assert_eq!(merged["plugin.a.k"], "new");
        assert_eq!(merged["plugin.keep.k"], 1);
        assert_eq!(merged["plugin.b.k"], true);
    }

    #[test]
    fn plugin_widget_file_manifest_parses() {
        let manifest = r#"{
            "id": "com.example.timer",
            "name": "倒计时",
            "version": "1.0.0",
            "type": "widget",
            "emoji": "⏱️",
            "widgetComponent": "__plugin__",
            "widgetFile": "widget.js",
            "widgetElement": "hd-timer-widget",
            "actions": [],
            "sizes": [{ "w": 2, "h": 1 }]
        }"#;
        let m: Manifest = serde_json::from_str(manifest).expect("parse ok");
        assert_eq!(m.widget_component.as_deref(), Some("__plugin__"));
        assert_eq!(m.widget_file.as_deref(), Some("widget.js"));
        assert_eq!(m.widget_element.as_deref(), Some("hd-timer-widget"));
        // 旧 manifest（无这些字段）仍可解析
        let old = serde_json::from_str::<Manifest>(
            r#"{"id":"a","name":"A","version":"1","type":"icon","emoji":"📦","actions":[]}"#,
        )
        .expect("old manifest still parses");
        assert_eq!(old.widget_file, None);
        assert_eq!(old.widget_element, None);
    }
}
