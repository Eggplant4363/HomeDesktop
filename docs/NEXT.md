# 明日续工笔记（2026-02-13 晚收工）

> 给明天的新会话/新上下文的交接文档。产品需求见 `PRD.md`，完整历程见 `CHANGELOG.md`。

## 当前状态：全部里程碑 M0–M5 完成 ✅

**产品**：手机式桌面启动器（Tauri 2 + Svelte 5）。图标 + 小组件（时钟/天气）同屏混排、多页、文件夹、搜索、拖拽排序（鼠标/触屏长按）、拖入文件夹、跨页移动、整数倍尺寸（1×1 基准）、外观设置（背景/图标大小）、**Alt+Space 全局热键**、插件机制（`plugins/` 目录 + manifest.json）。

**测试**：前端 Vitest **44/44**、Rust core **12/12**、svelte-check 0 警告。

**打包产物**（`src-tauri/target/x86_64-pc-windows-gnu/release/bundle/`）：
- `nsis/HomeDesktop_0.1.0_x64-setup.exe`（1.11 MB）
- `msi/HomeDesktop_0.1.0_x64_en-US.msi`（1.68 MB）
- 绿色版 `release/homedesktop.exe`（3.15 MB，实测运行 ~26MB 内存）

## 快速上手

```powershell
# 开发运行
$env:Path = "$env:USERPROFILE\.cargo\bin;D:\Document\Temp\Harness\HomeDesktop\.toolchain\wdk\w64devkit\bin;$env:Path"
$env:COREPACK_HOME = "D:\Document\Temp\Harness\HomeDesktop\.corepack"
corepack pnpm tauri dev

# 前端测试
corepack pnpm test
# Rust 核心测试（加 RUSTUP_TOOLCHAIN）
$env:RUSTUP_TOOLCHAIN = "stable-x86_64-pc-windows-gnu"
cargo test -p homedesktop-core

# 打包
corepack pnpm tauri build
```

## 本机环境要点（重要，都是踩过的坑）

1. **npm 不可用**（Node 24 移除了 zlib.Zlib，npm 11 的 minizlib 崩溃）→ 一律用 **corepack pnpm**（`COREPACK_HOME` 指向仓库内 `.corepack/`）
2. **无 MSVC** → Rust **GNU 工具链** + **w64devkit GCC**（`.toolchain/wdk/`）；`.cargo/config.toml` 指定 `target = "x86_64-pc-windows-gnu"` + `linker = "gcc"`；项目目录 rustup override
3. **w64devkit 缺 libgcc_eh.a** → 已创建空库 + 全 profile `panic = "abort"`（workspace 根 `Cargo.toml`）
4. **cdylib 在 GNU 下报 export ordinal** → `crate-type = ["rlib"]`（移动端需要时再加回 staticlib/cdylib）
5. **测试 exe 无法运行**（无 manifest → comctl32 v5 缺 v6 API）→ 纯逻辑已拆到 `crates/homedesktop-core` 独立测试；`cargo test`（src-tauri 壳）不要跑
6. crates 走 rsproxy 镜像（`.cargo/config.toml`）；GitHub 慢可走 ghfast.top 或代理 `10.10.1.2:7890`
7. tauri 的 `beforeDevCommand`/`beforeBuildCommand` 用 `node scripts/run-pnpm.cjs`（本地找 .corepack，CI 回退 PATH 中的 pnpm）

## 明天的候选方向（P2，未做）

1. **插件市场**：本地 zip 插件包安装（manifest + 前端组件）
2. **主题**：浅色/深色切换
3. **壁纸图片**：目前只支持纯色/渐变，可加本地图片文件
4. **应用抽屉 / 自动扫描已安装应用**：从系统扫描应用填充"所有应用"列表（现在是插件手动添加）
5. **托盘图标** + 开机自启
6. **macOS/Linux 实测**：CI 矩阵已配好，本地只有 Windows

## 常见操作

- 改代码后：`corepack pnpm run check`（svelte-check）+ `corepack pnpm test`；Rust 改 core crate 后 `cargo test -p homedesktop-core`
- 重启应用：杀掉 homedesktop 进程后重新 `corepack pnpm tauri dev`（Rust 改动 tauri dev 不自动重建 core crate，需重启）
- 布局/配置存在 `%APPDATA%\dev.homedesktop.app\`（layout.json / config.json / plugins/）
