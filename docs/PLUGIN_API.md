# HomeDesktop 插件开发 API

> 版本：v2 ｜ 2026-02-13 ｜ 面向第三方插件开发者
> 对应框架：插件清单 schema v2（sizes/settings 为 v2 新增）

## 1. 插件是什么

插件 = **一个目录 + `manifest.json`**。放对位置后，HomeDesktop 启动时会自动发现，插件声明的图标/小组件就能被用户添加到桌面。

**插件目录（扫描顺序，先到先得）**：
1. 用户数据目录 `plugins/`（Windows: `%APPDATA%\dev.homedesktop.app\plugins\`）——用户/安装的插件
2. 应用资源目录 `plugins/`
3. 项目根 `plugins/`（开发时 `pnpm tauri dev` 直接生效，改完 manifest 重启应用即可）

## 2. manifest.json 完整字段

```json
{
  "id": "com.example.myplugin",
  "name": "我的插件",
  "version": "1.0.0",
  "type": "icon",
  "emoji": "🧩",
  "sizes": [{ "w": 1, "h": 1 }],
  "settings": [],
  "widgetComponent": null,
  "actions": []
}
```

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `id` | string | ✅ | 全局唯一标识，建议反域名格式 `com.example.xxx`；重复时高优先级目录覆盖 |
| `name` | string | ✅ | 显示名称 |
| `version` | string | ✅ | 语义化版本 |
| `type` | string | ✅ | `"icon"`（图标，点击执行动作）或 `"widget"`（小组件） |
| `emoji` | string | 否 | 图标 emoji（未设置时前端显示 📦） |
| `sizes` | Size[] | 否 | **支持的尺寸档**（v2）。`Size = { "w": 1, "h": 1 }`，1×1 基准整数倍。**用户切换大小时只能选这里声明的档**；添加时默认取第一档。不声明时：widget 回退框架默认档，icon 固定 1×1 |
| `settings` | Setting[] | 否 | **可配置项声明**（v2），框架提供统一设置菜单 |
| `widgetComponent` | string | widget 必填 | 前端小组件组件 id（见 §5，目前仅支持框架内置：`clock` / `weather`） |
| `actions` | Action[] | icon 建议 | 点击图标执行的动作（见 §3） |

## 3. 动作（actions）

```json
"actions": [
  { "kind": "app",     "path": "C:\\Program Files\\App\\app.exe" },
  { "kind": "command", "cmd": "start https://example.com" }
]
```

| kind | 字段 | 行为 |
|---|---|---|
| `app` | `path`（可执行文件/文件/URL 的绝对路径） | 启动应用（Windows: `start`；macOS: `open`；Linux: `xdg-open`） |
| `command` | `cmd`（命令字符串） | 执行命令（Windows: `cmd /C`；其他: `sh -c`） |
| `system_apps` | — | **前端动作**：点击后打开系统已安装应用列表（可搜索、显示真实图标、一键添加）。仅供框架内置插件使用（见下），后端不执行 |

### 内置插件示例：「系统应用」

框架自带一个用插件机制实现的内置插件（manifest 位于 `plugins/system-apps/manifest.json`，随安装包分发在 `resources/plugins/system-apps/`）：

```json
{
  "id": "dev.homedesktop.system-apps",
  "name": "系统应用",
  "version": "1.0.0",
  "type": "icon",
  "emoji": "📱",
  "sizes": [{ "w": 1, "h": 1 }],
  "actions": [{ "kind": "system_apps" }]
}
```

把它添加到桌面后，点击即弹出系统应用面板：列出注册表 App Paths + 开始菜单快捷方式扫描出的应用，每项显示**系统真实图标**（懒加载，`app_icon` 命令提取 exe/lnk 图标并缓存）。选中应用后，当前「📱 系统应用」图标会**原位替换**为该应用（标题/启动动作/真实图标一并就位，位置不变）；想要更多应用，就再添加几个「系统应用」图标（每个是一个槽位）。应用列表不再出现在「＋」菜单里，统一走此插件。

> 说明：`system_apps` 动作由前端处理（框架内置），普通插件请勿使用；zip 分发的插件若声明该动作，点击同样会打开系统应用面板。

> 安全提示：插件机制可执行任意命令，仅安装可信来源的插件。

## 4. 设置项（settings）——统一设置菜单

声明后，用户在**编辑模式**点击图标的 **⚙** 即弹出框架渲染的统一设置菜单。

```json
"settings": [
  { "key": "city",    "label": "城市",     "type": "text",   "default": "北京" },
  { "key": "max",     "label": "最大数量",  "type": "number", "default": 10 },
  { "key": "unit",    "label": "单位",      "type": "select",
    "options": [ { "label": "摄氏度 (°C)", "value": "celsius" },
                 { "label": "华氏度 (°F)", "value": "fahrenheit" } ],
    "default": "celsius" },
  { "key": "enabled", "label": "启用",      "type": "toggle", "default": true }
]
```

| 字段 | 说明 |
|---|---|
| `key` | 设置项唯一键（插件内） |
| `label` | 菜单里显示的标签 |
| `type` | `text` / `number` / `select` / `toggle` |
| `options` | 仅 `select`：`[{ label, value }]` |
| `default` | 默认值（未设置时生效） |

**存储约定**：设置值保存于应用配置 `config.json` 的键 `plugin.<插件id>.<settingKey>`。小组件可在运行时读取（见 §5）。

## 5. 小组件（widget）开发

### 5.1 组件注册

`widgetComponent` 映射到前端小组件注册表 `src/widgets/index.ts`：

```ts
export const widgetRegistry: Record<string, WidgetDef> = {
  clock: { component: ClockWidget, defaultSize: { w: 2, h: 1 } },
  weather: { component: WeatherWidget, defaultSize: { w: 2, h: 1 } },
};
```

> ⚠️ **当前限制（MVP）**：`widgetComponent` 必须是框架内置组件（`clock` / `weather`）。**暂不支持**在 zip 插件包中携带自定义前端组件代码（未来插件市场 v2 将支持加载插件自带 JS）。如需新增小组件类型，需在框架仓库扩展 `src/widgets/` 注册表。

### 5.2 数据缓存与定时刷新（框架提供，强烈建议）

网络型小组件**不要**在挂载时直接请求（切换页面会反复加载）。使用框架的小组件运行时：

```ts
import {
  registerWidget,      // 声明数据源 + 刷新周期
  refreshWidget,       // 手动后台刷新
  isWidgetStale,       // 缓存是否过期
  widgetCache,         // 响应式缓存（$state）
} from "../core/widgetRuntime.svelte";

