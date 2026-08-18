// 通用键值配置（对应 Rust 命令 config_get / config_set）
import { invoke } from "@tauri-apps/api/core";

export async function getConfig(key: string): Promise<unknown> {
  try {
    return await invoke("config_get", { key });
  } catch (e) {
    console.error("[homedesktop] config_get failed:", key, e);
    return undefined;
  }
}

export async function setConfig(key: string, value: unknown): Promise<void> {
  try {
    await invoke("config_set", { key, value });
  } catch (e) {
    console.error("[homedesktop] config_set failed:", key, e);
  }
}
