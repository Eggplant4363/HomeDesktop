<script lang="ts">
  import type { PluginInfo } from "../core/types";
  import { getPluginSetting, setPluginSetting } from "../core/pluginSettings.svelte";
  import { log } from "../core/logger";

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
  /** 展开的提供商 id */
  let expandedProvider = $state<string | null>(null);
  /** 正在编辑共享配置的提供商 id */
  let configProvider = $state<string | null>(null);
  /** 保存成功反馈（1.6s 后消失） */
  let savedProvider = $state<string | null>(null);
  let provUrl = $state("");
  let provToken = $state("");

  const q = $derived(appQuery.trim().toLowerCase());

  /** 提供商列表 */
  const providers = $derived(
    plugins.filter((p) => p.pluginType === "provider" && (!q || p.name.toLowerCase().includes(q))),
  );
  /** 无提供商的普通插件 */
  const regular = $derived(
    plugins.filter(
      (p) =>
        p.pluginType !== "provider" &&
        !p.providerId &&
        (!q || p.name.toLowerCase().includes(q)),
    ),
  );
  /** 各提供商的子插件 */
  const subsOf = (providerId: string) =>
    plugins.filter(
      (p) =>
        p.providerId === providerId &&
        (!q || p.name.toLowerCase().includes(q) || providerId.toLowerCase().includes(q)),
    );

  function toggleProvider(id: string): void {
    expandedProvider = expandedProvider === id ? null : id;
    configProvider = null;
  }

  /** 打开提供商共享配置（url/token）；优先显示已保存的值，没有才用 manifest 默认值 */
  async function openConfig(prov: PluginInfo): Promise<void> {
    configProvider = configProvider === prov.id ? null : prov.id;
    if (configProvider !== prov.id) return;
    const defUrl = String(prov.settings?.find((s) => s.key === "url")?.default ?? "");
    const defToken = String(prov.settings?.find((s) => s.key === "token")?.default ?? "");
    provUrl = (await getPluginSetting<string>(prov.id, "url", defUrl)) ?? defUrl;
    provToken = (await getPluginSetting<string>(prov.id, "token", defToken)) ?? defToken;
  }

  async function saveProviderConfig(prov: PluginInfo): Promise<void> {
    await setPluginSetting(prov.id, "url", provUrl.trim());
    await setPluginSetting(prov.id, "token", provToken.trim());
    log.info(`提供商配置已保存: ${prov.name}`);
    savedProvider = prov.id;
    setTimeout(() => {
      if (savedProvider === prov.id) savedProvider = null;
    }, 1600);
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
      {#each regular as p (p.id)}
        <button class="row" onclick={() => onadd?.(p)}>
          <span class="emoji">{p.emoji ?? "📦"}</span>
          <span class="name">{p.name}</span>
          {#if p.pluginType === "widget"}
            <span class="tag">小组件</span>
          {/if}
          <span class="ver">v{p.version}</span>
        </button>
      {/each}
      {#each providers as prov (prov.id)}
        <div class="prov">
          <button class="prov-row" onclick={() => toggleProvider(prov.id)}>
            <span class="emoji">{prov.emoji ?? "📦"}</span>
            <span class="name">{prov.name}</span>
            <span class="prov-count">{subsOf(prov.id).length}</span>
            <span class="chev">{expandedProvider === prov.id ? "▾" : "▸"}</span>
          </button>
          {#if expandedProvider === prov.id}
            <div class="subs">
              {#each subsOf(prov.id) as sub (sub.id)}
                <button class="row sub-row" onclick={() => onadd?.(sub)}>
                  <span class="emoji">{sub.emoji ?? "📦"}</span>
                  <span class="name">{sub.name}</span>
                  {#if sub.domain}
                    <span class="tag dim">{sub.domain}</span>
                  {/if}
                  <span class="ver">v{sub.version}</span>
                </button>
              {/each}
              <button class="row cfg-row" onclick={() => openConfig(prov)}>
                <span class="emoji">⚙</span>
                <span class="name">{configProvider === prov.id ? "收起配置" : "配置「" + prov.name + "」"}</span>
              </button>
              {#if configProvider === prov.id}
                <div class="prov-config">
                  <input type="text" placeholder="HomeAssistant 地址" bind:value={provUrl} />
                  <input type="text" placeholder="长期访问令牌" bind:value={provToken} />
                  <button class="cfg-save" class:saved={savedProvider === prov.id} onclick={() => void saveProviderConfig(prov)}>{savedProvider === prov.id ? "✓ 已保存" : "保存"}</button>
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {/each}
      {#if regular.length === 0 && providers.length === 0 && appQuery === ""}
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
  .prov {
    margin-top: 4px;
  }
  .prov-row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    border: none;
    background: color-mix(in srgb, var(--accent) 10%, transparent);
    color: var(--fg);
    border-radius: 10px;
    cursor: pointer;
    text-align: left;
  }
  .prov-row:hover {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
  }
  .prov-count {
    font-size: 10px;
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 15%, transparent);
    border-radius: 8px;
    padding: 1px 7px;
  }
  .chev {
    color: var(--fg-dim);
    font-size: 11px;
  }
  .subs {
    padding: 2px 0 2px 12px;
  }
  .sub-row {
    border-left: 2px solid var(--border);
    border-radius: 0 10px 10px 0;
  }
  .tag.dim {
    background: transparent;
    color: var(--fg-dim);
    border: 1px solid var(--border);
  }
  .cfg-row {
    color: var(--fg-dim);
  }
  .prov-config {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 6px 12px 6px 24px;
  }
  .prov-config input {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 12px;
    padding: 6px 9px;
    outline: none;
  }
  .prov-config input:focus {
    border-color: var(--accent);
  }
  .cfg-save {
    align-self: flex-end;
    border: none;
    border-radius: 8px;
    background: var(--accent);
    color: #fff;
    font-size: 12px;
    padding: 5px 16px;
    cursor: pointer;
  }
  .cfg-save.saved {
    background: #2e7d32;
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
