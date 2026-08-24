<script lang="ts">
  import AppIcon from "./AppIcon.svelte";
  import FaviconIcon from "./FaviconIcon.svelte";
  import type { IconCell, PluginInfo } from "../core/types";
  import { appearance } from "../core/appearance.svelte";
  import { iconGlyphSize, iconEmojiFontSize, iconRadius } from "../core/iconStandard";

  let {
    item,
    plugin,
    editMode = false,
    onlaunch,
    ondelete,
    onmove,
    onedit,
    onresize,
    onsettings,
    onresizeto,
    onresizeend,
  }: {
    item: IconCell;
    plugin?: PluginInfo;
    /** 编辑模式：常显缩放/移入/自定义/删除操作 */
    editMode?: boolean;
    onlaunch?: () => void;
    ondelete?: (id: string) => void;
    onmove?: (id: string) => void;
    onedit?: (id: string) => void;
    onresize?: (id: string) => void;
    onsettings?: (id: string) => void;
    /** 拖动右下角手柄自由调整尺寸（0.5 格步进） */
    onresizeto?: (id: string, w: number, h: number) => void;
    /** 松开手柄时回调（用于一次性持久化，避免拖动过程狂写配置） */
    onresizeend?: (id: string) => void;
  } = $props();

  // ---------- 拖拽缩放手柄 ----------
  let resizing = false;
  let rsx = 0;
  let rsy = 0;
  let rsw = 0;
  let rsh = 0;
  let rSlot = 120;

  function startResize(e: PointerEvent): void {
    e.preventDefault();
    e.stopPropagation();
    resizing = true;
    rsx = e.clientX;
    rsy = e.clientY;
    rsw = item.size?.w ?? 1;
    rsh = item.size?.h ?? 1;
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
    const snap = (n: number) => Math.max(1, Math.min(8, Math.round(n * 2) / 2));
    const w = snap(rsw + (e.clientX - rsx) / rSlot);
    const h = snap(rsh + (e.clientY - rsy) / rSlot);
    onresizeto?.(item.id, w, h);
  }

  function endResize(e: PointerEvent): void {
    if (!resizing) return;
    resizing = false;
    try {
      (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      /* 忽略 */
    }
    onresizeend?.(item.id);
  }

  /** 图标尺寸随网格等比缩放（占格子约 50%，给标签留空间） */
  // 图标留出文字空间：icon+文字 完整居中放进格子，上下位置一致
  const iconSize = $derived(iconGlyphSize(appearance.tileSize));
  const iconR = $derived(iconRadius(appearance.tileSize));
  /** emoji/彩色图标的行内样式 */
  const iconStyle = $derived.by(() => {
    const fs = iconEmojiFontSize(appearance.tileSize);
    const parts = [`font-size:${fs}px`];
    if (item.color) {
      parts.push(`background:${item.color}`, `width:${iconSize}px`, `height:${iconSize}px`, `border-radius:${iconR}px`);
    }
    return parts.join(";");
  });

  /** 应用抽屉图标：无插件但有自带动作 → 显示系统真实图标（AppIcon 内回退头像）
   *  带自定义 emoji 的文件/文件夹图标（📄/📁）不在此列，走 emoji 显示 */
  const isAppIcon = $derived(!plugin && !!item.action && !item.emoji);
  /** 显示用的 emoji：自定义 > 插件 emoji > 📦 */
  const displayEmoji = $derived(item.emoji ?? plugin?.emoji ?? "📦");
  /** 网页快捷方式：插件含 url 设置项 → 显示网站 favicon */
  const urlDefault = $derived(plugin?.settings?.find((s) => s.key === "url")?.default);
  const isWebIcon = $derived(urlDefault !== undefined);
</script>

<div
  class="tile"
  class:editing={editMode}
  data-cell-id={item.id}
  role="button"
  tabindex="0"
  onclick={onlaunch}
  onkeydown={(e) => e.key === "Enter" && onlaunch?.()}
>
  {#if editMode}
    <div class="actions">
      {#if onresize}
        <button
          class="act resize"
          title="设置尺寸（整数倍）"
          onclick={(e) => {
            e.stopPropagation();
            onresize?.(item.id);
          }}
        >⇲</button>
      {/if}
      {#if onmove}
        <button
          class="act move"
          title="移入文件夹"
          onclick={(e) => {
            e.stopPropagation();
            onmove?.(item.id);
          }}
        >📁</button>
      {/if}
      {#if onedit}
        <button
          class="act edit"
          title="自定义（重命名/图标/颜色）"
          onclick={(e) => {
            e.stopPropagation();
            onedit?.(item.id);
          }}
        >📝</button>
      {/if}
      {#if onsettings && plugin?.settings?.length}
        <button
          class="act settings"
          title="插件设置"
          onclick={(e) => {
            e.stopPropagation();
            onsettings?.(item.id);
          }}
        >⚙</button>
      {/if}
      <button
        class="act del"
        title="删除"
        onclick={(e) => {
          e.stopPropagation();
          ondelete?.(item.id);
        }}
      >×</button>
    </div>
  {/if}
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
  {#if item.iconPath}
    <!-- 借用系统应用图标（M9） -->
    <AppIcon path={item.iconPath} name={item.title} size={iconSize} radius={iconR} />
  {:else if isAppIcon}
    <AppIcon
      path={item.action?.kind === "app" ? item.action.path : ""}
      name={item.title}
      size={iconSize}
      radius={iconR}
    />
  {:else if isWebIcon}
    <!-- 网页快捷方式：显示网站 favicon（兜底链见 FaviconIcon） -->
    <FaviconIcon
      cellId={item.id}
      pluginId={plugin?.id ?? ""}
      fallbackUrl={String(urlDefault ?? "")}
      fallbackEmoji={displayEmoji}
    />
  {:else}
    <div
      class="icon"
      class:colored={!!item.color}
      style={iconStyle}
    >{displayEmoji}</div>
  {/if}
  {#if item.showLabel !== false}
    <div class="label">{item.title}</div>
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
    gap: 12px;
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
  /* 编辑模式抖动（苹果风格） */
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
  .icon {
    line-height: 1;
  }
  .icon.colored {
    border-radius: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #fff;
  }
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
  .actions {
    position: absolute;
    top: 6px;
    right: 8px;
    display: flex;
    gap: 4px;
    z-index: 2;
  }
  .act {
    width: 22px;
    height: 22px;
    border: none;
    border-radius: 50%;
    font-size: 12px;
    line-height: 1;
    cursor: pointer;
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }
  .act.move {
    background: var(--accent);
  }
  .act.edit {
    background: var(--fg-dim);
  }
  .act.settings {
    background: var(--fg-dim);
  }
  .act.resize {
    background: var(--accent);
  }
  .act.del {
    background: var(--danger);
  }
</style>
