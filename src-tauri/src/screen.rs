//! ViewDesk screen-sharing sender — Rust port of the browser sender script.
//! Handles WebSocket signaling, WebRTC negotiation, and native screen capture.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use openh264::encoder::{
    BitRate, Complexity, Encoder, EncoderConfig, FrameRate, IntraFramePeriod, QpRange,
    RateControlMode, UsageType,
};
use openh264::formats::YUVSource;
use openh264::OpenH264API;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{mpsc, Mutex, Notify};
use tokio::time::{interval, sleep};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tokio_util::sync::CancellationToken;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MediaEngine, MIME_TYPE_H264};
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::media::Sample;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::offer_answer_options::RTCOfferOptions;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::policy::ice_transport_policy::RTCIceTransportPolicy;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_local::TrackLocal;
use webrtc::rtp_transceiver::rtp_codec::{RTCRtpCodecCapability, RTCRtpCodecParameters, RTPCodecType};
use webrtc::rtp_transceiver::RTCPFeedback;

#[cfg(windows)]
use windows::Win32::Foundation::POINT;
#[cfg(windows)]
use windows::Win32::Graphics::Gdi::{MonitorFromPoint, MONITOR_DEFAULTTONEAREST};
#[cfg(windows)]
use windows_capture::dxgi_duplication_api::{DxgiDuplicationApi, Error as DxgiError};
#[cfg(windows)]
use windows_capture::monitor::Monitor;

const HEARTBEAT_MS: u64 = 500;
const WS_RECONNECT_BASE_MS: u64 = 1000;
const WS_RECONNECT_MAX_MS: u64 = 30_000;
const ICE_RECOVERY_DELAY_MS: u64 = 8000;
const MAX_ICE_RESTARTS: u32 = 2;
/// Minimum gap between full stream restarts — prevents reconnect storms on WAN.
const RESTART_COOLDOWN_MS: u64 = 8000;
const TARGET_FPS: u64 = 30;
const FRAME_INTERVAL_MS: u64 = 1000 / TARGET_FPS;
/// Cap encode width — balances sharpness vs CPU.
const MAX_STREAM_WIDTH: usize = 1440;
/// Higher bitrate = sharper text; costs bandwidth only, not CPU.
const ENCODE_BITRATE_BPS: u32 = 6_000_000;

struct FrameScratch {
    yuv: Vec<u8>,
    encoded: Vec<u8>,
    width: usize,
    height: usize,
    scale: Option<ScaleCtx>,
}

struct ScaleCtx {
    src_w: usize,
    src_h: usize,
    x0: Vec<usize>,
    x1: Vec<usize>,
    y0: Vec<usize>,
    y1: Vec<usize>,
}

impl ScaleCtx {
    fn build(src_w: usize, src_h: usize, dst_w: usize, dst_h: usize) -> Self {
        Self {
            src_w,
            src_h,
            x0: (0..dst_w).map(|dx| dx * src_w / dst_w).collect(),
            x1: (0..dst_w).map(|dx| (dx + 1) * src_w / dst_w).collect(),
            y0: (0..dst_h).map(|dy| dy * src_h / dst_h).collect(),
            y1: (0..dst_h).map(|dy| (dy + 1) * src_h / dst_h).collect(),
        }
    }
}

impl FrameScratch {
    fn new() -> Self {
        Self {
            yuv: Vec::new(),
            encoded: Vec::with_capacity(64 * 1024),
            width: 0,
            height: 0,
            scale: None,
        }
    }

    fn ensure_size(&mut self, width: usize, height: usize) {
        let y_size = width * height;
        let total = y_size + y_size / 2;
        if self.width != width || self.height != height {
            self.yuv.resize(total, 0);
            self.width = width;
            self.height = height;
            self.scale = None;
        }
    }

    fn ensure_scale(&mut self, src_w: usize, src_h: usize) {
        let needs_new = self
            .scale
            .as_ref()
            .map_or(true, |ctx| ctx.src_w != src_w || ctx.src_h != src_h);
        if needs_new {
            self.scale = Some(ScaleCtx::build(src_w, src_h, self.width, self.height));
        }
    }
}

/// Keep in sync with ViewDesk `sender-config` / `src/lib/webrtc-config.ts`.
fn ice_servers() -> Vec<RTCIceServer> {
    expand_ice_servers(&[
        IceServerDef {
            urls: &["stun:stun.l.google.com:19302"],
            username: None,
            credential: None,
        },
        IceServerDef {
            urls: &["stun:stun1.l.google.com:19302"],
            username: None,
            credential: None,
        },
        IceServerDef {
            urls: &[
                "turn:bot.artesierra.com:3478",
                "turn:bot.artesierra.com:3478?transport=tcp",
            ],
            username: Some("viewdesk"),
            credential: Some("viewdesk"),
        },
        IceServerDef {
            urls: &[
                "turn:openrelay.metered.ca:80",
                "turn:openrelay.metered.ca:443",
                "turn:openrelay.metered.ca:443?transport=tcp",
                "turns:openrelay.metered.ca:443",
                "turns:openrelay.metered.ca:443?transport=tcp",
            ],
            username: Some("openrelayproject"),
            credential: Some("openrelayproject"),
        },
    ])
}

struct IceServerDef {
    urls: &'static [&'static str],
    username: Option<&'static str>,
    credential: Option<&'static str>,
}

/// Split multi-URL entries for TURN compatibility (mirrors `expandIceServers` in ice-utils.ts).
fn expand_ice_servers(servers: &[IceServerDef]) -> Vec<RTCIceServer> {
    let mut out = Vec::new();
    for server in servers {
        if server.urls.len() <= 1 {
            out.push(rtc_ice_server(
                server.urls[0],
                server.username,
                server.credential,
            ));
        } else {
            for url in server.urls {
                out.push(rtc_ice_server(url, server.username, server.credential));
            }
        }
    }
    out
}

