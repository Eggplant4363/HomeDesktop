// 搜索过滤纯函数测试
import { describe, expect, it } from "vitest";
import { filterCells, filterIcons } from "./search";
import type { Cell, IconCell, PluginInfo } from "./types";

const plugins: PluginInfo[] = [
  {
    id: "dev.homedesktop.notepad",
    name: "记事本",
    version: "0.1.0",
    pluginType: "icon",
    emoji: "📝",
    actions: [{ kind: "app", path: "notepad.exe" }],
  },
  {
    id: "dev.homedesktop.web",
    name: "浏览器",
    version: "0.1.0",
    pluginType: "icon",
    emoji: "🌐",
    actions: [{ kind: "command", cmd: "start https://example.com" }],
  },
];

const cells: Cell[] = [
  {
    kind: "icon",
    id: "i1",
    pluginId: "dev.homedesktop.notepad",
    title: "记事本（演示）",
    size: { w: 1, h: 1 },
  },
  {
    kind: "icon",
    id: "i2",
    pluginId: "dev.homedesktop.web",
    title: "打开 GitHub",
    size: { w: 1, h: 1 },
  },
  {
    kind: "folder",
    id: "f1",
    name: "办公工具",
    emoji: "📁",
    items: [],
  },
];

describe("filterCells", () => {
  it("空查询返回全部", () => {
    expect(filterCells(cells, plugins, "")).toHaveLength(3);
  });

  it("按图标标题匹配（大小写不敏感）", () => {
    const r = filterCells(cells, plugins, "github");
    expect(r).toHaveLength(1);
    expect(r[0].id).toBe("i2");
  });

  it("按插件名匹配", () => {
    const r = filterCells(cells, plugins, "记事本");
    expect(r).toHaveLength(1);
    expect(r[0].id).toBe("i1");
  });

  it("按文件夹名匹配", () => {
    const r = filterCells(cells, plugins, "办公");
    expect(r).toHaveLength(1);
    expect(r[0].id).toBe("f1");
    expect(r[0].kind).toBe("folder");
  });

  it("无匹配返回空数组", () => {
    expect(filterCells(cells, plugins, "不存在的词")).toHaveLength(0);
  });

  it("插件不存在时退化为仅匹配标题", () => {
    const orphan: Cell = {
      kind: "icon",
      id: "i3",
      pluginId: "missing.plugin",
      title: "孤儿子项",
      size: { w: 1, h: 1 },
    };
    const r = filterCells([...cells, orphan], plugins, "孤儿");
    expect(r).toHaveLength(1);
    expect(r[0].id).toBe("i3");
  });
});

describe("filterIcons", () => {
  const items: IconCell[] = [
    {
      kind: "icon",
      id: "a",
      pluginId: "dev.homedesktop.notepad",
      title: "记事本",
      size: { w: 1, h: 1 },
    },
    {
      kind: "icon",
      id: "b",
      pluginId: "dev.homedesktop.web",
      title: "打开 GitHub",
      size: { w: 1, h: 1 },
    },
  ];

  it("空查询返回全部", () => {
    expect(filterIcons(items, plugins, "")).toHaveLength(2);
  });

  it("按标题匹配", () => {
    expect(filterIcons(items, plugins, "github")).toHaveLength(1);
  });

  it("按插件名匹配", () => {
    const r = filterIcons(items, plugins, "浏览器");
    expect(r).toHaveLength(1);
    expect(r[0].id).toBe("b");
  });
});
