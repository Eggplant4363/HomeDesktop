<script lang="ts">
  import { query } from "../core/stores.svelte";
</script>

<div class="search">
  <span class="glass">🔍</span>
  <input
    type="text"
    placeholder="搜索应用 / 插件 / 智能家居…"
    bind:value={query.text}
    oncompositionend={(e) => {
      query.text = (e.currentTarget as HTMLInputElement).value; // IME 组词结束强制取最终值
    }}
  />
  {#if query.text}
    <button class="clear" onclick={() => (query.text = "")}>×</button>
  {/if}
</div>

<style>
  .search {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--bg-elev);
    border-radius: 12px;
    padding: 0 14px;
    height: 40px;
  }
  .glass {
    font-size: 14px;
    opacity: 0.6;
  }
  input {
    flex: 1;
    border: none;
    outline: none;
    background: transparent;
    color: var(--fg);
    font-size: 14px;
  }
  input::placeholder {
    color: var(--fg-dim);
  }
  .clear {
    border: none;
    background: transparent;
    color: var(--fg-dim);
    font-size: 16px;
    cursor: pointer;
  }
</style>
