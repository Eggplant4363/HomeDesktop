<script lang="ts">
  // 剪贴板历史小组件：依赖 Rust 后台全局监听（任何应用复制都自动收录）。
  // 展示最近历史；点击条目=写回系统剪贴板（供原应用粘贴）；可清空/移除单条。
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";

  let { }: { cellId?: string } = $props();

  interface ClipEntry {
    id: number;
    text: string;
    at: number;
  }

  let items = $state<ClipEntry[]>([]);

  async function loadState(): Promise<void> {
    try {
      items = await invoke<ClipEntry[]>("clip_state");
    } catch {
      /* 忽略 */
    }
  }

  async function paste(t: string): Promise<void> {
    try {
      await invoke("clip_paste", { text: t });
    } catch {
      /* 忽略 */
    }
  }

  async function clearAll(): Promise<void> {
    await invoke("clip_clear");
    items = [];
  }

  function del(id: number): void {
    items = items.filter((i) => i.id !== id);
  }

  let un: (() => void)[] = [];
  onMount(() => {
    void loadState();
    listen<{ id: number; text: string; at: number }>("cbpush", (ev) => {
      const e = ev.payload;
      const txt = e?.text ?? "";
      if (!txt) return;
      items = [{ id: e.id, text: txt, at: e.at }, ...items.filter((i) => i.text !== txt)].slice(0, 80);
    }).then((f) => { un.push(f); }).catch(() => {});
    return () => { un.forEach((f) => f()); };
  });
</script>

<div class="cb">
  <div class="head">
    <span class="title">📋 剪贴板历史</span>
    {#if items.length}
      <button class="clear" onclick={() => void clearAll()}>清空</button>
    {/if}
  </div>
  <div class="hint">复制任意内容自动收录（含其他软件）· 点条目=临时复制，可粘贴回</div>

  {#if items.length === 0}
    <div class="empty">复制点文字试试——会自动记到这里</div>
  {:else}
    <div class="list">
      {#each items as it (it.id)}
        <button class="row" onclick={() => void paste(it.text)} title="复制这条供粘贴">
          <span class="txt">{it.text.length > 200 ? it.text.slice(0, 200) + "…" : it.text}</span>
          <span
            class="x"
            role="button"
            onclick={(e) => { e.stopPropagation(); del(it.id); }}
          >×</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .cb {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
    overflow: hidden;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .title {
    font-size: 13px;
    font-weight: 600;
    flex: 1;
  }
  .clear {
    border: none;
    background: transparent;
    color: var(--danger);
    cursor: pointer;
    font-size: 12px;
  }
  .hint {
    font-size: 10px;
    color: var(--fg-dim);
    opacity: 0.85;
  }
  .empty {
    font-size: 11px;
    color: var(--fg-dim);
    text-align: center;
    margin: auto;
    padding: 12px;
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 8px;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--bg-input);
    text-align: left;
    cursor: pointer;
  }
  .row:hover {
    border-color: var(--accent);
  }
  .txt {
    flex: 1;
    font-size: 12px;
    color: var(--fg);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .x {
    color: var(--fg-dim);
    cursor: pointer;
    font-size: 13px;
  }
  .x:hover {
    color: var(--danger);
  }
</style>
