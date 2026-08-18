// 系统应用图标缓存测试（mock @tauri-apps/api/core）
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import { appIcons, loadAppIcon } from "./appIcons.svelte";

const mockInvoke = vi.mocked(invoke);

beforeEach(() => {
  vi.clearAllMocks();
  for (const k of Object.keys(appIcons)) delete appIcons[k];
});

describe("loadAppIcon", () => {
  it("成功时缓存 data URL", async () => {
    mockInvoke.mockResolvedValue("data:image/png;base64,AAA");
    const url = await loadAppIcon("C:\\a.exe");
    expect(url).toBe("data:image/png;base64,AAA");
    expect(appIcons["C:\\a.exe"]).toBe("data:image/png;base64,AAA");
    expect(mockInvoke).toHaveBeenCalledWith("app_icon", { path: "C:\\a.exe" });
  });

  it("失败时返回 null 且不缓存", async () => {
    mockInvoke.mockRejectedValue(new Error("no icon"));
    expect(await loadAppIcon("C:\\b.exe")).toBeNull();
    expect("C:\\b.exe" in appIcons).toBe(false);
  });

  it("并发调用只请求一次", async () => {
    mockInvoke.mockResolvedValue("data:image/png;base64,BBB");
    const [a, b] = await Promise.all([
      loadAppIcon("C:\\c.exe"),
      loadAppIcon("C:\\c.exe"),
    ]);
    expect(a).toBe("data:image/png;base64,BBB");
    expect(b).toBe("data:image/png;base64,BBB");
    // 只统计 app_icon 命令（log_write 等日志调用不计入）
    expect(mockInvoke.mock.calls.filter((c) => c[0] === "app_icon")).toHaveLength(1);
  });
});
