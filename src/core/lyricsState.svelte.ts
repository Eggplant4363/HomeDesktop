// 歌词面板全局状态：音乐小组件点 🎤 打开，App 根部渲染 LyricsPanel
export interface LyricsTrack {
  title: string;
  artist: string;
  album: string;
  duration: number;
}

export const lyrics = $state<{ open: boolean; track: LyricsTrack }>({
  open: false,
  track: { title: "", artist: "", album: "", duration: 0 },
});

export function openLyrics(track: LyricsTrack): void {
  lyrics.track = track;
  lyrics.open = true;
}

export function closeLyrics(): void {
  lyrics.open = false;
}