// 缓存按"图标实例"隔离：不同实例（不同配置）互不干扰。组件拿到 cellId 后按实例注册：
let { cellId }: { cellId?: string } = $props();
const instanceId = cellId ?? "myplugin";

onMount(() => {
  registerWidget<MyData>({
    id: instanceId,                        // 实例 id（不是插件 id）
    refreshMs: 30 * 60_000,                // 30 分钟后台自动刷新（与页面切换无关）
    fetch: async () => { /* 拉数据并返回 */ },
  });
  if (isWidgetStale(instanceId, 30 * 60_000)) void refreshWidget(instanceId);
});

// 组件里：
const data = $derived(widgetCache[instanceId]?.data as MyData | undefined);
```

### 5.3 读取插件设置（按实例独立）

**每个图标/小组件实例有独立的设置**（config key `cell.<cellId>.<key>`）；未设置时回退插件级默认（`plugin.<pluginId>.<key>`，旧共享配置自动兼容）。小组件组件通过 `cellId` prop 拿到自己的实例 id：

```ts
import {
  getCellSetting,   // 异步加载（实例优先，回退插件级默认）
  peekCellSetting,  // 模板/派生里同步读（响应式）
  setCellSetting,   // 写入（只影响当前实例）
} from "../core/pluginSettings.svelte";

let { cellId }: { cellId?: string } = $props();
const instanceId = cellId ?? "com.example.myplugin";

