// 单元格操作（阶段3 从 stores.svelte.ts 拆出）
// 画布适配（越界/重叠→空位、超行自动分页）、增删单元、位置交换、尺寸、重叠修复、
// 拖拽排序与跨页移动、插件相关批量增删改。

import type { Cell, IconCell, Layout, PluginInfo } from "./types";
import { log } from "./logger";
import {
  activePageCols,
  activePageRows,
  cellRect,
  findFreeSlot,
  FOLDER_COLS,
  rectsOverlap,
} from "./layout";
import { cloneCell, currentPage, layout } from "./layoutStore.svelte";

/** 一轮适配：把越界/重叠单元移到画布内空位（不重排正常单元） */
function fitPass(
  arr: Cell[],
  cols: number,
  rows: number | undefined,
): { changed: boolean; moved: number } {
  const placed: { x: number; y: number; w: number; h: number }[] = [];
  let changed = false;
  let moved = 0;
  for (const cell of arr) {
    const w = cell.kind === "folder" ? (cell.size?.w ?? 1) : cell.size.w;
    const h = cell.kind === "folder" ? (cell.size?.h ?? 1) : cell.size.h;
    const x = cell.x ?? 0;
    const y = cell.y ?? 0;
    const rect = { x, y, w, h };
    const fits =
      x + w <= cols && (!rows || y + h <= rows) && !placed.some((p) => rectsOverlap(rect, p));
    if (fits) {
      placed.push(rect);
      continue;
    }
    const slot = findFreeSlot(placed, cols, w, h, rows);
    cell.x = slot.x;
    cell.y = slot.y;
    placed.push({ x: slot.x, y: slot.y, w, h });
    changed = true;
    moved += 1;
  }
  return { changed, moved };
}

/** 确定性重排（大块优先、行优先）：结果只取决于单元集合，必然收敛 */
function repackDeterministic(arr: Cell[], cols: number, rows: number | undefined): void {
  const ordered = [...arr].sort((a, b) => {
    const ha = a.kind === "folder" ? (a.size?.h ?? 1) : a.size.h;
    const hb = b.kind === "folder" ? (b.size?.h ?? 1) : b.size.h;
    if (hb !== ha) return hb - ha;
    const wa = a.kind === "folder" ? (a.size?.w ?? 1) : a.size.w;
    const wb = b.kind === "folder" ? (b.size?.w ?? 1) : b.size.w;
    if (wb !== wa) return wb - wa;
    return (a.id ?? "").localeCompare(b.id ?? "");
  });
  const placed: { x: number; y: number; w: number; h: number }[] = [];
  for (const cell of ordered) {
    const w = cell.kind === "folder" ? (cell.size?.w ?? 1) : cell.size.w;
    const h = cell.kind === "folder" ? (cell.size?.h ?? 1) : cell.size.h;
    const slot = findFreeSlot(placed, cols, w, h, rows);
    cell.x = slot.x;
    cell.y = slot.y;
    placed.push({ x: slot.x, y: slot.y, w, h });
  }
}

/** 本页放不下的单元 → 自动分页到下一页空位；返回移走数量 */
function paginateOverflow(
  arr: Cell[],
  page: number,
  cols: number,
  rows: number | undefined,
): number {
  if (!rows) return 0;
  const overflow = arr.filter(
    (c) => (c.y ?? 0) + (c.kind === "folder" ? (c.size?.h ?? 1) : c.size.h) > rows,
  );
  if (overflow.length === 0) return 0;
  overflow.sort(
    (a, b) =>
      (a.y ?? 0) - (b.y ?? 0) ||
      (a.x ?? 0) - (b.x ?? 0) ||
      (a.kind === "folder" ? (a.size?.w ?? 1) : a.size.w) -
        (b.kind === "folder" ? (b.size?.w ?? 1) : b.size.w),
  );
  for (const c of overflow) {
    const i = arr.indexOf(c);
    if (i >= 0) arr.splice(i, 1);
  }
  let p = page + 1;
  for (const c of overflow) {
    const w = c.kind === "folder" ? (c.size?.w ?? 1) : c.size.w;
    const h = c.kind === "folder" ? (c.size?.h ?? 1) : c.size.h;
    for (;;) {
      if (!layout.pages[p]) layout.pages[p] = [];
      const slot = findFreeSlot(layout.pages[p].map(cellRect), cols, w, h, rows);
      if (slot.x + w <= cols && slot.y + h <= rows) {
        c.x = slot.x;
        c.y = slot.y;
        layout.pages[p].push(c);
        break;
      }
      p++;
    }
  }
  return overflow.length;
}

