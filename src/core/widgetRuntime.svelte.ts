// 小组件运行时：为插件提供"数据缓存 + 定时刷新"机制
//
// 解决的问题：切换页面时小组件组件被卸载重挂，网络型小组件（如天气）每次都会重新请求。
// 业界方案（Android AppWidget / iOS WidgetKit）：小组件数据由宿主统一管理，
// 生命周期与页面切换解耦——数据缓存常驻，定时器按周期后台刷新。
//
// 用法：
//   插件模块加载时 registerWidget({ id, refreshMs, fetch })
//   组件挂载时从 widgetCache 同步渲染缓存；运行时定时器会按 refreshMs 后台刷新，
//   刷新后 widgetCache 更新，所有正在渲染该小组件的组件自动更新。

export interface WidgetModule<T> {
  /** 插件/小组件唯一 id */
  id: string;
  /** 拉取数据（网络/IO） */
  fetch: () => Promise<T>;
  /** 刷新周期（毫秒） */
  refreshMs: number;
}

/** 缓存（$state 使组件渲染响应刷新） */
export const widgetCache = $state<Record<string, { data: unknown; fetchedAt: number }>>({});

const modules = new Map<string, WidgetModule<unknown>>();
let schedulerStarted = false;

/** 注册小组件模块（幂等）；首个注册时启动全局调度器 */
export function registerWidget<T>(mod: WidgetModule<T>): void {
  modules.set(mod.id, mod as WidgetModule<unknown>);
  ensureScheduler();
}

export function getWidgetData<T>(id: string): T | undefined {
  return widgetCache[id]?.data as T | undefined;
}

export function getWidgetFetchedAt(id: string): number {
  return widgetCache[id]?.fetchedAt ?? 0;
}

/** 立即后台刷新（组件挂载时若缓存过期可调用；不阻塞首屏）。
 *  返回是否真的执行了拉取（false = 未注册该实例/拉取失败） */
export async function refreshWidget(id: string): Promise<boolean> {
  const mod = modules.get(id);
  if (!mod) return false;
  try {
    const data = await mod.fetch();
    widgetCache[id] = { data, fetchedAt: Date.now() };
    return true;
  } catch (e) {
    console.error(`[homedesktop] widget ${id} refresh failed:`, e);
    return false;
  }
}

/** 缓存是否过期 */
export function isWidgetStale(id: string, refreshMs: number): boolean {
  const entry = widgetCache[id];
  return !entry || Date.now() - entry.fetchedAt > refreshMs;
}

/** 全局调度器：每分钟检查一次，过期即后台刷新（与页面切换无关） */
function ensureScheduler(): void {
  if (schedulerStarted) return;
  schedulerStarted = true;
  setInterval(() => {
    for (const mod of modules.values()) {
      if (isWidgetStale(mod.id, mod.refreshMs)) {
        void refreshWidget(mod.id);
      }
    }
  }, 60_000);
}
