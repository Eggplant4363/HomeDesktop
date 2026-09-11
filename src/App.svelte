<script lang="ts">
  // App.svelte（重构后）：只保留「装配 + 顶层生命周期 + 模板骨架」。
  // 业务逻辑已拆分到 core/：
  //   editorState      弹窗/编辑器 UI 状态、toast、确认框、删除动效
  //   windowFx         窗口显隐动画、页面滑动切换
  //   layoutActions    图标/文件夹/页面增删改、拖放、启动、插件安装、备份
  //   appSettingsActions  外观/系统/代理设置
  import { onMount } from "svelte";
  import Grid from "./components/Grid.svelte";
  import FolderView from "./components/FolderView.svelte";
  import { fade } from "svelte/transition";
  import PluginSettingsMenu from "./components/PluginSettingsMenu.svelte";
  import HaSettingsMenu from "./components/HaSettingsMenu.svelte";
  import SearchBar from "./components/SearchBar.svelte";
  import AddMenu from "./components/AddMenu.svelte";
  import SystemAppsPanel from "./components/SystemAppsPanel.svelte";
  import ShortcutRow from "./components/ShortcutRow.svelte";
  import SearchPanel from "./components/SearchPanel.svelte";
  import PluginsPanel from "./components/PluginsPanel.svelte";
  import LyricsPanel from "./components/LyricsPanel.svelte";

  import { open } from "@tauri-apps/plugin-dialog";
  import { loadPlugins } from "./core/pluginLoader";
  import { loadLayout } from "./core/persistence";
  import { loadAppearance, appearance, backgroundCss, backgroundPresets, tileSizePresets, getCurrentBackground } from "./core/appearance.svelte";
  import {
    addPage,
    currentPage,
    enterEditMode,
    exitEditMode,
    getDisplayPageCount,
    repairPageOverlaps,
    setCellSize,
    getFocusCell,
    clearFocusCell,
    layout,
    openFolder,
    plugins,
    query,
    removePage,
    ui,
  } from "./core/stores.svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { listen } from "@tauri-apps/api/event";
  import { log } from "./core/logger";
  import { lyrics, closeLyrics } from "./core/lyricsState.svelte";

  // ---- 拆出的模块 ----
  import {
    editor,
    toast,
    askConfirm,
    runConfirm,
    shatterThen,
    flashAdded,
    closeAllOverlays,
  } from "./core/editorState.svelte";
  import {
    winFx,
    hideWindow,
    showWindow,
    windowLike,
    bindSlideWrap,
    onSwipeMove,
    onSwipeEnd,
    slide,
  } from "./core/windowFx.svelte";
  import {
    persist,
    addPluginCell,
    newFolder,
    editFolder,
    saveFolderEdit,
    editIcon,
    saveIconEdit,
    deleteCell,
    deleteFolderItem,
    launch,
    pickApp,
    openFolderById,
    toggleMoveTarget,
    moveToFolder,
    folderDropAt,
    dropAt,
    dropIntoFolder,
    resizeTo,
    resizeEnd,
    toggleSettingsTarget,
    prevPage,
    nextPage,
    installPluginFromFile,
    uninstallPlugin,
    exportBackup,
    pickImportBackup,
    runImport,
    findPageCellById,
    refreshWebTitle,
  } from "./core/layoutActions.svelte";
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
    initSystemSettings,
    savePadShortcut,
    saveSearchShortcut,
    loadProxy,
    saveProxy,
    syncCustomColor,
  } from "./core/appSettingsActions.svelte";
  import { invoke } from "@tauri-apps/api/core";

  /** 滑动容器元素（供 windowFx 测量宽度） */
  let slideWrapEl = $state<HTMLElement | undefined>();
  $effect(() => {
    bindSlideWrap(slideWrapEl);
  });

  /** 通用尺寸预设（任何图标/小组件都可自定义尺寸） */
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
  /** 自定义尺寸输入（网格单位） */
  let customSizeW = $state(2);
  let customSizeH = $state(2);

  /** 设置菜单目标插件（文件夹内图标同样可打开设置） */
  const settingsPlugin = $derived.by(() => {
    if (!editor.settingsTarget) return undefined;
    for (const page of layout.pages) {
      for (const c of page) {
        if (c.kind === "folder") {
          const icon = c.items.find((i) => i.id === editor.settingsTarget);
          if (icon) return plugins.find((p) => p.id === icon.pluginId);
        }
      }
      const cell = page.find((c) => c.id === editor.settingsTarget);
      if (cell?.kind === "icon") return plugins.find((p) => p.id === cell.pluginId);
    }
    return undefined;
  });

  const inFolder = $derived(openFolder.folderId !== null);
  const folders = $derived(layout.pages.flatMap((page) => page.filter((c) => c.kind === "folder")));

  /** 总页数：紧凑显示（分辨率变化）时用显示层页数，否则用存储页数 */
  const totalPages = $derived(getDisplayPageCount() || layout.pages.length);

  // 同步给 windowFx（滑动切换判断边界用）
  $effect(() => {
    winFx.totalPages = totalPages;
  });

  // 监听搜索定位：切页后闪现目标单元格
  $effect(() => {
    const f = getFocusCell();
    if (f) {
      flashAdded(f);
      clearFocusCell();
    }
  });

  // 托盘 / 快捷键（Rust）发来的显示/隐藏请求：统一走动画
  let unlistenWinEvents: (() => void)[] = [];
  $effect(() => {
    void appearance.effects.windowAnim;
    let alive = true;
    listen("hide-window", () => hideWindow()).then((un) => alive && unlistenWinEvents.push(un));
    listen("show-window", () => showWindow()).then((un) => alive && unlistenWinEvents.push(un));
    return () => {
      alive = false;
      for (const un of unlistenWinEvents) un();
      unlistenWinEvents = [];
    };
  });

  // 紧凑显示（分辨率变化）切换时：页索引越界则钳制
  $effect(() => {
    const n = getDisplayPageCount();
    if (n > 0 && currentPage.index >= n) {
      currentPage.index = n - 1;
    } else if (n === 0 && currentPage.index >= layout.pages.length) {
      currentPage.index = Math.max(0, layout.pages.length - 1);
    }
  });

  onMount(async () => {
    // 任务栏不显示图标（全屏启动器用托盘控制）
    try {
      await getCurrentWindow().setSkipTaskbar(true);
      log.debug("已设置任务栏隐藏（skipTaskbar）");
    } catch (e) {
      log.error(`设置任务栏隐藏失败: ${e}`);
    }
    await Promise.all([loadPlugins(), loadLayout(), loadAppearance()]);
    syncCustomColor();
    await initSystemSettings();
    void loadProxy();

    // ---------- 全屏模式兜底 ----------
    const win = getCurrentWindow();
    void win.setFullscreen(true).catch(() => {
      /* 已全屏时可能不 resolve，故不阻塞启动 */
    });

    // 全局搜索快捷键（Rust 事件）→ 打开/关闭搜索面板
    await listen("homedesktop:search", () => {
      log.debug("收到搜索快捷键事件");
      editor.showSearch = !editor.showSearch;
    });

    log.info(
      `启动完成: 插件 ${plugins.length} 个, 页面 ${layout.pages.length} 页, ` +
        `主题 ${appearance.theme}, tileSize ${appearance.tileSize}`,
    );
  });

  // ---------- 顶层动作（薄封装：转调 core 模块） ----------

  function closeSearch(): void {
    editor.showSearch = false;
  }

  function handleToggleEdit(): void {
    if (ui.editMode) {
      exitEditMode();
      log.info("退出编辑模式");
    } else {
      closeAllOverlays();
      enterEditMode();
      log.info("进入编辑模式");
    }
  }

  function handleSearchOpenIcon(cellId: string): void {
    closeSearch();
    launch(cellId);
  }

  function handleOpenFolder(folderId: string): void {
    openFolderById(folderId);
  }

  function handleSearchOpenFolder(folderId: string): void {
    closeSearch();
    handleOpenFolder(folderId);
    log.info(`搜索打开文件夹: ${folderId}`);
  }

  async function handleSearchOpenApp(path: string): Promise<void> {
    closeSearch();
    try {
      await invoke("launch_path", { path });
      log.info(`搜索启动应用: ${path}`);
      hideWindow();
    } catch (e) {
      log.error(`搜索启动应用失败: ${path} -> ${e}`);
      toast(String(e));
    }
  }

  async function handleSearchOpenPlugin(pluginId: string): Promise<void> {
    closeSearch();
    try {
      await invoke("launch_action", { pluginId });
      log.info(`搜索启动插件: ${pluginId}`);
      hideWindow();
    } catch (e) {
      log.error(`搜索启动插件失败: ${pluginId} -> ${e}`);
      toast(String(e));
    }
  }

  // Grid/FolderView 的零参回调（模块函数需要 totalPages）
  function handlePrev(): void { prevPage(); }
  function handleNext(): void { nextPage(totalPages); }

  // 借用系统应用图标（面板选中后写回编辑态）
  function handleBorrowIconCompat(app: { path: string }): void {
    editor.iconEditIconPath = app.path;
    editor.iconEditIconImage = "";
    editor.showIconBorrow = false;
  }

  // 打开尺寸弹窗（保留 ⇲ 入口的兼容：现仅用于内部，UI 已改为 ⤡ 拖拽）
  function toggleSizeTargetCompat(id: string): void {
    editor.sizeTarget = editor.sizeTarget === id ? null : id;
    editor.settingsTarget = null;
  }

  /** 应用预设/自定义尺寸 */
  function handleApplySize(size: { w: number; h: number }): void {
    if (editor.sizeTarget && setCellSize(editor.sizeTarget, size)) {
      if (repairPageOverlaps(currentPage.index)) log.info("设置尺寸后修复重叠");
      persist();
      log.info(`调整单元尺寸: ${editor.sizeTarget} -> ${size.w}x${size.h}`);
      toast(`尺寸已设为 ${size.w}×${size.h}`);
    }
    editor.sizeTarget = null;
  }

  /** 选择图片作为图标（Rust 读文件 → 压缩 ≤256px → data URL） */
  async function pickIconImage(): Promise<void> {
    try {
      const picked = await open({
        multiple: false,
        filters: [{ name: "图片", extensions: ["png", "jpg", "jpeg", "webp", "gif", "bmp", "ico"] }],
      });
      if (typeof picked !== "string" || !picked) return;
      const dataUrl = await invoke<string | null>("image_to_data_url", { path: picked });
      if (!dataUrl) {
        toast("不支持的图片格式");
        return;
      }
      editor.iconEditIconImage = await downscaleDataUrl(dataUrl, 256);
      editor.iconEditIconPath = "";
      toast("已选择图片图标（点保存生效）");
      log.info(`选择图片图标: ${picked}`);
    } catch (e) {
      log.error(`选择图片图标失败: ${e}`);
      toast(`图片读取失败：${e}`);
    }
  }

  /** data URL 图片压缩到 maxSize 边长内（canvas），避免布局文件过大 */
  function downscaleDataUrl(dataUrl: string, maxSize: number): Promise<string> {
    return new Promise((resolve, reject) => {
      const img = new Image();
      img.onload = () => {
        try {
          const scale = Math.min(1, maxSize / Math.max(img.width, img.height));
          const w = Math.max(1, Math.round(img.width * scale));
          const h = Math.max(1, Math.round(img.height * scale));
          const canvas = document.createElement("canvas");
          canvas.width = w;
          canvas.height = h;
          const ctx = canvas.getContext("2d");
          if (!ctx) throw new Error("canvas 不可用");
          ctx.drawImage(img, 0, 0, w, h);
          resolve(canvas.toDataURL("image/png"));
        } catch (e) {
          reject(e);
        }
      };
      img.onerror = () => reject(new Error("图片解码失败"));
      img.src = dataUrl;
    });
  }

  function handleAddPage(): void {
    addPage();
    currentPage.index = layout.pages.length - 1;
    log.info(`新增页面 ${currentPage.index + 1}`);
  }

  function handleRemovePage(): void {
    removePage(currentPage.index);
    persist();
    log.info(`删除页面 ${currentPage.index + 1}`);
  }

  /** 打开插件管理页面 */
  function openPlugins(): void {
    editor.showPlugins = true;
  }
