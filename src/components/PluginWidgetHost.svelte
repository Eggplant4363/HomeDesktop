<script lang="ts">
  // 插件自带小组件宿主（M16 沙箱版）：sandbox iframe 加载插件 JS + 设置桥
  import { onMount } from "svelte";
  import {
    buildPluginIframeSrcdoc,
    handleBridgeMessage,
    pluginWidgetUrl,
  } from "../core/pluginWidgetHost.svelte";
  import { log } from "../core/logger";
  import type { PluginInfo } from "../core/types";

  let {
    plugin,
    cellId,
    onopensettings,
  }: {
    plugin: PluginInfo;
    cellId: string;
    /** 插件 iframe 内点击（非交互元素）→ 打开设置 */
    onopensettings?: () => void;
  } = $props();

  let error = $state("");
  let srcdoc = $state("");

  function onMessage(e: MessageEvent): void {
    const d = e.data;
    if (!d || d.__hd !== true) return;
    if (d.t === "click") {
      onopensettings?.();
      return;
    }
    handleBridgeMessage(e);
  }

  async function mount(): Promise<void> {
    if (!plugin.widgetFile || !plugin.dir) {
      error = "插件缺少 widgetFile/dir 声明";
      return;
    }
    error = "";
    try {
      // 主窗口同源读取插件 JS（asset 协议与壁纸一致），内容内联进沙箱 srcdoc
      const url = pluginWidgetUrl(plugin.dir, plugin.widgetFile);
      const res = await fetch(url);
      if (!res.ok) throw new Error(`读取插件脚本失败: ${url} (${res.status})`);
      const code = await res.text();
      srcdoc = buildPluginIframeSrcdoc({
        code,
        elementTag: plugin.widgetElement ?? "",
        cellId,
      });
      log.info(`插件小组件已挂载(iframe): ${plugin.id} ${plugin.widgetElement}`);
    } catch (e) {
      error = String(e);
      log.error(`插件小组件初始化失败: ${plugin.id} -> ${e}`);
    }
  }

  onMount(() => {
    window.addEventListener("message", onMessage);
    void mount();
  });

  $effect(() => {
    // 卸载时移除消息监听
    return () => {
      window.removeEventListener("message", onMessage);
    };
  });
</script>

<div class="host">
  {#if srcdoc}
    <iframe
      class="frame"
      sandbox="allow-scripts"
      title="插件小组件"
      srcdoc={srcdoc}
    ></iframe>
  {:else if error}
    <div class="host-error">
      <span>{error}</span>
      <button onclick={() => void mount()}>重试</button>
    </div>
  {/if}
</div>

<style>
  .host {
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }
  .frame {
    width: 100%;
    height: 100%;
    border: none;
    background: transparent;
  }
  .host-error {
    font-size: 11px;
    color: var(--danger);
    display: flex;
    align-items: center;
    gap: 8px;
    max-width: 90%;
  }
  .host-error button {
    border: 1px solid var(--border);
    border-radius: 6px;
    background: transparent;
    color: var(--fg);
    font-size: 11px;
    padding: 2px 8px;
    cursor: pointer;
    white-space: nowrap;
  }
</style>
