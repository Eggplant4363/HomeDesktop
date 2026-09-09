<script lang="ts">
  // 行情速览：汇率(frankfurter，免Key&支持CORS) + 加密(CoinGecko) 小卡片。
  import { onMount } from "svelte";
  import { getPluginSetting } from "../core/pluginSettings.svelte";

  let { }: { cellId?: string } = $props();
  const pid = "dev.homedesktop.marketquotes";
  const FX_BASE = ["USD", "CNY", "EUR", "JPY", "HKD", "CHF"];

  interface Coin { id: string; icon: string; name: string; }
  const COINS: Coin[] = [
    { id: "bitcoin", icon: "₿", name: "BTC" },
    { id: "ethereum", icon: "Ξ", name: "ETH" },
    { id: "solana", icon: "◎", name: "SOL" },
    { id: "binancecoin", icon: "▪", name: "BNB" },
  ];

  // 选中作为折算币种（简化为人民币 CNY / 美元 USD）
  let fxRates: Record<string, number> = {}; // fxFrom=? use ratesUSD-based: store "CNY"
  let coinsPrice: Record<string, { usd?: number; cny?: number; pct?: number }> = {};
  let refreshSec = $state(60);
  let err = $state("");
  let loading = $state(true);
  let showUSD = $state(false); // 折USD(默认CNY)

  async function load(): Promise<void> {
    refreshSec = (await getPluginSetting<number>(pid, "refreshSec", refreshSec)) ?? 60;
  }

  async function refreshFX(): Promise<void> {
    try {
      // frankfurter: rates relative to target
      const r = await fetch("https://api.frankfurter.app/latest?from=" + (showUSD ? "USD" : "CNY") + "&to=" + FX_BASE.join(","));
      const j = await r.json();
      fxRates = j?.rates ?? {};
    } catch {
      /* 忽略 */
    }
  }

  async function refreshCoins(): Promise<void> {
    try {
      const r = await fetch(
        "https://api.coingecko.com/api/v3/simple/price?ids=" + COINS.map((c) => c.id).join(",") +
        "&vs_currencies=usd%2Ccny&include_24hr_change=true"
      );
      const j = await r.json();
      const map: typeof coinsPrice = {};
      for (const c of COINS) {
        const d = j?.[c.id];
        map[c.id] = { usd: d?.usd, cny: d?.cny, pct: d?.usd_24h_change };
      }
      coinsPrice = map;
    } catch (e) {
      err = String((e as Error)?.message ?? e);
    }
  }

  let timer: ReturnType<typeof setInterval> | undefined;
  async function refreshAll(): Promise<void> {
    await Promise.all([refreshFX(), refreshCoins()]);
    loading = false;
    err = "";
  }

  onMount(() => {
    void load().then(() => void refreshAll());
    timer = setInterval(() => void refreshAll(), Math.max(15, refreshSec) * 1000);
    return () => clearInterval(timer);
  });
</script>

<div class="mq">
  <div class="head">
    <span class="title">📈 行情速览</span>
    <span class="unit" role="button" onclick={() => { showUSD = !showUSD; void refreshFX(); }}>{showUSD ? "USD" : "CNY"}</span>
  </div>

  {#if loading}
    <div class="empty">加载中…</div>
  {:else}
    <div class="grid">
      <div class="blk fx-hd">汇率（折{showUSD ? "美元" : "人民币"} 1 ← 常用）</div>
      {#each Object.entries(fxRates) as [sym, v] (sym)}
        <div class="card">
          <span class="ci">{sym}</span>
          <span class="val">{(1 / v).toFixed(showUSD ? 4 : 2)} {showUSD ? "USD" : "CNY"}</span>
        </div>
      {/each}

      <div class="blk coin-hd">加密货币</div>
      {#each COINS as c (c.id)}
        {@const d = coinsPrice[c.id]}
        <div class="card">
          <span class="ci">{c.icon}</span>
          <span class="nic">{c.name}</span>
          <span class="val-num">{showUSD ? (d?.usd ?? "—") : (d?.cny ?? "—")}{showUSD ? " $" : "¥"}</span>
          {#if d?.pct != null}
            <span class:up={d.pct >= 0} class:down={d.pct < 0}>{d.pct >= 0 ? "▲" : "▼"}{Math.abs(d.pct).toFixed(2)}%</span>
          {/if}
        </div>
      {/each}
    </div>
    {#if err}
      <div class="err">部分数据不可用（{err.slice(0, 40)}）</div>
    {/if}
  {/if}
</div>

<style>
  .mq { height: 100%; display: flex; flex-direction: column; gap: 6px; overflow: hidden; }
  .head { display: flex; align-items: center; gap: 8px; }
  .title { font-size: 13px; font-weight: 600; flex: 1; }
  .unit { font-size: 11px; color: var(--fg-dim); cursor: pointer; border-bottom: 1px dashed var(--border); }
  .grid { flex: 1; min-height: 0; overflow-y: auto; display: grid; grid-template-columns: 1fr 1fr; gap: 5px; align-content: start; }
  .blk { grid-column: span 2; font-size: 11px; color: var(--fg-dim); margin-top: 4px; }
  .card { display: flex; align-items: center; gap: 6px; padding: 5px 7px; border: 1px solid var(--border); border-radius: 7px; background: var(--bg-input); }
  .ci { font-size: 14px; }
  .nic { font-size: 11px; color: var(--fg-dim); flex: 1; }
  .val, .val-num { font-size: 12px; font-variant-numeric: tabular-nums; }
  .up { color: #4caf50; font-size: 11px; }
  .down { color: #f44336; font-size: 11px; }
  .empty { color: var(--fg-dim); font-size: 11px; margin: auto; }
  .err { font-size: 10px; color: var(--danger); }
</style>
