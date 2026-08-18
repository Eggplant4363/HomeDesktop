<script lang="ts">
  // 插件管理页面：两个页签——已安装（卸载/更新）/ 市场（本地目录浏览安装）
  import { plugins } from "../core/stores.svelte";
  import MarketList from "./MarketList.svelte";

  let {
    onclose,
    onupdate,
    onuninstall,
    onmessage,
  }: {
    onclose?: () => void;
    /** 更新：重新选 zip 包安装覆盖 */
    onupdate?: () => void;
    /** 卸载（由外层弹确认框） */
    onuninstall?: (pluginId: string, name: string) => void;
    onmessage?: (msg: string, isError?: boolean) => void;
  } = $props();

  let tab = $state<"installed" | "market">("installed");
</script>

<div
  class="overlay"
  role="button"
  aria-label="关闭"
  tabindex="-1"
  onclick={(e) => e.target === e.currentTarget && onclose?.()}
  onkeydown={(e) => e.key === "Escape" && onclose?.()}
>
  <div class="panel">
    <div class="head">
      <span>🧩 插件管理</span>
      <button class="close" onclick={onclose}>×</button>
    </div>

    <div class="tabs">
      <button class="tab" class:active={tab === "installed"} onclick={() => (tab = "installed")}>
        已安装（{plugins.length}）
      </button>
      <button class="tab" class:active={tab === "market"} onclick={() => (tab = "market")}>
        市场
      </button>
    </div>

    <div class="body">
      {#if tab === "installed"}
        <div class="installed-list">
          {#each plugins as p (p.id)}
            <div class="row">
              <span class="emoji">{p.emoji ?? "📦"}</span>
              <span class="name">{p.name}</span>
              {#if p.pluginType === "widget"}
                <span class="tag">小组件</span>
              {/if}
              <span class="ver">v{p.version}</span>
              {#if p.builtin}
                <span class="tag builtin">内置</span>
              {:else}
                <button class="mini-btn" title="重新选择 zip 包安装覆盖" onclick={() => onupdate?.()}>更新</button>
                <button class="mini-btn danger" onclick={() => onuninstall?.(p.id, p.name)}>卸载</button>
              {/if}
            </div>
          {/each}
          {#if plugins.length === 0}
            <div class="empty">暂无插件</div>
          {/if}
        </div>
      {:else}
        <MarketList onmessage={onmessage} />
      {/if}
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
    z-index: 55;
  }
  .panel {
    width: 460px;
    max-height: 74vh;
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
    padding: 14px 16px 8px;
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
  .tabs {
    display: flex;
    gap: 6px;
    padding: 0 16px 10px;
  }
  .tab {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 13px;
    padding: 6px 14px;
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
  .body {
    overflow-y: auto;
    padding: 0 12px 14px;
  }
  .installed-list {
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
  .tag.builtin {
    background: var(--fg-dim);
  }
  .mini-btn {
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 11px;
    padding: 3px 10px;
    cursor: pointer;
    white-space: nowrap;
  }
  .mini-btn:hover {
    border-color: var(--accent);
  }
  .mini-btn.danger {
    color: var(--danger);
  }
  .empty {
    padding: 22px;
    text-align: center;
    color: var(--fg-dim);
    font-size: 13px;
  }
</style>
