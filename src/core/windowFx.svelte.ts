// 窗口与转场特效（从 App.svelte 拆出，阶段1重构）
// - 窗口显示/隐藏动画（macOS 风格淡出缩放），供点击图标启动外部程序后调用
// - 页面左右滑动切换动画（鼠标拖拽 / 滚轮 / 触屏）
// - windowLike：与应用窗口显隐一致的转场（文件夹打开等复用）

import { getCurrentWindow } from "@tauri-apps/api/window";
import { appearance } from "./appearance.svelte";
import { currentPage } from "./stores.svelte";
import { log } from "./logger";

/** 窗口显示/隐藏动画状态（.anim-hidden → 淡出缩放；移除 → 淡入缩放） */
export const winFx = $state({
  hidden: false,
  /** 当前总页数（App 计算后写入，供滑动切换判断边界） */
  totalPages: 1,
});

let hideTimer: ReturnType<typeof setTimeout> | undefined;

/** cubic-bezier(0.25, 0.1, 0.25, 1)（CSS ease）求值，与 .shell 窗口过渡一致 */
export function easeBezier(t: number): number {
  const x1 = 0.25, y1 = 0.1, x2 = 0.25, y2 = 1;
  const cx = 3 * x1, bx = 3 * (x2 - x1) - cx, ax = 1 - cx - bx;
  const cy = 3 * y1, by = 3 * (y2 - y1) - cy, ay = 1 - cy - by;
  const sample = (a: number, b: number, c: number, u: number) => ((a * u + b) * u + c) * u;
  const deriv = (a: number, b: number, c: number, u: number) => (3 * a * u + 2 * b) * u + c;
  if (t <= 0) return 0;
  if (t >= 1) return 1;
  let x = t;
  for (let i = 0; i < 8; i++) {
    const err = sample(ax, bx, cx, x) - t;
    if (Math.abs(err) < 1e-6) break;
    x -= err / deriv(ax, bx, cx, x);
  }
  return sample(ay, by, cy, x);
}

/** 与应用窗口显示/隐藏一致的过渡：淡入淡出 + 轻微缩放（0.96 → 1） */
export function windowLike(node: HTMLElement) {
  void node;
  return {
    duration: 200,
    easing: easeBezier,
    css: (t: number) => `opacity: ${t}; transform: scale(${0.96 + 0.04 * t})`,
  };
}

/** 隐藏本应用窗口：特效开启时先播 200ms 淡出缩放再真正隐藏 */
export function hideWindow(): void {
  if (!appearance.effects.windowAnim) {
    const win = getCurrentWindow();
    win
      .hide()
      .then(() => log.info("隐藏应用窗口"))
      .catch((e) => log.error(`隐藏窗口失败: ${e}`));
    return;
  }
  winFx.hidden = true;
  if (hideTimer) clearTimeout(hideTimer);
  hideTimer = setTimeout(() => {
    hideTimer = undefined;
    const win = getCurrentWindow();
    win
      .hide()
      .then(() => log.info("隐藏应用窗口（动画后）"))
      .catch((e) => log.error(`隐藏窗口失败: ${e}`));
  }, 200);
}

/** 显示窗口（Rust 托盘/快捷键已 show，这里只负责淡入） */
export function showWindow(): void {
  winFx.hidden = false;
  log.debug("显示应用窗口（淡入）");
}

// ---------- 页面左右滑动切换（鼠标左键 / 触屏 / 滚轮） ----------

/** 滑动容器实时位移（px）与过渡 */
export const slide = $state({
  x: 0,
  transition: "none",
});

let slideWrap: HTMLElement | undefined;
let slideAnimating = false;

/** 绑定滑动容器元素（供测量宽度） */
export function bindSlideWrap(el: HTMLElement | undefined): void {
  slideWrap = el;
}

/** 拖动中：跟随手指位移（阻尼已在 Grid 处理） */
export function onSwipeMove(dx: number): void {
  if (slideAnimating) return;
  slide.x = dx;
}

/** 松手：达阈值 → 两段式滑出/滑入切换页；未达 → 回弹 */
export function onSwipeEnd(dx: number): void {
  if (slideAnimating) return;
  slideAnimating = true;
  const width = slideWrap?.clientWidth || window.innerWidth;
  const th = Math.max(50, width * 0.18);
  const dir =
    dx <= -th && currentPage.index < winFx.totalPages - 1
      ? 1
      : dx >= th && currentPage.index > 0
        ? -1
        : 0;
  const ease = "transform 0.22s cubic-bezier(0.25, 0.1, 0.25, 1)";
  if (dir === 0) {
    slide.transition = ease;
    slide.x = 0;
    setTimeout(() => {
      slide.transition = "none";
      slideAnimating = false;
    }, 240);
    return;
  }
  const from = currentPage.index;
  // 第一段：当前页滑出
  slide.transition = ease;
  slide.x = -dir * width;
  setTimeout(() => {
    currentPage.index += dir;
    log.info(`滑动切换页: ${from + 1} → ${currentPage.index + 1}`);
    // 第二段：新页从对面滑入
    slide.transition = "none";
    slide.x = dir * width;
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        slide.transition = ease;
        slide.x = 0;
        setTimeout(() => {
          slide.transition = "none";
          slideAnimating = false;
        }, 260);
      });
    });
  }, 230);
}
