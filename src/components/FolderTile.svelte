<script lang="ts">
  import type { FolderCell } from "../core/types";
  import { appearance } from "../core/appearance.svelte";
  import { iconGlyphSize, ICON_TEXT_GAP } from "../core/iconStandard";

  let {
    folder,
    editMode = false,
    onopen,
    onedit,
    ondelete,
    onresize,
    onresizeto,
    onresizeend,
  }: {
    folder: FolderCell;
    /** 编辑模式：常显编辑/删除按钮，且不可打开 */
    editMode?: boolean;
    onopen?: () => void;
    onedit?: (id: string) => void;
    ondelete?: (id: string) => void;
    /** 设置尺寸（整数倍弹窗） */
    onresize?: (id: string) => void;
    /** 拖拽缩放实时 */
    onresizeto?: (id: string, w: number, h: number) => void;
    /** 拖拽缩放结束 */
    onresizeend?: (id: string) => void;
  } = $props();

  const w = $derived(folder.size?.w ?? 1);
  const h = $derived(folder.size?.h ?? 1);
  /** 图标与应用图标同尺寸（iconStandard 统一标准） */
  const iconSize = $derived(iconGlyphSize(appearance.tileSize));

  // ---------- 拖拽缩放（整格吸附，1..8） ----------
  let resizing = false;
  let rsx = 0, rsy = 0, rsw = 0, rsh = 0, rSlot = 0;
  function startResize(e: PointerEvent): void {
    if (!onresizeto) return;
    e.preventDefault();
    e.stopPropagation();
    resizing = true;
    rsx = e.clientX;
    rsy = e.clientY;
    rsw = w;
    rsh = h;
    const el = e.currentTarget as HTMLElement;
    const style = getComputedStyle(el);
    const t = parseFloat(style.getPropertyValue("--tile-size"));
    const g = parseFloat(style.getPropertyValue("--gap"));
    if (Number.isFinite(t) && t > 0) rSlot = t + (Number.isFinite(g) && g >= 0 ? g : 0);
    try {
      el.setPointerCapture(e.pointerId);
    } catch {
      /* 忽略 */
    }
  }
  function onResizeMove(e: PointerEvent): void {
    if (!resizing) return;
    e.preventDefault();
    const snap = (n: number) => Math.max(1, Math.min(8, Math.round(n)));
    const nw = snap(rsw + (e.clientX - rsx) / rSlot);
    const nh = snap(rsh + (e.clientY - rsy) / rSlot);
    onresizeto?.(folder.id, nw, nh);
  }
  function endResize(e: PointerEvent): void {
    if (!resizing) return;
    resizing = false;
    try {
      (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      /* 忽略 */
    }
    onresizeend?.(folder.id);
  }
</script>

<div
  class="tile"
  class:editing={editMode}
  data-cell-id={folder.id}
  role="button"
  tabindex="0"
  style="gap: {ICON_TEXT_GAP}px;"
  onclick={onopen}
  onkeydown={(e) => e.key === "Enter" && onopen?.()}
>
  {#if editMode}
    <div class="actions">
      {#if onresize}
        <button
          class="act"
          title="设置尺寸"
          onclick={(e) => {
            e.stopPropagation();
            onresize?.(folder.id);
          }}
        >⇲</button>
      {/if}
      {#if onedit}
        <button
          class="edit"
          title="重命名 / 换图标"
          onclick={(e) => {
            e.stopPropagation();
            onedit?.(folder.id);
          }}
        >📝</button>
      {/if}
      <button
        class="del"
        title="删除文件夹"
        onclick={(e) => {
          e.stopPropagation();
          ondelete?.(folder.id);
        }}
      >×</button>
    </div>
  {/if}

  <div class="icon" style="font-size:{iconSize}px;">{folder.emoji}</div>
  <div class="label">{folder.name}</div>

  {#if editMode && onresizeto}
    <div
      class="resize-handle"
      role="button"
      tabindex="-1"
      title="拖动调整大小"
      onpointerdown={startResize}
      onpointermove={onResizeMove}
      onpointerup={endResize}
      onpointercancel={endResize}
    >⤡</div>
  {/if}
</div>

<style>
  .tile {
    position: relative;
    height: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px; /* ICON_TEXT_GAP */
    padding: 6px;
    background: var(--bg-elev);
    border: 1px solid color-mix(in srgb, var(--border) 70%, transparent);
    border-radius: var(--radius);
    cursor: pointer;
    transition: transform 0.12s, background 0.15s;
  }
  .tile:hover {
    background: var(--bg-hover);
  }
  .tile:focus-visible {
    outline: 2px solid var(--accent);
  }
  .tile.editing {
    animation: wiggle 0.28s ease-in-out infinite;
  }
  @keyframes wiggle {
    0%,
    100% {
      transform: rotate(-1.3deg);
    }
    50% {
      transform: rotate(1.3deg);
    }
  }
  /* 文件夹图标：与应用图标同尺寸（iconGlyphSize 字号），无底色 */
  .icon {
    line-height: 1;
  }
  /* 与 IconTile 标签完全一致：字号/字重/颜色/对齐，保证字体高度对齐 */
  .label {
    font-size: 13px;
    font-weight: 500;
    color: var(--fg);
    text-align: center;
    max-width: 92%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .actions {
    position: absolute;
    top: 6px;
    right: 8px;
    display: flex;
    gap: 4px;
    z-index: 2;
  }
  .del,
  .edit,
  .act {
    width: 22px;
    height: 22px;
    border: none;
    border-radius: 50%;
    color: #fff;
    font-size: 12px;
    line-height: 1;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }
  .del {
    background: var(--danger);
  }
  .edit {
    background: var(--accent);
  }
  .act {
    background: var(--fg-dim);
  }
  .resize-handle {
    position: absolute;
    right: 0;
    bottom: 0;
    width: 18px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    color: var(--fg-dim);
    cursor: nwse-resize;
    user-select: none;
    touch-action: none;
    z-index: 3;
  }
  .resize-handle:hover {
    color: var(--accent);
  }
</style>