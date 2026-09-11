//! 插件 manifest 与布局 schema（schema v3）。

use serde::{Deserialize, Serialize};

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
pub(crate) struct SubManifest {
    pub(crate) id: String,
    pub(crate) name: String,
    #[serde(rename = "type")]
    pub(crate) plugin_type: String,
    #[serde(default)]
    pub(crate) emoji: Option<String>,
    #[serde(default)]
    pub(crate) actions: Vec<ActionSpec>,
    #[serde(default)]
    pub(crate) widget_component: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) widget_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) widget_element: Option<String>,
    #[serde(default)]
    pub(crate) sizes: Vec<Size>,
    #[serde(default)]
    pub(crate) settings: Vec<SettingSpec>,
    /// 子插件专属字段（如 HA 的实体域：light/switch/sensor）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) domain: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Manifest {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) version: String,
    #[serde(rename = "type")]
    pub(crate) plugin_type: String,
    pub(crate) emoji: Option<String>,
    /// 提供商容器无顶层 actions（子插件各自带），故允许缺省
    #[serde(default)]
    pub(crate) actions: Vec<ActionSpec>,
    /// 提供商容器：子插件数组（type=provider 时有效）
    #[serde(default)]
    pub(crate) plugins: Vec<SubManifest>,
    /// 实体域（如 HA 子插件：light/switch/sensor）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) domain: Option<String>,
    /// type=widget 时前端渲染哪个小组件（"clock" | "weather" …；"__plugin__"=插件自带组件）
    pub(crate) widget_component: Option<String>,
    /// 插件自带小组件（M16）：插件目录内的 JS 文件（自定义元素定义）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) widget_file: Option<String>,
    /// 插件自带小组件（M16）：自定义元素标签名
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) widget_element: Option<String>,
    /// 插件声明支持的尺寸集合（小米/安卓 widget 设计：切换大小只能选这些档）
    #[serde(default)]
    pub(crate) sizes: Vec<Size>,
    /// 插件声明可配置项（框架提供统一设置菜单）
    #[serde(default)]
    pub(crate) settings: Vec<SettingSpec>,
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

/// 1×1 默认尺寸（文件夹未保存尺寸时回退）
fn size_one() -> Size {
    Size { w: 1, h: 1 }
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
    /// 自定义图片图标（"来自图片"，data URL；渲染时优先于 icon_path）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_image: Option<String>,
    /// 是否显示名称（缺省=显示；false=只显示图标）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_label: Option<bool>,
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
        /// 自定义图片图标（"来自图片"，data URL；渲染时优先于 icon_path）
        #[serde(default, skip_serializing_if = "Option::is_none")]
        icon_image: Option<String>,
        /// 是否显示名称（缺省=显示；false=只显示图标）
        #[serde(default, skip_serializing_if = "Option::is_none")]
        show_label: Option<bool>,
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
        /// 文件夹尺寸（可调大；缺省 1×1，旧布局兼容）
        #[serde(default = "size_one")]
        size: Size,
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
pub(crate) const MAX_ROWS: u16 = 500;

