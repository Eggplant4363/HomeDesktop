// 应用级设置动作（从 App.svelte 拆出，阶段1重构）
// - 外观：主题 / 背景 / 壁纸 / 图标大小 / 网格间距 / 特效开关
// - 系统：开机自启、快捷键
// - 网络代理配置
// 统一在此协调 appearance 状态与持久化。

import { invoke } from "@tauri-apps/api/core";
import {
  appearance,
  applyTheme,
  getCurrentBackground,
  saveAppearance,
  setBackgroundForCurrentTheme,
  type Background,
} from "./appearance.svelte";
import { comboLabel } from "./shortcuts";
import {
  autostartEnabled,
  getSearchShortcut,
  getToggleShortcut,
  pickWallpaperImage,
  setAutostart,
  setSearchShortcut,
  setToggleShortcut,
} from "./system";
import { log } from "./logger";
import { toast } from "./editorState.svelte";

/** 外观/系统/代理 的可变表单状态 */
export const settings = $state({
  /** 自定义背景色（临时值，与当前主题背景联动） */
  customColor: "#10141c",
  /** 开机自启状态 */
  autoStart: false,
  /** Pad 开关快捷键（默认 alt+space） */
  padShortcut: "alt+space",
  /** 搜索唤起快捷键（默认 ctrl+space） */
  searchShortcut: "ctrl+space",
  /** 快捷键注册状态（冲突警告展示） */
  shortcutStatus: {} as Record<string, { ok: boolean; spec?: string; error?: string }>,
  /** 网络代理表单 */
  proxy: {
    mode: "none",
    host: "",
    port: "",
    username: "",
    password: "",
  },
});

// ---------- 外观 ----------

/** 应用当前主题的背景色到 customColor（切换主题/初始化时调用） */
export function syncCustomColor(): void {
  const bg = getCurrentBackground();
  settings.customColor = bg.kind === "color" ? bg.value : "#10141c";
}

export function setBackground(bg: Background): void {
  setBackgroundForCurrentTheme(bg);
  if (bg.kind === "color") settings.customColor = bg.value;
  void saveAppearance();
  log.info(`设置背景(${appearance.theme}主题): ${bg.kind}`);
}

export function setCustomColor(): void {
  setBackgroundForCurrentTheme({ kind: "color", value: settings.customColor });
  void saveAppearance();
  log.info(`自定义背景色(${appearance.theme}主题): ${settings.customColor}`);
}

export function setTheme(theme: "dark" | "light"): void {
  appearance.theme = theme;
  applyTheme(theme);
  syncCustomColor();
  void saveAppearance();
  log.info(`切换主题: ${theme}`);
}

/** 特效开关：添加图标弹出动画 */
export function toggleIconAddEffect(): void {
  appearance.effects.iconAdd = !appearance.effects.iconAdd;
  void saveAppearance();
  log.info(`添加图标特效: ${appearance.effects.iconAdd ? "开启" : "关闭"}`);
  toast(`添加图标特效已${appearance.effects.iconAdd ? "开启" : "关闭"}`);
}

/** 特效开关：窗口显示/隐藏过渡动画 */
export function toggleWindowAnim(): void {
  appearance.effects.windowAnim = !appearance.effects.windowAnim;
  void saveAppearance();
  log.info(`窗口过渡特效: ${appearance.effects.windowAnim ? "开启" : "关闭"}`);
  toast(`窗口过渡特效已${appearance.effects.windowAnim ? "开启" : "关闭"}`);
}

/** 网格间距滑块 */
export function setGridSpacing(v: number): void {
  appearance.gridSpacing = v;
  void saveAppearance();
}

/** 图标大小 */
export function setTileSize(size: number): void {
  appearance.tileSize = size;
  void saveAppearance();
  log.info(`设置图标大小: ${size}px`);
  toast(`图标大小 → ${size}px`);
}

