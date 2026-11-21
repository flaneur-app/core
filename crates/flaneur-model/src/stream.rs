// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use url::Url;

use crate::{Item, Seconds, Timestamp};

/// Everything needed to play one item: the streams the service offers and their expiry.
///
/// Stream URLs often expire (YouTube's last about six hours) and may be tied to the IP that
/// resolved them, so a `Playable` is resolved right before playback and never stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Playable {
    /// The item, with the full details the watch page shows.
    pub item: Item,
    /// Separate or muxed streams, any order; the view-model's selector picks.
    pub streams: Vec<Stream>,
    /// HLS master playlist, for lives and services that only offer HLS.
    pub hls_manifest: Option<Url>,
    /// DASH manifest, when the service offers one.
    pub dash_manifest: Option<Url>,
    /// When the URLs above stop working, when known.
    pub expires_at: Option<Timestamp>,
    /// Where playback should start: the service's or the local resume position.
    pub start_at: Option<Seconds>,
    /// Opaque state the connector needs to report playback back to the service (for YouTube,
    /// the playback tracking URLs of the player response). Handed back in playback reports.
    pub tracking: Option<String>,
}

/// One stream of an item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stream {
    /// Direct URL.
    pub url: Url,
    /// Audio, video, or both.
    pub kind: StreamKind,
    /// File format.
    pub container: Container,
    /// Average bitrate in bits per second.
    pub bitrate: Option<u32>,
    /// Size in bytes.
    pub size: Option<u64>,
    /// Video width in pixels.
    pub width: Option<u32>,
    /// Video height in pixels.
    pub height: Option<u32>,
    /// Frames per second.
    pub fps: Option<u16>,
    /// Video codec.
    pub video_codec: Option<VideoCodec>,
    /// HDR video.
    pub hdr: Option<bool>,
    /// Audio track details.
    pub audio: Option<AudioTrack>,
}

/// What a stream carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StreamKind {
    /// Audio and video in one file.
    Muxed,
    /// Video only, to pair with an audio stream.
    Video,
    /// Audio only.
    Audio,
}

/// File format of a stream.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Container {
    /// MPEG-4 (`.mp4`, `.m4a`).
    Mp4,
    /// `WebM`.
    WebM,
    /// MPEG transport stream segments.
    Ts,
    /// Anything else, with its MIME type.
    Other(String),
}

/// Video codecs the selector knows how to rank.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VideoCodec {
    /// H.264 / AVC.
    H264,
    /// VP9.
    Vp9,
    /// AV1.
    Av1,
    /// H.265 / HEVC.
    H265,
    /// Anything else, with its codec string.
    Other(String),
}

/// Audio track of a stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioTrack {
    /// Codec string, such as `opus` or `mp4a.40.2`.
    pub codec: Option<String>,
    /// Language (BCP 47), for multi-language items.
    pub language: Option<String>,
    /// Display name of the track ("English (original)").
    pub label: Option<String>,
    /// The original track, not a dub.
    pub original: Option<bool>,
    /// Audio sample rate in hertz.
    pub sample_rate: Option<u32>,
    /// Number of channels.
    pub channels: Option<u8>,
}
