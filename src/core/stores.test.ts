// 全局状态测试（stores.svelte.ts）
import { beforeEach, describe, expect, it } from "vitest";
import {
  addCell,
  addIconToFolder,
  addPage,
  createFolder,
  currentPage,
  deleteFolder,
  fitCellsToCols,
  layout,
  moveCellAcrossPages,
  moveIconToFolder,
  openFolder,
  plugins,
  removeCell,
  removeCellsByPlugin,
  removeIconFromFolder,
  removePage,
  renameFolder,
  reorderCellById,
  reorderIconInFolder,
  replaceCellWithApp,
  setCellPosition,
  setCellSize,
  setFolderEmoji,
  setIconPosition,
  setLayout,
  setPlugins,
  updateIconAppearance,
} from "./stores.svelte";
import type { Cell, IconCell, Layout, PluginInfo } from "./types";

function icon(id: string): IconCell {
  return { kind: "icon", id, pluginId: "p-" + id, title: id, size: { w: 1, h: 1 } };
}

function emptyLayout(): Layout {
  return { version: 2, pages: [[]] };
}

beforeEach(() => {
  setLayout(emptyLayout());
  currentPage.index = 0;
});

describe("stores: 单元格", () => {
  it("setLayout 替换页面内容", () => {
    setLayout({ version: 2, pages: [[icon("a"), icon("b")]] });
    expect(layout.pages[0].map((c) => c.id)).toEqual(["a", "b"]);
  });

  it("addCell 追加到当前页", () => {
    addCell(icon("x"));
    expect(layout.pages[0].map((c) => c.id)).toEqual(["x"]);
  });

  it("removeCell 删除指定 id", () => {
    setLayout({ version: 2, pages: [[icon("a"), icon("b"), icon("c")]] });
    removeCell("b");
    expect(layout.pages[0].map((c) => c.id)).toEqual(["a", "c"]);
  });

  it("removeCell 对不存在的 id 不报错", () => {
    removeCell("zzz");
    expect(layout.pages[0]).toHaveLength(0);
  });

  it("currentPage 越界时 setLayout 重置到 0", () => {
    setLayout({ version: 2, pages: [[icon("a")]] });
    currentPage.index = 5;
    setLayout({ version: 2, pages: [[icon("b")]] });
    expect(currentPage.index).toBe(0);
  });

  it("setLayout 深拷贝（后续修改不影响原对象）", () => {
    const next: Layout = {
      version: 2,
      pages: [
        [
          icon("a"),
          { kind: "folder", id: "f", name: "F", emoji: "📁", items: [icon("inner")] },
        ],
      ],
    };
    setLayout(next);
    const pageCell = layout.pages[0][1];
    if (pageCell.kind === "folder") {
      pageCell.items.push(icon("new"));
    }
    expect(
      (next.pages[0][1] as Extract<Cell, { kind: "folder" }>).items,
    ).toHaveLength(1);
  });
});

describe("stores: 文件夹", () => {
  it("createFolder 添加文件夹到当前页", () => {
    const id = createFolder("工具", "🧰");
    const cell = layout.pages[0][0];
    expect(cell.kind).toBe("folder");
    if (cell.kind === "folder") {
      expect(cell.id).toBe(id);
      expect(cell.name).toBe("工具");
      expect(cell.emoji).toBe("🧰");
      expect(cell.items).toHaveLength(0);
    }
  });

  it("deleteFolder 删除并关闭打开的文件夹", () => {
    const id = createFolder("工具");
    openFolder.folderId = id;
    deleteFolder(id);
    expect(layout.pages[0]).toHaveLength(0);
    expect(openFolder.folderId).toBeNull();
  });

  it("addIconToFolder / removeIconFromFolder", () => {
    const id = createFolder("工具");
    expect(addIconToFolder(id, icon("a"))).toBe(true);
    expect(addIconToFolder("不存在", icon("b"))).toBe(false);
    const folder = layout.pages[0][0];
    if (folder.kind === "folder") {
      expect(folder.items.map((i) => i.id)).toEqual(["a"]);
    }
    removeIconFromFolder(id, "a");
    if (folder.kind === "folder") {
      expect(folder.items).toHaveLength(0);
    }
  });

  it("moveIconToFolder 从当前页移入文件夹", () => {
    addCell(icon("a"));
    addCell(icon("b"));
    const folderId = createFolder("工具");
    expect(moveIconToFolder("a", folderId)).toBe(true);
    // a 从页中移除
    expect(layout.pages[0].map((c) => c.id)).toEqual(["b", folderId]);
    const folder = layout.pages[0].find((c) => c.id === folderId);
    if (folder?.kind === "folder") {
      expect(folder.items.map((i) => i.id)).toEqual(["a"]);
    }
  });

  it("moveIconToFolder 对不存在图标返回 false", () => {
    const folderId = createFolder("工具");
    expect(moveIconToFolder("nope", folderId)).toBe(false);
  });

  it("moveIconToFolder 支持文件夹之间移动（从文件夹 A 到 B）", () => {
    setLayout({
      version: 2,
      pages: [
        [
          { kind: "folder", id: "fA", name: "A", emoji: "📁", items: [icon("x")] },
          { kind: "folder", id: "fB", name: "B", emoji: "📁", items: [] },
        ],
      ],
    });
    expect(moveIconToFolder("x", "fB")).toBe(true);
    const fA = layout.pages[0][0];
    const fB = layout.pages[0][1];
    if (fA.kind === "folder" && fB.kind === "folder") {
      expect(fA.items).toHaveLength(0);
      expect(fB.items.map((i) => i.id)).toEqual(["x"]);
    }
  });

  it("moveIconToFolder 不能移进自己所在的文件夹", () => {
    setLayout({
      version: 2,
      pages: [
        [{ kind: "folder", id: "fA", name: "A", emoji: "📁", items: [icon("x")] }],
      ],
    });
    expect(moveIconToFolder("x", "fA")).toBe(false);
    const fA = layout.pages[0][0];
    if (fA.kind === "folder") {
      expect(fA.items).toHaveLength(1);
    }
  });
});

