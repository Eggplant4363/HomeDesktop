// 统一插件图标设计标准：
// - 所有插件的图标同尺寸、占同样大小的空间（图标框 = iconGlyphSize）
// - 图标+文字整体居中（上下/左右）
// - 图标与文字间距一致（ICON_TEXT_GAP）
// - 文字字号/字重/颜色一致（ICON_LABEL_*），保证字体高度对齐
// 所有组件（IconTile / FolderTile / HaWidget / 未来插件）都从这里取值，保证视觉统一

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

/** 标准标签字号 / 字重（所有磁贴文字高度对齐） */
export const ICON_LABEL_SIZE = 13;
export const ICON_LABEL_WEIGHT = 500;

/** emoji 图标字号（与应用图标同尺寸：0.6 倍显得小，与真实图标不一致） */
export function iconEmojiFontSize(tile: number): number {
  return iconGlyphSize(tile);
}

/** 图标框圆角（统一） */
export function iconRadius(tile: number): number {
  return Math.round(iconGlyphSize(tile) * 0.27);
}
