// 插件加载器 + 应用抽屉：调用 Rust 命令
import { invoke } from "@tauri-apps/api/core";
import type { PluginInfo } from "./types";
import { setPlugins } from "./stores.svelte";

export interface AppInfo {
  name: string;
  path: string;
}

export async function loadPlugins(): Promise<void> {
  try {
    const list = await invoke<PluginInfo[]>("plugins_list");
    setPlugins(list);
  } catch (e) {
    console.error("[homedesktop] loadPlugins failed:", e);
  }
}

export async function launchPlugin(pluginId: string): Promise<void> {
  await invoke("launch_action", { pluginId });
}

/** 按单元格 id 启动（图标自带动作优先，回退插件） */
export async function launchCell(cellId: string): Promise<void> {
  await invoke("launch_cell", { cellId });
}

/** 扫描系统已安装应用 */
export async function scanApps(): Promise<AppInfo[]> {
  try {
    return await invoke<AppInfo[]>("apps_scan");
  } catch (e) {
    console.error("[homedesktop] apps_scan failed:", e);
    return [];
  }
}

/** 从本地 zip 安装插件包（返回安装后的插件） */
export async function installPlugin(zipPath: string): Promise<PluginInfo> {
  return await invoke<PluginInfo>("plugins_install", { zipPath });
}
