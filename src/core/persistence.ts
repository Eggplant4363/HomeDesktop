// 布局持久化：读写 app 数据目录下的 layout.json（由 Rust 命令实现）
import { invoke } from "@tauri-apps/api/core";
import type { Layout } from "./types";
import { setLayout } from "./stores.svelte";

export async function loadLayout(): Promise<void> {
  try {
    const data = await invoke<Layout | null>("layout_load");
    if (data) setLayout(data);
  } catch (e) {
    console.error("[homedesktop] loadLayout failed:", e);
  }
}

export async function saveLayout(layout: Layout): Promise<void> {
  try {
    await invoke("layout_save", { layout });
  } catch (e) {
    console.error("[homedesktop] saveLayout failed:", e);
  }
}
