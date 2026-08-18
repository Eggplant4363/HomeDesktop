# M16 设计：插件 v2 —— zip 插件可携带自定义 JS 小组件

> 状态：方案稿（2026-02-13，待用户确认）｜ 目标：让第三方插件**不修改框架代码**就能提供新类型小组件（倒计时/汇率/监控…）

## 1. 背景与约束

- 现状：小组件（时钟/天气/日历/系统监控/待办）都是**框架内置**的 Svelte 组件，`widgetComponent` 映射到前端注册表；第三方 zip 插件只能做"图标 + 动作 + 设置"，无法提供自己的小组件 UI。
- 约束：
  - webview = 系统 WebView2（Chromium），可执行现代浏览器 JS。
  - **无法在运行时编译 Svelte 源码** → 插件必须携带**预编译的浏览器可执行代码**。
  - 插件目录在 `%APPDATA%\dev.homedesktop.app\plugins\<id>\`，asset 协议 scope 已覆盖 `$APPDATA/**` → 插件目录内静态文件可通过 `asset://localhost/<绝对路径>` 加载。

## 2. 核心方案：Web Component（自定义元素）

插件包携带一个**自定义元素** JS 文件；框架负责加载该 JS、在桌面网格里实例化元素、并提供"设置桥"。

### 2.1 manifest 扩展（向后兼容，新增可选字段）

```json
{
  "id": "com.example.timer",
  "name": "倒计时",
  "version": "1.0.0",
  "type": "widget",
  "emoji": "⏱️",
  "widgetComponent": "__plugin__",        // 占位：表示"插件自带组件"（框架内置值保留原语义）
  "widgetFile": "widget.js",              // 新增：插件目录内的 JS 文件（自定义元素定义）
  "widgetElement": "hd-timer-widget",     // 新增：自定义元素标签名
  "sizes": [{ "w": 2, "h": 1 }, { "w": 2, "h": 2 }],
  "settings": [
    { "key": "target", "label": "目标秒数", "type": "number", "default": 60 }
  ]
}
```

- 旧插件（无 `widgetFile`）完全不受影响。

### 2.2 前端加载机制（WidgetTile 内新分支）

1. `WidgetTile` 发现 `plugin.widgetFile` → 进入**插件组件宿主**：
   - 用 asset 协议构造脚本 URL：`asset://localhost/<plugins目录绝对路径>/widget.js`。
   - 首次加载：动态 `<script src=...>`（全局去重缓存，避免重复加载）。
   - 加载完成后：`document.createElement(plugin.widgetElement)` 挂入 tile，元素 `setAttribute("cell-id", item.id)`。
   - 组件卸载（切页/删除）：移除元素（脚本保持已加载，下次直接创建元素）。
2. 桥接 API（挂全局，插件 JS 内使用）：

```js
window.__homedesktopPlugin = {
  getSetting(cellId, key, fallback),   // 读实例设置（config_get，含插件级回退）
  setSetting(cellId, key, value),      // 写实例设置（config_set，立即生效）
};
```

- 设置菜单复用现有 `PluginSettingsMenu`（按 manifest.settings 渲染、按 cellId 存储）→ 插件用 `getSetting` 读到自己实例的配置。

### 2.3 安全

- 插件 JS 运行在 webview 主上下文，与现有 `command` 动作同级权限（可 invoke 任意命令）→ 沿用"仅安装可信插件"提示。
- 明确的沙箱（iframe 隔离）列为 v2.1，不做进 M16。

## 3. 明确不做（v2.1 再说）

- iframe 沙箱隔离
- 插件访问框架内部 store（只暴露设置桥）
- 样式隔离（自定义元素可用 shadow DOM 自带样式；框架只提供最小容器）

## 4. 验收标准

1. zip 安装一个带 `widgetFile` 的插件 → ＋ 菜单可见、可添加到桌面 → 渲染出自定义元素 → 正常显示。
2. 编辑模式 ⚙ 设置生效：改设置 → 插件元素经 `getSetting` 读到新值（配合现有"保存后刷新"）。
3. 切页/重启：脚本只加载一次，元素随挂载重建，不重复加载。
4. 卸载插件 → 桌面该插件图标/元素消失。
5. 旧插件（无 widgetFile）行为不变。
6. 示例插件包（倒计时）+ 文档更新。

## 5. 实施拆解

| 步骤 | 内容 |
|---|---|
| 1 | core：`Manifest`/`PluginInfo` 增加可选 `widget_file`/`widget_element`（serde）+ 解析测试 |
| 2 | 前端 `types.ts` 同步 |
| 3 | 前端 `src/core/pluginWidgetHost.svelte.ts`：asset URL 构造、脚本加载缓存、元素创建/销毁（抽纯函数可单测） |
| 4 | `WidgetTile.svelte`：widgetFile 分支 → 宿主挂载 |
| 5 | 示例插件 `plugins-dist/demo-timer/`（widget.js 自定义元素 + manifest + zip） |
| 6 | `docs/PLUGIN_API.md` widget 章节更新 + CHANGELOG |
| 7 | 验证：core 测试 / vitest / svelte-check / dev 实测 + 日志 |
