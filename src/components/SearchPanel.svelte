<script lang="ts">
  // 全局搜索面板（M10）：快捷键唤起，跨「桌面图标/文件夹 + 插件 + 系统应用」搜索
  // 快捷键唤起时前端只监听事件并显示本面板；键盘：↑↓ 选择、Enter 激活、Esc 关闭
  import { onMount } from "svelte";
  import { layout, plugins } from "../core/stores.svelte";
  import { scanApps } from "../core/pluginLoader";
  import type { AppInfo } from "../core/pluginLoader";
  import AppIcon from "./AppIcon.svelte";
  import { getAllSearchEntries } from "../core/searchNames.svelte";
  import { matchesPinyin } from "../core/pinyinSearch";

  let {
    onclose,
    onopenicon,
    onopenfolder,
    onopenapp,
    onopenplugin,
  }: {
    onclose?: () => void;
    onopenicon?: (cellId: string) => void;
    onopenfolder?: (folderId: string) => void;
    onopenapp?: (path: string) => void;
    onopenplugin?: (pluginId: string) => void;
  } = $props();

  type Result =
    | { kind: "icon"; id: string; title: string; emoji: string; group: "桌面" }
    | { kind: "folder"; id: string; title: string; emoji: string; group: "桌面" }
    | { kind: "plugin"; id: string; title: string; emoji: string; group: "插件" }
    | { kind: "app"; id: string; title: string; path: string; group: "应用" }
    | { kind: "entry"; id: string; title: string; sublabel?: string; emoji: string; group: "智能家居"; action?: () => void };

  let query = $state("");
  let apps = $state<AppInfo[]>([]);
  let sel = $state(0);
  let inputEl: HTMLInputElement | undefined;

  onMount(async () => {
    apps = await scanApps();
    inputEl?.focus();
  });

  /** 所有候选（未过滤） */
  const candidates = $derived.by((): Result[] => {
    const list: Result[] = [];
    for (const page of layout.pages) {
      for (const cell of page) {
        if (cell.kind === "folder") {
          list.push({ kind: "folder", id: cell.id, title: cell.name, emoji: cell.emoji, group: "桌面" });
          for (const icon of cell.items) {
            list.push({
              kind: "icon",
              id: icon.id,
              title: icon.title,
              emoji: icon.emoji ?? plugins.find((p) => p.id === icon.pluginId)?.emoji ?? "📦",
              group: "桌面",
            });
          }
        } else {
          list.push({
            kind: "icon",
            id: cell.id,
            title: cell.title,
            emoji: cell.emoji ?? plugins.find((p) => p.id === cell.pluginId)?.emoji ?? "📦",
            group: "桌面",
          });
        }
      }
    }
    for (const p of plugins) {
      list.push({ kind: "plugin", id: p.id, title: p.name, emoji: p.emoji ?? "📦", group: "插件" });
    }
    for (const a of apps) {
      list.push({ kind: "app", id: a.path, title: a.name, path: a.path, group: "应用" });
    }
    // 插件注册的搜索名（如 HomeAssistant 实体"客厅灯"）
    for (const e of getAllSearchEntries()) {
      list.push({
        kind: "entry",
        id: e.sublabel || e.label,
        title: e.label,
        sublabel: e.sublabel,
        emoji: e.emoji ?? "🏠",
        group: "智能家居",
        action: e.action,
      });
    }
    return list;
  });

  const flat = $derived(
    query
      ? candidates.filter((r) => matchesPinyin(r.title, query.toLowerCase()))
      : candidates,
  );

  function activate(r: Result | undefined): void {
    if (!r) return;
    if (r.kind === "icon") onopenicon?.(r.id);
    else if (r.kind === "folder") onopenfolder?.(r.id);
    else if (r.kind === "plugin") onopenplugin?.(r.id);
    else if (r.kind === "entry") {
      r.action?.();
      onclose?.();
    } else onopenapp?.(r.path);
  }

  function onInputKeydown(e: KeyboardEvent): void {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      sel = Math.min(sel + 1, flat.length - 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      sel = Math.max(sel - 1, 0);
    } else if (e.key === "Enter") {
      e.preventDefault();
      activate(flat[sel]);
    } else if (e.key === "Escape") {
      onclose?.();
    }
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
  <div class="panel">
    <div class="search">
      <span class="glass">🔍</span>
      <input
        bind:this={inputEl}
        type="text"
        placeholder="搜索图标 / 插件 / 应用…（Enter 启动）"
        bind:value={query}
        onkeydown={onInputKeydown}
        oninput={() => (sel = 0)}
        oncompositionend={(e) => {
          query = (e.currentTarget as HTMLInputElement).value; // IME 组词结束强制取最终值
          sel = 0;
        }}
      />
      <button class="close" onclick={onclose} title="关闭（Esc）">×</button>
    </div>
    <div class="list">
      {#if flat.length === 0}
        <div class="empty">{query ? "没有匹配的结果" : "暂无内容"}</div>
      {:else}
        {#each flat as r, i (r.kind + r.id)}
          <button
            class="row"
            class:active={i === sel}
            onmouseenter={() => (sel = i)}
            onclick={() => activate(r)}
          >
            {#if r.kind === "app"}
              <AppIcon path={r.path} name={r.title} size={30} radius={8} />
            {:else}
              <span class="emoji">{r.emoji}</span>
            {/if}
            <span class="name">
              {r.title}{#if r.kind === "entry" && r.sublabel}<span class="sub">{r.sublabel}</span>{/if}
            </span>
            <span class="tag">{r.group}</span>
          </button>
        {/each}
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
    align-items: flex-start;
    justify-content: center;
    padding-top: 12vh;
    z-index: 60;
  }
  .panel {
    width: 480px;
    max-height: 60vh;
    background: var(--bg-elev);
    border-radius: 16px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.35);
  }
  .search {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border);
  }
  .glass {
    font-size: 15px;
    opacity: 0.6;
  }
  .search input {
    flex: 1;
    border: none;
    outline: none;
    background: transparent;
    color: var(--fg);
    font-size: 15px;
  }
  .search input::placeholder {
    color: var(--fg-dim);
  }
  .close {
    border: none;
    background: transparent;
    color: var(--fg-dim);
    font-size: 18px;
    cursor: pointer;
  }
  .list {
    overflow-y: auto;
    padding: 6px;
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
  .row:hover,
  .row.active {
    background: var(--bg-hover);
  }
  .emoji {
    font-size: 18px;
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
  .sub {
    margin-left: 6px;
    font-size: 11px;
    color: var(--fg-dim);
  }
  .tag {
    font-size: 10px;
    color: var(--fg-dim);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 1px 6px;
    white-space: nowrap;
  }
  .empty {
    padding: 22px;
    text-align: center;
    color: var(--fg-dim);
    font-size: 13px;
  }
</style>
