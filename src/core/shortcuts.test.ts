// 快捷键工具测试（纯函数）
import { describe, expect, it } from "vitest";
import { comboFromEvent, comboLabel, normalizeCombo } from "./shortcuts";

/** Node 环境无 KeyboardEvent，用最小对象模拟（comboFromEvent 只读这几个字段） */
function ev(
  key: string,
  mods: { ctrl?: boolean; alt?: boolean; shift?: boolean; meta?: boolean } = {},
): KeyboardEvent {
  return {
    key,
    ctrlKey: mods.ctrl ?? false,
    altKey: mods.alt ?? false,
    shiftKey: mods.shift ?? false,
    metaKey: mods.meta ?? false,
    preventDefault: () => {},
    stopPropagation: () => {},
  } as unknown as KeyboardEvent;
}

describe("normalizeCombo", () => {
  it("去空格与小写化", () => {
    expect(normalizeCombo(" Alt + Space ")).toBe("alt+space");
    expect(normalizeCombo("CTRL+SHIFT+A")).toBe("ctrl+shift+a");
  });
});

describe("comboLabel", () => {
  it("显示格式化", () => {
    expect(comboLabel("alt+space")).toBe("Alt + Space");
    expect(comboLabel("ctrl+shift+a")).toBe("Ctrl + Shift + A");
    expect(comboLabel("f11")).toBe("F11");
  });
});

describe("comboFromEvent", () => {
  it("组合键解析", () => {
    expect(comboFromEvent(ev(" ", { alt: true }))).toBe("alt+space");
    expect(comboFromEvent(ev("a", { ctrl: true, shift: true }))).toBe("ctrl+shift+a");
    expect(comboFromEvent(ev("F11"))).toBe("f11");
    expect(comboFromEvent(ev("5", { ctrl: true }))).toBe("ctrl+5");
  });

  it("仅修饰键 / 特殊键不录入", () => {
    expect(comboFromEvent(ev("Control", { ctrl: true }))).toBeNull();
    expect(comboFromEvent(ev("Alt", { alt: true }))).toBeNull();
    expect(comboFromEvent(ev("Escape"))).toBeNull();
    expect(comboFromEvent(ev("ArrowUp"))).toBeNull();
    expect(comboFromEvent(ev("Enter"))).toBeNull();
  });
});