fn rtc_ice_server(
    url: &str,
    username: Option<&str>,
    credential: Option<&str>,
) -> RTCIceServer {
    RTCIceServer {
        urls: vec![url.to_owned()],
        username: username.unwrap_or_default().to_owned(),
        credential: credential.unwrap_or_default().to_owned(),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ScreenError {
    #[error("websocket: {0}")]
    WebSocket(String),
    #[error("webrtc: {0}")]
    WebRtc(#[from] webrtc::Error),
    #[error("capture: {0}")]
    Capture(String),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn generate_sender_id() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn to_ws_url(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    if trimmed.starts_with("ws://") || trimmed.starts_with("wss://") {
        return trimmed.to_owned();
    }
    if let Some(host) = trimmed.strip_prefix("https://") {
        return format!("wss://{host}");
    }
    if let Some(host) = trimmed.strip_prefix("http://") {
        return format!("ws://{host}");
    }
    format!("ws://{trimmed}")
}

pub fn default_sender_name() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "Sender".to_owned())
}

#[derive(Clone)]
pub struct ScreenSenderConfig {
    pub signaling_url: String,
    pub sender_id: String,
    pub sender_name: String,
    /// Resolves a point on the desktop monitor to capture (e.g. app window center).
    pub monitor_point: Option<Arc<dyn Fn() -> Option<(i32, i32)> + Send + Sync>>,
}

impl Default for ScreenSenderConfig {
    fn default() -> Self {
        Self {
            signaling_url: "wss://bot.artesierra.com/signaling".to_owned(),
            sender_id: generate_sender_id(),
            sender_name: default_sender_name(),
            monitor_point: None,
        }
    }
}

impl std::fmt::Debug for ScreenSenderConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScreenSenderConfig")
            .field("signaling_url", &self.signaling_url)
            .field("sender_id", &self.sender_id)
            .field("sender_name", &self.sender_name)
            .field("monitor_point", &self.monitor_point.is_some())
            .finish()
    }
}

#[derive(Serialize)]
struct WsEnvelope<'a> {
    event: &'static str,
    payload: &'a Value,
}

struct StreamSession {
    peer_connection: Option<Arc<RTCPeerConnection>>,
    capture_cancel: Option<CancellationToken>,
    pending_ice: Vec<RTCIceCandidateInit>,
    streaming: bool,
    /// Receiver asked to stream — survives signaling drops for auto-resume.
    wants_stream: bool,
    needs_resume: bool,
    ice_restart_attempts: u32,
    recovery_in_progress: bool,
    recovery_tx: Option<mpsc::UnboundedSender<()>>,
    stream_ready: Option<Arc<Notify>>,
    media_active: Arc<AtomicBool>,
    force_keyframe: Arc<AtomicBool>,
    last_stream_start: Option<Instant>,
}

impl StreamSession {
    fn new() -> Self {
        Self {
            peer_connection: None,
            capture_cancel: None,
            pending_ice: Vec::new(),
            streaming: false,
            wants_stream: false,
            needs_resume: false,
            ice_restart_attempts: 0,
            recovery_in_progress: false,
            recovery_tx: None,
            stream_ready: None,
            media_active: Arc::new(AtomicBool::new(false)),
            force_keyframe: Arc::new(AtomicBool::new(false)),
            last_stream_start: None,
        }
    }
}

pub struct ScreenSender {
    config: ScreenSenderConfig,
    webrtc_api: Arc<webrtc::api::API>,
    session: Arc<Mutex<StreamSession>>,
    closed: Arc<AtomicBool>,
}

impl ScreenSender {
    pub fn new(config: ScreenSenderConfig) -> Result<Self, ScreenError> {
        let mut media_engine = MediaEngine::default();
        register_screen_share_codecs(&mut media_engine)
            .map_err(|e| ScreenError::WebRtc(e))?;

        let mut registry = Registry::new();
        registry = register_default_interceptors(registry, &mut media_engine)
            .map_err(|e| ScreenError::WebRtc(e))?;

        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .with_interceptor_registry(registry)
            .build();

        Ok(Self {
            config,
            webrtc_api: Arc::new(api),
            session: Arc::new(Mutex::new(StreamSession::new())),
            closed: Arc::new(AtomicBool::new(false)),
        })
    }

    #[allow(dead_code)]
    pub fn stop(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }

    pub async fn run(self: Arc<Self>) -> Result<(), ScreenError> {
        let sender = self;
        let mut backoff_ms = WS_RECONNECT_BASE_MS;

        while !sender.closed.load(Ordering::SeqCst) {
            match sender.clone().connect_once().await {
                Ok(()) => backoff_ms = WS_RECONNECT_BASE_MS,
                Err(_) if sender.closed.load(Ordering::SeqCst) => break,
                Err(_) => {}
            }

            if sender.closed.load(Ordering::SeqCst) {
                break;
            }

            sleep(Duration::from_millis(backoff_ms)).await;
            backoff_ms = (backoff_ms.saturating_mul(2)).min(WS_RECONNECT_MAX_MS);
        }

        sender.stop_streaming().await;
        Ok(())
    }

