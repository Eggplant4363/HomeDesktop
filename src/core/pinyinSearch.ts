// 拼音搜索匹配：输入拼音（如 "keting"、"kt"）也能命中中文名称（如"客厅灯"）
import { pinyin } from "pinyin-pro";

/** 是否含中文字符（非中文直接按原文匹配，跳过拼音转换省性能） */
export function hasChinese(s: string): boolean {
  return /[\u4e00-\u9fff]/.test(s);
}

const cache = new Map<string, { full: string; first: string }>();

/** 计算名称的拼音（无空格全文 + 首字母缩写），缓存加速 */
function pinyinKey(label: string): { full: string; first: string } {
  if (!hasChinese(label)) {
    const l = label.toLowerCase();
    return { full: l, first: l };
  }
  const hit = cache.get(label);
  if (hit) return hit;
  const full = pinyin(label, { toneType: "none", type: "array" })
    .join("")
    .toLowerCase();
  const first = pinyin(label, { pattern: "first", toneType: "none", type: "array" })
    .join("")
    .toLowerCase();
  const key = { full, first };
  cache.set(label, key);
  return key;
}

/** 名称是否命中查询 q（小写）：原文包含 / 拼音全文包含 / 拼音首字母包含 */
export function matchesPinyin(label: string, q: string): boolean {
  if (!q) return false;
  const l = label.toLowerCase();
  if (l.includes(q)) return true;
  const { full, first } = pinyinKey(label);
  return full.includes(q) || first.includes(q);
}
