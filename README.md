# HomeDesktop

跨平台（Windows / macOS / Linux）轻量级桌面启动器与智能家居中控屏：像手机桌面一样管理"图标"——应用、文件夹、插件小组件都通过**插件机制**添加到网格中。

> ⚠️ 开发中（M0/M1 阶段）。产品需求见 [docs/PRD.md](docs/PRD.md)，技术选型见 [docs/tech-selection.md](docs/tech-selection.md)，架构见 [docs/architecture.md](docs/architecture.md)，里程碑见 [docs/milestones.md](docs/milestones.md)。

## 技术栈

- **Tauri 2**（Rust 后端 + 系统 WebView）— 二进制约 5-10MB，启动快
- **Svelte 5 + TypeScript + Vite 6** 前端

## 环境要求

- Node.js ≥ 20.17（**本机注意**：Node 24 与 npm 11 存在 zlib 兼容问题，请使用 corepack pnpm）
- Rust stable（rustup）
- WebView2（Windows）/ WKWebView（macOS）/ WebKitGTK（Linux）

## 开发

```bash
# 依赖安装（本机用 corepack pnpm；pnpm 仓库缓存位于 .pnpm-store/）
corepack pnpm install --store-dir .pnpm-store

# 开发运行（自动启动 Vite + Tauri 窗口）
corepack pnpm tauri dev
```

## 目录结构

```
src/           前端（Svelte 5）
src/core/      核心：类型 / 状态 / 持久化 / 插件加载器
src/components/ 网格 / 图标块 / 搜索 / 添加菜单
src-tauri/     Rust 壳（插件注册表、布局持久化、动作执行）
plugins/       内置插件（每插件一目录 + manifest.json）
docs/          产品与技术文档
.dsh/skills/   开发流程 skill
```

## 插件开发

> 📖 **完整开发者 API 文档见 [`docs/PLUGIN_API.md`](docs/PLUGIN_API.md)**（manifest schema v2：尺寸声明 `sizes`、统一设置菜单 `settings`、小组件数据缓存/定时刷新机制、zip 分发与安装）

每个插件 = 一个目录 + `manifest.json`：

```json
{
  "id": "com.example.myplugin",
  "name": "我的插件",
  "version": "1.0.0",
  "type": "icon",
  "emoji": "🧩",
  "sizes": [{ "w": 1, "h": 1 }],
  "settings": [
    { "key": "city", "label": "城市", "type": "text", "default": "北京" }
  ],
  "actions": [{ "kind": "app", "path": "C:\\Windows\\System32\\notepad.exe" }]
}
```

- `type`: `icon`（图标，点击执行动作）| `widget`（小组件，如时钟/天气）
- `sizes`: 插件支持的尺寸档（用户切大小只能选这些档；图标类固定 1×1）
- `settings`: 可配置项声明（框架提供统一设置菜单：text/number/select/toggle）
- `actions[].kind`: `app`（启动应用）| `command`（执行命令）
- 小组件：`widgetComponent` 指定框架内置组件（`clock` / `weather`），数据缓存/定时刷新用 `widgetRuntime`

插件目录扫描顺序：用户数据目录 `plugins/` > 资源目录 `plugins/` > 项目根 `plugins/`（开发时）。
分发：zip（根目录含 manifest.json）→ 应用内 ＋ → 从 zip 安装插件。

## 打包（Windows 实测产物）

```bash
corepack pnpm tauri build
```

| 产物 | 大小 |
|---|---|
| `src-tauri/target/.../release/bundle/nsis/HomeDesktop_*-setup.exe` | **~1.1 MB** |
| `src-tauri/target/.../release/bundle/msi/HomeDesktop_*.msi` | ~1.7 MB |
| `src-tauri/target/.../release/homedesktop.exe` | ~3.2 MB |

启动内存 ~26MB。macOS/Linux 打包由 `.github/workflows/build.yml`（三平台矩阵）覆盖。

## 功能一览（M0–M5 + P2 迭代）

- 手机式桌面：图标 + 小组件（时钟/天气）同屏混排，多页、文件夹、搜索
- 拖拽排序（鼠标/触屏长按）、拖入文件夹、拖到边缘翻页、**跨页移动**
- 图标/小组件尺寸 = 1×1 基准的整数倍（⇲ 调整）
- 外观设置（⚙）：**主题（深/浅）**、背景预设/自定义颜色/**本地图片壁纸**、图标大小
- 全局热键：**Alt+Space** 显示/隐藏窗口；**系统托盘**图标（单击显示/隐藏）
- **开机自启**开关（⚙）
- **系统应用插件**：内置「📱 系统应用」插件（manifest 驱动）——点击打开系统已安装应用面板（注册表 + 开始菜单扫描），每项显示**系统真实图标**，选中后**原位替换**该图标为应用（＋ 菜单不再列应用，统一走插件）
- **插件市场**：从本地 zip 安装插件包（`plugins-dist/demo-cmd-2.0.0.zip` 为示例，开发者文档见 `docs/PLUGIN_API.md`）
- 插件机制：`plugins/` 目录 + manifest.json（图标 / 小组件）
- 布局/配置本地持久化（layout.json / config.json）
