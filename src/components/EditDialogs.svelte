<script lang="ts">
  // 各类编辑/选择弹窗集合（阶段1 从 App.svelte 模板抽出）
  // 含：移入文件夹、尺寸（保留内部能力）、文件夹编辑、图标自定义、借用系统图标、
  //     通用确认框、备份导入方式选择。
  import SystemAppsPanel from "./SystemAppsPanel.svelte";
  import { editor, runConfirm } from "../core/editorState.svelte";
  import {
    deleteCell,
    moveToFolder,
    saveFolderEdit,
    saveIconEdit,
    runImport,
  } from "../core/layoutActions.svelte";
  import { layout, setCellSize, repairPageOverlaps, currentPage } from "../core/stores.svelte";
  import { log } from "../core/logger";
  import { toast } from "../core/editorState.svelte";
  import { persist } from "../core/layoutActions.svelte";
  import { pickIconImage } from "../core/iconImage";
  import type { Cell } from "../core/types";

  let {
    folders = [] as Extract<Cell, { kind: "folder" }>[],
  }: { folders?: Extract<Cell, { kind: "folder" }>[] } = $props();

  /** 通用尺寸预设 */
  const sizeOptions: { w: number; h: number }[] = [
    { w: 1, h: 1 },
    { w: 1, h: 2 },
    { w: 2, h: 1 },
    { w: 2, h: 2 },
    { w: 2, h: 3 },
    { w: 3, h: 1 },
    { w: 3, h: 2 },
    { w: 3, h: 3 },
    { w: 4, h: 2 },
    { w: 4, h: 3 },
    { w: 4, h: 4 },
    { w: 5, h: 2 },
    { w: 6, h: 2 },
    { w: 6, h: 3 },
  ];
  let customSizeW = $state(2);
  let customSizeH = $state(2);

  /** 应用预设/自定义尺寸（保留能力；UI 已改为 ⤡ 拖拽为主） */
  function handleApplySize(size: { w: number; h: number }): void {
    if (editor.sizeTarget && setCellSize(editor.sizeTarget, size)) {
      if (repairPageOverlaps(currentPage.index)) log.info("设置尺寸后修复重叠");
      persist();
      log.info(`调整单元尺寸: ${editor.sizeTarget} -> ${size.w}x${size.h}`);
      toast(`尺寸已设为 ${size.w}×${size.h}`);
    }
    editor.sizeTarget = null;
  }

  /** 借用系统应用图标：写回编辑态 */
  function borrowIcon(app: { path: string }): void {
    editor.iconEditIconPath = app.path;
    editor.iconEditIconImage = "";
    editor.showIconBorrow = false;
  }

  /** 备份导入（保留入口：runImport 由本组件模态触发） */
  async function doImport(mode: "overwrite" | "merge"): Promise<void> {
    await runImport(mode);
  }
  // deleteCell 由确认框动作持有；此处引用避免 lint 误判未使用
  void deleteCell;
  void layout;
</script>

