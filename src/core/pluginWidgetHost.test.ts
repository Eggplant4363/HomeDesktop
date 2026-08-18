// 插件小组件宿主测试：路径拼接 + 沙箱 srcdoc 构造（纯函数）
import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn(
    (p: string) => `asset://localhost/${encodeURIComponent(p.replace(/\\/g, "/"))}`,
  ),
}));

import { invoke } from "@tauri-apps/api/core";
import {
  buildPluginIframeSrcdoc,
  handleBridgeMessage,
  pluginWidgetPath,
} from "./pluginWidgetHost.svelte";

const mockInvoke = vi.mocked(invoke);

describe("pluginWidgetPath", () => {
  it("Windows 路径用反斜杠拼接", () => {
    const dir = "C:\\Users\\x\\AppData\\Roaming\\dev.homedesktop.app\\plugins\\com.a.b";
    expect(pluginWidgetPath(dir, "widget.js")).toBe(
      "C:\\Users\\x\\AppData\\Roaming\\dev.homedesktop.app\\plugins\\com.a.b\\widget.js",
    );
  });

  it("正斜杠路径用正斜杠拼接", () => {
    expect(pluginWidgetPath("/home/user/plugins/p1", "w.js")).toBe(
      "/home/user/plugins/p1/w.js",
    );
  });
});

describe("buildPluginIframeSrcdoc", () => {
  it("包含沙箱桥、脚本 src、元素标签与 cell-id", () => {
    const doc = buildPluginIframeSrcdoc({
      code: "customElements.define('hd-timer-widget', class {})",
      elementTag: "hd-timer-widget",
      cellId: "cell-1",
    });
    expect(doc).toContain("__homedesktopPlugin");
    expect(doc).toContain("customElements.define('hd-timer-widget'");
    expect(doc).toContain('createElement("hd-timer-widget")');
    expect(doc).toContain('setAttribute("cell-id","cell-1")');
    expect(doc).toContain("postMessage");
  });

  it("插件代码里的 </script> 会被转义防止截断", () => {
    const doc = buildPluginIframeSrcdoc({
      code: "const s = '</script>';",
      elementTag: "hd-x",
      cellId: "c",
    });
    expect(doc).not.toContain("</script>';");
    expect(doc).toContain("<\\/script>';");
  });

  it("沙箱桥 shim 提供 notify 能力", () => {
    const doc = buildPluginIframeSrcdoc({
      code: "customElements.define('hd-x', class {})",
      elementTag: "hd-x",
      cellId: "c",
    });
    expect(doc).toContain("notify: function(title, body)");
    expect(doc).toContain('call("notify", { title: title, body: body })');
  });
});

describe("handleBridgeMessage", () => {
  it("notify 消息 → 调用 app_notify 并回包", async () => {
    expect(vi.isMockFunction(invoke)).toBe(true);
    mockInvoke.mockResolvedValue(true);
    const post = vi.fn();
    const e = {
      data: { __hd: true, t: "notify", title: "倒计时结束", body: "时间到", id: 7 },
      source: { postMessage: post },
    } as unknown as MessageEvent;
    expect(handleBridgeMessage(e)).toBe(true);
    expect(invoke).toHaveBeenCalledWith("app_notify", {
      title: "倒计时结束",
      body: "时间到",
    });
    await vi.waitFor(() => {
      expect(post).toHaveBeenCalledWith(
        { __hd: true, id: 7, result: true },
        { targetOrigin: "*" },
      );
    });
  });

  it("非桥消息返回 false", () => {
    const e = { data: { hello: 1 } } as unknown as MessageEvent;
    expect(handleBridgeMessage(e)).toBe(false);
  });
});
