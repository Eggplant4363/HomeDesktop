<script lang="ts">
  // 网页快捷方式图标：读取实例 url 设置 → 显示网站 favicon
  // 优先级：页面声明的图标（link rel=icon）→ /favicon.ico → /favicon.png → emoji
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCellSetting } from "../core/pluginSettings.svelte";

  let {
    cellId,
    pluginId,
    fallbackUrl,
    fallbackEmoji,
  }: { cellId: string; pluginId: string; fallbackUrl: string; fallbackEmoji: string } =
    $props();

  let host = $state<string | null>(null);
  /** 页面 <link rel="icon"> 声明的图标（绝对 URL） */
  let declaredIcon = $state<string | null>(null);
  /** 当前尝试的候选序号 */
  let attempt = $state(0);

  onMount(async () => {
    try {
      const raw = await getCellSetting<string>(cellId, pluginId, "url", fallbackUrl);
      const url = String(raw ?? "");
      host = new URL(url).hostname || null;
      try {
        const icon = await invoke<string | null>("web_fetch_icon", { url });
        if (icon) declaredIcon = new URL(icon).href; // 规范化（自动编码中文等）
      } catch {
        /* 无声明图标，走默认路径 */
      }
    } catch {
      host = null;
    }
  });

  const candidates = $derived.by(() => {
    const list: string[] = [];
    if (declaredIcon) list.push(declaredIcon);
    if (host) {
      list.push(`https://${host}/favicon.ico`);
      list.push(`https://${host}/favicon.png`);
    }
    return list;
  });
  const showImg = $derived(host !== null && attempt < candidates.length);
  const src = $derived(candidates[attempt] ?? "");
</script>

{#if showImg}
  <img
    class="web-icon"
    src={src}
    alt=""
    draggable="false"
    onerror={() => (attempt += 1)}
  />
{:else}
  <div class="icon">{fallbackEmoji}</div>
{/if}

<style>
  .web-icon {
    width: 52px;
    height: 52px;
    border-radius: 14px;
    object-fit: contain;
    background: #fff;
    padding: 6px;
  }
  .icon {
    font-size: 34px;
    line-height: 1;
  }
</style>
