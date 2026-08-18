## 2026-02-13（自由摆放：主页面 + 文件夹内）⏳ 待验收

- **需求**（用户提出）：支持自由摆放，主页面与文件夹内部都可；吸附网格、不自动重排
- **实现**：
  - 布局 schema **v2 → v3**：Cell/IconItem 增加 x/y（网格坐标，serde 缺省）；core 迁移 v1→v3 幂等（旧布局按 PAGE_COLS=12 / FOLDER_COLS=6 虚拟网格分配坐标）
  - 渲染：主页面（Grid）与文件夹（FolderView）改为**虚拟画布 + 绝对定位**（不再 CSS grid 流式排布）；画布宽度固定 12/6 列，高度随内容；＋ 按钮自动放在首个空位
  - 拖拽：编辑模式拖动 → 单元跟随指针 → 落点**吸附网格**（虚线幽灵框预览）→ 目标槽被占用**交换位置**；悬停文件夹可拖入；边缘翻页跨页移动带坐标
  - stores：setCellPosition / setIconPosition（占用即交换）、addCell / addIconToFolder / moveIconToFolder 自动找空位、moveCellAcrossPages 改为 (id, x, y)
- **验证**：core 22/22、vitest 82/82、svelte-check 0 错误 0 警告
## 2026-02-13（发布 v0.2.1：小版本更新）✅

- **内容**（自 v0.2.0）：网络代理设置、在线市场索引 SHA 固定、移动图标仅限编辑模式、图标区黑框修复
- **发布流程**：release/v0.2.1 分支 → 版本号 0.2.0→0.2.1 → 合并回 main → 打标签 v0.2.1 → CI 自动构建三平台并创建草稿 Release
## 2026-02-13（交互修复：移动仅限编辑模式 + 图标区黑框）✅ 用户验收通过

- **移动仅限编辑模式**（用户要求"非编辑模式下不能移动"）：桌面网格与文件夹内，鼠标拖动只在 ✎ 编辑模式拾取；非编辑模式点击正常启动；触屏长按仍为进入编辑模式的入口
- **图标区外圈黑框修复**（用户反馈：空白处按键出现黑色边框）：网格容器 tabindex=-1 点击聚焦后，再按键触发 Chromium 默认 focus ring → 禁用 .grid 的 outline（含 :focus/:focus-visible），图标自身键盘焦点描边保留
## 2026-02-13（网络代理设置）✅

- **需求**（用户提出"添加代理的功能"，此前已规划）：应用内可配置网络代理，用于在线市场等网络请求
- **实现**：
  - 设置面板新增「网络代理」区块：模式（不使用 / HTTP / SOCKS5）+ 地址 + 端口 + 可选用户名密码，一键保存
  - 持久化：config.json 的 proxy.mode / proxy.host / proxy.port / proxy.username / proxy.password
  - Rust：ureq 升级 socks-proxy feature；market_agent() 按配置构造 Agent（Proxy::new 支持 user:pass@）；market_remote_list / market_remote_install 统一走该 Agent；代理信息写入操作日志
  - 无代理（mode=none）时行为与之前完全一致
- **验证**：cargo check 通过；svelte-check 0 错误 0 警告
## 2026-02-13（发布 v0.2.0：在线插件市场）✅

- **新功能**：在线插件市场（GitHub 仓库 HomeDesktopPlugins，jsDelivr CDN 分发；市场页新增在线页签；plugin.marketUrl 可配置）
- **修复**：在线安装卡死（URL 斜杠 + 异步化 + 下载进度显示）；卸载确认框/toast 被弹层遮挡（z-index 分层修正）
- **发布流程**：release/v0.2.0 分支 → 版本号 0.1.0→0.2.0 → 合并回 main → 打标签 v0.2.0 → CI 自动构建三平台并创建草稿 Release
## 2026-02-13（修复：在线安装卡死 + 卸载不可用）✅

- **问题**（用户实测反馈）：
  1. 市场在线安装点击后**卡住**：下载 URL 少斜杠（base trim 掉结尾 / 后直接拼文件名 → marketdemo-notepad-...），请求 404/超时 42s；且命令是**同步**的，阻塞主线程 → 整个 UI 冻结
  2. 插件**卸载不了**：卸载操作本身正确，但被冻结的 UI 吞掉点击；且 dev 模式下项目 plugins/ 目录存在同名插件，卸载用户副本后列表仍显示（变为"内置"标记）
- **修复**：
  - plugins.rs：URL 拼接改为 {base}/{file}（统一一个分隔符）；market_remote_list/market_remote_install 改为 **async + spawn_blocking**（下载在阻塞线程池执行，不卡 UI）；新增 **IPC Channel 下载进度**（received/total，64KB 分块推送）
  - MarketList.svelte：在线安装显示**进度标签**（"安装中 45%"；无 Content-Length 时显示"安装中…"），防连点
  - App.svelte：卸载后若 dev 目录仍有同名插件，toast 明确提示"用户副本已卸载（开发目录仍有同名内置插件）"
- **验证**：cargo check 通过；svelte-check 0 错误 0 警告；修复后 URL 实测 200
## 2026-02-13（产品决策：规划"应用内代理设置"功能，未实现）

- **需求**（用户提出，原话大意："后续可以在软件里面设置一个代理的功能"）：应用内可配置网络代理，用于在线市场下载等网络请求
- **背景**：在线市场默认走 jsDelivr CDN（raw.githubusercontent 在国内网络不可达）；代理功能可让用户用自己的代理访问任意市场源
- **方案要点（待实现）**：设置面板加代理配置（类型 HTTP/SOCKS5、地址、端口、可选用户名密码）；Rust 侧 ureq Agent 应用代理；持久化到 config.json；已记入 PRD「明确不做」清单（规划后续）
- **状态**：⏳ 规划中，未排期
## 2026-02-13（在线插件市场：GitHub 仓库拉取安装）✅

- **需求**（用户提出）：插件市场建一个 GitHub 仓库（HomeDesktopPlugins），应用可以从仓库下载插件。
- **实现**：
  - 新仓库 **https://github.com/Eggplant4363/HomeDesktopPlugins**（公开）：market/index.json 索引 + 插件 zip 包（demo-timer / demo-cmd）
  - 后端：新增 market_remote_list(url)（拉取索引，标记已安装）+ market_remote_install(base, file)（下载 zip 到本地 market/ 后复用安装链路；64MB 上限、文件名防穿越）；HTTP 客户端用 **ureq 2**
  - 前端：MarketList 拆为 **本地市场 / 在线市场** 双页签；在线页签一键安装
  - **网络适配**：raw.githubusercontent.com 在国内网络不可达（实测 000）→ 默认索引走 **jsDelivr CDN**（实测 200）；zip base 自动跟随索引目录；支持配置 plugin.marketUrl 覆盖
- **验证**：cargo check 通过；svelte-check 0 错误 0 警告；vitest 83/83；jsDelivr 索引与两个 zip 均 200
## 2026-02-13（发布 v0.1.0 Release）✅

- **发布**：打标签 0.1.0 → workflow 新增标签触发 + release 任务（softprops/action-gh-release），三平台构建后自动创建**草稿 Release**
- **修复**：bundle 产物上传路径 bug —— 产物在工作区根 	arget/release/bundle/（cargo workspace 共享 target），原路径 src-tauri/target/... 匹配不到文件 → 三平台安装包从未上传（CI 日志 No files were found... No artifacts will be uploaded）；改为 	arget/release/bundle/**/* + 	arget/*/release/bundle/**/*（覆盖 mac 交叉目标）
- **产物**（8 个，全部上传成功）：Windows NSIS exe + MSI、macOS aarch64 dmg、Linux deb/rpm/AppImage（+ deb 内部 tar.gz×2）
# 变更日志 — HomeDesktop

> 按 skill `product-driven-project` P5 要求维护：代码与文档同步记录。
> 本文件即本项目长期记忆（Memory）：每轮迭代/每个 bug 修复/每个产品决策都记录于此。

## 2026-02-13（发布到 GitHub + CI 三平台全绿）✅

- **发布**：仓库 https://github.com/Eggplant4363/HomeDesktop （私有，SSH 密钥 id_ed25519_homedesktop，提交身份 Eggplant4363）；README 重写（功能清单/插件示例/打包说明）+ 主界面截图 + MIT LICENSE
- **CI 修复历程**（build.yml，三平台矩阵 Windows/macOS/Linux）：
  1. Windows 缺 WebView2Loader.dll（exe 静态导入）→ 构建前从 NuGet（Microsoft.Web.WebView2）拉取
  2. macOS/Linux 编译失败：windows 0.61 的无条件依赖把 windows-future 0.2.1 拖进依赖树，其 bindings.rs 在非 Windows 引用 windows_core::imp::IMarshal/marshaler（上游 bug，无 0.2.2 修复版）；winreg 0.55 在非 Windows 直接 compile_error! → 两个依赖都改为 [target.'cfg(windows)'.dependencies]，apps.rs::scan_registry_app_paths 加 cfg 门控 + 非 Windows 空实现（stats.rs 原有 cfg 桩）
  3. macOS/Linux 仍失败：tauri.conf.json bundle.resources 引用 resources/WebView2Loader.dll，仅 Windows 由 build.rs 从 target 复制 → DLL 入库（1.0.3650.58，与发布产物一致；Windows 上 build.rs 仍用 webview2-com-sys 生成版覆盖）
