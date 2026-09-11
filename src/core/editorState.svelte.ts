// 编辑器 UI 状态（从 App.svelte 拆出，阶段1重构）
// 只保存"打开了哪个弹窗/正在编辑哪一项"这类视图状态，不含布局业务逻辑。
// 说明：用可变 $state 对象承载（Svelte 5 不允许直接导出可重新赋值的 $state 变量，
// 因此统一以 `export const editor = $state({...})` 形式导出，字段可直接读写）。

import { log } from "./logger";

/** 各类编辑/弹窗状态 */
export const editor = $state({
  /** ＋ 添加菜单 */
  showAdd: false,
  /** 外观设置弹窗 */
  showSettings: false,
  /** 系统应用面板（内置「系统应用」插件点击后打开） */
  showSystemApps: false,
  /** 触发面板的「系统应用」槽位图标 id（选中应用后原位替换它） */
  systemAppsSource: null as string | null,
  /** 插件管理页面（已安装 / 市场） */
  showPlugins: false,
  /** 全局搜索面板 */
  showSearch: false,

  /** 正在编辑的文件夹（重命名/换 emoji 弹窗） */
  folderEditTarget: null as string | null,
  folderEditName: "",
  folderEditEmoji: "",

  /** 正在自定义的图标（重命名/换 emoji/换颜色/借用系统图标/图片图标） */
  iconEditTarget: null as string | null,
  iconEditTitle: "",
  iconEditEmoji: "",
  iconEditColor: "",
  iconEditIconPath: "",
  iconEditIconImage: "",
  iconEditShowLabel: true,
  /** 当前编辑的是否为小组件：弹窗隐藏"借用系统图标"，名称存实例设置 */
  iconEditIsWidget: false,
  /** 借用系统图标面板 */
  showIconBorrow: false,

  /** 正在选择"移入哪个文件夹"的图标 id */
  moveTarget: null as string | null,
  /** 正在选择尺寸的图标 id（仅 ⤡ 拖拽直接改，不弹窗） */
  sizeTarget: null as string | null,
  /** 正在打开插件设置菜单的单元格 id */
  settingsTarget: null as string | null,

  /** 通用确认框：null = 未打开 */
  confirmAction: null as { title: string; message: string; onConfirm: () => void } | null,
  /** 备份导入：选好文件后等待选择"覆盖/合并" */
  pendingImportSrc: null as string | null,

  /** 通用提示条文本 */
  error: "",

  /** 正在播放删除动效的单元 id（播完再真正移除） */
  breakingId: null as string | null,
  /** 新添加图标的 id（播放入场特效），1.6s 后清除 */
  addedFlashId: null as string | null,
});

/** 删除动效时长（ms）：与 CSS 动画一致，播完再执行真正删除 */
export const BREAK_MS = 420;

let toastTimer: ReturnType<typeof setTimeout> | undefined;

/** 通用提示条（操作反馈） */
export function toast(msg: string, ms = 2500): void {
  editor.error = msg;
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (editor.error = ""), ms);
}

/** 播放删除动效后执行 doIt（约 BREAK_MS 后关闭动画并真正删除） */
export function shatterThen(id: string | null, doIt: () => void): void {
  if (!id) {
    doIt();
    return;
  }
  editor.breakingId = id;
  setTimeout(() => {
    editor.breakingId = null;
    try {
      doIt();
    } catch (e) {
      log.error(`删除执行失败: ${e}`);
    }
  }, BREAK_MS);
}

let flashTimer: ReturnType<typeof setTimeout> | undefined;

/** 新添加/聚焦单元入场特效 */
export function flashAdded(id: string): void {
  editor.addedFlashId = id;
  if (flashTimer) clearTimeout(flashTimer);
  flashTimer = setTimeout(() => {
    if (editor.addedFlashId === id) editor.addedFlashId = null;
  }, 1600);
}

/** 打开通用确认框（所有删除操作统一走这里） */
export function askConfirm(title: string, message: string, onConfirm: () => void): void {
  editor.confirmAction = { title, message, onConfirm };
}

/** 执行确认框动作并关闭 */
export function runConfirm(): void {
  const act = editor.confirmAction;
  editor.confirmAction = null;
  act?.onConfirm();
}

/** 关闭所有浮层（切换编辑模式等场景复用） */
export function closeAllOverlays(): void {
  editor.showAdd = false;
  editor.showSettings = false;
  editor.moveTarget = null;
  editor.sizeTarget = null;
}
