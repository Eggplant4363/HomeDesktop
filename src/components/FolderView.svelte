<script lang="ts">
  import IconTile from "./IconTile.svelte";
  import WidgetTile from "./WidgetTile.svelte";
  import {
    enterEditMode,
    findFolder,
    openFolder,
    plugins,
    query,
    ui,
  } from "../core/stores.svelte";
  import { filterIcons } from "../core/search";
  import type { IconCell, PluginInfo } from "../core/types";

  let {
    onaddclick,
    onlaunch,
    onmove,
    onediticon,
    ondelete,
    onreorder,
    onresize,
    onsettings,
  }: {
    onaddclick?: () => void;
    onlaunch?: (pluginId: string) => void;
    onmove?: (iconId: string) => void;
    onediticon?: (iconId: string) => void;
    /** 删除文件夹内图标（由外层统一弹确认框） */
    ondelete?: (folderId: string, iconId: string) => void;
    /** 文件夹内拖拽排序：targetId 为 null 表示追加到末尾 */
    onreorder?: (dragId: string, targetId: string | null, pos: "before" | "after") => void;
    onresize?: (iconId: string) => void;
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

  const visible = $derived(
    folder ? filterIcons(folder.items, plugins, query.text) : [],
  );

  // ---------- 文件夹内拖拽排序（M8，Pointer Events；简化版：无翻页/无跨文件夹） ----------

  let gridEl = $state<HTMLElement | undefined>();
  let dragging = false;
  let draggingId = $state<string | null>(null);
  let overId = $state<string | null>(null);
  let overPos = $state<"before" | "after">("before");
  let suppressClick = false;
  let pointerId: number | null = null;
  let dragCandidate: string | null = null;
  let startX = 0;
  let startY = 0;
  let longPressTimer: ReturnType<typeof setTimeout> | undefined;

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
    if (overId && overId !== draggingId) {
      onreorder?.(draggingId!, overId, overPos);
    } else {
      // 空白处释放 → 追加到末尾
      onreorder?.(draggingId!, null, "after");
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
  }

  function updateOver(x: number, y: number): void {
    const el = document.elementFromPoint(x, y);
    const tile = (el as HTMLElement | null)?.closest?.<HTMLElement>("[data-cell-id]");
    const targetId = tile?.dataset.cellId ?? null;
    if (targetId && targetId !== draggingId) {
      const rect = tile!.getBoundingClientRect();
      overId = targetId;
      overPos = x < rect.left + rect.width / 2 ? "before" : "after";
    } else {
      overId = null;
    }
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
    onclick={onClickCapture}
    onkeydown={(e) => {
      if (e.key === "Escape") endDrag();
    }}
  >
    {#each visible as icon (icon.id)}
      {@const isOver = overId === icon.id}
      {@const isDragging = draggingId === icon.id}
      <div
        class="cell-wrap"
        class:over={isOver}
        class:before={isOver && overPos === "before"}
        class:after={isOver && overPos === "after"}
        class:dragging={isDragging}
        style="grid-column: span {icon.size.w}; grid-row: span {icon.size.h};"
      >
        {#if isWidget(icon)}
          <WidgetTile
            item={icon}
            editMode={ui.editMode}
            ondelete={() => ondelete?.(folder.id, icon.id)}
            onmove={() => onmove?.(icon.id)}
            onresize={() => onresize?.(icon.id)}
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
            onsettings={() => onsettings?.(icon.id)}
          />
        {/if}
      </div>
    {/each}

    {#if query.text === ""}
      <button class="add-tile" onclick={onaddclick}>＋</button>
    {/if}
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
    display: grid;
    /* 固定基准粒度（同主网格）：尺寸严格为 1×1 的整数倍 */
    grid-template-columns: repeat(auto-fill, var(--tile-size));
    grid-auto-rows: var(--tile-size);
    justify-content: center;
    gap: var(--gap);
    align-content: start;
    height: calc(100% - 56px);
    overflow-y: auto;
    padding: 4px;
    touch-action: pan-y; /* 触屏：纵向滚动交给浏览器，长按/横向由应用处理 */
  }
  .cell-wrap {
    position: relative;
    touch-action: pan-y;
  }
  .cell-wrap.dragging {
    opacity: 0.4;
  }
  .cell-wrap.before::before,
  .cell-wrap.after::after {
    content: "";
    position: absolute;
    top: 8%;
    bottom: 8%;
    width: 4px;
    border-radius: 2px;
    background: var(--accent);
    z-index: 3;
  }
  .cell-wrap.before::before {
    left: -8px;
  }
  .cell-wrap.after::after {
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
  }
  .add-tile:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
</style>
