<script lang="ts">
  // 待办清单小组件（M12）：本地存储（config.json 的 todo.<实例>.items），按实例独立
  import { onMount } from "svelte";
  import { getConfig, setConfig } from "../core/config";

  let { cellId }: { cellId?: string } = $props();

  const instanceId = $derived(cellId ?? "todo");
  const key = $derived(`todo.${instanceId}.items`);

  interface TodoItem {
    id: string;
    text: string;
    done: boolean;
  }

  let items = $state<TodoItem[]>([]);
  let input = $state("");
  let loaded = $state(false);

  onMount(async () => {
    const v = await getConfig(key);
    if (Array.isArray(v)) items = v as TodoItem[];
    loaded = true;
  });

  async function save(): Promise<void> {
    await setConfig(key, items);
  }

  async function add(): Promise<void> {
    const t = input.trim();
    if (!t) return;
    items = [...items, { id: crypto.randomUUID(), text: t, done: false }];
    input = "";
    await save();
  }

  async function toggle(id: string): Promise<void> {
    items = items.map((x) => (x.id === id ? { ...x, done: !x.done } : x));
    await save();
  }

  async function remove(id: string): Promise<void> {
    items = items.filter((x) => x.id !== id);
    await save();
  }
</script>

<div class="todo">
  <div class="addrow">
    <input
      type="text"
      placeholder="添加待办…（Enter）"
      bind:value={input}
      onkeydown={(e) => e.key === "Enter" && void add()}
    />
    <button class="add" onclick={() => void add()}>＋</button>
  </div>
  <div class="list">
    {#each items as t (t.id)}
      <div class="item" class:done={t.done}>
        <button class="check" title={t.done ? "标记未完成" : "标记完成"} onclick={() => void toggle(t.id)}>
          {t.done ? "✅" : "⬜"}
        </button>
        <span class="text">{t.text}</span>
        <button class="del" title="删除此待办" onclick={() => void remove(t.id)}>×</button>
      </div>
    {/each}
    {#if items.length === 0 && loaded}
      <div class="empty">暂无待办</div>
    {/if}
  </div>
</div>

<style>
  .todo {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
    overflow: hidden;
  }
  .addrow {
    display: flex;
    gap: 6px;
  }
  .addrow input {
    flex: 1;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 12px;
    padding: 4px 8px;
    outline: none;
    min-width: 0;
  }
  .addrow input:focus {
    border-color: var(--accent);
  }
  .add {
    border: none;
    border-radius: 6px;
    background: var(--accent);
    color: #fff;
    font-size: 13px;
    padding: 0 10px;
    cursor: pointer;
  }
  .list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
  }
  .item.done .text {
    text-decoration: line-through;
    opacity: 0.55;
  }
  .check {
    border: none;
    background: transparent;
    font-size: 12px;
    cursor: pointer;
    padding: 0;
  }
  .text {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .del {
    border: none;
    background: transparent;
    color: var(--fg-dim);
    font-size: 13px;
    cursor: pointer;
    padding: 0 2px;
  }
  .del:hover {
    color: var(--danger);
  }
  .empty {
    font-size: 11px;
    color: var(--fg-dim);
    text-align: center;
    padding: 10px 0;
  }
</style>
