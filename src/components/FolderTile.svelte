<script lang="ts">
  import type { FolderCell } from "../core/types";

  let {
    folder,
    editMode = false,
    onopen,
    onedit,
    ondelete,
  }: {
    folder: FolderCell;
    /** 编辑模式：常显编辑/删除按钮，且不可打开 */
    editMode?: boolean;
    onopen?: () => void;
    onedit?: (id: string) => void;
    ondelete?: (id: string) => void;
  } = $props();
</script>

<div
  class="tile"
  class:editing={editMode}
  data-cell-id={folder.id}
  role="button"
  tabindex="0"
  onclick={onopen}
  onkeydown={(e) => e.key === "Enter" && onopen?.()}
>
  {#if editMode}
    <div class="actions">
      {#if onedit}
        <button
          class="edit"
          title="重命名 / 换图标"
          onclick={(e) => {
            e.stopPropagation();
            onedit?.(folder.id);
          }}
        >📝</button>
      {/if}
      <button
        class="del"
        title="删除文件夹"
        onclick={(e) => {
          e.stopPropagation();
          ondelete?.(folder.id);
        }}
      >×</button>
    </div>
  {/if}
  <div class="icon">{folder.emoji}</div>
  <div class="label">{folder.name}</div>
  <div class="count">{folder.items.length} 个图标</div>
</div>

<style>
  .tile {
    position: relative;
    height: var(--tile-size);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    background: var(--bg-elev);
    border: 1px dashed var(--fg-dim);
    border-radius: var(--radius);
    cursor: pointer;
    transition: transform 0.12s, background 0.15s;
  }
  .tile:hover {
    background: var(--bg-hover);
  }
  .tile:focus-visible {
    outline: 2px solid var(--accent);
  }
  .tile.editing {
    animation: wiggle 0.28s ease-in-out infinite;
  }
  @keyframes wiggle {
    0%,
    100% {
      transform: rotate(-1.3deg);
    }
    50% {
      transform: rotate(1.3deg);
    }
  }
  .icon {
    font-size: 34px;
    line-height: 1;
  }
  .label {
    font-size: 12px;
    color: var(--fg-dim);
    max-width: 90%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .count {
    font-size: 10px;
    color: var(--fg-dim);
    opacity: 0.7;
  }
  .actions {
    position: absolute;
    top: 6px;
    right: 8px;
    display: flex;
    gap: 4px;
    z-index: 2;
  }
  .del,
  .edit {
    width: 22px;
    height: 22px;
    border: none;
    border-radius: 50%;
    color: #fff;
    font-size: 12px;
    line-height: 1;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }
  .del {
    background: var(--danger);
  }
  .edit {
    background: var(--accent);
  }
</style>
