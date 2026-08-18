<script lang="ts">
  import { onMount } from "svelte";
  import Grid from "./components/Grid.svelte";
  import FolderView from "./components/FolderView.svelte";
  import PluginSettingsMenu from "./components/PluginSettingsMenu.svelte";
  import SearchBar from "./components/SearchBar.svelte";
  import AddMenu from "./components/AddMenu.svelte";
  import SystemAppsPanel from "./components/SystemAppsPanel.svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { installPlugin, loadPlugins, launchCell } from "./core/pluginLoader";
  import type { AppInfo } from "./core/pluginLoader";
  import { loadLayout, saveLayout } from "./core/persistence";
  import {
    addCell,
    addIconToFolder,
    addPage,
    createFolder,
    currentPage,
    enterEditMode,
    exitEditMode,
    layout,
    moveCellAcrossPages,
    moveIconToFolder,
    openFolder,
    plugins,
    query,
    removeCell,
    removeCellsByPlugin,
    removeIconFromFolder,
    removePage,
    renameFolder,
    reorderIconInFolder,
    replaceCellWithApp,
    setCellSize,
    setFolderEmoji,
    ui,
    updateIconAppearance,
  } from "./core/stores.svelte";
  import {
    appearance,
    applyTheme,
    backgroundCss,
    backgroundPresets,
    getCurrentBackground,
    loadAppearance,
    saveAppearance,
    setBackgroundForCurrentTheme,
    tileSizePresets,
  } from "./core/appearance.svelte";
  import { getWidgetDef } from "./widgets";
  import {
    autostartEnabled,
    getFullscreenPref,
    getSearchShortcut,
    getToggleShortcut,
    pickWallpaperImage,
    setAutostart,
    setFullscreenPref,
    setSearchShortcut,
    setToggleShortcut,
  } from "./core/system";
  import { comboLabel } from "./core/shortcuts";
  import ShortcutRow from "./components/ShortcutRow.svelte";
  import SearchPanel from "./components/SearchPanel.svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import { log } from "./core/logger";
  import type { Cell, IconCell, PluginInfo } from "./core/types";
  import type { Background } from "./core/appearance.svelte";
  import PluginsPanel from "./components/PluginsPanel.svelte";

  let showAdd = $state(false);
  let showSettings = $state(false);
  /** 系统应用面板（内置「系统应用」插件点击后打开） */
  let showSystemApps = $state(false);
  /** 触发面板的「系统应用」槽位图标 id（选中应用后原位替换它） */
  let systemAppsSource = $state<string | null>(null);
  /** 全屏 Pad 状态（M6：与窗口真实状态同步，供 ⛶ 按钮/Esc 使用） */
  let isFullscreen = $state(false);
  /** Pad 开关快捷键（M7，可配置，默认 alt+space） */
  let padShortcut = $state("alt+space");
  /** 搜索唤起快捷键（M10，可配置，默认 ctrl+space） */
  let searchShortcut = $state("ctrl+space");
  /** 全局搜索面板（M10） */
  let showSearch = $state(false);
  /** 正在编辑的文件夹 id（M8：重命名/换 emoji 弹窗） */
  let folderEditTarget = $state<string | null>(null);
  let folderEditName = $state("");
  let folderEditEmoji = $state("");
  /** 正在自定义的图标 id（M9：重命名/换 emoji/换颜色/借用系统图标弹窗） */
  let iconEditTarget = $state<string | null>(null);
  let iconEditTitle = $state("");
  let iconEditEmoji = $state("");
  let iconEditColor = $state("");
  let iconEditIconPath = $state("");
  let showIconBorrow = $state(false);
  /** 通用确认框（所有删除操作都要确认）：null = 未打开 */
  let confirmAction = $state<{ title: string; message: string; onConfirm: () => void } | null>(
    null,
  );
  /** 备份导入：选好文件后等待选择"覆盖/合并" */
  let pendingImportSrc = $state<string | null>(null);
  /** 插件管理页面（已安装 / 市场） */
  let showPlugins = $state(false);
  let error = $state("");
  /** 通用提示条（操作反馈） */
  function toast(msg: string, ms = 2500): void {
    error = msg;
    setTimeout(() => (error = ""), ms);
  }
  /** 正在选择"移入哪个文件夹"的图标 id */
  let moveTarget = $state<string | null>(null);
  /** 正在选择尺寸的图标 id */
  let sizeTarget = $state<string | null>(null);
  /** 正在打开插件设置菜单的单元格 id */
  let settingsTarget = $state<string | null>(null);
  /** 自定义背景色的临时值 */
  let customColor = $state("#10141c");
  /** 开机自启状态 */
  let autoStart = $state(false);

  /** 未声明 sizes 的插件回退到的默认尺寸档 */
  const defaultSizeOptions: { w: number; h: number }[] = [
    { w: 1, h: 1 },
    { w: 2, h: 1 },
    { w: 1, h: 2 },
    { w: 2, h: 2 },
    { w: 3, h: 2 },
    { w: 4, h: 2 },
    { w: 4, h: 3 },
  ];

  /** 尺寸选择器选项：优先插件声明的 sizes（小米/安卓设计） */
  const sizeOptions = $derived.by(() => {
    if (!sizeTarget) return defaultSizeOptions;
    const cell = layout.pages[currentPage.index]?.find((c) => c.id === sizeTarget);
    const plugin = cell?.kind === "icon" && plugins.find((p) => p.id === cell.pluginId);
    if (!plugin) return [{ w: 1, h: 1 }]; // 非插件图标（应用抽屉）固定 1×1
    const sizes = plugin.sizes;
    return sizes && sizes.length > 0 ? sizes : defaultSizeOptions;
  });

  /** 设置菜单目标插件 */
  const settingsPlugin = $derived.by(() => {
    if (!settingsTarget) return undefined;
    for (const page of layout.pages) {
      const cell = page.find((c) => c.id === settingsTarget);
      if (cell?.kind === "icon") return plugins.find((p) => p.id === cell.pluginId);
    }
    return undefined;
  });

  const inFolder = $derived(openFolder.folderId !== null);
  const folders = $derived(
    layout.pages.flatMap((page) => page.filter((c) => c.kind === "folder")),
  );

  onMount(async () => {
    await Promise.all([loadPlugins(), loadLayout(), loadAppearance()]);
    customColor =
      getCurrentBackground().kind === "color" ? getCurrentBackground().value : "#10141c";
    autoStart = await autostartEnabled();

    // ---------- 全屏 Pad（M6） ----------
    const win = getCurrentWindow();
    // 窗口尺寸变化（含全屏切换，热键由 Rust 触发时也走这里）→ 同步状态并记忆
    win.onResized(() => {
      void refreshFullscreen();
    });
    isFullscreen = await win.isFullscreen();
    // 记忆上次状态：上次全屏 → 启动直接进全屏 Pad
    if (await getFullscreenPref()) {
      await win.setFullscreen(true);
      isFullscreen = true;
      log.info("启动即全屏 Pad（记忆上次状态）");
    }
    // Esc 退出全屏（回窗口模式）
    window.addEventListener("keydown", handleGlobalKeydown);

    // ---------- 快捷键（M7/M10） ----------
    padShortcut = await getToggleShortcut();
    searchShortcut = await getSearchShortcut();
    // 全局搜索快捷键（Rust 事件）→ 打开/关闭搜索面板
    await listen("homedesktop:search", () => {
      log.debug("收到搜索快捷键事件");
      showSearch = !showSearch;
    });

    log.info(
      `启动完成: 插件 ${plugins.length} 个, 页面 ${layout.pages.length} 页, ` +
        `主题 ${appearance.theme}, tileSize ${appearance.tileSize}`,
    );
  });

  /** 全屏状态与窗口同步（变化时写入记忆配置） */
  async function refreshFullscreen(): Promise<void> {
    try {
      const v = await getCurrentWindow().isFullscreen();
      if (v !== isFullscreen) {
        isFullscreen = v;
        void setFullscreenPref(v);
      }
    } catch (e) {
      console.error("[homedesktop] refresh fullscreen failed:", e);
    }
  }

  function handleGlobalKeydown(e: KeyboardEvent): void {
    // 全屏 Pad 中按 Esc → 退出全屏（回到窗口模式）
    if (e.key === "Escape" && isFullscreen) {
      e.preventDefault();
      void handleToggleFullscreen();
    }
  }

  /** 顶栏 ⛶：窗口模式 ⇄ 全屏 Pad（窗口保持可见） */
  async function handleToggleFullscreen(): Promise<void> {
    try {
      const win = getCurrentWindow();
      const next = !(await win.isFullscreen());
      await win.setFullscreen(next);
      isFullscreen = next;
      await setFullscreenPref(next);
      log.info(`全屏 Pad: ${next ? "进入" : "退出"}`);
    } catch (e) {
      log.error(`全屏切换失败: ${e}`);
      toast(String(e));
    }
  }

  /** 保存"显示/隐藏 Pad"快捷键（M7）；返回错误信息（null = 成功） */
  async function handleSetPadShortcut(combo: string): Promise<string | null> {
    const err = await setToggleShortcut(combo);
    if (err) {
      log.error(`快捷键保存失败: ${combo} -> ${err}`);
      return err;
    }
    padShortcut = combo;
    log.info(`快捷键已修改: ${comboLabel(combo)}`);
    toast(`快捷键已改为 ${comboLabel(combo)}`);
    return null;
  }

  /** 保存搜索唤起快捷键（M10） */
  async function handleSetSearchShortcut(combo: string): Promise<string | null> {
    const err = await setSearchShortcut(combo);
    if (err) {
      log.error(`快捷键保存失败: ${combo} -> ${err}`);
      return err;
    }
    searchShortcut = combo;
    log.info(`搜索快捷键已修改: ${comboLabel(combo)}`);
    toast(`搜索快捷键已改为 ${comboLabel(combo)}`);
    return null;
  }

  // ---------- 全局搜索面板（M10） ----------

  function closeSearch(): void {
    showSearch = false;
  }

  /** 搜索面板：启动桌面图标（含 system_apps 拦截）/ 打开文件夹 */
  function handleSearchOpenIcon(cellId: string): void {
    closeSearch();
    handleLaunch(cellId);
  }

  function handleSearchOpenFolder(folderId: string): void {
    closeSearch();
    openFolder.folderId = folderId;
    log.info(`搜索打开文件夹: ${folderId}`);
  }

  /** 搜索面板：直接启动系统应用（不走桌面布局） */
  async function handleSearchOpenApp(path: string): Promise<void> {
    closeSearch();
    try {
      await invoke("launch_path", { path });
      log.info(`搜索启动应用: ${path}`);
    } catch (e) {
      log.error(`搜索启动应用失败: ${path} -> ${e}`);
      toast(String(e));
    }
  }

  /** 搜索面板：直接执行插件动作 */
  async function handleSearchOpenPlugin(pluginId: string): Promise<void> {
    closeSearch();
    try {
      await invoke("launch_action", { pluginId });
      log.info(`搜索启动插件: ${pluginId}`);
    } catch (e) {
      log.error(`搜索启动插件失败: ${pluginId} -> ${e}`);
      toast(String(e));
    }
  }

  function persist(): void {
    void saveLayout(layout);
  }

  function handleAdd(plugin: PluginInfo): void {
    const def = plugin.pluginType === "widget" ? getWidgetDef(plugin.widgetComponent) : undefined;
    const icon: IconCell = {
      kind: "icon",
      id: crypto.randomUUID(),
      pluginId: plugin.id,
      title: plugin.name,
      size: plugin.sizes?.[0] ?? def?.defaultSize ?? { w: 1, h: 1 },
    };
    if (openFolder.folderId) {
      addIconToFolder(openFolder.folderId, icon);
    } else {
      addCell(icon);
    }
    persist();
    showAdd = false;
    log.info(`添加图标: ${plugin.name} (${icon.size.w}x${icon.size.h})`);
  }

  function handleNewFolder(name: string): void {
    const id = createFolder(name);
    persist();
    if (moveTarget) {
      moveIconToFolder(moveTarget, id);
      moveTarget = null;
      persist();
    }
    showAdd = false;
    log.info(`新建文件夹: ${name} (${id})`);
  }

  function handleDelete(id: string): void {
    const cell = findPageCellById(id);
    if (!cell) return;
    if (cell.kind === "folder") {
      askConfirm(
        `删除文件夹「${cell.name}」？`,
        `文件夹内 ${cell.items.length} 个图标将一并删除，且无法撤销。`,
        () => {
          removeCell(id);
          persist();
          log.info(`删除文件夹: ${id} (${cell.name})`);
        },
      );
    } else {
      askConfirm(`删除图标「${cell.title}」？`, "删除后无法撤销。", () => {
        removeCell(id);
        persist();
        log.info(`删除图标: ${id} (${cell.title})`);
      });
    }
  }

  /** 删除文件夹内图标（带确认） */
  function handleDeleteFolderItem(folderId: string, iconId: string): void {
    const folder = findFolderById(folderId);
    const icon = folder?.items.find((i) => i.id === iconId);
    askConfirm(`删除「${icon?.title ?? iconId}」？`, "该图标将从文件夹中删除，且无法撤销。", () => {
      removeIconFromFolder(folderId, iconId);
      persist();
      log.info(`删除文件夹内图标: ${folderId}/${iconId}`);
    });
  }

  /** 在页面级查找单元格（不查文件夹内） */
  function findPageCellById(cellId: string): Cell | undefined {
    for (const page of layout.pages) {
      const c = page.find((x) => x.id === cellId);
      if (c) return c;
    }
    return undefined;
  }

  /** 按单元格 id 在所有页面与文件夹内查找图标 */
  function findIconCell(cellId: string): IconCell | undefined {
    for (const page of layout.pages) {
      for (const cell of page) {
        if (cell.kind === "icon" && cell.id === cellId) return cell;
        if (cell.kind === "folder") {
          const found = cell.items.find((i) => i.id === cellId);
          if (found) return found;
        }
      }
    }
    return undefined;
  }

  function handleLaunch(cellId: string): void {
    const icon = findIconCell(cellId);
    const plugin = icon ? plugins.find((p) => p.id === icon.pluginId) : undefined;
    // 内置「系统应用」插件：动作由前端处理，打开应用面板（本图标作为"槽位"，选中应用后原位替换）
    const kind = icon?.action?.kind ?? plugin?.actions?.[0]?.kind;
    if (kind === "system_apps") {
      log.info(`打开系统应用面板: ${cellId}`);
      showAdd = false;
      systemAppsSource = cellId;
      showSystemApps = true;
      return;
    }
    log.info(`启动图标: ${cellId}`);
    launchCell(cellId).catch((e) => {
      log.error(`启动失败: ${cellId} -> ${e}`);
      toast(String(e));
    });
  }

  /** 选中应用：原位替换「系统应用」槽位图标（保持位置不变） */
  function handlePickApp(app: AppInfo): void {
    if (systemAppsSource && replaceCellWithApp(systemAppsSource, app)) {
      persist();
      log.info(`应用槽位替换: ${systemAppsSource} -> ${app.name} (${app.path})`);
      toast(`已添加应用：${app.name}`);
    } else {
      toast("未找到「系统应用」图标，请重新添加");
    }
    systemAppsSource = null;
    showSystemApps = false;
  }

  async function handleInstallPlugin(): Promise<void> {
    try {
      const picked = await open({
        multiple: false,
        filters: [{ name: "插件包", extensions: ["zip"] }],
      });
      if (typeof picked !== "string" || !picked) return;
      log.info(`安装插件包: ${picked}`);
      const installed = await installPlugin(picked);
      await loadPlugins();
      toast(`插件「${installed.name}」安装成功`);
      log.info(`插件安装成功: ${installed.name}`);
    } catch (e) {
      log.error(`插件安装失败: ${e}`);
      toast(`安装失败：${e}`, 4000);
    }
  }

  /** 卸载插件（M11）：确认 → 删目录 → 清理桌面上的该插件图标 → 刷新列表 */
  function handleUninstallPlugin(pluginId: string, name: string): void {
    askConfirm(`卸载插件「${name}」？`, "将从磁盘删除该插件，桌面上它的图标也会被移除。", () => {
      void (async () => {
        try {
          await invoke("plugins_uninstall", { pluginId });
          const removed = removeCellsByPlugin(pluginId);
          persist();
          await loadPlugins();
          log.info(`插件已卸载: ${pluginId}（清理桌面图标 ${removed} 个）`);
          // dev 环境：项目 plugins/ 或资源目录里可能有同名插件 → 卸载后列表仍显示（内置标记）
          const stillThere = plugins.some((p) => p.id === pluginId);
          toast(
            stillThere
              ? `「${name}」用户副本已卸载（开发目录仍有同名内置插件）`
              : `插件「${name}」已卸载`,
          );
        } catch (e) {
          log.error(`插件卸载失败: ${pluginId} -> ${e}`);
          toast(`卸载失败：${e}`, 4000);
        }
      })();
    });
  }

  /** 打开插件管理页面 */
  function openPlugins(): void {
    showPlugins = true;
  }

  // ---------- 数据备份（M13） ----------

  /** 导出布局 + 配置为备份文件 */
  async function handleExportBackup(): Promise<void> {
    try {
      const dest = await save({
        defaultPath: `homedesktop-backup-${new Date().toISOString().slice(0, 10)}.json`,
        filters: [{ name: "HomeDesktop 备份", extensions: ["json"] }],
      });
      if (!dest) return;
      await invoke("backup_export", { dest });
      log.info(`备份已导出: ${dest}`);
      toast("备份已导出");
    } catch (e) {
      log.error(`备份导出失败: ${e}`);
      toast(`导出失败：${e}`, 4000);
    }
  }

  /** 从备份文件恢复布局 + 配置（先选文件，再选"覆盖/合并"） */
  async function handleImportBackup(): Promise<void> {
    try {
      const src = await open({
        multiple: false,
        filters: [{ name: "HomeDesktop 备份", extensions: ["json"] }],
      });
      if (typeof src !== "string" || !src) return;
      pendingImportSrc = src;
    } catch (e) {
      log.error(`备份导入失败: ${e}`);
      toast(`导入失败：${e}`, 4000);
    }
  }

  /** 按所选模式执行导入（成功后自动重载） */
  async function runImport(mode: "overwrite" | "merge"): Promise<void> {
    if (!pendingImportSrc) return;
    const src = pendingImportSrc;
    pendingImportSrc = null;
    try {
      const summary = await invoke<{ pages: number; cells: number; configKeys: number }>(
        "backup_import",
        { src, mode },
      );
      log.info(
        `备份已导入(${mode === "merge" ? "合并" : "覆盖"}): ${src}（${summary.pages} 页 ${summary.cells} 单元）`,
      );
      toast(`导入成功（${mode === "merge" ? "合并" : "覆盖"}）：${summary.pages} 页，即将重载`);
      setTimeout(() => window.location.reload(), 1200);
    } catch (e) {
      log.error(`备份导入失败: ${src} -> ${e}`);
      toast(`导入失败：${e}`, 4000);
    }
  }

  function handleMove(iconId: string): void {
    moveTarget = moveTarget === iconId ? null : iconId;
  }

  function handleMoveToFolder(folderId: string): void {
    if (moveTarget && moveIconToFolder(moveTarget, folderId)) {
      persist();
      log.info(`移入文件夹: ${moveTarget} -> ${folderId}`);
    }
    moveTarget = null;
  }

  /** 文件夹内拖拽排序（M8） */
  function handleFolderReorder(
    dragId: string,
    targetId: string | null,
    pos: "before" | "after",
  ): void {
    const fid = openFolder.folderId;
    if (fid && reorderIconInFolder(fid, dragId, targetId, pos)) {
      persist();
      log.info(`文件夹内重排: ${dragId} ${pos} ${targetId ?? "末尾"}`);
    }
  }

  /** 打开文件夹编辑弹窗（M8：重命名/换 emoji） */
  function handleEditFolder(folderId: string): void {
    const f = findFolderById(folderId);
    if (!f) return;
    folderEditTarget = folderId;
    folderEditName = f.name;
    folderEditEmoji = f.emoji;
  }

  function findFolderById(folderId: string): Extract<Cell, { kind: "folder" }> | undefined {
    for (const page of layout.pages) {
      const c = page.find((x) => x.kind === "folder" && x.id === folderId);
      if (c?.kind === "folder") return c;
    }
    return undefined;
  }

  function handleSaveFolderEdit(): void {
    if (!folderEditTarget) return;
    const id = folderEditTarget;
    let changed = false;
    if (renameFolder(id, folderEditName)) changed = true;
    if (setFolderEmoji(id, folderEditEmoji)) changed = true;
    if (changed) persist();
    log.info(`编辑文件夹: ${id} 名称=${folderEditName} emoji=${folderEditEmoji}`);
    folderEditTarget = null;
  }

  /** 打开图标自定义弹窗（M9） */
  function handleEditIcon(iconId: string): void {
    const icon = findIconCell(iconId);
    if (!icon) return;
    iconEditTarget = iconId;
    iconEditTitle = icon.title;
    iconEditEmoji = icon.emoji ?? "";
    iconEditColor = icon.color ?? "";
    iconEditIconPath = icon.iconPath ?? "";
  }

  /** 保存图标自定义（重命名/emoji/颜色/借用系统图标），空值=清除回退默认 */
  function handleSaveIconEdit(): void {
    if (!iconEditTarget) return;
    const id = iconEditTarget;
    updateIconAppearance(id, {
      title: iconEditTitle,
      emoji: iconEditEmoji,
      color: iconEditColor,
      iconPath: iconEditIconPath,
    });
    persist();
    log.info(
      `自定义图标: ${id} 名称=${iconEditTitle} emoji=${iconEditEmoji || "(默认)"} ` +
        `颜色=${iconEditColor || "(默认)"} 图标=${iconEditIconPath || "(默认)"}`,
    );
    iconEditTarget = null;
  }

  /** 借用系统应用图标（M9）：面板选中后记录其路径，返回编辑弹窗 */
  function handleBorrowIcon(app: AppInfo): void {
    iconEditIconPath = app.path;
    showIconBorrow = false;
  }

  function handleResize(iconId: string): void {
    sizeTarget = sizeTarget === iconId ? null : iconId;
    settingsTarget = null;
  }

  function handleSettings(cellId: string): void {
    settingsTarget = settingsTarget === cellId ? null : cellId;
    sizeTarget = null;
  }

  function handleApplySize(size: { w: number; h: number }): void {
    if (sizeTarget && setCellSize(sizeTarget, size)) {
      persist();
      log.info(`调整单元尺寸: ${sizeTarget} -> ${size.w}x${size.h}`);
      toast(`尺寸已设为 ${size.w}×${size.h}`);
    }
    sizeTarget = null;
  }

  // ---------- 拖拽排序 ----------

  function handleReorder(dragId: string, targetId: string, pos: "before" | "after"): void {
    if (moveCellAcrossPages(dragId, targetId, pos)) {
      persist();
    }
  }

  function handleDropInto(dragId: string, folderId: string): void {
    if (moveIconToFolder(dragId, folderId)) {
      persist();
      log.info(`拖入文件夹: ${dragId} -> ${folderId}`);
    }
  }

  // ---------- 外观设置 ----------

  function handleSetBackground(bg: Background): void {
    setBackgroundForCurrentTheme(bg);
    if (bg.kind === "color") customColor = bg.value;
    void saveAppearance();
    log.info(`设置背景(${appearance.theme}主题): ${bg.kind}`);
  }

  function handleCustomColor(): void {
    setBackgroundForCurrentTheme({ kind: "color", value: customColor });
    void saveAppearance();
    log.info(`自定义背景色(${appearance.theme}主题): ${customColor}`);
  }

  function handleSetTheme(theme: "dark" | "light"): void {
    appearance.theme = theme;
    applyTheme(theme);
    customColor =
      getCurrentBackground().kind === "color" ? getCurrentBackground().value : "#10141c";
    void saveAppearance();
    log.info(`切换主题: ${theme}`);
  }

  function handleSetTileSize(size: number): void {
    appearance.tileSize = size;
    void saveAppearance();
    log.info(`设置图标大小: ${size}px`);
    toast(`图标大小 → ${size}px`);
  }

  async function handlePickWallpaper(): Promise<void> {
    const stored = await pickWallpaperImage();
    if (stored) {
      setBackgroundForCurrentTheme({ kind: "image", value: stored });
      void saveAppearance();
      log.info(`设置图片壁纸(${appearance.theme}主题): ${stored}`);
      toast(`壁纸已设置（${appearance.theme === "dark" ? "深色" : "浅色"}主题）`);
    } else {
      log.warn("选择壁纸被取消或失败");
    }
  }

  async function handleToggleAutostart(): Promise<void> {
    autoStart = !autoStart;
    await setAutostart(autoStart);
    log.info(`开机自启: ${autoStart ? "开启" : "关闭"}`);
  }

  /** 编辑模式开关（苹果风格：编辑中显示缩放/删除等操作） */
  function handleToggleEdit(): void {
    if (ui.editMode) {
      exitEditMode();
      log.info("退出编辑模式");
    } else {
      showAdd = false;
      showSettings = false;
      moveTarget = null;
      sizeTarget = null;
      enterEditMode();
      log.info("进入编辑模式");
    }
  }

  function handlePrev(): void {
    if (currentPage.index > 0) currentPage.index--;
  }

  function handleNext(): void {
    if (currentPage.index < layout.pages.length - 1) currentPage.index++;
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

  /** 弹出通用确认框（所有删除操作统一走这里） */
  function askConfirm(title: string, message: string, onConfirm: () => void): void {
    confirmAction = { title, message, onConfirm };
  }

  function runConfirm(): void {
    confirmAction?.onConfirm();
    confirmAction = null;
  }
</script>

<main
  class="shell"
  style="--tile-size: {appearance.tileSize}px; background: {backgroundCss(getCurrentBackground())};"
>
  <header class="topbar">
    <SearchBar />
    {#if ui.editMode}
      <button class="icon-btn edit-done" title="完成" onclick={handleToggleEdit}>✓ 完成</button>
    {:else}
      <button class="icon-btn" title="编辑模式" onclick={handleToggleEdit}>✎</button>
    {/if}
    <button class="icon-btn" title="外观设置" onclick={() => (showSettings = !showSettings)}>⚙</button>
    <button class="icon-btn" title="添加图标/小组件" onclick={() => (showAdd = !showAdd)}>＋</button>
    <button
      class="icon-btn"
      title={isFullscreen ? "退出全屏（Esc）" : "全屏 Pad"}
      onclick={() => void handleToggleFullscreen()}
    >⛶</button>
  </header>

  {#if error}
    <div class="toast">{error}</div>
  {/if}

  {#if showAdd}
    <AddMenu
      plugins={plugins}
      onadd={(p) => handleAdd(p)}
      onnewfolder={(name) => handleNewFolder(name)}
      oninstallplugin={handleInstallPlugin}
      onopenplugins={openPlugins}
      onclose={() => (showAdd = false)}
    />
  {/if}

  {#if showSystemApps}
    <SystemAppsPanel
      onadd={(app) => handlePickApp(app)}
      onclose={() => {
        systemAppsSource = null;
        showSystemApps = false;
      }}
    />
  {/if}

  {#if showSettings}
    <div
      class="overlay"
      role="button"
      aria-label="关闭"
      tabindex="-1"
      onclick={(e) => e.target === e.currentTarget && (showSettings = false)}
      onkeydown={(e) => e.key === "Escape" && (showSettings = false)}
    >
      <div class="settings">
        <div class="set-head">外观设置</div>

        <div class="set-section">
          <div class="set-label">主题</div>
          <div class="size-presets">
            <button
              class="size-btn"
              class:active={appearance.theme === "dark"}
              onclick={() => handleSetTheme("dark")}
            >🌙 深色</button>
            <button
              class="size-btn"
              class:active={appearance.theme === "light"}
              onclick={() => handleSetTheme("light")}
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
                onclick={() => handleSetBackground(p.bg)}
              ></button>
            {/each}
          </div>
          <div class="custom-color">
            <input type="color" bind:value={customColor} onchange={handleCustomColor} />
            <span>自定义颜色</span>
          </div>
          <button class="wallpaper-btn" onclick={handlePickWallpaper}>🖼 选择图片壁纸…</button>
          {#if getCurrentBackground().kind === "image"}
            <div class="wallpaper-hint">当前：{getCurrentBackground().value.split(/[\\/]/).pop()}</div>
          {/if}
        </div>

        <div class="set-section">
          <div class="set-label">系统</div>
          <button class="wallpaper-btn" onclick={handleToggleAutostart}>
            {autoStart ? "✅ 开机自启已开启（点击关闭）" : "⛔ 开机自启已关闭（点击开启）"}
          </button>
        </div>

        <div class="set-section">
          <div class="set-label">快捷键</div>
          <ShortcutRow
            label="显示 / 隐藏 Pad（全屏）"
            value={padShortcut}
            onchange={handleSetPadShortcut}
          />
          <ShortcutRow
            label="打开全局搜索"
            value={searchShortcut}
            onchange={handleSetSearchShortcut}
          />
        </div>

        <div class="set-section">
          <div class="set-label">插件</div>
          <button class="wallpaper-btn" onclick={openPlugins}>🧩 打开插件管理（已安装 / 市场）…</button>
        </div>

        <div class="set-section">
          <div class="set-label">数据备份</div>
          <div class="fe-borrow">
            <button class="fe-btn save" onclick={() => void handleExportBackup()}>📤 导出备份…</button>
            <button class="fe-btn cancel" onclick={() => void handleImportBackup()}>📥 导入备份…</button>
          </div>
        </div>

        <div class="set-section">
          <div class="set-label">图标大小</div>
          <div class="size-presets">
            {#each tileSizePresets as p (p.label)}
              <button
                class="size-btn"
                class:active={appearance.tileSize === p.value}
                onclick={() => handleSetTileSize(p.value)}
              >{p.label}</button>
            {/each}
          </div>
        </div>
      </div>
    </div>
  {/if}

  {#if moveTarget && !showAdd}
    <div
      class="overlay"
      role="button"
      aria-label="关闭"
      tabindex="-1"
      onclick={() => (moveTarget = null)}
      onkeydown={(e) => e.key === "Escape" && (moveTarget = null)}
    >
      <div class="move-menu">
        <div class="move-head">移入文件夹</div>
        {#each folders as f (f.id)}
          <button class="move-row" onclick={() => handleMoveToFolder(f.id)}>
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

  {#if sizeTarget && !showAdd}
    <div
      class="overlay"
      role="button"
      aria-label="关闭"
      tabindex="-1"
      onclick={() => (sizeTarget = null)}
      onkeydown={(e) => e.key === "Escape" && (sizeTarget = null)}
    >
      <div class="move-menu">
        <div class="move-head">设置尺寸（插件支持的尺寸档）</div>
        <div class="size-grid">
          {#each sizeOptions as opt (opt.w + "x" + opt.h)}
            <button class="size-opt" onclick={() => handleApplySize(opt)}>
              {opt.w}×{opt.h}
            </button>
          {/each}
        </div>
      </div>
    </div>
  {/if}

  {#if settingsPlugin && settingsPlugin.settings?.length}
    <PluginSettingsMenu
      plugin={settingsPlugin}
      cellId={settingsTarget ?? undefined}
      onclose={() => (settingsTarget = null)}
    />
  {/if}

  {#if folderEditTarget}
    <div
      class="overlay"
      role="button"
      aria-label="关闭"
      tabindex="-1"
      onclick={(e) => e.target === e.currentTarget && (folderEditTarget = null)}
      onkeydown={(e) => e.key === "Escape" && (folderEditTarget = null)}
    >
      <div class="move-menu folder-edit">
        <div class="move-head">编辑文件夹</div>
        <div class="fe-row">
          <span class="fe-label">名称</span>
          <input
            class="fe-input"
            type="text"
            placeholder="文件夹名称"
            bind:value={folderEditName}
          />
        </div>
        <div class="fe-row">
          <span class="fe-label">图标</span>
          <input
            class="fe-input fe-emoji"
            type="text"
            placeholder="📁"
            bind:value={folderEditEmoji}
          />
        </div>
        <div class="fe-presets">
          {#each ["📁", "🧰", "🎮", "📷", "🛠️", "💼", "🎵", "📚", "⭐", "🔥", "🎯", "🏠"] as e (e)}
            <button
              class="fe-preset"
              class:active={folderEditEmoji === e}
              onclick={() => (folderEditEmoji = e)}
            >{e}</button>
          {/each}
        </div>
        <div class="fe-actions">
          <button class="fe-btn cancel" onclick={() => (folderEditTarget = null)}>取消</button>
          <button class="fe-btn save" onclick={handleSaveFolderEdit}>保存</button>
        </div>
      </div>
    </div>
  {/if}

  {#if iconEditTarget}
    <div
      class="overlay"
      role="button"
      aria-label="关闭"
      tabindex="-1"
      onclick={(e) => e.target === e.currentTarget && (iconEditTarget = null)}
      onkeydown={(e) => e.key === "Escape" && (iconEditTarget = null)}
    >
      <div class="move-menu folder-edit">
        <div class="move-head">自定义图标</div>
        <div class="fe-row">
          <span class="fe-label">名称</span>
          <input
            class="fe-input"
            type="text"
            placeholder="图标名称"
            bind:value={iconEditTitle}
          />
        </div>
        <div class="fe-row">
          <span class="fe-label">图标</span>
          <input
            class="fe-input fe-emoji"
            type="text"
            placeholder="默认"
            bind:value={iconEditEmoji}
          />
          <span class="fe-hint">留空=插件默认</span>
        </div>
        <div class="fe-presets">
          {#each ["🎮", "📷", "🛠️", "💼", "🎵", "📚", "⭐", "🔥", "🎯", "🏠", "✏️", "💡"] as e (e)}
            <button
              class="fe-preset"
              class:active={iconEditEmoji === e}
              onclick={() => (iconEditEmoji = e)}
            >{e}</button>
          {/each}
        </div>
        <div class="fe-row">
          <span class="fe-label">颜色</span>
          <div class="fe-colors">
            {#each ["#e53935", "#fb8c00", "#fdd835", "#43a047", "#1e88e5", "#8e24aa", "#00897b", ""] as c (c || "none")}
              <button
                class="fe-color"
                class:active={iconEditColor === c}
                style={c ? `background: ${c};` : ""}
                title={c || "默认（透明）"}
                onclick={() => (iconEditColor = c)}
              >{c ? "" : "默认"}</button>
            {/each}
          </div>
        </div>
        <div class="fe-row">
          <span class="fe-label">图标</span>
          <div class="fe-borrow">
            <button class="fe-btn save" onclick={() => (showIconBorrow = true)}>🎨 借用系统应用图标…</button>
            {#if iconEditIconPath}
              <button class="fe-btn cancel" onclick={() => (iconEditIconPath = "")}>清除</button>
            {/if}
          </div>
        </div>
        <div class="fe-actions">
          <button class="fe-btn cancel" onclick={() => (iconEditTarget = null)}>取消</button>
          <button class="fe-btn save" onclick={handleSaveIconEdit}>保存</button>
        </div>
      </div>
    </div>
  {/if}

  {#if showIconBorrow && iconEditTarget}
    <SystemAppsPanel
      mode="borrow"
      onadd={(app) => handleBorrowIcon(app)}
      onclose={() => (showIconBorrow = false)}
    />
  {/if}

  {#if confirmAction}
    <div
      class="overlay"
      role="button"
      aria-label="关闭"
      tabindex="-1"
      onclick={(e) => e.target === e.currentTarget && (confirmAction = null)}
      onkeydown={(e) => e.key === "Escape" && (confirmAction = null)}
    >
      <div class="move-menu folder-edit">
        <div class="move-head">{confirmAction.title}</div>
        <div class="cf-text">{confirmAction.message}</div>
        <div class="fe-actions">
          <button class="fe-btn cancel" onclick={() => (confirmAction = null)}>取消</button>
          <button class="fe-btn save cf-danger" onclick={runConfirm}>删除</button>
        </div>
      </div>
    </div>
  {/if}

  {#if pendingImportSrc}
    <div
      class="overlay"
      role="button"
      aria-label="关闭"
      tabindex="-1"
      onclick={(e) => e.target === e.currentTarget && (pendingImportSrc = null)}
      onkeydown={(e) => e.key === "Escape" && (pendingImportSrc = null)}
    >
      <div class="move-menu folder-edit">
        <div class="move-head">导入备份方式</div>
        <div class="cf-text">覆盖 = 完全恢复成备份时的状态（当前新增的图标会消失）；合并 = 保留当前图标，只补上备份里有而当前没有的。</div>
        <div class="fe-actions">
          <button class="fe-btn cancel" onclick={() => (pendingImportSrc = null)}>取消</button>
          <button class="fe-btn save" onclick={() => void runImport("overwrite")}>覆盖导入</button>
          <button class="fe-btn save" onclick={() => void runImport("merge")}>合并导入</button>
        </div>
      </div>
    </div>
  {/if}

  {#if showSearch}
    <SearchPanel
      onclose={closeSearch}
      onopenicon={(id) => handleSearchOpenIcon(id)}
      onopenfolder={(id) => handleSearchOpenFolder(id)}
      onopenapp={(path) => void handleSearchOpenApp(path)}
      onopenplugin={(id) => void handleSearchOpenPlugin(id)}
    />
  {/if}

  {#if showPlugins}
    <PluginsPanel
      onclose={() => (showPlugins = false)}
      onupdate={handleInstallPlugin}
      onuninstall={(id, name) => handleUninstallPlugin(id, name)}
      onmessage={(msg, isError) => toast(msg, isError ? 4000 : 2500)}
    />
  {/if}

  <section class="content">
    {#if inFolder}
      <FolderView
        onaddclick={() => (showAdd = true)}
        onlaunch={(id) => handleLaunch(id)}
        onmove={(id) => handleMove(id)}
        onediticon={(id) => handleEditIcon(id)}
        ondelete={(fid, iid) => handleDeleteFolderItem(fid, iid)}
        onreorder={handleFolderReorder}
        onresize={(id) => handleResize(id)}
        onsettings={(id) => handleSettings(id)}
      />
    {:else}
      <Grid
        cells={layout.pages[currentPage.index] ?? []}
        queryText={query.text}
        onlaunch={(id) => handleLaunch(id)}
        ondelete={(id) => handleDelete(id)}
        onaddclick={() => (showAdd = true)}
        onopenfolder={(id) => (openFolder.folderId = id)}
        oneditfolder={(id) => handleEditFolder(id)}
        onediticon={(id) => handleEditIcon(id)}
        onmoveicon={(id) => handleMove(id)}
        onresize={(id) => handleResize(id)}
        onsettings={(id) => handleSettings(id)}
        onreorder={handleReorder}
        ondropinto={handleDropInto}
        onflipprev={handlePrev}
        onflipnext={handleNext}
      />
    {/if}
  </section>

  {#if !inFolder}
    <footer class="pager">
      {#if layout.pages.length > 1}
        <div class="pager-group">
          <button class="icon-btn" onclick={handlePrev} disabled={currentPage.index === 0}>‹</button>
          <span>{currentPage.index + 1} / {layout.pages.length}</span>
          <button class="icon-btn" onclick={handleNext} disabled={currentPage.index === layout.pages.length - 1}>›</button>
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
</main>

<style>
  .shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    padding: 18px 24px;
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
    z-index: 50;
  }
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    /* App 级弹层（设置/确认框/导入模式等）必须盖过组件级弹层
       （插件管理 55、搜索 60、插件设置 45、添加菜单 40）——确认框不能出现在后面 */
    z-index: 100;
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
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 13px;
    padding: 7px 10px;
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
  .fe-borrow {
    flex: 1;
    display: flex;
    gap: 8px;
  }
  .settings {
    width: 340px;
    max-height: 80vh;
    overflow-y: auto;
    background: var(--bg-elev);
    border-radius: 16px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
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
</style>
