// 快捷键工具（M7）：组合键规范化 / 显示格式化 / 从键盘事件解析（纯函数，便于单测）
// 存储与注册使用小写格式，如 "alt+space"、"ctrl+shift+a"、"f11"

/** 规范化：" Alt + Space " → "alt+space" */
export function normalizeCombo(spec: string): string {
  return spec.trim().toLowerCase().replace(/\s+/g, "");
}

/** 显示格式化："alt+space" → "Alt + Space" */
export function comboLabel(spec: string): string {
  const parts = normalizeCombo(spec)
    .split("+")
    .filter(Boolean)
    .map((p) => (p === "space" ? "Space" : p.charAt(0).toUpperCase() + p.slice(1)));
  return parts.join(" + ");
}

/** 从键盘事件构建组合键（"ctrl+alt+a"）；仅修饰键/不支持键返回 null */
export function comboFromEvent(e: KeyboardEvent): string | null {
  const mods: string[] = [];
  if (e.ctrlKey) mods.push("ctrl");
  if (e.altKey) mods.push("alt");
  if (e.shiftKey) mods.push("shift");
  if (e.metaKey) mods.push("super");

  const key = e.key.toLowerCase();
  let main: string | null = null;
  if (key === " " || key === "spacebar") main = "space";
  else if (key.length === 1 && /[a-z0-9]/.test(key)) main = key;
  else if (/^f\d{1,2}$/.test(key)) main = key;
  if (!main) return null; // 单独按修饰键 / 特殊键（Esc、方向键等）不录入

  return [...mods, main].join("+");
}
