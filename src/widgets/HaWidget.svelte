<script lang="ts">
  // HomeAssistant 小组件 v4（极简）：只显示第一个实例的 MDI 图标 + 名称
  // 实体/图标配置只在设置菜单（编辑模式 ⚙）里改；widget 无任何交互控件
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCellSetting, peekCellSetting } from "../core/pluginSettings.svelte";
  import { registerWidget, unregisterWidget, widgetCache, getWidgetData } from "../core/widgetRuntime.svelte";
  import { log } from "../core/logger";
  import { layout, plugins } from "../core/stores.svelte";
  import { appearance } from "../core/appearance.svelte";
  import { iconGlyphSize } from "../core/iconStandard";
  import { registerSearchNames, markProviderFetched } from "../core/searchNames.svelte";
  import { focusCell } from "../core/stores.svelte";

  import { getPluginSetting, peekPluginSetting } from "../core/pluginSettings.svelte";
  import MdiIcon from "../components/MdiIcon.svelte";

  let { cellId }: { cellId?: string } = $props();

  /** 当前实例所属插件（子插件，如 …light/…switch/…sensor）。
   *  layout.pages 是 Cell[][]（每页一个数组），需先按页再按 cell 遍历（含文件夹内）。 */
  const myPlugin = $derived.by(() => {
    if (!cellId) return undefined;
    for (const page of layout.pages) {
      for (const c of page) {
        if (c.kind === "folder") {
          const sub = c.items.find((i) => i.id === cellId);
          if (sub) return sub.pluginId ? plugins.find((p) => p.id === sub.pluginId) : undefined;
        } else if (c.id === cellId) {
          return c.pluginId ? plugins.find((p) => p.id === c.pluginId) : undefined;
        }
      }
    }
    return undefined;
  });

  /** 提供商 id（共享配置键）：子插件 → providerId；旧单插件格式 → 插件自身 id */
  const providerId = $derived((myPlugin?.providerId as string | undefined) ?? myPlugin?.id ?? "");
  /** 实体域过滤（子插件专用；空 = 全部） */
  const domain = $derived((myPlugin?.domain as string | undefined) ?? "");

  interface HaState {
    entityId: string;
    state: string;
    friendlyName?: string | null;
    unit?: string | null;
    domain: string;
  }

  let states = $state<HaState[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  /** 切换进行中 */
  let working = $state(false);
  /** 乐观状态覆盖：{ entityId: "on"|"off" }，点击瞬间生效，同步真实状态后清除 */
  let overrides = $state<Record<string, string>>({});
  /** 自定义图标表 { entityId: "mdi-xxx" }（cell.<cellId>.icons，响应式：设置菜单保存后立即生效） */
  const iconsRaw = $derived(
    peekCellSetting<string>(cellId ?? providerId, providerId, "icons") ?? "",
  );
  const icons = $derived.by<Record<string, string>>(() => {
    if (!iconsRaw) return {};
    try {
      return JSON.parse(iconsRaw);
    } catch {
      return {};
    }
  });
  /** 自定义名称表 { entityId: "名称" }（cell.<cellId>.names，响应式） */
  const namesRaw = $derived(
    peekCellSetting<string>(cellId ?? providerId, providerId, "names") ?? "",
  );
  const names = $derived.by<Record<string, string>>(() => {
    if (!namesRaw) return {};
    try {
      return JSON.parse(namesRaw);
    } catch {
      return {};
    }
  });

  /** url/token：提供商级共享设置（config plugin.<providerId>.url/token）；旧格式回退实例级 */
  const url = $derived(peekPluginSetting<string>(providerId, "url") ?? "");
  const token = $derived(peekPluginSetting<string>(providerId, "token") ?? "");
  const entitiesRaw = $derived(
    peekCellSetting<string>(cellId ?? providerId, providerId, "entities") ?? "",
  );
  const entities = $derived(
    entitiesRaw.split(",").map((s) => s.trim()).filter(Boolean),
  );

  const configured = $derived(!!url && !!token && entities.length > 0);

  /** 默认图标按实体域（MDI，参考 HA） */
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

  /** 图标方块尺寸跟随网格（与普通图标一致） */
  // 圆形图标 = 统一标准图标尺寸（与其他插件一致，保证文字同水平线）
  const haTile = $derived(iconGlyphSize(appearance.tileSize));
  const haIcon = $derived(iconGlyphSize(appearance.tileSize) - 10);

  /** 实体图标：自定义优先；否则按"状态 + 域"选（HA 风格：开=实心彩色、关=空心灰） */
  function iconOf(s: HaState): string {
    if (icons[s.entityId]) return icons[s.entityId];
    const on = s.state === "on";
    const variants: Record<string, string> = {
      light: on ? "mdi-lightbulb-on" : "mdi-lightbulb-outline",
      fan: on ? "mdi-fan" : "mdi-fan-off",
      input_boolean: on ? "mdi-toggle-switch" : "mdi-toggle-switch-off",
      switch: on ? "mdi-power" : "mdi-power-off",
      binary_sensor: on ? "mdi-toggle-switch" : "mdi-toggle-switch-outline",
    };
    return variants[s.domain] || DEFAULT_ICONS[s.domain] || "mdi-help-circle";
  }

  /** 域主题色（HA 风格：开=域颜色、关=灰） */
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
    // 开的状态统一琥珀色（FFC107）
    return "#FFC107";
  }

  /** 展示第一个实例（设置菜单里勾选的第一个） */
  const shown = $derived(states[0]);
  /** 开/关派生状态（模板直接读取，保证响应式更新） */
  const shownOn = $derived(
    (overrides[shown?.entityId ?? ""] ?? shown?.state) === "on",
  );

  /** 拉取并应用状态（含域过滤）；失败时设置 error 并抛出（runtime 视作刷新失败） */
  async function fetchStates(): Promise<HaState[]> {
    try {
      const data = await invoke<HaState[]>("ha_states", {
        url,
        token,
        entities,
      });
      const filtered = domain ? data.filter((s) => s.domain === domain) : data;
      states = filtered;
      error = null;
      if (cellId) widgetCache[cellId] = { data: filtered, fetchedAt: Date.now() };
      return filtered;
    } catch (e) {
      error = String(e);
      log.error(`HA 状态获取失败: ${e}`);
      throw e;
    } finally {
      loading = false;
    }
  }

  async function refresh(): Promise<void> {
    if (!configured) {
      loading = false;
      return;
    }
    try {
      await fetchStates();
    } catch {
      // error 已在 fetchStates 中设置
    }
  }

  onMount(() => {
    // 注册到小组件运行时：设置保存后 refreshWidget(cellId) 可立即刷新
    if (cellId) {
      registerWidget<HaState[]>({
        id: cellId,
        refreshMs: 2000,
        fetch: () => (configured ? fetchStates() : Promise.resolve(states)),
      });
    }
    // 挂载时从运行时缓存恢复状态（搜索过滤重挂时立即显示，不闪"连接中"）
    const cached = cellId ? getWidgetData<HaState[]>(cellId) : undefined;
    if (cached) {
      states = cached;
      loading = false;
    }
    const init = async () => {
      await getPluginSetting(providerId, "url", "");
      await registerHaAllEntities(providerId, url, token);
      await getPluginSetting(providerId, "token", "");
      await getCellSetting(cellId ?? providerId, providerId, "entities", "");
      // 预热 icons/names 缓存（触发响应式更新）
      await getCellSetting<string>(cellId ?? providerId, providerId, "icons", "");
      await getCellSetting<string>(cellId ?? providerId, providerId, "names", "");
      void refresh();
    };
    void init();
    const timer = setInterval(() => void refresh(), 2000);
    return () => {
      clearInterval(timer);
      if (cellId) unregisterWidget(cellId);
    };
  });

  /** 抓取提供商下全部实体并注册为搜索名（"客厅灯"等任意实体可搜到），按提供商去重 */
  async function registerHaAllEntities(pid: string, u: string, tk: string): Promise<void> {
    if (!cellId || !u || !tk) return;
    if (!markProviderFetched(pid)) return;
    try {
      const all = await invoke<HaState[]>("ha_entities", { url: u, token: tk, domain: null });
      const entries: { label: string; sublabel: string; emoji: string; pluginId: string; action: () => void }[] = all.map((s) => ({
        label: s.friendlyName || s.entityId,
        sublabel: s.entityId,
        emoji: "🏠",
        pluginId: pid,
        action: () => focusCell(cellId!),
      }));
      registerSearchNames(pid, entries);
      log.info(`HA 搜索名注册: ${pid} 共 ${entries.length} 个实体`);
    } catch (e) {
      log.error(`HA 搜索名注册失败: ${e}`);
    }
  }

  // 按 widget 注册配置实体（显示名=自定义||友好名，点击定位到本小组件）
  $effect(() => {
    if (!cellId) return;
    const entries = states.map((s) => ({
      label: nameOf(s),
      sublabel: s.entityId,
      emoji: "🏠",
      pluginId: providerId,
      action: () => focusCell(cellId!),
    }));
    // 粘性注册：不随卸载清除；states 为空时不覆盖（搜索过滤重挂载时会先空后填，防止清掉条目）
    if (states.length === 0) return;
    registerSearchNames(`cell.${cellId}`, entries);
  });

  function isOn(s: HaState): boolean {
    return (overrides[s.entityId] ?? s.state) === "on";
  }
  function nameOf(s: HaState): string {
    return names[s.entityId] || s.friendlyName || s.entityId;
  }
  /** 可切换的实体域 */
  function isToggleable(s: HaState): boolean {
    return ["light", "switch", "fan", "input_boolean"].includes(s.domain);
  }

  /** 点击图标切换开关（light/switch/fan/input_boolean 均支持 toggle）。
   *  乐观覆盖：点击瞬间强制目标状态（颜色立刻变），1.5s 后同步真实状态并清除覆盖 */
  async function toggle(s: HaState): Promise<void> {
    if (working || !s) return;
    working = true;
    const current = (overrides[s.entityId] ?? s.state) === "on";
    const target = current ? "off" : "on";
    overrides[s.entityId] = target; // $state 更新 → 立即重渲染变色
    try {
      await invoke("ha_call", {
        url,
        token,
        domain: s.domain,
        service: "toggle",
        entityId: s.entityId,
      });
      log.info(`HA 切换: ${s.entityId} -> ${target}`);
      setTimeout(() => {
        delete overrides[s.entityId];
        void refresh();
      }, 1500);
    } catch (e) {
      delete overrides[s.entityId];
      log.error(`HA 切换失败: ${s.entityId} -> ${e}`);
      error = String(e);
    } finally {
      working = false;
    }
  }
