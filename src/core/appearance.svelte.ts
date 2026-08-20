// 外观设置：主题（深/浅）、背景（按主题分别存储，Android 13+/iOS 同款方案）、图标基准尺寸
// 持久化于 config.json 的 "appearance" 键
import { convertFileSrc } from "@tauri-apps/api/core";
import { getConfig, setConfig } from "./config";

export type Background =
  | { kind: "color"; value: string }
  | { kind: "gradient"; value: string }
  | { kind: "image"; value: string }; // value = 本地文件绝对路径

export type Theme = "dark" | "light";

export const appearance = $state<{
  /** 深浅色各自独立的背景 */
  backgrounds: Record<Theme, Background>;
  tileSize: number;
  theme: Theme;
  /** 特效开关（设置 → 特效）：添加图标弹出动画、窗口显示/隐藏过渡等 */
  effects: { iconAdd: boolean; windowAnim: boolean };
}>({
  backgrounds: {
    dark: { kind: "color", value: "#10141c" },
    light: { kind: "color", value: "#eef1f6" },
  },
  tileSize: 84,
  theme: "dark",
  effects: { iconAdd: true, windowAnim: true },
});

/** 当前主题对应的背景（函数内读取 $state，模板中调用仍保持响应式） */
export function getCurrentBackground(): Background {
  return appearance.backgrounds[appearance.theme];
}

const KEY = "appearance";

function isBackground(v: unknown): v is Background {
  return (
    !!v &&
    typeof v === "object" &&
    ((v as Background).kind === "color" ||
      (v as Background).kind === "gradient" ||
      (v as Background).kind === "image")
  );
}

export async function loadAppearance(): Promise<void> {
  const raw = await getConfig(KEY);
  if (raw && typeof raw === "object") {
    const r = raw as {
      backgrounds?: Record<Theme, Background>;
      background?: Background; // 旧版单背景（迁移用）
      tileSize?: number;
      theme?: Theme;
      effects?: { iconAdd?: boolean; windowAnim?: boolean };
    };
    // 新版：按主题分背景
    if (r.backgrounds && isBackground(r.backgrounds.dark) && isBackground(r.backgrounds.light)) {
      appearance.backgrounds.dark = r.backgrounds.dark;
      appearance.backgrounds.light = r.backgrounds.light;
    } else if (isBackground(r.background)) {
      // 旧版迁移：单背景 → 深色；浅色用默认
      appearance.backgrounds.dark = r.background;
    }
    if (typeof r.tileSize === "number" && r.tileSize >= 56 && r.tileSize <= 120) {
      appearance.tileSize = r.tileSize;
    }
    if (r.theme === "dark" || r.theme === "light") {
      appearance.theme = r.theme;
    }
    if (r.effects && typeof r.effects.iconAdd === "boolean") {
      appearance.effects.iconAdd = r.effects.iconAdd;
    }
    if (r.effects && typeof r.effects.windowAnim === "boolean") {
      appearance.effects.windowAnim = r.effects.windowAnim;
    }
  }
  applyTheme();
}

export async function saveAppearance(): Promise<void> {
  await setConfig(KEY, {
    backgrounds: {
      dark: appearance.backgrounds.dark,
      light: appearance.backgrounds.light,
    },
    tileSize: appearance.tileSize,
    theme: appearance.theme,
    effects: appearance.effects,
  });
}

/** 设置当前主题的背景 */
export function setBackgroundForCurrentTheme(bg: Background): void {
  appearance.backgrounds[appearance.theme] = bg;
}

/** 应用主题到 <html data-theme> */
export function applyTheme(theme: Theme = appearance.theme): void {
  document.documentElement.dataset.theme = theme;
}

/** 背景 → CSS background 值 */
export function backgroundCss(bg: Background): string {
  if (bg.kind === "image") {
    return `url("${convertFileSrc(bg.value)}") center / cover no-repeat`;
  }
  return bg.value;
}

export const backgroundPresets: { label: string; bg: Background }[] = [
  { label: "深蓝", bg: { kind: "color", value: "#10141c" } },
  { label: "暮蓝", bg: { kind: "gradient", value: "linear-gradient(160deg, #1c2436, #10141c)" } },
  { label: "暖橙", bg: { kind: "gradient", value: "linear-gradient(160deg, #2b1a12, #10141c)" } },
  { label: "森林", bg: { kind: "gradient", value: "linear-gradient(160deg, #0f1c14, #10141c)" } },
  { label: "紫夜", bg: { kind: "gradient", value: "linear-gradient(160deg, #1a1228, #10141c)" } },
];

export const tileSizePresets: { label: string; value: number }[] = [
  { label: "小", value: 64 },
  { label: "中", value: 84 },
  { label: "大", value: 104 },
];
