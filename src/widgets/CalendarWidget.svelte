<script lang="ts">
  // 日历小组件：月历视图（可翻月、今天高亮）+ 农历日期/节日
  import { getMonthGrid } from "../core/calendar";
  import { solarToLunar, lunarFestival, isLunarNewYearsEve } from "../core/lunar";

  let { cellId }: { cellId?: string } = $props();

  const now = new Date();
  let viewYear = $state(now.getFullYear());
  let viewMonth = $state(now.getMonth() + 1);

  const weeks = $derived(getMonthGrid(viewYear, viewMonth));
  const weekdays = ["日", "一", "二", "三", "四", "五", "六"];

  function isToday(d: number): boolean {
    return d === now.getDate() && viewYear === now.getFullYear() && viewMonth === now.getMonth() + 1;
  }

  function lunarOf(d: number) {
    return solarToLunar(new Date(viewYear, viewMonth - 1, d));
  }

  /** 格子里显示的农历标签：除夕/节日 > 初一显示月名 > 日名 */
  function lunarLabel(d: number): string {
    const l = lunarOf(d);
    if (isLunarNewYearsEve(l.year, l.month, l.day)) return "除夕";
    const fest = lunarFestival(l.month, l.day);
    if (fest) return fest;
    if (l.day === 1) return l.monthName.replace("月", "");
    return l.dayName;
  }

  /** 当前视图月的农历（干支）年月 */
  const headLunar = $derived(solarToLunar(new Date(viewYear, viewMonth - 1, 1)));

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
    <div class="titles">
      <div class="title">{viewYear}年{viewMonth}月</div>
      <div class="lunar-title">{headLunar.yearName}{headLunar.monthName}</div>
    </div>
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
        {#if d === null}
          <span class="day"></span>
        {:else}
          <span
            class="day"
            class:today={isToday(d)}
            class:fest={["春节", "中秋", "除夕"].includes(lunarLabel(d))}
          >
            <span class="solar">{d}</span>
            <span class="lunar">{lunarLabel(d)}</span>
          </span>
        {/if}
      {/each}
    </div>
  {/each}
</div>

<style>
  .calendar {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .titles {
    display: flex;
    flex-direction: column;
    align-items: center;
    line-height: 1.2;
  }
  .title {
    font-size: 13px;
    font-weight: 600;
  }
  .lunar-title {
    font-size: 9px;
    color: var(--accent);
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
    font-size: 9px;
    color: var(--fg-dim);
  }
  .day {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
    padding: 1px 0;
    border-radius: 6px;
    font-size: 11px;
  }
  .day .lunar {
    font-size: 8px;
    color: var(--fg-dim);
    line-height: 1;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .day.today {
    background: var(--accent);
  }
  .day.today .solar {
    color: #fff;
    font-weight: 700;
  }
  .day.today .lunar {
    color: rgba(255, 255, 255, 0.85);
  }
  .day.fest .lunar {
    color: var(--danger);
  }
  .day.today.fest .lunar {
    color: #fff;
  }
</style>
