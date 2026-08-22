// 农历转换测试：用已知公历/农历对应日期验证
import { describe, expect, it } from "vitest";
import { solarToLunar, lunarFestival, isLunarNewYearsEve } from "./lunar";

function lunar(dateStr: string) {
  const [y, m, d] = dateStr.split("-").map(Number);
  return solarToLunar(new Date(y, m - 1, d));
}

describe("lunar: 农历转换", () => {
  it("2024-02-10 = 甲辰年 正月初一（春节）", () => {
    const l = lunar("2024-02-10");
    expect(l.yearName).toBe("甲辰年");
    expect(l.monthName).toBe("正月");
    expect(l.dayName).toBe("初一");
    expect(lunarFestival(l.month, l.day)).toBe("春节");
  });

  it("2024-09-17 = 八月十五（中秋）", () => {
    const l = lunar("2024-09-17");
    expect(l.monthName).toBe("八月");
    expect(l.dayName).toBe("十五");
    expect(lunarFestival(l.month, l.day)).toBe("中秋");
  });

  it("2025-01-29 = 乙巳年 正月初一（春节）", () => {
    const l = lunar("2025-01-29");
    expect(l.yearName).toBe("乙巳年");
    expect(l.monthName).toBe("正月");
    expect(l.dayName).toBe("初一");
  });

  it("2023-06-22 = 五月初五（端午）", () => {
    const l = lunar("2023-06-22");
    expect(l.monthName).toBe("五月");
    expect(l.dayName).toBe("初五");
    expect(lunarFestival(l.month, l.day)).toBe("端午");
  });

  it("2024-02-09 = 癸卯年 腊月三十（除夕）", () => {
    const l = lunar("2024-02-09");
    expect(l.month).toBe(12);
    expect(l.dayName).toBe("三十");
    expect(isLunarNewYearsEve(l.year, l.month, l.day)).toBe(true);
  });

  it("2000-02-05 = 庚辰年 正月初一", () => {
    const l = lunar("2000-02-05");
    expect(l.yearName).toBe("庚辰年");
    expect(l.monthName).toBe("正月");
    expect(l.dayName).toBe("初一");
  });
});
