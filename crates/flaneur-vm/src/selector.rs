// SPDX-License-Identifier: AGPL-3.0-or-later

//! Picks the streams to play from what a service offers.
//!
//! Speed is the first priority of Flâneur, and the codec matters most for it: a codec the GPU
//! cannot decode falls back to the CPU and makes playback stutter on modest machines. The app
//! tells the selector which codecs have hardware decoding; those always rank first.

use flaneur_model::{Playable, Stream, StreamKind, VideoCodec};

/// What the user and the machine prefer.
#[derive(Debug, Clone)]
pub struct SelectionPrefs {
    /// Highest video height to pick (for instance 1080).
    pub max_height: u32,
    /// Codecs with hardware decoding on this machine, any order.
    pub hardware_codecs: Vec<VideoCodec>,
    /// Software codecs ranked by preference, best first; codecs missing here rank last.
    pub codec_order: Vec<VideoCodec>,
    /// Preferred audio language (BCP 47); `None` keeps the original track.
    pub audio_language: Option<String>,
    /// Audio only (the "audio mode").
    pub audio_only: bool,
}

impl Default for SelectionPrefs {
    fn default() -> Self {
        Self {
            max_height: 1080,
            hardware_codecs: Vec::new(),
            codec_order: vec![VideoCodec::Av1, VideoCodec::Vp9, VideoCodec::H264],
            audio_language: None,
            audio_only: false,
        }
    }
}

/// What to hand to the player.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Selection<'a> {
    /// Separate video and audio streams, played together.
    Split {
        /// Video stream.
        video: &'a Stream,
        /// Audio stream.
        audio: &'a Stream,
    },
    /// One stream with audio and video (or audio only, in audio mode).
    Single(&'a Stream),
    /// An adaptive manifest (HLS), when no direct stream fits; the player picks the variant.
    Hls(&'a url::Url),
}

/// Picks the streams of a [`Playable`]; `None` when nothing is playable.
#[must_use]
pub fn select_streams<'a>(playable: &'a Playable, prefs: &SelectionPrefs) -> Option<Selection<'a>> {
    let audio = best_audio(&playable.streams, prefs);
    if prefs.audio_only
        && let Some(audio) = audio
    {
        return Some(Selection::Single(audio));
    }

    let video = playable
        .streams
        .iter()
        .filter(|s| s.kind == StreamKind::Video && height(s) <= prefs.max_height)
        .max_by_key(|s| video_rank(s, prefs));
    if let (Some(video), Some(audio)) = (video, audio) {
        return Some(Selection::Split { video, audio });
    }

    let muxed = playable
        .streams
        .iter()
        .filter(|s| s.kind == StreamKind::Muxed && height(s) <= prefs.max_height)
        .max_by_key(|s| video_rank(s, prefs));
    if let Some(muxed) = muxed {
        return Some(Selection::Single(muxed));
    }

    playable.hls_manifest.as_ref().map(Selection::Hls)
}

fn height(stream: &Stream) -> u32 {
    stream.height.unwrap_or(0)
}

/// Higher is better: hardware decoding first, then resolution, then codec order, then fps and
/// bitrate.
fn video_rank(stream: &Stream, prefs: &SelectionPrefs) -> (bool, u32, usize, u16, u32) {
    let codec = stream.video_codec.as_ref();
    let hardware = codec.is_some_and(|c| prefs.hardware_codecs.contains(c));
    let codec_score = codec
        .and_then(|c| prefs.codec_order.iter().position(|o| o == c))
        .map_or(0, |pos| prefs.codec_order.len() - pos);
    (
        hardware,
        height(stream),
        codec_score,
        stream.fps.unwrap_or(0),
        stream.bitrate.unwrap_or(0),
    )
}