describe("stores: 尺寸（整数倍）", () => {
  it("setCellSize 修改主网格图标尺寸", () => {
    addCell(icon("a"));
    expect(setCellSize("a", { w: 2, h: 2 })).toBe(true);
    const cell = layout.pages[0][0];
    if (cell.kind === "icon") {
      expect(cell.size).toEqual({ w: 2, h: 2 });
    }
  });

  it("setCellSize 修正为最小 1×1 的正整数", () => {
    addCell(icon("a"));
    setCellSize("a", { w: 0.5, h: -2 });
    const cell = layout.pages[0][0];
    if (cell.kind === "icon") {
      expect(cell.size).toEqual({ w: 1, h: 1 });
    }
  });

  it("setCellSize 修改文件夹内图标尺寸", () => {
    const folderId = createFolder("工具");
    addIconToFolder(folderId, icon("inner"));
    expect(setCellSize("inner", { w: 3, h: 2 })).toBe(true);
    const folder = layout.pages[0].find((c) => c.id === folderId);
    if (folder?.kind === "folder") {
      expect(folder.items[0].size).toEqual({ w: 3, h: 2 });
    }
  });

  it("setCellSize 对不存在的 id 返回 false", () => {
    expect(setCellSize("nope", { w: 2, h: 1 })).toBe(false);
  });
});

describe("stores: 自由摆放（v3）", () => {
  it("addCell 自动放到首个空位并记录坐标", () => {
    addCell(icon("a"));
    addCell(icon("b"));
    expect(layout.pages[0][0].x).toBe(0);
    expect(layout.pages[0][0].y).toBe(0);
    expect(layout.pages[0][1].x).toBe(1);
    expect(layout.pages[0][1].y).toBe(0);
  });

  it("setCellPosition 修改坐标", () => {
    addCell(icon("a"));
    expect(setCellPosition("a", 3, 2)).toBe(true);
    expect(layout.pages[0][0].x).toBe(3);
    expect(layout.pages[0][0].y).toBe(2);
  });

  it("setCellPosition 目标被占用则交换位置（不自动重排）", () => {
    setLayout({
      version: 3,
      pages: [[{ ...icon("a"), x: 0, y: 0 }, { ...icon("b"), x: 1, y: 0 }]],
    });
    setCellPosition("a", 1, 0);
    const cells = layout.pages[0];
    const a = cells.find((c) => c.id === "a");
    const b = cells.find((c) => c.id === "b");
    expect(a?.x).toBe(1);
    expect(a?.y).toBe(0);
    expect(b?.x).toBe(0);
    expect(b?.y).toBe(0);
  });

  it("fitCellsToCols 把越界单元移回画布内空位", () => {
    // 假设按 12 列迁移出 x=10/11 的单元，实际画布只有 8 列
    setLayout({
      version: 3,
      pages: [
        [
          { ...icon("a"), x: 0, y: 0 },
          { ...icon("b"), x: 11, y: 0 }, // 越界（11+1 > 8）
          { ...icon("c"), x: 10, y: 0 }, // 越界
        ],
      ],
    });
    expect(fitCellsToCols(0, 8)).toBe(true);
    const cells = layout.pages[0];
    for (const cell of cells) {
      const w = cell.kind === "folder" ? 1 : cell.size.w;
      expect((cell.x ?? 0) + w).toBeLessThanOrEqual(8);
    }
    // 幂等：再次调用无变化
    expect(fitCellsToCols(0, 8)).toBe(false);
  });

  it("setIconPosition 文件夹内定位并交换", () => {
    const folderId = createFolder("工具");
    addIconToFolder(folderId, icon("i1"));
    addIconToFolder(folderId, icon("i2"));
    expect(setIconPosition(folderId, "i1", 1, 0)).toBe(true);
    const folder = layout.pages[0].find((c) => c.id === folderId);
    if (folder?.kind === "folder") {
      const i1 = folder.items.find((i) => i.id === "i1");
      const i2 = folder.items.find((i) => i.id === "i2");
      expect(i1?.x).toBe(1);
      expect(i2?.x).toBe(0);
    }
  });
});

