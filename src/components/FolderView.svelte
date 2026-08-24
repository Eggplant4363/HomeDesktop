<script lang="ts">
  // 文件夹内图标（v3 自由摆放）：FOLDER_COLS 列虚拟画布，拖拽吸附网格、不自动重排
  import IconTile from "./IconTile.svelte";
  import WidgetTile from "./WidgetTile.svelte";
  import {
    enterEditMode,
    findFolder,
    openFolder,
    plugins,
    ui,
  } from "../core/stores.svelte";
  import { appearance } from "../core/appearance.svelte";
  import type { IconCell, PluginInfo } from "../core/types";
  import { FOLDER_COLS } from "../core/layout";

  let {
    onaddclick,
    onlaunch,
    onmove,
    onediticon,
    ondelete,
    ondropat,
    onresize,
    onresizeto,
    onresizeend,
    onsettings,
  }: {
    onaddclick?: () => void;
    onlaunch?: (pluginId: string) => void;
    onmove?: (iconId: string) => void;
    onediticon?: (iconId: string) => void;
    /** 删除文件夹内图标（由外层统一弹确认框） */
    ondelete?: (folderId: string, iconId: string) => void;
    /** 自由摆放落点：文件夹内图标放到 (x, y) 网格坐标 */
    ondropat?: (folderId: string, iconId: string, x: number, y: number) => void;
    onresize?: (iconId: string) => void;
    onresizeto?: (iconId: string, w: number, h: number) => void;
    onresizeend?: (iconId: string) => void;
    onsettings?: (cellId: string) => void;
  } = $props();

  const folder = $derived(
    openFolder.folderId ? findFolder(openFolder.folderId)?.folder : undefined,
  );

  function pluginOf(icon: IconCell): PluginInfo | undefined {
    return plugins.find((p) => p.id === icon.pluginId);
  }

  function isWidget(icon: IconCell): boolean {
    return pluginOf(icon)?.pluginType === "widget";
  }

  // 完整文件夹：始终显示全部图标（不受全局搜索过滤，避免"过滤功能"错觉与移动冲突）
  const visible = $derived(folder ? folder.items : []);

  // ---------- 文件夹内拖拽（自由摆放：吸附网格、不自动重排） ----------

  let gridEl = $state<HTMLElement | undefined>();
  let canvasEl = $state<HTMLElement | undefined>();
  let tile = $state(84);
  let gap = $state(16);
  const PAD = 6;
  const SLOT = $derived(tile + gap);

  let dragging = false;
  let draggingId = $state<string | null>(null);
  let dragSlot = $state<{ x: number; y: number; w: number; h: number } | null>(null);
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

  function pxX(icon: IconCell): number {
    return PAD + (packed.get(icon.id)?.x ?? icon.x ?? 0) * SLOT;
  }
  function pxY(icon: IconCell): number {
    return PAD + (packed.get(icon.id)?.y ?? icon.y ?? 0) * SLOT;
  }
  function pxW(icon: IconCell): number {
    return icon.size.w * tile + (icon.size.w - 1) * gap;
  }
  function pxH(icon: IconCell): number {
    return icon.size.h * tile + (icon.size.h - 1) * gap;
  }

  /** 打开时图标从左上角紧凑排列（显示占位，行优先，无空档） */
  const packed = $derived.by(() => {
    const map = new Map<string, { x: number; y: number }>();
    if (!folder) return map;
    let cx = 0, cy = 0, rowH = 0;
    for (const icon of folder.items) {
      const w = icon.size.w;
      const h = icon.size.h;
      if (cx + w > FOLDER_COLS) {
        cx = 0;
        cy += rowH;
        rowH = 0;
      }
      map.set(icon.id, { x: cx, y: cy });
      cx += w;
      rowH = Math.max(rowH, h);
    }
    return map;
  });
  /** 画布宽度贴合打包内容（图标从视图左上角开始，不居中偏移） */
  const packedW = $derived.by(() => {
    let maxX = 0;
    for (const icon of folder?.items ?? []) {
      const p = packed.get(icon.id);
      if (p) maxX = Math.max(maxX, p.x + icon.size.w);
    }
    return maxX;
  });
  const canvasW = $derived(packedW * tile + (packedW - 1) * gap + PAD * 2);
  const canvasH = $derived.by(() => {
    let max = 0;
    for (const icon of folder?.items ?? []) {
      if (icon.id === draggingId) continue;
      max = Math.max(max, ((icon.y ?? 0) + icon.size.h) * SLOT);
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
  }

  // 首次挂载与图标大小变化时重新度量
  $effect(() => {
    void appearance.tileSize;
    void appearance.gridSpacing;
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
      updateTarget(e.clientX, e.clientY);
      // 幽灵图标吸附到落点
      if (dragSlot && draggingId) {
        const ic = folder?.items.find((i) => i.id === draggingId);
        dragDx = (dragSlot.x - (ic?.x ?? 0)) * SLOT;
        dragDy = (dragSlot.y - (ic?.y ?? 0)) * SLOT;
      }
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
    if (dragSlot && dragSlot.x >= 0 && folder) {
      ondropat?.(folder.id, draggingId!, dragSlot.x, dragSlot.y);
    }
    suppressClick = true;
    endDrag();
  }

  function onPointerCancel(): void {
    clearLongPress();
    if (dragging) endDrag();
  }

  /** 点击处理：拖拽后的点击忽略；点空白（非图标/非按钮）→ 返回上一层 */
  function onGridClick(e: MouseEvent): void {
    if (suppressClick) {
      e.preventDefault();
      e.stopPropagation();
      suppressClick = false;
      return;
    }
    if (ui.editMode) return; // 编辑模式点空白不关闭（避免误触）
    const t = e.target as HTMLElement;
    if (t.closest("[data-cell-id]") || t.closest("button")) return;
    openFolder.folderId = null;
  }

  function beginDrag(id: string): void {
    if (dragging) return;
    dragging = true;
    draggingId = id;
    const icon = folder?.items.find((i) => i.id === id);
    dragSlot = {
      x: icon?.x ?? 0,
      y: icon?.y ?? 0,
      w: icon?.size.w ?? 1,
      h: icon?.size.h ?? 1,
    };
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
  }

  function updateTarget(x: number, y: number): void {
    if (!canvasEl || !draggingId) return;
    const icon = folder?.items.find((i) => i.id === draggingId);
    const w = icon?.size.w ?? 1;
    const h = icon?.size.h ?? 1;
    const slot = SLOT;
    // 增量式：落点 = 原始格位 + 拖动格数（抓取位置不影响，边缘也能精确放第一格）
    const dx = x - dragStartPointerX;
    const dy = y - dragStartPointerY;
    const tx = (icon?.x ?? 0) + Math.round(dx / slot);
    const ty = (icon?.y ?? 0) + Math.round(dy / slot);
    dragSlot = {
      x: Math.max(0, Math.min(FOLDER_COLS - w, tx)),
      y: Math.max(0, ty),
      w,
      h,
    };
  }

  function clearLongPress(): void {
    if (longPressTimer) {
      clearTimeout(longPressTimer);
      longPressTimer = undefined;
    }
  }
</script>

{#if folder}
  <header class="folder-head">
    <button class="icon-btn" title="返回" onclick={() => (openFolder.folderId = null)}>←</button>
    <span class="folder-name">{folder.emoji} {folder.name}</span>
    <span class="count">{folder.items.length} 项</span>
  </header>

  <div
    class="grid"
    role="grid"
    tabindex="-1"
    bind:this={gridEl}
    onpointerdown={onPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
    onpointercancel={onPointerCancel}
    onclick={onGridClick}
    onkeydown={(e) => {
      if (e.key === "Escape") endDrag();
    }}
  >
    <div
      class="canvas"
      bind:this={canvasEl}
      style="width:{canvasW}px;height:{canvasH}px;"
    >
      {#each visible as icon (icon.id)}
        {@const isDragging = draggingId === icon.id}
        <div
          class="cell-wrap"
          class:dragging={isDragging}
          data-cell-id={icon.id}
          style="left:{pxX(icon)}px;top:{pxY(icon)}px;width:{pxW(icon)}px;height:{pxH(icon)}px;{isDragging ? `transform: translate(${dragDx}px,${dragDy}px);z-index:20;opacity:.8;` : ""}"
        >
          {#if isWidget(icon)}
            <WidgetTile
              item={icon}
              editMode={ui.editMode}
              ondelete={() => ondelete?.(folder.id, icon.id)}
              onmove={() => onmove?.(icon.id)}
              onresize={() => onresize?.(icon.id)}
            onresizeto={onresizeto ? (id, w, h) => onresizeto?.(id, w, h) : undefined}
            onresizeend={onresizeend ? (id) => onresizeend?.(id) : undefined}
              onsettings={() => onsettings?.(icon.id)}
            />
          {:else}
            <IconTile
              item={icon}
              plugin={pluginOf(icon)}
              editMode={ui.editMode}
              onlaunch={() => !ui.editMode && onlaunch?.(icon.id)}
              ondelete={() => ondelete?.(folder.id, icon.id)}
              onmove={() => onmove?.(icon.id)}
              onedit={() => onediticon?.(icon.id)}
              onresize={() => onresize?.(icon.id)}
            onresizeto={onresizeto ? (id, w, h) => onresizeto?.(id, w, h) : undefined}
            onresizeend={onresizeend ? (id) => onresizeend?.(id) : undefined}
              onsettings={() => onsettings?.(icon.id)}
            />
          {/if}
        </div>
      {/each}

      {#if dragSlot}
        <div
          class="slot-ghost"
          style="left:{PAD + dragSlot.x * SLOT}px;top:{PAD + dragSlot.y * SLOT}px;width:{dragSlot.w * tile + (dragSlot.w - 1) * gap}px;height:{dragSlot.h * tile + (dragSlot.h - 1) * gap}px;"
        ></div>
      {/if}


    </div>
  </div>
{/if}

<style>
  .folder-head {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
  }
  .icon-btn {
    width: 36px;
    height: 36px;
    border: none;
    border-radius: 10px;
    background: var(--bg-elev);
    color: var(--fg);
    font-size: 18px;
    cursor: pointer;
  }
  .icon-btn:hover {
    background: var(--bg-hover);
  }
  .folder-name {
    font-size: 15px;
    font-weight: 600;
  }
  .count {
    font-size: 12px;
    color: var(--fg-dim);
  }
  .grid {
    position: relative;
    height: calc(100% - 56px);
    overflow-y: auto;
    overflow-x: hidden;
    touch-action: pan-y;
    /* 容器 tabindex=-1 可被点击聚焦；按键后 Chromium 会画默认黑色 focus ring → 禁用 */
    outline: none;
  }
  .grid:focus,
  .grid:focus-visible {
    outline: none;
  }
  .canvas {
    position: relative;
  }
  .cell-wrap {
    position: absolute;
    touch-action: pan-y;
  }
  .cell-wrap.dragging {
    transition: none;
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
