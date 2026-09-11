<script lang="ts">
  // 外观设置弹窗（阶段1 从 App.svelte 模板抽出）
  // 主题 / 背景 / 壁纸 / 特效 / 系统 / 快捷键 / 插件入口 / 网络代理 / 数据备份 / 图标大小 / 网格间距
  import ShortcutRow from "./ShortcutRow.svelte";
  import {
    appearance,
    backgroundCss,
    backgroundPresets,
    getCurrentBackground,
    tileSizePresets,
  } from "../core/appearance.svelte";
  import { editor } from "../core/editorState.svelte";
  import {
    settings,
    setBackground,
    setCustomColor,
    setTheme,
    toggleIconAddEffect,
    toggleWindowAnim,
    setGridSpacing,
    setTileSize,
    pickWallpaper,
    toggleAutostart,
    savePadShortcut,
    saveSearchShortcut,
    saveProxy,
  } from "../core/appSettingsActions.svelte";
  import { exportBackup, pickImportBackup } from "../core/layoutActions.svelte";

  let { onopenplugins }: { onopenplugins?: () => void } = $props();

  function close(): void {
    editor.showSettings = false;
  }
</script>

<div
  class="overlay"
  role="button"
  aria-label="关闭"
  tabindex="-1"
  onclick={(e) => e.target === e.currentTarget && close()}
  onkeydown={(e) => e.key === "Escape" && close()}
