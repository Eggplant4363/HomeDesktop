<script lang="ts">
  // 主桌面网格（v3 自由摆放）：虚拟画布（PAGE_COLS 列）内绝对定位，拖拽吸附网格、不自动重排
  import IconTile from "./IconTile.svelte";
  import FolderTile from "./FolderTile.svelte";
  import WidgetTile from "./WidgetTile.svelte";
  import type { Cell, IconCell, PluginInfo } from "../core/types";
  import { currentPage, enterEditMode, plugins, setDisplayPageCount, ui } from "../core/stores.svelte";
  import { appearance } from "../core/appearance.svelte";
  import { filterCells } from "../core/search";
  import { PAGE_COLS, setActivePageCols, setActivePageRows } from "../core/layout";
  import { log } from "../core/logger";

  let {
    cells,
    /** 全部页面（紧凑重排时跨页按原顺序分页显示用） */
    pages,
    queryText = "",
    onlaunch,
    ondelete,
    onaddclick,
    onopenfolder,
    oneditfolder,
    onediticon,
    onmoveicon,
    onresize,
    onresizeto,
    onresizeend,
    onsettings,
    ondropat,
    ondropinto,
    onflipprev,
    onflipnext,
    onwheelnav,
    onfitted,
    onblankclick,
    /** 新添加的单元 id：播放"弹出 + 光环"入场特效（无则不高亮） */
    highlightId = null,
    /** 页面左右滑动切换：拖动中实时位移（px，阻尼后）；松手时原始位移（决定方向） */
    onswipemove,
    onswipeend,
  }: {
    cells: Cell[];
    pages: Cell[][];
    queryText?: string;
    onlaunch?: (pluginId: string) => void;
    ondelete?: (id: string) => void;
    onaddclick?: () => void;
    onopenfolder?: (folderId: string) => void;
    oneditfolder?: (folderId: string) => void;
    onediticon?: (iconId: string) => void;
    onmoveicon?: (iconId: string) => void;
    onresize?: (iconId: string) => void;
    onresizeto?: (iconId: string, w: number, h: number) => void;
    onresizeend?: (iconId: string) => void;
    onsettings?: (cellId: string) => void;
    highlightId?: string | null;
    /** 自由摆放落点：dragId 放到 (x, y) 网格坐标 */
    ondropat?: (dragId: string, x: number, y: number) => void;
    ondropinto?: (dragId: string, folderId: string) => void;
    onflipprev?: () => void;
    onflipnext?: () => void;
    /** 滚轮翻页（带滑动动画） */
    onwheelnav?: (dir: 1 | -1) => void;
    /** 画布适配调整了布局后回调（用于持久化） */
    onfitted?: () => void;
    /** 点击空白区域（非编辑模式、非拖拽）→ 外层隐藏应用 */
    onblankclick?: () => void;
    onswipemove?: (dx: number) => void;
    onswipeend?: (dx: number) => void;
  } = $props();

  // ---------- 拖拽（Pointer Events：鼠标移动即拖、触屏长按拾取；仅编辑模式可移动） ----------

  let gridEl: HTMLElement | undefined;
  let canvasEl: HTMLElement | undefined;

  /** CSS 变量（--tile-size / --gap），挂载与窗口变化时读取 */
  let tile = $state(84);
  let gap = $state(16);
  const PAD = 6;
  const SLOT = $derived(tile + gap);

  /** 当前窗口能放下的最小列数（Grid 按宽度计算，同步到 stores 供自动找空位使用） */
  let cols = $state(PAGE_COLS);
  /** 视口能容纳的最大行数（画布高度边界，超出则压缩） */
  let maxRows = $state(500);

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
  // 页面左右滑动切页（正常模式：鼠标左键/触屏）
  const SWIPE_START = 24; // 判定为滑动的起始位移 px
  const SWIPE_FACTOR = 0.55; // 手指位移 → 页面位移 阻尼
  let swipeActive = false; // 滑动进行中（非响应式）
  let swipeRawDx = 0; // 松手时的原始位移
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

  /** 窗口放不下（分辨率/窗口变小，任一页图标越界）：需要显示层紧凑重排 */
  const needsPacked = $derived.by(() => {
    if (queryText) return false;
    for (const pg of pages) {
      for (const cell of pg) {
        const w = cell.kind === "folder" ? (cell.size?.w ?? 1) : cell.size.w;
        const h = cell.kind === "folder" ? (cell.size?.h ?? 1) : cell.size.h;
        if ((cell.x ?? 0) + w > cols) return true;
        if ((cell.y ?? 0) + h > maxRows) return true;
      }
    }
    return false;
  });

  /** 紧凑重排（仅显示层，不修改存储坐标）——**每页独立、页内滚动、不跨页**：
   *  每个存储页的图标按原顺序（行 → 列）在当前列数下逐行填充到**本页自己的显示页**；
   *  本页放不下 → 页面纵向滚动查看（**页1的图标永远在页1**，不会散到第2/3/4页）；
   *  页数保持与存储页一致（不产生"差别大"的额外页）；
   *  分辨率恢复后自动还原存储布局 */
  const packedPages = $derived.by(() => {
    const out: { page: number; x: number; y: number; cell: Cell }[] = [];
    if (!needsPacked) return out;
    for (let pi = 0; pi < pages.length; pi++) {
      const pg = pages[pi];
      if (pg.length === 0) continue;
      const ordered = [...pg].sort((a, b) => {
        const ay = a.y ?? 0, by = b.y ?? 0;
        const ax = a.x ?? 0, bx = b.x ?? 0;
        return ay - by || ax - bx || a.id.localeCompare(b.id);
      });
      let cx = 0, cy = 0, rowH = 0;
      for (const cell of ordered) {
        const w = cell.kind === "folder" ? (cell.size?.w ?? 1) : cell.size.w;
        const h = cell.kind === "folder" ? (cell.size?.h ?? 1) : cell.size.h;
        if (cx + w > Math.max(1, cols)) { cx = 0; cy += rowH; rowH = 0; }
        // 不跨页：超出视口行数时继续往下放，画布变高由纵向滚动查看
        out.push({ page: pi, x: cx, y: cy, cell });
        cx += w;
        rowH = Math.max(rowH, h);
      }
    }
    return out;
  });
  /** 紧凑显示页数 = 存储页数（不产生额外页） */
  const packedCount = $derived(pages.length || 1);
  /** 当前显示页（=存储页序号） */
  const dispPage = $derived(needsPacked ? Math.min(currentPage.index, packedCount - 1) : currentPage.index);
  /** 紧凑位置表（全部显示页） */
  const packedPos = $derived.by(() => {
    const map = new Map<string, { x: number; y: number }>();
    for (const p of packedPages) map.set(p.cell.id, { x: p.x, y: p.y });
    return map;
  });

  /** 渲染单元：搜索=命中集；紧凑模式=当前显示页；否则=存储页 */
  const visible = $derived.by(() => {
    if (queryText) return filterCells(cells, plugins, queryText);
    if (needsPacked) return packedPages.filter((p) => p.page === dispPage).map((p) => p.cell);
    return cells;
  });

  /** 紧凑占位（显示用）：搜索命中集 / 紧凑重排结果 */
  const packed = $derived.by(() => {
    if (!queryText && !needsPacked) return new Map<string, { x: number; y: number }>();
    if (needsPacked) return packedPos;
    const map = new Map<string, { x: number; y: number }>();
    let cx = 0, cy = 0, rowH = 0;
    for (const cell of visible) {
      const w = cell.kind === "folder" ? (cell.size?.w ?? 1) : cell.size.w;
      const h = cell.kind === "folder" ? (cell.size?.h ?? 1) : cell.size.h;
      if (cx + w > Math.max(1, cols)) { cx = 0; cy += rowH; rowH = 0; }
      map.set(cell.id, { x: cx, y: cy });
      cx += w;
      rowH = Math.max(rowH, h);
    }
    return map;
  });

  // 紧凑模式：把显示页数同步给外层（翻页/分页指示器用）；0=恢复用存储页数
  $effect(() => {
    void needsPacked;
    void packedCount;
    setDisplayPageCount(needsPacked ? packedCount : 0);
  });

  /** 单元像素几何（画布内）——搜索时用紧凑占位，否则用存储坐标 */
  function effX(cell: Cell): number {
    return packed.get(cell.id)?.x ?? cell.x ?? 0;
  }
  function effY(cell: Cell): number {
    return packed.get(cell.id)?.y ?? cell.y ?? 0;
  }
  function pxX(cell: Cell): number {
    return PAD + effX(cell) * SLOT;
  }
  function pxY(cell: Cell): number {
    return PAD + effY(cell) * SLOT;
  }
  function pxW(cell: Cell): number {
    const w = cell.kind === "folder" ? (cell.size?.w ?? 1) : cell.size.w;
    return w * tile + (w - 1) * gap;
  }
  function pxH(cell: Cell): number {
    const h = cell.kind === "folder" ? (cell.size?.h ?? 1) : cell.size.h;
    return h * tile + (h - 1) * gap;
  }

  const canvasW = $derived(cols * tile + (cols - 1) * gap + PAD * 2);
  const canvasH = $derived.by(() => {
    let max = 0;
    for (const cell of visible) {
      if (cell.id === draggingId) continue;
      const h = cell.kind === "folder" ? (cell.size?.h ?? 1) : cell.size.h;
      const top = packed.get(cell.id)?.y ?? cell.y ?? 0;
      max = Math.max(max, (top + h) * SLOT);
    }
    return max + PAD + 10;
  });


  function measure(): void {
    if (!gridEl) return;
    const style = getComputedStyle(gridEl);
    const t = parseFloat(style.getPropertyValue("--tile-size"));
    const g = parseFloat(style.getPropertyValue("--gap"));

    if (Number.isFinite(t) && t > 0) tile = t;
    if (Number.isFinite(g) && g >= 0) gap = g;
    // 按窗口宽度计算能放下的最小列数（画布恰好铺满，图标不被裁）
    const c = Math.max(1, Math.floor((gridEl.clientWidth - PAD * 2) / SLOT));
    if (c !== cols) {
      cols = c;
      setActivePageCols(c);
    }
    const mr = Math.max(1, Math.floor((gridEl.clientHeight - PAD * 2) / SLOT));
    if (mr !== maxRows) maxRows = mr;
    setActivePageRows(mr);
  }

  // 首次挂载、图标大小/网格间距变化、窗口尺寸变化时重新度量（tile/gap/列数用于画布与吸附计算）
  $effect(() => {
    void appearance.tileSize;
    void appearance.gridSpacing;
    measure();
    const onResize = () => measure();
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  });

  // 注意：分辨率/窗口变化时**不**重排、不持久化（避免永久打乱顺序）。
  // 越界单元由 needsPacked 显示层紧凑排列兜底，分辨率恢复后自动还原存储布局。

  function onPointerDown(e: PointerEvent): void {
    if ((e.target as HTMLElement).closest("button")) return; // 忽略操作按钮
    if (e.pointerType === "mouse" && e.button !== 0) return; // 仅鼠标左键
    pointerId = e.pointerId;
    startX = e.clientX;
    startY = e.clientY;
    swipeActive = false;
    swipeRawDx = 0;
    const el = (e.target as HTMLElement).closest<HTMLElement>("[data-cell-id]");
    dragCandidate = el?.dataset.cellId ?? null;
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
      updateTarget(e.clientX, e.clientY);
      // 幽灵图标吸附到落点（所见即所得：图标显示位置 = 松手后的位置）
      if (dragSlot && draggingId) {
        const dc = cellById(draggingId);
        dragDx = (dragSlot.x - (dc?.x ?? 0)) * SLOT;
        dragDy = (dragSlot.y - (dc?.y ?? 0)) * SLOT;
      }
      edgeFlip(e.clientX);
      return;
    }
    if (pointerId === null || pointerId !== e.pointerId) return;
    // 未按住任何键（鼠标悬停 / 残留状态）→ 清理并忽略，绝不触发滑动
    if (e.buttons === 0) {
      swipeActive = false;
      swipeRawDx = 0;
      pointerId = null;
      dragCandidate = null;
      return;
    }
    // 正常模式：左右滑动切换页面（鼠标左键按住拖动 / 触屏滑动）
    if (!ui.editMode) {
      const dx = e.clientX - startX;
      const dy = e.clientY - startY;
      if (!swipeActive) {
        if (Math.abs(dx) > SWIPE_START && Math.abs(dx) > Math.abs(dy) * 1.2) {
          swipeActive = true;
          clearLongPress();
          // 捕获指针：滑动过程中与松手后的 click 都归网格（避免误触图标/空白）
          try {
            gridEl?.setPointerCapture(e.pointerId);
          } catch {
            /* 忽略 */
          }
        }
      }
      if (swipeActive) {
        e.preventDefault();
        swipeRawDx = dx;
        onswipemove?.((dx - Math.sign(dx) * SWIPE_START) * SWIPE_FACTOR);
        return;
      }
    }
    if (!dragCandidate) return;
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
    if (swipeActive) {
      swipeActive = false;
      suppressClick = true; // 阻止随后的 click（图标启动 / 空白隐藏）
      const dx = swipeRawDx;
      swipeRawDx = 0;
      const pid = pointerId;
      pointerId = null;
      dragCandidate = null;
      try {
        gridEl?.releasePointerCapture(pid!);
      } catch {
        /* 忽略 */
      }
      onswipeend?.(dx);
      return;
    }
    if (!dragging) {
      // 普通点击：同样清理指针状态，避免残留导致"没按住鼠标页面跟着滑动"
      pointerId = null;
      dragCandidate = null;
      return;
    }
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
    swipeActive = false;
    swipeRawDx = 0;
    pointerId = null;
    dragCandidate = null;
    if (dragging) endDrag();
  }

  function onClickCapture(e: MouseEvent): void {
    if (suppressClick) {
      e.preventDefault();
      e.stopPropagation();
      suppressClick = false;
      return;
    }
    // 点击空白区域（画布/网格背景，非图标）→ 隐藏应用（编辑模式/拖拽中不触发）
    if (!ui.editMode && !dragging && (e.target === canvasEl || e.target === gridEl)) {
      onblankclick?.();
    }
  }

  function beginDrag(id: string): void {
    if (dragging) return;
    if (needsPacked) return; // 紧凑显示模式（分辨率变化）下禁用拖拽，避免显示位与存储位错位
    dragging = true;
    draggingId = id;
    const cell = cellById(id);
    const w = cell?.kind === "folder" ? (cell?.size?.w ?? 1) : cell?.size.w ?? 1;
    const h = cell?.kind === "folder" ? (cell?.size?.h ?? 1) : cell?.size.h ?? 1;
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

  /** 计算落点（吸附网格）与文件夹拖入目标。
   *  增量式：落点 = 原始格位 + 拖动距离的格数，抓取位置不影响，移一格即动一格 */
  function updateTarget(x: number, y: number): void {
    if (!canvasEl || !draggingId) return;
    const cell = cellById(draggingId);
    const w = cell?.kind === "folder" ? (cell?.size?.w ?? 1) : cell?.size.w ?? 1;
    const h = cell?.kind === "folder" ? (cell?.size?.h ?? 1) : cell?.size.h ?? 1;
    const slot = SLOT;
    const dx = x - dragStartPointerX;
    const dy = y - dragStartPointerY;
    const tx = (cell?.x ?? 0) + Math.round(dx / slot);
    const ty = (cell?.y ?? 0) + Math.round(dy / slot);
    dragSlot = {
      x: Math.max(0, Math.min(cols - w, tx)),
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

  /** 鼠标滚轮翻页（正常模式）：上滚上一页、下滚下一页；落在可滚动内容上则不翻页 */
  let lastWheelFlip = 0;
  function onWheel(e: WheelEvent): void {
    if (ui.editMode) return;
    let el: HTMLElement | null = e.target as HTMLElement;
    while (el && el !== gridEl) {
      const oy = getComputedStyle(el).overflowY;
      if (oy === "auto" || oy === "scroll") return; // 小组件内部列表：交给它滚动
      el = el.parentElement;
    }
    const delta = e.deltaY !== 0 ? e.deltaY : e.deltaX;
    if (Math.abs(delta) < 20) return;
    e.preventDefault();
    const now = Date.now();
    if (now - lastWheelFlip < 400) return;
    lastWheelFlip = now;
    if (delta > 0) onwheelnav?.(1);
    else onwheelnav?.(-1);
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
  onwheel={onWheel}
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
          class:just-added={highlightId === cell.id && appearance.effects.iconAdd}
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
            onresize={() => onresize?.(cell.id)}
            onresizeto={onresizeto ? (id, w, h) => onresizeto?.(id, w, h) : undefined}
            onresizeend={onresizeend ? (id) => onresizeend?.(id) : undefined}
          />
        </div>
      {:else if isWidgetCell(cell)}
        <div
          class="drop-wrap"
          class:dragging={isDragging}
          class:just-added={highlightId === cell.id && appearance.effects.iconAdd}
          data-cell-id={cell.id}
          style="left:{pxX(cell)}px;top:{pxY(cell)}px;width:{pxW(cell)}px;height:{pxH(cell)}px;{isDragging ? `transform: translate(${dragDx}px,${dragDy}px);z-index:20;opacity:.8;` : ""}"
        >
          <WidgetTile
            item={cell}
            editMode={ui.editMode}
            ondelete={() => ondelete?.(cell.id)}
            onmove={() => onmoveicon?.(cell.id)}
            onedit={() => onediticon?.(cell.id)}
                        onresize={() => onresize?.(cell.id)}
            onresizeto={onresizeto ? (id, w, h) => onresizeto?.(id, w, h) : undefined}
            onresizeend={onresizeend ? (id) => onresizeend?.(id) : undefined}
            onsettings={() => onsettings?.(cell.id)}
          />
        </div>
      {:else}
        <div
          class="drop-wrap"
          class:dragging={isDragging}
          class:just-added={highlightId === cell.id && appearance.effects.iconAdd}
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
            onresizeto={onresizeto ? (id, w, h) => onresizeto?.(id, w, h) : undefined}
            onresizeend={onresizeend ? (id) => onresizeend?.(id) : undefined}
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


  </div>
</div>

<style>
  .grid {
    position: relative;
    height: 100%;
    overflow-y: auto; /* 正常布局不超高无滚动条；分辨率变小紧凑显示超高时可滚动查看（不自动分页） */
    overflow-x: auto; /* 极端窄窗口兜底：允许横向滚动查看 */
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
    transition: left 0.28s cubic-bezier(0.25, 0.1, 0.25, 1), top 0.28s cubic-bezier(0.25, 0.1, 0.25, 1);
    position: absolute;
    touch-action: pan-y;
  }
  .drop-wrap.dragging {
    transition: none;
  }
  /* 新添加图标特效：缩放弹出（highlightId 由 App 在添加时设置，1.6s 后清除；可在设置 → 特效 关闭） */
  .drop-wrap.just-added {
    animation: just-added-pop 0.5s cubic-bezier(0.34, 1.56, 0.64, 1) both;
  }
  @keyframes just-added-pop {
    0% {
      transform: scale(0.15);
      opacity: 0;
    }
    70% {
      transform: scale(1.08);
      opacity: 1;
    }
    100% {
      transform: scale(1);
      opacity: 1;
    }
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

</style>