describe("stores: 跨页移动（边缘翻页拖拽）", () => {
  it("moveCellAcrossPages 把第 1 页的 a 移到第 2 页指定坐标", () => {
    setLayout({ version: 3, pages: [[icon("a"), icon("b")], [icon("c"), icon("d")]] });
    currentPage.index = 1; // 拖到第 2 页（当前页）
    expect(moveCellAcrossPages("a", 3, 1)).toBe(true);
    expect(layout.pages[0].map((c) => c.id)).toEqual(["b"]);
    expect(layout.pages[1].map((c) => c.id)).toContain("a");
    const a = layout.pages[1].find((c) => c.id === "a");
    expect(a?.x).toBe(3);
    expect(a?.y).toBe(1);
  });

  it("moveCellAcrossPages 对不存在的 id 返回 false", () => {
    setLayout({ version: 3, pages: [[icon("a")], [icon("b")]] });
    currentPage.index = 1;
    expect(moveCellAcrossPages("zzz", 0, 0)).toBe(false);
    expect(layout.pages[0]).toHaveLength(1);
  });

  it("moveIconToFolder 支持从其他页移入文件夹", () => {
    setLayout({
      version: 2,
      pages: [
        [icon("far")],
        [{ kind: "folder", id: "f1", name: "工具", emoji: "📁", items: [] }],
      ],
    });
    currentPage.index = 0;
    expect(moveIconToFolder("far", "f1")).toBe(true);
    expect(layout.pages[0]).toHaveLength(0);
    const f = layout.pages[1][0];
    if (f.kind === "folder") {
      expect(f.items.map((i) => i.id)).toEqual(["far"]);
    }
  });
});

describe("stores: 页面", () => {
  it("addPage 追加空页", () => {
    addPage();
    expect(layout.pages).toHaveLength(2);
  });

  it("removePage 删除指定页（至少保留一页）", () => {
    addPage();
    currentPage.index = 1;
    removePage(1);
    expect(layout.pages).toHaveLength(1);

    removePage(0);
    expect(layout.pages).toHaveLength(1);
  });
});

describe("stores: 插件", () => {
  it("setPlugins 替换插件列表", () => {
    const list: PluginInfo[] = [
      {
        id: "p1",
        name: "P1",
        version: "1",
        pluginType: "icon",
        emoji: "📦",
        actions: [],
      },
    ];
    setPlugins(list);
    expect(plugins).toHaveLength(1);
    expect(plugins[0].id).toBe("p1");

    setPlugins([]);
    expect(plugins).toHaveLength(0);
  });
});

describe("stores: 系统应用槽位替换", () => {
  it("replaceCellWithApp 原位替换页面图标（位置/数量不变）", () => {
    setLayout({
      version: 2,
      pages: [[icon("slot"), icon("other")]],
    });
    const ok = replaceCellWithApp("slot", { name: "记事本", path: "C:\\notepad.exe" });
    expect(ok).toBe(true);
    expect(layout.pages[0]).toHaveLength(2);
    const slot = layout.pages[0][0] as IconCell;
    expect(slot.id).toBe("slot");
    expect(slot.title).toBe("记事本");
    expect(slot.pluginId).toBe("builtin.app");
    expect(slot.action).toEqual({ kind: "app", path: "C:\\notepad.exe" });
    // 其他图标不受影响
    expect((layout.pages[0][1] as IconCell).title).toBe("other");
  });

  it("replaceCellWithApp 替换文件夹内图标", () => {
    setLayout({
      version: 2,
      pages: [
        [
          { kind: "folder", id: "f", name: "F", emoji: "📁", items: [icon("inner-slot")] },
        ],
      ],
    });
    const ok = replaceCellWithApp("inner-slot", { name: "计算器", path: "C:\\calc.exe" });
    expect(ok).toBe(true);
    const folder = layout.pages[0][0];
    if (folder.kind === "folder") {
      expect(folder.items[0].title).toBe("计算器");
      expect(folder.items[0].action).toEqual({ kind: "app", path: "C:\\calc.exe" });
    }
  });

  it("replaceCellWithApp 找不到源图标时返回 false 且不改动", () => {
    setLayout({ version: 2, pages: [[icon("a")]] });
    expect(replaceCellWithApp("nope", { name: "X", path: "C:\\x.exe" })).toBe(false);
    expect(layout.pages[0]).toHaveLength(1);
    expect((layout.pages[0][0] as IconCell).title).toBe("a");
  });
});

