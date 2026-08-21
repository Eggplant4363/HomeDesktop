<script lang="ts">
  // 待办清单小组件（完善版）：本地存储（config.json 的 todo.<实例>.items），按实例独立
  // 功能：添加/勾选/删除、双击编辑、优先级（高/中/低）、截止日期（过期红色高亮）、
  //       过滤（全部/待办/完成）、进度计数+进度条、到期系统提醒（一次）、清空已完成、智能排序
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getConfig, setConfig } from "../core/config";

  let { cellId }: { cellId?: string } = $props();

  const instanceId = $derived(cellId ?? "todo");
  const key = $derived(`todo.${instanceId}.items`);

  type Priority = "high" | "medium" | "low" | null;

  interface TodoItem {
    id: string;
    text: string;
    done: boolean;
    priority?: Priority;
    /** 截止日期 YYYY-MM-DD */
    due?: string | null;
    /** 已发送过到期提醒（只提醒一次） */
    notified?: boolean;
  }

  let items = $state<TodoItem[]>([]);
  let loaded = $state(false);
  let input = $state("");
  let filter = $state<"all" | "active" | "done">("all");
  // 新增草稿的优先级 / 日期
  let draftPriority = $state<Priority>(null);
  let draftDue = $state<string>("");
  // 编辑状态
  let editingId = $state<string | null>(null);
  let editText = $state("");
  // 日期编辑（item 内联）
  let dateEditingId = $state<string | null>(null);

  const PRIORITY_LABEL: Record<string, string> = { high: "高", medium: "中", low: "低" };
  const PRIORITY_COLOR: Record<string, string> = {
    high: "var(--danger)",
    medium: "#e67e22",
    low: "#3498db",
  };

  function toDateStr(d: Date): string {
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, "0");
    const day = String(d.getDate()).padStart(2, "0");
    return `${y}-${m}-${day}`;
  }
  const today = $derived(toDateStr(new Date()));

  function isOverdue(due: string | null | undefined): boolean {
    return !!due && due < today;
  }
  function isToday(due: string | null | undefined): boolean {
    return !!due && due === today;
  }

  onMount(async () => {
    const v = await getConfig(key);
    if (Array.isArray(v)) items = v as TodoItem[];
    loaded = true;
    void checkDue();
  });

  async function save(): Promise<void> {
    await setConfig(key, items);
  }

  // ---------- 排序：待办优先 → 优先级高→低 → 截止日期近→远；已完成沉底 ----------
  const sorted = $derived(
    [...items].sort((a, b) => {
      if (a.done !== b.done) return a.done ? 1 : -1;
      const rank = (p: Priority) => (p === "high" ? 0 : p === "medium" ? 1 : p === "low" ? 2 : 3);
      const ra = rank(a.priority ?? null);
      const rb = rank(b.priority ?? null);
      if (ra !== rb) return ra - rb;
      const da = a.due ?? "9999-12-31";
      const db = b.due ?? "9999-12-31";
      return da < db ? -1 : da > db ? 1 : 0;
    }),
  );

  const visible = $derived(
    sorted.filter((x) => (filter === "all" ? true : filter === "done" ? x.done : !x.done)),
  );

  const doneCount = $derived(items.filter((x) => x.done).length);
  const progress = $derived(items.length === 0 ? 0 : doneCount / items.length);

  // ---------- 添加 ----------
  async function add(): Promise<void> {
    const t = input.trim();
    if (!t) return;
    items = [
      ...items,
      { id: crypto.randomUUID(), text: t, done: false, priority: draftPriority, due: draftDue || null },
    ];
    input = "";
    draftDue = "";
    await save();
  }

  // ---------- 勾选 / 删除 ----------
  async function toggle(id: string): Promise<void> {
    items = items.map((x) => (x.id === id ? { ...x, done: !x.done } : x));
    await save();
  }

  async function remove(id: string): Promise<void> {
    items = items.filter((x) => x.id !== id);
    await save();
  }

  async function clearDone(): Promise<void> {
    items = items.filter((x) => !x.done);
    await save();
  }

  // ---------- 编辑 ----------
  function startEdit(id: string, text: string): void {
    editingId = id;
    editText = text;
  }

  async function commitEdit(id: string): Promise<void> {
    const t = editText.trim();
    editingId = null;
    if (!t) return;
    items = items.map((x) => (x.id === id ? { ...x, text: t } : x));
    await save();
  }

  // ---------- 优先级（点击循环） ----------
  const CYCLE: Priority[] = [null, "high", "medium", "low"];
  async function cyclePriority(id: string): Promise<void> {
    const item = items.find((x) => x.id === id);
    if (!item) return;
    const cur = item.priority ?? null;
    const next = CYCLE[(CYCLE.indexOf(cur) + 1) % CYCLE.length];
    items = items.map((x) => (x.id === id ? { ...x, priority: next } : x));
    await save();
  }

  // ---------- 截止日期（内联日期输入） ----------
  async function commitDue(id: string, value: string): Promise<void> {
    dateEditingId = null;
    items = items.map((x) => (x.id === id ? { ...x, due: value || null } : x));
    await save();
  }

  // ---------- 到期提醒（每分钟检查一次，已到期的发一次系统通知） ----------
  async function checkDue(): Promise<void> {
    let changed = false;
    for (const item of items) {
      if (!item.done && item.due && item.due <= today && !item.notified) {
        try {
          await invoke("app_notify", { title: "待办提醒", body: `「${item.text}」已到期` });
        } catch {
          /* 通知失败忽略 */
        }
        items = items.map((x) => (x.id === item.id ? { ...x, notified: true } : x));
        changed = true;
      }
    }
    if (changed) await save();
  }

  $effect(() => {
    const t = setInterval(() => void checkDue(), 60000);
    return () => clearInterval(t);
  });

  /** 内联编辑输入框自动聚焦 */
  function focusEl(el: HTMLInputElement | null): void {
    el?.focus();
  }