</script>

<div class="ha">
  {#if !configured}
    <div class="empty">
      <div class="e-icon">🏠</div>
      <div>未配置</div>
      <div class="e-hint">＋ → 🏠 配置；编辑模式 ⚙ 勾选实体</div>
    </div>
  {:else if loading}
    <div class="empty">
      <div class="e-icon">⏳</div>
      <div>连接中…</div>
    </div>
  {:else if error}
    <div class="empty">
      <div class="e-icon">⚠️</div>
      <div class="e-hint">{error}</div>
    </div>
  {:else if !shown}
    <div class="empty">
      <div class="e-icon">🔌</div>
      <div>未配置实体</div>
      <div class="e-hint">编辑模式 ⚙ 勾选实体</div>
    </div>
  {:else}
    <div class="main" title={shown.entityId}>
      <button
        class="icon-tile"
        style="width:{haTile}px;height:{haTile}px;--dc:{tileColor(shown)}"
        class:on={shownOn}
        class:togglable={isToggleable(shown)}
        disabled={working}
        title={isToggleable(shown) ? (shownOn ? "点击关闭" : "点击开启") : shown.entityId}
        onclick={() => isToggleable(shown) && void toggle(shown)}
      ><MdiIcon name={iconOf(shown)} size={haIcon} /></button>
      <div class="name">{nameOf(shown)}</div>
    </div>
  {/if}
</div>

<style>
  .ha {
    height: 100%;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .main {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 6px;
  }
  .icon-tile {
    border: 1px solid var(--border);
    background: var(--bg-input);
    padding: 0;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--fg-dim);
    transition: all 0.2s;
    cursor: default;
    flex-shrink: 0;
  }
  .icon-tile.togglable {
    cursor: pointer;
  }
  .icon-tile.togglable:hover {
    transform: scale(1.06);
  }
  .icon-tile.togglable:active {
    transform: scale(0.95);
  }
  .icon-tile:disabled {
    opacity: 0.5;
  }
  .icon-tile.on {
    background: var(--dc);
    border-color: var(--dc);
    color: #fff;
  }
  .name {
    font-size: 13px;
    font-weight: 500;
    color: var(--fg);
    max-width: 60%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    font-size: 12px;
    color: var(--fg-dim);
    text-align: center;
    padding: 6px;
  }
  .e-icon {
    font-size: 24px;
  }
  .e-hint {
    font-size: 10px;
    opacity: 0.8;
    max-width: 90%;
    overflow-wrap: anywhere;
  }
</style>
