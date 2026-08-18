<script lang="ts">
  // 实时时钟小组件：显示秒/日期按"图标实例"独立设置（未设置回退插件级默认）
  import { onMount } from "svelte";
  import { getCellSetting, peekCellSetting } from "../core/pluginSettings.svelte";

  let { cellId }: { cellId?: string } = $props();

  const PLUGIN_ID = "dev.homedesktop.clock";
  const instanceId = $derived(cellId ?? PLUGIN_ID);

  let now = $state(new Date());
  let timer: ReturnType<typeof setInterval> | undefined;

  $effect(() => {
    timer = setInterval(() => {
      now = new Date();
    }, 1000);
    return () => clearInterval(timer);
  });

  const hh = $derived(String(now.getHours()).padStart(2, "0"));
  const mm = $derived(String(now.getMinutes()).padStart(2, "0"));
  const ss = $derived(String(now.getSeconds()).padStart(2, "0"));

  const showSeconds = $derived(
    peekCellSetting<boolean>(instanceId, PLUGIN_ID, "showSeconds") ?? true,
  );
  const showDate = $derived(peekCellSetting<boolean>(instanceId, PLUGIN_ID, "showDate") ?? true);

  const weekdays = ["日", "一", "二", "三", "四", "五", "六"];
  const dateText = $derived(
    `${now.getFullYear()}年${now.getMonth() + 1}月${now.getDate()}日 星期${weekdays[now.getDay()]}`,
  );

  onMount(async () => {
    // 加载实例设置（填充缓存，模板中的 $derived 会自动响应）
    await getCellSetting(instanceId, PLUGIN_ID, "showSeconds", true);
    await getCellSetting(instanceId, PLUGIN_ID, "showDate", true);
  });
</script>

<div class="clock">
  <div class="time">
    <span class="hhmm">{hh}:{mm}</span>
    {#if showSeconds}
      <span class="sec">{ss}</span>
    {/if}
  </div>
  {#if showDate}
    <div class="date">{dateText}</div>
  {/if}
</div>

<style>
  .clock {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
  }
  .time {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .hhmm {
    font-size: 42px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    letter-spacing: 1px;
  }
  .sec {
    font-size: 18px;
    color: var(--fg-dim);
    font-variant-numeric: tabular-nums;
  }
  .date {
    font-size: 13px;
    color: var(--fg-dim);
  }
</style>
