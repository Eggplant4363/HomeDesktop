<script lang="ts">
  // 网页快捷方式图标：读取实例 url 设置 → Rust 侧免证书校验抓取图标 → data URL 显示
  // （WebView 加载远程图不跳过自签证书校验，故图标字节由 Rust 抓取后转 data:）
  // url 设置变化（pluginSettings 缓存）时自动重新抓取
  import { invoke } from "@tauri-apps/api/core";
  import { peekCellSetting } from "../core/pluginSettings.svelte";
  import { log } from "../core/logger";

  let {
    cellId,
    pluginId,
    fallbackUrl,
    fallbackEmoji,
  }: { cellId: string; pluginId: string; fallbackUrl: string; fallbackEmoji: string } =
    $props();

  let src = $state<string | null>(null);
  let failed = $state(false);

  const url = $derived(peekCellSetting<string>(cellId, pluginId, "url") ?? fallbackUrl);

  $effect(() => {
    const u = url;
    if (!u) {
      failed = true;
      return;
    }
    let cancelled = false;
    invoke<string | null>("web_fetch_icon", { url: u })
      .then((icon) => {
        if (cancelled) return;
        if (icon) {
          src = icon;
          failed = false;
        } else {
          failed = true;
        }
      })
      .catch((e) => {
        if (cancelled) return;
        log.error(`获取网站图标失败: ${u} -> ${e}`);
        failed = true;
      });
    return () => {
      cancelled = true;
    };
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
