<script lang="ts">
  // 网页快捷方式图标：读取实例 url 设置 → Rust 侧免证书校验抓取图标 → data URL 显示
  // （WebView 加载远程图不跳过自签证书校验，故图标字节由 Rust 抓取后转 data:）
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

  let src = $state<string | null>(null);
  let failed = $state(false);

  onMount(async () => {
    try {
      const raw = await getCellSetting<string>(cellId, pluginId, "url", fallbackUrl);
      const url = String(raw ?? "");
      if (!url) {
        failed = true;
        return;
      }
      const icon = await invoke<string | null>("web_fetch_icon", { url });
      if (icon) src = icon;
      else failed = true;
    } catch {
      failed = true;
    }
  });
</script>

{#if src && !failed}
  <img
    class="web-icon"
    src={src}
    alt=""
    draggable="false"
    onerror={() => (failed = true)}
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
