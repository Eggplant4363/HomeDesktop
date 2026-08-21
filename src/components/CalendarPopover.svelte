<script lang="ts">
  // 可视化日期 + 时间选择器（月历网格 + 快捷选项 + 时间）
  let {
    value = "",
    time = "",
    onchange,
  }: {
    /** 已选日期 YYYY-MM-DD */
    value?: string;
    /** 已选时间 HH:mm */
    time?: string;
    onchange?: (date: string, time: string) => void;
  } = $props();

  const pad = (n: number) => String(n).padStart(2, "0");
  const weekdays = ["日", "一", "二", "三", "四", "五", "六"];

  function todayStr(): string {
    const d = new Date();
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  }
  function nowTime(): string {
    const d = new Date();
    return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }

  // svelte-ignore state_referenced_locally: 组件每次打开重新挂载，props 仅作初始值
  const initial = value || todayStr();
  const [iy, im] = initial.split("-").map(Number);
  let viewYear = $state(iy);
  let viewMonth = $state(im - 1);
  // svelte-ignore state_referenced_locally: 同上，仅取初始值
  let selDate = $state(initial);
  // svelte-ignore state_referenced_locally
  let selTime = $state(time || nowTime());

  function emit(): void {
    onchange?.(selDate, selTime);
  }

  const today = $derived(todayStr());

  /** 月历格子：null=空白，否则为日期串 */
  const cells = $derived.by(() => {
    const first = new Date(viewYear, viewMonth, 1).getDay();
    const days = new Date(viewYear, viewMonth + 1, 0).getDate();
    const out: (string | null)[] = [];
    for (let i = 0; i < first; i++) out.push(null);
    for (let d = 1; d <= days; d++) out.push(`${viewYear}-${pad(viewMonth + 1)}-${pad(d)}`);
    return out;
  });

  function pick(d: string): void {
    selDate = d;
    emit();
  }

  function setView(y: number, m: number): void {
    viewYear = y;
    viewMonth = ((m % 12) + 12) % 12;
  }

  function quick(daysFromToday: number | null): void {
    if (daysFromToday === null) {
      selDate = "";
      selTime = "";
      emit();
      return;
    }
    const d = new Date();
    d.setDate(d.getDate() + daysFromToday);
    selDate = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
    selTime = nowTime();
    emit();
  }

  function daysBetween(a: string, b: string): number {
    const [y1, m1, d1] = a.split("-").map(Number);
    const [y2, m2, d2] = b.split("-").map(Number);
    return Math.round(
      (new Date(y2, m2 - 1, d2).getTime() - new Date(y1, m1 - 1, d1).getTime()) / 86400000,
    );
  }
</script>

<div class="cal">
  <div class="head">
    <button class="nav" onclick={() => setView(viewYear, viewMonth - 1)}>‹</button>
    <span class="month">{viewYear}年{viewMonth + 1}月</span>
    <button class="nav" onclick={() => setView(viewYear, viewMonth + 1)}>›</button>
  </div>
  <div class="grid">
    {#each weekdays as w (w)}
      <span class="wd">{w}</span>
    {/each}
    {#each cells as d, i (i)}
      {#if d === null}
        <span class="blank"></span>
      {:else}
        <button
          class="day"
          class:sel={d === selDate}
          class:today={d === today}
          onclick={() => pick(d)}
        >{Number(d.slice(8))}</button>
      {/if}
    {/each}
  </div>
  <div class="quick">
    <button class="q" onclick={() => quick(0)}>今天</button>
    <button class="q" onclick={() => quick(1)}>明天</button>
    <button class="q" onclick={() => quick(7)}>下周</button>
    {#if selDate}
      <button class="q" title="距今 {daysBetween(today, selDate)} 天" onclick={() => quick(null)}>清除</button>
    {/if}
  </div>
  {#if selDate}
    <div class="time-row">
      <span class="t-label">时间</span>
      <input
        class="t-input"
        type="time"
        value={selTime}
        onchange={(e) => {
          selTime = (e.target as HTMLInputElement).value || selTime;
          emit();
        }}
      />
      {#if selDate === today}
        <span class="t-tip">今天 {selTime}</span>
      {:else}
        <span class="t-tip">{selDate} {selTime}</span>
      {/if}
    </div>
  {/if}
</div>

<style>
  .cal {
    width: 232px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 10px;
    box-shadow: 0 8px 28px rgba(0, 0, 0, 0.35);
    font-size: 12px;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 6px;
  }
  .nav {
    border: none;
    background: transparent;
    color: var(--fg);
    font-size: 15px;
    cursor: pointer;
    width: 24px;
    height: 24px;
    border-radius: 6px;
  }
  .nav:hover {
    background: var(--bg-hover);
  }
  .month {
    font-weight: 600;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 2px;
    margin-bottom: 8px;
  }
  .wd {
    text-align: center;
    font-size: 10px;
    color: var(--fg-dim);
    padding: 2px 0;
  }
  .day,
  .blank {
    height: 26px;
    border-radius: 7px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
  }
  .day {
    border: none;
    background: transparent;
    color: var(--fg);
    cursor: pointer;
  }
  .day:hover {
    background: var(--bg-hover);
  }
  .day.today {
    color: var(--accent);
    font-weight: 700;
  }
  .day.sel {
    background: var(--accent);
    color: #fff;
    font-weight: 700;
  }
  .quick {
    display: flex;
    gap: 4px;
    margin-bottom: 8px;
  }
  .q {
    flex: 1;
    border: 1px solid var(--border);
    background: transparent;
    color: var(--fg-dim);
    font-size: 10px;
    border-radius: 6px;
    padding: 3px 0;
    cursor: pointer;
  }
  .q:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .time-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .t-label {
    font-size: 10px;
    color: var(--fg-dim);
  }
  .t-input {
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 11px;
    padding: 2px 5px;
    outline: none;
    width: 76px;
  }
  .t-tip {
    font-size: 10px;
    color: var(--fg-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
