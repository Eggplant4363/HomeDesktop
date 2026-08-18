<script lang="ts">
  // 系统监控小组件（M12）：CPU / 内存使用率，widgetRuntime 每 3 秒后台刷新
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import {
    isWidgetStale,
    refreshWidget,
    registerWidget,
    widgetCache,
  } from "../core/widgetRuntime.svelte";

  let { cellId }: { cellId?: string } = $props();

  const instanceId = $derived(cellId ?? "sysmonitor");
  const REFRESH_MS = 3_000;

  interface SysStats {
    cpu: number | null;
    mem: number;
  }

  async function fetchStats(): Promise<SysStats> {
    const s = await invoke<SysStats | null>("sys_stats");
    if (!s) throw new Error("系统监控不可用");
    return s;
  }

  onMount(async () => {
    registerWidget<SysStats>({ id: instanceId, refreshMs: REFRESH_MS, fetch: fetchStats });
    if (isWidgetStale(instanceId, REFRESH_MS)) void refreshWidget(instanceId);
  });

  const data = $derived(widgetCache[instanceId]?.data as SysStats | undefined);
  const cpuPct = $derived(Math.min(100, Math.max(0, Math.round(data?.cpu ?? 0))));
  const memPct = $derived(Math.min(100, Math.max(0, Math.round(data?.mem ?? 0))));
  const cpuText = $derived(data?.cpu != null ? `${cpuPct}%` : "…");
  const memText = $derived(data ? `${memPct}%` : "…");
</script>

<div class="sys">
  <div class="line">
    <span class="label">🧠 CPU</span>
    <span class="val">{cpuText}</span>
  </div>
  <div class="bar"><div class="fill" style="width: {cpuPct}%;"></div></div>
  <div class="line">
    <span class="label">💾 内存</span>
    <span class="val">{memText}</span>
  </div>
  <div class="bar"><div class="fill mem" style="width: {memPct}%;"></div></div>
</div>

<style>
  .sys {
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 5px;
  }
  .line {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    font-size: 12px;
  }
  .label {
    color: var(--fg-dim);
  }
  .val {
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .bar {
    height: 5px;
    background: var(--bg-hover);
    border-radius: 3px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--accent);
    border-radius: 3px;
    transition: width 0.6s ease;
  }
  .fill.mem {
    background: var(--fg-dim);
  }
</style>
