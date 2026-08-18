<script lang="ts">
  import type { PluginInfo } from "../core/types";

  let {
    plugins,
    onadd,
    onnewfolder,
    oninstallplugin,
    onopenplugins,
    onclose,
  }: {
    plugins: PluginInfo[];
    onadd?: (p: PluginInfo) => void;
    onnewfolder?: (name: string) => void;
    oninstallplugin?: () => void;
    onopenplugins?: () => void;
    onclose?: () => void;
  } = $props();

  let newFolderName = $state("");
  let appQuery = $state("");

  const filteredPlugins = $derived(
    appQuery
      ? plugins.filter((p) => p.name.toLowerCase().includes(appQuery.toLowerCase()))
      : plugins,
  );
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
      <span>添加图标 / 小组件</span>
      <button class="close" onclick={onclose}>×</button>
    </div>
    <div class="search">
      <input
        type="text"
        placeholder="搜索插件…"
        bind:value={appQuery}
      />
    </div>
    <div class="list">
      <div class="section-label">插件 / 小组件（应用请用「📱 系统应用」插件添加）</div>
      {#each filteredPlugins as p (p.id)}
        <button class="row" onclick={() => onadd?.(p)}>
          <span class="emoji">{p.emoji ?? "📦"}</span>
          <span class="name">{p.name}</span>
          {#if p.pluginType === "widget"}
            <span class="tag">小组件</span>
          {/if}
          <span class="ver">v{p.version}</span>
        </button>
      {/each}
      {#if filteredPlugins.length === 0 && appQuery === ""}
        <div class="empty">暂无可用插件</div>
      {/if}

      <div class="divider"></div>
      <div class="new-folder">
        <input
          type="text"
          placeholder="新文件夹名称…"
          bind:value={newFolderName}
          onkeydown={(e) => {
            if (e.key === "Enter" && newFolderName.trim()) {
              onnewfolder?.(newFolderName.trim());
              newFolderName = "";
            }
          }}
        />
        <button
          class="create"
          disabled={!newFolderName.trim()}
          onclick={() => {
            onnewfolder?.(newFolderName.trim());
            newFolderName = "";
          }}
        >新建文件夹</button>
      </div>

      {#if oninstallplugin}
        <button class="install-btn" onclick={oninstallplugin}>📦 从 zip 安装插件…</button>
      {/if}
      {#if onopenplugins}
        <button class="install-btn" onclick={onopenplugins}>🧩 插件管理（已安装 / 市场）…</button>
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
    z-index: 40;
  }
  .menu {
    width: 400px;
    max-height: 70vh;
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
  }
  .close {
    border: none;
    background: transparent;
    color: var(--fg-dim);
    font-size: 18px;
    cursor: pointer;
  }
  .search {
    padding: 4px 16px 10px;
  }
  .search input {
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-input);
    color: var(--fg);
    padding: 8px 10px;
    font-size: 13px;
    outline: none;
  }
  .search input:focus {
    border-color: var(--accent);
  }
  .list {
    overflow-y: auto;
    padding: 0 8px 8px;
  }
  .section-label {
    padding: 8px 12px 4px;
    font-size: 11px;
    color: var(--fg-dim);
  }
  .row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    border: none;
    background: transparent;
    color: var(--fg);
    border-radius: 10px;
    cursor: pointer;
    text-align: left;
  }
  .row:hover {
    background: var(--bg-hover);
  }
  .emoji {
    font-size: 20px;
    width: 30px;
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
  }
  .tag {
    font-size: 10px;
    color: #fff;
    background: var(--accent);
    border-radius: 6px;
    padding: 2px 6px;
    white-space: nowrap;
  }
  .divider {
    height: 1px;
    background: var(--border);
    margin: 8px 8px;
  }
  .empty {
    padding: 14px;
    text-align: center;
    color: var(--fg-dim);
    font-size: 12px;
  }
  .new-folder {
    display: flex;
    gap: 8px;
    padding: 6px 8px;
  }
  .new-folder input {
    flex: 1;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-input);
    color: var(--fg);
    padding: 8px 10px;
    font-size: 13px;
    outline: none;
  }
  .new-folder input:focus {
    border-color: var(--accent);
  }
  .create {
    border: none;
    border-radius: 8px;
    background: var(--accent);
    color: #fff;
    font-size: 13px;
    padding: 0 12px;
    cursor: pointer;
  }
  .create:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .install-btn {
    width: 100%;
    border: 1px dashed var(--border);
    border-radius: 10px;
    background: transparent;
    color: var(--fg-dim);
    font-size: 12px;
    padding: 9px 12px;
    margin-top: 8px;
    cursor: pointer;
  }
  .install-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
</style>
