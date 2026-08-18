<script lang="ts">
  import IconTile from "./IconTile.svelte";
  import FolderTile from "./FolderTile.svelte";
  import WidgetTile from "./WidgetTile.svelte";
  import type { Cell, IconCell, PluginInfo } from "../core/types";
  import { enterEditMode, plugins, ui } from "../core/stores.svelte";
  import { filterCells } from "../core/search";

  let {
    cells,
    queryText = "",
    onlaunch,
    ondelete,
    onaddclick,
    onopenfolder,
    oneditfolder,
    onediticon,
    onmoveicon,
    onresize,
    onsettings,
    onreorder,
    ondropinto,
    onflipprev,
    onflipnext,
  }: {
    cells: Cell[];
    queryText?: string;
    onlaunch?: (pluginId: string) => void;
    ondelete?: (id: string) => void;
    onaddclick?: () => void;
    onopenfolder?: (folderId: string) => void;
    oneditfolder?: (folderId: string) => void;
    onediticon?: (iconId: string) => void;
    onmoveicon?: (iconId: string) => void;
    onresize?: (iconId: string) => void;
    onsettings?: (cellId: string) => void;
    onreorder?: (dragId: string, targetId: string, pos: "before" | "after") => void;
    ondropinto?: (dragId: string, folderId: string) => void;
    onflipprev?: () => void;
    onflipnext?: () => void;
  } = $props();

  // ---------- 拖拽（Pointer Events：鼠标移动即拖、触屏长按拾取） ----------
  // Tauri/WebView2 中 HTML5 原生 DnD 不可靠且触屏不支持，故自行实现。

  let gridEl: HTMLElement | undefined;
  let dragging = false;
  let draggingId = $state<string | null>(null);
  let overId = $state<string | null>(null);
  let overPos = $state<"before" | "after" | "into">("before");
  let suppressClick = false;

  let pointerId: number | null = null;
  let dragCandidate: string | null = null;
  let startX = 0;
  let startY = 0;
  let longPressTimer: ReturnType<typeof setTimeout> | undefined;
  let edgeTimer: ReturnType<typeof setTimeout> | undefined;
  let lastFlip = 0;

  function pluginOf(icon: IconCell): PluginInfo | undefined {
    return plugins.find((p) => p.id === icon.pluginId);
  }

  function isWidgetCell(cell: Cell): boolean {
    return cell.kind === "icon" && pluginOf(cell)?.pluginType === "widget";
  }

  function cellById(id: string | null): Cell | undefined {
    return id ? cells.find((c) => c.id === id) : undefined;
  }

  const visible = $derived(filterCells(cells, plugins, queryText));

  // ---------- 事件 ----------

  function onPointerDown(e: PointerEvent): void {
    if ((e.target as HTMLElement).closest("button")) return; // 忽略操作按钮
    const tile = (e.target as HTMLElement).closest<HTMLElement>("[data-cell-id]");
    if (!tile) return;
    pointerId = e.pointerId;
    dragCandidate = tile.dataset.cellId ?? null;
    startX = e.clientX;
    startY = e.clientY;
    if (e.pointerType === "touch" && dragCandidate) {
      // 触屏：长按 → 进入编辑模式并拾取（苹果风格）
      longPressTimer = setTimeout(() => {
        if (!ui.editMode) enterEditMode();
        beginDrag(dragCandidate!);
      }, 350);
    }
  }

  function onPointerMove(e: PointerEvent): void {
    if (dragging) {
      e.preventDefault();
      updateOver(e.clientX, e.clientY);
      edgeFlip(e.clientX);
      return;
    }
    if (pointerId === null || pointerId !== e.pointerId || !dragCandidate) return;
    const dist = Math.hypot(e.clientX - startX, e.clientY - startY);
    if (e.pointerType === "touch") {
      if (ui.editMode) {
        // 编辑模式下：小幅移动即拾取拖拽
        if (dist > 6) {
          clearLongPress();
          beginDrag(dragCandidate);
        }
      } else if (dist > 14) {
        // 正常模式：快速滑动视为滚动
        clearLongPress();
      }
    } else if (dist > 4) {
      beginDrag(dragCandidate);
    }
  }

  function onPointerUp(e: PointerEvent): void {
    clearLongPress();
    if (!dragging) return;
    e.preventDefault();
    if (overId && overId !== draggingId) {
      const target = cellById(overId);
      if (target?.kind === "folder") {
        ondropinto?.(draggingId!, overId);
      } else {
        onreorder?.(draggingId!, overId, overPos === "after" ? "after" : "before");
      }
    }
    suppressClick = true;
    endDrag();
  }

  function onPointerCancel(): void {
    clearLongPress();
    if (dragging) endDrag();
  }

  function onClickCapture(e: MouseEvent): void {
    if (suppressClick) {
      e.preventDefault();
      e.stopPropagation();
      suppressClick = false;
    }
  }

  function beginDrag(id: string): void {
    if (dragging) return;
    dragging = true;
    draggingId = id;
    overId = null;
    const pid = pointerId;
    if (gridEl && pid !== null) {
      try {
        gridEl.setPointerCapture(pid);
      } catch {
        /* 忽略 */
      }
    }
  }

  function endDrag(): void {
    dragging = false;
    draggingId = null;
    overId = null;
    dragCandidate = null;
    const pid = pointerId;
    pointerId = null;
    if (gridEl && pid !== null) {
      try {
        gridEl.releasePointerCapture(pid);
      } catch {
        /* 忽略 */
      }
    }
    clearEdgeTimer();
  }

  // ---------- 网格度量与槽位计算 ----------
  // 支持拖到空行/行尾空白/页面边缘：不依赖悬停在已有图标上

  interface SlotMetrics {
    cols: number;
    tile: number;
    gap: number;
    left: number;
    top: number;
  }

  function computeGridMetrics(): SlotMetrics {
    const rect = gridEl?.getBoundingClientRect();
    const style = gridEl ? getComputedStyle(gridEl) : null;
    const tile = style ? parseFloat(style.getPropertyValue("--tile-size")) || 84 : 84;
    const gap = style ? parseFloat(style.getPropertyValue("--gap")) || 16 : 16;
    const padL = style ? parseFloat(style.paddingLeft) || 0 : 0;
    const padR = style ? parseFloat(style.paddingRight) || 0 : 0;
    const padT = style ? parseFloat(style.paddingTop) || 0 : 0;
    const width = rect ? rect.width - padL - padR : 1;
    const cols = Math.max(1, Math.floor((width + gap) / (tile + gap)));
    const contentW = cols * tile + (cols - 1) * gap;
    const left = rect ? rect.left + (rect.width - contentW) / 2 : 0;
    const top = rect ? rect.top + padT : 0;
    return { cols, tile, gap, left, top };
  }

  /** 模拟网格排布（跳过拖动中的单元），返回各单元的行列位置 */
  function placeCells(cols: number): { id: string; row: number; col: number }[] {
    const placements: { id: string; row: number; col: number }[] = [];
    const occupied = new Set<string>();
    let row = 0;
    let col = 0;
    for (const cell of cells) {
      if (cell.id === draggingId) continue;
      // 文件夹视为 1×1
      const w = cell.kind === "folder" ? 1 : cell.size.w;
      const h = cell.kind === "folder" ? 1 : cell.size.h;
      while (occupied.has(`${row},${col}`)) {
        col += 1;
        if (col >= cols) {
          row += 1;
          col = 0;
        }
      }
      placements.push({ id: cell.id, row, col });
      for (let r = 0; r < h; r++) {
        for (let c = 0; c < w; c++) {
          if (col + c < cols) occupied.add(`${row + r},${col + c}`);
        }
      }
      col += w;
      if (col >= cols) {
        row += 1;
        col = 0;
      }
    }
    return placements;
  }

  function updateOver(x: number, y: number): void {
    // 1) 直接悬停在已有单元上：按水平位置精确判断 前/后/入文件夹
    const el = document.elementFromPoint(x, y);
    const tile = (el as HTMLElement | null)?.closest?.<HTMLElement>("[data-cell-id]");
    const targetId = tile?.dataset.cellId ?? null;
    if (targetId && targetId !== draggingId) {
      const target = cellById(targetId);
      if (target?.kind === "folder") {
        overId = targetId;
        overPos = "into";
      } else {
        const rect = tile!.getBoundingClientRect();
        overId = targetId;
        overPos = x < rect.left + rect.width / 2 ? "before" : "after";
      }
      return;
    }

    // 2) 空白/边缘：槽位计算 → 插入到该槽位之后第一个单元之前；没有则追加到末尾
    const m = computeGridMetrics();
    const scol = Math.min(Math.max(Math.floor((x - m.left) / (m.tile + m.gap)), 0), m.cols - 1);
    const srow = Math.max(Math.floor((y - m.top) / (m.tile + m.gap)), 0);
    const placements = placeCells(m.cols);
    const hit = placements.find(
      (p) => p.row > srow || (p.row === srow && p.col >= scol),
    );
    if (hit) {
      overId = hit.id;
      overPos = "before";
    } else if (placements.length > 0) {
      overId = placements[placements.length - 1].id;
      overPos = "after";
    } else {
      overId = null;
    }
  }

  function edgeFlip(x: number): void {
    const edge = 56;
    if (x < edge || x > window.innerWidth - edge) {
      const goPrev = x < edge;
      if (!edgeTimer && Date.now() - lastFlip > 800) {
        edgeTimer = setTimeout(() => {
          edgeTimer = undefined;
          lastFlip = Date.now();
          if (goPrev) onflipprev?.();
          else onflipnext?.();
        }, 650);
      }
    } else if (edgeTimer) {
      clearTimeout(edgeTimer);
      edgeTimer = undefined;
    }
  }

  function clearLongPress(): void {
    if (longPressTimer) {
      clearTimeout(longPressTimer);
      longPressTimer = undefined;
    }
  }

  function clearEdgeTimer(): void {
    if (edgeTimer) {
      clearTimeout(edgeTimer);
      edgeTimer = undefined;
    }
  }
