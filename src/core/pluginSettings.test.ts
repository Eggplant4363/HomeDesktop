// 插件设置测试：实例级（cell.<cellId>.<key>）与插件级（plugin.<id>.<key>）隔离/回退
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import {
  getCellSetting,
  getPluginSetting,
  peekCellSetting,
  pluginSettings,
  setCellSetting,
  setPluginSetting,
} from "./pluginSettings.svelte";

const mockInvoke = vi.mocked(invoke);
/** 模拟 config.json 键值存储 */
const store = new Map<string, unknown>();

beforeEach(() => {
  store.clear();
  for (const k of Object.keys(pluginSettings)) delete pluginSettings[k];
  mockInvoke.mockReset();
  mockInvoke.mockImplementation(
    // eslint-disable-next-line @typescript-eslint/no-explicit-any -- invoke 参数类型宽泛
    async (cmd: string, args?: any) => {
      if (cmd === "config_get") return store.get(String(args?.key ?? "")) ?? null;
      if (cmd === "config_set") {
        if (args?.key) store.set(String(args.key), args.value);
        return null;
      }
      return null;
    },
  );
});

describe("实例级设置", () => {
  it("不同图标实例相互独立", async () => {
    await setCellSetting("cellA", "p", "city", "济南");
    await setCellSetting("cellB", "p", "city", "上海");
    expect(await getCellSetting("cellA", "p", "city", "北京")).toBe("济南");
    expect(await getCellSetting("cellB", "p", "city", "北京")).toBe("上海");
    // 未设置的实例回退默认值
    expect(await getCellSetting("cellC", "p", "city", "北京")).toBe("北京");
  });

  it("未设置实例回退插件级默认（兼容旧共享设置）", async () => {
    await setPluginSetting("p", "unit", "celsius");
    expect(await getCellSetting("cellX", "p", "unit", "fahrenheit")).toBe("celsius");
  });

  it("peekCellSetting 优先实例缓存，其次插件级缓存", async () => {
    await setCellSetting("cellA", "p", "city", "济南");
    await setPluginSetting("p", "city", "广州");
    expect(peekCellSetting("cellA", "p", "city")).toBe("济南");
    expect(peekCellSetting("cellOther", "p", "city")).toBe("广州");
  });

  it("setCellSetting 只写实例键，不影响插件级", async () => {
    await setCellSetting("cellA", "p", "city", "济南");
    expect(store.get("cell.cellA.city")).toBe("济南");
    expect(store.has("plugin.p.city")).toBe(false);
  });
});
