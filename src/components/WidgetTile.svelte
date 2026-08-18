<script lang="ts">
  import type { IconCell } from "../core/types";
  import { plugins } from "../core/stores.svelte";
  import { getWidgetDef } from "../widgets";
  import PluginWidgetHost from "./PluginWidgetHost.svelte";

  let {
    item,
    editMode = false,
    ondelete,
    onmove,
    onresize,
    onsettings,
  }: {
    item: IconCell;
    /** 编辑模式：常显缩放/移入/删除操作 */
    editMode?: boolean;
    ondelete?: (id: string) => void;
    onmove?: (id: string) => void;
    onresize?: (id: string) => void;
    onsettings?: (id: string) => void;
  } = $props();

  const plugin = $derived(plugins.find((p) => p.id === item.pluginId));
  const def = $derived(getWidgetDef(plugin?.widgetComponent));
</script>

<div
  class="widget-tile"
  class:editing={editMode}
  data-cell-id={item.id}
  role="button"
  tabindex="0"
  onclick={(e) => {
    // 正常模式点击小组件 = 打开设置（如倒计时设置时间）；
    // 点击落在组件内部交互元素（按钮/输入框等）时不触发
    if (editMode) return;
    if ((e.target as HTMLElement).closest("button, input, select, textarea, a")) return;
    onsettings?.(item.id);
  }}
  onkeydown={(e) => {
    if (editMode) return;
    if (
      e.key === "Enter" &&
      !(e.target as HTMLElement).closest("button, input, select, textarea, a")
    ) {
      onsettings?.(item.id);
    }
  }}
>
  {#if editMode && (ondelete || onresize || onsettings || onmove)}
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
      {#if ondelete}
        <button
          class="act del"
          title="删除"
          onclick={(e) => {
            e.stopPropagation();
            ondelete?.(item.id);
          }}
        >×</button>
      {/if}
    </div>
  {/if}
  {#if def}
    {@const C = def.component}
    <!-- cellId：小组件实例 id，用于按实例读取独立设置 -->
    <C cellId={item.id} />
  {:else if plugin?.widgetFile}
    <!-- 插件自带小组件（M16）：sandbox iframe 加载插件 JS -->
    <PluginWidgetHost
      plugin={plugin}
      cellId={item.id}
      onopensettings={() => onsettings?.(item.id)}
    />
  {:else}
    <div class="unknown">
      <div>{plugin?.emoji ?? "🧩"}</div>
      <div class="unknown-name">{plugin?.name ?? "未知小组件"}</div>
    </div>
  {/if}
</div>

<style>
  .widget-tile {
    position: relative;
    height: 100%;
    min-height: var(--tile-size);
    background: var(--bg-elev);
    border-radius: var(--radius);
    overflow: hidden;
    padding: 10px 12px;
  }
  .widget-tile.editing {
    animation: wiggle 0.28s ease-in-out infinite;
  }
  @keyframes wiggle {
    0%,
    100% {
      transform: rotate(-0.8deg);
    }
    50% {
      transform: rotate(0.8deg);
    }
  }
  .actions {
    position: absolute;
    top: 8px;
    right: 10px;
    display: flex;
    gap: 4px;
    z-index: 6;
  }
  .act {
    width: 24px;
    height: 24px;
    border: none;
    border-radius: 50%;
    font-size: 13px;
    line-height: 1;
    cursor: pointer;
    color: #fff;
  }
  .act.resize {
    background: var(--accent);
  }
  .act.move {
    background: var(--accent);
  }
  .act.settings {
    background: var(--fg-dim);
  }
  .act.del {
    background: var(--danger);
  }
  .unknown {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    color: var(--fg-dim);
    font-size: 28px;
  }
  .unknown-name {
    font-size: 12px;
  }
</style>
