// 插件设置：统一的键值存取
// - 插件级（默认）：config key = "plugin.<pluginId>.<settingKey>"（所有实例共享的默认值）
// - 实例级（每个图标独立）：config key = "cell.<cellId>.<settingKey>"；未设置时回退插件级默认
// 带内存缓存（$state）保证设置变更时所有组件响应式更新
import { getConfig, setConfig } from "./config";

/** 设置缓存（$state 响应式） */
export const pluginSettings = $state<Record<string, unknown>>({});

function cacheKey(pluginId: string, key: string): string {
  return `plugin.${pluginId}.${key}`;
}

function cellCacheKey(cellId: string, key: string): string {
  return `cell.${cellId}.${key}`;
}

/** 读取插件级设置（实例回退用）；未设置时返回 default */
export async function getPluginSetting<T>(
  pluginId: string,
  key: string,
  fallback?: T,
): Promise<T | undefined> {
  const ck = cacheKey(pluginId, key);
  if (ck in pluginSettings) return pluginSettings[ck] as T;
  const v = await getConfig(ck);
  const val = (v as T | null) ?? fallback;
  pluginSettings[ck] = val;
  return val;
}

/** 写入插件级设置（更新缓存 + 持久化） */
export async function setPluginSetting(
  pluginId: string,
  key: string,
  value: unknown,
): Promise<void> {
  const ck = cacheKey(pluginId, key);
  pluginSettings[ck] = value;
  await setConfig(ck, value);
}

/** 同步读插件级缓存 */
export function peekPluginSetting<T>(pluginId: string, key: string): T | undefined {
  return pluginSettings[cacheKey(pluginId, key)] as T | undefined;
}

// ---------- 实例级设置（每个图标独立；回退插件级默认） ----------

/** 读取实例设置；未设置时回退插件级设置，再回退 default */
export async function getCellSetting<T>(
  cellId: string,
  pluginId: string,
  key: string,
  fallback?: T,
): Promise<T | undefined> {
  const ck = cellCacheKey(cellId, key);
  if (ck in pluginSettings) return pluginSettings[ck] as T;
  const v = await getConfig(ck);
  if (v !== null && v !== undefined) {
    pluginSettings[ck] = v as T;
    return v as T;
  }
  // 回退插件级默认（兼容旧的共享设置，如 plugin.dev.homedesktop.weather.city）
  return getPluginSetting(pluginId, key, fallback);
}

/** 写入实例设置（只影响当前图标） */
export async function setCellSetting(
  cellId: string,
  pluginId: string,
  key: string,
  value: unknown,
): Promise<void> {
  const ck = cellCacheKey(cellId, key);
  pluginSettings[ck] = value;
  await setConfig(ck, value);
}

/** 同步读实例缓存（优先实例级，其次插件级缓存） */
export function peekCellSetting<T>(cellId: string, pluginId: string, key: string): T | undefined {
  const ck = cellCacheKey(cellId, key);
  if (ck in pluginSettings) return pluginSettings[ck] as T;
  return peekPluginSetting<T>(pluginId, key);
}
