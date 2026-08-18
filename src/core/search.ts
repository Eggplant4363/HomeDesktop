// 搜索过滤纯函数（便于单元测试）
import type { Cell, IconCell, PluginInfo } from "./types";

/** 主网格搜索：图标按标题/插件名，文件夹按名称/emoji */
export function filterCells(
  cells: Cell[],
  plugins: PluginInfo[],
  queryText: string,
): Cell[] {
  if (!queryText) return cells;
  const q = queryText.toLowerCase();
  return cells.filter((cell) => {
    if (cell.kind === "folder") {
      return (
        cell.name.toLowerCase().includes(q) ||
        cell.emoji.toLowerCase().includes(q)
      );
    }
    const p = plugins.find((x) => x.id === cell.pluginId);
    return (
      cell.title.toLowerCase().includes(q) ||
      (p?.name ?? "").toLowerCase().includes(q)
    );
  });
}

/** 文件夹内图标搜索（复用标题/插件名匹配；保留完整 IconCell 结构） */
export function filterIcons(
  items: IconCell[],
  plugins: PluginInfo[],
  queryText: string,
): IconCell[] {
  if (!queryText) return items;
  const q = queryText.toLowerCase();
  return items.filter((it) => {
    const p = plugins.find((x) => x.id === it.pluginId);
    return (
      it.title.toLowerCase().includes(q) ||
      (p?.name ?? "").toLowerCase().includes(q)
    );
  });
}
