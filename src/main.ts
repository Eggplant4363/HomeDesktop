import { mount } from "svelte";
import App from "./App.svelte";
import "./styles.css";
import { log } from "./core/logger";

// 全局 JS 错误捕获：任何未捕获异常/拒绝都写入应用日志，便于定位"点了没反应"
window.addEventListener("error", (e) => {
  log.error(`JS 错误: ${e.message} @ ${e.filename}:${e.lineno}:${e.colno}`);
});
window.addEventListener("unhandledrejection", (e) => {
  const reason = e.reason instanceof Error ? e.reason.message : String(e.reason);
  log.error(`未处理的 Promise 拒绝: ${reason}`);
});

const app = mount(App, {
  target: document.getElementById("app")!,
});

export default app;
