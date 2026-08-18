<script lang="ts">
  // 天气小组件：数据由 widgetRuntime 统一缓存 + 定时刷新
  // 设置按"图标实例"独立（cell.<cellId>.city/unit），未设置时回退插件级默认
  import { onMount } from "svelte";
  import {
    getCellSetting,
    peekCellSetting,
  } from "../core/pluginSettings.svelte";
  import {
    isWidgetStale,
    refreshWidget,
    registerWidget,
    widgetCache,
  } from "../core/widgetRuntime.svelte";
  import { log } from "../core/logger";

  let { cellId }: { cellId?: string } = $props();

  interface GeoResult {
    latitude: number;
    longitude: number;
    name: string;
    country?: string;
  }

  interface WeatherData {
    temperatureC: number;
    weatherCode: number;
    humidity: number;
    windSpeed: number;
    cityLabel: string;
  }

  const PLUGIN_ID = "dev.homedesktop.weather";
  const REFRESH_MS = 30 * 60_000; // 30 分钟
  /** 实例键：每个图标实例独立缓存/设置 */
  const instanceId = $derived(cellId ?? PLUGIN_ID);

  const WMO: Record<number, { emoji: string; label: string }> = {
    0: { emoji: "☀️", label: "晴" },
    1: { emoji: "🌤️", label: "大部晴朗" },
    2: { emoji: "⛅", label: "多云" },
    3: { emoji: "☁️", label: "阴" },
    45: { emoji: "🌫️", label: "雾" },
    48: { emoji: "🌫️", label: "雾凇" },
    51: { emoji: "🌦️", label: "毛毛雨" },
    53: { emoji: "🌦️", label: "毛毛雨" },
    55: { emoji: "🌦️", label: "毛毛雨" },
    61: { emoji: "🌧️", label: "小雨" },
    63: { emoji: "🌧️", label: "中雨" },
    65: { emoji: "🌧️", label: "大雨" },
    71: { emoji: "🌨️", label: "小雪" },
    73: { emoji: "🌨️", label: "中雪" },
    75: { emoji: "❄️", label: "大雪" },
    80: { emoji: "🌦️", label: "阵雨" },
    81: { emoji: "🌧️", label: "强阵雨" },
    82: { emoji: "⛈️", label: "暴雨" },
    95: { emoji: "⛈️", label: "雷暴" },
    96: { emoji: "⛈️", label: "雷暴伴冰雹" },
    99: { emoji: "⛈️", label: "强雷暴" },
  };

  async function fetchWeatherData(): Promise<WeatherData> {
    const city = peekCellSetting<string>(instanceId, PLUGIN_ID, "city") ?? "北京";
    log.info(`天气拉取: 实例=${instanceId} 城市=${city}`);
    const geoUrl = `https://geocoding-api.open-meteo.com/v1/search?name=${encodeURIComponent(city)}&count=1&language=zh&format=json`;
    const geoRes = await fetch(geoUrl);
    if (!geoRes.ok) throw new Error("地理编码失败");
    const geoJson = await geoRes.json();
    const hit: GeoResult | undefined = geoJson.results?.[0];
    if (!hit) throw new Error("未找到城市");

    const wxUrl =
      `https://api.open-meteo.com/v1/forecast?latitude=${hit.latitude}&longitude=${hit.longitude}` +
      `&current=temperature_2m,weather_code,relative_humidity_2m,wind_speed_10m&timezone=auto`;
    const wxRes = await fetch(wxUrl);
    if (!wxRes.ok) throw new Error("天气接口失败");
    const wxJson = await wxRes.json();
    const cur = wxJson.current;
    if (!cur) throw new Error("无天气数据");

    return {
      temperatureC: cur.temperature_2m,
      weatherCode: cur.weather_code,
      humidity: cur.relative_humidity_2m,
      windSpeed: Math.round(cur.wind_speed_10m),
      cityLabel: hit.name + (hit.country ? ` · ${hit.country}` : ""),
    };
  }

  // 响应式读取缓存（后台刷新后自动更新；按实例隔离，不同城市互不影响）
  const data = $derived(widgetCache[instanceId]?.data as WeatherData | undefined);
  const wmo = $derived(
    data ? WMO[data.weatherCode] ?? { emoji: "🌡️", label: String(data.weatherCode) } : null,
  );
  // 单位设置（按实例，回退插件级默认）
  const unit = $derived(peekCellSetting<string>(instanceId, PLUGIN_ID, "unit") ?? "celsius");
  const displayTemp = $derived(
    data ? (unit === "fahrenheit" ? Math.round((data.temperatureC * 9) / 5 + 32) : Math.round(data.temperatureC)) : 0,
  );

  onMount(async () => {
    // 加载实例设置（填充缓存；未设置回退插件级默认，兼容旧共享配置）
    log.info(`天气实例挂载: ${instanceId}`);
    await getCellSetting(instanceId, PLUGIN_ID, "city", "北京");
    await getCellSetting(instanceId, PLUGIN_ID, "unit", "celsius");
    // 按实例注册数据源（幂等）：widgetRuntime 定时后台刷新，与页面切换无关
    registerWidget<WeatherData>({
      id: instanceId,
      refreshMs: REFRESH_MS,
      fetch: fetchWeatherData,
    });
    // 有缓存直接渲染；过期/缺失则后台刷新
    if (isWidgetStale(instanceId, REFRESH_MS)) {
      void refreshWidget(instanceId);
    }
  });
</script>

<div class="weather">
  {#if data && wmo}
    <div class="main">
      <span class="emoji">{wmo.emoji}</span>
      <span class="temp">{displayTemp}°</span>
      <div class="meta">
        <div class="city">{data.cityLabel}</div>
        <div class="extra">{wmo.label} · 💧{data.humidity}% · 🌬{data.windSpeed}km/h</div>
      </div>
    </div>
  {:else}
    <div class="hint">
      <span>加载中…</span>
      <button class="retry" onclick={() => void refreshWidget(instanceId)}>重试</button>
    </div>
  {/if}
</div>

<style>
  .weather {
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .main {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .emoji {
    font-size: 40px;
  }
  .temp {
    font-size: 44px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .city {
    font-size: 14px;
    font-weight: 600;
  }
  .extra {
    font-size: 11px;
    color: var(--fg-dim);
  }
  .hint {
    font-size: 13px;
    color: var(--fg-dim);
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .retry {
    border: 1px solid var(--border);
    border-radius: 6px;
    background: transparent;
    color: var(--fg);
    font-size: 11px;
    padding: 2px 8px;
    cursor: pointer;
  }
</style>
