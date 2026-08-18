// 插件自带小组件宿主（M16 沙箱版）：
// - 插件 JS 在 **sandbox iframe**（allow-scripts，无 same-origin）里运行，
//   无法访问主上下文 / 直接 invoke → 安全隔离
// - 宿主注入 window.__homedesktopPlugin 桥（iframe 内 shim 通过 postMessage 转发）
// - iframe 内点击（非交互元素）转发给宿主 → 打开设置
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { log } from "./logger";

/** 拼插件 JS 文件路径（按平台分隔符；纯函数便于单测） */
export function pluginWidgetPath(pluginDir: string, file: string): string {
  const sep = pluginDir.includes("\\") ? "\\" : "/";
  return pluginDir + sep + file;
}

/** 插件 JS 的 asset 协议 URL（官方 convertFileSrc：含 URL 编码，Tauri 2 正确格式） */
export function pluginWidgetUrl(pluginDir: string, file: string): string {
  return convertFileSrc(pluginWidgetPath(pluginDir, file));
}

/** iframe 内桥 shim：把 window.__homedesktopPlugin 转发到父窗口（postMessage） */
const BRIDGE_SHIM = `(function(){
  var pending = {}, seq = 0;
  function call(t, payload) {
    return new Promise(function(resolve){
      var id = ++seq; pending[id] = resolve;
      payload.t = t; payload.__hd = true; payload.id = id;
      window.parent.postMessage(payload, "*");
    });
  }
  window.__homedesktopPlugin = {
    getSetting: function(cellId, key, fallback) {
      return call("get", { cellId: cellId, key: key, fallback: fallback });
    },
    setSetting: function(cellId, key, value) {
      return call("set", { cellId: cellId, key: key, value: value });
    },
    notify: function(title, body) {
      return call("notify", { title: title, body: body });
    }
  };
  window.addEventListener("message", function(e){
    var d = e.data;
    if (d && d.__hd === true && pending[d.id]) { pending[d.id](d.result); delete pending[d.id]; }
  });
  // 点击（非交互元素）转发给宿主 → 打开设置
  document.addEventListener("click", function(e){
    var t = e.target;
    if (t && t.closest && t.closest("button,input,select,textarea,a,[data-hd-noopen]")) return;
    window.parent.postMessage({ __hd: true, t: "click" }, "*");
  }, true);
})();`;

/** 构造沙箱 iframe 的 srcdoc（插件 JS 内容内联，避免沙箱内跨源加载 asset 被阻；纯函数便于单测） */
export function buildPluginIframeSrcdoc(opts: {
  code: string;
  elementTag: string;
  cellId: string;
}): string {
  const tag = JSON.stringify(opts.elementTag);
  const cellId = JSON.stringify(opts.cellId);
  // 防止插件代码里出现 </script> 截断 srcdoc
  const code = opts.code.replace(/<\/script/gi, "<\\/script");
  return `<!doctype html><html><head><meta charset="utf-8"><style>
html,body{margin:0;padding:0;height:100%;}
body{display:flex;}
#root{flex:1;min-width:0;min-height:0;}
</style></head><body><div id="root"></div>
<script>${BRIDGE_SHIM}<\/script>
<script>${code}<\/script>
<script>
var root=document.getElementById("root");
var el=document.createElement(${tag});
el.setAttribute("cell-id",${cellId});
root.appendChild(el);
<\/script>
</body></html>`;
}

/** 处理来自插件 iframe 的桥消息（get/set 设置）；返回是否已处理 */
export function handleBridgeMessage(e: MessageEvent): boolean {
  const d = e.data;
  if (!d || d.__hd !== true || typeof d.t !== "string" || d.t === "click") return false;
  const reply = (result: unknown): void => {
    e.source?.postMessage({ __hd: true, id: d.id, result }, { targetOrigin: "*" });
  };
  if (d.t === "get") {
    invoke("config_get", { key: `cell.${d.cellId}.${d.key}` })
      .then((v) => reply(v ?? d.fallback))
      .catch((err) => {
        log.error(`插件读取设置失败: ${d.cellId}/${d.key} -> ${err}`);
        reply(d.fallback);
      });
    return true;
  }
  if (d.t === "set") {
    invoke("config_set", { key: `cell.${d.cellId}.${d.key}`, value: d.value })
      .then(() => reply(true))
      .catch((err) => {
        log.error(`插件写入设置失败: ${d.cellId}/${d.key} -> ${err}`);
        reply(null);
      });
    return true;
  }
  if (d.t === "notify") {
    const title = String(d.title ?? "");
    const body = String(d.body ?? "");
    log.info(`插件通知请求: ${title} | ${body}`);
    invoke("app_notify", { title, body })
      .then(() => reply(true))
      .catch((err) => {
        log.error(`插件通知失败: ${err}`);
        reply(null);
      });
    return true;
  }
  return false;
}
