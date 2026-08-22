<script lang="ts">
  // HomeAssistant 子插件专用设置菜单（编辑模式 ⚙）：
  // - 自动从 HA 拉取该域全部实体（如 switch.*），勾选即添加（只显示友好名，ID 悬浮可见）
  // - 每个实体可选图标：MDI 预设（按域）+ 自定义 mdi 图标名
  // 存储：entities（逗号分隔 cell 设置）+ icons（cell.<cellId>.icons = { entityId: "mdi-xxx" }）
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { PluginInfo } from "../core/types";
  import {
    getCellSetting,
    setCellSetting,
    getPluginSetting,
  } from "../core/pluginSettings.svelte";
  import { refreshWidget, widgetCache } from "../core/widgetRuntime.svelte";
  import { log } from "../core/logger";
  import MdiIcon from "./MdiIcon.svelte";

  let {
    plugin,
    cellId,
    onclose,
  }: {
    plugin: PluginInfo;
    /** 当前图标的实例 id */
    cellId?: string;
    onclose?: () => void;
  } = $props();

  interface HaState {
    entityId: string;
    state: string;
    friendlyName?: string | null;
    unit?: string | null;
    domain: string;
  }

  const providerId = $derived(plugin.providerId ?? plugin.id);
  const domain = $derived(plugin.domain ?? "");

  let url = $state("");
  let token = $state("");
  let all = $state<HaState[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let connected = $state(false);
  /** 勾选的实体 id */
  let selected = $state<Record<string, boolean>>({});
  /** 自定义图标表 { entityId: "mdi-xxx" } */
  let icons = $state<Record<string, string>>({});
  /** 自定义名称表 { entityId: "名称" } */
  let names = $state<Record<string, string>>({});
  /** 正在编辑名称的实体 */
  let editingName = $state<string | null>(null);
  let nameDraft = $state("");
  let query = $state("");
  /** 正在选图标的实体 */
  let iconFor = $state<string | null>(null);
  let iconRect = $state({ top: 0, left: 0 });
  let iconDraft = $state("");
  let iconMode = $state<"grid" | "custom">("grid");

  const q = $derived(query.trim().toLowerCase());
  const list = $derived(
    all.filter(
      (s) =>
        !q ||
        (s.friendlyName ?? "").toLowerCase().includes(q) ||
        s.entityId.toLowerCase().includes(q),
    ),
  );
  const selectedCount = $derived(Object.values(selected).filter(Boolean).length);

  /** 默认图标按域（MDI，与 HaWidget 一致） */
  const DEFAULT_ICONS: Record<string, string> = {
    light: "mdi-lightbulb-on",
    switch: "mdi-power",
    fan: "mdi-fan",
    input_boolean: "mdi-toggle-switch",
    button: "mdi-radiobox-blank",
    binary_sensor: "mdi-toggle-switch-outline",
    sensor: "mdi-chart-box-outline",
    temperature: "mdi-thermometer",
    humidity: "mdi-water-percent",
    motion: "mdi-motion-sensor",
    media_player: "mdi-television",
    climate: "mdi-air-conditioner",
    cover: "mdi-blinds",
    lock: "mdi-lock",
    camera: "mdi-cctv",
  };

  /** 每域可选的 MDI 预设图标 */
  const ICON_SETS: Record<string, string[]> = {
    light: ["mdi-lightbulb", "mdi-lightbulb-on", "mdi-lightbulb-outline", "mdi-lamp", "mdi-floor-lamp", "mdi-ceiling-light", "mdi-desk-lamp", "mdi-string-lights", "mdi-spotlight", "mdi-wall-sconce", "mdi-candle", "mdi-moon-waning-crescent"],
    switch: ["mdi-power", "mdi-toggle-switch", "mdi-toggle-switch-off", "mdi-power-plug", "mdi-power-socket", "mdi-electric-switch", "mdi-laptop", "mdi-desktop-tower", "mdi-monitor", "mdi-printer", "mdi-television", "mdi-router", "mdi-fan", "mdi-air-conditioner"],
    sensor: ["mdi-chart-box-outline", "mdi-thermometer", "mdi-water-percent", "mdi-weather-windy", "mdi-weather-partly-cloudy", "mdi-gauge", "mdi-battery", "mdi-wave", "mdi-flame", "mdi-leaf", "mdi-weather-rainy", "mdi-lightning-bolt"],
    fan: ["mdi-fan", "mdi-weather-windy", "mdi-snowflake", "mdi-air-conditioner", "mdi-wind-turbine"],
    input_boolean: ["mdi-toggle-switch", "mdi-toggle-switch-off", "mdi-power", "mdi-lightbulb", "mdi-bell", "mdi-door"],
  };
  const ICONS = $derived(ICON_SETS[domain] ?? ["mdi-help-circle", "mdi-chart-box-outline", "mdi-toggle-switch", "mdi-lightbulb", "mdi-power", "mdi-star", "mdi-heart", "mdi-cog", "mdi-bell", "mdi-rocket"]);

  /** 域主题色（HA 风格） */
  const DOMAIN_COLORS: Record<string, string> = {
    light: "#ffb300",
    switch: "#29b6f6",
    fan: "#26c6da",
    input_boolean: "#26a69a",
    button: "#26a69a",
    binary_sensor: "#66bb6a",
    sensor: "#66bb6a",
    temperature: "#ef5350",
    humidity: "#42a5f5",
    motion: "#ab47bc",
    media_player: "#5c6bc0",
    climate: "#ff7043",
    cover: "#8d6e63",
    lock: "#78909c",
    camera: "#ec407a",
  };
  function tileColor(s: HaState): string {
    return DOMAIN_COLORS[s.domain] ?? "var(--accent)";
  }

  /** 归一化图标名："laptop" / "mdi:laptop" / "mdi-laptop" → "mdi-laptop" */
  function normalizeIcon(raw: string): string {
    let v = raw.trim();
    if (!v) return "";
    if (v.startsWith("mdi:")) v = v.slice(4);
    return v.startsWith("mdi-") ? v : `mdi-${v}`;
  }

  function iconOf(id: string): string {
    return icons[id] || DEFAULT_ICONS[domain] || "mdi-help-circle";
  }

  function displayName(s: HaState): string {
    return names[s.entityId] || s.friendlyName || s.entityId;
  }

  onMount(async () => {
    url = (await getPluginSetting<string>(providerId, "url", "")) ?? "";
    token = (await getPluginSetting<string>(providerId, "token", "")) ?? "";
    // 已选实体
    const entRaw =
      (await getCellSetting<string>(cellId ?? plugin.id, plugin.id, "entities", "")) ?? "";
    for (const e of entRaw.split(",").map((s) => s.trim()).filter(Boolean)) selected[e] = true;
    // 已存图标
    const icoRaw =
      (await getCellSetting<string>(cellId ?? plugin.id, plugin.id, "icons", "")) ?? "";
    if (icoRaw) {
      try {
        icons = JSON.parse(icoRaw);
      } catch {
        icons = {};
      }
    }
    const nmRaw =
      (await getCellSetting<string>(cellId ?? plugin.id, plugin.id, "names", "")) ?? "";
    if (nmRaw) {
      try {
        names = JSON.parse(nmRaw);
      } catch {
        names = {};
      }
    }
    await load();
  });

  async function load(): Promise<void> {
    if (!url || !token) {
      error = "请先在 ＋ → 🏠 HomeAssistant → 配置 中填写地址和长期令牌";
      loading = false;
      return;
    }
    loading = true;
    error = null;
    try {
      all = await invoke<HaState[]>("ha_entities", { url, token, domain: domain || null });
      connected = true;
      log.info(`HA 实体拉取: 域=${domain || "全部"} 共 ${all.length} 个`);
    } catch (e) {
      error = String(e);
      connected = false;
    } finally {
      loading = false;
    }
  }

  /** 勾选/取消 → 立即保存并刷新 widget */
  async function toggleEntity(id: string): Promise<void> {
    if (selected[id]) {
      delete selected[id];
    } else {
      selected[id] = true;
    }
    await saveEntities();
  }

  async function saveEntities(): Promise<void> {
    const ids = Object.keys(selected).filter((k) => selected[k]);
    const v = ids.join(",");
    await setCellSetting(cellId ?? plugin.id, plugin.id, "entities", v);
    log.info(`HA 实体已保存: ${cellId ?? plugin.id} 共 ${ids.length} 个`);
    const key = cellId ?? plugin.id;
    delete widgetCache[key];
    void refreshWidget(key);
  }

  function setAll(on: boolean): void {
    for (const s of list) {
      if (on) selected[s.entityId] = true;
      else delete selected[s.entityId];
    }
    void saveEntities();
  }

  /** 打开图标选择器 */
  function openIconPicker(e: MouseEvent, id: string): void {
    e.stopPropagation();
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    iconRect = { top: r.bottom + 4, left: r.left - 60 };
    iconFor = iconFor === id ? null : id;
    iconDraft = (icons[id] ?? "").replace(/^mdi-/, "");
    iconMode = "grid";
  }

  /** 保存自定义名称（空 = 恢复 HA 原名） */
  async function saveName(id: string, raw: string): Promise<void> {
    const v = raw.trim();
    if (v) names[id] = v;
    else delete names[id];
    await setCellSetting(cellId ?? plugin.id, plugin.id, "names", JSON.stringify(names));
    editingName = null;
    const key = cellId ?? plugin.id;
    delete widgetCache[key];
    void refreshWidget(key);
  }

  async function setIcon(id: string | null, raw: string): Promise<void> {
    if (!id) return;
    const norm = normalizeIcon(raw);
    if (norm) icons[id] = norm;
    else delete icons[id];
    await setCellSetting(cellId ?? plugin.id, plugin.id, "icons", JSON.stringify(icons));
    iconFor = null;
    const key = cellId ?? plugin.id;
    delete widgetCache[key];
    void refreshWidget(key);
  }
</script>

<div
  class="overlay"
  role="button"
  aria-label="关闭"
  tabindex="-1"
  onclick={(e) => e.target === e.currentTarget && onclose?.()}
  onkeydown={(e) => e.key === "Escape" && onclose?.()}
>
  <div class="menu">
    <div class="head">
      <span>⚙ {plugin.name} 设置</span>
      <button class="close" onclick={onclose}>×</button>
    </div>

    <div class="body">
      {#if !url || !token}
        <div class="notice">
          ⚠️ 未配置 HomeAssistant 连接<br />
          <span class="hint">请先点 ＋ → 🏠 HomeAssistant → 配置，填写地址和长期令牌</span>
        </div>
      {:else if loading}
        <div class="notice">⏳ 正在从 HomeAssistant 获取实体…</div>
      {:else if error}
        <div class="notice err">
          ⚠️ {error}
          <button class="retry" onclick={() => void load()}>重试</button>
        </div>
      {:else}
        <div class="bar">
          <span class="ok">✓ 已连接 · {all.length} 个实体{domain ? `（${domain}）` : ""}</span>
          <button class="mini" onclick={() => void load()}>刷新</button>
        </div>

        <div class="search">
          <input type="text" placeholder="搜索实体…" bind:value={query} />
        </div>

        <div class="tip">✓ 勾选 = 添加到桌面 widget；点左侧图标换图标；点名称或 ✎ 改显示名称</div>

        <div class="list">
          {#each list as s (s.entityId)}
            <div
              class="row"
              role="button"
              tabindex="-1"
              title={s.entityId}
              onclick={() => void toggleEntity(s.entityId)}
              onkeydown={(e) => {
                if (e.key === "Enter") void toggleEntity(s.entityId);
              }}
            >
              <button
                class="check"
                class:on={!!selected[s.entityId]}
                aria-label="选择"
                onclick={(e) => {
                  e.stopPropagation();
                  void toggleEntity(s.entityId);
                }}
              >{selected[s.entityId] ? "✓" : ""}</button>
              <button
                class="ico"
                class:on={s.state === "on"}
                style="--dc:{tileColor(s)}"
                title="点击更换图标"
                onclick={(e) => openIconPicker(e, s.entityId)}
              >
                <MdiIcon name={iconOf(s.entityId)} size={22} />
                <span class="ico-edit">✎</span>
              </button>
              {#if editingName === s.entityId}
                <span class="nm-edit">
                  <input
                    type="text"
                    placeholder="显示名称（空=原名）"
                    bind:value={nameDraft}
                    onclick={(e) => e.stopPropagation()}
                    onkeydown={(e) => {
                      if (e.key === "Enter") void saveName(s.entityId, nameDraft);
                      if (e.key === "Escape") editingName = null;
                    }}
                  />
                  <button onclick={(e) => { e.stopPropagation(); void saveName(s.entityId, nameDraft); }}>✓</button>
                </span>
              {:else}
                <button
                  class="nm"
                  title="点击修改显示名称"
                  onclick={(e) => {
                    e.stopPropagation();
                    editingName = s.entityId;
                    nameDraft = names[s.entityId] ?? s.friendlyName ?? "";
                  }}
                >{displayName(s)}</button>
                <button
                  class="nm-edit-btn"
                  title="修改显示名称"
                  onclick={(e) => {
                    e.stopPropagation();
                    editingName = s.entityId;
                    nameDraft = names[s.entityId] ?? s.friendlyName ?? "";
                  }}
                >✎</button>
              {/if}
            </div>
          {/each}
          {#if list.length === 0}
            <div class="empty">没有匹配的实体</div>
          {/if}
        </div>

        <div class="foot">
          <span>已选 {selectedCount} 个</span>
          <div class="foot-btns">
            <button class="mini" onclick={() => setAll(true)}>全选</button>
            <button class="mini" onclick={() => setAll(false)}>清空</button>
          </div>
        </div>
      {/if}
    </div>
  </div>

  {#if iconFor}
    <button class="backdrop" aria-label="收起图标选择" onclick={() => (iconFor = null)}></button>
    <div
      class="icon-pop"
      style="top:{iconRect.top}px;left:{iconRect.left}px"
    >
      <div class="ip-head">
        <span>选择图标</span>
        <div class="ip-tabs">
          <button class:on={iconMode === "grid"} onclick={() => (iconMode = "grid")}>预设</button>
          <button class:on={iconMode === "custom"} onclick={() => (iconMode = "custom")}>自定义</button>
        </div>
      </div>
      {#if iconMode === "grid"}
        <div class="ip-grid">
          {#each ICONS as ico (ico)}
            <button
              class="ip-cell"
              class:on={icons[iconFor] === ico}
              onclick={() => void setIcon(iconFor, ico)}
            ><MdiIcon name={ico} size={22} /></button>
          {/each}
        </div>
        <button class="ip-reset" onclick={() => void setIcon(iconFor, "")}>恢复默认（按类型自动）</button>
      {:else}
        <div class="ip-custom">
          <input
            type="text"
            placeholder="mdi 图标名，如 laptop"
            bind:value={iconDraft}
            onkeydown={(e) => {
              if (e.key === "Enter") void setIcon(iconFor, iconDraft);
              if (e.key === "Escape") iconFor = null;
            }}
          />
          <button onclick={() => void setIcon(iconFor, iconDraft)}>应用</button>
        </div>
        <div class="ip-preview">
          预览：<MdiIcon name={normalizeIcon(iconDraft) || "mdi-help-circle"} size={20} />
          <span class="ip-code">{normalizeIcon(iconDraft) || "（默认）"}</span>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 45;
  }
  .menu {
    width: 360px;
    max-height: 78vh;
    background: var(--bg-elev);
    border-radius: 16px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 16px;
    font-weight: 600;
    font-size: 14px;
  }
  .close {
    border: none;
    background: transparent;
    color: var(--fg-dim);
    font-size: 18px;
    cursor: pointer;
  }
  .body {
    padding: 0 12px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
    overflow: hidden;
  }
  .notice {
    padding: 18px 10px;
    text-align: center;
    font-size: 12px;
    color: var(--fg-dim);
  }
  .notice.err {
    color: var(--fg);
  }
  .hint {
    font-size: 11px;
    opacity: 0.8;
  }
  .retry {
    display: block;
    margin: 10px auto 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 12px;
    padding: 5px 14px;
    cursor: pointer;
  }
  .bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .ok {
    font-size: 12px;
    color: #2e7d32;
  }
  .tip {
    font-size: 11px;
    color: var(--fg-dim);
  }
  .mini {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 11px;
    padding: 3px 10px;
    cursor: pointer;
  }
  .search input {
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 12px;
    padding: 7px 10px;
    outline: none;
    box-sizing: border-box;
  }
  .search input:focus {
    border-color: var(--accent);
  }
  .list {
    flex: 1;
    min-height: 120px;
    max-height: 44vh;
    overflow-y: auto;
    scrollbar-width: none;
    -ms-overflow-style: none;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .list::-webkit-scrollbar {
    display: none;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 8px;
    border-radius: 10px;
    background: var(--bg-hover);
    cursor: pointer;
  }
  .row:hover {
    background: color-mix(in srgb, var(--bg-hover) 60%, var(--accent) 12%);
  }
  .check {
    width: 20px;
    height: 20px;
    border-radius: 6px;
    border: 1.5px solid var(--border);
    background: transparent;
    color: #fff;
    font-size: 12px;
    line-height: 1;
    cursor: pointer;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .check.on {
    background: var(--accent);
    border-color: var(--accent);
  }
  .ico {
    position: relative;
    border: 1px solid var(--border);
    background: var(--bg-input);
    border-radius: 12px;
    width: 40px;
    height: 40px;
    cursor: pointer;
    flex-shrink: 0;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--fg-dim);
    transition: all 0.15s;
  }
  .ico:hover {
    border-color: var(--dc);
    transform: scale(1.05);
  }
  .ico.on {
    background: var(--dc);
    border-color: var(--dc);
    color: #fff;
  }
  .ico-edit {
    position: absolute;
    right: -4px;
    bottom: -4px;
    font-size: 10px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 50%;
    width: 14px;
    height: 14px;
    line-height: 13px;
    text-align: center;
  }
  .nm {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    color: var(--fg);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    border: none;
    background: transparent;
    padding: 0;
    text-align: left;
    cursor: text;
  }
  .nm:hover {
    color: var(--accent);
  }
  .empty {
    padding: 16px;
    text-align: center;
    color: var(--fg-dim);
    font-size: 12px;
  }
  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 12px;
    color: var(--fg-dim);
  }
  .foot-btns {
    display: flex;
    gap: 6px;
  }
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    border: none;
    background: transparent;
    padding: 0;
  }
  .icon-pop {
    position: fixed;
    z-index: 51;
    width: 260px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 10px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  }
  .ip-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
    font-size: 12px;
    font-weight: 600;
  }
  .ip-tabs {
    display: flex;
    gap: 4px;
  }
  .ip-tabs button {
    border: 1px solid var(--border);
    border-radius: 6px;
    background: transparent;
    color: var(--fg-dim);
    font-size: 11px;
    padding: 2px 8px;
    cursor: pointer;
  }
  .ip-tabs button.on {
    border-color: var(--accent);
    color: var(--accent);
  }
  .ip-grid {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 5px;
  }
  .ip-cell {
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--bg-input);
    font-size: 20px;
    padding: 8px 0;
    cursor: pointer;
    line-height: 1;
    color: var(--fg);
  }
  .ip-cell:hover {
    border-color: var(--accent);
  }
  .ip-cell.on {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    color: var(--accent);
  }
  .ip-reset {
    margin-top: 8px;
    width: 100%;
    border: none;
    background: transparent;
    color: var(--fg-dim);
    font-size: 11px;
    cursor: pointer;
    padding: 4px;
  }
  .ip-reset:hover {
    color: var(--fg);
  }
  .ip-custom {
    display: flex;
    gap: 6px;
  }
  .ip-custom input {
    flex: 1;
    min-width: 0;
    border: 1px solid var(--accent);
    border-radius: 8px;
    background: var(--bg-input);
    color: var(--fg);
    font-size: 12px;
    padding: 6px 8px;
    outline: none;
  }
  .ip-custom button {
    border: none;
    border-radius: 8px;
    background: var(--accent);
    color: #fff;
    font-size: 12px;
    padding: 0 14px;
    cursor: pointer;
    flex-shrink: 0;
  }
  .ip-preview {
    margin-top: 8px;
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 18px;
    color: var(--fg);
  }
  .ip-code {
    font-size: 10px;
    color: var(--fg-dim);
    font-family: monospace;
  }
</style>
