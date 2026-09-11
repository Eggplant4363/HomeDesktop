// 共用：磁贴右下角 ⤡ 拖拽缩放逻辑（阶段5 从 IconTile/WidgetTile/FolderTile 抽取）
// 三个磁贴此前各自复制了约 40 行同样的指针处理，这里集中一处，行为保持一致：
// - 按下时读取尺寸、从 CSS 变量 --tile-size / --gap 计算每格像素（SLOT）
// - 拖动按整格吸附（IconTile/WidgetTile 支持 0.5 格步进，FolderTile 为整格）
// - 指针事件用 setPointerCapture，保证拖出元素仍能收到 move/up

/** 缩放回调集合 */
export interface TileResizeHandlers {
  /** 实时尺寸变化（拖动过程中） */
  onresizeto?: (id: string, w: number, h: number) => void;
  /** 松手（用于一次性持久化） */
  onresizeend?: (id: string) => void;
}

/** 尺寸取值来源：IconCell/WidgetTile 用 size，FolderTile 用 w/h */
export interface TileResizeTarget {
  id: string;
  /** 当前宽（格） */
  w: number;
  /** 当前高（格） */
  h: number;
}

/**
 * 创建缩放处理器。
 * @param getTarget 返回当前被缩放单元（每次按下时读取，保证拿到最新值）
 * @param opts.step   吸附步进（0.5 = 半格；默认 1 = 整格）
 * @param opts.snapMin/snapMax 尺寸上下限（默认 1..8）
 */
export function createTileResize(
  getTarget: () => TileResizeTarget | undefined,
  handlers: TileResizeHandlers,
  opts: { step?: number; snapMin?: number; snapMax?: number } = {},
) {
  const step = opts.step ?? 1;
  const min = opts.snapMin ?? 1;
  const max = opts.snapMax ?? 8;

  let resizing = $state(false);
  let rsx = 0;
  let rsy = 0;
  let rsw = 0;
  let rsh = 0;
  let rSlot = 120;

  /** 读取磁贴尺寸与间距，算出"一格"的像素跨度 */
  function measureSlot(el: HTMLElement): void {
    const style = getComputedStyle(el);
    const t = parseFloat(style.getPropertyValue("--tile-size"));
    const g = parseFloat(style.getPropertyValue("--gap"));
    if (Number.isFinite(t) && t > 0) rSlot = t + (Number.isFinite(g) && g >= 0 ? g : 0);
  }

  function startResize(e: PointerEvent): void {
    const target = getTarget();
    if (!target || !handlers.onresizeto) return;
    e.preventDefault();
    e.stopPropagation();
    resizing = true;
    rsx = e.clientX;
    rsy = e.clientY;
    rsw = target.w;
    rsh = target.h;
    const el = e.currentTarget as HTMLElement;
    measureSlot(el);
    try {
      el.setPointerCapture(e.pointerId);
    } catch {
      /* 忽略 */
    }
  }

  function onResizeMove(e: PointerEvent): void {
    if (!resizing) return;
    const target = getTarget();
    if (!target) return;
    e.preventDefault();
    // 按 step 吸附（0.5 步进时取半步）
    const snap = (n: number) => {
      const v = Math.round(n / step) * step;
      return Math.max(min, Math.min(max, v));
    };
    const nw = snap(rsw + (e.clientX - rsx) / rSlot);
    const nh = snap(rsh + (e.clientY - rsy) / rSlot);
    handlers.onresizeto?.(target.id, nw, nh);
  }

  function endResize(e: PointerEvent): void {
    if (!resizing) return;
    resizing = false;
    const target = getTarget();
    try {
      (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      /* 忽略 */
    }
    if (target) handlers.onresizeend?.(target.id);
  }

  return {
    /** 是否正在缩放（可用于加样式） */
    get resizing() {
      return resizing;
    },
    startResize,
    onResizeMove,
    endResize,
  };
}
