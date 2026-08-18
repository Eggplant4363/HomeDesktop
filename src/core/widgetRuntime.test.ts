// widgetRuntime 测试：按实例注册/刷新/缓存隔离
// 注：调度器 setInterval 用假定时器避免拖住 vitest 退出；Date 保持真实
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  isWidgetStale,
  refreshWidget,
  registerWidget,
  widgetCache,
} from "./widgetRuntime.svelte";

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["setInterval", "setTimeout", "clearInterval", "clearTimeout"] });
  for (const k of Object.keys(widgetCache)) delete widgetCache[k];
});

afterEach(() => {
  vi.useRealTimers();
});

describe("registerWidget / refreshWidget", () => {
  it("按实例注册并刷新缓存", async () => {
    registerWidget({
      id: "cellA",
      refreshMs: 1000,
      fetch: async () => ({ city: "济南" }),
    });
    const ok = await refreshWidget("cellA");
    expect(ok).toBe(true);
    expect((widgetCache["cellA"]?.data as { city: string }).city).toBe("济南");
    expect(widgetCache["cellA"]?.fetchedAt).toBeDefined();
  });

  it("未注册的实例刷新是 no-op 且返回 false", async () => {
    expect(await refreshWidget("nope")).toBe(false);
    expect(widgetCache["nope"]).toBeUndefined();
  });

  it("不同实例缓存互相隔离", async () => {
    registerWidget({ id: "cellA", refreshMs: 1000, fetch: async () => "A" });
    registerWidget({ id: "cellB", refreshMs: 1000, fetch: async () => "B" });
    await refreshWidget("cellA");
    expect(widgetCache["cellA"]?.data).toBe("A");
    expect(widgetCache["cellB"]).toBeUndefined();
  });

  it("isWidgetStale 按实例判断过期", async () => {
    registerWidget({ id: "x", refreshMs: 1000, fetch: async () => 1 });
    expect(isWidgetStale("x", 1000)).toBe(true);
    await refreshWidget("x");
    expect(isWidgetStale("x", 1000)).toBe(false);
  });
});
