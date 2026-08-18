<script lang="ts">
  // 快捷键录入行（M7）：显示当前组合键，点击后进入"按下组合键"录制态
  import { onMount } from "svelte";
  import { comboFromEvent, comboLabel, normalizeCombo } from "../core/shortcuts";

  let {
    label,
    value,
    onchange,
  }: {
    label: string;
    value: string;
    /** 保存组合键；返回错误信息字符串（null = 成功） */
    onchange?: (combo: string) => Promise<string | null>;
  } = $props();

  let recording = $state(false);
  let error = $state("");
  /** 保存中暂显的新值（成功前显示） */
  let pending = $state<string | null>(null);

  onMount(() => {
    // 捕获阶段监听：录制中先于设置面板的 Esc/关闭逻辑处理，避免按键被面板拦截
    const handler = (e: KeyboardEvent) => onKeydown(e);
    window.addEventListener("keydown", handler, true);
    return () => window.removeEventListener("keydown", handler, true);
  });

  function start(): void {
    recording = true;
    error = "";
    pending = null;
  }

  function cancel(): void {
    recording = false;
    pending = null;
  }

  async function commit(combo: string): Promise<void> {
    recording = false;
    pending = combo;
    const err = await onchange?.(combo);
    if (err) {
      error = err;
      pending = null;
    }
  }

  function onKeydown(e: KeyboardEvent): void {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      cancel();
      return;
    }
    const combo = comboFromEvent(e);
    if (!combo) return;
    if (combo === normalizeCombo(value)) {
      cancel();
      return;
    }
    void commit(combo);
  }
</script>

<div class="sc-row">
  <span class="sc-label">{label}</span>
  <button
    class="sc-btn"
    class:recording={recording}
    title="点击后按下组合键"
    onclick={start}
  >
    {recording ? "按下组合键…（Esc 取消）" : comboLabel(pending ?? value)}
  </button>
  {#if error}
    <span class="sc-error">{error}</span>
  {/if}
</div>

<style>
  .sc-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .sc-label {
    flex: 1;
    font-size: 13px;
    color: var(--fg);
  }
  .sc-btn {
    min-width: 130px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 12px;
    padding: 6px 10px;
    cursor: pointer;
  }
  .sc-btn:hover {
    border-color: var(--accent);
  }
  .sc-btn.recording {
    border-color: var(--accent);
    color: var(--accent);
  }
  .sc-error {
    font-size: 11px;
    color: var(--danger);
  }
</style>
