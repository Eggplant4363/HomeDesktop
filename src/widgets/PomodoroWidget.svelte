<script lang="ts">
  // 番茄钟 / 倒计时 / 秒表 三合一增强（纯前端）
  // 标签：🍅专注 ·☕休息(番茄) | ⏳倒计时 | ⏱秒表
  import { onMount } from "svelte";
  import { getPluginSetting, peekPluginSetting } from "../core/pluginSettings.svelte";

  let { }: { cellId?: string } = $props();
  const pid = "dev.homedesktop.pomodoro";

  type Mode = "pomo" | "count" | "stop";
  type Phase = "focus" | "break";

  let mode: Mode = $state("pomo");
  let phase: Phase = $state("focus");
  let workMin = $state(25);
  let breakMin = $state(5);
  // 会话
  let count = $state(0); // 已完专注个数
  let running = $state(false);
  let remaining = $state(25 * 60); // 秒
  let total = $state(25 * 60);
  // 倒计时字段
  let cdMin = $state("1");
  let cdSec = $state("00");
  // 秒表
  let swMs = $state(0);
  let lapTick = 0;
  let timer: ReturnType<typeof setInterval> | undefined;

  function mmss(s: number): string {
    s = Math.max(0, Math.round(s));
    const m = Math.floor(s / 60), ss = s % 60;
    return `${String(m).padStart(2, "0")}:${String(ss).padStart(2, "0")}`;
  }

  async function load(): Promise<void> {
    workMin = (await getPluginSetting<number>(pid, "workMin", workMin)) ?? 25;
    breakMin = (await getPluginSetting<number>(pid, "breakMin", breakMin)) ?? 5;
    applyPhase("focus");
  }

  function applyPhase(p: Phase): void {
    phase = p;
    total = p === "focus" ? workMin * 60 : breakMin * 60;
    remaining = total;
    running = false;
  }

  async function notifyDone(): Promise<void> {
    // 到时提醒（如需要此 app 已有系统通知，可用 tauri 通知，此处先 UI 闪一下即可）
  }

  function finishPomoPhase(): void {
    if (phase === "focus") {
      count += 1;
      void notifyDone();
      applyPhase("break");
    } else {
      void notifyDone();
      applyPhase("focus");
    }
  }

  function tick(): void {
    if (mode === "pomo") {
      remaining -= 1;
      if (remaining <= 0) finishPomoPhase();
    } else if (mode === "count") {
      remaining -= 1;
      running = remaining > 0;
    } else {
      swMs += 10;
    }
  }

  function toggle(): void {
    if (mode === "pomo") {
      running = !running;
    } else if (mode === "count") {
      if (!running) {
        const m = parseInt(cdMin || "0", 10);
        const s = parseInt(cdSec || "0", 10);
        total = m * 60 + s;
        remaining = total > 0 ? total : 0;
      }
      running = !running && remaining > 0;
    } else {
      running = !running;
    }
  }

  function reset(): void {
    if (mode === "pomo") {
      void load();
    } else if (mode === "count") {
      remaining = 0; running = false;
    } else {
      swMs = 0; running = false;
    }
  }

  onMount(() => {
    void load();
    timer = setInterval(() => {
      if (running) tick();
    }, mode === "stop" ? 10 : 1000);
    return () => clearInterval(timer);
  });
</script>

<div class="pv">
  <div class="modes">
    <button class:on={mode === "pomo"} onclick={() => { mode = "pomo"; if (!running) void load(); }}>🍅 番茄</button>
    <button class:on={mode === "count"} onclick={() => { mode = "count"; running = false; }}>⏳ 倒计时</button>
    <button class:on={mode === "stop"} onclick={() => { mode = "stop"; swMs = 0; running = false; }}>⏱ 秒表</button>
  </div>

  {#if mode === "pomo"}
    <div class="center">
      <div class="phase" class:on={phase === "focus"}>{phase === "focus" ? "专注中" : "休息中"}</div>
      <div class="clock">{mmss(remaining)}</div>
      <div class="meta">{phase === "focus" ? "专注" : "休息"} {phase === "focus" ? workMin : breakMin} 分钟 · 已完 {count} 轮</div>
    </div>
  {:else if mode === "count"}
    <div class="center">
      <div class="clock tog" onclick={() => void toggle()}>{mmss(running ? remaining : total)}</div>
      {#if !running}
        <div class="row2">
          <input class="num" type="number" bind:value={cdMin} min="0" max="999" />
          <span>分</span>
          <input class="num" type="number" bind:value={cdSec} min="0" max="59" />
          <span>秒</span>
        </div>
      {/if}
    </div>
  {:else}
    <div class="center">
      <div class="clock">{new Date(swMs).toISOString().slice(14, 19)}.</div>
    </div>
  {/if}

  <div class="ctl">
    {#if true}
      <button class="primary" onclick={() => void toggle()}>{running ? "⏸ 暂停" : "▶ 开始"}</button>
    {/if}
    <button onclick={reset}>↺ 重置</button>
  </div>

  {#if mode === "pomo" && phase === "focus"}
    <div class="tip">专注 {workMin} 分 / 休息 {breakMin} 分</div>
  {/if}
</div>

<style>
  .pv {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
    overflow: hidden;
  }
  .modes {
    display: flex;
    gap: 6px;
  }
  .modes button {
    flex: 1;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: transparent;
    color: var(--fg-dim);
    cursor: pointer;
    padding: 4px;
    font-size: 12px;
  }
  .modes button.on {
    border-color: var(--accent);
    color: var(--accent);
  }
  .center {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
  }
  .phase {
    font-size: 12px;
    color: var(--fg-dim);
  }
  .phase.on {
    color: var(--accent);
  }
  .clock {
    font-size: 40px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    line-height: 1;
  }
  .clock.tog {
    cursor: pointer;
  }
  .meta,
  .tip {
    font-size: 11px;
    color: var(--fg-dim);
  }
  .row2 {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
  }
  .num {
    width: 46px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--fg);
    padding: 2px 4px;
    text-align: center;
  }
  .ctl {
    display: flex;
    gap: 8px;
    justify-content: center;
  }
  .ctl button {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: transparent;
    color: var(--fg);
    cursor: pointer;
    padding: 3px 10px;
  }
  .ctl .primary {
    background: var(--accent);
    color: #fff;
    border-color: var(--accent);
  }
</style>
