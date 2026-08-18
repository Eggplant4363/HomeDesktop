// 自由摆放（v3）：页面虚拟网格常量与几何工具（与 homedesktop-core 的 PAGE_COLS/FOLDER_COLS 一致）

/** 主页面虚拟列数（默认/迁移用） */
export const PAGE_COLS = 12;
/** 文件夹内虚拟列数 */
export const FOLDER_COLS = 6;

/** 当前主页面实际列数：Grid 按窗口宽度计算后同步（能放下的最小列数）；默认 12 */
export let activePageCols = PAGE_COLS;
export function setActivePageCols(n: number): void {
  if (Number.isInteger(n) && n >= 1) activePageCols = n;
}

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export function rectsOverlap(a: Rect, b: Rect): boolean {
  return a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
}

/** 在 occupied 矩形集中找 w×h 的首个空位（行优先扫描；超出列宽换行） */
export function findFreeSlot(
  occupied: Rect[],
  cols: number,
  w: number,
  h: number,
): { x: number; y: number } {
  for (let y = 0; y < 500; y++) {
    for (let x = 0; x < cols; x++) {
      if (x + w > cols) continue;
      const r: Rect = { x, y, w, h };
      if (!occupied.some((o) => rectsOverlap(r, o))) return { x, y };
    }
  }
  return { x: 0, y: 0 };
}

/** 单元 → 占位矩形（缺坐标视为 0,0；缺 size 视为 1x1） */
export function cellRect(cell: {
  x?: number;
  y?: number;
  size?: { w: number; h: number };
}): Rect {
  return { x: cell.x ?? 0, y: cell.y ?? 0, w: cell.size?.w ?? 1, h: cell.size?.h ?? 1 };
}