// 异步加载（填充缓存）
await getCellSetting(instanceId, "com.example.myplugin", "city", "北京");
// 模板/派生里同步读（响应式）
const city = $derived(peekCellSetting<string>(instanceId, "com.example.myplugin", "city") ?? "北京");
```

### 5.4 插件自带小组件（插件 v2，M16）

不再局限于框架内置组件：插件 zip 可携带**预编译的自定义元素 JS**，框架加载后直接在桌面网格实例化。

**manifest 新增字段**：

| 字段 | 类型 | 说明 |
|---|---|---|
| `widgetFile` | string | 插件目录内的 JS 文件（自定义元素定义），如 `widget.js` |
| `widgetElement` | string | 自定义元素标签名，如 `hd-timer-widget` |
| `widgetComponent` | string | 固定写 `"__plugin__"`（占位：表示插件自带组件） |

```json
{
  "id": "com.example.timer",
  "name": "倒计时",
  "type": "widget",
  "widgetComponent": "__plugin__",
  "widgetFile": "widget.js",
  "widgetElement": "hd-timer-widget",
  "sizes": [{ "w": 2, "h": 1 }, { "w": 2, "h": 2 }],
  "settings": [
    { "key": "target", "label": "目标秒数", "type": "number", "default": 60 },
    { "key": "enableNotify", "label": "到点通知", "type": "toggle", "default": true }
  ]
}
```

**widget.js 写法**：定义并注册自定义元素；框架挂载时设置 `cell-id` 属性（实例 id）。通过全局桥读写实例设置：

```js
(() => {
  class MyWidget extends HTMLElement {
    connectedCallback() { this.render(); }
    async render() {
      const bridge = window.__homedesktopPlugin;
      const target = await bridge.getSetting(this.getAttribute("cell-id"), "target", 60);
      // ... 渲染（可用 attachShadow 自带样式）
    }
  }
  if (!customElements.get("hd-timer-widget")) {
    customElements.define("hd-timer-widget", MyWidget);
  }
})();
```

**桥 API `window.__homedesktopPlugin`**：

| 方法 | 说明 |
|---|---|
| `getSetting(cellId, key, fallback?)` | 读实例设置（`cell.<cellId>.<key>`，无则回退 fallback） |
| `setSetting(cellId, key, value)` | 写实例设置（立即持久化） |
| `notify(title, body)` | **通知能力**：发系统通知（Windows 右下角 toast），返回 Promise，成功解析 `true` |

> 设置菜单（编辑模式 ⚙）按 manifest.settings 渲染、按实例存储 → 插件用 `getSetting` 读到自己的配置。
> **安全（沙箱）**：插件 JS 运行在 **sandbox iframe**（`allow-scripts`，无 same-origin）内，**无法访问主应用上下文、无法直接调用系统命令**；设置读写与通知都经 postMessage 桥转发，只开放 `getSetting`/`setSetting`/`notify`。插件内部按钮等交互元素加 `data-hd-noopen` 属性可避免点击触发设置菜单。
> **通知能力**：`notify(title, body)` → 宿主 `app_notify` 命令 → `tauri-plugin-notification` 弹系统通知；建议到点事件只发一次（用实例标志位防重）。完整示例：`plugins-dist/demo-timer-1.0.0.zip`（倒计时到点发通知，`enableNotify` 开关可关）。

## 6. 分发与安装

- **开发调试**：把插件目录放进项目根 `plugins/`，`pnpm tauri dev` 启动即被扫描（重启应用刷新）。
- **打包分发**：把插件目录打成 **zip**（**zip 根目录必须包含 `manifest.json`**；可附带说明/资源文件，会一起解压）→ 用户应用内 **＋ → 📦 从 zip 安装插件…** 选择 zip 即可。安装到用户数据目录 `plugins/<id>/`，自动出现在列表中。
  - 示例安装包：`plugins-dist/demo-cmd-2.0.0.zip`（图标类：manifest + 说明.txt，演示 `sizes`/`settings`）、`plugins-dist/demo-timer-1.0.0.zip`（**自带小组件**：manifest + widget.js，演示 `widgetFile`/`widgetElement`）
- **在线市场（M18）**：应用「插件管理 → 市场 → 在线市场」从远程仓库拉取安装。仓库结构：
  - `market/index.json`：索引（`base` = zip 下载基础 URL，`plugins[]` = 插件元数据）
  - `market/*.zip`：插件安装包
  - 官方仓库：`github.com/Eggplant4363/HomeDesktopPlugins`；默认经 **jsDelivr CDN** 访问（国内网络直连 raw.githubusercontent 不可达）；可用配置项 `plugin.marketUrl` 覆盖索引地址（zip 下载地址自动跟随索引目录）
- 校验规则：manifest 缺少 `id`/`name` 或 JSON 非法 → 安装失败并提示；扫描到非法插件目录只跳过并告警，不影响应用。

## 7. 完整示例

```json
{
  "id": "com.example.todo",
  "name": "待办清单",
  "version": "1.0.0",
  "type": "icon",
  "emoji": "✅",
  "sizes": [{ "w": 1, "h": 1 }],
  "settings": [
    { "key": "listPath", "label": "清单文件", "type": "text", "default": "C:\\todo.txt" }
  ],
  "actions": [
    { "kind": "app", "path": "C:\\Windows\\System32\\notepad.exe" }
  ]
}
```

一个最简 zip 插件只需一个 `manifest.json`；widget 类型需要框架注册表中存在对应 `widgetComponent`（见 §5.1）。

## 8. 版本历史

| 版本 | 变更 |
|---|---|
| v1 | 基础：id/name/version/type/emoji/actions/widgetComponent |
| v2 | 新增 `sizes`（声明支持的尺寸档）与 `settings`（统一设置菜单声明） |