>
  <div class="settings">
    <div class="set-head">外观设置</div>

    <div class="set-section">
      <div class="set-label">主题</div>
      <div class="size-presets">
        <button
          class="size-btn"
          class:active={appearance.theme === "dark"}
          onclick={() => setTheme("dark")}
        >🌙 深色</button>
        <button
          class="size-btn"
          class:active={appearance.theme === "light"}
          onclick={() => setTheme("light")}
        >☀️ 浅色</button>
      </div>
    </div>

    <div class="set-section">
      <div class="set-label">背景（当前为{appearance.theme === "dark" ? "深色" : "浅色"}主题，切换主题可分别设置）</div>
      <div class="bg-presets">
        {#each backgroundPresets as p (p.label)}
          <button
            class="bg-swatch"
            title={p.label}
            style="background: {backgroundCss(p.bg)};"
            onclick={() => setBackground(p.bg)}
          ></button>
        {/each}
      </div>
      <div class="custom-color">
        <input type="color" bind:value={settings.customColor} onchange={setCustomColor} />
        <span>自定义颜色</span>
      </div>
      <button class="wallpaper-btn" onclick={pickWallpaper}>🖼 选择图片壁纸…</button>
      {#if getCurrentBackground().kind === "image"}
        <div class="wallpaper-hint">当前：{getCurrentBackground().value.split(/[\\/]/).pop()}</div>
      {/if}
    </div>

    <div class="set-section">
      <div class="set-label">特效</div>
      <button class="wallpaper-btn" onclick={toggleIconAddEffect}>
        {appearance.effects.iconAdd
          ? "✅ 添加图标弹出特效已开启（点击关闭）"
          : "⛔ 添加图标弹出特效已关闭（点击开启）"}
      </button>
      <button class="wallpaper-btn" onclick={toggleWindowAnim}>
        {appearance.effects.windowAnim
          ? "✅ 窗口显示/隐藏过渡已开启（点击关闭）"
          : "⛔ 窗口显示/隐藏过渡已关闭（点击开启）"}
      </button>
    </div>

    <div class="set-section">
      <div class="set-label">系统</div>
      <button class="wallpaper-btn" onclick={toggleAutostart}>
        {settings.autoStart ? "✅ 开机自启已开启（点击关闭）" : "⛔ 开机自启已关闭（点击开启）"}
      </button>
    </div>

    <div class="set-section">
      <div class="set-label">快捷键</div>
      <ShortcutRow
        label="显示 / 隐藏 Pad（全屏）"
        value={settings.padShortcut}
        onchange={savePadShortcut}
      />
      {#if settings.shortcutStatus["togglePad"] && !settings.shortcutStatus["togglePad"].ok}
        <div class="sc-warn">⚠ “{settings.shortcutStatus["togglePad"].spec}”注册失败：{settings.shortcutStatus["togglePad"].error}，此快捷键未生效（不影响使用）</div>
      {/if}
      <ShortcutRow
        label="打开全局搜索"
        value={settings.searchShortcut}
        onchange={saveSearchShortcut}
      />
      {#if settings.shortcutStatus["search"] && !settings.shortcutStatus["search"].ok}
        <div class="sc-warn">⚠ “{settings.shortcutStatus["search"].spec}”注册失败：{settings.shortcutStatus["search"].error}，此快捷键未生效（不影响使用）</div>
      {/if}
    </div>

    <div class="set-section">
      <div class="set-label">插件</div>
      <button class="wallpaper-btn" onclick={() => onopenplugins?.()}>🧩 打开插件管理（已安装 / 市场）…</button>
    </div>

    <div class="set-section">
      <div class="set-label">网络代理（在线市场下载使用）</div>
      <div class="proxy-row">
        <select class="proxy-select" bind:value={settings.proxy.mode}>
          <option value="none">不使用代理</option>
          <option value="http">HTTP 代理</option>
          <option value="socks5">SOCKS5 代理</option>
        </select>
      </div>
      <div class="proxy-row">
        <input class="proxy-input" placeholder="代理地址（如 127.0.0.1）" bind:value={settings.proxy.host} />
        <input
          class="proxy-input port"
          placeholder="端口（如 7890）"
          bind:value={settings.proxy.port}
        />
      </div>
      <div class="proxy-row">
        <input
          class="proxy-input"
          placeholder="用户名（可选）"
          bind:value={settings.proxy.username}
        />
        <input
          class="proxy-input"
          type="password"
          placeholder="密码（可选）"
          bind:value={settings.proxy.password}
        />
      </div>
      <div class="proxy-row">
        <button class="wallpaper-btn" onclick={() => void saveProxy()}>💾 保存代理设置</button>
        <span class="proxy-hint">保存后重新打开在线市场即生效</span>
      </div>
    </div>

    <div class="set-section">
      <div class="set-label">数据备份</div>
      <div class="fe-borrow">
        <button class="fe-btn save" onclick={() => void exportBackup()}>📤 导出备份…</button>
        <button class="fe-btn cancel" onclick={() => void pickImportBackup()}>📥 导入备份…</button>
      </div>
    </div>

    <div class="set-section">
      <div class="set-label">图标大小</div>
      <div class="size-presets">
        {#each tileSizePresets as p (p.label)}
          <button
            class="size-btn"
            class:active={appearance.tileSize === p.value}
            onclick={() => setTileSize(p.value)}
          >{p.label}</button>
        {/each}
      </div>
    </div>

    <div class="set-section">
      <div class="set-label">网格间距 <span class="set-val">{appearance.gridSpacing}px</span></div>
      <input
        class="spacing-range"
        type="range"
        min="0"
        max="30"
        step="1"
        value={appearance.gridSpacing}
        oninput={(e) => setGridSpacing(Number((e.currentTarget as HTMLInputElement).value))}
      />
    </div>
  </div>
</div>

<style>
  /* 弹窗遮罩（各弹窗共用的形状；本组件自带一份以便独立） */
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 30;
  }
  .settings {
    width: 340px;
    max-height: 80vh;
    overflow-y: auto;
    /* 不显示竖向滚动条（滚轮仍可滚动） */
    scrollbar-width: none;
    -ms-overflow-style: none;
    background: var(--bg-elev);
    border-radius: 16px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .settings::-webkit-scrollbar {
    display: none;
  }
  .set-head {
    font-weight: 600;
    font-size: 15px;
  }
  .set-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .set-label {
    font-size: 12px;
    color: var(--fg-dim);
  }
  .set-val {
    color: var(--accent);
  }
  .sc-warn {
    font-size: 11px;
    color: var(--danger);
    line-height: 1.4;
  }
  .bg-presets {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .bg-swatch {
    width: 34px;
    height: 34px;
    border: 1px solid var(--border);
    border-radius: 8px;
    cursor: pointer;
  }
  .bg-swatch:hover {
    border-color: var(--accent);
  }
  .custom-color {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--fg-dim);
  }
  .custom-color input {
    width: 34px;
    height: 26px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: transparent;
    cursor: pointer;
  }
  .size-presets {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .size-btn {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: transparent;
    color: var(--fg);
    font-size: 12px;
    padding: 5px 10px;
    cursor: pointer;
  }
  .size-btn:hover {
    border-color: var(--accent);
  }
  .size-btn.active {
    border-color: var(--accent);
    color: var(--accent);
  }
  .wallpaper-btn {
    border: 1px dashed var(--border);
    border-radius: 10px;
    background: transparent;
    color: var(--fg);
    font-size: 12px;
    padding: 8px 12px;
    cursor: pointer;
    text-align: left;
  }
  .wallpaper-btn:hover {
    border-color: var(--accent);
  }
  .wallpaper-hint {
    font-size: 11px;
    color: var(--fg-dim);
  }
  .proxy-row {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .proxy-select {
    flex: 1;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 12px;
    padding: 6px 8px;
    outline: none;
  }
  .proxy-input {
    flex: 1;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 12px;
    padding: 6px 8px;
    outline: none;
  }
  .proxy-input.port {
    max-width: 110px;
  }
  .proxy-hint {
    font-size: 11px;
    color: var(--fg-dim);
  }
  /* 备份按钮（与图标编辑弹窗保持一致的外观） */
  .fe-borrow {
    display: flex;
    gap: 8px;
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
</style>
