<script lang="ts">
  // 纪念日/倒数日：添加多个纪念日（可年度重复 or 一次性），显示距今 X 天。
  import { onMount } from "svelte";
  import { getConfig, setConfig } from "../core/config";

  let { cellId }: { cellId?: string } = $props();
  const KEY = `anni.${cellId ?? "default"}.items`;

  interface Occ {
    id: string;
    name: string;
    date: string; // "YYYY-MM-DD"
    monthly: boolean; // 是否每年重复
  }

  let items = $state<Occ[]>([]);
  let name = $state("");
  let date = $state("");
  let monthly = $state(true);
  let editing = $state(false);
  let nid = 0;

  async function load(): Promise<void> {
    const raw = await getConfig(KEY);
    items = Array.isArray(raw) ? (raw as Occ[]) : [];
    if (!date) date = new Date().toISOString().slice(0, 10);
  }
  function persist(): void { void setConfig(KEY, items); }
  function today(): number {
    const n = new Date();
    return new Date(n.getFullYear(), n.getMonth(), n.getDate()).getTime();
  }
  function days(occ: Occ): { d: number; past: boolean } {
    const now = new Date();
    const t = today();
    if (occ.monthly) {
      // 下一个该日期（月/日）
      const [y, m, d] = occ.date.split("-").map(Number);
      let dt = new Date(now.getFullYear(), m - 1, d);
      if (dt.getTime() < t) dt = new Date(now.getFullYear() + 1, m - 1, d);
      const diff = Math.round((dt.getTime() - t) / 86400000);
      return { d: diff, past: t > new Date(now.getFullYear(), m - 1, d).getTime() };
    }
    const dt = new Date(occ.date + "T00:00:00").getTime();
    const diff = Math.round((dt - t) / 86400000);
    return { d: Math.abs(diff), past: dt < t };
  }
  function add(): void {
    if (!name.trim() || !date) return;
    items = [{ id: `${Date.now()}-${nid++}`, name: name.trim(), date, monthly }, ...items];
    name = "";
    persist();
    editing = false;
  }
  function del(id: string): void {
    items = items.filter((o) => o.id !== id);
    persist();
  }
  onMount(() => { void load(); pers2(); });

  function pers2() {
    // 每 6h 归零日误差本就用日期级别，无需高刷。
  }
</script>

<div class="an">
  <div class="head">
    <span class="title">🎂 纪念日</span>
    <button class="add" onclick={() => (editing = !editing)}>{editing ? "收起" : "＋ 添加"}</button>
  </div>

  {#if editing}
    <div class="form">
      <input class="in" placeholder="名称：恋爱/生日/纪念…" bind:value={name} />
      <input class="in d" type="date" bind:value={date} />
      <label class="chk"><input type="checkbox" bind:checked={monthly} /> 每年重复</label>
      <button class="save" onclick={add} disabled={!name.trim() || !date}>添加</button>
    </div>
  {/if}

  <div class="list">
    {#each [...items].sort((a,b) => { const da=days(a), db=days(b); return (da.past?1:0)-(db.past?1:0) || da.d-db.d; }) as occ (occ.id)}
      {@const dd = days(occ)}
      <div class="row">
        <div class="nm">
          <div class="name">{occ.name}{occ.monthly ? "" : "（一次性）"}</div>
          {#if dd.past}<div class="lbl past">已过 {dd.d} 天</div>{:else}<div class="lbl">还有 {dd.d} 天</div>{/if}
        </div>
        <button class="x" onclick={() => del(occ.id)}>×</button>
      </div>
    {/each}
    {#if items.length === 0}<div class="empty">点“＋ 添加”记一个纪念日</div>{/if}
  </div>
</div>

<style>
  .an { height: 100%; display: flex; flex-direction: column; gap: 6px; overflow: hidden; }
  .head { display: flex; align-items: center; gap: 8px; }
  .title { font-size: 13px; font-weight: 600; flex: 1; }
  .add { border: none; border-radius: 6px; background: var(--bg-input); color: var(--fg); cursor: pointer; padding: 3px 8px; font-size: 12px; }
  .form { display: flex; flex-direction: column; gap: 4px; padding: 6px; border: 1px solid var(--border); border-radius: 8px; }
  .in { border: 1px solid var(--border); border-radius: 6px; background: var(--bg-input); color: var(--fg); padding: 4px 6px; font-size: 12px; width: auto; }
  .d { width: fit-content; }
  .chk { font-size: 11px; color: var(--fg-dim); display: flex; gap: 4px; align-items: center; }
  .save { border: none; border-radius: 6px; background: var(--accent); color: #fff; padding: 4px 8px; cursor: pointer; font-size: 12px; align-self: flex-start; }
  .list { flex: 1; min-height: 0; overflow-y: auto; display: flex; flex-direction: column; gap: 4px; }
  .row { display: flex; align-items: center; gap: 6px; padding: 6px 8px; border: 1px solid var(--border); border-radius: 8px; background: var(--bg-input); }
  .nm { flex: 1; }
  .name { font-size: 13px; }
  .lbl { font-size: 11px; color: var(--accent); }
  .lbl.past { color: var(--fg-dim); }
  .x { border: none; background: transparent; color: var(--fg-dim); cursor: pointer; font-size: 14px; }
  .empty { font-size: 11px; color: var(--fg-dim); text-align: center; margin: auto; }
</style>
