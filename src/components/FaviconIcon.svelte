<script lang="ts">
  // 网页快捷方式图标：读取实例 url 设置 → 显示网站 favicon
  // 兜底链：/favicon.ico → /favicon.png → emoji（不依赖第三方服务，国内网络可用）
  import { onMount } from "svelte";
  import { getCellSetting } from "../core/pluginSettings.svelte";

  let {
    cellId,
    pluginId,
    fallbackUrl,
    fallbackEmoji,
  }: { cellId: string; pluginId: string; fallbackUrl: string; fallbackEmoji: string } =
    $props();

  let host = $state<string | null>(null);
  /** 0 = /favicon.ico；1 = /favicon.png；2 = 放弃 → emoji */
  let attempt = $state(0);

  onMount(async () => {
    try {
      const raw = await getCellSetting<string>(cellId, pluginId, "url", fallbackUrl);
      host = new URL(String(raw ?? "")).hostname || null;
    } catch {
      host = null;
    }
  });

  const src = $derived.by(() => {
    if (!host) return "";
    return `https://${host}/${attempt === 0 ? "favicon.ico" : "favicon.png"}`;
  });
  const showImg = $derived(host !== null && attempt < 2);
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