</script>

<div class="todo">
  <div class="head">
    <div class="progress" title="{doneCount}/{items.length} 已完成">
      <div class="bar" style="width:{progress * 100}%"></div>
    </div>
    <span class="count">{doneCount}/{items.length}</span>
  </div>
  <div class="tabs">
    <button class="tab" class:on={filter === "all"} onclick={() => (filter = "all")}>全部</button>
    <button class="tab" class:on={filter === "active"} onclick={() => (filter = "active")}>待办</button>
    <button class="tab" class:on={filter === "done"} onclick={() => (filter = "done")}>完成</button>
    {#if doneCount > 0}
      <button class="clear" title="清空已完成" onclick={() => void clearDone()}>🗑 清空</button>
    {/if}
  </div>
  <div class="addrow">
    <input
      type="text"
      placeholder="添加待办…（Enter）"
      bind:value={input}
      onkeydown={(e) => e.key === "Enter" && void add()}
    />
    <button
      class="flag"
      class:on={draftPriority !== null}
      style={draftPriority ? `color:${PRIORITY_COLOR[draftPriority]}` : ""}
      title={draftPriority ? `新增优先级：${PRIORITY_LABEL[draftPriority]}` : "设置新增优先级"}
      onclick={() => (draftPriority = CYCLE[(CYCLE.indexOf(draftPriority) + 1) % CYCLE.length])}
    >⚑</button>
    <button class="date-btn" title={draftDue ? `截止：${draftDue}` : "设置截止日期"} onclick={() => dateEditingId = "draft"}>📅</button>
    <button class="add" onclick={() => void add()}>＋</button>
  </div>
  {#if dateEditingId === "draft"}
    <div class="date-inline">
      <input type="date" value={draftDue} onchange={(e) => (draftDue = (e.target as HTMLInputElement).value)} />
      <button onclick={() => (dateEditingId = null)}>✓</button>
    </div>
  {/if}
  <div class="list">
    {#each visible as t (t.id)}
      <div class="item" class:done={t.done} class:overdue={isOverdue(t.due)}>
        <button class="check" title={t.done ? "标记未完成" : "标记完成"} onclick={() => void toggle(t.id)}>
          {t.done ? "✅" : "⬜"}
        </button>
        <button
          class="prio"
          class:on={t.priority !== null}
          style={t.priority ? `color:${PRIORITY_COLOR[t.priority]}` : ""}
          title={t.priority ? `优先级：${PRIORITY_LABEL[t.priority]}（点击切换）` : "无优先级（点击设置）"}
          onclick={() => void cyclePriority(t.id)}
        >⚑</button>
        {#if editingId === t.id}
          <input
            class="edit"
            bind:value={editText}
            onkeydown={(e) => {
              if (e.key === "Enter") void commitEdit(t.id);
              if (e.key === "Escape") editingId = null;
            }}
            onblur={() => void commitEdit(t.id)}
            use:focusEl
          />
        {:else}
          <span
            class="text"
            title="双击编辑"
            role="button"
            tabindex="-1"
            ondblclick={() => startEdit(t.id, t.text)}
            onkeydown={(e) => e.key === "Enter" && startEdit(t.id, t.text)}
          >{t.text}</span>
        {/if}
        {#if dateEditingId === t.id}
          <input
            class="due-edit"
            type="date"
            value={t.due ?? ""}
            onchange={(e) => void commitDue(t.id, (e.target as HTMLInputElement).value)}
          />
        {:else}
          {#if t.due}
            <button
              class="due"
              class:overdue={isOverdue(t.due)}
              class:today={isToday(t.due)}
              title={`截止：${t.due}（点击修改）${isOverdue(t.due) ? "，已过期" : ""}`}
              onclick={() => (dateEditingId = t.id)}
            >{t.due.slice(5).replace("-", "/")}</button>
          {:else}
            <button class="due empty" title="设置截止日期" onclick={() => (dateEditingId = t.id)}>📅</button>
          {/if}
        {/if}
        <button class="del" title="删除此待办" onclick={() => void remove(t.id)}>×</button>
      </div>
    {/each}
    {#if visible.length === 0 && loaded}
      <div class="empty">{filter === "all" ? "暂无待办" : filter === "done" ? "没有已完成项" : "没有待办项"}</div>
    {/if}
  </div>
</div>

<style>
  .todo {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 4px;
    overflow: hidden;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .progress {
    flex: 1;
    height: 5px;
    border-radius: 3px;
    background: var(--bg-hover);
    overflow: hidden;
  }
  .bar {
    height: 100%;
    background: var(--accent);
    border-radius: 3px;
    transition: width 0.3s;
  }
  .count {
    font-size: 10px;
    color: var(--fg-dim);
    font-variant-numeric: tabular-nums;
  }
  .tabs {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .tab {
    border: none;
    background: transparent;
    color: var(--fg-dim);
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 6px;
    cursor: pointer;
  }
  .tab.on {
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    color: var(--accent);
  }
  .clear {
    margin-left: auto;
    border: none;
    background: transparent;
    color: var(--fg-dim);
    font-size: 10px;
    cursor: pointer;
    padding: 1px 4px;
  }
  .clear:hover {
    color: var(--danger);
  }
  .addrow {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .addrow input {
    flex: 1;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 11px;
    padding: 3px 7px;
    outline: none;
    min-width: 0;
  }
  .addrow input:focus {
    border-color: var(--accent);
  }
  .flag,
  .date-btn {
    border: 1px solid var(--border);
    border-radius: 6px;
    background: transparent;
    color: var(--fg-dim);
    font-size: 11px;
    padding: 0 6px;
    height: 22px;
    cursor: pointer;
  }
  .flag.on {
    border-color: currentColor;
  }
  .add {
    border: none;
    border-radius: 6px;
    background: var(--accent);
    color: #fff;
    font-size: 12px;
    height: 22px;
    padding: 0 9px;
    cursor: pointer;
  }
  .date-inline {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
    color: var(--fg-dim);
  }
  .date-inline input {
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 10px;
    padding: 1px 4px;
    outline: none;
  }
  .date-inline button {
    border: none;
    background: var(--accent);
    color: #fff;
    border-radius: 6px;
    font-size: 10px;
    cursor: pointer;
    padding: 1px 6px;
  }
  .list {
    flex: 1;
    overflow-y: auto;
    scrollbar-width: none;
    -ms-overflow-style: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .list::-webkit-scrollbar {
    display: none;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    padding: 2px 4px;
    border-radius: 6px;
  }
  .item:hover {
    background: var(--bg-hover);
  }
  .item.done .text {
    text-decoration: line-through;
    opacity: 0.5;
  }
  .item.overdue .due {
    color: var(--danger);
    font-weight: 600;
  }
  .check {
    border: none;
    background: transparent;
    font-size: 11px;
    cursor: pointer;
    padding: 0;
    flex-shrink: 0;
  }
  .prio {
    border: none;
    background: transparent;
    font-size: 10px;
    cursor: pointer;
    padding: 0;
    opacity: 0.35;
    flex-shrink: 0;
  }
  .prio.on {
    opacity: 1;
  }
  .text {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: default;
    min-width: 0;
  }
  .edit {
    flex: 1;
    border: 1px solid var(--accent);
    border-radius: 4px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 11px;
    padding: 1px 5px;
    outline: none;
    min-width: 0;
  }
  .due {
    border: none;
    background: transparent;
    color: var(--fg-dim);
    font-size: 10px;
    cursor: pointer;
    padding: 0 2px;
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
  }
  .due.today {
    color: var(--accent);
  }
  .due.empty {
    opacity: 0.4;
  }
  .due-edit {
    width: 92px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 10px;
    outline: none;
  }
  .del {
    border: none;
    background: transparent;
    color: var(--fg-dim);
    font-size: 12px;
    cursor: pointer;
    padding: 0 2px;
    flex-shrink: 0;
  }
  .del:hover {
    color: var(--danger);
  }
  .empty {
    font-size: 10px;
    color: var(--fg-dim);
    text-align: center;
    padding: 8px 0;
  }
</style>
