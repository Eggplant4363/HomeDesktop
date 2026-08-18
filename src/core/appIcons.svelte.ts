// 系统应用真实图标缓存：按路径懒加载（app_icon 命令 → PNG data URL），
// 加载中/失败时由组件回退首字母头像。缓存为 $state 普通对象（与 pluginSettings 一致，
// 避免 $state Map 代理在模块级不触发模板重渲染的问题）。
import { invoke } from "@tauri-apps/api/core";
import { log } from "./logger";

/** path → data URL（$state 响应式） */
export const appIcons = $state<Record<string, string>>({});

const pending = new Map<string, Promise<string | null>>();

// 并发限制（M14）：系统应用面板一次挂载会并发请求上百个图标，
// 限制同时最多 MAX_CONCURRENT 个，避免洪泛 IPC 与后端线程池。
const MAX_CONCURRENT = 6;
let active = 0;
const waiters: (() => void)[] = [];

function acquire(): Promise<void> {
  if (active < MAX_CONCURRENT) {
    active++;
    return Promise.resolve();
  }
  return new Promise((resolve) => {
    waiters.push(() => {
      active++;
      resolve();
    });
  });
}

function release(): void {
  active--;
  const next = waiters.shift();
  if (next) next();
}

/** 加载并缓存某个路径的系统图标（幂等，可并发调用；受并发限制） */
export async function loadAppIcon(path: string): Promise<string | null> {
  const cached = appIcons[path];
  if (cached) return cached;
  let p = pending.get(path);
  if (!p) {
    p = (async () => {
      await acquire();
      try {
        return await invoke<string | null>("app_icon", { path });
      } finally {
        release();
      }
    })()
      .then((url) => {
        if (url) {
          appIcons[path] = url;
          log.debug(`app_icon 结果: ${path} -> ${url.length} bytes`);
        } else {
          log.debug(`app_icon 结果: ${path} -> null`);
        }
        return url;
      })
      .catch((e) => {
        log.debug(`app_icon 失败: ${path} -> ${e}`);
        return null;
      })
      .finally(() => {
        pending.delete(path);
      });
    pending.set(path, p);
  }
  return p;
}
