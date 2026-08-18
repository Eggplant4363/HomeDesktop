// 插件加载器测试（mock @tauri-apps/api/core）
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import { loadPlugins, launchPlugin } from "./pluginLoader";
import { plugins } from "./stores.svelte";
import type { PluginInfo } from "./types";

const mockInvoke = vi.mocked(invoke);

const samplePlugins: PluginInfo[] = [
  {
    id: "dev.homedesktop.demo",
    name: "Demo",
    version: "0.1.0",
    pluginType: "icon",
    emoji: "🧪",
    actions: [{ kind: "command", cmd: "echo hi" }],
  },
];

beforeEach(() => {
  vi.clearAllMocks();
});

describe("loadPlugins", () => {
  it("调用 plugins_list 并填充 stores", async () => {
    mockInvoke.mockResolvedValue(samplePlugins);
    await loadPlugins();
    expect(mockInvoke).toHaveBeenCalledWith("plugins_list");
    expect(plugins).toHaveLength(1);
    expect(plugins[0].id).toBe("dev.homedesktop.demo");
  });

  it("invoke 失败时不抛出（降级为空列表）", async () => {
    mockInvoke.mockRejectedValue(new Error("boom"));
    await expect(loadPlugins()).resolves.toBeUndefined();
  });
});

describe("launchPlugin", () => {
  it("按插件 id 调用 launch_action", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await launchPlugin("dev.homedesktop.demo");
    expect(mockInvoke).toHaveBeenCalledWith("launch_action", {
      pluginId: "dev.homedesktop.demo",
    });
  });

  it("失败时向上抛错", async () => {
    mockInvoke.mockRejectedValue(new Error("plugin not found"));
    await expect(launchPlugin("nope")).rejects.toThrow("plugin not found");
  });
});