- **结果**：三平台 Actions 全绿（Windows NSIS/MSI、macOS dmg、Linux deb/AppImage 产物自动上传 artifact）
# 变更日志 — HomeDesktop

> 按 skill `product-driven-project` P5 要求维护：代码与文档同步记录。
> 本文件即本项目长期记忆（Memory）：每轮迭代/每个 bug 修复/每个产品决策都记录于此。

## 2026-02-13（插件通知能力：系统右下角 toast）✅ 用户验收通过

- **需求**（用户提出）：给插件加**通知能力**（不是界面按钮）——比如倒计时到点时在桌面右下角弹通知。
- **实现**：
  - 后端：接入 **`tauri-plugin-notification`**（`lib.rs` 注册 `.plugin(tauri_plugin_notification::init())`）；新增 **`app_notify(title, body)`** 命令（`src-tauri/src/notify.rs`，经 `NotificationExt` 弹 Windows toast，成功写操作日志 `系统通知: …`）；capabilities 增加 `notification:default`
  - 插件桥：`window.__homedesktopPlugin.notify(title, body)`（shim + 宿主 `handleBridgeMessage` 的 `notify` 分支 → `invoke("app_notify")`，成功回包 `true`）
  - 示例：`demo-timer` 倒计时**到点发一次**通知（实例标志位防重，目标秒数变化重置）；manifest 新增设置 **`enableNotify`（到点通知，默认开）**；widget.js/manifest 同步到 `plugins/`（dev 扫描）并重新打包 `demo-timer-1.0.0.zip`（市场种子同步更新）
- **文档**：`PLUGIN_API.md` 桥 API 表新增 `notify` 行 + 通知能力说明
- **验证**：cargo check 通过；vitest 83/83（新增 `handleBridgeMessage` notify 用例）；svelte-check 0 错误 0 警告

