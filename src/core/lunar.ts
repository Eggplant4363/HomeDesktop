// 农历（阴历）转换：公历 → 农历，经典查表法（1900-2100），纯函数便于单测
// 数据表：每年一个 16 位编码 —— 位 0-3 = 闰月月份（0=无闰月），位 4-15 = 每月大小（1=大月30天）

const LUNAR_INFO = [
  0x04bd8, 0x04ae0, 0x0a570, 0x054d5, 0x0d260, 0x0d950, 0x16554, 0x056a0, 0x09ad0, 0x055d2,
  0x04ae0, 0x0a5b6, 0x0a4d0, 0x0d250, 0x1d255, 0x0b540, 0x0d6a0, 0x0ada2, 0x095b0, 0x14977,
  0x04970, 0x0a4b0, 0x0b4b5, 0x06a50, 0x06d40, 0x1ab54, 0x02b60, 0x09570, 0x052f2, 0x04970,
  0x06566, 0x0d4a0, 0x0ea50, 0x06e95, 0x05ad0, 0x02b60, 0x186e3, 0x092e0, 0x1c8d7, 0x0c950,
  0x0d4a0, 0x1d8a6, 0x0b550, 0x056a0, 0x1a5b4, 0x025d0, 0x092d0, 0x0d2b2, 0x0a950, 0x0b557,
  0x06ca0, 0x0b550, 0x15355, 0x04da0, 0x0a5b0, 0x14573, 0x052b0, 0x0a9a8, 0x0e950, 0x06aa0,
  0x0aea6, 0x0ab50, 0x04b60, 0x0aae4, 0x0a570, 0x05260, 0x0f263, 0x0d950, 0x05b57, 0x056a0,
  0x096d0, 0x04dd5, 0x04ad0, 0x0a4d0, 0x0d4d4, 0x0d250, 0x0d558, 0x0b540, 0x0b6a0, 0x195a6,
  0x095b0, 0x049b0, 0x0a974, 0x0a4b0, 0x0b27a, 0x06a50, 0x06d40, 0x0af46, 0x0ab60, 0x09570,
  0x04af5, 0x04970, 0x064b0, 0x074a3, 0x0ea50, 0x06b58, 0x055c0, 0x0ab60, 0x096d5, 0x092e0,
  0x0c960, 0x0d954, 0x0d4a0, 0x0da50, 0x07552, 0x056a0, 0x0abb7, 0x025d0, 0x092d0, 0x0cab5,
  0x0a950, 0x0b4a0, 0x0baa4, 0x0ad50, 0x055d9, 0x04ba0, 0x0a5b0, 0x15176, 0x052b0, 0x0a930,
  0x07954, 0x06aa0, 0x0ad50, 0x05b52, 0x04b60, 0x0a6e6, 0x0a4e0, 0x0d260, 0x0ea65, 0x0d530,
  0x05aa0, 0x076a3, 0x096d0, 0x04afb, 0x04ad0, 0x0a4d0, 0x1d0b6, 0x0d250, 0x0d520, 0x0dd45,
  0x0b5a0, 0x056d0, 0x055b2, 0x049b0, 0x0a577, 0x0a4b0, 0x0aa50, 0x1b255, 0x06d20, 0x0ada0,
  0x14b63, 0x09370, 0x049f8, 0x04970, 0x064b0, 0x168a6, 0x0ea50, 0x06b20, 0x1a6c4, 0x0aae0,
  0x0a2e0, 0x0d2e3, 0x0c960, 0x0d557, 0x0d4a0, 0x0da50, 0x05d55, 0x056a0, 0x0a6d0, 0x055d4,
  0x052d0, 0x0a9b8, 0x0a950, 0x0b4a0, 0x0b6a6, 0x0ad50, 0x055a0, 0x0aba4, 0x0a5b0, 0x052b0,
  0x0b273, 0x06930, 0x07337, 0x06aa0, 0x0ad50, 0x14b55, 0x04b60, 0x0a570, 0x054e4, 0x0d160,
  0x0e968, 0x0d520, 0x0daa0, 0x16aa6, 0x056d0, 0x04ae0, 0x0a9d4, 0x0a2d0, 0x0d150, 0x0f252,
  0x0d520,
];

const MIN_YEAR = 1900;
const BASE_UTC = Date.UTC(1900, 0, 31); // 农历 1900 年正月初一

