<script lang="ts">
  // 待办清单小组件（完善版 v2）：本地存储（config.json 的 todo.<实例>.items），按实例独立
  // 功能：添加/勾选/删除、双击编辑、优先级（高/中/低）、可视化日期时间选择、多级子任务、
  //       展开/折叠、过滤、进度（含子任务递归）、到时系统提醒（一次）、清空已完成、智能排序、详情弹层
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getConfig, setConfig } from "../core/config";
  import CalendarPopover from "../components/CalendarPopover.svelte";

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
    /** 截止时间 HH:mm（可选，与 due 配合做到时提醒） */
    dueTime?: string | null;
    /** 已发送过到期提醒（只提醒一次） */
    notified?: boolean;
    /** 提醒提前量（分钟）：null=不提醒，0=准时，5/15/30/60/1440=提前N分钟 */
    remind?: number | null;
    /** 子任务（多级） */
    children?: TodoItem[];
    /** 展开子任务（UI 状态，持久化） */
    expanded?: boolean;
    createdAt: number;
  }

  let items = $state<TodoItem[]>([]);
  let loaded = $state(false);
  let input = $state("");
  let filter = $state<"all" | "active" | "done">("all");
  let draftPriority = $state<Priority>(null);
  let draftDue = $state("");
  let draftTime = $state("");
  let draftOpen = $state(false); // 新增时是否显示可视化日期选择
  // 详情弹层
  let detailId = $state<string | null>(null);
  let detailText = $state("");
  // 内联编辑
  let editingId = $state<string | null>(null);
  let editText = $state("");
  // 添加子任务
  let addingChildId = $state<string | null>(null);
  let childInput = $state("");

  const REMINDS: { value: number | null; label: string }[] = [
    { value: null, label: "不提醒" },
    { value: 0, label: "准时" },
    { value: 5, label: "提前5分" },
    { value: 15, label: "提前15分" },
    { value: 30, label: "提前30分" },
    { value: 60, label: "提前1小时" },
    { value: 1440, label: "提前1天" },
  ];
  const PRIORITY_LABEL: Record<string, string> = { high: "高", medium: "中", low: "低" };
  const PRIORITY_COLOR: Record<string, string> = {
    high: "var(--danger)",
    medium: "#e67e22",
    low: "#3498db",
  };

  function pad(n: number): string {
    return String(n).padStart(2, "0");
  }
  function dateStr(d: Date): string {
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  }
  function timeStr(d: Date): string {
    return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }
  const today = $derived(dateStr(new Date()));
  const nowHM = $derived(timeStr(new Date()));

  // ---------- 递归工具 ----------
  function updateRec(list: TodoItem[], id: string, fn: (t: TodoItem) => TodoItem): TodoItem[] {
    return list.map((t) => {
      if (t.id === id) return fn(t);
      if (t.children && t.children.length > 0) return { ...t, children: updateRec(t.children, id, fn) };
      return t;
    });
  }
  function removeRec(list: TodoItem[], id: string): TodoItem[] {
    return list
      .filter((t) => t.id !== id)
      .map((t) =>
        t.children && t.children.length > 0 ? { ...t, children: removeRec(t.children, id) } : t,
      );
  }
  function findRec(list: TodoItem[], id: string): TodoItem | undefined {
    for (const t of list) {
      if (t.id === id) return t;
      if (t.children) {
        const f = findRec(t.children, id);
        if (f) return f;
      }
    }
    return undefined;
  }
  /** 递归计数（含子任务） */
  function countRec(list: TodoItem[]): { done: number; total: number } {
    let done = 0;
    let total = 0;
    for (const t of list) {
      total += 1;
      if (t.done) done += 1;
      if (t.children && t.children.length > 0) {
        const c = countRec(t.children);
        done += c.done;
        total += c.total;
      }
    }
    return { done, total };
  }
  /** 递归排序：待办优先 → 优先级 → 截止日期近→远；子任务同规则 */
  function sortRec(list: TodoItem[]): TodoItem[] {
    const rank = (p: Priority) => (p === "high" ? 0 : p === "medium" ? 1 : p === "low" ? 2 : 3);
    return [...list]
      .sort((a, b) => {
        if (a.done !== b.done) return a.done ? 1 : -1;
        const ra = rank(a.priority ?? null);
        const rb = rank(b.priority ?? null);
        if (ra !== rb) return ra - rb;
        const da = a.due ?? "9999-12-31";
        const db = b.due ?? "9999-12-31";
        return da < db ? -1 : da > db ? 1 : 0;
      })
      .map((t) =>
        t.children && t.children.length > 0 ? { ...t, children: sortRec(t.children) } : t,
      );
  }
  /** 展开的扁平视图（含层级） */
  interface FlatItem {
    item: TodoItem;
    depth: number;
  }
  function flatten(list: TodoItem[], out: FlatItem[] = [], depth = 0): FlatItem[] {
    for (const t of list) {
      out.push({ item: t, depth });
      if (t.children && t.children.length > 0 && t.expanded !== false) {
        flatten(t.children, out, depth + 1);
      }
    }
    return out;
  }

  // ---------- 派生数据 ----------
  const sorted = $derived(sortRec(items));
  const flat = $derived(flatten(sorted));
  const visible = $derived(
    flat.filter(({ item }) => (filter === "all" ? true : filter === "done" ? item.done : !item.done)),
  );
  const allCount = $derived(countRec(items));
  const progress = $derived(allCount.total === 0 ? 0 : allCount.done / allCount.total);

  function childCount(t: TodoItem): { done: number; total: number } {
    if (!t.children || t.children.length === 0) return { done: 0, total: 0 };
    return countRec(t.children);
  }
  function isOverdue(t: TodoItem): boolean {
    if (t.done || !t.due) return false;
    if (t.due < today) return true;
    if (t.due === today && t.dueTime && t.dueTime < nowHM) return true;
    return false;
  }
  function isTodayDue(t: TodoItem): boolean {
    return !!t.due && t.due === today && !t.done;
  }
  function dueLabel(t: TodoItem): string {
    if (!t.due) return "";
    return t.due.slice(5).replace("-", "/") + (t.dueTime ? ` ${t.dueTime}` : "");
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

  // ---------- 添加 ----------
  async function add(): Promise<void> {
    const t = input.trim();
    if (!t) return;
    items = [
      ...items,
      {
        id: crypto.randomUUID(),
        text: t,
        done: false,
        priority: draftPriority,
        due: draftDue || null,
        dueTime: draftDue ? draftTime || null : null,
        remind: 0,
        createdAt: Date.now(),
      },
    ];
    input = "";
    draftPriority = null;
    draftDue = "";
    draftTime = "";
    draftOpen = false;
    await save();
  }

  async function addChild(parentId: string): Promise<void> {
    const t = childInput.trim();
    if (!t) return;
    items = updateRec(items, parentId, (p) => ({
      ...p,
      expanded: true,
      children: [
        ...(p.children ?? []),
        { id: crypto.randomUUID(), text: t, done: false, createdAt: Date.now() },
      ],
    }));
    childInput = "";
    addingChildId = null;
    await save();
  }

  // ---------- 勾选 / 删除 / 清空 ----------
  async function toggle(id: string): Promise<void> {
    items = updateRec(items, id, (t) => ({ ...t, done: !t.done }));
    await save();
  }
  async function remove(id: string): Promise<void> {
    items = removeRec(items, id);
    if (detailId === id) detailId = null;
    await save();
  }
  async function clearDone(): Promise<void> {
    const prune = (list: TodoItem[]): TodoItem[] =>
      list
        .filter((t) => !t.done)
        .map((t) => (t.children ? { ...t, children: prune(t.children) } : t));
    items = prune(items);
    await save();
  }

  // ---------- 内联编辑 ----------
  function startEdit(id: string, text: string): void {
    editingId = id;
    editText = text;
  }
  async function commitEdit(id: string): Promise<void> {
    const t = editText.trim();
    editingId = null;
    if (!t) return;
    items = updateRec(items, id, (x) => ({ ...x, text: t }));
    await save();
  }
  function focusEl(el: HTMLInputElement | null): void {
    el?.focus();
  }

  // ---------- 优先级 / 展开 ----------
  const CYCLE: Priority[] = [null, "high", "medium", "low"];
  const PRIORITIES: Priority[] = [null, "high", "medium", "low"];
  async function cyclePriority(id: string): Promise<void> {
    const item = findRec(items, id);
    if (!item) return;
    const cur = item.priority ?? null;
    const next = CYCLE[(CYCLE.indexOf(cur) + 1) % CYCLE.length];
    items = updateRec(items, id, (x) => ({ ...x, priority: next }));
    await save();
  }
  async function toggleExpand(id: string): Promise<void> {
    items = updateRec(items, id, (x) => ({ ...x, expanded: x.expanded === false ? true : false }));
    await save();
  }

  // ---------- 详情弹层 ----------
  function openDetail(id: string): void {
    const t = findRec(items, id);
    if (!t) return;
    detailId = id;
    detailText = t.text;
  }
  async function commitDetailText(): Promise<void> {
    const t = detailText.trim();
    if (detailId && t) {
      items = updateRec(items, detailId, (x) => ({ ...x, text: t }));
      await save();
    }
  }
  async function setDetailPriority(p: Priority): Promise<void> {
    if (!detailId) return;
    items = updateRec(items, detailId, (x) => ({ ...x, priority: p }));
    await save();
  }
  async function setDetailDue(date: string, time: string): Promise<void> {
    if (!detailId) return;
    items = updateRec(items, detailId, (x) => ({
      ...x,
      due: date || null,
      dueTime: date ? time || null : null,
      notified: false, // 改期后重新提醒
    }));
    await save();
  }
  async function setDetailRemind(v: number | null): Promise<void> {
    if (!detailId) return;
    items = updateRec(items, detailId, (x) => ({ ...x, remind: v, notified: false }));
    await save();
  }

  // ---------- 到时提醒（每 30s 检查，到时发一次系统通知） ----------
  async function checkDue(): Promise<void> {
    const now = Date.now();
    let changed = false;
    const walk = async (list: TodoItem[]): Promise<void> => {
      for (const item of list) {
        const lead = item.remind === null ? -1 : (item.remind ?? 0); // -1 = 不提醒
        if (!item.done && item.due && lead >= 0 && !item.notified) {
          const at =
            new Date(`${item.due}T${item.dueTime ?? "23:59"}:00`).getTime() - lead * 60000;
          if (!Number.isNaN(at) && now >= at) {
            const body =
              lead > 0
                ? `「${item.text}」将在 ${lead >= 60 ? `${lead / 60} 小时` : `${lead} 分钟`}后到期`
                : `「${item.text}」${item.dueTime ? "已到设定时间" : "已到期"}`;
            try {
              await invoke("app_notify", { title: "待办提醒", body });
            } catch {
              /* 通知失败忽略 */
            }
            items = updateRec(items, item.id, (x) => ({ ...x, notified: true }));
            changed = true;
          }
        }
        if (item.children && item.children.length > 0) await walk(item.children);
      }
    };
    await walk(items);
    if (changed) await save();
  }

  $effect(() => {
    const t = setInterval(() => void checkDue(), 30000);
    return () => clearInterval(t);
  });

  // 详情弹层当前任务
  const detailItem = $derived(detailId ? findRec(items, detailId) : undefined);
</script>

<div class="todo">
  <div class="head">
    <div class="progress" title="{allCount.done}/{allCount.total} 已完成">
      <div class="bar" style="width:{progress * 100}%"></div>
    </div>
    <span class="count">{allCount.done}/{allCount.total}</span>
  </div>
  <div class="tabs">
    <button class="tab" class:on={filter === "all"} onclick={() => (filter = "all")}>全部</button>
    <button class="tab" class:on={filter === "active"} onclick={() => (filter = "active")}>待办</button>
    <button class="tab" class:on={filter === "done"} onclick={() => (filter = "done")}>完成</button>
    {#if allCount.done > 0}
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
    <button
      class="cal-btn"
      class:on={draftOpen}
      title="可视化选择日期时间"
      onclick={() => (draftOpen = !draftOpen)}
    >📅</button>
    <button class="add" onclick={() => void add()}>＋</button>
  </div>
  {#if draftOpen}
    <div class="cal-wrap">
      <CalendarPopover
        value={draftDue}
        time={draftTime}
        onchange={(d, t) => {
          draftDue = d;
          draftTime = t;
        }}
      />
    </div>
  {/if}
  <div class="list">
    {#each visible as { item: t, depth } (t.id)}
      <div
        class="item"
        class:done={t.done}
        class:overdue={isOverdue(t)}
        style="padding-left:{6 + depth * 16}px;"
      >
        {#if t.children && t.children.length > 0}
          <button
            class="exp"
            title={t.expanded === false ? "展开子任务" : "折叠子任务"}
            onclick={() => void toggleExpand(t.id)}
          >{t.expanded === false ? "▸" : "▾"}</button>
          {#if childCount(t).total > 0}
            <span class="child-prog" title="子任务进度">{childCount(t).done}/{childCount(t).total}</span>
          {/if}
        {:else}
          <span class="exp-placeholder"></span>
        {/if}
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
        {#if t.due}
          <button
            class="due"
            class:overdue={isOverdue(t)}
            class:today={isTodayDue(t)}
            title={`截止：${t.due}${t.dueTime ? ` ${t.dueTime}` : ""}（点击修改）`}
            onclick={() => openDetail(t.id)}
          >{dueLabel(t)}</button>
        {/if}
        <button
          class="plus"
          title="添加子任务"
          onclick={() => (addingChildId = addingChildId === t.id ? null : t.id)}
        >＋</button>
        <button class="detail-btn" title="详情（优先级/日期时间/子任务）" onclick={() => openDetail(t.id)}>⋯</button>
        <button class="del" title="删除此待办" onclick={() => void remove(t.id)}>×</button>
      </div>
      {#if addingChildId === t.id}
        <div class="child-add" style="padding-left:{6 + (depth + 1) * 16}px;">
          <input
            placeholder="子任务…（Enter）"
            bind:value={childInput}
            onkeydown={(e) => {
              if (e.key === "Enter") void addChild(t.id);
              if (e.key === "Escape") addingChildId = null;
            }}
            use:focusEl
          />
          <button onclick={() => void addChild(t.id)}>✓</button>
        </div>
      {/if}
    {/each}
    {#if visible.length === 0 && loaded}
      <div class="empty">{filter === "all" ? "暂无待办" : filter === "done" ? "没有已完成项" : "没有待办项"}</div>
    {/if}
  </div>
</div>

{#if detailItem}
  <div
    class="detail-overlay"
    role="button"
    tabindex="-1"
    onclick={(e) => e.target === e.currentTarget && (detailId = null)}
    onkeydown={(e) => e.key === "Escape" && (detailId = null)}
  >
    <div class="detail">
      <div class="d-head">
        <span>待办详情</span>
        <button class="d-close" onclick={() => (detailId = null)}>×</button>
      </div>
      <input
        class="d-text"
        value={detailText}
        oninput={(e) => (detailText = (e.target as HTMLInputElement).value)}
        onblur={() => void commitDetailText()}
        onkeydown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
      />
      <div class="d-row">
        <span class="d-label">优先级</span>
        <div class="prio-btns">
          {#each PRIORITIES as p, i (i)}
            <button
              class="pbtn"
              class:on={detailItem.priority === p}
              style={p ? `color:${PRIORITY_COLOR[p]}` : ""}
              onclick={() => void setDetailPriority(p)}
            >{p ? PRIORITY_LABEL[p] : "无"}</button>
          {/each}
        </div>
      </div>
      <div class="d-row">
        <span class="d-label">提醒（相对截止时间的提前量）</span>
        <div class="remind-btns">
          {#each REMINDS as r, i (i)}
            <button
              class="rbtn"
              class:on={(detailItem.remind ?? 0) === r.value}
              onclick={() => void setDetailRemind(r.value)}
            >{r.label}</button>
          {/each}
        </div>
      </div>
      <div class="d-row">
        <span class="d-label">截止（可视化选择日期与时间，到时提醒）</span>
        <div class="d-cal">
          <CalendarPopover
            value={detailItem.due ?? ""}
            time={detailItem.dueTime ?? ""}
            onchange={(d, t) => void setDetailDue(d, t)}
          />
        </div>
      </div>
      <div class="d-foot">
        <button class="d-del" onclick={() => void remove(detailItem.id)}>🗑 删除此待办</button>
        <button class="d-ok" onclick={() => (detailId = null)}>完成</button>
      </div>
    </div>
  </div>
{/if}

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
  .cal-btn {
    border: 1px solid var(--border);
    border-radius: 6px;
    background: transparent;
    color: var(--fg-dim);
    font-size: 11px;
    padding: 0 6px;
    height: 22px;
    cursor: pointer;
  }
  .flag.on,
  .cal-btn.on {
    border-color: var(--accent);
    color: var(--accent);
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
  .cal-wrap {
    position: relative;
    z-index: 5;
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
  .exp,
  .exp-placeholder {
    width: 14px;
    flex-shrink: 0;
    border: none;
    background: transparent;
    color: var(--fg-dim);
    font-size: 10px;
    cursor: pointer;
    padding: 0;
  }
  .child-prog {
    font-size: 9px;
    color: var(--fg-dim);
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
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
  .plus,
  .detail-btn {
    border: none;
    background: transparent;
    color: var(--fg-dim);
    font-size: 11px;
    cursor: pointer;
    padding: 0 2px;
    flex-shrink: 0;
    opacity: 0;
  }
  .item:hover .plus,
  .item:hover .detail-btn {
    opacity: 1;
  }
  .detail-btn:hover {
    color: var(--accent);
  }
  .del {
    border: none;
    background: transparent;
    color: var(--fg-dim);
    font-size: 12px;
    cursor: pointer;
    padding: 0 2px;
    flex-shrink: 0;
    opacity: 0;
  }
  .item:hover .del {
    opacity: 1;
  }
  .del:hover {
    color: var(--danger);
  }
  .child-add {
    display: flex;
    gap: 4px;
    align-items: center;
    padding-bottom: 2px;
  }
  .child-add input {
    flex: 1;
    border: 1px dashed var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 11px;
    padding: 2px 7px;
    outline: none;
    min-width: 0;
  }
  .child-add button {
    border: none;
    background: var(--accent);
    color: #fff;
    border-radius: 6px;
    font-size: 11px;
    cursor: pointer;
    padding: 2px 8px;
  }
  .empty {
    font-size: 10px;
    color: var(--fg-dim);
    text-align: center;
    padding: 8px 0;
  }
  /* 详情弹层 */
  .detail-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 110;
  }
  .detail {
    width: 280px;
    max-height: 82vh;
    overflow-y: auto;
    scrollbar-width: none;
    background: var(--bg-elev);
    border-radius: 14px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.4);
  }
  .detail::-webkit-scrollbar {
    display: none;
  }
  .d-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-weight: 600;
    font-size: 13px;
  }
  .d-close {
    border: none;
    background: transparent;
    color: var(--fg-dim);
    font-size: 16px;
    cursor: pointer;
  }
  .d-text {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 13px;
    padding: 7px 9px;
    outline: none;
  }
  .d-text:focus {
    border-color: var(--accent);
  }
  .d-row {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .d-label {
    font-size: 10px;
    color: var(--fg-dim);
  }
  .prio-btns {
    display: flex;
    gap: 5px;
  }
  .pbtn {
    flex: 1;
    border: 1px solid var(--border);
    background: transparent;
    border-radius: 7px;
    font-size: 11px;
    padding: 4px 0;
    cursor: pointer;
  }
  .pbtn.on {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 15%, transparent);
  }
  .d-cal {
    display: flex;
    justify-content: center;
  }
  .remind-btns {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .rbtn {
    flex: 1;
    min-width: 58px;
    border: 1px solid var(--border);
    background: transparent;
    border-radius: 7px;
    font-size: 10px;
    padding: 4px 0;
    cursor: pointer;
    color: var(--fg-dim);
  }
  .rbtn.on {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 15%, transparent);
    color: var(--accent);
    font-weight: 600;
  }
  .d-foot {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }
  .d-del {
    border: none;
    background: transparent;
    color: var(--danger);
    font-size: 11px;
    cursor: pointer;
    padding: 4px 6px;
    border-radius: 6px;
  }
  .d-del:hover {
    background: color-mix(in srgb, var(--danger) 15%, transparent);
  }
  .d-ok {
    border: none;
    background: var(--accent);
    color: #fff;
    border-radius: 8px;
    font-size: 12px;
    padding: 6px 18px;
    cursor: pointer;
  }
</style>
