<script lang="ts">
  // 应用图标：优先显示系统真实图标（懒加载 + 缓存），加载中/失败回退首字母彩色头像
  import { onMount } from "svelte";
  import { appIcons, loadAppIcon } from "../core/appIcons.svelte";

  let {
    path = "",
    name = "",
    size = 52,
    radius = 14,
  }: {
    /** 可执行文件/快捷方式路径；为空则始终显示头像 */
    path?: string;
    /** 应用名（头像回退时的首字母来源） */
    name?: string;
    size?: number;
    radius?: number;
  } = $props();

  const hue = $derived(
    (() => {
      let h = 0;
      for (let i = 0; i < name.length; i++) {
        h = (h * 31 + name.charCodeAt(i)) >>> 0;
      }
      return h % 360;
    })(),
  );

  onMount(() => {
    if (path) void loadAppIcon(path);
  });
</script>

{#if path && appIcons[path]}
  <img
    class="app-icon"
    src={appIcons[path]}
    alt={name}
    style="width: {size}px; height: {size}px; border-radius: {radius}px;"
  />
{:else}
  <div
    class="avatar"
    style="width: {size}px; height: {size}px; border-radius: {radius}px; background: hsl({hue} 55% 45%); font-size: {Math.round(size * 0.46)}px;"
  >{name[0]?.toUpperCase() ?? "?"}</div>
{/if}

<style>
  .app-icon {
    object-fit: contain;
    flex-shrink: 0;
    background: transparent;
  }
  .avatar {
    color: #fff;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
</style>