    async fn connect_once(self: Arc<Self>) -> Result<(), ScreenError> {
        let ws_url = to_ws_url(&self.config.signaling_url);
        let (ws_stream, _) = connect_async(&ws_url)
            .await
            .map_err(|e| ScreenError::WebSocket(e.to_string()))?;

        let (mut sink, mut stream) = ws_stream.split();
        let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Value>();
        let (recovery_tx, mut recovery_rx) = mpsc::unbounded_channel::<()>();
        {
            let mut session = self.session.lock().await;
            session.recovery_tx = Some(recovery_tx.clone());
        }

        let heartbeat_tx = out_tx.clone();
        let sender_id = self.config.sender_id.clone();
        let sender_name = self.config.sender_name.clone();
        let closed = Arc::clone(&self.closed);
        let session_for_heartbeat = Arc::clone(&self.session);

        let heartbeat_handle = tokio::spawn(async move {
            let mut ticker = interval(Duration::from_millis(HEARTBEAT_MS));
            loop {
                ticker.tick().await;
                if closed.load(Ordering::SeqCst) {
                    break;
                }
                let streaming = session_for_heartbeat.lock().await.streaming;
                let payload = serde_json::json!({
                    "type": "heartbeat",
                    "senderId": sender_id,
                    "name": sender_name,
                    "ts": chrono_timestamp_ms(),
                    "streaming": streaming,
                });
                if heartbeat_tx.send(payload).is_err() {
                    break;
                }
            }
        });

        let writer = tokio::spawn(async move {
            while let Some(payload) = out_rx.recv().await {
                let envelope = WsEnvelope {
                    event: "signal",
                    payload: &payload,
                };
                let text = match serde_json::to_string(&envelope) {
                    Ok(text) => text,
                    Err(_) => continue,
                };
                if sink.send(Message::Text(text.into())).await.is_err() {
                    break;
                }
            }
        });

        let out_for_reader = out_tx.clone();
        self.clone()
            .resume_stream_if_needed(&out_for_reader)
            .await;

        loop {
            if self.closed.load(Ordering::SeqCst) {
                let _ = out_for_reader.send(serde_json::json!({
                    "type": "bye",
                    "senderId": self.config.sender_id,
                }));
                sleep(Duration::from_millis(100)).await;
                break;
            }

            tokio::select! {
                message = stream.next() => {
                    let Some(message) = message else { break };
                    let message = message.map_err(|e| ScreenError::WebSocket(e.to_string()))?;
                    if let Message::Text(text) = message {
                        if let Ok(msg) = serde_json::from_str::<IncomingWsMessage>(&text) {
                            if msg.event == "signal" {
                                if let Some(payload) = msg.payload {
                            self.clone()
                                .handle_signal(payload, &out_for_reader)
                                .await;
                                }
                            }
                        }
                    }
                }
                _ = recovery_rx.recv() => {
                    while recovery_rx.try_recv().is_ok() {}
                    self.clone()
                        .try_recover_connection(&out_for_reader)
                        .await;
                }
            }
        }

        {
            let mut session = self.session.lock().await;
            session.recovery_tx = None;
        }

        heartbeat_handle.abort();
        writer.abort();
        // Signaling dropped — keep the media session alive; resume after reconnect.
        Ok(())
    }

    async fn handle_signal(
        self: Arc<Self>,
        payload: Value,
        out_tx: &mpsc::UnboundedSender<Value>,
    ) {
        let signal_type = payload
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();

        match signal_type {
            "command" => {
                let to = payload.get("to").and_then(Value::as_str).unwrap_or_default();
                if to != self.config.sender_id {
                    return;
                }
                match payload.get("action").and_then(Value::as_str) {
                    Some("start") => {
                        let _ = self.ensure_streaming(out_tx).await;
                    }
                    Some("stop") => self.stop_streaming().await,
                    _ => {}
                }
            }
            "answer" => {
                if !signal_to_matches_sender(&payload, &self.config.sender_id) {
                    return;
                }
                let Some(sdp_value) = payload.get("sdp") else {
                    return;
                };
                let Ok(sdp) = parse_remote_answer(sdp_value) else {
                    return;
                };

                let (pc, pending) = {
                    let mut session = self.session.lock().await;
                    let Some(pc) = session.peer_connection.clone() else {
                        return;
                    };
                    let pending = session.pending_ice.drain(..).collect::<Vec<_>>();
                    (pc, pending)
                };

                if pc.set_remote_description(sdp).await.is_err() {
                    return;
                }
                for candidate in pending {
                    let _ = pc.add_ice_candidate(candidate).await;
                }
                signal_stream_ready_if_connected(&self.session, &pc).await;
            }
            "ice" => {
                if !signal_to_matches_sender(&payload, &self.config.sender_id) {
                    return;
                }
                let Ok(candidate) = serde_json::from_value::<RTCIceCandidateInit>(
                    payload
                        .get("candidate")
                        .cloned()
                        .unwrap_or(Value::Null),
                ) else {
                    return;
                };

                let mut session = self.session.lock().await;
                let Some(pc) = session.peer_connection.as_ref() else {
                    session.pending_ice.push(candidate);
                    return;
                };

                if pc.remote_description().await.is_some() {
                    let _ = pc.add_ice_candidate(candidate).await;
                } else {
                    session.pending_ice.push(candidate);
                }
            }
            "fs-req" => {
                let to = payload.get("to").and_then(Value::as_str).unwrap_or_default();
                if to != self.config.sender_id {
                    return;
                }

                let request_id = payload.get("id").cloned().unwrap_or(Value::Null);
                let method = payload
                    .get("method")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                let params = payload
                    .get("params")
                    .cloned()
                    .unwrap_or_else(|| Value::Object(Default::default()));
                let sender_id = self.config.sender_id.clone();
                let out = out_tx.clone();

                tokio::spawn(async move {
                    let (ok, data, error) =
                        tokio::task::spawn_blocking(move || {
                            crate::fs_handler::handle_fs_method(&method, &params)
                        })
                        .await
                        .unwrap_or((false, None, Some("Filesystem task cancelled".to_owned())));

                    let mut response = serde_json::json!({
                        "type": "fs-res",
                        "id": request_id,
                        "to": "receiver",
                        "from": sender_id,
                        "ok": ok,
                    });

                    if ok {
                        response["data"] = data.unwrap_or(Value::Null);
                    } else if let Some(message) = error {
                        response["error"] = Value::String(message);
                    }

                    let _ = out.send(response);
                });
            }
            _ => {}
        }
    }

