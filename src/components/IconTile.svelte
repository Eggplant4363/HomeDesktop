<script lang="ts">
  import AppIcon from "./AppIcon.svelte";
  import FaviconIcon from "./FaviconIcon.svelte";
  import type { IconCell, PluginInfo } from "../core/types";

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
  } = $props();

  /** 应用抽屉图标：无插件但有自带动作 → 显示系统真实图标（AppIcon 内回退头像） */
  const isAppIcon = $derived(!plugin && !!item.action);
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
  {#if item.iconPath}
    <!-- 借用系统应用图标（M9） -->
    <AppIcon path={item.iconPath} name={item.title} size={52} radius={14} />
  {:else if isAppIcon}
    <AppIcon
      path={item.action?.kind === "app" ? item.action.path : ""}
      name={item.title}
      size={52}
      radius={14}
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
      style={item.color ? `background: ${item.color};` : ""}
    >{displayEmoji}</div>
  {/if}
  <div class="label">{item.title}</div>
</div>

<style>
  .tile {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    background: var(--bg-elev);
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
    font-size: 34px;
    line-height: 1;
  }
  .icon.colored {
    width: 52px;
    height: 52px;
    border-radius: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #fff;
  }
  .label {
    font-size: 12px;
    color: var(--fg-dim);
    max-width: 90%;
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
  .act {
    width: 22px;
    height: 22px;
    border: none;
    border-radius: 50%;
    font-size: 12px;
    line-height: 1;
    cursor: pointer;
    color: #fff;
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