fn best_audio<'a>(streams: &'a [Stream], prefs: &SelectionPrefs) -> Option<&'a Stream> {
    streams
        .iter()
        .filter(|s| s.kind == StreamKind::Audio)
        .max_by_key(|s| {
            let track = s.audio.as_ref();
            let language_match = match (
                &prefs.audio_language,
                track.and_then(|t| t.language.as_ref()),
            ) {
                (Some(wanted), Some(lang)) => {
                    lang == wanted || lang.starts_with(&format!("{wanted}-"))
                }
                _ => false,
            };
            let original = track.and_then(|t| t.original).unwrap_or(false);
            (language_match, original, s.bitrate.unwrap_or(0))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use flaneur_model::{
        AudioTrack, Container, ContentFlags, Item, ItemKind, ItemRef, ReactionCounts, ServiceId,
    };
    use url::Url;

    fn stream(
        kind: StreamKind,
        height: Option<u32>,
        codec: Option<VideoCodec>,
        bitrate: u32,
    ) -> Stream {
        Stream {
            url: Url::parse(&format!(
                "https://cdn.example/{kind:?}/{height:?}/{codec:?}/{bitrate}"
            ))
            .unwrap(),
            kind,
            container: Container::WebM,
            bitrate: Some(bitrate),
            size: None,
            width: None,
            height,
            fps: Some(30),
            video_codec: codec,
            hdr: None,
            audio: None,
        }
    }

    fn audio(language: &str, original: bool, bitrate: u32) -> Stream {
        Stream {
            audio: Some(AudioTrack {
                codec: Some("opus".into()),
                language: Some(language.into()),
                label: None,
                original: Some(original),
                sample_rate: None,
                channels: None,
            }),
            ..stream(StreamKind::Audio, None, None, bitrate)
        }
    }

    fn playable(streams: Vec<Stream>) -> Playable {
        Playable {
            item: Item {
                r#ref: ItemRef::new(ServiceId::new("test"), "x"),
                kind: ItemKind::Video,
                title: "x".into(),
                description: None,
                duration: None,
                published: None,
                scheduled_start: None,
                channel: None,
                thumbnails: vec![],
                views: None,
                reactions: ReactionCounts::default(),
                flags: ContentFlags::default(),
                web_url: None,
                watch: None,
            },
            streams,
            hls_manifest: None,
            dash_manifest: None,
            expires_at: None,
            start_at: None,
            tracking: None,
        }
    }

    #[test]
    fn hardware_codec_beats_higher_resolution() {
        let p = playable(vec![
            stream(
                StreamKind::Video,
                Some(1080),
                Some(VideoCodec::Av1),
                3_000_000,
            ),
            stream(
                StreamKind::Video,
                Some(720),
                Some(VideoCodec::H264),
                2_000_000,
            ),
            audio("en", true, 128_000),
        ]);
        let prefs = SelectionPrefs {
            hardware_codecs: vec![VideoCodec::H264],
            ..SelectionPrefs::default()
        };
        let Some(Selection::Split { video, .. }) = select_streams(&p, &prefs) else {
            panic!("expected split streams");
        };
        assert_eq!(video.video_codec, Some(VideoCodec::H264));
    }

    #[test]
    fn respects_max_height_then_codec_order() {
        let p = playable(vec![
            stream(
                StreamKind::Video,
                Some(2160),
                Some(VideoCodec::Av1),
                9_000_000,
            ),
            stream(
                StreamKind::Video,
                Some(1080),
                Some(VideoCodec::H264),
                3_000_000,
            ),
            stream(
                StreamKind::Video,
                Some(1080),
                Some(VideoCodec::Vp9),
                2_500_000,
            ),
            audio("en", true, 128_000),
        ]);
        let Some(Selection::Split { video, .. }) = select_streams(&p, &SelectionPrefs::default())
        else {
            panic!("expected split streams");
        };
        assert_eq!(
            (video.height, video.video_codec.clone()),
            (Some(1080), Some(VideoCodec::Vp9))
        );
    }

    #[test]
    fn prefers_requested_language_then_original_track() {
        let p = playable(vec![
            stream(StreamKind::Video, Some(720), Some(VideoCodec::H264), 1),
            audio("en", true, 160_000),
            audio("fr-FR", false, 128_000),
        ]);
        let fr = SelectionPrefs {
            audio_language: Some("fr".into()),
            ..SelectionPrefs::default()
        };
        let Some(Selection::Split { audio: a, .. }) = select_streams(&p, &fr) else {
            panic!("expected split streams");
        };
        assert_eq!(a.audio.as_ref().unwrap().language.as_deref(), Some("fr-FR"));

        let Some(Selection::Split { audio: a, .. }) =
            select_streams(&p, &SelectionPrefs::default())
        else {
            panic!("expected split streams");
        };
        assert_eq!(a.audio.as_ref().unwrap().language.as_deref(), Some("en"));
    }

    #[test]
    fn falls_back_to_muxed_then_hls() {
        let muxed = playable(vec![stream(
            StreamKind::Muxed,
            Some(360),
            Some(VideoCodec::H264),
            1,
        )]);
        assert!(matches!(
            select_streams(&muxed, &SelectionPrefs::default()),
            Some(Selection::Single(_))
        ));

        let mut live = playable(vec![]);
        live.hls_manifest = Some(Url::parse("https://cdn.example/master.m3u8").unwrap());
        assert!(matches!(
            select_streams(&live, &SelectionPrefs::default()),
            Some(Selection::Hls(_))
        ));
        assert!(select_streams(&playable(vec![]), &SelectionPrefs::default()).is_none());
    }

    #[test]
    fn audio_mode_picks_audio_alone() {
        let p = playable(vec![
            stream(StreamKind::Video, Some(720), Some(VideoCodec::H264), 1),
            audio("en", true, 128_000),
        ]);
        let prefs = SelectionPrefs {
            audio_only: true,
            ..SelectionPrefs::default()
        };
        assert!(matches!(
            select_streams(&p, &prefs),
            Some(Selection::Single(s)) if s.kind == StreamKind::Audio
        ));
    }
}
