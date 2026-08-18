<script lang="ts">
  // 主桌面网格（v3 自由摆放）：虚拟画布（PAGE_COLS 列）内绝对定位，拖拽吸附网格、不自动重排
  import IconTile from "./IconTile.svelte";
  import FolderTile from "./FolderTile.svelte";
  import WidgetTile from "./WidgetTile.svelte";
  import type { Cell, IconCell, PluginInfo } from "../core/types";
  import { enterEditMode, plugins, ui } from "../core/stores.svelte";
  import { appearance } from "../core/appearance.svelte";
  import { filterCells } from "../core/search";
  import { cellRect, findFreeSlot, PAGE_COLS } from "../core/layout";

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
    ondropat,
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
    /** 自由摆放落点：dragId 放到 (x, y) 网格坐标 */
    ondropat?: (dragId: string, x: number, y: number) => void;
    ondropinto?: (dragId: string, folderId: string) => void;
    onflipprev?: () => void;
    onflipnext?: () => void;
  } = $props();

  // ---------- 拖拽（Pointer Events：鼠标移动即拖、触屏长按拾取；仅编辑模式可移动） ----------

  let gridEl: HTMLElement | undefined;
  let canvasEl: HTMLElement | undefined;

  /** CSS 变量（--tile-size / --gap），挂载与窗口变化时读取 */
  let tile = $state(84);
  let gap = $state(16);
  const PAD = 6;
  const SLOT = $derived(tile + gap);

  let dragging = false;
  let draggingId = $state<string | null>(null);
  let dragSlot = $state<{ x: number; y: number; w: number; h: number } | null>(null);
  let overFolderId = $state<string | null>(null);
  let dragDx = $state(0);
  let dragDy = $state(0);
  let suppressClick = false;

  let pointerId: number | null = null;
  let dragCandidate: string | null = null;
  let startX = 0;
  let startY = 0;
  let dragStartPointerX = 0;
  let dragStartPointerY = 0;
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

  /** 单元像素几何（画布内） */
  function pxX(cell: Cell): number {
    return PAD + (cell.x ?? 0) * SLOT;
  }
  function pxY(cell: Cell): number {
    return PAD + (cell.y ?? 0) * SLOT;
  }
  function pxW(cell: Cell): number {
    const w = cell.kind === "folder" ? 1 : cell.size.w;
    return w * tile + (w - 1) * gap;
  }
  function pxH(cell: Cell): number {
    const h = cell.kind === "folder" ? 1 : cell.size.h;
    return h * tile + (h - 1) * gap;
  }

  const canvasW = $derived(PAGE_COLS * tile + (PAGE_COLS - 1) * gap + PAD * 2);
  const canvasH = $derived.by(() => {
    let max = 0;
    for (const cell of cells) {
      if (cell.id === draggingId) continue;
      const bottom = (cell.y ?? 0) + (cell.kind === "folder" ? 1 : cell.size.h);
      max = Math.max(max, bottom * SLOT);
    }
    return max + PAD + 10;
  });

  /** ＋ 新增按钮位置：页面上首个空位 */
  const addPos = $derived(
    queryText === ""
      ? findFreeSlot(cells.map(cellRect), PAGE_COLS, 1, 1)
      : null,
  );

  function measure(): void {
    if (!gridEl) return;
    const style = getComputedStyle(gridEl);
    const t = parseFloat(style.getPropertyValue("--tile-size"));
    const g = parseFloat(style.getPropertyValue("--gap"));
    if (Number.isFinite(t) && t > 0) tile = t;
    if (Number.isFinite(g) && g >= 0) gap = g;
  }

  // 首次挂载与图标大小变化时重新度量（tile/gap 用于画布尺寸与拖拽吸附计算）
  $effect(() => {
    void appearance.tileSize;
    measure();
  });

  function onPointerDown(e: PointerEvent): void {
    if ((e.target as HTMLElement).closest("button")) return; // 忽略操作按钮
    const el = (e.target as HTMLElement).closest<HTMLElement>("[data-cell-id]");
    if (!el) return;
    pointerId = e.pointerId;
    dragCandidate = el.dataset.cellId ?? null;
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
      dragDx = e.clientX - dragStartPointerX;
      dragDy = e.clientY - dragStartPointerY;
      updateTarget(e.clientX, e.clientY);
      edgeFlip(e.clientX);
      return;
    }
    if (pointerId === null || pointerId !== e.pointerId || !dragCandidate) return;
    const dist = Math.hypot(e.clientX - startX, e.clientY - startY);
    if (e.pointerType === "touch") {
      if (ui.editMode) {
        if (dist > 6) {
          clearLongPress();
          beginDrag(dragCandidate);
        }
      } else if (dist > 14) {
        clearLongPress();
      }
    } else if (dist > 4 && ui.editMode) {
      // 仅编辑模式可拖拽移动（非编辑模式点击/滑动不拾取）
      beginDrag(dragCandidate);
    }
  }

  function onPointerUp(e: PointerEvent): void {
    clearLongPress();
    if (!dragging) return;
    e.preventDefault();
    if (overFolderId && overFolderId !== draggingId) {
      ondropinto?.(draggingId!, overFolderId);
    } else if (dragSlot && dragSlot.x >= 0) {
      ondropat?.(draggingId!, dragSlot.x, dragSlot.y);
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
    const cell = cellById(id);
    const w = cell?.kind === "folder" ? 1 : cell?.size.w ?? 1;
    const h = cell?.kind === "folder" ? 1 : cell?.size.h ?? 1;
    dragSlot = { x: cell?.x ?? 0, y: cell?.y ?? 0, w, h };
    overFolderId = null;
    dragStartPointerX = startX;
    dragStartPointerY = startY;
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
    dragSlot = null;
    overFolderId = null;
    dragDx = 0;
    dragDy = 0;
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

  /** 计算落点（吸附网格）与文件夹拖入目标 */
  function updateTarget(x: number, y: number): void {
    if (!canvasEl || !draggingId) return;
    const cell = cellById(draggingId);
    const w = cell?.kind === "folder" ? 1 : cell?.size.w ?? 1;
    const h = cell?.kind === "folder" ? 1 : cell?.size.h ?? 1;
    const rect = canvasEl.getBoundingClientRect();
    const slot = SLOT;
    const tx = Math.round((x - rect.left - PAD - (w * slot) / 2) / slot);
    const ty = Math.round((y - rect.top - PAD - (h * slot) / 2) / slot);
    dragSlot = {
      x: Math.max(0, Math.min(PAGE_COLS - w, tx)),
      y: Math.max(0, ty),
      w,
      h,
    };
    // 悬停在文件夹上 → 拖入文件夹
    const el = document.elementFromPoint(x, y);
    const tileEl = (el as HTMLElement | null)?.closest?.<HTMLElement>("[data-cell-id]");
    const tid = tileEl?.dataset.cellId ?? null;
    overFolderId =
      tid && tid !== draggingId && cellById(tid)?.kind === "folder" ? tid : null;
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
  <div
    class="canvas"
    bind:this={canvasEl}
    style="width:{canvasW}px;height:{canvasH}px;"
  >
    {#each visible as cell (cell.id)}
      {@const isDragging = draggingId === cell.id}
      {@const isFolderOver = overFolderId === cell.id}
      {#if cell.kind === "folder"}
        <div
          class="drop-wrap"
          class:dragging={isDragging}
          class:folder-over={isFolderOver}
          data-cell-id={cell.id}
          style="left:{pxX(cell)}px;top:{pxY(cell)}px;width:{pxW(cell)}px;height:{pxH(cell)}px;{isDragging ? `transform: translate(${dragDx}px,${dragDy}px);z-index:20;opacity:.8;` : ""}"
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
          class:dragging={isDragging}
          data-cell-id={cell.id}
          style="left:{pxX(cell)}px;top:{pxY(cell)}px;width:{pxW(cell)}px;height:{pxH(cell)}px;{isDragging ? `transform: translate(${dragDx}px,${dragDy}px);z-index:20;opacity:.8;` : ""}"
        >
          <WidgetTile
            item={cell}
            editMode={ui.editMode}
            ondelete={() => ondelete?.(cell.id)}
            onmove={() => onmoveicon?.(cell.id)}
            onresize={() => onresize?.(cell.id)}
            onsettings={() => onsettings?.(cell.id)}
          />
        </div>
      {:else}
        <div
          class="drop-wrap"
          class:dragging={isDragging}
          data-cell-id={cell.id}
          style="left:{pxX(cell)}px;top:{pxY(cell)}px;width:{pxW(cell)}px;height:{pxH(cell)}px;{isDragging ? `transform: translate(${dragDx}px,${dragDy}px);z-index:20;opacity:.8;` : ""}"
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

    {#if dragSlot && !overFolderId}
      <div
        class="slot-ghost"
        style="left:{PAD + dragSlot.x * SLOT}px;top:{PAD + dragSlot.y * SLOT}px;width:{dragSlot.w * tile + (dragSlot.w - 1) * gap}px;height:{dragSlot.h * tile + (dragSlot.h - 1) * gap}px;"
      ></div>
    {/if}

    {#if addPos}
      <button
        class="add-tile"
        style="left:{PAD + addPos.x * SLOT}px;top:{PAD + addPos.y * SLOT}px;width:{tile}px;height:{tile}px;"
        onclick={() => onaddclick?.()}
      >＋</button>
    {/if}
  </div>
</div>

<style>
  .grid {
    position: relative;
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
    touch-action: pan-y; /* 触屏：纵向滚动交给浏览器，长按/横向由应用处理 */
    /* 容器 tabindex=-1 可被点击聚焦；按键后 Chromium 会画默认黑色 focus ring → 禁用 */
    outline: none;
  }
  .grid:focus,
  .grid:focus-visible {
    outline: none;
  }
  .canvas {
    position: relative;
    margin: 0 auto;
  }
  .drop-wrap {
    position: absolute;
    touch-action: pan-y;
  }
  .drop-wrap.dragging {
    transition: none;
  }
  .drop-wrap.folder-over {
    outline: 3px solid var(--accent);
    outline-offset: 2px;
    border-radius: var(--radius);
  }
  .slot-ghost {
    position: absolute;
    border: 2px dashed var(--accent);
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    pointer-events: none;
    z-index: 5;
  }
  .add-tile {
    position: absolute;
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
