<script lang="ts">
  // 音乐控制小组件：读取 Windows 系统媒体会话（SMTC）当前播放的音乐
  // 显示封面 / 标题 / 歌手，提供 上一曲 / 播放暂停 / 下一曲 控制（仅 Windows 生效）
  import { invoke } from "@tauri-apps/api/core";
  import { log } from "../core/logger";

  let { cellId }: { cellId?: string } = $props();

  interface MediaInfo {
    title: string;
    artist: string;
    album: string;
    app: string;
    state: string; // playing | paused | stopped | closed | changing
    thumbnail: string | null;
  }

  let info = $state<MediaInfo | null>(null);
  let error = $state<string | null>(null);
  let working = $state(false);

  async function refresh(): Promise<void> {
    try {
      const data = await invoke<MediaInfo>("media_now_playing");
      info = data;
      if (data && (data.title || data.state !== "closed")) {
        if (error) error = null;
      }
    } catch (e) {
      // 平台不支持 / 无会话：保留上次信息，或显示空态
      if (!info) error = String(e);
    }
  }

  // 每 2s 轮询；组件卸载时清理
  $effect(() => {
    void cellId;
    void refresh();
    const timer = setInterval(refresh, 2000);
    return () => clearInterval(timer);
  });



  /** 上一曲 / 播放暂停 / 下一曲；操作后立即刷新 */
  async function control(action: "previous" | "playpause" | "next"): Promise<void> {
    if (working) return;
    working = true;
    try {
      await invoke("media_control", { action });
      log.info(`音乐控制: ${action}`);
      setTimeout(refresh, 300);
    } catch (e) {
      log.error(`音乐控制失败(${action}): ${e}`);
    } finally {
      working = false;
    }
  }

  const playing = $derived(info?.state === "playing");
  const hasTrack = $derived(!!info?.title || info?.state === "playing" || info?.state === "paused");
</script>

<div class="music">
  {#if hasTrack && info}
    <div class="cover-wrap">
      {#if info.thumbnail}
        <img class="cover" src={info.thumbnail} alt="封面" draggable="false" />
      {:else}
        <div class="cover fallback">🎵</div>
      {/if}
    </div>
    <div class="meta">
      <div class="title" title={info.title}>{info.title || "未知曲目"}</div>
      <div class="artist" title={info.artist}>{info.artist || (info.album ? `专辑 · ${info.album}` : "—")}</div>
      {#if info.app}
        <div class="app">{info.app.split("!").pop()}</div>
      {/if}
    </div>
    <div class="controls">
      <button class="ctrl" title="上一曲" onclick={() => control("previous")} disabled={working}>⏮</button>
      <button
        class="ctrl main"
        title={playing ? "暂停" : "播放"}
        onclick={() => control("playpause")}
        disabled={working}
      >{playing ? "⏸" : "▶"}</button>
      <button class="ctrl" title="下一曲" onclick={() => control("next")} disabled={working}>⏭</button>
    </div>
  {:else}
    <div class="empty">
      <div class="empty-icon">🎵</div>
      <div class="empty-text">{error ? "无法读取媒体信息" : "未检测到正在播放的音乐"}</div>
      <div class="empty-hint">播放任意音乐（Spotify / 网易云 / 浏览器等）后自动显示</div>
    </div>
  {/if}
</div>

<style>
  .music {
    height: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 10px;
    box-sizing: border-box;
  }
  .cover-wrap {
    flex-shrink: 0;
    width: 64px;
    height: 64px;
  }
  .cover {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: 12px;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.35);
    display: block;
  }
  .cover.fallback {
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 30px;
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    border-radius: 12px;
  }
  .meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .title {
    font-size: 15px;
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
  .app {
    font-size: 10px;
    color: var(--fg-dim);
    opacity: 0.7;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .controls {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .ctrl {
    width: 30px;
    height: 30px;
    border: none;
    border-radius: 10px;
    background: var(--bg-hover);
    color: var(--fg);
    font-size: 14px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }
  .ctrl:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 25%, transparent);
  }
  .ctrl.main {
    width: 36px;
    height: 36px;
    font-size: 15px;
    background: var(--accent);
    color: #fff;
  }
  .ctrl:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .empty {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    text-align: center;
  }
  .empty-icon {
    font-size: 26px;
  }
  .empty-text {
    font-size: 13px;
    color: var(--fg-dim);
  }
  .empty-hint {
    font-size: 10px;
    color: var(--fg-dim);
    opacity: 0.7;
  }
</style>
