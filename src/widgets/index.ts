// 小组件注册表：manifest.widgetComponent → Svelte 组件 + 默认网格尺寸
import ClockWidget from "./ClockWidget.svelte";
import WeatherWidget from "./WeatherWidget.svelte";
import CalendarWidget from "./CalendarWidget.svelte";
import SysMonitorWidget from "./SysMonitorWidget.svelte";
import TodoWidget from "./TodoWidget.svelte";
import MusicWidget from "./MusicWidget.svelte";
import type { Component } from "svelte";

export interface WidgetDef {
  component: Component;
  defaultSize: { w: number; h: number };
}

export const widgetRegistry: Record<string, WidgetDef> = {
  clock: { component: ClockWidget, defaultSize: { w: 2, h: 1 } },
  weather: { component: WeatherWidget, defaultSize: { w: 2, h: 1 } },
  calendar: { component: CalendarWidget, defaultSize: { w: 2, h: 2 } },
  sysmonitor: { component: SysMonitorWidget, defaultSize: { w: 2, h: 1 } },
  todo: { component: TodoWidget, defaultSize: { w: 2, h: 2 } },
  music: { component: MusicWidget, defaultSize: { w: 3, h: 1 } },
};

export function getWidgetDef(component: string | undefined): WidgetDef | undefined {
  return component ? widgetRegistry[component] : undefined;
}