/** 选择图片壁纸 */
export async function pickWallpaper(): Promise<void> {
  const stored = await pickWallpaperImage();
  if (stored) {
    setBackgroundForCurrentTheme({ kind: "image", value: stored });
    void saveAppearance();
    log.info(`设置图片壁纸(${appearance.theme}主题): ${stored}`);
    toast(`壁纸已设置（${appearance.theme === "dark" ? "深色" : "浅色"}主题）`);
  } else {
    log.warn("选择壁纸被取消或失败");
  }
}

// ---------- 系统 ----------

export async function toggleAutostart(): Promise<void> {
  settings.autoStart = !settings.autoStart;
  await setAutostart(settings.autoStart);
  log.info(`开机自启: ${settings.autoStart ? "开启" : "关闭"}`);
}

/** 初始化：读取自启状态与已保存快捷键 */
export async function initSystemSettings(): Promise<void> {
  settings.autoStart = await autostartEnabled();
  settings.padShortcut = await getToggleShortcut();
  settings.searchShortcut = await getSearchShortcut();
  await refreshShortcutStatus();
}

/** 刷新快捷键注册状态（冲突提示） */
export async function refreshShortcutStatus(): Promise<void> {
  try {
    const list = await invoke<{ action: string; spec: string; ok: boolean; error?: string | null }[]>(
      "shortcuts_status",
    );
    const m: Record<string, { ok: boolean; spec?: string; error?: string }> = {};
    for (const s of list) m[s.action] = { ok: s.ok, spec: s.spec, error: s.error ?? undefined };
    settings.shortcutStatus = m;
  } catch {
    /* 忽略 */
  }
}

/** 保存"显示/隐藏 Pad"快捷键；返回错误信息（null = 成功） */
export async function savePadShortcut(combo: string): Promise<string | null> {
  const err = await setToggleShortcut(combo);
  if (err) {
    log.error(`快捷键保存失败: ${combo} -> ${err}`);
    return err;
  }
  settings.padShortcut = combo;
  log.info(`快捷键已修改: ${comboLabel(combo)}`);
  toast(`快捷键已改为 ${comboLabel(combo)}`);
  void refreshShortcutStatus();
  return null;
}

/** 保存搜索唤起快捷键 */
export async function saveSearchShortcut(combo: string): Promise<string | null> {
  const err = await setSearchShortcut(combo);
  if (err) {
    log.error(`快捷键保存失败: ${combo} -> ${err}`);
    return err;
  }
  settings.searchShortcut = combo;
  log.info(`搜索快捷键已修改: ${comboLabel(combo)}`);
  toast(`搜索快捷键已改为 ${comboLabel(combo)}`);
  void refreshShortcutStatus();
  return null;
}

// ---------- 网络代理 ----------

export async function loadProxy(): Promise<void> {
  try {
    const g = (key: string) => invoke<string | null>("config_get", { key });
    settings.proxy.mode = (await g("proxy.mode")) ?? "none";
    settings.proxy.host = (await g("proxy.host")) ?? "";
    settings.proxy.port = (await g("proxy.port")) ?? "";
    settings.proxy.username = (await g("proxy.username")) ?? "";
    settings.proxy.password = (await g("proxy.password")) ?? "";
  } catch (e) {
    log.error(`读取代理配置失败: ${e}`);
  }
}

export async function saveProxy(): Promise<void> {
  try {
    const s = (key: string, value: unknown) => invoke("config_set", { key, value });
    await s("proxy.mode", settings.proxy.mode);
    await s("proxy.host", settings.proxy.host.trim());
    await s("proxy.port", settings.proxy.port.trim());
    await s("proxy.username", settings.proxy.username);
    await s("proxy.password", settings.proxy.password);
    const desc =
      settings.proxy.mode === "none"
        ? "已关闭"
        : `${settings.proxy.mode}://${settings.proxy.host}:${settings.proxy.port}`;
    log.info(`代理设置已保存: ${desc}`);
    toast(`代理设置已保存（${desc}）`);
  } catch (e) {
    log.error(`保存代理配置失败: ${e}`);
    toast(`保存失败：${e}`, 4000);
  }
}
