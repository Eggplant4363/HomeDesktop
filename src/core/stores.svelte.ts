// 全局状态门面（阶段3 重构）
// 实际实现已按职责拆分：
//   layoutStore.svelte.ts  核心可变状态（plugins/layout/query/currentPage/openFolder/ui）+ 基础操作
//   cellOps.svelte.ts      画布适配、单元增删/位置/尺寸、重叠修复、外观、插件批量增删
//   folderOps.svelte.ts    文件夹查找/创建/删除/内部项增删改排/重命名/移入
// 本文件仅做聚合 re-export，保持既有 `import ... from "core/stores.svelte"` 路径不变。

export {
  plugins,
  layout,
  query,
  currentPage,
  openFolder,
  ui,
  enterEditMode,
  exitEditMode,
  cloneCell,
  setPlugins,
  setLayout,
  getDisplayPageCount,
  setDisplayPageCount,
  focusCell,
  getFocusCell,
  clearFocusCell,
  addPage,
  removePage,
} from "./layoutStore.svelte";

export {
  fitCellsToCols,
  addCell,
  removeCell,
  setCellPosition,
  setCellSize,
  repairLayoutForPersist,
  repairPageOverlaps,
  updateIconAppearance,
  replaceCellWithApp,
  removeCellsByPlugin,
  reorderCellById,
  moveCellAcrossPages,
} from "./cellOps.svelte";

export {
  findFolder,
  createFolder,
  deleteFolder,
  setIconPosition,
  repackFolder,
  addIconToFolder,
  removeIconFromFolder,
  reorderIconInFolder,
  renameFolder,
  setFolderEmoji,
  moveIconToFolder,
} from "./folderOps.svelte";
