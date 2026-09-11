// 自定义图标："来自图片" 的选择与压缩（阶段1 从 App.svelte 抽出）
// 用 Rust 命令读取图片文件 → canvas 压缩到限定边长 → data URL（避免布局文件过大）。

import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { log } from "./logger";
import { editor, toast } from "./editorState.svelte";

/** data URL 图片压缩到 maxSize 边长内（canvas） */
export function downscaleDataUrl(dataUrl: string, maxSize: number): Promise<string> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => {
      try {
        const scale = Math.min(1, maxSize / Math.max(img.width, img.height));
        const w = Math.max(1, Math.round(img.width * scale));
        const h = Math.max(1, Math.round(img.height * scale));
        const canvas = document.createElement("canvas");
        canvas.width = w;
        canvas.height = h;
        const ctx = canvas.getContext("2d");
        if (!ctx) throw new Error("canvas 不可用");
        ctx.drawImage(img, 0, 0, w, h);
        resolve(canvas.toDataURL("image/png"));
      } catch (e) {
        reject(e);
      }
    };
    img.onerror = () => reject(new Error("图片解码失败"));
    img.src = dataUrl;
  });
}

/** 选择图片作为图标（写回编辑态；点保存生效） */
export async function pickIconImage(): Promise<void> {
  try {
    const picked = await open({
      multiple: false,
      filters: [
        { name: "图片", extensions: ["png", "jpg", "jpeg", "webp", "gif", "bmp", "ico"] },
      ],
    });
    if (typeof picked !== "string" || !picked) return;
    const dataUrl = await invoke<string | null>("image_to_data_url", { path: picked });
    if (!dataUrl) {
      toast("不支持的图片格式");
      return;
    }
    editor.iconEditIconImage = await downscaleDataUrl(dataUrl, 256);
    editor.iconEditIconPath = "";
    toast("已选择图片图标（点保存生效）");
    log.info(`选择图片图标: ${picked}`);
  } catch (e) {
    log.error(`选择图片图标失败: ${e}`);
    toast(`图片读取失败：${e}`);
  }
}
