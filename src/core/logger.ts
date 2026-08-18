// 前端操作日志：分级写入 Rust 日志系统（homedesktop.log）
// 开发构建默认记录全部；release 构建日志默认关闭（Rust 侧过滤）
import { invoke } from "@tauri-apps/api/core";

let seq = 0;

function send(level: string, message: string): void {
  seq++;
  const line = `[${seq}] ${message}`;
  // fire-and-forget：日志失败不影响功能
  invoke("log_write", { level, message: line }).catch(() => {});
  // 开发时也输出到控制台方便调试
  if (import.meta.env.DEV) {
    const tag = level.toUpperCase();
    if (level === "error") console.error(`[homedesktop:${tag}]`, message);
    else if (level === "warn") console.warn(`[homedesktop:${tag}]`, message);
    else console.log(`[homedesktop:${tag}]`, message);
  }
}

export const log = {
  debug: (m: string) => send("debug", m),
  info: (m: string) => send("info", m),
  warn: (m: string) => send("warn", m),
  error: (m: string) => send("error", m),
};
