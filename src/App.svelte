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
  import SettingsDialog from "./components/SettingsDialog.svelte";
  import EditDialogs from "./components/EditDialogs.svelte";
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
    <SettingsDialog onopenplugins={openPlugins} />
  {/if}

  <EditDialogs {folders} />

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
        onaddclick={() => (editor.showAdd = true)}
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
          onwheelnav={(dir) => onSwipeEnd(-dir * (slideWrapEl?.clientWidth || window.innerWidth) * 0.3)}
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
</style>