</script>


<main
  class="shell"
  class:anim-hidden={winFx.hidden}
  style="--tile-size: {appearance.tileSize}px; --gap: {appearance.gridSpacing}px; background: {backgroundCss(getCurrentBackground())};"
>
  <header class="topbar">
    <SearchBar />
    {#if ui.editMode}
      <button class="icon-btn edit-done" title="完成" onclick={handleToggleEdit}>✓ 完成</button>
    {:else}
      <button class="icon-btn" title="编辑模式" onclick={handleToggleEdit}>✎</button>
    {/if}
    <button class="icon-btn" title="外观设置" onclick={() => (editor.showSettings = !editor.showSettings)}>⚙</button>
    <button class="icon-btn" title="添加图标/小组件" onclick={() => (editor.showAdd = !editor.showAdd)}>＋</button>
  </header>

  {#if editor.error}
    <div class="toast">{editor.error}</div>
  {/if}

  {#if editor.showAdd}
    <AddMenu
      plugins={plugins}
      onadd={(p) => addPluginCell(p)}
      onnewfolder={(name) => newFolder(name)}
      oninstallplugin={installPluginFromFile}
      onopenplugins={openPlugins}
      onclose={() => (editor.showAdd = false)}
    />
  {/if}

  {#if editor.showSystemApps}
    <SystemAppsPanel
      onadd={(app) => pickApp(app)}
      onclose={() => {
        editor.systemAppsSource = null;
        editor.showSystemApps = false;
      }}
    />
  {/if}

  {#if editor.showSettings}
    <div
      class="overlay"
      role="button"
      aria-label="关闭"
      tabindex="-1"
      onclick={(e) => e.target === e.currentTarget && (editor.showSettings = false)}
      onkeydown={(e) => e.key === "Escape" && (editor.showSettings = false)}
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
          <button class="wallpaper-btn" onclick={openPlugins}>🧩 打开插件管理（已安装 / 市场）…</button>
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
  {/if}

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

  {#if settingsPlugin && settingsPlugin.settings?.length}
    {#if settingsPlugin.domain}
      <HaSettingsMenu
        plugin={settingsPlugin}
        cellId={editor.settingsTarget ?? undefined}
        onclose={() => (editor.settingsTarget = null)}
      />
    {:else}
      <PluginSettingsMenu
        plugin={settingsPlugin}
        cellId={editor.settingsTarget ?? undefined}
        onclose={() => (editor.settingsTarget = null)}
        onurlchange={(cellId, url) => void refreshWebTitle(cellId, url)}
      />
    {/if}
  {/if}

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

  {#if editor.showIconBorrow && editor.iconEditTarget}
    <SystemAppsPanel
      mode="borrow"
      onadd={(app) => handleBorrowIconCompat(app)}
      onclose={() => (editor.showIconBorrow = false)}
    />
  {/if}

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
          <button class="fe-btn save" onclick={() => void runImport("overwrite")}>覆盖导入</button>
          <button class="fe-btn save" onclick={() => void runImport("merge")}>合并导入</button>
        </div>
      </div>
    </div>
  {/if}

  {#if editor.showSearch}
    <SearchPanel
      onclose={closeSearch}
      onopenicon={(id) => handleSearchOpenIcon(id)}
      onopenfolder={(id) => handleSearchOpenFolder(id)}
      onopenapp={(path) => void handleSearchOpenApp(path)}
      onopenplugin={(id) => void handleSearchOpenPlugin(id)}
    />
  {/if}

  {#if editor.showPlugins}
    <PluginsPanel
      onclose={() => (editor.showPlugins = false)}
      onupdate={installPluginFromFile}
      onuninstall={(id, name) => uninstallPlugin(id, name)}
      onmessage={(msg, isError) => toast(msg, isError ? 4000 : 2500)}
    />
  {/if}

  <section class="content">
    {#if inFolder}
      <div class="folder-wrap" transition:windowLike>
      <FolderView
        breakingId={editor.breakingId}
        onaddclick={() => (editor.showAdd = true)}
        onlaunch={(id) => launch(id)}
        onmove={(id) => toggleMoveTarget(id)}
        onediticon={(id) => editIcon(id)}
        ondelete={(fid, iid) => deleteFolderItem(fid, iid)}
        ondropat={folderDropAt}
        onresize={(id) => toggleSizeTargetCompat(id)}
        onresizeto={(id, w, h) => resizeTo(id, w, h)}
        onresizeend={(id) => resizeEnd(id)}
        onsettings={(id) => toggleSettingsTarget(id)}
      />
      </div>
    {:else}
      <div
        class="page-slide"
        transition:fade={{ duration: 200 }}
        bind:this={slideWrapEl}
        style="transform: translateX({slide.x}px); transition: {slide.transition};"
      >
        <Grid
          cells={query.text ? layout.pages.flat() : layout.pages[currentPage.index] ?? []}
          pages={layout.pages}
          queryText={query.text}
          highlightId={editor.addedFlashId}
          breakingId={editor.breakingId}
        onlaunch={(id) => launch(id)}
        ondelete={(id) => deleteCell(id)}
        onaddclick={() => (editor.showAdd = true)}
        onopenfolder={(id) => handleOpenFolder(id)}
        oneditfolder={(id) => editFolder(id)}
        onediticon={(id) => editIcon(id)}
        onmoveicon={(id) => toggleMoveTarget(id)}
        onresize={(id) => toggleSizeTargetCompat(id)}
        onresizeto={(id, w, h) => resizeTo(id, w, h)}
        onresizeend={(id) => resizeEnd(id)}
        onsettings={(id) => toggleSettingsTarget(id)}
        ondropat={dropAt}
        ondropinto={dropIntoFolder}
        onflipprev={handlePrev}
        onflipnext={handleNext}
        onwheelnav={(dir) => onSwipeEnd(-dir * (slideWrapEl?.clientWidth || window.innerWidth) * 0.3)}
          onfitted={() => persist()}
          onblankclick={hideWindow}
          onswipemove={onSwipeMove}
          onswipeend={onSwipeEnd}
        />
      </div>
    {/if}
  </section>

  {#if !inFolder}
    <footer class="pager">
      {#if totalPages > 1}
        <div class="pager-group">
          <button class="icon-btn" onclick={handlePrev} disabled={currentPage.index === 0}>‹</button>
          <span>{currentPage.index + 1} / {totalPages}</span>
          <button class="icon-btn" onclick={handleNext} disabled={currentPage.index === totalPages - 1}>›</button>
          <button
            class="icon-btn page-op"
            title="删除当前页（需要确认）"
            onclick={() =>
              askConfirm(
                `删除第 ${currentPage.index + 1} 页？`,
                "该页所有图标 / 小组件 / 文件夹将被移除，且无法撤销。",
                handleRemovePage,
              )}
          >−</button>
        </div>
      {/if}
      <button class="add-page-btn" onclick={handleAddPage} title="新增页面">＋ 新增页面</button>
    </footer>
  {/if}
  {#if lyrics.open}
    <LyricsPanel onclose={closeLyrics} />
  {/if}
</main>

<style>
  .shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    padding: 18px 24px;
    /* 窗口显示/隐藏过渡（macOS 风格：淡入淡出 + 轻微缩放） */
    transition:
      opacity 0.2s cubic-bezier(0.25, 0.1, 0.25, 1),
      transform 0.2s cubic-bezier(0.25, 0.1, 0.25, 1);
  }
  .shell.anim-hidden {
    opacity: 0;
    transform: scale(0.96);
  }
  .topbar {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 20px;
  }
  .icon-btn {
    width: 40px;
    height: 40px;
    border: none;
    border-radius: 12px;
    background: var(--bg-elev);
    color: var(--fg);
    font-size: 20px;
    cursor: pointer;
  }
  .icon-btn:hover:not(:disabled) {
    background: var(--bg-hover);
  }
  .icon-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .icon-btn.edit-done {
    width: auto;
    padding: 0 14px;
    font-size: 14px;
    background: var(--accent);
    color: #fff;
  }
  .icon-btn.edit-done:hover {
    background: var(--accent);
    opacity: 0.9;
  }
  .content {
    flex: 1;
    overflow: hidden;
  }
  /* 页面左右滑动容器（切页动画） */
  .folder-wrap {
    height: 100%;
  }
  .page-slide {
    height: 100%;
    will-change: transform;
  }
  .pager {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 10px;
    color: var(--fg-dim);
    font-size: 13px;
  }
  .pager-group {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 auto;
  }
  .pager .page-op {
    width: 32px;
    height: 32px;
    font-size: 18px;
  }
  .add-page-btn {
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg-elev);
    color: var(--fg);
    font-size: 13px;
    padding: 8px 16px;
    cursor: pointer;
    white-space: nowrap;
  }
  .add-page-btn:hover {
    background: var(--bg-hover);
    border-color: var(--accent);
  }
  .toast {
    position: fixed;
    top: 64px;
    left: 50%;
    transform: translateX(-50%);
    background: var(--danger);
    color: #fff;
    padding: 8px 16px;
    border-radius: 10px;
    font-size: 13px;
    /* 提示消息必须盖过所有弹层（插件管理 55、确认框 200） */
    z-index: 300;
  }
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    /* App 级弹层（设置/编辑等）低于组件级弹层（插件管理 55/搜索 60），
       这样从设置打开的插件管理页能盖在设置之上 */
    z-index: 30;
  }
  /* 必须最前的弹层（卸载确认框、导入模式选择）：盖过一切 */
  .overlay.top {
    z-index: 200;
  }
  .move-menu {
    width: 280px;
    max-height: 50vh;
    overflow-y: auto;
    background: var(--bg-elev);
    border-radius: 14px;
    padding: 8px;
  }
  .move-head {
    padding: 10px 12px;
    font-weight: 600;
    font-size: 14px;
  }
  .move-row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border: none;
    background: transparent;
    color: var(--fg);
    border-radius: 10px;
    cursor: pointer;
    text-align: left;
    font-size: 14px;
  }
  .move-row:hover {
    background: var(--bg-hover);
  }
  .custom-size {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 10px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }
  .cs-label {
    font-size: 12px;
    color: var(--fg-dim);
  }
  .custom-size input {
    width: 46px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 13px;
    padding: 4px 6px;
    outline: none;
    text-align: center;
  }
  .custom-size input:focus {
    border-color: var(--accent);
  }
  .cs-apply {
    margin-left: auto;
    border: none;
    border-radius: 8px;
    background: var(--accent);
    color: #fff;
    font-size: 12px;
    padding: 5px 14px;
    cursor: pointer;
  }
  .move-empty {
    padding: 18px;
    text-align: center;
    color: var(--fg-dim);
    font-size: 12px;
  }
  .size-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
    padding: 4px 8px 12px;
  }
  .size-opt {
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 13px;
    padding: 10px 0;
    cursor: pointer;
  }
  .size-opt:hover {
    border-color: var(--accent);
    background: var(--bg-hover);
  }
  .folder-edit {
    width: 320px;
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
  .cf-text {
    padding: 4px 12px 8px;
    font-size: 12px;
    color: var(--fg-dim);
  }
  .fe-hint {
    font-size: 11px;
    color: var(--fg-dim);
    white-space: nowrap;
  }
  .fe-colors {
    flex: 1;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .fe-color {
    width: 30px;
    height: 30px;
    border: 2px solid var(--border);
    border-radius: 50%;
    font-size: 10px;
    color: var(--fg-dim);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .fe-color.active {
    border-color: var(--accent);
  }
  .icon-preview {
    width: 28px;
    height: 28px;
    border-radius: 7px;
    object-fit: cover;
    flex-shrink: 0;
  }
  .fe-borrow {
    flex: 1;
    display: flex;
    gap: 8px;
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
  .sc-warn {
    font-size: 11px;
    color: var(--danger);
    background: color-mix(in srgb, var(--danger) 12%, transparent);
    border-radius: 8px;
    padding: 5px 8px;
    line-height: 1.5;
  }
  .bg-presets {
    display: flex;
    gap: 10px;
  }
  .bg-swatch {
    width: 44px;
    height: 44px;
    border: 2px solid var(--border);
    border-radius: 12px;
    cursor: pointer;
  }
  .bg-swatch:hover {
    border-color: var(--accent);
  }
  .custom-color {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12px;
    color: var(--fg-dim);
  }
  .custom-color input {
    width: 40px;
    height: 28px;
    border: none;
    background: transparent;
    cursor: pointer;
  }
  .size-presets {
    display: flex;
    gap: 8px;
  }
.set-val {
    font-size: 11px;
    color: var(--fg-dim);
    font-weight: 400;
  }
  .spacing-range {
    width: 100%;
    accent-color: var(--accent);
    margin-top: 2px;
  }
  .size-btn {
    flex: 1;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 13px;
    padding: 8px 0;
    cursor: pointer;
  }
  .size-btn:hover {
    border-color: var(--accent);
  }
  .size-btn.active {
    border-color: var(--accent);
    background: var(--bg-hover);
    color: #fff;
  }
  .wallpaper-btn {
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 13px;
    padding: 9px 12px;
    cursor: pointer;
    text-align: left;
  }
  .wallpaper-btn:hover {
    border-color: var(--accent);
  }
  .wallpaper-hint {
    font-size: 11px;
    color: var(--fg-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .proxy-row {
    display: flex;
    gap: 8px;
    margin-top: 8px;
    align-items: center;
  }
  .proxy-select {
    flex: 1;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 13px;
    padding: 8px 10px;
  }
  .proxy-input {
    flex: 1;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 13px;
    padding: 8px 10px;
    min-width: 0;
  }
  .proxy-input.port {
    flex: 0 0 90px;
  }
  .proxy-hint {
    font-size: 11px;
    color: var(--fg-dim);
  }
</style>
