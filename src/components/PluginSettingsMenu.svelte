<script lang="ts">
  // 插件统一设置菜单（安卓风格：编辑模式点 ⚙ → 配置）
  // 设置按"图标实例"独立保存（cell.<cellId>.<key>）；未设置时回退插件级默认
  // 输入即保存（oninput），保存后立即触发该实例的小组件刷新（如天气换城市马上生效）
  import { onMount } from "svelte";
  import type { PluginInfo } from "../core/types";
  import { getCellSetting, setCellSetting } from "../core/pluginSettings.svelte";
  import { refreshWidget, widgetCache } from "../core/widgetRuntime.svelte";
  import { log } from "../core/logger";

  let {
    plugin,
    cellId,
    onclose,
  }: {
    plugin: PluginInfo;
    /** 当前图标的实例 id（每个图标独立设置） */
    cellId?: string;
    onclose?: () => void;
  } = $props();

  let values = $state<Record<string, string | number | boolean>>({});
  let refreshTimer: ReturnType<typeof setTimeout> | undefined;

  onMount(async () => {
    for (const s of plugin.settings ?? []) {
      const v = await getCellSetting(cellId ?? plugin.id, plugin.id, s.key, s.default);
      if (v !== undefined) values[s.key] = v;
    }
  });

  /** 保存设置 + 防抖刷新该实例（连续输入只刷一次；图标插件无模块时刷新是 no-op） */
  async function save(key: string, value: string | number | boolean): Promise<void> {
    values[key] = value;
    await setCellSetting(cellId ?? plugin.id, plugin.id, key, value);
    const instanceId = cellId ?? plugin.id;
    clearTimeout(refreshTimer);
    refreshTimer = setTimeout(() => {
      // 先清掉该实例缓存避免短暂显示旧数据，再触发后台刷新
      delete widgetCache[instanceId];
      void refreshWidget(instanceId).then((ok) => {
        log.info(`设置保存后刷新${ok ? "完成" : "未执行(无数据模块或失败)"}: ${instanceId} ${key}=${value}`);
      });
    }, 500);
  }
</script>

<div
  class="overlay"
  role="button"
  aria-label="关闭"
  tabindex="-1"
  onclick={(e) => e.target === e.currentTarget && onclose?.()}
  onkeydown={(e) => e.key === "Escape" && onclose?.()}
>
  <div class="menu">
    <div class="head">
      <span>⚙ {plugin.name} 设置</span>
      <button class="close" onclick={onclose}>×</button>
    </div>
    <div class="body">
      {#each plugin.settings ?? [] as s (s.key)}
        <div class="row">
          <span class="label">{s.label}</span>
          {#if s.type === "toggle"}
            <button
              class="toggle"
              class:on={!!values[s.key]}
              onclick={() => save(s.key, !values[s.key])}
            >{values[s.key] ? "开" : "关"}</button>
          {:else if s.type === "select"}
            <select
              value={String(values[s.key] ?? "")}
              onchange={(e) => save(s.key, (e.currentTarget as HTMLSelectElement).value)}
            >
              {#each s.options ?? [] as opt (opt.label)}
                <option value={String(opt.value)}>{opt.label}</option>
              {/each}
            </select>
          {:else if s.type === "number"}
            <input
              type="number"
              value={String(values[s.key] ?? "")}
              oninput={(e) => save(s.key, Number((e.currentTarget as HTMLInputElement).value))}
            />
          {:else}
            <input
              type="text"
              value={String(values[s.key] ?? "")}
              oninput={(e) => save(s.key, (e.currentTarget as HTMLInputElement).value)}
            />
          {/if}
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 45;
  }
  .menu {
    width: 320px;
    background: var(--bg-elev);
    border-radius: 16px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 16px;
    font-weight: 600;
    font-size: 14px;
  }
  .close {
    border: none;
    background: transparent;
    color: var(--fg-dim);
    font-size: 18px;
    cursor: pointer;
  }
  .body {
    padding: 4px 16px 14px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .label {
    font-size: 13px;
    color: var(--fg);
  }
  input[type="text"],
  input[type="number"],
  select {
    flex: 1;
    max-width: 170px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-input);
    color: var(--fg);
    padding: 7px 10px;
    font-size: 13px;
    outline: none;
  }
  input:focus,
  select:focus {
    border-color: var(--accent);
  }
  .toggle {
    width: 64px;
    border: 1px solid var(--border);
    border-radius: 16px;
    background: var(--bg-input);
    color: var(--fg-dim);
    font-size: 12px;
    padding: 5px 0;
    cursor: pointer;
  }
  .toggle.on {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
</style>
