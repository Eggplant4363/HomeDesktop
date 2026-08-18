// 日历工具测试
import { describe, expect, it } from "vitest";
import { getMonthGrid } from "./calendar";

describe("getMonthGrid", () => {
  it("2026-02（1 号是周日，28 天）→ 4 行 × 7 列，无占位", () => {
    const weeks = getMonthGrid(2026, 2);
    expect(weeks.length).toBe(4);
    expect(weeks.every((w) => w.length === 7)).toBe(true);
    expect(weeks[0].every((d) => d !== null)).toBe(true);
    expect(weeks[3][6]).toBe(28);
  });

  it("2026-01（1 号是周四）→ 首行有 4 个 null 占位", () => {
    const weeks = getMonthGrid(2026, 1);
    // 2026-01-01 是周四（getDay()=4）
    expect(weeks[0].slice(0, 4)).toEqual([null, null, null, null]);
    expect(weeks[0][4]).toBe(1);
  });

  it("2024-02（闰年 29 天）", () => {
    const weeks = getMonthGrid(2024, 2);
    const days = weeks.flat().filter((d) => d !== null);
    expect(days.length).toBe(29);
    expect(days[days.length - 1]).toBe(29);
  });
});
