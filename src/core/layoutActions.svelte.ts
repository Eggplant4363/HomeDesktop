// 布局与内容操作（从 App.svelte 拆出，阶段1重构）
// 承载"增删改图标/文件夹/页面、拖放落点、尺寸、启动、插件安装卸载、备份"等动作，
// 统一在此协调 stores 层的布局变更与持久化（persist）。

import { open, save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import {
  addCell,
  addIconToFolder,
  createFolder,
  currentPage,
  layout,
  moveIconToFolder,
  openFolder,
  plugins,
  removeCell,
  removeCellsByPlugin,
  removeIconFromFolder,
  renameFolder,
  replaceCellWithApp,
  setFolderEmoji,
  setCellPosition,
  setCellSize,
  setIconPosition,
  repairPageOverlaps,
  repackFolder,
  updateIconAppearance,
} from "./stores.svelte";
import { saveLayout } from "./persistence";
import { installPlugin, loadPlugins, launchCell } from "./pluginLoader";
import type { AppInfo } from "./pluginLoader";
import { getWidgetDef } from "../widgets";
import { log } from "./logger";
import { peekCellSetting, setCellSetting } from "./pluginSettings.svelte";
import { getWidgetData } from "./widgetRuntime.svelte";
import { editor, askConfirm, flashAdded, shatterThen, toast } from "./editorState.svelte";
import { hideWindow } from "./windowFx.svelte";

/** 持久化当前布局 */
export function persist(): void {
  void saveLayout(layout);
}

// ---------- 查找 ----------

/** 在页面级查找单元格（不查文件夹内） */
export function findPageCellById(cellId: string) {
  for (const page of layout.pages) {
    const c = page.find((x) => x.id === cellId);
    if (c) return c;
  }
  return undefined;
}

/** 按单元格 id 在所有页面与文件夹内查找图标 */
export function findIconCell(cellId: string) {
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

/** 查找文件夹单元格 */
export function findFolderById(folderId: string) {
  for (const page of layout.pages) {
    const cell = page.find((c) => c.kind === "folder" && c.id === folderId);
    if (cell && cell.kind === "folder") return cell;
  }
  return undefined;
}

// ---------- 添加 ----------

/** 添加插件图标/小组件（在文件夹内则加入文件夹，否则找空位并自动跳页） */
export function addPluginCell(plugin: {
  id: string;
  name: string;
  pluginType: string;
  widgetComponent?: string;
  sizes?: { w: number; h: number }[];
  settings?: { key: string; default?: unknown }[];
}): void {
  try {
    const def = plugin.pluginType === "widget" ? getWidgetDef(plugin.widgetComponent) : undefined;
    const icon = {
      kind: "icon" as const,
      id: crypto.randomUUID(),
      pluginId: plugin.id,
      title: plugin.name,
      size: plugin.sizes?.[0] ?? def?.defaultSize ?? { w: 1, h: 1 },
    };
    if (openFolder.folderId) {
      addIconToFolder(openFolder.folderId, icon);
    } else {
      const placedPage = addCell(icon);
      if (placedPage !== currentPage.index) {
        currentPage.index = placedPage;
        log.info(`新图标在第 ${placedPage + 1} 页，已自动跳转`);
      }
      flashAdded(icon.id);
    }
    persist();
    editor.showAdd = false;
    log.info(`添加图标: ${plugin.name} (${icon.size.w}x${icon.size.h})`);
    // 网页快捷方式：自动获取网站标题作为图标名
    const urlDefault = plugin.settings?.find((s) => s.key === "url")?.default;
    if (urlDefault) void refreshWebTitle(icon.id, String(urlDefault));
  } catch (e) {
    log.error(`添加图标失败: ${plugin.id} -> ${e}`);
    toast(`添加失败：${e}`);
  }
}

/** 网页快捷方式：拉取网站标题并设为图标名（失败保持原名称） */
export async function refreshWebTitle(cellId: string, url: string): Promise<void> {
  try {
    const title = await invoke<string | null>("web_fetch_title", { url });
    if (title && title.trim()) {
      updateIconAppearance(cellId, { title: title.trim() });
      persist();
      log.info(`网页标题: ${title.trim()} (${url})`);
    }
  } catch (e) {
    log.error(`获取网页标题失败: ${url} -> ${e}`);
  }
}

/** 新建文件夹（若正在"移入文件夹"流程则顺手移入） */
export function newFolder(name: string): void {
  const { id, page } = createFolder(name);
  persist();
  if (editor.moveTarget) {
    moveIconToFolder(editor.moveTarget, id);
    editor.moveTarget = null;
    persist();
  }
  editor.showAdd = false;
  if (page !== currentPage.index) {
    currentPage.index = page;
    log.info(`新文件夹在第 ${page + 1} 页，已自动跳转`);
  }
  flashAdded(id);
  log.info(`新建文件夹: ${name} (${id})`);
}

// ---------- 编辑 ----------

/** 打开文件夹编辑弹窗 */
export function editFolder(folderId: string): void {
  const cell = findFolderById(folderId);
  if (!cell) return;
  editor.folderEditTarget = folderId;
  editor.folderEditName = cell.name;
  editor.folderEditEmoji = cell.emoji;
}

/** 保存文件夹改名/换 emoji */
export function saveFolderEdit(): void {
  const id = editor.folderEditTarget;
  if (!id) return;
  let changed = false;
  if (renameFolder(id, editor.folderEditName)) changed = true;
  if (setFolderEmoji(id, editor.folderEditEmoji)) changed = true;
  if (changed) persist();
  log.info(`编辑文件夹: ${id} 名称=${editor.folderEditName} emoji=${editor.folderEditEmoji}`);
  editor.folderEditTarget = null;
}

/** 打开图标自定义弹窗（含小组件的当前显示名预填） */
export function editIcon(iconId: string): void {
  const icon = findIconCell(iconId);
  if (!icon) return;
  editor.iconEditTarget = iconId;
  editor.iconEditIsWidget = plugins.find((p) => p.id === icon.pluginId)?.pluginType === "widget";
  editor.iconEditTitle = icon.title;
  if (editor.iconEditIsWidget) {
    const custom = peekCellSettingName(iconId, icon.pluginId);
    if (custom) editor.iconEditTitle = custom;
    else {
      const cached = getWidgetDataName(iconId);
      if (cached) editor.iconEditTitle = cached;
    }
  }
  editor.iconEditEmoji = icon.emoji ?? "";
  editor.iconEditColor = icon.color ?? "";
  editor.iconEditIconPath = icon.iconPath ?? "";
  editor.iconEditIconImage = icon.iconImage ?? "";
  editor.iconEditShowLabel = icon.showLabel !== false;
}

/** 保存图标自定义（名称/emoji/颜色/图片/借用图标/是否显示名称） */
export async function saveIconEdit(): Promise<void> {
  const id = editor.iconEditTarget;
  if (!id) return;
  const icon = findIconCell(id);
  const isWidget =
    !!icon && plugins.find((p) => p.id === icon.pluginId)?.pluginType === "widget";
  updateIconAppearance(id, {
    title: editor.iconEditTitle,
    emoji: editor.iconEditEmoji,
    color: editor.iconEditColor,
    iconPath: editor.iconEditIconPath,
    iconImage: editor.iconEditIconImage,
    showLabel: editor.iconEditShowLabel,
  });
  if (isWidget && icon) {
    // 小组件自定义名称写入实例设置（空=恢复默认）
    await setCellSettingName(id, icon.pluginId, editor.iconEditTitle.trim());
  }
  persist();
  log.info(
    `自定义图标: ${id} 名称=${editor.iconEditTitle} emoji=${editor.iconEditEmoji || "(默认)"} ` +
      `颜色=${editor.iconEditColor || "(默认)"} 图标=${editor.iconEditIconPath || "(默认)"}`,
  );
  editor.iconEditTarget = null;
}

// ---------- 删除 ----------

/** 删除图标/文件夹（含确认 + 删除动效） */
export function deleteCell(id: string): void {
  const cell = findPageCellById(id);
  if (!cell) return;
  if (cell.kind === "folder") {
    askConfirm(
      `删除文件夹「${cell.name}」？`,
      `文件夹内 ${cell.items.length} 个图标将一并删除，且无法撤销。`,
      () =>
        shatterThen(id, () => {
          removeCell(id);
          persist();
          log.info(`删除文件夹: ${id}`);
        }),
    );
  } else {
    askConfirm(`删除图标「${cell.title}」？`, "删除后无法撤销。", () =>
      shatterThen(id, () => {
        removeCell(id);
        persist();
        log.info(`删除图标: ${id}`);
      }),
    );
  }
}

/** 删除文件夹内图标（带确认 + 删除动效） */
export function deleteFolderItem(folderId: string, iconId: string): void {
  const folder = findFolderById(folderId);
  const icon = folder?.items.find((i) => i.id === iconId);
  askConfirm(`删除「${icon?.title ?? iconId}」？`, "该图标将从文件夹中删除，且无法撤销。", () =>
    shatterThen(iconId, () => {
      removeIconFromFolder(folderId, iconId);
      persist();
      log.info(`删除文件夹内图标: ${folderId}`);
    }),
  );
}

// ---------- 启动 ----------

/** 启动单元格（含内置「系统应用」槽位拦截） */
export function launch(cellId: string): void {
  const icon = findIconCell(cellId);
  const plugin = icon ? plugins.find((p) => p.id === icon.pluginId) : undefined;
  const kind = icon?.action?.kind ?? plugin?.actions?.[0]?.kind;
  if (kind === "system_apps") {
    log.info(`打开系统应用面板: ${cellId}`);
    editor.showAdd = false;
    editor.systemAppsSource = cellId;
    editor.showSystemApps = true;
    return;
  }
  log.info(`启动图标: ${cellId}`);
  launchCell(cellId)
    .then(() => hideWindow())
    .catch((e) => {
      log.error(`启动失败: ${cellId} -> ${e}`);
      toast(String(e));
    });
}

/** 选中应用/文件/文件夹：原位替换「系统应用」槽位图标 */
export function pickApp(app: AppInfo & { emoji?: string; iconPath?: string }): void {
  const src = editor.systemAppsSource;
  if (src && replaceCellWithApp(src, app, app.emoji, app.iconPath)) {
    persist();
    log.info(`槽位替换: ${src} -> ${app.name} (${app.path})`);
    toast(`已添加：${app.name}`);
  } else {
    toast("未找到「系统应用」图标，请重新添加");
  }
  editor.systemAppsSource = null;
  editor.showSystemApps = false;
}

// ---------- 打开文件夹 ----------

/** 打开文件夹：内部图标下从左下角紧凑排列并持久化 */
export function openFolderById(folderId: string): void {
  openFolder.folderId = folderId;
  if (repackFolder(folderId)) persist();
}

// ---------- 移动 / 拖放 / 尺寸 ----------

export function toggleMoveTarget(iconId: string): void {
  editor.moveTarget = editor.moveTarget === iconId ? null : iconId;
}

export function moveToFolder(folderId: string): void {
  if (editor.moveTarget && moveIconToFolder(editor.moveTarget, folderId)) {
    persist();
    log.info(`移入文件夹: ${editor.moveTarget} -> ${folderId}`);
  }
  editor.moveTarget = null;
}

export function folderDropAt(folderId: string, iconId: string, x: number, y: number): void {
  if (setIconPosition(folderId, iconId, x, y)) {
    persist();
    log.info(`文件夹内摆放: ${iconId} -> (${x}, ${y})`);
  }
}

/** 页面级落点：目标槽被占用自动交换；多格重叠残留 → 修复到空闲位 */
export function dropAt(dragId: string, x: number, y: number): void {
  if (setCellPosition(dragId, x, y)) {
    if (repairPageOverlaps(currentPage.index)) log.info(`落点后修复重叠: ${dragId}`);
    persist();
    log.info(`摆放: ${dragId} -> (${x}, ${y})`);
  }
}

export function dropIntoFolder(dragId: string, folderId: string): void {
  if (moveIconToFolder(dragId, folderId)) {
    persist();
    log.info(`拖入文件夹: ${dragId} -> ${folderId}`);
  }
}

/** 拖拽手柄实时调尺寸（不持久化，松手时保存） */
export function resizeTo(iconId: string, w: number, h: number): void {
  setCellSize(iconId, { w, h });
}

export function resizeEnd(iconId: string): void {
  if (repairPageOverlaps(currentPage.index)) log.info(`改尺寸后修复重叠: ${iconId}`);
  persist();
  log.info(`拖拽调整尺寸完成: ${iconId}`);
}

export function toggleSettingsTarget(cellId: string): void {
  editor.settingsTarget = editor.settingsTarget === cellId ? null : cellId;
  editor.sizeTarget = null;
}

// ---------- 页面 ----------

export function prevPage(): void {
  if (currentPage.index > 0) currentPage.index--;
}

export function nextPage(totalPages: number): void {
  if (currentPage.index < totalPages - 1) currentPage.index++;
}

// ---------- 插件安装/卸载 ----------

export async function installPluginFromFile(): Promise<void> {
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

/** 卸载插件：确认 → 删目录 → 清理桌面图标 → 刷新列表 */
export function uninstallPlugin(pluginId: string, name: string): void {
  askConfirm(`卸载插件「${name}」？`, "将从磁盘删除该插件，桌面上它的图标也会被移除。", () => {
    void (async () => {
      try {
        await invoke("plugins_uninstall", { pluginId });
        const removed = removeCellsByPlugin(pluginId);
        persist();
        await loadPlugins();
        log.info(`插件已卸载: ${pluginId}（清理桌面图标 ${removed} 个）`);
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

// ---------- 备份 ----------

export async function exportBackup(): Promise<void> {
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

export async function pickImportBackup(): Promise<void> {
  try {
    const src = await open({
      multiple: false,
      filters: [{ name: "HomeDesktop 备份", extensions: ["json"] }],
    });
    if (typeof src !== "string" || !src) return;
    editor.pendingImportSrc = src;
  } catch (e) {
    log.error(`备份导入失败: ${e}`);
    toast(`导入失败：${e}`, 4000);
  }
}

export async function runImport(mode: "overwrite" | "merge"): Promise<void> {
  const src = editor.pendingImportSrc;
  if (!src) return;
  editor.pendingImportSrc = null;
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

// ---------- 与设置/小组件运行时的桥接 ----------

/** 读取小组件实例的自定义名称（cell.<id>.name） */
function peekCellSettingName(cellId: string, pluginId: string): string {
  return peekCellSetting<string>(cellId, pluginId, "name") ?? "";
}

/** 写入小组件实例的自定义名称（空字符串=恢复默认） */
async function setCellSettingName(cellId: string, pluginId: string, name: string): Promise<void> {
  await setCellSetting(cellId, pluginId, "name", name);
}

/** 从小组件运行时缓存取第一个实体的友好名（用作名称预填） */
function getWidgetDataName(cellId: string): string {
  const cached = getWidgetData<{ friendlyName?: string | null }[]>(cellId)?.[0];
  return cached?.friendlyName ?? "";
}
