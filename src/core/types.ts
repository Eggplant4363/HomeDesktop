// HomeDesktop 核心类型定义（与 homedesktop-core 的 schema 一一对应）

export type ActionSpec =
  | { kind: "app"; path: string }
  | { kind: "command"; cmd: string }
  | { kind: "system_apps" };

/** 网格尺寸（1×1 基准的整数倍） */
export interface Size {
  w: number;
  h: number;
}

export type SettingType = "text" | "number" | "select" | "toggle";

/** 插件设置项声明（框架统一设置菜单渲染） */
export interface SettingOption {
  label: string;
  value: string | number | boolean;
}

export interface SettingSpec {
  key: string;
  label: string;
  type: SettingType;
  options?: SettingOption[];
  default?: string | number | boolean;
}

export interface PluginInfo {
  id: string;
  name: string;
  version: string;
  pluginType: "icon" | "widget";
  /** 用 emoji 作图标，避免 MVP 阶段处理图片资源 */
  emoji?: string;
  actions: ActionSpec[];
  /** type=widget 时前端渲染哪个小组件（"clock" | "weather" …） */
  widgetComponent?: string;
  /** 插件声明支持的尺寸集合（小米/安卓设计：切大小只能选这些档；空=回退默认） */
  sizes?: Size[];
  /** 插件可配置项声明（统一设置菜单） */
  settings?: SettingSpec[];
  /** 是否内置插件（随应用分发，不可卸载；M11） */
  builtin?: boolean;
  /** 插件目录绝对路径（M16：加载插件自带 JS 用 asset 协议） */
  dir?: string;
  /** 插件自带小组件（M16）：插件目录内的 JS 文件（自定义元素定义） */
  widgetFile?: string;
  /** 插件自带小组件（M16）：自定义元素标签名 */
  widgetElement?: string;
}

/** 插件市场目录项（本地 market/*.zip） */
export interface MarketItem {
  file: string;
  id: string;
  name: string;
  version: string;
  pluginType: "icon" | "widget";
  emoji?: string;
  installed: boolean;
}

/** 在线市场项（远程仓库 index.json） */
export interface RemoteMarketItem extends MarketItem {
  size?: number;
  description?: string;
}

/** 在线市场（远程仓库索引） */
export interface RemoteMarket {
  base: string;
  items: RemoteMarketItem[];
}

/** 图标单元（网格内与文件夹内共用） */
export interface IconCell {
  kind: "icon";
  id: string;
  pluginId: string;
  title: string;
  /** 网格占位（单元计），小组件插件可能 > 1x1 */
  size: { w: number; h: number };
  /** 图标自带的启动动作（应用抽屉扫描出的应用）；缺省时按 pluginId 查插件 */
  action?: ActionSpec;
  /** 自定义显示（M9）：覆盖插件 emoji（空/缺省=用插件 emoji） */
  emoji?: string;
  /** 自定义背景色（M9，hex 如 #e53935；空=默认） */
  color?: string;
  /** 借用系统应用图标（M9：存系统应用路径，渲染时用 app_icon 取真实图标） */
  iconPath?: string;
  /** 自由摆放（v3）：页面/文件夹内网格坐标（缺省由迁移分配） */
  x?: number;
  y?: number;
}

/** 文件夹单元（内含图标列表） */
export interface FolderCell {
  kind: "folder";
  id: string;
  name: string;
  emoji: string;
  items: IconCell[];
  /** 自由摆放（v3）：页面网格坐标（缺省由迁移分配；文件夹占 1x1） */
  x?: number;
  y?: number;
}

/** 网格单元格：图标 | 文件夹 */
export type Cell = IconCell | FolderCell;

export interface Layout {
  version: number;
  pages: Cell[][];
}

export function isFolder(cell: Cell): cell is FolderCell {
  return cell.kind === "folder";
}

export function isIcon(cell: Cell): cell is IconCell {
  return cell.kind === "icon";
}
