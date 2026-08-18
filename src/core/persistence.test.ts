// 布局持久化测试（mock @tauri-apps/api/core）
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import { loadLayout, saveLayout } from "./persistence";
import { layout } from "./stores.svelte";
import type { Layout } from "./types";

const mockInvoke = vi.mocked(invoke);

const saved: Layout = {
  version: 2,
  pages: [
    [
      {
        kind: "icon",
        id: "i1",
        pluginId: "dev.homedesktop.demo",
        title: "Demo",
        size: { w: 1, h: 1 },
      },
    ],
  ],
};

beforeEach(() => {
  vi.clearAllMocks();
});

describe("loadLayout", () => {
  it("有数据时恢复布局", async () => {
    mockInvoke.mockResolvedValue(saved);
    await loadLayout();
    expect(mockInvoke).toHaveBeenCalledWith("layout_load");
    expect(layout.pages[0][0].id).toBe("i1");
  });

  it("invoke 返回 null 时保持现状", async () => {
    mockInvoke.mockResolvedValue(null);
    await loadLayout();
    expect(mockInvoke).toHaveBeenCalledWith("layout_load");
  });

  it("invoke 失败时不抛出", async () => {
    mockInvoke.mockRejectedValue(new Error("io error"));
    await expect(loadLayout()).resolves.toBeUndefined();
  });
});

describe("saveLayout", () => {
  it("将布局传给 layout_save", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await saveLayout(saved);
    expect(mockInvoke).toHaveBeenCalledWith("layout_save", {
      layout: saved,
    });
  });

  it("失败时不抛出", async () => {
    mockInvoke.mockRejectedValue(new Error("disk full"));
    await expect(saveLayout(saved)).resolves.toBeUndefined();
  });
});
