<script lang="ts">
  // 系统应用面板：内置「系统应用」插件点击后打开的应用列表（真实图标 + 搜索）
  // 支持"选择自定义 exe 位置"：文件选择器选任意 .exe → 自动提取图标加入
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { scanApps } from "../core/pluginLoader";
  import type { AppInfo } from "../core/pluginLoader";
  import { log } from "../core/logger";
  import AppIcon from "./AppIcon.svelte";

  let {
    onadd,
    onclose,
    mode = "add",
  }: {
    onadd?: (app: AppInfo & { emoji?: string; iconPath?: string }) => void;
    onclose?: () => void;
    /** add=系统应用插件槽位替换；borrow=给图标借用系统应用图标（M9） */
    mode?: "add" | "borrow";
  } = $props();

  let apps = $state<AppInfo[]>([]);
  let appQuery = $state("");
  let picking = $state(false);

  onMount(async () => {
    apps = await scanApps();
  });

  /** 选择自定义 exe：文件对话框 → 取文件名 → 交给外层添加（AppIcon 自动提取图标） */
  async function pickCustomExe(): Promise<void> {
    if (picking) return;
    picking = true;
    try {
      const picked = await open({
        multiple: false,
        filters: [{ name: "应用程序", extensions: ["exe"] }],
      });
      if (typeof picked !== "string" || !picked) return;
      const base = picked.split(/[\\/]/).pop() ?? "";
      const name = base.replace(/\.exe$/i, "") || "自定义应用";
      log.info(`选择自定义 exe: ${picked}`);
      // iconPath → AppIcon 提取真实图标
      onadd?.({ name, path: picked, iconPath: picked });
    } catch (e) {
      log.error(`选择自定义 exe 失败: ${e}`);
    } finally {
      picking = false;
    }
  }

  /** 选择任意文件（文档/图片等）：点击后用系统默认程序打开 */
  async function pickCustomFile(): Promise<void> {
    if (picking) return;
    picking = true;
    try {
      const picked = await open({ multiple: false });
      if (typeof picked !== "string" || !picked) return;
      const base = picked.split(/[\\/]/).pop() ?? "";
      const name = base || "文件";
      log.info(`选择文件: ${picked}`);
      // iconPath → AppIcon 提取系统文件类型图标（docx/pdf/图片等都有真实图标）
      onadd?.({ name, path: picked, iconPath: picked });
    } catch (e) {
      log.error(`选择文件失败: ${e}`);
    } finally {
      picking = false;
    }
  }

  /** 选择文件夹：点击后在资源管理器中打开 */
  async function pickCustomFolder(): Promise<void> {
    if (picking) return;
    picking = true;
    try {
      const picked = await open({ directory: true, multiple: false });
      if (typeof picked !== "string" || !picked) return;
      const name = picked.split(/[\\/]/).pop() || "文件夹";
      log.info(`选择文件夹: ${picked}`);
      onadd?.({ name, path: picked, emoji: "📁" });
    } catch (e) {
      log.error(`选择文件夹失败: ${e}`);
    } finally {
      picking = false;
    }
  }

  const filteredApps = $derived(
    appQuery
      ? apps.filter((a) => a.name.toLowerCase().includes(appQuery.toLowerCase()))
      : apps,
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
  <div class="panel">
    <div class="head">
      <span>{mode === "borrow" ? "🎨 选择图标来源应用" : "📱 系统应用 — 点击选择应用，原位替换此图标"}</span>
      <button class="close" onclick={onclose}>×</button>
    </div>
    <div class="search">
      <input
        type="text"
        placeholder="搜索已安装的应用…"
        bind:value={appQuery}
      />
      {#if mode === "add"}
        <div class="custom-row">
          <button class="custom-btn" onclick={pickCustomExe} disabled={picking}>
            📂 选 exe…
          </button>
          <button class="custom-btn" onclick={pickCustomFile} disabled={picking}>
            📄 选文件…
          </button>
          <button class="custom-btn" onclick={pickCustomFolder} disabled={picking}>
            📁 选文件夹…
          </button>
        </div>
      {/if}
    </div>
    <div class="list">
      {#each filteredApps as app (app.name + app.path)}
        <button class="row" onclick={() => onadd?.(app)}>
          <AppIcon path={app.path} name={app.name} size={36} radius={10} />
          <span class="name">{app.name}</span>
          <span class="path" title={app.path}>{app.path}</span>
        </button>
      {/each}
      {#if filteredApps.length === 0}
        <div class="empty">{appQuery ? "未找到匹配的应用" : "未扫描到应用"}</div>
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
  .custom-row {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }
  .custom-btn {
    flex: 1;
    border: 1px dashed var(--border);
    border-radius: 10px;
    background: transparent;
    color: var(--fg-dim);
    font-size: 12px;
    padding: 9px 6px;
    cursor: pointer;
    white-space: nowrap;
  }
  .custom-btn:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent);
  }
  .custom-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .list {
    overflow-y: auto;
    padding: 0 8px 8px;
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
  .name {
    flex: 1;
    font-size: 14px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .path {
    font-size: 11px;
    color: var(--fg-dim);
    max-width: 40%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .empty {
    padding: 18px;
    text-align: center;
    color: var(--fg-dim);
    font-size: 12px;
  }
</style>