    async fn ensure_streaming(
        self: Arc<Self>,
        out_tx: &mpsc::UnboundedSender<Value>,
    ) -> Result<(), ScreenError> {
        {
            let mut session = self.session.lock().await;
            session.wants_stream = true;
            session.needs_resume = false;
        }

        if session_stream_in_progress(&self.session).await {
            return Ok(());
        }

        if !can_restart_stream(&self.session).await {
            return Ok(());
        }

        self.restart_streaming(out_tx).await
    }

    async fn resume_stream_if_needed(
        self: Arc<Self>,
        out_tx: &mpsc::UnboundedSender<Value>,
    ) {
        let should_resume = {
            let session = self.session.lock().await;
            if !session.wants_stream {
                false
            } else if session.needs_resume {
                true
            } else if !session.streaming {
                true
            } else {
                session.peer_connection.as_ref().is_some_and(|pc| {
                    matches!(
                        pc.connection_state(),
                        RTCPeerConnectionState::Failed | RTCPeerConnectionState::Closed
                    )
                })
            }
        };

        if !should_resume {
            return;
        }

        if session_stream_in_progress(&self.session).await {
            return;
        }

        if !can_restart_stream(&self.session).await {
            return;
        }

        let _ = self.restart_streaming(out_tx).await;
    }

    async fn restart_streaming(
        self: Arc<Self>,
        out_tx: &mpsc::UnboundedSender<Value>,
    ) -> Result<(), ScreenError> {
        self.teardown_media().await;
        self.start_streaming(out_tx).await
    }

    async fn teardown_media(&self) {
        let mut session = self.session.lock().await;
        session.streaming = false;
        session.recovery_in_progress = false;
        session.media_active.store(false, Ordering::SeqCst);
        session.stream_ready = None;
        if let Some(cancel) = session.capture_cancel.take() {
            cancel.cancel();
        }
        if let Some(pc) = session.peer_connection.take() {
            let _ = pc.close().await;
        }
        session.pending_ice.clear();
    }

    async fn start_streaming(
        self: Arc<Self>,
        out_tx: &mpsc::UnboundedSender<Value>,
    ) -> Result<(), ScreenError> {
        {
            let session = self.session.lock().await;
            if session.streaming {
                return Ok(());
            }
        }

        let point = self.config.monitor_point.as_ref().and_then(|resolve| resolve());

        #[cfg(not(windows))]
        {
            let _ = point;
            return Err(ScreenError::Capture(
                "native screen capture is only supported on Windows".into(),
            ));
        }

        #[cfg(windows)]
        let monitor = pick_desktop_monitor(point)?;

        let config = RTCConfiguration {
            ice_servers: ice_servers(),
            ice_candidate_pool_size: 10,
            ice_transport_policy: RTCIceTransportPolicy::All,
            ..Default::default()
        };

        let peer_connection = Arc::new(
            self.webrtc_api
                .new_peer_connection(config)
                .await
                .map_err(|e| ScreenError::WebRtc(e))?,
        );

        let video_track = Arc::new(TrackLocalStaticSample::new(
            rtp_capability(),
            "video".to_owned(),
            "daily-huddle".to_owned(),
        ));

        let rtp_sender = peer_connection
            .add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal + Send + Sync>)
            .await
            .map_err(|e| ScreenError::WebRtc(e))?;

        tokio::spawn(async move {
            let mut rtcp_buf = vec![0u8; 1500];
            while rtp_sender.read(&mut rtcp_buf).await.is_ok() {}
        });

        let sender_id = self.config.sender_id.clone();
        let out_for_ice = out_tx.clone();
        peer_connection.on_ice_candidate(Box::new(move |candidate: Option<RTCIceCandidate>| {
            let out_for_ice = out_for_ice.clone();
            let sender_id = sender_id.clone();
            Box::pin(async move {
                if let Some(candidate) = candidate {
                    if let Ok(init) = candidate.to_json() {
                        let _ = out_for_ice.send(serde_json::json!({
                            "type": "ice",
                            "from": sender_id,
                            "to": "receiver",
                            "candidate": init,
                        }));
                    }
                }
            })
        }));

        let stream_ready = Arc::new(Notify::new());
        let media_active = {
            let mut session = self.session.lock().await;
            session.stream_ready = Some(Arc::clone(&stream_ready));
            session.media_active.store(false, Ordering::SeqCst);
            Arc::clone(&session.media_active)
        };
        let stream_ready_ice = Arc::clone(&stream_ready);
        peer_connection.on_ice_connection_state_change(Box::new(
            move |state: RTCIceConnectionState| {
                if ice_connection_ready(state) {
                    stream_ready_ice.notify_waiters();
                }
                Box::pin(async {})
            },
        ));

        let stream_ready_pc = Arc::clone(&stream_ready);
        peer_connection.on_peer_connection_state_change(Box::new(
            move |state: RTCPeerConnectionState| {
                if state == RTCPeerConnectionState::Connected {
                    stream_ready_pc.notify_waiters();
                }
                Box::pin(async {})
            },
        ));

        let session_for_pc = Arc::clone(&self.session);
        peer_connection.on_peer_connection_state_change(Box::new(
            move |state: RTCPeerConnectionState| {
                let session = Arc::clone(&session_for_pc);
                Box::pin(async move {
                    if state == RTCPeerConnectionState::Failed {
                        let tx = session.lock().await.recovery_tx.clone();
                        if let Some(tx) = tx {
                            let _ = tx.send(());
                        }
                    }
                })
            },
        ));