/** 画布适配：越界/重叠 → 画布内空位；仍超出最大行数 → 自动分页。返回是否发生调整 */
export function fitCellsToCols(page: number, cols: number, maxRows?: number): boolean {
  const arr = layout.pages[page];
  if (!arr || cols < 1) return false;
  const rows = maxRows && maxRows > 0 ? maxRows : undefined;
  let moved = 0;
  let rounds = 0;
  for (let round = 0; round < 4; round++) {
    const r = fitPass(arr, cols, rows);
    if (!r.changed) break;
    moved += r.moved;
    rounds = round + 1;
  }
  if (rounds >= 4) {
    repackDeterministic(arr, cols, rows);
    moved += arr.length;
  }
  const overflowMoved = paginateOverflow(arr, page, cols, rows);
  moved += overflowMoved;
  if (moved > 0)
    log.info(
      `画布适配: 第 ${page + 1} 页 ${cols} 列${rows ? ` ${rows} 行` : ""}，调整 ${moved} 个单元${overflowMoved > 0 ? `（其中 ${overflowMoved} 个自动移到下一页）` : ""}${rounds >= 4 ? "（重排兜底）" : ""}`,
    );
  return moved > 0;
}

/** 在页面空闲处放置新单元（放不下自动放到下一页）；返回实际放置的页码 */
export function addCell(cell: Cell, page = currentPage.index): number {
  const w = cell.kind === "folder" ? (cell.size?.w ?? 1) : cell.size.w;
  const h = cell.kind === "folder" ? (cell.size?.h ?? 1) : cell.size.h;
  for (let p = page; ; p++) {
    if (!layout.pages[p]) layout.pages[p] = [];
    const cells = layout.pages[p];
    const slot = findFreeSlot(cells.map(cellRect), activePageCols, w, h, activePageRows);
    if (slot.x + w <= activePageCols && slot.y + h <= activePageRows) {
      cells.push({ ...cloneCell(cell), x: slot.x, y: slot.y });
      if (p > page)
        log.info(`第 ${page + 1} 页已满，自动放到第 ${p + 1} 页（${w}x${h}）`);
      return p;
    }
  }
}

export function removeCell(id: string, page = currentPage.index): void {
  const arr = layout.pages[page];
  if (!arr) return;
  const i = arr.findIndex((c) => c.id === id);
  if (i >= 0) arr.splice(i, 1);
}

/** 设置单元位置：目标槽被占用则与占用单元交换位置；单元在别的页则移动到目标页 */
export function setCellPosition(
  cellId: string,
  x: number,
  y: number,
  page = currentPage.index,
): boolean {
  const targetArr = layout.pages[page];
  if (!targetArr) return false;
  const sx = Math.max(0, Math.min(activePageCols - 1, Math.round(x)));
  const sy = Math.max(0, Math.round(y));

  let fromPage = -1;
  let fromIndex = -1;
  for (let p = 0; p < layout.pages.length; p++) {
    const i = layout.pages[p].findIndex((c) => c.id === cellId);
    if (i >= 0) {
      fromPage = p;
      fromIndex = i;
      break;
    }
  }
  if (fromPage < 0) return false;

  const moving = layout.pages[fromPage][fromIndex];
  const movingW = moving.kind === "icon" ? moving.size.w : (moving.size?.w ?? 1);
  const movingH = moving.kind === "icon" ? moving.size.h : (moving.size?.h ?? 1);
  const movingRect = { x: sx, y: sy, w: movingW, h: movingH };

  const other = targetArr.find((c) => c.id !== cellId && rectsOverlap(movingRect, cellRect(c)));
  if (other) {
    const otherX = other.x ?? 0;
    const otherY = other.y ?? 0;
    other.x = moving.x ?? 0;
    other.y = moving.y ?? 0;
    moving.x = otherX;
    moving.y = otherY;
  } else {
    moving.x = sx;
    moving.y = sy;
  }
  if (fromPage !== page) {
    const [cell] = layout.pages[fromPage].splice(fromIndex, 1);
    targetArr.push(cell);
  }
  return true;
}