</script>

<div
  class="grid"
  role="grid"
  tabindex="-1"
  class:searching={queryText !== ""}
  bind:this={gridEl}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={onPointerCancel}
  onclick={onClickCapture}
  onkeydown={(e) => {
    if (e.key === "Escape") endDrag();
  }}
>
  {#each visible as cell (cell.id)}
    {@const isOver = overId === cell.id}
    {@const isDragging = draggingId === cell.id}
    {#if cell.kind === "folder"}
      <div
        class="drop-wrap"
        class:over={isOver}
        class:into={isOver && overPos === "into"}
        class:dragging={isDragging}
      >
        <FolderTile
          folder={cell}
          editMode={ui.editMode}
          onopen={() => !ui.editMode && onopenfolder?.(cell.id)}
          onedit={() => oneditfolder?.(cell.id)}
          ondelete={() => ondelete?.(cell.id)}
        />
      </div>
    {:else if isWidgetCell(cell)}
      <div
        class="drop-wrap"
        class:over={isOver}
        class:before={isOver && overPos === "before"}
        class:after={isOver && overPos === "after"}
        class:dragging={isDragging}
        style="grid-column: span {cell.size.w}; grid-row: span {cell.size.h};"
      >
        <WidgetTile item={cell} editMode={ui.editMode} ondelete={() => ondelete?.(cell.id)} onmove={() => onmoveicon?.(cell.id)} onresize={() => onresize?.(cell.id)} onsettings={() => onsettings?.(cell.id)} />
      </div>
    {:else}
      <div
        class="drop-wrap"
        class:over={isOver}
        class:before={isOver && overPos === "before"}
        class:after={isOver && overPos === "after"}
        class:dragging={isDragging}
        style="grid-column: span {cell.size.w}; grid-row: span {cell.size.h};"
      >
        <IconTile
          item={cell}
          plugin={pluginOf(cell)}
          editMode={ui.editMode}
          onlaunch={() => !ui.editMode && onlaunch?.(cell.id)}
          ondelete={() => ondelete?.(cell.id)}
          onmove={() => onmoveicon?.(cell.id)}
          onedit={() => onediticon?.(cell.id)}
          onresize={() => onresize?.(cell.id)}
          onsettings={() => onsettings?.(cell.id)}
        />
      </div>
    {/if}
  {/each}

  {#if queryText === ""}
    <button class="add-tile" onclick={() => onaddclick?.()}>＋</button>
  {/if}
</div>

<style>
  .grid {
    display: grid;
    /* 固定基准粒度：每列恰为 --tile-size，跨 N 列 = N×基准 + (N-1)×gap，严格整数倍 */
    grid-template-columns: repeat(auto-fill, var(--tile-size));
    grid-auto-rows: var(--tile-size);
    justify-content: center;
    gap: var(--gap);
    align-content: start;
    height: 100%;
    overflow-y: auto;
    padding: 4px;
    touch-action: pan-y; /* 触屏：纵向滚动交给浏览器，长按/横向由应用处理 */
  }
  .grid.searching {
    align-content: start;
  }
  .drop-wrap {
    position: relative;
    touch-action: pan-y;
  }
  .drop-wrap.dragging {
    opacity: 0.4;
  }
  .drop-wrap.into {
    outline: 2px dashed var(--accent);
    outline-offset: 3px;
    border-radius: var(--radius);
  }
  .drop-wrap.before::before,
  .drop-wrap.after::after {
    content: "";
    position: absolute;
    top: 8%;
    bottom: 8%;
    width: 4px;
    border-radius: 2px;
    background: var(--accent);
    z-index: 3;
  }
  .drop-wrap.before::before {
    left: -8px;
  }
  .drop-wrap.after::after {
    right: -8px;
  }
  .add-tile {
    height: var(--tile-size);
    border: 2px dashed var(--bg-elev);
    border-radius: var(--radius);
    background: transparent;
    color: var(--fg-dim);
    font-size: 28px;
    cursor: pointer;
    transition: border-color 0.15s, color 0.15s;
  }
  .add-tile:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
</style>
