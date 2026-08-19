<script lang="ts">
  // 网页快捷方式图标：读取实例 url 设置 → Rust 免证书校验抓取图标 → data URL 显示
  // - 用 getCellSetting（读 config，尊重已保存的网址）→ 各图标显示各自网站的图标
  // - 图标由 Rust 磁盘缓存（按网址哈希），抓一次永久生效（"存一次"）
  // - url 设置变化（pluginSettings 缓存）时自动重新抓取
  import { invoke } from "@tauri-apps/api/core";
  import { getCellSetting, peekCellSetting } from "../core/pluginSettings.svelte";
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
  let url = $state<string | null>(null);

  // url 解析：读配置（含已保存值），监听设置缓存变化（改网址自动更新）
  $effect(() => {
    void peekCellSetting(cellId, pluginId, "url");
    void getCellSetting<string>(cellId, pluginId, "url", fallbackUrl).then((u) => {
      url = String(u ?? fallbackUrl ?? "");
    });
  });

  // url 变化 → 抓取图标（Rust 侧磁盘缓存保证只抓一次）
  $effect(() => {
    const u = url;
    if (!u) {
      failed = true;
      return;
    }
    let cancelled = false;
    src = null;
    failed = false;
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