        let offer = peer_connection
            .create_offer(None)
            .await
            .map_err(|e| ScreenError::WebRtc(e))?;
        peer_connection
            .set_local_description(offer)
            .await
            .map_err(|e| ScreenError::WebRtc(e))?;

        if let Some(local_desc) = peer_connection.local_description().await {
            let _ = out_tx.send(serde_json::json!({
                "type": "offer",
                "senderId": self.config.sender_id,
                "sdp": local_desc,
            }));
        }

        let capture_cancel = CancellationToken::new();
        let capture_token = capture_cancel.clone();
        let track_for_capture = Arc::clone(&video_track);
        let force_keyframe = {
            let session = self.session.lock().await;
            Arc::clone(&session.force_keyframe)
        };
        let (std_frame_tx, std_frame_rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(1);
        let (frame_tx, mut frame_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let capture_for_writer = capture_cancel.clone();
        let stream_ready_writer = Arc::clone(&stream_ready);
        let peer_for_writer = Arc::clone(&peer_connection);

        tokio::task::spawn_blocking(move || {
            while let Ok(encoded) = std_frame_rx.recv() {
                let _ = frame_tx.send(encoded);
            }
        });

        // Start native capture immediately (no browser screen-picker dialog).
        #[cfg(windows)]
        tokio::task::spawn_blocking(move || {
            run_capture_loop_blocking(monitor, std_frame_tx, capture_token, force_keyframe);
        });

        tokio::spawn(async move {
            wait_until_can_send_media(peer_for_writer, stream_ready_writer).await;
            media_active.store(true, Ordering::SeqCst);
            while let Some(mut encoded) = frame_rx.recv().await {
                if capture_for_writer.is_cancelled() {
                    break;
                }
                while let Ok(newer) = frame_rx.try_recv() {
                    if has_idr(&encoded) && !has_idr(&newer) {
                        break;
                    }
                    encoded = newer;
                }
                let _ = track_for_capture
                    .write_sample(&Sample {
                        data: Bytes::from(encoded),
                        duration: Duration::from_millis(FRAME_INTERVAL_MS),
                        ..Default::default()
                    })
                    .await;
            }
            media_active.store(false, Ordering::SeqCst);
        });

        let mut session = self.session.lock().await;
        session.peer_connection = Some(peer_connection);
        session.capture_cancel = Some(capture_cancel);
        session.pending_ice.clear();
        session.streaming = true;
        session.wants_stream = true;
        session.needs_resume = false;
        session.ice_restart_attempts = 0;
        session.recovery_in_progress = false;
        session.last_stream_start = Some(Instant::now());

        Ok(())
    }

    async fn stop_streaming(&self) {
        {
            let mut session = self.session.lock().await;
            if !session.streaming && !session.wants_stream {
                return;
            }
            session.wants_stream = false;
            session.needs_resume = false;
        }
        self.teardown_media().await;
    }

    async fn try_recover_connection(
        self: Arc<Self>,
        out_tx: &mpsc::UnboundedSender<Value>,
    ) {
        sleep(Duration::from_millis(ICE_RECOVERY_DELAY_MS)).await;

        let (wants_stream, pc, attempts) = {
            let mut s = self.session.lock().await;
            if s.recovery_in_progress {
                return;
            }
            let wants = s.wants_stream;
            let pc = s.peer_connection.clone();
            let attempts = s.ice_restart_attempts;
            if wants && pc.is_some() {
                s.recovery_in_progress = true;
            }
            (wants, pc, attempts)
        };

        if !wants_stream {
            return;
        }

        let Some(pc) = pc else {
            let mut s = self.session.lock().await;
            s.recovery_in_progress = false;
            s.needs_resume = true;
            return;
        };

        let ice_state = pc.ice_connection_state();
        let pc_state = pc.connection_state();
        if ice_connection_ready(ice_state)
            || matches!(pc_state, RTCPeerConnectionState::Connected | RTCPeerConnectionState::Connecting)
        {
            let mut s = self.session.lock().await;
            s.recovery_in_progress = false;
            s.ice_restart_attempts = 0;
            return;
        }

        // Transient WAN blips — ICE often self-recovers from Disconnected.
        if ice_state == RTCIceConnectionState::Disconnected
            || pc_state == RTCPeerConnectionState::Disconnected
        {
            let mut s = self.session.lock().await;
            s.recovery_in_progress = false;
            return;
        }

        let recovered = if attempts < MAX_ICE_RESTARTS
            && matches!(
                ice_state,
                RTCIceConnectionState::Failed
            )
        {
            self.try_ice_restart(&pc, out_tx).await
        } else {
            false
        };

        if recovered {
            let mut s = self.session.lock().await;
            s.recovery_in_progress = false;
            s.ice_restart_attempts += 1;
            s.needs_resume = false;
            s.force_keyframe.store(true, Ordering::SeqCst);
            return;
        }

        {
            let mut s = self.session.lock().await;
            s.recovery_in_progress = false;
            s.ice_restart_attempts = 0;
        }

        if !can_restart_stream(&self.session).await {
            let mut s = self.session.lock().await;
            s.needs_resume = true;
            return;
        }

        let _ = self.restart_streaming(out_tx).await;
    }

    async fn try_ice_restart(
        &self,
        pc: &RTCPeerConnection,
        out_tx: &mpsc::UnboundedSender<Value>,
    ) -> bool {
        if pc.restart_ice().await.is_err() {
            return false;
        }

        let offer = match pc
            .create_offer(Some(RTCOfferOptions {
                ice_restart: true,
                ..Default::default()
            }))
            .await
        {
            Ok(offer) => offer,
            Err(_) => return false,
        };

        if pc.set_local_description(offer).await.is_err() {
            return false;
        }

        if let Some(local_desc) = pc.local_description().await {
            out_tx
                .send(serde_json::json!({
                    "type": "offer",
                    "senderId": self.config.sender_id,
                    "sdp": local_desc,
                }))
                .is_ok()
        } else {
            false
        }
    }
}

#[derive(Deserialize)]
struct IncomingWsMessage {
    event: String,
    payload: Option<Value>,
}

fn capture_err(err: impl std::fmt::Display) -> ScreenError {
    ScreenError::Capture(err.to_string())
}

fn ice_connection_ready(state: RTCIceConnectionState) -> bool {
    matches!(
        state,
        RTCIceConnectionState::Connected | RTCIceConnectionState::Completed
    )
}

/// True while a peer connection exists and negotiation is underway or healthy.
async fn session_stream_in_progress(session: &Arc<Mutex<StreamSession>>) -> bool {
    let session = session.lock().await;
    if !session.streaming {
        return false;
    }
    let Some(pc) = session.peer_connection.as_ref() else {
        return false;
    };
    matches!(
        pc.connection_state(),
        RTCPeerConnectionState::New
            | RTCPeerConnectionState::Connecting
            | RTCPeerConnectionState::Connected
    )
}

async fn can_restart_stream(session: &Arc<Mutex<StreamSession>>) -> bool {
    let session = session.lock().await;
    session
        .last_stream_start
        .map(|t| t.elapsed() >= Duration::from_millis(RESTART_COOLDOWN_MS))
        .unwrap_or(true)
}

async fn signal_stream_ready_if_connected(
    session: &Arc<Mutex<StreamSession>>,
    pc: &RTCPeerConnection,
) {
    if !peer_media_ready(pc).await {
        return;
    }
    let notify = session.lock().await.stream_ready.clone();
    if let Some(notify) = notify {
        notify.notify_waiters();
    }
}

/// Wait until negotiation is complete and the peer connection is fully up.
/// Matches the browser receiver showing LIVE (`connectionState === "connected"`).
async fn wait_until_can_send_media(pc: Arc<RTCPeerConnection>, notify: Arc<Notify>) {
    loop {
        if peer_media_ready(pc.as_ref()).await {
            sleep(Duration::from_millis(150)).await;
            return;
        }
        tokio::select! {
            _ = notify.notified() => {}
            _ = sleep(Duration::from_millis(100)) => {}
        }
    }
}

async fn peer_media_ready(pc: &RTCPeerConnection) -> bool {
    if pc.remote_description().await.is_none() {
        return false;
    }
    if !ice_connection_ready(pc.ice_connection_state()) {
        return false;
    }
    matches!(
        pc.connection_state(),
        RTCPeerConnectionState::Connected
    )
}

fn signal_to_matches_sender(payload: &Value, sender_id: &str) -> bool {
    payload
        .get("to")
        .and_then(Value::as_str)
        .map_or(true, |to| to == sender_id)
}

fn parse_remote_answer(sdp_value: &Value) -> Result<RTCSessionDescription, ScreenError> {
    match sdp_value {
        Value::String(sdp_text) => RTCSessionDescription::answer(sdp_text.clone()).map_err(ScreenError::WebRtc),
        Value::Object(_) => {
            let desc: RTCSessionDescription = serde_json::from_value(sdp_value.clone())?;
            RTCSessionDescription::answer(desc.sdp).map_err(ScreenError::WebRtc)
        }
        _ => Err(ScreenError::WebSocket("invalid answer sdp".into())),
    }
}

#[cfg(windows)]
fn monitor_from_point(x: i32, y: i32) -> Option<Monitor> {
    let hmonitor = unsafe { MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST) };
    if hmonitor.is_invalid() {
        return None;
    }
    Some(Monitor::from_raw_hmonitor(hmonitor.0))
}

