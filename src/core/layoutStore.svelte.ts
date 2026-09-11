// 核心状态（阶段3 从 stores.svelte.ts 拆出）
// 只放"全局可变状态 + 其最基础的操作"，其余按职责分到 cellOps/folderOps。
// stores.svelte.ts 作为聚合门面 re-export 本模块，保证既有 import 路径不变。

import type { Cell, Layout, PluginInfo } from "./types";

export const plugins = $state<PluginInfo[]>([]);
export const layout = $state<Layout>({ version: 3, pages: [[]] });
export const query = $state({ text: "" });
export const currentPage = $state({ index: 0 });
/** 当前打开的文件夹（null = 主网格视图） */
export const openFolder = $state<{ page: number; folderId: string | null }>({
  page: 0,
  folderId: null,
});
/** 编辑模式（进入后图标抖动、显示缩放/删除等操作；正常模式点击即启动） */
export const ui = $state({ editMode: false });

export function enterEditMode(): void {
  ui.editMode = true;
}

export function exitEditMode(): void {
  ui.editMode = false;
}

/** 深拷贝单元格（文件夹需连 items 一起拷） */
export function cloneCell(cell: Cell): Cell {
  if (cell.kind === "folder") {
    return { ...cell, items: cell.items.map((i) => ({ ...i })) };
  }
  return { ...cell };
}

export function setPlugins(list: PluginInfo[]): void {
  plugins.splice(0, plugins.length, ...list);
}

export function setLayout(next: Layout): void {
  layout.version = next.version;
  layout.pages.splice(
    0,
    layout.pages.length,
    ...next.pages.map((page) => page.map(cloneCell)),
  );
  if (currentPage.index >= layout.pages.length) currentPage.index = 0;
  openFolder.folderId = null;
}

// ---------- 显示页数（紧凑重排时用） ----------

let _displayPageCount = $state(0);

/** 显示层页数：分辨率变化紧凑重排时由 Grid 同步（0=用存储页数） */
export function getDisplayPageCount(): number {
  return _displayPageCount;
}
export function setDisplayPageCount(n: number): void {
  if (Number.isInteger(n) && n >= 0) _displayPageCount = n;
}

// ---------- 搜索聚焦 ----------

let _focus = $state<string | null>(null);

/** 请求聚焦某单元格：切换其所在页 */
export function focusCell(id: string): void {
  for (let p = 0; p < layout.pages.length; p++) {
    const has = layout.pages[p].some(
      (c) => c.id === id || (c.kind === "folder" && c.items.some((i) => i.id === id)),
    );
    if (has) {
      currentPage.index = p;
      break;
    }
  }
  _focus = id;
}

export function getFocusCell(): string | null {
  return _focus;
}

export function clearFocusCell(): void {
  _focus = null;
}

// ---------- 页面操作 ----------

export function addPage(): void {
  layout.pages.push([]);
}

export function removePage(index = currentPage.index): void {
  if (layout.pages.length <= 1) return;
  layout.pages.splice(index, 1);
  if (currentPage.index >= layout.pages.length) currentPage.index = layout.pages.length - 1;
}
