// 文件夹操作（阶段3 从 stores.svelte.ts 拆出）
// 查找、创建、删除、内部图标增删改排、重命名/换 emoji、移入文件夹、打开时紧凑重排。

import type { Cell, IconCell } from "./types";
import { cellRect, findFreeSlot, FOLDER_COLS, rectsOverlap } from "./layout";
import { layout, openFolder } from "./layoutStore.svelte";
import { addCell } from "./cellOps.svelte";

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

/** 新建文件夹（自动找空位，可能落在后续页） */
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

/** 设置文件夹内图标位置（占用则交换） */
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

/** 打开文件夹时：内部图标从左上角紧凑排列。返回是否发生变动 */
export function repackFolder(folderId: string): boolean {
  const found = findFolder(folderId);
  if (!found) return false;
  const items = found.folder.items;
  let cx = 0,
    cy = 0,
    rowH = 0;
  let dirty = false;
  for (const icon of items) {
    const w = icon.size.w;
    const h = icon.size.h;
    if (cx + w > FOLDER_COLS) {
      cx = 0;
      cy += rowH;
      rowH = 0;
    }
    if ((icon.x ?? 0) !== cx || (icon.y ?? 0) !== cy) {
      icon.x = cx;
      icon.y = cy;
      dirty = true;
    }
    cx += w;
    rowH = Math.max(rowH, h);
  }
  return dirty;
}

/** 向文件夹内添加图标（自动找空位） */
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

/** 文件夹内重排：把 dragId 移到 targetId 之前/之后；targetId 为 null 追加末尾 */
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

/** 文件夹重命名 */
export function renameFolder(folderId: string, name: string): boolean {
  const found = findFolder(folderId);
  if (!found) return false;
  const trimmed = name.trim();
  if (trimmed) found.folder.name = trimmed;
  return true;
}

/** 文件夹换 emoji */
export function setFolderEmoji(folderId: string, emoji: string): boolean {
  const found = findFolder(folderId);
  if (!found) return false;
  const trimmed = emoji.trim();
  if (trimmed) found.folder.emoji = trimmed;
  return true;
}

/** 把图标移入文件夹（支持跨页/跨文件夹；自动找空位） */
export function moveIconToFolder(iconId: string, folderId: string): boolean {
  const target = findFolder(folderId);
  if (!target) return false;
  const slot = findFreeSlot(target.folder.items.map(cellRect), FOLDER_COLS, 1, 1);

  // 1) 页面级图标
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

  // 2) 文件夹内图标（移到另一个文件夹）
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
