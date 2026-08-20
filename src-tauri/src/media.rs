//! 音乐控制（仅 Windows）：通过系统媒体传输控制（SMTC）读取当前播放的媒体信息
//! 并控制上一曲 / 下一曲 / 播放 / 暂停。
//! 只要应用接入了 Windows 媒体会话（Spotify、系统「音乐」、Edge/Chrome 播放网页媒体、
//! 网易云/QQ音乐等多数播放器），本模块就能读到标题/歌手/专辑封面并控制播放。
//! 实现：专用工作线程（MTA COM 初始化）+ 命令 → 通道请求 → SMTC 调用。
#![cfg(windows)]

use base64::Engine;
use serde::Serialize;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::OnceLock;
use std::time::Duration;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};
use windows::Storage::Streams::DataReader;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub app: String,
    /// playing | paused | stopped | closed | changing
    pub state: String,
    /// 专辑封面 data URL（PNG/JPEG），无封面为 null
    pub thumbnail: Option<String>,
}

enum Request {
    Info(Sender<Result<MediaInfo, String>>),
    Control(String, Sender<Result<(), String>>),
}

static MEDIA_TX: OnceLock<Sender<Request>> = OnceLock::new();

fn worker_tx() -> &'static Sender<Request> {
    MEDIA_TX.get_or_init(|| {
        let (tx, rx) = channel::<Request>();
        std::thread::Builder::new()
            .name("media-smtc".into())
            .spawn(move || worker_loop(rx))
            .expect("启动媒体工作线程失败");
        tx
    })
}

fn worker_loop(rx: Receiver<Request>) {
    // WinRT 调用需要本线程初始化 COM（MTA）
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    while let Ok(req) = rx.recv() {
        match req {
            Request::Info(tx) => {
                let _ = tx.send(fetch_info());
            }
            Request::Control(action, tx) => {
                let _ = tx.send(control(&action));
            }
        }
    }
}

fn manager() -> Result<GlobalSystemMediaTransportControlsSessionManager, String> {
    GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .map_err(|e| format!("SMTC 不可用: {e}"))?
        .get()
        .map_err(|e| format!("SMTC 初始化失败: {e}"))
}

/// 当前播放信息；无媒体会话时返回空信息（title 为空、state=closed），前端显示空态
fn fetch_info() -> Result<MediaInfo, String> {
    let mgr = manager()?;
    let session = mgr
        .GetCurrentSession()
        .map_err(|e| format!("获取媒体会话失败: {e}"))?;
    let props = session
        .TryGetMediaPropertiesAsync()
        .map_err(|e| format!("获取媒体属性失败: {e}"))?
        .get()
        .map_err(|e| format!("媒体属性读取失败: {e}"))?;

    let mut info = MediaInfo {
        title: props.Title().map(|s| s.to_string()).unwrap_or_default(),
        artist: props.Artist().map(|s| s.to_string()).unwrap_or_default(),
        album: props.AlbumTitle().map(|s| s.to_string()).unwrap_or_default(),
        app: session
            .SourceAppUserModelId()
            .map(|s| s.to_string())
            .unwrap_or_default(),
        state: "closed".into(),
        thumbnail: None,
    };
    info.state = match session
        .GetPlaybackInfo()
        .ok()
        .and_then(|p| p.PlaybackStatus().ok())
        .unwrap_or(GlobalSystemMediaTransportControlsSessionPlaybackStatus::Closed)
    {
        GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing => "playing",
        GlobalSystemMediaTransportControlsSessionPlaybackStatus::Paused => "paused",
        GlobalSystemMediaTransportControlsSessionPlaybackStatus::Stopped => "stopped",
        GlobalSystemMediaTransportControlsSessionPlaybackStatus::Closed => "closed",
        _ => "changing",
    }
    .into();

    // 专辑封面（头像）：SMTC 缩略图 → 读取为 data URL
    if let Ok(thumb) = props.Thumbnail() {
        info.thumbnail = read_thumbnail(thumb).ok();
    }
    Ok(info)
}

/// 封面大小上限（2MB，避免超大 payload）
const MAX_THUMB: usize = 2 * 1024 * 1024;

