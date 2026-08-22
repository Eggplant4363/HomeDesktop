<script lang="ts">
  // MDI 图标（Material Design Icons，内联 SVG，无需字体文件，保证任何环境可渲染）
  // name 支持 "mdi-laptop" / "laptop" / "mdi:laptop"；emoji（如 🔌）按文字渲染；未知 mdi 名回退 help-circle
  import * as mdi from "@mdi/js";

  let { name, size = 24 }: { name: string; size?: number } = $props();

  const clean = $derived(name.replace(/^mdi[-:]/, ""));
  const key = $derived(
    "mdi" +
      clean
        .split("-")
        .filter(Boolean)
        .map((s) => s.charAt(0).toUpperCase() + s.slice(1))
        .join(""),
  );
  const path = $derived((mdi as Record<string, string>)[key]);
  const isEmoji = $derived(/[^\x00-\x7F]/.test(clean));
</script>

{#if path}
  <svg
    viewBox="0 0 24 24"
    width={size}
    height={size}
    fill="currentColor"
    aria-hidden="true"
  ><path d={path} /></svg>
{:else if isEmoji}
  <span style="font-size:{size}px;line-height:1" aria-hidden="true">{clean}</span>
{:else}
  <svg
    viewBox="0 0 24 24"
    width={size}
    height={size}
    fill="currentColor"
    aria-hidden="true"
  ><path d={mdi.mdiHelpCircle} /></svg>
{/if}
