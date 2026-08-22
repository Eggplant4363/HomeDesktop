<script lang="ts">
  // 可视化日期 + 时间选择器（月历网格 + 快捷选项 + 预设时间胶囊 + 自定义时间）
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
  const TIME_PRESETS = ["09:00", "12:00", "14:00", "18:00", "20:00", "22:00"];

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
  // svelte-ignore state_referenced_locally
  let selDate = $state(initial);
  // svelte-ignore state_referenced_locally
  let selTime = $state(time || nowTime());
  let customOpen = $state(false);

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
    <button class="nav" title="上个月" onclick={() => setView(viewYear, viewMonth - 1)}>‹</button>
    <span class="month">{viewYear}年{viewMonth + 1}月</span>
    <button class="nav" title="下个月" onclick={() => setView(viewYear, viewMonth + 1)}>›</button>
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
    <button class="q" class:on={selDate === today} onclick={() => quick(0)}>今天</button>
    <button class="q" onclick={() => quick(1)}>明天</button>
    <button class="q" onclick={() => quick(7)}>下周</button>
    {#if selDate}
      <button class="q danger" title="距今 {daysBetween(today, selDate)} 天" onclick={() => quick(null)}>清除</button>
    {/if}
  </div>
  {#if selDate}
    <div class="divider"></div>
    <div class="time-head">
      <span class="t-label">时间</span>
      {#if selDate === today}
        <span class="t-tip">今天 {selTime}</span>
      {:else}
        <span class="t-tip">{selDate} {selTime}</span>
      {/if}
    </div>
    <div class="t-presets">
      {#each TIME_PRESETS as p (p)}
        <button
          class="tp"
          class:on={selTime === p && !customOpen}
          onclick={() => {
            customOpen = false;
            selTime = p;
            emit();
          }}
        >{p}</button>
      {/each}
    </div>
    <div class="t-custom">
      <button class="custom-toggle" class:on={customOpen} onclick={() => (customOpen = !customOpen)}>
        🕐 {customOpen ? "收起自定义" : "自定义时间"}
      </button>
      {#if customOpen}
        <input
          class="t-input"
          type="time"
          value={selTime}
          onchange={(e) => {
            selTime = (e.target as HTMLInputElement).value || selTime;
            emit();
          }}
        />
      {/if}
    </div>
  {/if}
</div>

<style>
  .cal {
    width: 268px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 16px;
    padding: 14px;
    box-shadow: 0 10px 36px rgba(0, 0, 0, 0.4);
    font-size: 12px;
    color: var(--fg);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
  }
  .nav {
    width: 28px;
    height: 28px;
    border: none;
    background: var(--bg-hover);
    color: var(--fg);
    font-size: 15px;
    border-radius: 9px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s;
  }
  .nav:hover {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    color: var(--accent);
  }
  .month {
    font-weight: 700;
    font-size: 13px;
    letter-spacing: 0.5px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 3px;
    margin-bottom: 10px;
  }
  .wd {
    text-align: center;
    font-size: 10px;
    color: var(--fg-dim);
    padding: 2px 0;
    opacity: 0.8;
  }
  .day,
  .blank {
    height: 30px;
    border-radius: 9px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
  }
  .day {
    border: none;
    background: transparent;
    color: var(--fg);
    cursor: pointer;
    transition:
      background 0.12s,
      color 0.12s;
  }
  .day:hover {
    background: var(--bg-hover);
  }
  .day.today {
    color: var(--accent);
    font-weight: 700;
    box-shadow: inset 0 0 0 1.5px color-mix(in srgb, var(--accent) 55%, transparent);
  }
  .day.sel {
    background: var(--accent);
    color: #fff;
    font-weight: 700;
    box-shadow: none;
  }
  .day.sel:hover {
    background: var(--accent);
  }
  .quick {
    display: flex;
    gap: 5px;
  }
  .q {
    flex: 1;
    border: 1px solid var(--border);
    background: transparent;
    color: var(--fg-dim);
    font-size: 11px;
    border-radius: 8px;
    padding: 5px 0;
    cursor: pointer;
    transition: all 0.15s;
  }
  .q:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .q.on {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    color: var(--accent);
    font-weight: 600;
  }
  .q.danger:hover {
    border-color: var(--danger);
    color: var(--danger);
  }
  .divider {
    height: 1px;
    background: var(--border);
    margin: 12px 0 10px;
  }
  .time-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin-bottom: 8px;
  }
  .t-label {
    font-size: 11px;
    color: var(--fg-dim);
    font-weight: 600;
  }
  .t-tip {
    font-size: 11px;
    color: var(--accent);
    font-variant-numeric: tabular-nums;
    font-weight: 600;
  }
  .t-presets {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 5px;
  }
  .tp {
    border: 1px solid var(--border);
    background: transparent;
    color: var(--fg-dim);
    font-size: 11px;
    border-radius: 8px;
    padding: 6px 0;
    cursor: pointer;
    font-variant-numeric: tabular-nums;
    transition: all 0.15s;
  }
  .tp:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .tp.on {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
    font-weight: 600;
  }
  .t-custom {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
  }
  .custom-toggle {
    flex: 1;
    border: 1px dashed var(--border);
    background: transparent;
    color: var(--fg-dim);
    font-size: 10px;
    border-radius: 8px;
    padding: 5px 0;
    cursor: pointer;
    transition: all 0.15s;
  }
  .custom-toggle:hover,
  .custom-toggle.on {
    border-color: var(--accent);
    color: var(--accent);
  }
  .t-input {
    color-scheme: inherit;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 12px;
    padding: 4px 8px;
    outline: none;
    width: 96px;
  }
  .t-input:focus {
    border-color: var(--accent);
  }
</style>
