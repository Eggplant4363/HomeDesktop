// 插件搜索名注册（统一接口）
// 插件/小组件可注册额外的可搜索名称（如 HA 的实体友好名"卧室灯"），
// 全局搜索即可命中；点击结果触发 action（如定位到对应小组件）。
import type { Component } from "svelte";

export interface SearchEntry {
  /** 命中名称（大小写不敏感匹配） */
  label: string;
  /** 副标题（如实体 ID / 所属插件） */
  sublabel?: string;
  /** 结果图标 emoji */
  emoji?: string;
  /** 点击结果时执行（可选） */
  action?: () => void;
  /** 归属插件 id（排序/去重用） */
  pluginId?: string;
}

/** pluginId -> 注册的搜索条目（$state 响应式） */
export const pluginSearchEntries = $state<Record<string, SearchEntry[]>>({});

/** 插件注册搜索名（幂等；多次调用覆盖该插件的列表） */
export function registerSearchNames(pluginId: string, entries: SearchEntry[]): void {
  pluginSearchEntries[pluginId] = entries;
}

/** 插件注销搜索名 */
export function unregisterSearchNames(pluginId: string): void {
  delete pluginSearchEntries[pluginId];
}

/** 扁平化全部条目（供搜索匹配）——内部 derived，用 getter 读取（Svelte 不允许直接导出 derived） */
const _all = $derived.by(() => Object.values(pluginSearchEntries).flat());
/** 读取全部搜索条目（响应式） */
export function getAllSearchEntries(): SearchEntry[] {
  return _all;
}

/** 读取某个单元格注册的搜索条目（小组件按 cell.<cellId> 注册） */
export function getSearchEntriesForCell(cellId: string): SearchEntry[] {
  return pluginSearchEntries[`cell.${cellId}`] ?? [];
}

/** 已全量抓取过的提供商（模块级单例，避免每个 widget 重复抓取） */
const fetchedProviders = new Set<string>();
/** 尝试标记提供商已抓取；返回是否首次（true=应执行抓取） */
export function markProviderFetched(pid: string): boolean {
  if (fetchedProviders.has(pid)) return false;
  fetchedProviders.add(pid);
  return true;
}
