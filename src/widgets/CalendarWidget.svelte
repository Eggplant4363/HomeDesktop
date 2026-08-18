<script lang="ts">
  // 日历小组件（M12）：月历视图，可翻月，今天高亮
  import { getMonthGrid } from "../core/calendar";

  let { cellId }: { cellId?: string } = $props();

  const now = new Date();
  let viewYear = $state(now.getFullYear());
  let viewMonth = $state(now.getMonth() + 1);

  const weeks = $derived(getMonthGrid(viewYear, viewMonth));
  const weekdays = ["日", "一", "二", "三", "四", "五", "六"];

  function isToday(d: number): boolean {
    return d === now.getDate() && viewYear === now.getFullYear() && viewMonth === now.getMonth() + 1;
  }

  function prev(): void {
    if (viewMonth === 1) {
      viewMonth = 12;
      viewYear--;
    } else {
      viewMonth--;
    }
  }

  function next(): void {
    if (viewMonth === 12) {
      viewMonth = 1;
      viewYear++;
    } else {
      viewMonth++;
    }
  }
</script>

<div class="calendar">
  <div class="head">
    <button class="nav" title="上个月" onclick={prev}>‹</button>
    <span class="title">{viewYear}年{viewMonth}月</span>
    <button class="nav" title="下个月" onclick={next}>›</button>
  </div>
  <div class="weekdays">
    {#each weekdays as w (w)}
      <span>{w}</span>
    {/each}
  </div>
  {#each weeks as week, wi (wi)}
    <div class="row">
      {#each week as d, i (i)}
        <span class="day" class:today={d !== null && isToday(d)}>{d ?? ""}</span>
      {/each}
    </div>
  {/each}
</div>

<style>
  .calendar {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .title {
    font-size: 13px;
    font-weight: 600;
  }
  .nav {
    border: none;
    background: transparent;
    color: var(--fg-dim);
    font-size: 14px;
    cursor: pointer;
    padding: 0 6px;
  }
  .nav:hover {
    color: var(--fg);
  }
  .weekdays,
  .row {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    text-align: center;
  }
  .weekdays span {
    font-size: 10px;
    color: var(--fg-dim);
  }
  .day {
    font-size: 11px;
    padding: 2px 0;
    border-radius: 6px;
  }
  .day.today {
    background: var(--accent);
    color: #fff;
    font-weight: 700;
  }
</style>