/** 设置单元格尺寸（w/h 取整，1..8） */
export function setCellSize(
  id: string,
  size: { w: number; h: number },
  page = currentPage.index,
): boolean {
  const snap = (n: number) => Math.max(1, Math.min(8, Math.round(n)));
  const w = snap(size.w);
  const h = snap(size.h);

  const arr = layout.pages[page];
  const cell = arr?.find((c) => c.id === id);
  if (cell && cell.kind === "icon") {
    cell.size = { w, h };
    return true;
  }
  if (cell && cell.kind === "folder") {
    cell.size = { w, h };
    return true;
  }
  // 可能在文件夹内
  for (const pageCells of layout.pages) {
    for (const c of pageCells) {
      if (c.kind === "folder") {
        const icon = c.items.find((i) => i.id === id);
        if (icon) {
          icon.size = { w, h };
          return true;
        }
      }
    }
  }
  return false;
}

/** 保存前修复：小数尺寸/坐标取整（Rust 端只接受整数） */
export function repairLayoutForPersist(): void {
  for (const page of layout.pages) {
    for (const c of page) {
      if (c.kind === "icon") {
        c.size.w = Math.round(c.size.w);
        c.size.h = Math.round(c.size.h);
        if (c.x !== undefined) c.x = Math.round(c.x);
        if (c.y !== undefined) c.y = Math.round(c.y);
      } else if (c.kind === "folder") {
        for (const it of c.items) {
          it.size.w = Math.round(it.size.w);
          it.size.h = Math.round(it.size.h);
          if (it.x !== undefined) it.x = Math.round(it.x);
          if (it.y !== undefined) it.y = Math.round(it.y);
        }
        if (c.x !== undefined) c.x = Math.round(c.x);
        if (c.y !== undefined) c.y = Math.round(c.y);
      }
    }
  }
}

/** 修复页面内重叠/越界：把有问题的单元移到同页空闲位置。返回是否发生调整 */
export function repairPageOverlaps(page = currentPage.index): boolean {
  const arr = layout.pages[page];
  if (!arr || arr.length < 2) return false;
  const ordered = [...arr].sort(
    (a, b) => (a.y ?? 0) - (b.y ?? 0) || (a.x ?? 0) - (b.x ?? 0) || a.id.localeCompare(b.id),
  );
  const occupied: { x: number; y: number; w: number; h: number }[] = [];
  let dirty = false;
  for (const cell of ordered) {
    const w = cell.kind === "folder" ? (cell.size?.w ?? 1) : cell.size.w;
    const h = cell.kind === "folder" ? (cell.size?.h ?? 1) : cell.size.h;
    const rect = { x: cell.x ?? 0, y: cell.y ?? 0, w, h };
    const outOfBounds = rect.x + rect.w > activePageCols || rect.y + rect.h > activePageRows;
    const overlaps = occupied.some((o) => rectsOverlap(rect, o));
    if (outOfBounds || overlaps) {
      const slot = findFreeSlot(occupied, activePageCols, w, h, activePageRows);
      cell.x = slot.x;
      cell.y = slot.y;
      dirty = true;
    }
    occupied.push({ x: cell.x ?? 0, y: cell.y ?? 0, w, h });
  }
  return dirty;
}

