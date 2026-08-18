// 系统能力：壁纸图片选择、开机自启
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";

/** 弹出文件选择框，把选中的图片拷贝到 app 数据目录，返回存储路径（失败/取消返回 null） */
export async function pickWallpaperImage(): Promise<string | null> {
  try {
    const picked = await open({
      multiple: false,
      filters: [
        { name: "图片", extensions: ["png", "jpg", "jpeg", "webp", "gif", "bmp"] },
      ],
    });
    if (typeof picked !== "string" || !picked) return null;
    return await invoke<string>("set_wallpaper", { src: picked });
  } catch (e) {
    console.error("[homedesktop] pick wallpaper failed:", e);
    return null;
  }
}

export async function autostartEnabled(): Promise<boolean> {
  try {
    return await isEnabled();
  } catch (e) {
    console.error("[homedesktop] autostart isEnabled failed:", e);
    return false;
  }
}

export async function setAutostart(on: boolean): Promise<void> {
  try {
    if (on) await enable();
    else await disable();
  } catch (e) {
    console.error("[homedesktop] set autostart failed:", e);
  }
}

// ---------- 全屏 Pad（M6）：记忆上次是否全屏 ----------

const FULLSCREEN_KEY = "ui.fullscreen";

export async function getFullscreenPref(): Promise<boolean> {
  try {
    return (await invoke<boolean | null>("config_get", { key: FULLSCREEN_KEY })) ?? false;
  } catch (e) {
    console.error("[homedesktop] get fullscreen pref failed:", e);
    return false;
  }
}

export async function setFullscreenPref(on: boolean): Promise<void> {
  try {
    await invoke("config_set", { key: FULLSCREEN_KEY, value: on });
  } catch (e) {
    console.error("[homedesktop] set fullscreen pref failed:", e);
  }
}

// ---------- 可配置全局快捷键（M7/M10）：togglePad + search 两个动作 ----------

const TOGGLE_KEY = "shortcuts.togglePad";
const SEARCH_KEY = "shortcuts.search";

/** 读取 Pad 开关快捷键（默认 alt+space 兜底） */
export async function getToggleShortcut(): Promise<string> {
  try {
    const v = await invoke<string | null>("config_get", { key: TOGGLE_KEY });
    return v && v.trim() ? v : "alt+space";
  } catch (e) {
    console.error("[homedesktop] get toggle shortcut failed:", e);
    return "alt+space";
  }
}

/** 保存 Pad 开关快捷键；返回错误信息（null = 成功，立即生效并持久化） */
export async function setToggleShortcut(combo: string): Promise<string | null> {
  try {
    await invoke("shortcuts_set", { action: "togglePad", value: combo });
    return null;
  } catch (e) {
    console.error("[homedesktop] set toggle shortcut failed:", e);
    return String(e);
  }
}

/** 读取搜索唤起快捷键（默认 ctrl+space 兜底） */
export async function getSearchShortcut(): Promise<string> {
  try {
    const v = await invoke<string | null>("config_get", { key: SEARCH_KEY });
    return v && v.trim() ? v : "ctrl+space";
  } catch (e) {
    console.error("[homedesktop] get search shortcut failed:", e);
    return "ctrl+space";
  }
}

/** 保存搜索唤起快捷键；返回错误信息（null = 成功） */
export async function setSearchShortcut(combo: string): Promise<string | null> {
  try {
    await invoke("shortcuts_set", { action: "search", value: combo });
    return null;
  } catch (e) {
    console.error("[homedesktop] set search shortcut failed:", e);
    return String(e);
  }
}
