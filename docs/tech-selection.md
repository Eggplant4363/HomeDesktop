# 技术选型 — HomeDesktop

> 版本：v1.0 ｜ 日期：2026-02-13 ｜ 状态：已确认（用户选择 Tauri 2）
> 文档按 skill `product-driven-project` 的 P2 阶段产出。

## 1. 硬约束（来自 PRD NFR）

| # | 约束 | 说明 |
|---|---|---|
| C1 | 跨平台 | Windows 10+ / macOS 12+ / Linux x64，一套代码 |
| C2 | 轻量 | 安装包 ≤ 15MB，常驻内存 ≤ 200MB |
| C3 | 启动快 | 冷启动到可交互 ≤ 2s |
| C4 | 插件化 | 插件可独立添加图标/小组件，不侵入核心 |
| C5 | 生态可用 | 桌面能力：窗口管理、托盘、全局快捷键、本地存储 |

## 2. 候选技术栈决策矩阵

评分 1-5（5 最优）：

| 候选 | C1 跨平台 | C2 轻量 | C3 启动快 | C4 插件化 | C5 生态 | 总分 | 安装成本 |
|---|---|---|---|---|---|---|---|
| **Tauri 2**（Rust + 系统 WebView） | 5 | 5 | 5 | 5 | 4 | **24** | 需装 Rust（一次性） |
| Electron（Node + Chromium） | 5 | 1 | 2 | 4 | 5 | 17 | 低（Node 已有） |
| Neutralinojs（Node + 系统 WebView） | 4 | 5 | 5 | 3 | 3 | 20 | 低（Node 已有） |
| Qt / PySide6（C++/Python） | 4 | 3 | 3 | 3 | 4 | 17 | 中（Python 包） |

## 3. 结论：Tauri 2

**理由**：
1. **C2/C3 最优**：使用系统 WebView（Windows WebView2 / macOS WKWebView / Linux WebKitGTK），二进制仅 5-10MB，启动远快于打包 Chromium 的 Electron。
2. **C4 插件化最匹配**：Tauri 官方插件体系（`tauri-plugin-*`）+ Rust 侧可自由扩展 command，配合前端插件清单注册机制，正好支撑"插件式图标/小组件"产品形态。
3. **C5 桌面能力完整**：窗口、托盘、全局快捷键（`tauri-plugin-global-shortcut`）、存储（`tauri-plugin-store`）均为官方维护插件。
4. 用户已确认接受 Rust 工具链安装成本。

**环境约束记录（本机 2026-02-13）**：
- Node v24.14.0 ✅、npm 11.9.0 ✅（`npm.cmd`）、corepack 0.34.6 ✅（可启用 pnpm）
- WebView2 运行时 ✅
- Rust 未安装 → 正在通过镜像安装（rustup，USTC 镜像）
- 无代理；沙箱环境 TLS 受限需完整权限执行安装

## 4. 前端技术栈

| 组件 | 选择 | 理由 |
|---|---|---|
| 构建工具 | Vite 6 | 快、Tauri 官方模板默认 |
| 框架 | **Svelte 5**（+ TypeScript） | 轻量（无虚拟 DOM，运行时 ~3KB）、编译期优化、适合网格/小组件高频渲染场景；对比 React 更适合"轻量"约束 |
| 状态管理 | Svelte 自带 stores + 本地 JSON 持久化 | MVP 不需要重型状态库 |
| 样式 | 原生 CSS + CSS 变量（主题） | 避免引入 UI 框架体积；主题用 CSS 变量实现 |

## 5. 后端（Rust）关键依赖

| crate / 插件 | 用途 |
|---|---|
| tauri 2.x | 应用壳 |
| tauri-plugin-store | 配置/布局持久化（JSON） |
| tauri-plugin-global-shortcut | 热键唤起（P1） |
| serde / serde_json | 插件清单解析 |
| tauri-plugin-shell | 启动外部应用/命令（图标动作） |

## 6. 备选回退

若后续发现 Svelte 生态某处不满足（如拖拽库），可平替为 React + @dnd-kit（体积代价 ~50KB gzip，仍满足 ≤15MB 包）；框架更换仅影响前端层，不影响 Rust 壳与插件机制设计。
