<script lang="ts">
  // 插件市场：本地（market/*.zip 目录） + 在线（远程仓库 market/index.json，M18）
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { installPlugin, loadPlugins } from "../core/pluginLoader";
  import type { MarketItem, RemoteMarket, RemoteMarketItem } from "../core/types";
  import { log } from "../core/logger";

  let {
    onmessage,
  }: {
    onmessage?: (msg: string, isError?: boolean) => void;
  } = $props();

  // 在线市场仓库索引（HomeDesktopPlugins 仓库，公开）
  const ONLINE_INDEX_URL =
    "https://raw.githubusercontent.com/Eggplant4363/HomeDesktopPlugins/main/market/index.json";

  let tab = $state<"local" | "online">("local");

  let market = $state<{ dir: string; items: MarketItem[] } | null>(null);
  let online = $state<RemoteMarket | null>(null);
  let error = $state("");
  let loading = $state(false);

  async function refresh(): Promise<void> {
    try {
      market = await invoke<{ dir: string; items: MarketItem[] }>("market_scan");
      error = "";
    } catch (e) {
      error = String(e);
      log.error(`市场扫描失败: ${e}`);
    }
  }

  async function refreshOnline(): Promise<void> {
    loading = true;
    try {
      online = await invoke<RemoteMarket>("market_remote_list", { url: ONLINE_INDEX_URL });
      error = "";
      log.info(`在线市场拉取成功: ${online.items.length} 个插件`);
    } catch (e) {
      error = String(e);
      log.error(`在线市场拉取失败: ${e}`);
      onmessage?.(`在线市场连接失败：${e}`, true);
    } finally {
      loading = false;
    }
  }

  async function install(file: string): Promise<void> {
    try {
      const installed = await installPlugin(file);
      await loadPlugins();
      await refresh();
      log.info(`市场安装插件: ${installed.name}`);
      onmessage?.(`插件「${installed.name}」安装成功`);
    } catch (e) {
      log.error(`市场安装失败: ${file} -> ${e}`);
      onmessage?.(`安装失败：${e}`, true);
    }
  }

  async function installRemote(item: RemoteMarketItem, base: string): Promise<void> {
    try {
      const installed = await invoke<{ name: string; version: string }>("market_remote_install", {
        base,
        file: item.file,
      });
      await loadPlugins();
      await refresh();
      await refreshOnline(); // 同步"已安装"状态
      log.info(`在线安装插件: ${installed.name} v${installed.version}`);
      onmessage?.(`插件「${installed.name}」安装成功`);
    } catch (e) {
      log.error(`在线安装失败: ${item.file} -> ${e}`);
      onmessage?.(`在线安装失败：${e}`, true);
    }
  }

  onMount(() => {
    void refresh();
    void refreshOnline();
  });
</script>

<div class="market-list">
  <div class="tabs">
    <button class="tab" class:active={tab === "local"} onclick={() => (tab = "local")}>
      本地市场
    </button>
    <button class="tab" class:active={tab === "online"} onclick={() => (tab = "online")}>
      在线市场
    </button>
  </div>

  {#if tab === "local"}
    <div class="hint">
      把 zip 插件包放进：<code>{market?.dir ?? "…"}</code>
    </div>
    <button class="refresh" onclick={() => void refresh()}>🔄 刷新列表</button>
    <div class="list">
      {#each market?.items ?? [] as m (m.file)}
        <div class="row">
          <span class="emoji">{m.emoji ?? "📦"}</span>
          <span class="name">{m.name}</span>
          {#if m.pluginType === "widget"}
            <span class="tag">小组件</span>
          {/if}
          <span class="ver">v{m.version}</span>
          {#if m.installed}
            <span class="tag installed">已安装</span>
          {:else}
            <button class="install-btn" onclick={() => void install(m.file)}>安装</button>
          {/if}
        </div>
      {/each}
      {#if market && market.items.length === 0}
        <div class="empty">市场目录暂无插件包</div>
      {/if}
    </div>
  {:else}
    <div class="hint">
      在线市场：<code>{ONLINE_INDEX_URL}</code>
    </div>
    <button class="refresh" disabled={loading} onclick={() => void refreshOnline()}>
      {loading ? "加载中…" : "🔄 刷新列表"}
    </button>
    {#if error}
      <div class="error">{error}</div>
    {/if}
    <div class="list">
      {#each online?.items ?? [] as m (m.id)}
        <div class="row">
          <span class="emoji">{m.emoji ?? "📦"}</span>
          <span class="name">
            {m.name}
            {#if m.description}
              <span class="desc">{m.description}</span>
            {/if}
          </span>
          {#if m.pluginType === "widget"}
            <span class="tag">小组件</span>
          {/if}
          <span class="ver">
            v{m.version}{#if m.size}({(m.size / 1024).toFixed(0)}KB){/if}
          </span>
          {#if m.installed}
            <span class="tag installed">已安装</span>
          {:else}
            <button class="install-btn" onclick={() => void installRemote(m, online?.base ?? "")}>
              安装
            </button>
          {/if}
        </div>
      {/each}
      {#if online && online.items.length === 0}
        <div class="empty">在线市场暂无插件</div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .market-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .tabs {
    display: flex;
    gap: 6px;
  }
  .tab {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 12px;
    padding: 4px 12px;
    cursor: pointer;
  }
  .tab:hover {
    border-color: var(--accent);
  }
  .tab.active {
    border-color: var(--accent);
    background: var(--bg-hover);
    color: #fff;
  }
  .hint {
    font-size: 11px;
    color: var(--fg-dim);
    word-break: break-all;
  }
  .hint code {
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 1px 5px;
  }
  .refresh {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 12px;
    padding: 6px 10px;
    cursor: pointer;
    align-self: flex-start;
  }
  .refresh:hover {
    border-color: var(--accent);
  }
  .refresh:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .error {
    font-size: 12px;
    color: var(--danger);
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 10px;
  }
  .row:hover {
    background: var(--bg-hover);
  }
  .emoji {
    font-size: 18px;
    width: 28px;
    text-align: center;
    flex-shrink: 0;
  }
  .name {
    flex: 1;
    font-size: 14px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .desc {
    font-size: 11px;
    color: var(--fg-dim);
    margin-left: 6px;
  }
  .ver {
    font-size: 11px;
    color: var(--fg-dim);
    white-space: nowrap;
  }
  .tag {
    font-size: 10px;
    color: #fff;
    background: var(--accent);
    border-radius: 6px;
    padding: 1px 6px;
    white-space: nowrap;
  }
  .tag.installed {
    background: var(--fg-dim);
  }
  .install-btn {
    border: none;
    border-radius: 6px;
    background: var(--accent);
    color: #fff;
    font-size: 12px;
    padding: 4px 12px;
    cursor: pointer;
    white-space: nowrap;
  }
  .install-btn:hover {
    opacity: 0.9;
  }
  .empty {
    padding: 20px;
    text-align: center;
    color: var(--fg-dim);
    font-size: 13px;
  }
</style>
