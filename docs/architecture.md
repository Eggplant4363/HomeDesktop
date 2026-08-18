# 架构设计 — HomeDesktop

> 版本：v1.0 ｜ 日期：2026-02-13 ｜ 按 skill P3 阶段产出。

## 1. 模块划分

```
┌─────────────────────────────────────────────────────┐
│ 插件层 plugins/                                      │
│   内置插件：app-launcher（应用启动）、custom-command   │
│   小组件插件：clock / weather（时钟、天气）             │
│   每个插件 = 目录 + manifest.json + 前端组件入口        │
├─────────────────────────────────────────────────────┤
│ 功能层 frontend/src/                                 │
│   桌面网格（Grid）、文件夹（Folder）、搜索（Search）、   │
│   小组件（Widgets）、设置（Settings, P2）              │
├─────────────────────────────────────────────────────┤
│ 核心层                                                │
│   Rust：窗口/生命周期、插件注册表（plugin registry）、   │
│         配置持久化（config.json）、命令分发（commands） │
│   前端：桌面状态（stores）、插件加载器（loader）、        │
│         布局持久化（persistence）                       │
└─────────────────────────────────────────────────────┘
```

- **核心层**：不依赖具体插件；只定义"图标/小组件"抽象与注册机制。
- **功能层**：实现具体交互，通过核心层的注册表查询插件能力。
- **插件层**：以 manifest 声明 + 前端组件实现，新增插件不改核心。

## 2. 关键数据流

```
插件 manifest (JSON) ──扫描──▶ 插件注册表(Rust) ──invoke──▶ 前端 loader
                                                                 │
用户布局配置 (store JSON) ◀──持久化── 布局 stores(前端) ◀─渲染─ 网格/文件夹
                                                                 │
图标动作（启动应用/执行命令）──invoke──▶ Rust commands ──▶ tauri-plugin-shell
```

- 配置与布局：`app-config.json` + `layout.json`，由 `tauri-plugin-store` 管理，存于应用数据目录。
- 插件发现：启动时扫描 `plugins/` 目录（用户目录）与内置 `plugins/`；manifest 校验失败仅告警跳过。

## 3. 插件机制设计（核心）

**manifest.json schema（v1）**：
```json
{
  "id": "dev.homedesktop.clock",
  "name": "时钟小组件",
  "version": "0.1.0",
  "type": "widget | icon",          // 小组件或图标
  "entry": "dist/index.js",         // 前端组件入口（相对插件目录）
  "defaultSize": { "w": 2, "h": 2 },// 网格占位（宽高以单元计）
  "actions": [                      // type=icon 时：图标点击动作
    { "kind": "app", "appPath": "..." } | { "kind": "command", "cmd": "..." }
  ]
}
```

**注册/发现流程**：
1. Rust 启动时扫描插件目录 → 解析 manifest → 注册表存 `PluginInfo`。
2. 前端通过 `invoke('plugins:list')` 获取插件列表，`invoke('plugins:load_asset', {id, path})` 读取插件资源（convertFileSrc 提供给 WebView）。
3. 用户把插件图标/小组件"添加"到网格 → 写入 `layout.json`。
4. 权限边界：插件仅通过白名单 invoke 命令与核心交互（不能任意执行系统命令；仅 `type=icon` 且声明 `actions` 时允许 shell 启动）。

## 4. 目录结构

```
HomeDesktop/
├─ .dsh/skills/product-driven-project/   # 本流程 skill
├─ docs/                                 # PRD / 技术选型 / 架构 / 里程碑
├─ src-tauri/                            # Rust 壳
│  ├─ src/                               #   main.rs, commands.rs, plugins.rs
│  ├─ tauri.conf.json                    #   窗口/打包配置
│  └─ Cargo.toml
├─ src/                                  # 前端 (Svelte 5 + TS + Vite)
│  ├─ main.ts / App.svelte
│  ├─ core/                              #   核心：stores、loader、persistence、config
│  ├─ components/                        #   Grid / Folder / WidgetTile / AddMenu
│  ├─ widgets/                           #   小组件注册表 + 时钟/天气
│  └─ lib/                               #   工具
├─ plugins/                              # 内置插件（每插件一目录 + manifest.json）
├─ package.json / vite.config.ts / tsconfig.json
└─ README.md
```

## 5. 关键依赖（已选）

| 依赖 | 用途 | 说明 |
|---|---|---|
| Tauri 2 + tauri-plugin-store / shell / global-shortcut | 壳/存储/启动应用/热键 | 官方维护 |
| Svelte 5 + Vite 6 + TypeScript | 前端 | 轻量 |
| (P1) tauri-plugin-http | 天气插件联网 | 官方维护 |

**不做的事**：不引入重型 UI 框架、不引入全局状态库、不引入 ORM/DB（JSON 足够）。

## 变更记录

| 日期 | 变更 |
|---|---|
| 2026-02-13 | v1.0 初始版 |