const MONTH_NAMES = ["正", "二", "三", "四", "五", "六", "七", "八", "九", "十", "冬", "腊"];
const DAY_NAMES = [
  "初一", "初二", "初三", "初四", "初五", "初六", "初七", "初八", "初九", "初十",
  "十一", "十二", "十三", "十四", "十五", "十六", "十七", "十八", "十九", "二十",
  "廿一", "廿二", "廿三", "廿四", "廿五", "廿六", "廿七", "廿八", "廿九", "三十",
];
const GAN = ["甲", "乙", "丙", "丁", "戊", "己", "庚", "辛", "壬", "癸"];
const ZHI = ["子", "丑", "寅", "卯", "辰", "巳", "午", "未", "申", "酉", "戌", "亥"];

function leapMonth(y: number): number {
  return LUNAR_INFO[y - MIN_YEAR] & 0xf;
}
function monthDays(y: number, m: number): number {
  return LUNAR_INFO[y - MIN_YEAR] & (0x10000 >> m) ? 30 : 29;
}
function leapDays(y: number): number {
  if (leapMonth(y)) return LUNAR_INFO[y - MIN_YEAR] & 0x10000 ? 30 : 29;
  return 0;
}
function lunarYearDays(y: number): number {
  let sum = 348; // 12 × 29
  for (let i = 0x8000; i > 0x8; i >>= 1) sum += LUNAR_INFO[y - MIN_YEAR] & i ? 1 : 0;
  return sum + leapDays(y);
}

export interface LunarDate {
  year: number;
  month: number; // 1-12
  /** 农历日（1-30） */
  day: number;
  leap: boolean;
  /** 农历年干支名，如 "乙巳年" */
  yearName: string;
  /** 农历月名，如 "六月"；闰月为 "闰六月" */
  monthName: string;
  /** 农历日名，如 "十五" */
  dayName: string;
}

/** 某农历年的月份序列（闰月插入其基准月之后），含每月天数 */
function monthsOfYear(year: number): { month: number; leap: boolean; days: number }[] {
  const leap = leapMonth(year);
  const list: { month: number; leap: boolean; days: number }[] = [];
  for (let m = 1; m <= 12; m++) {
    if (leap > 0 && m === leap) {
      list.push({ month: m, leap: true, days: leapDays(year) });
    }
    list.push({ month: m, leap: false, days: monthDays(year, m) });
  }
  return list;
}

/** 公历 → 农历 */
export function solarToLunar(date: Date): LunarDate {
  let days = Math.floor(
    (Date.UTC(date.getFullYear(), date.getMonth(), date.getDate()) - BASE_UTC) / 86400000,
  );
  // 逐年扣除（1900 年起）
  let year = MIN_YEAR;
  for (; year < 2101; year++) {
    const yd = lunarYearDays(year);
    if (days < yd) break;
    days -= yd;
  }
  // 逐月扣除（含闰月）
  const months = monthsOfYear(year);
  let mi = 0;
  for (; mi < months.length; mi++) {
    if (days < months[mi].days) break;
    days -= months[mi].days;
  }
  const m = months[Math.min(mi, months.length - 1)];
  return {
    year,
    month: m.month,
    day: days + 1,
    leap: m.leap,
    yearName: `${GAN[(year - 4) % 10]}${ZHI[(year - 4) % 12]}年`,
    monthName: `${m.leap ? "闰" : ""}${MONTH_NAMES[m.month - 1]}月`,
    dayName: DAY_NAMES[days],
  };
}

/** 农历节日（月-日 → 名称）；除夕按腊月最后一天动态判断 */
export function lunarFestival(month: number, day: number): string | null {
  const key = `${month}-${day}`;
  const map: Record<string, string> = {
    "1-1": "春节",
    "1-15": "元宵",
    "2-2": "龙抬头",
    "5-5": "端午",
    "7-7": "七夕",
    "7-15": "中元",
    "8-15": "中秋",
    "9-9": "重阳",
    "12-8": "腊八",
  };
  return map[key] ?? null;
}

/** 判断某农历日期是否为该农历年腊月的最后一天（除夕） */
export function isLunarNewYearsEve(year: number, month: number, day: number): boolean {
  if (month !== 12) return false;
  const lastDay = monthDays(year, 12);
  return day === lastDay;
}
