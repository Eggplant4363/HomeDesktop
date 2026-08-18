// 日历工具（M12）：月历网格计算（纯函数，便于单测）

/** 返回某年某月的 6 列周网格（周日开头），null = 非当月占位。month: 1-12 */
export function getMonthGrid(year: number, month: number): (number | null)[][] {
  const startDay = new Date(year, month - 1, 1).getDay(); // 0 = 周日
  const daysInMonth = new Date(year, month, 0).getDate();
  const cells: (number | null)[] = [];
  for (let i = 0; i < startDay; i++) cells.push(null);
  for (let d = 1; d <= daysInMonth; d++) cells.push(d);
  while (cells.length % 7 !== 0) cells.push(null);
  const weeks: (number | null)[][] = [];
  for (let i = 0; i < cells.length; i += 7) weeks.push(cells.slice(i, i + 7));
  return weeks;
}
