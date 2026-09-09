<script lang="ts">
  // 网络监控小组件：后台 Rust net_speed 采样，展示各主网卡上行/下行速率。
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getPluginSetting } from "../core/pluginSettings.svelte";

  let { }: { cellId?: string } = $props();
  const pid = "dev.homedesktop.netmon";

  interface NetStat { iface: string; rx: number; tx: number; }
  let list = $state<NetStat[]>([]);
  let refreshMs = $state(1000);
  let err = $state("");

  async function sample(): Promise<void> {
    try {
      const arr = await invoke<NetStat[]>("net_speed");
      // 取有流量的网卡前几，若都 0 则全列
      const withActivity = arr.filter((n) => n.rx > 0 || n.tx > 0);
      list = (withActivity.length ? withActivity : arr).slice(0, 6);
      err = "";
    } catch (e) {
      err = String((e as Error)?.message ?? e);
    }
  }
  function fmt(bps: number): string {
    const v = bps; // bytes/s
    if (v >= 1048576) return (v / 1048576).toFixed(1) + " MB/s";
    if (v >= 1024) return (v / 1024).toFixed(0) + " KB/s";
    return (v).toFixed(0) + " B/s";
  }

  onMount(() => {
    void getPluginSetting<number>(pid, "refreshMs", refreshMs).then((v) => {
      refreshMs = v ?? 1000;
    });
    void sample();
    const t = setInterval(() => void sample(), refreshMs);
    return () => clearInterval(t);
  });
</script>

<div class="nm">
  <div class="head"><span class="title">📶 网络</span></div>
  {#if err}
    <div class="err">{err}</div>
  {:else if list.length === 0}
    <div class="empty">采样中…</div>
  {:else}
    <div class="list">
      {#each list as n (n.iface)}
        <div class="row">
          <span class="nm2">{n.iface}</span>
          <span class="m">▼ {fmt(n.rx)}</span>
          <span class="m up-f">▲ {fmt(n.tx)}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .nm { height: 100%; display: flex; flex-direction: column; gap: 6px; overflow: hidden; }
  .head .title { font-size: 13px; font-weight: 600; }
  .list { flex: 1; min-height: 0; overflow-y: auto; display: flex; flex-direction: column; gap: 5px; }
  .row { display: flex; align-items: center; gap: 8px; padding: 5px 7px; border: 1px solid var(--border); border-radius: 7px; background: var(--bg-input); }
  .nm2 { flex: 1; font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--fg); }
  .m { font-size: 12px; font-variant-numeric: tabular-nums; color: var(--accent); }
  .up-f { color: #e67e22; }
  .empty { font-size: 11px; color: var(--fg-dim); margin: auto; }
  .err { font-size: 10px; color: var(--danger); }
</style>