#[cfg(windows)]
fn pick_desktop_monitor(point: Option<(i32, i32)>) -> Result<Monitor, ScreenError> {
    if let Some((x, y)) = point {
        if let Some(monitor) = monitor_from_point(x, y) {
            return Ok(monitor);
        }
    }

    Monitor::primary().map_err(capture_err)
}

fn stream_dimensions(src_width: usize, src_height: usize) -> (usize, usize) {
    let src_width = src_width & !1;
    let src_height = src_height & !1;
    if src_width <= MAX_STREAM_WIDTH {
        return (src_width, src_height);
    }
    let dst_width = MAX_STREAM_WIDTH;
    let dst_height = ((src_height * dst_width) / src_width) & !1;
    (dst_width, dst_height.max(2))
}

/// Integer BT.601 RGB → YUV for one pixel.
#[inline]
fn rgb_to_yuv(r: i32, g: i32, b: i32) -> (u8, u8, u8) {
    (
        ((77 * r + 150 * g + 29 * b + 128) >> 8) as u8,
        ((-43 * r - 85 * g + 128 * b + 32768) >> 8) as u8,
        ((128 * r - 107 * g - 21 * b + 32768) >> 8) as u8,
    )
}

/// DXGI Desktop Duplication returns BGRA8 pixel order.
#[inline]
fn read_rgb(pixels: &[u8], idx: usize) -> (i32, i32, i32) {
    (
        pixels[idx + 2] as i32,
        pixels[idx + 1] as i32,
        pixels[idx] as i32,
    )
}

