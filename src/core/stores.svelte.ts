// 全局状态（Svelte 5 runes 模块）
import type { Cell, IconCell, Layout, PluginInfo } from "./types";
import { log } from "./logger";
import { activePageCols, activePageRows, cellRect, findFreeSlot, FOLDER_COLS, rectsOverlap } from "./layout";

export const plugins = $state<PluginInfo[]>([]);
export const layout = $state<Layout>({ version: 3, pages: [[]] });
export const query = $state({ text: "" });
export const currentPage = $state({ index: 0 });
/** 当前打开的文件夹（null = 主网格视图） */
export const openFolder = $state<{ page: number; folderId: string | null }>({
  page: 0,
  folderId: null,
});
/** 编辑模式（苹果风格：进入后图标抖动、显示缩放/删除等操作；正常模式点击即启动） */
export const ui = $state({ editMode: false });

export function enterEditMode(): void {
  ui.editMode = true;
}

export function exitEditMode(): void {
  ui.editMode = false;
}

function cloneCell(cell: Cell): Cell {
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

// ---------- 单元格操作 ----------

/** 在页面空闲处放置新单元（自由摆放：自动找首个空位；当前页放不下 → 自动放到下一页，直到新建页）；返回实际放置的页码 */
export function addCell(cell: Cell, page = currentPage.index): number {
  const w = cell.kind === "folder" ? 1 : cell.size.w;
  const h = cell.kind === "folder" ? 1 : cell.size.h;
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
    const w = cell.kind === "folder" ? 1 : cell.size.w;
    const h = cell.kind === "folder" ? 1 : cell.size.h;
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

/** 确定性重排（大块优先、行优先）：结果只取决于单元集合，必然收敛（防止振荡死循环） */
function repackDeterministic(arr: Cell[], cols: number, rows: number | undefined): void {
  const ordered = [...arr].sort((a, b) => {
    const ha = a.kind === "folder" ? 1 : a.size.h;
    const hb = b.kind === "folder" ? 1 : b.size.h;
    if (hb !== ha) return hb - ha;
    const wa = a.kind === "folder" ? 1 : a.size.w;
    const wb = b.kind === "folder" ? 1 : b.size.w;
    if (wb !== wa) return wb - wa;
    return (a.id ?? "").localeCompare(b.id ?? "");
  });
  const placed: { x: number; y: number; w: number; h: number }[] = [];
  for (const cell of ordered) {
    const w = cell.kind === "folder" ? 1 : cell.size.w;
    const h = cell.kind === "folder" ? 1 : cell.size.h;
    const slot = findFreeSlot(placed, cols, w, h, rows);
    cell.x = slot.x;
    cell.y = slot.y;
    placed.push({ x: slot.x, y: slot.y, w, h });
  }
}

/** 本页放不下的单元 → 自动分页到下一页空位（放不下继续往后，直到新建页）；返回移走数量 */
function paginateOverflow(
  arr: Cell[],
  page: number,
  cols: number,
  rows: number | undefined,
): number {
  if (!rows) return 0;
  const overflow = arr.filter(
    (c) => (c.y ?? 0) + (c.kind === "folder" ? 1 : c.size.h) > rows,
  );
  if (overflow.length === 0) return 0;
  overflow.sort(
    (a, b) =>
      (a.y ?? 0) - (b.y ?? 0) || (a.x ?? 0) - (b.x ?? 0) ||
      (a.kind === "folder" ? 1 : a.size.w) - (b.kind === "folder" ? 1 : b.size.w),
  );
  for (const c of overflow) {
    const i = arr.indexOf(c);
    if (i >= 0) arr.splice(i, 1);
  }
  let p = page + 1;
  for (const c of overflow) {
    const w = c.kind === "folder" ? 1 : c.size.w;
    const h = c.kind === "folder" ? 1 : c.size.h;
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

/** 画布适配：越界/重叠 → 画布内空位；仍超出最大行数 → 自动分页（不显示滚动条）。
 *  多轮适配仍不稳定（如小数尺寸引起振荡）→ 确定性重排兜底，保证收敛、绝不死循环。返回是否发生调整 */
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
    // 多次适配仍不稳定（振荡）→ 确定性重排，必然收敛
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
export function removeCell(id: string, page = currentPage.index): void {
  const arr = layout.pages[page];
  if (!arr) return;
  const i = arr.findIndex((c) => c.id === id);
  if (i >= 0) arr.splice(i, 1);
}

/** 设置单元位置（自由摆放 v3）：目标槽被占用则与占用单元交换位置；单元在别的页则移动到目标页 */
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
  const movingW = moving.kind === "icon" ? moving.size.w : 1;
  const movingH = moving.kind === "icon" ? moving.size.h : 1;
  const movingRect = { x: sx, y: sy, w: movingW, h: movingH };

  // 目标槽与其他单元重叠 → 交换位置（不自动重排）
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

/** 设置文件夹内图标位置（同样：占用则交换） */
export function setIconPosition(folderId: string, iconId: string, x: number, y: number): boolean {
  const found = findFolder(folderId);
  if (!found) return false;
  const items = found.folder.items;
  const sx = Math.max(0, Math.min(FOLDER_COLS - 1, Math.round(x)));
  const sy = Math.max(0, Math.round(y));
  const icon = items.find((i) => i.id === iconId);
  if (!icon) return false;
  const iconRect = { x: sx, y: sy, w: icon.size.w, h: icon.size.h };
  const other = items.find((i) => i.id !== iconId && rectsOverlap(iconRect, cellRect(i)));
  if (other) {
    const ox = other.x ?? 0;
    const oy = other.y ?? 0;
    other.x = icon.x ?? 0;
    other.y = icon.y ?? 0;
    icon.x = ox;
    icon.y = oy;
  } else {
    icon.x = sx;
    icon.y = sy;
  }
  return true;
}

/** 找到文件夹单元格（含所在页） */
export function findFolder(
  folderId: string,
): { page: number; folder: Extract<Cell, { kind: "folder" }> } | undefined {
  for (let p = 0; p < layout.pages.length; p++) {
    for (const cell of layout.pages[p]) {
      if (cell.kind === "folder" && cell.id === folderId) {
        return { page: p, folder: cell };
      }
    }
  }
  return undefined;
}

export function createFolder(name: string, emoji = "📁"): { id: string; page: number } {
  const id = crypto.randomUUID();
  const page = addCell({ kind: "folder", id, name, emoji, items: [] });
  return { id, page };
}

export function deleteFolder(folderId: string): void {
  for (let p = 0; p < layout.pages.length; p++) {
    const arr = layout.pages[p];
    const i = arr.findIndex((c) => c.kind === "folder" && c.id === folderId);
    if (i >= 0) {
      arr.splice(i, 1);
      if (openFolder.folderId === folderId) openFolder.folderId = null;
      return;
    }
  }
}

/** 向文件夹内添加图标（文件夹可能在任何页；自由摆放：自动找空位） */
export function addIconToFolder(folderId: string, icon: IconCell): boolean {
  const found = findFolder(folderId);
  if (!found) return false;
  const slot = findFreeSlot(found.folder.items.map(cellRect), FOLDER_COLS, icon.size.w, icon.size.h);
  found.folder.items.push({ ...icon, x: slot.x, y: slot.y });
  return true;
}

/** 从文件夹内移除图标 */
export function removeIconFromFolder(folderId: string, iconId: string): void {
  const found = findFolder(folderId);
  if (!found) return;
  const i = found.folder.items.findIndex((x) => x.id === iconId);
  if (i >= 0) found.folder.items.splice(i, 1);
}

/** 文件夹内重排（M8）：把 dragId 移到 targetId 之前/之后；targetId 为 null 时追加到末尾 */
export function reorderIconInFolder(
  folderId: string,
  dragId: string,
  targetId: string | null,
  pos: "before" | "after",
): boolean {
  const found = findFolder(folderId);
  if (!found) return false;
  const items = found.folder.items;
  const from = items.findIndex((x) => x.id === dragId);
  if (from < 0) return false;
  const [icon] = items.splice(from, 1);
  if (!targetId) {
    items.push(icon);
    return true;
  }
  const to = items.findIndex((x) => x.id === targetId);
  if (to < 0) {
    items.push(icon);
    return true;
  }
  items.splice(to + (pos === "after" ? 1 : 0), 0, icon);
  return true;
}

/** 文件夹重命名（M8） */
export function renameFolder(folderId: string, name: string): boolean {
  const found = findFolder(folderId);
  if (!found) return false;
  const trimmed = name.trim();
  if (trimmed) found.folder.name = trimmed;
  return true;
}

/** 文件夹换 emoji（M8） */
export function setFolderEmoji(folderId: string, emoji: string): boolean {
  const found = findFolder(folderId);
  if (!found) return false;
  const trimmed = emoji.trim();
  if (trimmed) found.folder.emoji = trimmed;
  return true;
}

/** 把图标移入文件夹（在所有页面与文件夹内查找源图标；支持跨页/跨文件夹移动；入文件夹自动找空位） */
export function moveIconToFolder(iconId: string, folderId: string): boolean {
  const target = findFolder(folderId);
  if (!target) return false;
  const slot = findFreeSlot(target.folder.items.map(cellRect), FOLDER_COLS, 1, 1);

  // 1) 页面级图标（跨页拖拽/按钮移动）
  for (let p = 0; p < layout.pages.length; p++) {
    const arr = layout.pages[p];
    const i = arr?.findIndex((c) => c.kind === "icon" && c.id === iconId) ?? -1;
    if (i >= 0) {
      const icon = arr[i] as IconCell;
      arr.splice(i, 1);
      target.folder.items.push({ ...icon, x: slot.x, y: slot.y });
      return true;
    }
  }

  // 2) 文件夹内图标（移到另一个文件夹；不能移进自己所在的文件夹）
  for (const page of layout.pages) {
    for (const cell of page) {
      if (cell.kind === "folder" && cell.id !== folderId) {
        const i = cell.items.findIndex((x) => x.id === iconId);
        if (i >= 0) {
          const [icon] = cell.items.splice(i, 1);
          target.folder.items.push({ ...icon, x: slot.x, y: slot.y });
          return true;
        }
      }
    }
  }
  return false;
}

/** 自定义图标外观（M9）：重命名 / 换 emoji / 换颜色 / 借用系统应用图标（传 undefined 不改，传空字符串=清除） */
export function updateIconAppearance(
  cellId: string,
  patch: {
    title?: string;
    emoji?: string;
    color?: string;
    iconPath?: string;
  },
): boolean {
  const apply = (icon: IconCell): void => {
    if (patch.title !== undefined && patch.title.trim()) icon.title = patch.title.trim();
    if (patch.emoji !== undefined) icon.emoji = patch.emoji ? patch.emoji : undefined;
    if (patch.color !== undefined) icon.color = patch.color ? patch.color : undefined;
    if (patch.iconPath !== undefined) icon.iconPath = patch.iconPath ? patch.iconPath : undefined;
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

/** 原位替换图标为应用（系统应用插件槽位）：保持原位置与 id，改标题/动作/插件归属 */
export function replaceCellWithApp(
  cellId: string,
  app: { name: string; path: string },
): boolean {
  for (const page of layout.pages) {
    for (const cell of page) {
      if (cell.kind === "icon" && cell.id === cellId) {
        cell.pluginId = "builtin.app";
        cell.title = app.name;
        cell.action = { kind: "app", path: app.path };
        return true;
      }
      if (cell.kind === "folder") {
        const icon = cell.items.find((i) => i.id === cellId);
        if (icon) {
          icon.pluginId = "builtin.app";
          icon.title = app.name;
          icon.action = { kind: "app", path: app.path };
          return true;
        }
      }
    }
  }
  return false;
}

/** 移除某插件的所有图标（页面 + 文件夹内；M11 卸载插件时清理桌面） */
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

/** 设置单元格尺寸（w/h 必须为正整数，1×1 基准的整数倍） */
export function setCellSize(
  id: string,
  size: { w: number; h: number },
  page = currentPage.index,
): boolean {
  // 0.5 格步进（自定义尺寸粒度减半），上限 8
  const snap = (n: number) => Math.max(1, Math.min(8, Math.round(n * 2) / 2));
  const w = snap(size.w);
  const h = snap(size.h);

  const arr = layout.pages[page];
  const cell = arr?.find((c) => c.id === id);
  if (cell && cell.kind === "icon") {
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

// ---------- 页面操作 ----------

export function addPage(): void {
  layout.pages.push([]);
}

export function removePage(index = currentPage.index): void {
  if (layout.pages.length <= 1) return;
  layout.pages.splice(index, 1);
  if (currentPage.index >= layout.pages.length) {
    currentPage.index = layout.pages.length - 1;
  }
}

// ---------- 拖拽排序 ----------

/** 页内重排：把 dragId 单元移到 targetId 单元之前/之后 */
export function reorderCellById(
  dragId: string,
  targetId: string,
  pos: "before" | "after",
  page = currentPage.index,
): boolean {
  const arr = layout.pages[page];
  if (!arr) return false;
  const from = arr.findIndex((c) => c.id === dragId);
  const to = arr.findIndex((c) => c.id === targetId);
  if (from < 0 || to < 0 || from === to) return false;
  const [cell] = arr.splice(from, 1);
  let insertAt = arr.findIndex((c) => c.id === targetId);
  if (insertAt < 0) insertAt = arr.length;
  if (pos === "after") insertAt += 1;
  arr.splice(insertAt, 0, cell);
  return true;
}

/** 跨页移动（自由摆放 v3）：按 id 在所有页面中找源单元，移除并放到当前页指定坐标（支持边缘翻页拖拽） */
export function moveCellAcrossPages(dragId: string, x: number, y: number): boolean {
  let fromPage = -1;
  let fromIndex = -1;
  for (let p = 0; p < layout.pages.length; p++) {
    const i = layout.pages[p].findIndex((c) => c.id === dragId);
    if (i >= 0) {
      fromPage = p;
      fromIndex = i;
      break;
    }
  }
  if (fromPage < 0) return false;

  const targetArr = layout.pages[currentPage.index];
  if (!targetArr) return false;

  const [cell] = layout.pages[fromPage].splice(fromIndex, 1);
  const sx = Math.max(0, Math.min(activePageCols - 1, Math.round(x)));
  const sy = Math.max(0, Math.round(y));
  cell.x = sx;
  cell.y = sy;
  targetArr.push(cell);
  return true;
}