describe("stores: 文件夹交互补全（M8）", () => {
  function folderWith(items: IconCell[]): Layout {
    return {
      version: 2,
      pages: [[{ kind: "folder", id: "f", name: "F", emoji: "📁", items }]],
    };
  }

  it("reorderIconInFolder 移到目标之前/之后", () => {
    setLayout(folderWith([icon("a"), icon("b"), icon("c")]));
    expect(reorderIconInFolder("f", "c", "a", "before")).toBe(true);
    const f = layout.pages[0][0];
    if (f.kind === "folder") {
      expect(f.items.map((i) => i.id)).toEqual(["c", "a", "b"]);
    }
    expect(reorderIconInFolder("f", "a", "b", "after")).toBe(true);
    if (f.kind === "folder") {
      expect(f.items.map((i) => i.id)).toEqual(["c", "b", "a"]);
    }
  });

  it("reorderIconInFolder 空目标追加到末尾", () => {
    setLayout(folderWith([icon("a"), icon("b")]));
    expect(reorderIconInFolder("f", "a", null, "after")).toBe(true);
    const f = layout.pages[0][0];
    if (f.kind === "folder") {
      expect(f.items.map((i) => i.id)).toEqual(["b", "a"]);
    }
  });

  it("renameFolder / setFolderEmoji 生效", () => {
    setLayout(folderWith([]));
    expect(renameFolder("f", "  工具  ")).toBe(true);
    expect(setFolderEmoji("f", "🧰")).toBe(true);
    const f = layout.pages[0][0];
    if (f.kind === "folder") {
      expect(f.name).toBe("工具");
      expect(f.emoji).toBe("🧰");
    }
  });

  it("renameFolder 空名不改动并返回 true（保持原名）", () => {
    setLayout(folderWith([]));
    expect(renameFolder("f", "   ")).toBe(true);
    const f = layout.pages[0][0];
    if (f.kind === "folder") {
      expect(f.name).toBe("F");
    }
  });
});

describe("stores: 图标自定义（M9）", () => {
  it("updateIconAppearance 设置/清除自定义字段（页面图标）", () => {
    setLayout({ version: 2, pages: [[icon("a")]] });
    expect(updateIconAppearance("a", { title: " 游戏 ", emoji: "🎮", color: "#e53935" })).toBe(true);
    const cell = layout.pages[0][0] as IconCell;
    expect(cell.title).toBe("游戏");
    expect(cell.emoji).toBe("🎮");
    expect(cell.color).toBe("#e53935");
    // 空字符串 = 清除（回退插件默认）
    updateIconAppearance("a", { emoji: "", color: "" });
    expect(cell.emoji).toBeUndefined();
    expect(cell.color).toBeUndefined();
  });

  it("updateIconAppearance 支持文件夹内图标 + 借用系统图标", () => {
    setLayout({
      version: 2,
      pages: [[{ kind: "folder", id: "f", name: "F", emoji: "📁", items: [icon("inner")] }]],
    });
    expect(updateIconAppearance("inner", { iconPath: "C:\\game.exe" })).toBe(true);
    const folder = layout.pages[0][0];
    if (folder.kind === "folder") {
      expect(folder.items[0].iconPath).toBe("C:\\game.exe");
    }
  });

  it("updateIconAppearance 找不到时返回 false", () => {
    setLayout({ version: 2, pages: [[icon("a")]] });
    expect(updateIconAppearance("nope", { title: "X" })).toBe(false);
  });
});

describe("stores: 插件卸载清理（M11）", () => {
  it("removeCellsByPlugin 移除页面与文件夹内的该插件图标", () => {
    const p1 = { ...icon("a"), pluginId: "p1" };
    const p2 = { ...icon("b"), pluginId: "p2" };
    setLayout({
      version: 2,
      pages: [
        [
          p1,
          p2,
          { kind: "folder", id: "f", name: "F", emoji: "📁", items: [p1, p2] },
        ],
      ],
    });
    const removed = removeCellsByPlugin("p1");
    expect(removed).toBe(2);
    expect(layout.pages[0].map((c) => c.id)).toEqual(["b", "f"]);
    const folder = layout.pages[0][1];
    if (folder.kind === "folder") {
      expect(folder.items.map((i) => i.id)).toEqual(["b"]);
    }
  });

  it("removeCellsByPlugin 无匹配时返回 0", () => {
    setLayout({ version: 2, pages: [[icon("a")]] });
    expect(removeCellsByPlugin("nope")).toBe(0);
  });
});

// 确保 Cell 类型可被消费（编译期类型检查辅助）
const _cells: Cell[] = [icon("a")];
void _cells;