fn pixels_to_i420_scaled_into(
    pixels: &[u8],
    src_width: usize,
    src_height: usize,
    scratch: &mut FrameScratch,
) {
    let dst_width = scratch.width;
    let dst_height = scratch.height;
    let y_size = dst_width * dst_height;
    let uv_plane = y_size / 4;

    if src_width == dst_width && src_height == dst_height {
        let yuv = &mut scratch.yuv;
        for y in 0..dst_height {
            for x in 0..dst_width {
                let idx = (y * src_width + x) * 4;
                let (yy, u, v) = rgb_to_yuv_from_pixel(pixels, idx);
                yuv[y * dst_width + x] = yy;
                if x % 2 == 0 && y % 2 == 0 {
                    let uv_idx = (y / 2) * (dst_width / 2) + (x / 2);
                    yuv[y_size + uv_idx] = u;
                    yuv[y_size + uv_plane + uv_idx] = v;
                }
            }
        }
        return;
    }

    scratch.ensure_scale(src_width, src_height);
    let scale = scratch.scale.as_ref().expect("scale ctx");
    let yuv = &mut scratch.yuv;
    for dy in 0..dst_height {
        let sy0 = scale.y0[dy];
        let sy1 = scale.y1[dy].max(sy0 + 1);
        for dx in 0..dst_width {
            let sx0 = scale.x0[dx];
            let sx1 = scale.x1[dx].max(sx0 + 1);

            let mut r_sum = 0i32;
            let mut g_sum = 0i32;
            let mut b_sum = 0i32;
            let mut count = 0i32;

            for sy in sy0..sy1 {
                let row = sy * src_width * 4;
                for sx in sx0..sx1 {
                    let (r, g, b) = read_rgb(pixels, row + sx * 4);
                    r_sum += r;
                    g_sum += g;
                    b_sum += b;
                    count += 1;
                }
            }

            let (yy, u, v) = rgb_to_yuv(r_sum / count, g_sum / count, b_sum / count);
            yuv[dy * dst_width + dx] = yy;

            if dx % 2 == 0 && dy % 2 == 0 {
                let uv_idx = (dy / 2) * (dst_width / 2) + (dx / 2);
                yuv[y_size + uv_idx] = u;
                yuv[y_size + uv_plane + uv_idx] = v;
            }
        }
    }
}

#[inline]
fn rgb_to_yuv_from_pixel(pixels: &[u8], idx: usize) -> (u8, u8, u8) {
    let (r, g, b) = read_rgb(pixels, idx);
    rgb_to_yuv(r, g, b)
}

fn pace_to_frame_interval(started: Instant) {
    let elapsed = started.elapsed();
    if elapsed < Duration::from_millis(FRAME_INTERVAL_MS) {
        std::thread::sleep(Duration::from_millis(FRAME_INTERVAL_MS) - elapsed);
    }
}

#[cfg(windows)]
fn run_capture_loop_blocking(
    monitor: Monitor,
    frame_tx: std::sync::mpsc::SyncSender<Vec<u8>>,
    cancel: CancellationToken,
    force_keyframe: Arc<AtomicBool>,
) {
    let mut encoder: Option<VideoEncoder> = None;
    let mut scratch = FrameScratch::new();
    let mut pixel_scratch = Vec::new();
    let mut first_frame = true;
    let mut duplication: Option<DxgiDuplicationApi> = None;

    while !cancel.is_cancelled() {
        let started = std::time::Instant::now();

        if duplication.is_none() {
            duplication = DxgiDuplicationApi::new(monitor).ok();
        }
        let Some(dup) = duplication.as_mut() else {
            std::thread::sleep(Duration::from_millis(FRAME_INTERVAL_MS));
            continue;
        };

        let frame_result = dup.acquire_next_frame(FRAME_INTERVAL_MS as u32);
        let mut frame = match frame_result {
            Ok(frame) => frame,
            Err(DxgiError::Timeout) => continue,
            Err(DxgiError::AccessLost) => {
                duplication = None;
                continue;
            }
            Err(_) => {
                duplication = None;
                std::thread::sleep(Duration::from_millis(FRAME_INTERVAL_MS));
                continue;
            }
        };

        if frame.frame_info().LastPresentTime == 0 {
            continue;
        }

        let buffer = match frame.buffer() {
            Ok(buffer) => buffer,
            Err(_) => continue,
        };
        let src_width = buffer.width() as usize;
        let src_height = buffer.height() as usize;
        let pixels = buffer.as_nopadding_buffer(&mut pixel_scratch);
        process_captured_frame(
            pixels,
            src_width,
            src_height,
            &mut encoder,
            &mut scratch,
            &mut first_frame,
            &force_keyframe,
            &frame_tx,
        );

        pace_to_frame_interval(started);
    }
}

fn process_captured_frame(
    pixels: &[u8],
    src_width: usize,
    src_height: usize,
    encoder: &mut Option<VideoEncoder>,
    scratch: &mut FrameScratch,
    first_frame: &mut bool,
    force_keyframe: &Arc<AtomicBool>,
    frame_tx: &std::sync::mpsc::SyncSender<Vec<u8>>,
) {
    let (enc_width, enc_height) = stream_dimensions(src_width, src_height);
    if enc_width == 0 || enc_height == 0 {
        return;
    }

    let enc_w = enc_width as u32;
    let enc_h = enc_height as u32;
    let needs_new_encoder = encoder.as_ref().map_or(true, |enc| {
        enc.width() != enc_w || enc.height() != enc_h
    });
    if needs_new_encoder {
        *encoder = VideoEncoder::new(enc_w, enc_h, ENCODE_BITRATE_BPS, TARGET_FPS as u32).ok();
        *first_frame = true;
    }
    let Some(encoder) = encoder.as_mut() else {
        return;
    };

    scratch.ensure_size(enc_width, enc_height);
    pixels_to_i420_scaled_into(pixels, src_width, src_height, scratch);

    let mut need_idr = *first_frame || force_keyframe.swap(false, Ordering::SeqCst);
    if need_idr {
        encoder.force_keyframe();
        *first_frame = false;
    }

    let mut sent = false;
    for _ in 0..8 {
        match encoder.encode_i420(&scratch.yuv, &mut scratch.encoded) {
            Ok(true)
                if !scratch.encoded.is_empty()
                    && (!need_idr || has_idr(&scratch.encoded)) =>
            {
                sent = true;
                break;
            }
            Ok(true) => need_idr = true,
            Ok(false) => {
                std::thread::sleep(Duration::from_millis(5));
                if need_idr {
                    encoder.force_keyframe();
                }
            }
            Err(_) => return,
        }
    }
    if !sent {
        return;
    }

    match frame_tx.try_send(std::mem::take(&mut scratch.encoded)) {
        Ok(()) => {
            if scratch.encoded.capacity() < 64 * 1024 {
                scratch.encoded.reserve(64 * 1024);
            }
        }
        Err(std::sync::mpsc::TrySendError::Full(encoded)) => {
            scratch.encoded = encoded;
        }
        Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {}
    }
}

