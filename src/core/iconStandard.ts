// 统一插件图标设计标准：
// - 所有插件的图标同尺寸、占同样大小的空间
// - 图标+文字整体居中（上下/左右）
// - 图标与文字间距一致
// 所有组件（IconTile / HaWidget / 未来插件）都从这里取值，保证视觉统一

/** 图标/图形尺寸（随网格缩放，tileSize - 6，最小 38） */
export function iconGlyphSize(tile: number): number {
  return Math.max(38, tile - 6);
}

/** 图标与文字之间的距离（px，统一） */
export const ICON_TEXT_GAP = 12;

/** 图标占的空间（带内边距的容器，保证所有图标占位一致） */
export function iconBoxSize(tile: number): number {
  return iconGlyphSize(tile) + 8;
}

/** 内容顶部锚定的留白（所有组件一致，保证文字在同一水平线） */
export const ICON_TOP_PAD = 4;
