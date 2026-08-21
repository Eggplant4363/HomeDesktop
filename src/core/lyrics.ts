// 歌词：LRCLIB 免费接口（https://lrclib.net/docs，无需密钥）获取同步歌词 + 解析 LRC + 内存缓存

export interface LyricLine {
  /** 时间（秒） */
  time: number;
  text: string;
}

export interface LyricsResult {
  /** 带时间轴的逐行歌词 */
  synced: LyricLine[];
  /** 纯文本歌词（无时间轴） */
  plain: string | null;
  /** 纯音乐（无歌词） */
  instrumental: boolean;
}

export interface TrackRef {
  title: string;
  artist: string;
  album: string;
  duration: number;
}

const cache = new Map<string, LyricsResult>();

const LRCLIB_BASE = "https://lrclib.net/api";
/** 请求超时（网络不可达时不永久转圈） */
const FETCH_TIMEOUT = 8000;

async function fetchJson(url: string): Promise<unknown | null> {
  const ctrl = new AbortController();
  const timer = setTimeout(() => ctrl.abort(), FETCH_TIMEOUT);
  try {
    const res = await fetch(url, { signal: ctrl.signal });
    if (!res.ok) return null;
    return await res.json();
  } catch {
    return null;
  } finally {
    clearTimeout(timer);
  }
}

/** 获取歌词（按 标题/歌手/专辑/时长 精确匹配；失败或未找到返回 null） */
export async function fetchLyrics(track: TrackRef): Promise<LyricsResult | null> {
  const key = `${track.title}|${track.artist}|${track.album}|${Math.round(track.duration)}`;
  const hit = cache.get(key);
  if (hit) return hit;
  try {
    let result = await getExact(track);
    if (!result) result = await searchBest(track);
    if (result && (result.synced.length > 0 || result.plain)) {
      cache.set(key, result);
      return result;
    }
    return null;
  } catch {
    return null;
  }
}

/** 精确匹配：标题/歌手/专辑/时长 全对才返回 */
async function getExact(track: TrackRef): Promise<LyricsResult | null> {
  const url = new URL(LRCLIB_BASE + "/get");
  url.searchParams.set("track_name", track.title);
  url.searchParams.set("artist_name", track.artist);
  url.searchParams.set("album_name", track.album);
  if (track.duration > 0) url.searchParams.set("duration", String(Math.round(track.duration)));
  const raw = await fetchJson(url.toString());
  if (!raw) return null;
  const data = raw as Record<string, unknown>;
  return {
    synced: parseLrc((data.syncedLyrics as string) ?? ""),
    plain: (data.plainLyrics as string) ?? null,
    instrumental: !!data.instrumental,
  };
}

/** 搜索兜底：取与目标时长最接近且带时间轴歌词的条目 */
async function searchBest(track: TrackRef): Promise<LyricsResult | null> {
  const url = new URL(LRCLIB_BASE + "/search");
  if (track.title) url.searchParams.set("track_name", track.title);
  if (track.artist) url.searchParams.set("artist_name", track.artist);
  const list = (await fetchJson(url.toString())) as unknown;
  if (!Array.isArray(list) || list.length === 0) return null;
  const target = track.duration || 0;
  let best: Record<string, unknown> | null = null;
  let bestDiff = Infinity;
  for (const item of list) {
    const d = (item.duration as number) || 0;
    const diff = target > 0 ? Math.abs(d - target) : 0;
    if (item.syncedLyrics && diff < bestDiff) {
      best = item;
      bestDiff = diff;
    }
  }
  const item = best ?? list.find((i) => i.syncedLyrics) ?? list[0];
  if (!item) return null;
  return {
    synced: parseLrc((item.syncedLyrics as string) ?? ""),
    plain: (item.plainLyrics as string) ?? null,
    instrumental: !!item.instrumental,
  };
}

/** 解析 LRC：支持 [mm:ss]、[mm:ss.xx]、[mm:ss.xxx]、多个时间标签一行 */
export function parseLrc(lrc: string): LyricLine[] {
  const lines: LyricLine[] = [];
  const re = /\[(\d{1,2}):(\d{1,2})(?:[.:](\d{1,3}))?\]/g;
  for (const raw of lrc.split(/\r?\n/)) {
    const times: number[] = [];
    let m: RegExpExecArray | null;
    re.lastIndex = 0;
    while ((m = re.exec(raw))) {
      const mm = parseInt(m[1], 10);
      const ss = parseInt(m[2], 10);
      const frac = m[3] ? parseInt(m[3].padEnd(3, "0").slice(0, 3), 10) : 0;
      times.push(mm * 60 + ss + frac / 1000);
    }
    if (times.length === 0) continue;
    const text = raw.replace(re, "").trim();
    for (const t of times) lines.push({ time: t, text });
  }
  lines.sort((a, b) => a.time - b.time);
  return lines;
}

/** 当前播放进度对应的歌词行下标（-1 = 尚未开始） */
export function lineIndexAt(lines: LyricLine[], position: number): number {
  let idx = -1;
  for (let i = 0; i < lines.length; i++) {
    if (position >= lines[i].time) idx = i;
    else break;
  }
  return idx;
}