// --- H.264 encoder + WebRTC codec helpers (inline; no separate modules) ---

const H264_FMTP: &str = "level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42e01f";

fn register_screen_share_codecs(media_engine: &mut MediaEngine) -> Result<(), webrtc::Error> {
    let video_rtcp_feedback = vec![
        RTCPFeedback {
            typ: "goog-remb".to_owned(),
            parameter: "".to_owned(),
        },
        RTCPFeedback {
            typ: "ccm".to_owned(),
            parameter: "fir".to_owned(),
        },
        RTCPFeedback {
            typ: "nack".to_owned(),
            parameter: "".to_owned(),
        },
        RTCPFeedback {
            typ: "nack".to_owned(),
            parameter: "pli".to_owned(),
        },
    ];

    media_engine.register_codec(
        RTCRtpCodecParameters {
            capability: RTCRtpCodecCapability {
                mime_type: MIME_TYPE_H264.to_owned(),
                clock_rate: 90_000,
                channels: 0,
                sdp_fmtp_line: H264_FMTP.to_owned(),
                rtcp_feedback: video_rtcp_feedback,
            },
            payload_type: 96,
            ..Default::default()
        },
        RTPCodecType::Video,
    )
}

fn rtp_capability() -> RTCRtpCodecCapability {
    RTCRtpCodecCapability {
        mime_type: MIME_TYPE_H264.to_owned(),
        clock_rate: 90_000,
        sdp_fmtp_line: H264_FMTP.to_owned(),
        ..Default::default()
    }
}

fn has_idr(data: &[u8]) -> bool {
    annex_b_has_nal(data, |header| (header & 0x1F) == 5)
}

struct ReusedYuv<'a> {
    data: &'a [u8],
    width: usize,
    height: usize,
}

impl YUVSource for ReusedYuv<'_> {
    fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    fn strides(&self) -> (usize, usize, usize) {
        let uv_stride = self.width / 2;
        (self.width, uv_stride, uv_stride)
    }

    fn y(&self) -> &[u8] {
        let y_size = self.width * self.height;
        &self.data[..y_size]
    }

    fn u(&self) -> &[u8] {
        let y_size = self.width * self.height;
        let uv_plane = y_size / 4;
        &self.data[y_size..y_size + uv_plane]
    }

    fn v(&self) -> &[u8] {
        let y_size = self.width * self.height;
        let uv_plane = y_size / 4;
        &self.data[y_size + uv_plane..y_size + uv_plane * 2]
    }
}

struct VideoEncoder {
    encoder: Encoder,
    width: u32,
    height: u32,
}

impl VideoEncoder {
    fn new(width: u32, height: u32, bitrate_bps: u32, fps: u32) -> Result<Self, String> {
        Ok(Self {
            encoder: create_h264_encoder(bitrate_bps, fps)?,
            width,
            height,
        })
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn force_keyframe(&mut self) {
        self.encoder.force_intra_frame();
    }

    fn encode_i420(&mut self, i420: &[u8], out: &mut Vec<u8>) -> Result<bool, String> {
        let yuv = ReusedYuv {
            data: i420,
            width: self.width as usize,
            height: self.height as usize,
        };
        let bitstream = self
            .encoder
            .encode(&yuv)
            .map_err(|e| format!("openh264 encode: {e}"))?;
        out.clear();
        bitstream.write_vec(out);
        Ok(!out.is_empty())
    }
}

fn create_h264_encoder(bitrate_bps: u32, fps: u32) -> Result<Encoder, String> {
    let threads = std::thread::available_parallelism()
        .map(|count| count.get() as u16)
        .unwrap_or(4)
        .clamp(2, 8);

    let config = EncoderConfig::new()
        .usage_type(UsageType::ScreenContentRealTime)
        .adaptive_quantization(false)
        .background_detection(false)
        .debug(false)
        .complexity(Complexity::Low)
        .num_threads(threads)
        .max_frame_rate(FrameRate::from_hz(fps as f32))
        .bitrate(BitRate::from_bps(bitrate_bps))
        .rate_control_mode(RateControlMode::Bitrate)
        .qp(QpRange::new(18, 36))
        .scene_change_detect(false)
        .intra_frame_period(IntraFramePeriod::from_num_frames(fps));

    Encoder::with_api_config(OpenH264API::from_source(), config)
        .map_err(|e| format!("openh264 init: {e}"))
}

fn annex_b_has_nal(data: &[u8], mut is_idr: impl FnMut(u8) -> bool) -> bool {
    let mut i = 0usize;
    while i < data.len() {
        let (sc_len, nal_idx) = if i + 3 < data.len() && data[i..i + 3] == [0, 0, 1] {
            (3, i + 3)
        } else if i + 4 < data.len() && data[i..i + 4] == [0, 0, 0, 1] {
            (4, i + 4)
        } else {
            i += 1;
            continue;
        };
        if nal_idx < data.len() && is_idr(data[nal_idx]) {
            return true;
        }
        i = nal_idx.saturating_add(1).max(i + sc_len);
    }
    false
}

fn chrono_timestamp_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