fn read_thumbnail(
    thumb: windows::Storage::Streams::IRandomAccessStreamReference,
) -> Result<String, String> {
    let stream = thumb
        .OpenReadAsync()
        .map_err(|e| format!("打开封面流失败: {e}"))?
        .get()
        .map_err(|e| format!("封面流等待失败: {e}"))?;
    let size = stream.Size().map_err(|e| format!("读取封面大小失败: {e}"))? as usize;
    if size == 0 || size > MAX_THUMB {
        return Err(format!("封面大小异常: {size}"));
    }
    let reader = DataReader::CreateDataReader(&stream).map_err(|e| format!("创建读取器失败: {e}"))?;
    reader
        .LoadAsync(size as u32)
        .map_err(|e| format!("加载封面失败: {e}"))?
        .get()
        .map_err(|e| format!("封面加载等待失败: {e}"))?;
    let mut bytes = vec![0u8; size];
    reader
        .ReadBytes(&mut bytes)
        .map_err(|e| format!("读取封面字节失败: {e}"))?;
    // SMTC 不提供 MIME；按魔数判断（多数应用为 JPEG）
    let mime = if bytes.starts_with(b"\x89PNG") {
        "image/png"
    } else {
        "image/jpeg"
    };
    Ok(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    ))
}

/// 上一曲 / 下一曲 / 播放 / 暂停
fn control(action: &str) -> Result<(), String> {
    let mgr = manager()?;
    let session = mgr
        .GetCurrentSession()
        .map_err(|e| format!("获取媒体会话失败: {e}"))?;
    let is_playing = matches!(
        session
            .GetPlaybackInfo()
            .ok()
            .and_then(|p| p.PlaybackStatus().ok())
            .unwrap_or(GlobalSystemMediaTransportControlsSessionPlaybackStatus::Closed),
        GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
    );
    match action {
        "play" => {
            session
                .TryPlayAsync()
                .map_err(|e| format!("播放失败: {e}"))?
                .get()
                .map_err(|e| format!("播放等待失败: {e}"))?;
        }
        "pause" => {
            session
                .TryPauseAsync()
                .map_err(|e| format!("暂停失败: {e}"))?
                .get()
                .map_err(|e| format!("暂停等待失败: {e}"))?;
        }
        "playpause" => {
            if is_playing {
                session
                    .TryPauseAsync()
                    .map_err(|e| format!("暂停失败: {e}"))?
                    .get()
                    .map_err(|e| format!("暂停等待失败: {e}"))?;
            } else {
                session
                    .TryPlayAsync()
                    .map_err(|e| format!("播放失败: {e}"))?
                    .get()
                    .map_err(|e| format!("播放等待失败: {e}"))?;
            }
        }
        "next" | "previous" => {
            let ok = if action == "next" {
                session
                    .TrySkipNextAsync()
                    .map_err(|e| format!("下一曲失败: {e}"))?
                    .get()
                    .map_err(|e| format!("下一曲等待失败: {e}"))?
            } else {
                session
                    .TrySkipPreviousAsync()
                    .map_err(|e| format!("上一曲失败: {e}"))?
                    .get()
                    .map_err(|e| format!("上一曲等待失败: {e}"))?
            };
            if !ok {
                return Err("播放器拒绝了跳曲请求".into());
            }
            // 部分播放器 SMTC 跳曲后会自动暂停：若跳曲前在播放，补发一次播放命令恢复自动播放
            if is_playing {
                session
                    .TryPlayAsync()
                    .map_err(|e| format!("恢复播放失败: {e}"))?
                    .get()
                    .map_err(|e| format!("恢复播放等待失败: {e}"))?;
            }
        }
        _ => return Err(format!("未知控制动作: {action}")),
    }
    Ok(())
}

/// 查询当前播放信息（前端每 ~2s 轮询）
#[tauri::command]
pub fn media_now_playing() -> Result<MediaInfo, String> {
    let (tx, rx) = channel();
    worker_tx()
        .send(Request::Info(tx))
        .map_err(|e| format!("媒体线程不可用: {e}"))?;
    rx.recv_timeout(Duration::from_secs(5))
        .map_err(|e| format!("媒体查询超时: {e}"))?
}

/// 控制播放：previous / playpause / next（暂停/播放按当前状态切换）
#[tauri::command]
pub fn media_control(action: String) -> Result<(), String> {
    let (tx, rx) = channel();
    worker_tx()
        .send(Request::Control(action, tx))
        .map_err(|e| format!("媒体线程不可用: {e}"))?;
    rx.recv_timeout(Duration::from_secs(5))
        .map_err(|e| format!("媒体控制超时: {e}"))?
}