### 修复（用户实测没看到通知）
- **根因 ①**：`enableNotify` 设置类型误写为 `"boolean"` → 设置类型合法值是 `text|number|select|toggle`，manifest 解析失败（日志 `unknown variant 'boolean'`）→ 改为 **`"toggle"`**（`plugins-dist/` 与 `plugins/` 两处 + 文档示例）
- **根因 ②**：用户数据目录 `%APPDATA%\dev.homedesktop.app\plugins\dev.homedesktop.demo-timer\` 里是**旧版插件**（无 notify 的旧 widget.js）→ 直接覆盖更新用户副本（widget.js + manifest.json），并重新打包 zip + 更新市场种子
- 重启 dev 验证：invalid manifest 错误消失、`插件小组件已挂载(iframe): dev.homedesktop.demo-timer`（加载新版 widget.js）

## 2026-02-13（插件独立页面：已安装 / 市场 双页签）✅ 用户验收通过

- **需求**（用户提出）：插件单独做一个页面——一个"已安装"、一个"市场"。
- **实现**：
  - 新增 **`PluginsPanel.svelte`**：单页两个页签——**已安装**（插件列表：类型/版本/内置标记 + 更新/卸载，卸载仍走确认框）/**市场**（复用新抽出的 `MarketList`：浏览/刷新/安装）
  - 入口统一：**＋ 菜单「🧩 插件管理（已安装 / 市场）…」**；设置里原"插件列表 + 市场按钮"合并为一个「🧩 打开插件管理…」按钮
  - 卸载后列表同步（plugins store 响应式）、市场每次打开重扫（状态同步）
  - 删除废弃的 MarketPanel（逻辑并入 MarketList/PluginsPanel）
- **验证**：svelte-check 0 错误 0 警告、vitest 80/80

## 2026-02-13（再次打包发布：含 M16 插件 v2 + 沙箱 + 市场）

- 全量 release 构建（含 M16 全部：插件 v2 自定义小组件、iframe 沙箱、本地插件市场）：**NSIS 1.56MB / MSI 2.19MB / 绿色 exe 4.15MB**
- **静默安装实测**：`$INSTDIR` 含 exe + WebView2Loader.dll + 6 个内置插件（calendar/clock/sysmonitor/system-apps/todo/weather）；安装版运行正常（窗口响应）
- 产物已更新到 `release-packages/`（安装版实测目录 `.install-test/` 可自行运行）

## 2026-02-13（插件市场交互优化：独立面板 + 卸载状态同步）

- **需求**（用户提出）：① 市场要有按钮直接打开；② 卸载后市场列表要同步显示"未安装"。
- **实现**：
  - 新增独立 **`MarketPanel.svelte`** 面板（浏览/刷新/安装）；**＋ 菜单新增「🛒 打开插件市场…」按钮**，设置里「插件市场」区块也改为「打开插件市场…」按钮
  - 面板**每次打开重新扫描** → 在设置里卸载插件后重开市场，状态自动同步为"未安装"；安装成功也立即刷新状态
- **验证**：svelte-check 0 错误 0 警告、vitest 80/80

## 2026-02-13（插件市场目录：本地 market/ 浏览安装）

- **需求**：插件 v2 进阶第 2 步——本地插件市场。
- **Rust**：`market_scan` 命令——扫描 app 数据目录 `market/*.zip`，逐个读取 zip 内 manifest（core `ManifestInfo` 补 emoji 字段）列出 名称/版本/类型/是否已安装；`market_install` 复用现有 `plugins_install`
- **前端**：设置面板新增「插件市场（本地目录）」区块——显示市场目录路径、🔄 刷新列表、每个包 [安装]（已安装显示标签）；打开设置自动刷新
- **使用方式**：把 zip 插件包放进 `%APPDATA%\dev.homedesktop.app\market\` → 设置里点安装（已放 demo-timer / demo-cmd 两个示例包）
- **验证**：cargo check 通过、svelte-check 0 错误 0 警告、vitest 80/80；已重启 dev（tauri-dev45）

## 2026-02-13（Bug 修复：沙箱 iframe 内倒计时空白）

- **根因**：sandbox iframe（无 same-origin）内用 `<script src=asset://…>` **跨源加载 asset 被沙箱阻挡** → 插件代码没执行 → 空白。
- **修复**：改为**主窗口 fetch 读取插件 JS 内容**（同源 asset 与壁纸一致），**内联进 srcdoc**（`</script>` 转义防截断）；沙箱内不再有外部脚本加载。
- **验证**：svelte-check 0 错误 0 警告、vitest 80/80（新增转义测试）；已重启 dev（tauri-dev44）

## 2026-02-13（M16 进阶：插件 iframe 沙箱隔离）

- **安全升级**：插件自带小组件 JS 改为在 **sandbox iframe**（`allow-scripts`、无 same-origin）内运行——**无法访问主应用上下文、无法直接 invoke 系统命令**（之前与 command 动作同级权限）
- **桥重构**：iframe 内 shim 定义 `window.__homedesktopPlugin`，经 postMessage 转发 get/set 设置；宿主 `handleBridgeMessage` 响应；**插件开发 API 不变**（demo-timer 无需改动）
- **点击联动**：iframe 内点击（非交互元素，可加 `data-hd-noopen` 跳过）转发给宿主 → 打开设置菜单
- **验证**：svelte-check 0 错误 0 警告、vitest 79/79（新增 srcdoc 构造测试）；已重启 dev（tauri-dev43）
- 文档：PLUGIN_API §5.4 更新沙箱说明；PLUGIN_V2_DESIGN 待补充沙箱实施记录

## 2026-02-13（M16：插件 v2 进阶）✅ 用户验收通过（日志佐证：插件小组件已挂载）

- **manifest 新字段** `widgetFile`/`widgetElement`（+ `PluginInfo.dir`）：core 同步 + 测试；旧插件不受影响
- **宿主机制**：`pluginWidgetHost`（asset URL 用官方 convertFileSrc 编码 + 作用域运行时放行 dev 插件目录 + 脚本去重加载 + 元素实例化 + `window.__homedesktopPlugin` 设置桥）
- **交互**：点击小组件 = 打开设置（内部交互元素不触发）；倒计时示例目标秒数变化即重新计时
- **示例插件**：`plugins-dist/demo-timer-1.0.0.zip`「⏱️ 倒计时」
- **验证**：core 25/25、vitest 78/78、svelte-check 0/0；日志确认 `插件小组件已挂载: dev.homedesktop.demo-timer hd-timer-widget`

## 2026-02-13（M16 交互：点击小组件打开设置 + 倒计时改时间重开始）

- **需求**（用户提出）：点击插件小组件时设置定时时间。
- **实现**：
  - `WidgetTile` 正常模式点击 → 打开该插件设置菜单（Enter 键盘同样可用）；**点击落在组件内部按钮/输入框等交互元素时不触发**（待办等带控件的小组件不受影响）
  - 示例倒计时 `widget.js`：**目标秒数变化时重新开始倒计时**（设置新时间立即从头计）
- **验证**：svelte-check 0 错误 0 警告、vitest 78/78；已重启 dev（tauri-dev42）

## 2026-02-13（Bug 修复：插件脚本仍加载失败 → asset URL 格式）

- **二次根因**：放行作用域后仍失败——手拼的 `asset://localhost/<原始路径>` 是**未编码**格式；Tauri 2 的 asset 协议要求 **`convertFileSrc` 生成的编码格式**（项目内壁纸正是用 convertFileSrc 才正常）。
- **修复**：`pluginWidgetUrl` 改用官方 `convertFileSrc`（含 URL 编码）；纯路径拼接抽为 `pluginWidgetPath`（可单测）；宿主新增 `加载插件脚本: <url>` 诊断日志。
- **验证**：svelte-check 0 错误 0 警告、vitest 78/78；已重启 dev（tauri-dev41）

## 2026-02-13（Bug 修复：插件自带小组件脚本加载失败）

- **现象**（用户报告）：添加倒计时插件显示"插件脚本加载失败"。
- **根因**：asset 协议作用域只覆盖 `$APPDATA/**`；**dev 模式插件目录在项目根 `plugins/`（不在 APPDATA 下）**，`asset://` 加载被安全策略拒绝（zip 装到用户数据目录的不受影响）。
- **修复**：启动时 `plugins::allow_asset_scope` 把所有插件目录（用户数据/资源/dev 项目根）**动态加入 asset 协议作用域**（`allow_directory` 递归放行）。
- **验证**：cargo check 通过；已重启 dev（tauri-dev40）

## 2026-02-13（M16：插件 v2 进阶——zip 插件可携带自定义 JS 小组件）🚧 已实现待验收

- **manifest 新字段**：`widgetFile`（插件目录内 JS 文件）+ `widgetElement`（自定义元素标签名），`widgetComponent` 占位 `"__plugin__"`；core `Manifest`/`PluginInfo` 新增 + `PluginInfo.dir`（插件目录绝对路径）；旧插件不受影响（+2 例 core 测试）
- **宿主机制**：`pluginWidgetHost.svelte.ts`——`assetUrlFor`（路径→asset 协议 URL，防双斜杠）+ `loadPluginScript`（全局去重只加载一次）+ `createPluginElement`（cell-id 属性注入）；注入 **`window.__homedesktopPlugin` 桥**（`getSetting`/`setSetting` 读写实例设置）；`WidgetTile` 新增插件组件分支 → `PluginWidgetHost.svelte`（加载失败显示重试）
- **示例插件**：`plugins-dist/demo-timer-1.0.0.zip`「倒计时」——自定义元素（shadow DOM 自带样式）+ manifest（settings 目标秒数），已同步到 dev 项目 plugins/ 便于开发调试
- **文档**：PLUGIN_API.md 新增 §5.4（插件自带小组件：字段/写法/桥 API/安全提示）
- **验证**：core 测试 25/25、cargo check、svelte-check 0 错误 0 警告、vitest 78/78（新增 assetUrlFor 2 例）；已重启 dev（tauri-dev39）

## 2026-02-13（M15：重新打包发布）✅ 完成并验证

- **重新 `tauri build`**（含 M6–M14 全部新功能）：NSIS 安装包 1.55MB / MSI 2.18MB / 绿色 exe 4.13MB
- **修复打包缺口**：clock/weather 的 manifest 此前只存在于 dev 的项目根目录，未进发布包（安装版会没有时钟/天气小组件）——补进 `resources/plugins/` + `bundle.resources` 后重建，发布包内置插件 6 个（clock/weather/calendar/sysmonitor/todo/system-apps）
- **静默安装实测**：NSIS `/S` 安装到临时目录 → `$INSTDIR` 含 homedesktop.exe + **WebView2Loader.dll**（nsis-hooks 生效）+ `resources/plugins/` 6 个插件目录；安装版运行正常（窗口响应、应用数据完好）
- **说明**：release 构建日志按需求默认关闭（homedesktop.log 不生成），运行验证以"进程/窗口/资源齐全"为准
- **产物**（`release-packages/`）：`HomeDesktop_0.1.0_x64-setup.exe`、`HomeDesktop_0.1.0_x64_en-US.msi`、绿色 `homedesktop.exe` + `WebView2Loader.dll`

## 2026-02-13（M14：性能优化——图标请求并发限制）

- **问题**：系统应用面板一次挂载会并发发起上百个 `app_icon` 请求，洪泛 IPC 与后端线程池（之前"刷新慢/卡"的诱因之一）
- **修复**：`appIcons.loadAppIcon` 加**并发信号量（同时最多 6 个）**，其余排队；同路径仍去重、磁盘缓存仍生效，图标渐进式显示
- **验证**：svelte-check 0 错误 0 警告、vitest 76/76

## 2026-02-13（M15：重新打包发布）🚧 构建中

## 2026-02-13（M13 补充：导入时可选"覆盖 / 合并"）

- **需求**（用户询问后选择）：导入备份时让我选——覆盖 or 合并（避免"备份后新增的图标被覆盖掉"）
- **合并语义**：core 新增 `merge_layout`（**按图标 id 去重**，含文件夹内：保留当前布局，追加备份中当前不存在的单元到对应页/新页）+ `merge_config`（按键合并、备份胜出）；+3 例 core 测试
- **Rust**：`backup_import` 增加 `mode` 参数（"merge"=合并，其余=覆盖）
- **前端**：导入备份 → 先选文件 → 弹「导入方式」选择（覆盖导入 / 合并导入 / 取消），说明文字讲清两者区别
- **验证**：core 测试 22/22、cargo check、svelte-check 0 错误 0 警告、vitest 76/76；已重启 dev（tauri-dev38）

## 2026-02-13（M13：布局/配置导出导入）🚧 已实现待验收

- **导出**：`backup_export` 命令把 layout.json + config.json 合并写为单个备份文件（含 app 标识/版本/时间戳）；设置面板「数据备份」→ 📤 导出备份…（保存对话框）
- **导入**：`backup_import` 校验备份文件（app 标识 + layout 存在；layout 走 v1→v2 迁移）→ 先备份当前文件为 .bak → 写回 layout.json + config.json → 返回摘要（页数/单元/配置键数）；前端提示后自动 `location.reload()` 全量生效
- **非法文件**：格式错误 / 非本应用备份 → 明确报错提示
- **验证**：cargo check 通过、svelte-check 0 错误 0 警告、vitest 76/76；已重启 dev（tauri-dev37）

## 2026-02-13（Bug 修复：应用图标重启后变成首字母）

- **现象**（用户报告）：重启应用后，桌面应用图标变成首字母（头像回退），不显示真实图标。
- **排查**（日志/实证定位）：
  - Rust 侧磁盘缓存**确认正常**：`DefaultHasher` 跨进程稳定（实测 3 次同值），360zip/AnxReader/Foxmail 三个路径的缓存 PNG 全部存在且有效（1709-2894B）；重启时 `app_icon` 命中缓存返回 data URL
  - 问题在前端：`appIcons` 缓存用了 `$state(new Map())`——**$state 的 Map 代理在模块级可能不触发模板重渲染**（项目其他响应式全是对象/数组，唯独此处用 Map），导致 data URL 已写入缓存但 `{#if appIcons.get(path)}` 不更新，一直显示头像
- **修复**：`appIcons` 改为 `$state<Record<string, string>>` 普通对象（与 pluginSettings 一致，已被验证实时生效）；AppIcon 改用 `appIcons[path]`；新增诊断日志 `app_icon 结果: <path> -> N bytes / null`
- **验证**：svelte-check 0 错误 0 警告、vitest 76/76；干净重启后日志确认 `app_icon 结果: …anx_reader.exe -> 2302 bytes`（data URL 已入前端缓存）
- **经验沉淀**：`$state` 响应式缓存用**普通对象/数组**，避免 Map/Set 代理在模块级失效的坑

## 2026-02-13（M12：小组件扩充——日历 / 系统监控 / 待办）🚧 已实现待验收

- **日历** `CalendarWidget`：月历视图（周日起始），左右翻月，今天高亮；月历网格计算抽为纯函数 `calendar.ts`（+3 例单测）
- **系统监控** `SysMonitorWidget`：CPU/内存使用率 + 进度条；Rust `sys_stats` 命令（Windows：GetSystemTimes 两次采样差值算 CPU、GlobalMemoryStatusEx 算内存），widgetRuntime 每 3 秒后台刷新（切页不重载）；非 Windows 返回 None 显示"…"
- **待办清单** `TodoWidget`：本地存储 `todo.<实例>.items`（config.json），**按实例独立**；添加/勾选完成/删除
- **内置插件**：新增 3 个 manifest（calendar/sysmonitor/todo，widgetComponent 映射注册表；dev `plugins/` + 发布 `resources/plugins/` 并加入 bundle.resources）
- **验证**：cargo check 通过、svelte-check 0 错误 0 警告、vitest 76/76（新增日历 3 例）；已重启 dev（tauri-dev34）
- **过程记录**：首次重启插件仍 6 个——日志定位到 `invalid plugin manifest: missing field`，新 manifest 缺必填 `actions` 字段（core Manifest 要求）；补 `"actions": []` 后插件 9 个 ✅（dev35 日志佐证）

## 2026-02-13（M11：插件管理——卸载/更新）🚧 已实现待验收

- **内置标记**：core `PluginInfo` 新增 `builtin` 字段（serde 省略 false）+ `collect_plugins_with_builtin`（按来源目录索引标记：索引 0=用户数据目录可卸载，其余=内置）——插件列表可区分"内置 / 用户安装"
- **卸载**：新命令 `plugins_uninstall`（删除用户数据 `plugins/<id>/`，内置插件拒绝）；前端确认框（删除类操作统一确认）→ 卸载后 `removeCellsByPlugin` **自动清理桌面上该插件的图标**（页面 + 文件夹内）→ 刷新列表
- **更新**：复用 zip 安装（`plugins_install` 覆盖旧目录），设置面板每行「更新」按钮
- **设置面板「插件」区块**：列出所有插件（emoji/名称/类型/版本/内置标记），用户安装的显示 [更新][卸载]
- **验证**：core 测试 19/19（新增来源标记 + builtin 序列化）、cargo check、svelte-check 0 错误 0 警告、vitest 73/73（新增 removeCellsByPlugin 2 例）；已重启 dev（tauri-dev33）

## 2026-02-13（M10：全局搜索面板）🚧 已实现待验收

- **快捷键唤起**：`shortcuts.rs` 升级为多动作系统——新增 `search` 动作（默认 **Ctrl+Space**，可配置），按下后 Rust 发 `homedesktop:search` 事件，前端监听打开/关闭面板；`shortcuts_set` 命令增加 `action` 参数（togglePad / search）
- **搜索面板** `SearchPanel.svelte`：居中浮层，**跨「桌面图标/文件夹 + 插件 + 系统应用」**搜索；↑↓ 选择、Enter 激活、Esc/点背景关闭；应用结果显示真实图标（复用 app_icon 磁盘缓存）
- **激活行为**：桌面图标 → 走正常启动（含系统应用插件拦截）；文件夹 → 打开文件夹视图；插件 → 执行插件动作；系统应用 → 新增 `launch_path` 命令直接启动（不经过桌面布局）
- **设置**：快捷键区块新增「打开全局搜索」配置行
- **验证**：cargo check 通过、svelte-check 0 错误 0 警告、vitest 71/71；已重启 dev（tauri-dev32）
> **产品约定（用户 2026-02-13 提出）**：**所有删除操作都必须弹确认框**（页面/图标/文件夹/文件夹内图标…），统一走 App 的通用确认框。

## 2026-02-13（删除操作全部加确认）

- **通用确认框**：App 新增 `confirmAction` 状态 + `askConfirm(title, message, onConfirm)`/`runConfirm`——所有删除统一弹「标题 + 说明 + 取消/删除（红色）」。
- **覆盖范围**：
  - 删除页面（−）✅（上轮已加，本轮并入通用框）
  - 删除图标（编辑模式 ×）→ 确认「删除图标「xxx」？」
  - 删除文件夹（×）→ 确认「删除文件夹「xxx」？文件夹内 N 个图标将一并删除」
  - 删除文件夹内图标 → 确认（FolderView ondelete 改为传 folderId+iconId，由 App 统一处理）
- **验证**：svelte-check 0 错误 0 警告、vitest 71/71
> **验收约定（用户 2026-02-13 提出）**：每个用户可操作的功能必须写**操作日志**（如"文件夹内重排: …"），
> 验收时以**运行时日志（%APPDATA%\dev.homedesktop.app\homedesktop.log）为证据自行判断是否生效**，不依赖口头确认。

## 2026-02-13（Bug 修复：切换页面按钮"消失"= 误删页 + 防误删保护）

- **现象**（用户报告）：点底部 ‹/› 切换页按钮几次后，按钮组消失了。
- **根因**（日志定位）：`删除页面 1` + `layout_save: 1 页`——页面真的被删了（2 页→1 页，按钮组只在 >1 页时显示）。删除按钮「−」紧挨在 › 右边，快速点切换时**误触 −**，把当前页（含内容）静默删掉了。
- **修复**：
  - **删除页面前弹确认框**：「删除第 N 页？该页所有图标将被移除且无法撤销」[取消/删除]（红色删除按钮）
  - **layout 自动备份**：`layout_save` 写前把旧 layout.json 复制为 `layout.json.bak`（单槽位，意外删除可手动恢复）
- **验证**：cargo check 通过、svelte-check 0 错误 0 警告、vitest 71/71；已重启 dev（tauri-dev31）
- **说明**：本次误删的第 1 页内容当时没有备份，无法恢复（备份从本次修复起生效）

## 2026-02-13（性能：应用图标加载加速）

- **根因**（用户报告"应用图标有时候刷新比较慢"）：`app_icon` 是**同步命令**（占 UI 主线程），每次都要重新 SHGetFileInfo + GetDIBits + PNG 编码；首次打开应用面板要串行提取上百个图标；重启后内存缓存丢失全部重提
- **修复**（`apps.rs`）：
  - `app_icon` 改为 **async**，GDI 提取放 `tauri::async_runtime::spawn_blocking`（阻塞线程池，不卡 UI）
  - **磁盘持久化缓存**：提取成功后写入 `%APPDATA%\dev.homedesktop.app\icons\<path hash>.png`，下次（含重启后）直接读文件秒出
- **验证**：cargo check 通过；已重启 dev（tauri-dev30）

## 2026-02-13（M9：图标自定义）🚧 已实现待验收

- **layout schema 扩展**（M9 前提）：IconCell 新增可选字段 `emoji/color/iconPath`；**Rust Cell::Icon + IconItem 同步**（serde default + skip），往返持久化不丢字段（core 测试含自定义字段往返断言）
- **编辑模式 📝 按钮**：图标 tile 新增"自定义"按钮 → 弹窗：
  - **重命名**（标题）
  - **换 emoji**（12 个预设 + 手输，留空=回退插件默认）
  - **换颜色**（8 色预设圆点，留空=透明）
  - **借用系统应用图标**：🎨 打开系统应用面板（新增 borrow 模式）选一个应用，用它的真实图标
- 渲染：`IconCell.iconPath` → 真实系统图标；`color` → 彩色圆角底；`emoji` 覆盖插件 emoji；`updateIconAppearance` 统一更新（空字符串=清除）
- **验证**：cargo check 通过、core 测试 17/17（含自定义字段往返）、svelte-check 0 错误 0 警告、vitest 71/71（新增 M9 store 测试 3 例）
- **注意**：改 schema 后必须重启 dev（旧二进制持久化会丢新字段）——已重启（tauri-dev29）

## 2026-02-13（M8：文件夹交互补全）✅ 用户验收通过（日志佐证：文件夹内重排 / 编辑文件夹）

- **文件夹内拖拽排序**：`FolderView` 增加 Pointer Events 拖拽（鼠标移动>4px 即拖；触屏长按 350ms 进入编辑并拾取）——拖到另一图标前/后出现指示条，空白处释放追加到末尾；`reorderIconInFolder(folderId, dragId, targetId|null, pos)` 持久化
- **文件夹编辑**：编辑模式文件夹 tile 新增 📝 按钮 → 弹窗改名称 / 换 emoji（含 12 个预设 emoji 快捷选择）；`renameFolder` / `setFolderEmoji`
- **验证**：svelte-check 0 错误 0 警告、vitest 68/68（新增 M8 store 测试 4 例）；运行时日志确认用户实际操作（拖拽重排 + 两次改名保存）

## 2026-02-13（插件设置改为按实例独立）

### 再修复：设置仍不马上生效（用户二报）
- **新根因**：① 设置菜单文本/数字输入框用 `onchange`（**失焦/回车才触发保存**）——用户输入完直接看小组件，保存根本没执行；② 前一次修复依赖"设置菜单主动刷新"，但刷新时机被 onchange 卡住
- **修复**：
  - 文本/数字输入改为 **`oninput`**（每敲一个字即保存，实时更新设置缓存）
  - 刷新**防抖 500ms**（连续输入只刷一次）：保存后清掉该实例缓存 → 触发该实例刷新
  - `refreshWidget` 改为返回 `boolean`（是否真正执行拉取），保存后日志记录"刷新完成/未执行"，便于排查
- **诊断日志**（dev 可见于 homedesktop.log）：天气实例挂载 id、天气拉取的城市、设置保存后刷新结果
- **验证**：svelte-check 0 错误 0 警告、vitest 64/64（新增 widgetRuntime 实例隔离 4 例）；干净重启 dev 日志确认"天气实例挂载: <cellId> 城市=济南"按实例工作

## 2026-02-13（插件设置改为按实例独立）

### 补充修复：设置保存后不立马生效（用户报告）
- **根因**：天气的"城市"在**数据拉取时**读取——改城市只更新了设置缓存，天气数据要等 30 分钟定时刷新（或重开应用）才用新城市重新拉取，所以看起来"没生效"（时钟的开关是响应式派生，本来就即时）
- **修复**：`PluginSettingsMenu.save` 保存后**立即触发该实例刷新**——先 `delete widgetCache[instanceId]`（避免短暂显示旧数据），再 `refreshWidget(instanceId)`（图标插件无 widget 模块时是 no-op）
- **验证**：svelte-check 0 错误 0 警告、vitest 60/60

## 2026-02-13（插件设置改为按实例独立）

- **需求**（用户报告）：桌面放了多个天气小组件，改其中一个的城市，所有天气都跟着变（都变成济南）——希望**每个插件生成的图标有独立的实例设置**。
- **根因**：设置原来按插件 id 全局共享（`plugin.<id>.<key>`），所有同插件图标共用一个配置。
- **实现**：
  - `pluginSettings.svelte.ts` 新增实例级 API：`getCellSetting`/`setCellSetting`/`peekCellSetting`，键 `cell.<cellId>.<key>`；**未设置时回退插件级默认**（兼容旧共享配置 `plugin.<id>.<key>`），改一个实例只影响它自己
  - 小组件按实例工作：`WidgetTile` 把 `cellId` 传给组件；天气/时钟读实例设置
  - **天气数据缓存按实例隔离**：`registerWidget` 的 id 用实例 id（cellId），不同城市的天气小组件各刷各的、互不影响（原按插件 id 共享缓存）
  - `PluginSettingsMenu` 按实例保存（App 传入 `settingsTarget` cellId）
- **文档**：PLUGIN_API §5.2/§5.3 更新为按实例 API 示例
- **验证**：svelte-check 0 错误 0 警告、前端 vitest 60/60（新增 pluginSettings 实例隔离 4 例）

## 2026-02-13（Bug 修复：设置面板点击即关闭）

- **现象**（用户报告）：点快捷键录入区域，设置面板立刻消失，无法修改快捷键。
- **根因**：外观设置面板遮罩 `onclick={() => (showSettings = false)}` 是**任意点击都关闭**（其他设置项如主题/背景点一下也会误关）；快捷键录入需要面板保持打开，所以最先暴露。
- **修复**：
  - App.svelte 设置遮罩改为**仅点击背景关闭**（`e.target === e.currentTarget`，与插件设置菜单/添加菜单一致）
  - ShortcutRow 键盘监听改为**捕获阶段**（`window.addEventListener(..., true)`）：录制中按 Esc/组合键先于面板的 Esc 关闭逻辑处理，Esc 只取消录制、不再连带关闭面板
- **验证**：svelte-check 0 错误 0 警告、vitest 56/56

## 2026-02-13（M7：可配置全局快捷键）

- **快捷键不再写死**：`lib.rs` 不再用 builder 预注册 Alt+Space，改为 setup 时按配置动态注册（`shortcuts.rs`：`load_toggle` 读配置 `shortcuts.togglePad`，缺失/非法回退默认 `alt+space`）
- **动态修改**：命令 `shortcuts_set`——先注册新键（被其他应用占用则报错、旧键保持不变）→ 注销旧键 → 持久化，立即生效
- **设置面板「快捷键」区块**：`ShortcutRow.svelte`（点击进入"按下组合键…"录制态，Esc 取消），保存显示格式化（"Alt + Space"）+ 错误提示；`shortcuts.ts` 纯函数（`normalizeCombo`/`comboLabel`/`comboFromEvent`）+ 4 例单测
- 明确不做（用户确认）：图标级快捷键、翻页/搜索聚焦等其他动作
- 验证：cargo check 通过、svelte-check 0 错误 0 警告、前端 vitest 56/56

## 2026-02-13（M6：Launchpad 全屏 Pad）

- **全局快捷键 = Pad 开关**（`lib.rs` global-shortcut handler 改造，默认 Alt+Space）：窗口隐藏 → 显示并进全屏 Pad；全屏中 → 隐藏（Pad 消失）；窗口模式 → 进全屏
- **顶栏 ⛶ 按钮**：窗口模式 ⇄ 全屏 Pad 切换（`getCurrentWindow().setFullscreen`）；**Esc** 全屏中退出回窗口
- **记忆上次状态**：`system.ts` 读写配置 `ui.fullscreen`；启动时若上次为全屏 → 直接进全屏 Pad；窗口尺寸变化（含 Rust 热键触发的全屏切换）经 `onResized` 同步前端状态并自动记忆
- 规划文档：PRD 新增两个 P1 功能、milestones M6/M7 定稿（形态=同一张桌面铺满；快捷键仅 Pad 开关，不做图标级快捷键）
- 验证：cargo check 通过、svelte-check 0 错误 0 警告、前端 vitest 52/52

## 2026-02-13（编辑模式操作统一）

- **小组件 tile 补齐 📁 移入文件夹按钮**：`WidgetTile` 新增 `onmove`（图标/小组件/文件夹视图统一按钮布局：⇲ 📁 ⚙ ×）
- **文件夹视图内图标补齐 📁**：`FolderView` 新增 `onmove` 并传给 IconTile/WidgetTile → App `handleMove`
- **`moveIconToFolder` 支持文件夹之间移动**：源查找扩展到文件夹 items（文件夹 A → B），且不能移进自己所在的文件夹；新增 2 例测试
- 规则说明（有意为之的部分）：⚙ 设置仅当**插件声明了 settings** 才显示（应用无插件设置 → 无 ⚙）；文件夹固定 1×1、不可嵌套 → 无 ⇲/📁，仅 × 删除
- 验证：svelte-check 0 错误 0 警告、前端 vitest 52/52

## 2026-02-13（系统应用插件化 + 真实图标）

### 「添加系统应用」改造成内置插件（用户需求）
- **manifest 新增动作 `system_apps`**：core `ActionKind::SystemApps`（serde "system_apps"）；后端 `execute_action` 不执行该动作（由前端处理）；core 测试 16/16
- **内置插件「📱 系统应用」**（`plugins/system-apps/manifest.json` + 发布包 `resources/plugins/system-apps/` 随 bundle.resources 分发）：manifest 声明 `kind:"system_apps"`，像普通插件一样加入桌面；点击 → 前端拦截打开系统应用面板
- **系统应用面板** `SystemAppsPanel.svelte`：扫描注册表 App Paths + 开始菜单快捷方式，搜索过滤；启动拦截在 App.svelte `handleLaunch`（图标自带动作/插件动作任一为 system_apps 即打开面板）
- **槽位替换**（用户优化 2）：点击「系统应用」图标 → 面板选中应用 → `replaceCellWithApp`（stores）**原位替换**该图标（标题/pluginId/启动动作就地更新，页面与文件夹内均支持，位置不变）；想要多个应用就放多个「系统应用」图标
- **＋ 菜单移除应用区块**（用户优化 1）：AddMenu 删除「应用（系统已安装）」列表，添加应用统一走「系统应用」插件；真实图标改为面板/桌面图标按需懒加载
- **真实系统图标**：
  - Rust `app_icon` 命令（`apps.rs`）：Windows SHGetFileInfo 取 HICON → GetDIBits 取 32bpp 像素（颜色位图自带 alpha 则直用，否则 1bpp 掩码补透明）→ core `encode_rgba_png`（新增 image crate png 特性，纯函数可单测）→ base64 data URL；非 Windows 返回 None
  - 前端 `appIcons.svelte.ts`（$state Map 缓存 + 并发去重）、`AppIcon.svelte`（真实图标懒加载，加载中/失败回退首字母头像）；IconTile / AddMenu / SystemAppsPanel 统一接入
- 文档：PLUGIN_API.md §3 新增 `system_apps` 动作与内置插件示例；README 更新
- 验证：cargo check 通过、core 测试 16/16、svelte-check 0 错误 0 警告、前端 vitest 50/50

## 2026-02-13（插件开发者 API 文档）

- **新增 `docs/PLUGIN_API.md`**：面向第三方插件开发者的完整 API 文档——插件是什么/扫描顺序、manifest v2 全部字段、actions（app/command）、settings 统一设置菜单声明、widget 开发（组件注册表 + widgetRuntime 缓存/定时刷新 + 读取插件设置）、分发与安装（zip 规范 + 校验规则）、完整示例、版本历史
- **示例安装包升级 v2**：`plugins-dist/demo-cmd/manifest.json` 升级到 2.0.0（新增 `sizes` 1×1 + `settings` 打开地址/自动打开），重新打包为 `plugins-dist/demo-cmd-2.0.0.zip`（zip 根含 manifest.json，已校验内容），删除旧 1.0.0 zip；README 插件市场条目指向新包与 API 文档

## 2026-02-13（插件统一 API：尺寸声明 + 设置菜单）

### 插件 API v2（小米/安卓 widget 设计）
- **manifest 新增 `sizes`**：插件声明支持的尺寸档（如时钟/天气：2×1、2×2、4×2；图标类固定 1×1）——**切换大小只能选插件声明的那几档**（编辑模式 ⇲ 弹层按插件过滤）
- **manifest 新增 `settings`**：插件声明可配置项（`{key,label,type,options,default}`，type: text/number/select/toggle）——**框架提供统一设置菜单**（安卓风格：编辑模式点 ⚙）
- core：`SettingSpec/SettingType/SettingOption` + Manifest/PluginInfo 增加 `sizes/settings`（含默认值解析）；core 测试 13/13
- 前端：`pluginSettings.svelte.ts`（统一键值存储 `plugin.<id>.<key>` + $state 缓存响应式）、`PluginSettingsMenu.svelte`（统一渲染表单）；tile 编辑模式新增 **⚙ 设置按钮**（插件声明了 settings 才显示）
- 应用图标（非插件）固定 1×1；添加 widget 默认尺寸 = 插件 sizes[0]
- **内置插件接入**：
  - 时钟：sizes 三档 + 设置「显示秒/显示日期」开关（实时生效）
  - 天气：sizes 三档 + 设置「城市(文本)/温度单位(下拉 摄氏/华氏)」；天气配置从小组件内 ⚙ 迁移到统一设置菜单（旧 `weather.city` 配置自动迁移）
- 验证：svelte-check 0 警告、前端 44/44、core 13/13；应用已重启

## 2026-02-13（编辑模式 — 苹果风格）

### 编辑模式（用户需求：进入编辑模式后才出现缩放/删除等操作）
- **进入**：顶栏 **✎ 编辑**按钮（鼠标）；**触屏长按图标**（350ms，iOS 同款：进入编辑并拾取拖拽）
- **编辑模式表现**：
  - 所有图标**抖动**（wiggle 动画，苹果风格）
  - 每个图标**常显操作按钮**：⇲ 缩放 / 📁 移入文件夹 / × 删除（不再依赖悬停）
  - 可拖拽排序/移入文件夹/跨页（编辑模式下触屏小幅移动即拾取）
  - **点击图标不启动**（正常模式才启动）；文件夹不可打开
- **退出**：顶栏 **✓ 完成**按钮
- 正常模式：图标干净（无悬停按钮），点击即启动
- 实现：`stores.ui.editMode`（Svelte 模块 $state 不能整体重新赋值，用对象属性）；IconTile/WidgetTile/FolderTile 新增 `editMode` prop
- 验证：svelte-check 0 警告、前端 44/44；应用已重启

## 2026-02-13（小组件定时机制 + 深浅色独立壁纸）

### ① 小组件运行时（框架级机制，解决"切页重载"）
- **业界参考**：Android AppWidget / iOS WidgetKit——小组件数据由宿主统一管理，生命周期与页面切换解耦
- 新增 `src/core/widgetRuntime.svelte.ts`：
  - `registerWidget({id, refreshMs, fetch})`：插件声明刷新周期，全局调度器每分钟检查、过期即**后台刷新**（与页面切换无关）
  - `widgetCache`（$state）：数据缓存常驻模块级，**切页不丢**；组件挂载直接渲染缓存，刷新后自动更新
- `WeatherWidget` 重构：注册 30 分钟周期；挂载先渲染缓存（无网络请求），过期才后台刷新；改城市即时刷新

### ② 深浅色独立壁纸（Android 13+/iOS 同款方案）
- `appearance` 改为 `backgrounds: { dark, light }` 分开存储；`getCurrentBackground()` 按当前主题取
- 设置时作用于**当前主题**（UI 标注"当前为 X 主题"）；旧配置自动迁移（单背景 → 深色）
- 验证：svelte-check 0 警告、前端 44/44；应用已重启（日志确认前端加载、主题 light、迁移生效）

## 2026-02-13（安装包 WebView2Loader.dll 问题修复 ✅）

### 问题
- 安装版（NSIS）启动报"找不到 WebView2Loader.dll"：exe **静态导入**该 DLL，进程启动时必须与 exe 同目录
- tauri 的 `bundle.resources` 把 DLL 放到 `resources/` 子目录（exe 加载器不会去那里找）→ 安装后无法启动

### 修复（三层配合）
1. `build.rs`：在 `tauri_build::build()` **之前**把 webview2-com-sys 生成的 DLL 复制到 `src-tauri/resources/`（tauri-build 会校验 resources 路径，顺序必须在先）
2. `bundle.resources: ["resources/WebView2Loader.dll"]`：DLL 打进安装包（resources/ 目录）
3. **`nsis-hooks.nsh` + `bundle.windows.nsis.installerHooks`**：安装完成后把 DLL 从 `resources/` 复制到**安装根目录（exe 同目录）**
- 注：曾尝试 webview2-com-sys 的 `dynload` feature（0.38 已移除），已回退
- **验证**：静默安装 → exe 同目录出现 WebView2Loader.dll（156.6KB）→ **安装版启动成功**（~27MB）；release-packages 已更新；现有安装 `D:\Software\HomeDesktop` 已手动补齐 DLL 立即可用

## 2026-02-13（新版本打包完成 ✅）

- **release 重新打包成功**（含全部 P2 功能 + 日志系统 + 三项修复）：
  - `bundle/nsis/HomeDesktop_0.1.0_x64-setup.exe` — **1.35 MB**
  - `bundle/msi/HomeDesktop_0.1.0_x64_en-US.msi` — 2.0 MB
  - `release/homedesktop.exe` — 3.79 MB，实测运行正常（~28MB 内存）
- **release 日志关闭验证**：release 启动后日志文件不写入（init 清空后因级别 OFF 不再生成）✅

## 2026-02-13（修复全部验收通过 ✅）

- 用户确认：托盘左键菜单、壁纸、图标尺寸三项问题**全部修复验证通过**（尺寸根因为 `.drop-wrap` 包裹层导致 grid span 失效，已修复并实测：1x2/2x2/4x3 等整数倍尺寸正常渲染）
- 重新打包 release 安装包（含全部 P2 功能 + 日志系统 + 修复）

## 2026-02-13（图标尺寸渲染修复 — 根因）

### 根因（日志系统立功）
- 日志确认：`调整单元尺寸: 时钟 -> 4x3` 已触发且 `layout_save` 已保存（数据正确）
- **渲染层 bug**：M4 引入拖拽的 `.drop-wrap` 包裹层后，`grid-column/row: span` 样式仍写在**内层 tile 组件**上——**CSS Grid 的 span 只作用于网格容器直接子元素**，被包裹后全部失效 → 所有图标/小组件实际恒为 1×1
- 这也解释了为何 M3/M4 小组件看起来"还行"（时钟/天气文字大，1×1 也能看）但尺寸调整从未真正生效

### 修复
- `Grid.svelte`：span 样式移到 `.drop-wrap`（真正的网格子元素）上：`style="grid-column: span {cell.size.w}; grid-row: span {cell.size.h};"`
- `IconTile.svelte` / `WidgetTile.svelte`：移除根节点上的 span 样式；IconTile 高度改为 100% 适配跨行
- `FolderView.svelte`：图标/小组件加 `.cell-wrap` 包裹层并携带 span（文件夹内原为直接子元素，需同步）
- 验证：svelte-check 0 警告；HMR 已实时应用，应用运行中

## 2026-02-13（日志系统 + 图标大小诊断）

### 日志系统（用户需求）
- **Rust 分级日志**（`src-tauri/src/log.rs`）：级别 debug < info < warn < error < off；写入 **`%APPDATA%\dev.homedesktop.app\homedesktop.log`**（每次启动清空重建）；开发构建默认 **DEBUG**（记录全部操作），release 构建默认 **OFF**（用户要求关闭）；可用环境变量 `HOMEDESKTOP_LOG_LEVEL` 覆盖
- **前端日志**（`src/core/logger.ts`）：`log.debug/info/warn/error` → `invoke("log_write")` 写入同一日志文件；开发时同时输出到控制台
- 已埋点：启动信息（插件数/页数/主题/tileSize）、添加图标/应用/文件夹、删除、拖拽排序/移入文件夹、启动图标（launch_cell）、尺寸调整、主题/背景/壁纸、开机自启、安装插件、页面增删、托盘操作
- 关键命令 Rust 侧日志：`launch_action`、`launch_cell`、`layout_save`、托盘事件

### 图标大小诊断
- `handleSetTileSize` / `handleApplySize` 增加 **toast 操作反馈**（"图标大小 → Npx" / "尺寸已设为 W×H"）+ 日志
- 若点击后无 toast → 点击未触发（事件问题）；有 toast 但网格不变 → 渲染问题——用日志精确定位
- 注：本次 dev server 端口冲突（旧 vite 占 1420 导致新实例退出），实际运行实例为旧 dev server 自动重建的新版本（含全部修复 + 日志），已验证日志文件正常写入

## 2026-02-13（P2 问题修复）

### 用户反馈的 3 个问题（全部修复）
1. **托盘左键弹菜单** → `TrayIconBuilder.show_menu_on_left_click(false)`：菜单只在右键出现，左键 = 显示/隐藏窗口
2. **壁纸不生效** → asset 协议作用域写错：`$APPDATA` 已含应用标识（=`Roaming\dev.homedesktop.app`），原作用域 `$APPDATA/dev.homedesktop.app/**` 多拼一层导致拒绝加载；改为 **`$APPDATA/**`**（已设置的 wallpaper.png 2.9MB 现存于数据目录，重启即生效）
3. **图标大小不能调整** → P2 给设置弹层新增区块后内容超高，`.settings` 缺 `max-height/overflow` 导致"图标大小"被挤出窗口底部无法点击；加 **`max-height: 80vh; overflow-y: auto`**
- 验证：cargo check 通过（仅无害的缓存硬链接警告）、svelte-check 0 警告；应用已重启（新二进制）

## 2026-02-13（P2 迭代①–⑤ 完成，待验收）

### P2 功能（今日全部完成）
1. **主题（浅色/深色）**：⚙ 设置 → 🌙深色/☀️浅色；CSS 变量双色板（`[data-theme]`），组件硬编码颜色全部替换为变量（`--bg-input/--bg-hover/--border`）
2. **壁纸图片**：⚙ 设置 → 🖼 选择图片壁纸（tauri-plugin-dialog 选图 → Rust `set_wallpaper` 拷贝到 app 数据目录 → `convertFileSrc` + asset 协议渲染）；`tauri` 开启 `protocol-asset` feature + asset 作用域 `$APPDATA/dev.homedesktop.app/**`
3. **托盘图标 + 开机自启**：`TrayIconBuilder`（左键单击显示/隐藏 + 菜单：显示隐藏/退出）；`tauri-plugin-autostart`（⚙ 设置开关，`autostart:default` 权限）
4. **应用抽屉**：Rust `apps_scan`（注册表 App Paths + 开始菜单 .lnk 用 windows COM `IShellLinkW` 解析，winreg + windows 0.61）；图标单元新增可选 `action`（自带启动动作，`launch_cell` 优先 action 回退插件）；AddMenu 新增"应用"区（首字母彩色头像）+ 搜索；core 新增 `parse_manifest_info`
5. **插件市场 MVP**：Rust `plugins_install`（zip 校验 manifest + 防目录穿越解压到用户 plugins 目录 + 自动发现）；AddMenu "📦 从 zip 安装插件…"（dialog 选 zip → 安装 → 刷新列表）
- 依赖新增：`tauri-plugin-dialog`、`tauri-plugin-autostart`、`zip`、`winreg`、`windows`；`tauri` features: `protocol-asset`、`tray-icon`
- 测试：core **13/13**、前端 **44/44**、svelte-check 0 警告、cargo check 通过
- 应用已重启（新二进制），待用户验收

## 2026-02-13（M5 ✅ 全部里程碑完成）

### M5 — 打包验证（完成）
- **`tauri build`（release，LTO + opt-level s）成功产出**：
  - `target/.../release/bundle/nsis/HomeDesktop_0.1.0_x64-setup.exe` — **1.11 MB**
  - `target/.../release/bundle/msi/HomeDesktop_0.1.0_x64_en-US.msi` — **1.68 MB**
  - `target/.../release/homedesktop.exe` — **3.15 MB**
  - 远超 PRD 15MB 轻量目标；NSIS/WiX 由 tauri-bundler 自动下载（走代理成功）
- **release exe 实测运行正常**（内存 ~26MB，低于 debug）
- CI 工作流（`.github/workflows/build.yml`）win/mac/linux 三平台矩阵已就绪；`scripts/run-pnpm.cjs` PATH 回退保证 CI 可跑
- 说明：bundle identifier `dev.homedesktop.app` 在 macOS 上可能有 `.app` 后缀警告（Windows 无影响），后续如需上架 macOS 可调整

### 项目最终状态（2026-02-13）
- 全部里程碑 M0–M5 完成；产品 = 手机式桌面启动器（图标+小组件同屏混排、文件夹、多页、拖拽排序/跨页、搜索、外观设置、Alt+Space 热键）
- 测试：前端 44/44、Rust core 12/12、svelte-check 0 警告
- 打包体积：安装包 1.1MB / exe 3.15MB / 内存 ~26MB

## 2026-02-13（M4 ✅ / M5 打包进行中）

### M4 验收通过（用户确认 2026-02-13）
- **拖拽排序**（Pointer Events 自实现，Tauri WebView2 原生 DnD 不可靠且触屏不支持）：
  - 鼠标移动 4px 即拖；触屏**长按 350ms 拾取**（`touch-action: pan-y` 保证滚动不冲突）
  - **槽位计算**：支持拖到空行/行尾空白/页面边缘（模拟网格排布 + 指针行列 → 插入位置）
  - **跨页移动**：`moveCellAcrossPages` 按 id 全页查找源单元（边缘翻页拖拽不再弹回）；`moveIconToFolder` 同样跨页
  - 拖后抑制误触点击（suppressClick）
- **全局热键**：`tauri-plugin-global-shortcut`，**Alt+Space** 显示/隐藏窗口（已并入 M4）
- **外观设置**（⚙）：5 个背景预设 + 自定义颜色、图标大小 小/中/大（`--tile-size`，保持整数倍粒度），持久化于 config.json
- 测试：前端 **44/44**（新增跨页移动/跨页入文件夹 4 个）；svelte-check 0 错误 0 警告
- CI 兼容：`scripts/run-pnpm.cjs` 增加 PATH 回退（CI 无 .corepack 也能跑）

### M5 打包（进行中）
- [ ] `tauri build` release 构建中（LTO + opt-level s；NSIS/MSI 安装包）
- [ ] 验证 release exe 运行 + 安装包产物
- [ ] CI 三平台工作流核对

## 2026-02-13（M4 进行中：拖拽排序 / 全局热键 / 外观）

### M4 功能
- **拖拽排序**（`Grid.svelte` 容器级事件委托）：
  - 拖图标/小组件/文件夹到目标位 → **插入指示条**（前/后，按鼠标水平位置）
  - 拖到**文件夹**上 → 虚线高亮 → 松手移入文件夹
  - 拖到**窗口左右边缘** 650ms → 自动翻页（可连续翻）
  - `reorderCellById`（stores，页内重排）+ 持久化；新增 4 个排序单测（前端 40/40）
- **全局热键**：`tauri-plugin-global-shortcut`，**Alt+Space** 显示/隐藏主窗口（类似 PowerToys Run）；capabilities 增加 `global-shortcut:default`
- **外观设置**（⚙ 按钮，`appearance.svelte.ts` + `config.json` 持久化）：
  - 背景：5 个预设（深蓝/暮蓝/暖橙/森林/紫夜）+ 自定义颜色（color picker）
  - 图标大小：小 64 / 中 84 / 大 104（通过 `--tile-size` CSS 变量实时生效，网格保持整数倍）
- 验证：svelte-check 0 错误 0 警告；前端测试 40/40；cargo check 通过
- 应用已重启（新二进制），待用户验收

## 2026-02-13（尺寸粒度需求 / M4 前置）

### 尺寸：1×1 基准整数倍（用户需求）
- **固定基准粒度**：网格改为固定列宽 `repeat(auto-fill, var(--tile-size))` + `grid-auto-rows: var(--tile-size)` + 居中——去掉 `minmax(..., 1fr)` 弹性伸缩，任何尺寸都是 1×1 基准的**严格整数倍**（跨 N 列 = N×基准 + (N-1)×gap）
- **整数倍尺寸选择器**：悬停图标/小组件 → **⇲** → 弹层选择（1×1、2×1、1×2、2×2、3×2、4×2、4×3），仅整数选项
- `setCellSize`（stores，自动钳制为 ≥1 正整数；主网格 + 文件夹内都支持）+ 持久化
- 移除 WidgetTile 残留的 `large`/`1.6` 小数倍样式
- 测试：新增尺寸测试 4 个（前端 36/36）；svelte-check 0 错误 0 警告

## 2026-02-13（需求澄清 v1.1 / M3 ✅）

### 需求变更（PRD v1.1，2026-02-13 用户澄清）
- **无独立"中控模式"**：应用本身就是一张手机式桌面（参考小米桌面），图标 + 小组件**同屏混排**，无模式切换
- 小组件（时钟/天气）从 P1 提为 **P0**，直接渲染在桌面网格中（已实现：WidgetTile 在网格/文件夹内渲染，2x1 占位）
- 代码调整：删除 `Dashboard.svelte` 与 App 中的 mode/全屏逻辑（⛶ 按钮、enterDashboard/exitDashboard、getCurrentWindow 引用）；桌面 = Grid（图标/文件夹/小组件混排）+ 搜索 + 多页 + 添加
- 文档同步：PRD v1.1、milestones（M3 改判 ✅）、architecture（移除 Dashboard 模块）
- 验证：svelte-check 0 错误 0 警告；前端测试通过；core 12/12

## 2026-02-13（M2 ✅ / M3 进行中）

### M3 — 中控模式 + 小组件（进行中）
- **插件模型扩展**：manifest 增加 `widgetComponent`（旧 manifest 兼容解析）；core `PluginInfo` 新增 `widgetComponent` 字段；新增 widget manifest 测试（core 12/12）
- **小组件插件**：`plugins/clock`（🕐 时钟）、`plugins/weather`（🌤️ 天气）；前端 `src/widgets/` 注册表（`widgetComponent` → Svelte 组件 + 默认网格尺寸 2x1）
- **时钟小组件**：实时走时（HH:MM + 秒 + 日期），`$effect` + setInterval
- **天气小组件**：Open-Meteo（免费无 key，支持 CORS，WebView 直接 fetch）：地理编码 → 实时天气（温度/天气码/湿度/风速）；WMO 码→emoji/中文描述；⚙ 配置城市（存 `config.json`，经新 Rust 命令 `config_get/config_set`）
- **中控模式（Dashboard）**：顶部 ⛶ 切换；聚合所有小组件（跨页+文件夹内）大卡片展示；`core:window:allow-set-fullscreen` 权限 + `getCurrentWindow().setFullscreen()` 全屏切换；返回启动器按钮
- **网格渲染**：widget 类型图标 → `WidgetTile`（真实小组件）而非静态 emoji 块（主网格 + 文件夹内均支持）；AddMenu 显示"小组件"标签
- 测试：svelte-check 0 错误 0 警告；前端 32/32；core 12/12
- 待用户验收：添加时钟/天气 → 中控模式 → 全屏 → 重启持久化

## 2026-02-13（M1 ✅ / M2 ✅）

### M2 — 文件夹 / 多页（进行中）
- **布局 schema v2**：网格单元格改为 tagged enum `Cell = Icon | Folder`（`kind` 字段），`Layout` 增加 `version`；**v1→v2 自动迁移**（`migrate_layout`，幂等：旧图标项补 `kind:"icon"`、版本升 2）；用户 M1 已存的布局会自动迁移
- 核心逻辑仍全部在 `homedesktop-core`（新增 `Cell`/`IconItem`、迁移函数；serde 注意点：枚举 `rename_all` 只作用于变体名，结构体变体字段需变体级 `rename_all="camelCase"`）
- 前端：
  - `FolderTile`（文件夹块：emoji/名称/图标数，悬停删除）、`FolderView`（进入文件夹视图：返回/添加/删除内部图标）
  - 图标悬停新增 **📁 移入文件夹**（弹层选择文件夹；无文件夹时提示先创建）
  - AddMenu 新增"新建文件夹"（输入名称回车/按钮创建）
  - 多页：`＋页` 新增页面、翻页 ‹ ›、`−` 删除当前页（至少保留一页）
- **测试**：Rust core **11/11**（新增 v2 往返、v1 迁移、迁移幂等、kind 序列化）；前端 **32/32**（新增文件夹/页面/移入/搜索文件夹）；svelte-check 0 错误
- 应用已重启（新二进制），待用户验收：文件夹创建/展开/移入/多页/重启持久化

## 2026-02-13（M0 Done / M1 进行中）

### 已完成
- 创建开发流程 skill：`.dsh/skills/product-driven-project/SKILL.md`
- 产品文档：`docs/PRD.md`（v1.0）、`docs/tech-selection.md`（v1.0，Tauri 2 + Svelte 5）、`docs/architecture.md`（v1.0）、`docs/milestones.md`（v1.0，M0–M5）
- 脚手架（手工搭建，未用 create-tauri-app）：Svelte 5 + TS + Vite 6 前端 + Tauri 2 Rust 壳 + 2 个示例插件（demo-notepad / demo-web）
- **✅ M0 Done：应用可运行**——`homedesktop.exe`（debug 198MB / release 将显著缩小）启动成功，内存 ~40MB
- **单元测试**：
  - 前端 Vitest **21/21 通过**（`src/core/`：search / stores / pluginLoader / persistence）
  - Rust **homedesktop-core 8/8 通过**（manifest 解析、插件扫描去重、布局往返、动作错误路径）
  - 测试基础设施：`pnpm test`（vitest run）+ `cargo test -p homedesktop-core`；vite.config 增加 `test` 配置；CI 工作流已就绪
- **架构重构：crates/homedesktop-core** —— 纯逻辑（插件 manifest/扫描、布局、动作分发）拆成独立 crate，不依赖 tauri/WebView2，保证可独立单测；src-tauri 变薄封装
- 应用图标：`.toolchain/gen-icons.cjs` 纯 Node 生成（PNG+ICO，2x2 启动器网格样式）→ `src-tauri/icons/`
- CI：`.github/workflows/build.yml`（win/mac/linux 三平台矩阵）
- 工具链脚本：`scripts/run-pnpm.cjs`（tauri before*Command 用，规避 npm zlib 问题）

### 环境问题与解决（重要，本机专属）
- **Node 24 + npm 11 不兼容**（zlib.Zlib 被移除）→ 改用 **corepack pnpm 11.22.0**（`COREPACK_HOME=.corepack/`，store 在 `.pnpm-store/`）；pnpm 11 构建白名单在 `pnpm-workspace.yaml`（`allowBuilds.esbuild=true`）
- **无 MSVC 链接器** → 安装 **w64devkit GCC 16.2**（`.toolchain/wdk/`，ghfast.top 镜像 + bsdtar 解压）+ Rust **stable-x86_64-pc-windows-gnu** 工具链；`.cargo/config.toml` 指定 GNU target + `linker="gcc"`；项目内 rustup override
- **w64devkit 缺 libgcc_eh.a**（SEH 展开库）→ 创建空 `libgcc_eh.a` + 全 profile `panic="abort"`（workspace 根 Cargo.toml）
- **cdylib 在 GNU 下链接失败**（`export ordinal too large`，WebView2 符号）→ `crate-type=["rlib"]`（仅桌面；移动端需时再加回）
- **测试 exe 启动崩溃 0xc0000139**：无 manifest → comctl32 v5 缺 v6 API（SetWindowSubclass 等）；解法 = 纯逻辑拆到 homedesktop-core 独立测试（应用 exe 有 tauri 嵌入 manifest 不受影响）
- 沙箱：TLS（schannel）受限 → 构建类命令需完整权限执行；后续审批策略已改为 never + danger-full-access
- crates 镜像 rsproxy（`.cargo/config.toml`）；Rust 工具链经 rsproxy 安装（USTC 中途挂起弃用）；用户提供备用代理 `10.10.1.2:7890`
- GitHub 直连慢 → ghfast.top 镜像下载工具链

### M1 待办（下一步）
- [ ] 在运行的应用中验证：添加/删除图标、点击启动（notepad / 打开 GitHub）、重启布局恢复
- [ ] 补充"图标动作"人工验收清单（PRD 验收 1/2/4/5）
- [ ] M2：文件夹 / 多页 / 拖拽排序

### 命令备忘
```powershell
# 前端测试
corepack pnpm test
# Rust 核心测试
$env:Path = "$env:USERPROFILE\.cargo\bin;D:\...\.toolchain\wdk\w64devkit\bin;$env:Path"
$env:RUSTUP_TOOLCHAIN = "stable-x86_64-pc-windows-gnu"
cargo test -p homedesktop-core
# 开发运行
corepack pnpm tauri dev
```

### Bug 修复记录
- **插件列表为空（"暂无可用插件"）**：tauri dev 启动的进程 cwd 是 `src-tauri/`，原 `cwd/plugins` 扫描不到项目根 `plugins/`。修复：`plugin_dirs` 改为从 **exe 位置与 cwd 分别向上查找最多 6 层**收集 `<dir>/plugins`（去重、保序），并加 `[homedesktop] plugin dirs:` 诊断输出；已用完整 stderr 验证项目 `plugins/` 在扫描范围内。
