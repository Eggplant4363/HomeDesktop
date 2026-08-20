<script lang="ts">
  // 歌词面板：LRCLIB 同步歌词，随播放进度逐行高亮并自动居中滚动
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { fetchLyrics, lineIndexAt, parseLrc } from "../core/lyrics";
  import type { LyricLine, LyricsResult } from "../core/lyrics";
  import { lyrics } from "../core/lyricsState.svelte";
  import { log } from "../core/logger";

  let { onclose }: { onclose?: () => void } = $props();

  let result = $state<LyricsResult | null>(null);
  let loading = $state(true);
  let position = $state(0);
  let current = $state(0); // 当前歌词行下标
  let scrollEl = $state<HTMLElement | undefined>();
  let linesEl = $state<HTMLElement | undefined>();
  let pollTimer: ReturnType<typeof setInterval> | undefined;
  let thumbnail = $state<string | null>(null);

  // 曲目变化 → 重新获取歌词
  let trackKey = "";

  async function loadLyrics(): Promise<void> {
    const t = lyrics.track;
    const key = `${t.title}|${t.artist}|${Math.round(t.duration)}`;
    if (key === trackKey && result !== null) return;
    trackKey = key;
    loading = true;
    result = null;
    const r = await fetchLyrics(t);
    result = r;
    loading = false;
    if (r && r.synced.length > 0) {
      current = lineIndexAt(r.synced, position);
      log.info(`歌词获取成功: ${t.title}（${r.synced.length} 行）`);
    } else if (r?.plain) {
      log.info(`歌词获取成功(纯文本): ${t.title}`);
    } else {
      log.info(`歌词未找到: ${t.title}`);
    }
  }

  async function pollPosition(): Promise<void> {
    try {
      const data = await invoke<{
        title: string;
        artist: string;
        state: string;
        position: number;
        duration: number;
        thumbnail: string | null;
      }>("media_now_playing");
      // 歌曲变化：更新曲目信息并重新获取歌词
      const t = lyrics.track;
      if (data.title && data.title !== t.title) {
        lyrics.track = {
          title: data.title,
          artist: data.artist,
          album: t.album,
          duration: data.duration || t.duration,
        };
        result = null;
        void loadLyrics();
      }
      position = data.position ?? 0;
      if (data.thumbnail) thumbnail = data.thumbnail;
      if (result && result.synced.length > 0) {
        const idx = lineIndexAt(result.synced, position);
        if (idx !== current) current = idx;
      }
    } catch {
      // 轮询失败忽略（保留上次数据）
    }
  }

  onMount(() => {
    void loadLyrics();
    void pollPosition();
    pollTimer = setInterval(() => void pollPosition(), 1000);
    return () => {
      if (pollTimer) clearInterval(pollTimer);
    };
  });

  // 当前行自动居中滚动（使用 effect 避免渲染后立即滚动闪跳）
  let scrolledKey = "";
  $effect(() => {
    void current;
    if (!linesEl || !scrollEl || current < 0) return;
    const key = `${current}`;
    if (key === scrolledKey) return;
    scrolledKey = key;
    const line = linesEl.children[current] as HTMLElement | undefined;
    if (!line) return;
    const target = line.offsetTop - scrollEl.clientHeight / 2 + line.offsetHeight / 2;
    scrollEl.scrollTo({ top: Math.max(0, target), behavior: "smooth" });
  });

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === "Escape") onclose?.();
  }

  const lineCount = $derived(result?.synced.length ?? 0);
</script>

<svelte:window onkeydown={onKeydown} />

<div
  class="overlay"
  role="button"
  tabindex="-1"
  onclick={(e) => e.target === e.currentTarget && onclose?.()}
  onkeydown={(e) => e.key === "Escape" && onclose?.()}
>
  <div class="panel">
    <div class="head">
      <div class="meta">
        {#if thumbnail}
          <img class="cover" src={thumbnail} alt="封面" draggable="false" />
        {/if}
        <div class="titles">
          <div class="title">{lyrics.track.title || "未知曲目"}</div>
          <div class="artist">{lyrics.track.artist || "—"}</div>
        </div>
      </div>
      <button class="close" onclick={onclose} title="关闭 (Esc)">×</button>
    </div>

    <div class="body">
      {#if loading}
        <div class="hint">正在获取歌词…</div>
      {:else if result?.instrumental}
        <div class="hint">🎵 纯音乐，无歌词</div>
      {:else if result && lineCount > 0}
        <div class="lyrics" bind:this={scrollEl}>
          <div class="lines" bind:this={linesEl}>
            {#each result.synced as line, i (i)}
              <div
                class="line"
                class:active={i === current}
                class:passed={i < current}
              >{line.text || "♪"}</div>
            {/each}
          </div>
        </div>
      {:else if result?.plain}
        <div class="lyrics plain">
          {#each result.plain.split(/\r?\n/) as t (t)}
            <div class="line">{t || " "}</div>
          {/each}
        </div>
      {:else}
        <div class="hint">
          <div>未找到歌词</div>
          <div class="sub">可在 lrclib.net 提交缺失的歌词后重试</div>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 120;
    backdrop-filter: blur(4px);
  }
  .panel {
    width: min(560px, 92vw);
    height: min(640px, 88vh);
    background: var(--bg-elev);
    border-radius: 18px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 12px 48px rgba(0, 0, 0, 0.45);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 14px 18px;
    border-bottom: 1px solid var(--border);
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  .cover {
    width: 48px;
    height: 48px;
    border-radius: 10px;
    object-fit: cover;
    flex-shrink: 0;
  }
  .titles {
    min-width: 0;
  }
  .title {
    font-size: 16px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .artist {
    font-size: 12px;
    color: var(--fg-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .close {
    border: none;
    background: transparent;
    color: var(--fg-dim);
    font-size: 22px;
    cursor: pointer;
    flex-shrink: 0;
  }
  .close:hover {
    color: var(--fg);
  }
  .body {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .lyrics {
    flex: 1;
    overflow-y: auto;
    scrollbar-width: none;
    -ms-overflow-style: none;
    padding: 24px 20px;
  }
  .lyrics::-webkit-scrollbar {
    display: none;
  }
  .lines {
    display: flex;
    flex-direction: column;
  }
  .line {
    padding: 10px 4px;
    font-size: 15px;
    color: var(--fg-dim);
    text-align: center;
    transition:
      color 0.25s,
      font-size 0.25s,
      opacity 0.25s;
    line-height: 1.6;
  }
  .line.active {
    color: var(--accent);
    font-size: 18px;
    font-weight: 700;
  }
  .line.passed {
    opacity: 0.45;
  }
  .hint {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    color: var(--fg-dim);
    font-size: 14px;
  }
  .hint .sub {
    font-size: 11px;
    opacity: 0.7;
  }
</style>