<!-- 移入文件夹 -->
{#if editor.moveTarget && !editor.showAdd}
  <div
    class="overlay"
    role="button"
    aria-label="关闭"
    tabindex="-1"
    onclick={() => (editor.moveTarget = null)}
    onkeydown={(e) => e.key === "Escape" && (editor.moveTarget = null)}
  >
    <div class="move-menu">
      <div class="move-head">移入文件夹</div>
      {#each folders as f (f.id)}
        <button class="move-row" onclick={() => moveToFolder(f.id)}>
          <span>{f.emoji}</span>
          <span>{f.name}</span>
        </button>
      {/each}
      {#if folders.length === 0}
        <div class="move-empty">暂无文件夹，请先在 ＋ 中创建</div>
      {/if}
    </div>
  </div>
{/if}

<!-- 尺寸（保留能力） -->
{#if editor.sizeTarget && !editor.showAdd}
  <div
    class="overlay"
    role="button"
    aria-label="关闭"
    tabindex="-1"
    onclick={() => (editor.sizeTarget = null)}
    onkeydown={(e) => e.key === "Escape" && (editor.sizeTarget = null)}
  >
    <div class="move-menu">
      <div class="move-head">设置尺寸（可自定义）</div>
      <div class="size-grid">
        {#each sizeOptions as opt (opt.w + "x" + opt.h)}
          <button class="size-opt" onclick={() => handleApplySize(opt)}>
            {opt.w}×{opt.h}
          </button>
        {/each}
      </div>
      <div class="custom-size">
        <span class="cs-label">自定义</span>
        <input type="number" min="1" max="8" step="0.5" bind:value={customSizeW} />
        <span>×</span>
        <input type="number" min="1" max="8" step="0.5" bind:value={customSizeH} />
        <button class="cs-apply" onclick={() => handleApplySize({ w: customSizeW, h: customSizeH })}>应用</button>
      </div>
    </div>
  </div>
{/if}

<!-- 文件夹编辑 -->
{#if editor.folderEditTarget}
  <div
    class="overlay"
    role="button"
    aria-label="关闭"
    tabindex="-1"
    onclick={(e) => e.target === e.currentTarget && (editor.folderEditTarget = null)}
    onkeydown={(e) => e.key === "Escape" && (editor.folderEditTarget = null)}
  >
    <div class="move-menu folder-edit">
      <div class="move-head">编辑文件夹</div>
      <div class="fe-row">
        <span class="fe-label">名称</span>
        <input
          class="fe-input"
          type="text"
          placeholder="文件夹名称"
          bind:value={editor.folderEditName}
        />
      </div>
      <div class="fe-row">
        <span class="fe-label">图标</span>
        <input
          class="fe-input fe-emoji"
          type="text"
          placeholder="📁"
          bind:value={editor.folderEditEmoji}
        />
      </div>
      <div class="fe-presets">
        {#each ["📁", "🧰", "🎮", "📷", "🛠️", "💼", "🎵", "📚", "⭐", "🔥", "🎯", "🏠"] as e (e)}
          <button
            class="fe-preset"
            class:active={editor.folderEditEmoji === e}
            onclick={() => (editor.folderEditEmoji = e)}
          >{e}</button>
        {/each}
      </div>
      <div class="fe-actions">
        <button class="fe-btn cancel" onclick={() => (editor.folderEditTarget = null)}>取消</button>
        <button class="fe-btn save" onclick={saveFolderEdit}>保存</button>
      </div>
    </div>
  </div>
{/if}

<!-- 图标自定义 -->
{#if editor.iconEditTarget}
  <div
    class="overlay"
    role="button"
    aria-label="关闭"
    tabindex="-1"
    onclick={(e) => e.target === e.currentTarget && (editor.iconEditTarget = null)}
    onkeydown={(e) => e.key === "Escape" && (editor.iconEditTarget = null)}
  >
    <div class="move-menu folder-edit">
      <div class="move-head">自定义图标</div>
      <div class="fe-row">
        <span class="fe-label">名称</span>
        <input
          class="fe-input"
          type="text"
          placeholder="图标名称"
          bind:value={editor.iconEditTitle}
        />
        <button
          class="fe-toggle"
          class:on={editor.iconEditShowLabel}
          title="是否在桌面上显示名称"
          onclick={() => (editor.iconEditShowLabel = !editor.iconEditShowLabel)}
        >
          {editor.iconEditShowLabel ? "👁 显示" : "🙈 隐藏"}
        </button>
      </div>
      <div class="fe-row">
        <span class="fe-label">图标</span>
        <input
          class="fe-input fe-emoji"
          type="text"
          placeholder="默认"
          bind:value={editor.iconEditEmoji}
        />
        <span class="fe-hint">留空=插件默认</span>
      </div>
      <div class="fe-presets">
        {#each ["🎮", "📷", "🛠️", "💼", "🎵", "📚", "⭐", "🔥", "🎯", "🏠", "✏️", "💡"] as e (e)}
          <button
            class="fe-preset"
            class:active={editor.iconEditEmoji === e}
            onclick={() => (editor.iconEditEmoji = e)}
          >{e}</button>
        {/each}
      </div>
      <div class="fe-row">
        <span class="fe-label">颜色</span>
        <div class="fe-colors">
          {#each ["#e53935", "#fb8c00", "#fdd835", "#43a047", "#1e88e5", "#8e24aa", "#00897b", ""] as c (c || "none")}
            <button
              class="fe-color"
              class:active={editor.iconEditColor === c}
              style={c ? `background: ${c};` : ""}
              title={c || "默认（透明）"}
              onclick={() => (editor.iconEditColor = c)}
            >{c ? "" : "默认"}</button>
          {/each}
        </div>
      </div>
      {#if !editor.iconEditIsWidget}
        <div class="fe-row">
          <span class="fe-label">图标</span>
          <div class="fe-borrow">
            <button class="fe-btn save" onclick={() => (editor.showIconBorrow = true)}>🎨 借用系统应用图标…</button>
            <button class="fe-btn save" onclick={() => void pickIconImage()}>🖼 来自图片…</button>
            {#if editor.iconEditIconImage}
              <img class="icon-preview" src={editor.iconEditIconImage} alt="" title="当前图片图标" />
            {/if}
            {#if editor.iconEditIconPath || editor.iconEditIconImage}
              <button
                class="fe-btn cancel"
                onclick={() => {
                  editor.iconEditIconPath = "";
                  editor.iconEditIconImage = "";
                }}
              >清除</button>
            {/if}
          </div>
        </div>
      {/if}
      <div class="fe-actions">
        <button class="fe-btn cancel" onclick={() => (editor.iconEditTarget = null)}>取消</button>
        <button class="fe-btn save" onclick={saveIconEdit}>保存</button>
      </div>
    </div>
  </div>
{/if}

<!-- 借用系统应用图标面板 -->
{#if editor.showIconBorrow && editor.iconEditTarget}
  <SystemAppsPanel
    mode="borrow"
    onadd={(app) => borrowIcon(app)}
    onclose={() => (editor.showIconBorrow = false)}
  />
{/if}

<!-- 通用确认框 -->
{#if editor.confirmAction}
  <div
    class="overlay top"
    role="button"
    aria-label="关闭"
    tabindex="-1"
    onclick={(e) => e.target === e.currentTarget && (editor.confirmAction = null)}
    onkeydown={(e) => e.key === "Escape" && (editor.confirmAction = null)}
  >
    <div class="move-menu folder-edit">
      <div class="move-head">{editor.confirmAction.title}</div>
      <div class="cf-text">{editor.confirmAction.message}</div>
      <div class="fe-actions">
        <button class="fe-btn cancel" onclick={() => (editor.confirmAction = null)}>取消</button>
        <button class="fe-btn save cf-danger" onclick={runConfirm}>删除</button>
      </div>
    </div>
  </div>
{/if}

<!-- 备份导入方式 -->
{#if editor.pendingImportSrc}
  <div
    class="overlay top"
    role="button"
    aria-label="关闭"
    tabindex="-1"
    onclick={(e) => e.target === e.currentTarget && (editor.pendingImportSrc = null)}
    onkeydown={(e) => e.key === "Escape" && (editor.pendingImportSrc = null)}
  >
    <div class="move-menu folder-edit">
      <div class="move-head">导入备份方式</div>
      <div class="cf-text">覆盖 = 完全恢复成备份时的状态（当前新增的图标会消失）；合并 = 保留当前图标，只补上备份里有而当前没有的。</div>
      <div class="fe-actions">
        <button class="fe-btn cancel" onclick={() => (editor.pendingImportSrc = null)}>取消</button>
        <button class="fe-btn save" onclick={() => void doImport("overwrite")}>覆盖导入</button>
        <button class="fe-btn save" onclick={() => void doImport("merge")}>合并导入</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 30;
  }
  .overlay.top {
    z-index: 60;
  }
  .move-menu {
    min-width: 280px;
    max-width: 400px;
    max-height: 78vh;
    overflow-y: auto;
    background: var(--bg-elev);
    border-radius: 14px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .folder-edit {
    width: 320px;
  }
  .move-head {
    font-weight: 600;
    font-size: 14px;
    padding: 4px 4px 8px;
  }
  .move-row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    border: none;
    background: transparent;
    color: var(--fg);
    font-size: 13px;
    padding: 8px 10px;
    border-radius: 8px;
    cursor: pointer;
    text-align: left;
  }
  .move-row:hover {
    background: var(--bg-hover);
  }
  .move-empty {
    font-size: 12px;
    color: var(--fg-dim);
    padding: 10px;
    text-align: center;
  }
  .size-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 6px;
  }
  .size-opt {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: transparent;
    color: var(--fg);
    font-size: 12px;
    padding: 7px 0;
    cursor: pointer;
  }
  .size-opt:hover {
    border-color: var(--accent);
  }
  .custom-size {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    padding-top: 6px;
  }
  .cs-label {
    color: var(--fg-dim);
  }
  .custom-size input {
    width: 52px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-input);
    color: var(--fg);
    padding: 4px 6px;
    text-align: center;
  }
  .cs-apply {
    border: none;
    border-radius: 6px;
    background: var(--accent);
    color: #fff;
    padding: 5px 10px;
    cursor: pointer;
  }
  .fe-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 12px;
  }
  .fe-label {
    width: 36px;
    font-size: 13px;
    color: var(--fg-dim);
    flex-shrink: 0;
  }
  .fe-input {
    flex: 1;
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 12px;
    padding: 4px 8px;
    outline: none;
  }
  .fe-input:focus {
    border-color: var(--accent);
  }
  .fe-emoji {
    max-width: 72px;
    text-align: center;
    font-size: 16px;
  }
  .fe-hint {
    font-size: 11px;
    color: var(--fg-dim);
  }
  .fe-presets {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 6px 12px;
  }
  .fe-preset {
    width: 36px;
    height: 36px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: transparent;
    font-size: 18px;
    cursor: pointer;
  }
  .fe-preset:hover {
    border-color: var(--accent);
  }
  .fe-preset.active {
    border-color: var(--accent);
    background: var(--bg-hover);
  }
  .fe-colors {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .fe-color {
    width: 26px;
    height: 26px;
    border: 1px solid var(--border);
    border-radius: 50%;
    background: transparent;
    color: var(--fg-dim);
    font-size: 10px;
    cursor: pointer;
  }
  .fe-color.active {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 35%, transparent);
  }
  .fe-borrow {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }
  .fe-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 10px 12px 14px;
  }
  .fe-btn {
    border: none;
    border-radius: 8px;
    font-size: 13px;
    padding: 7px 16px;
    cursor: pointer;
  }
  .fe-btn.cancel {
    background: var(--bg-input);
    color: var(--fg);
  }
  .fe-btn.save {
    background: var(--accent);
    color: #fff;
  }
  .fe-btn.save.cf-danger {
    background: var(--danger);
  }
  .fe-toggle {
    flex-shrink: 0;
    border: 1px dashed var(--border);
    border-radius: 8px;
    background: transparent;
    color: var(--fg-dim);
    font-size: 12px;
    padding: 7px 10px;
    cursor: pointer;
    white-space: nowrap;
  }
  .fe-toggle:hover {
    border-color: var(--accent);
  }
  .fe-toggle.on {
    border-style: solid;
    border-color: var(--accent);
    color: var(--accent);
  }
  .icon-preview {
    width: 28px;
    height: 28px;
    border-radius: 7px;
    object-fit: cover;
    flex-shrink: 0;
  }
  .cf-text {
    padding: 4px 12px 8px;
    font-size: 12px;
    color: var(--fg-dim);
  }
</style>