/** 自定义图标外观：重命名/emoji/颜色/借用图标/图片图标/是否显示名称（undefined=不改，空串=清除） */
export function updateIconAppearance(
  cellId: string,
  patch: {
    title?: string;
    emoji?: string;
    color?: string;
    iconPath?: string;
    iconImage?: string;
    showLabel?: boolean;
  },
): boolean {
  const apply = (icon: IconCell): void => {
    if (patch.title !== undefined && patch.title.trim()) icon.title = patch.title.trim();
    if (patch.emoji !== undefined) icon.emoji = patch.emoji ? patch.emoji : undefined;
    if (patch.color !== undefined) icon.color = patch.color ? patch.color : undefined;
    if (patch.iconPath !== undefined) icon.iconPath = patch.iconPath ? patch.iconPath : undefined;
    if (patch.iconImage !== undefined) icon.iconImage = patch.iconImage ? patch.iconImage : undefined;
    if (patch.showLabel !== undefined) icon.showLabel = patch.showLabel;
  };
  for (const page of layout.pages) {
    for (const cell of page) {
      if (cell.kind === "icon" && cell.id === cellId) {
        apply(cell);
        return true;
      }
      if (cell.kind === "folder") {
        const icon = cell.items.find((i) => i.id === cellId);
        if (icon) {
          apply(icon);
          return true;
        }
      }
    }
  }
  return false;
}

/** 原位替换图标为应用/文件/文件夹（系统应用槽位）：保持位置与 id */
export function replaceCellWithApp(
  cellId: string,
  app: { name: string; path: string },
  emoji?: string,
  iconPath?: string,
): boolean {
  for (const page of layout.pages) {
    for (const cell of page) {
      if (cell.kind === "icon" && cell.id === cellId) {
        cell.pluginId = "builtin.app";
        cell.title = app.name;
        cell.action = { kind: "app", path: app.path };
        cell.emoji = emoji;
        cell.iconPath = iconPath;
        return true;
      }
      if (cell.kind === "folder") {
        const icon = cell.items.find((i) => i.id === cellId);
        if (icon) {
          icon.pluginId = "builtin.app";
          icon.title = app.name;
          icon.action = { kind: "app", path: app.path };
          icon.emoji = emoji;
          icon.iconPath = iconPath;
          return true;
        }
      }
    }
  }
  return false;
}

/** 移除某插件的所有图标（页面 + 文件夹内） */
export function removeCellsByPlugin(pluginId: string): number {
  let n = 0;
  for (const page of layout.pages) {
    for (let i = page.length - 1; i >= 0; i--) {
      const c = page[i];
      if (c.kind === "icon" && c.pluginId === pluginId) {
        page.splice(i, 1);
        n++;
      } else if (c.kind === "folder") {
        const items = c.items;
        for (let j = items.length - 1; j >= 0; j--) {
          if (items[j].pluginId === pluginId) {
            items.splice(j, 1);
            n++;
          }
        }
      }
    }
  }
  return n;
}

/** 页面内按 id 重排（把 dragId 插到 targetId 位置） */
export function reorderCellById(
  page: number,
  dragId: string,
  targetId: string,
): boolean {
  const arr = layout.pages[page];
  if (!arr) return false;
  const from = arr.findIndex((c) => c.id === dragId);
  const to = arr.findIndex((c) => c.id === targetId);
  if (from < 0 || to < 0 || from === to) return false;
  const [cell] = arr.splice(from, 1);
  arr.splice(to, 0, cell);
  return true;
}

/** 跨页移动单元到指定坐标 */
export function moveCellAcrossPages(dragId: string, x: number, y: number): boolean {
  const sx = Math.max(0, Math.round(x));
  const sy = Math.max(0, Math.round(y));
  for (let p = 0; p < layout.pages.length; p++) {
    const i = layout.pages[p].findIndex((c) => c.id === dragId);
    if (i >= 0) {
      const [cell] = layout.pages[p].splice(i, 1);
      cell.x = sx;
      cell.y = sy;
      if (!layout.pages[currentPage.index]) layout.pages[currentPage.index] = [];
      layout.pages[currentPage.index].push(cell);
      return true;
    }
  }
  return false;
}
