<script lang="ts">
  // HomeAssistant 小组件：显示配置的实体状态，开关/灯可一键控制，每 5s 刷新
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCellSetting, peekCellSetting } from "../core/pluginSettings.svelte";
  import { log } from "../core/logger";

  let { cellId }: { cellId?: string } = $props();

  const PLUGIN_ID = "dev.homedesktop.homeassistant";
  const instanceId = $derived(cellId ?? PLUGIN_ID);

  interface HaState {
    entityId: string;
    state: string;
    friendlyName?: string | null;
    unit?: string | null;
    domain: string;
  }

  let states = $state<HaState[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let working = $state(false);

  const url = $derived(peekCellSetting<string>(instanceId, PLUGIN_ID, "url") ?? "");
  const token = $derived(peekCellSetting<string>(instanceId, PLUGIN_ID, "token") ?? "");
  const entitiesRaw = $derived(peekCellSetting<string>(instanceId, PLUGIN_ID, "entities") ?? "");
  const entities = $derived(
    entitiesRaw.split(",").map((s) => s.trim()).filter(Boolean),
  );

  const configured = $derived(!!url && !!token && entities.length > 0);

  async function refresh(): Promise<void> {
    if (!configured) {
      loading = false;
      return;
    }
    try {
      const data = await invoke<HaState[]>("ha_states", {
        url,
        token,
        entities,
      });
      states = data;
      error = null;
    } catch (e) {
      error = String(e);
      log.error(`HA 状态获取失败: ${e}`);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    const init = async () => {
      await getCellSetting(instanceId, PLUGIN_ID, "url", "");
      await getCellSetting(instanceId, PLUGIN_ID, "token", "");
      await getCellSetting(instanceId, PLUGIN_ID, "entities", "");
      void refresh();
    };
    void init();
    const timer = setInterval(() => void refresh(), 5000);
    return () => clearInterval(timer);
  });

  /** 可开关的实体域 */
  function isToggleable(s: HaState): boolean {
    return ["light", "switch", "fan", "input_boolean"].includes(s.domain);
  }
  function stateLabel(s: HaState): string {
    if (s.state === "on") return "开";
    if (s.state === "off") return "关";
    return s.state;
  }
  function isOn(s: HaState): boolean {
    return s.state === "on";
  }

  /** 开关切换（light/switch/fan/input_boolean 均支持 toggle） */
  async function toggle(s: HaState): Promise<void> {
    if (working) return;
    working = true;
    try {
      await invoke("ha_call", {
        url,
        token,
        domain: s.domain,
        service: "toggle",
        entityId: s.entityId,
      });
      log.info(`HA 切换: ${s.entityId}`);
      setTimeout(refresh, 400);
    } catch (e) {
      log.error(`HA 切换失败: ${s.entityId} -> ${e}`);
      error = String(e);
    } finally {
      working = false;
    }
  }
</script>

<div class="ha">
  {#if !configured}
    <div class="empty">
      <div class="e-icon">🏠</div>
      <div>未配置 HomeAssistant</div>
      <div class="e-hint">点 ⋯ 设置地址 / 令牌 / 实体 ID</div>
    </div>
  {:else if loading}
    <div class="empty">连接中…</div>
  {:else if error}
    <div class="empty">
      <div class="e-icon">⚠️</div>
      <div class="e-hint">{error}</div>
    </div>
  {:else if states.length === 0}
    <div class="empty">
      <div class="e-icon">🔌</div>
      <div>未找到配置的实体</div>
      <div class="e-hint">检查实体 ID 是否正确</div>
    </div>
  {:else}
    <div class="list">
      {#each states as s (s.entityId)}
        <div class="row" class:on={isOn(s)}>
          <span class="name" title={s.entityId}>{s.friendlyName ?? s.entityId}</span>
          {#if isToggleable(s)}
            <button
              class="sw"
              class:on={isOn(s)}
              title={isOn(s) ? "关闭" : "开启"}
              onclick={() => void toggle(s)}
            >{isOn(s) ? "开" : "关"}</button>
          {:else}
            <span class="val">{stateLabel(s)}{s.unit ? ` ${s.unit}` : ""}</span>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .ha {
    height: 100%;
    overflow-y: auto;
    scrollbar-width: none;
    -ms-overflow-style: none;
  }
  .ha::-webkit-scrollbar {
    display: none;
  }
  .empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    font-size: 12px;
    color: var(--fg-dim);
    text-align: center;
    padding: 6px;
  }
  .e-icon {
    font-size: 24px;
  }
  .e-hint {
    font-size: 10px;
    opacity: 0.8;
    max-width: 90%;
    overflow-wrap: anywhere;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 2px;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 4px 8px;
    border-radius: 8px;
    background: var(--bg-hover);
    font-size: 12px;
  }
  .row.on {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--fg);
  }
  .val {
    font-size: 12px;
    color: var(--accent);
    font-weight: 600;
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
  }
  .sw {
    border: none;
    border-radius: 8px;
    padding: 3px 10px;
    font-size: 11px;
    cursor: pointer;
    flex-shrink: 0;
    background: var(--bg-elev);
    color: var(--fg-dim);
    transition: all 0.15s;
  }
  .sw:hover {
    border-color: var(--accent);
  }
  .sw.on {
    background: var(--accent);
    color: #fff;
    font-weight: 600;
  }
</style>
