<script lang="ts">
  // 资讯聚合：从若干 RSS 源经 rss2json 代理解析，展示标题/时间，点击用默认浏览器打开。
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getPluginSetting } from "../core/pluginSettings.svelte";

  let { }: { cellId?: string } = $props();
  const pid = "dev.homedesktop.news";

  interface Feed { id: string; label: string; emoji: string; url: string; }
  const FEEDS: Feed[] = [
    { id: "cnbeta", label: "cnBeta", emoji: "🧪", url: "https://www.cnbeta.com.tw/backend.php" },
    { id: "sspai", label: "少数派", emoji: "✍️", url: "https://sspai.com/feed" },
    { id: "36kr", label: "36氪", emoji: "💡", url: "https://36kr.com/feed" },
    { id: "github", label: "GitHub.news", emoji: "🐙", url: "https://github.blog/news-archive/feed/" },
  ];

  interface Item { title: string; link: string; pubDate?: string; }
  let active = $state<Feed>(FEEDS[0]);
  let items = $state<Item[]>([]);
  let loading = $state(false);
  let refreshSec = $state(300);

  async function load(): Promise<void> {
    refreshSec = (await getPluginSetting<number>(pid, "refreshSec", refreshSec)) ?? 300;
  }
  function open(link: string): void {
    void invoke("launch_path", { path: link ?? active.url }).catch(() => {});
  }
  async function fetchFeed(): Promise<void> {
    if (!active.url) return;
    loading = true;
    try {
      const url = "https://api.rss2json.com/v1/api.json?rss_url=" + encodeURIComponent(active.url);
      const r = await fetch(url);
      const j = await r.json();
      if (j?.status === "ok") {
        items = (j.items || []).slice(0, 20).map((it: any) => ({
          title: it?.title ?? "",
          link: it?.link ?? active.url,
          pubDate: it?.pubDate ?? "",
        }));
      }
    } catch {
      items = [];
    } finally {
      loading = false;
    }
  }
  function pick(f: Feed): void {
    active = f; items = []; void fetchFeed();
  }
  onMount(() => {
    void load().then(() => fetchFeed());
    const t = setInterval(() => { if (!loading) void fetchFeed(); }, Math.max(30, refreshSec) * 1000);
    return () => clearInterval(t);
  });
  function ago(d?: string): string {
    if (!d) return "";
    const diff = Date.now() - new Date(d.replace(" +0000", "Z")).getTime();
    if (!isFinite(diff)) return "";
    const m = Math.floor(diff / 60000);
    if (m < 1) return "刚刚";
    if (m < 60) return m + " 分前";
    const h = Math.floor(m / 60);
    if (h < 24) return h + " 小时前";
    return Math.floor(h / 24) + " 天前";
  }
</script>

<div class="nw">
  <div class="feeds">
    {#each FEEDS as f (f.id)}
      <button class:on={active.id === f.id} onclick={() => pick(f)}>{f.emoji} {f.label}</button>
    {/each}
  </div>
  {#if loading}
    <div class="status">更新中…</div>
  {:else if items.length === 0}
    <div class="status">暂无内容（可能是网络或源限制）</div>
  {:else}
    <div class="list">
      {#each items as it (it.title.length + it.link)}
        <button class="row" onclick={() => open(it.link)}>
          <span class="t">{it.title}</span>
          <span class="ago">{ago(it.pubDate)}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .nw { height: 100%; display: flex; flex-direction: column; gap: 6px; overflow: hidden; }
  .feeds { display: flex; gap: 5px; flex-wrap: wrap; }
  .feeds button { border: 1px solid var(--border); border-radius: 7px; background: transparent; color: var(--fg-dim); cursor: pointer; font-size: 11px; padding: 3px 7px; }
  .feeds button.on { border-color: var(--accent); color: var(--accent); }
  .status { font-size: 11px; color: var(--fg-dim); text-align: center; margin: auto; }
  .list { flex: 1; min-height: 0; overflow-y: auto; display: flex; flex-direction: column; gap: 3px; }
  .row { display: flex; align-items: center; gap: 8px; text-align: left; padding: 4px 7px; border-radius: 6px; background: var(--bg-input); border: 1px solid transparent; cursor: pointer; }
  .row:hover { border-color: var(--accent); }
  .t { flex: 1; font-size: 12px; color: var(--fg); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ago { font-size: 10px; color: var(--fg-dim); flex-shrink: 0; }
</style>
