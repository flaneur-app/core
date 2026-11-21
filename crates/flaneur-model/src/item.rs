// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use url::Url;

use crate::{ChannelSummary, ItemRef, PlaylistRef, Published, Seconds, Timestamp};

/// What an item is. Services expose these differently (a flag, a badge, a URL shape); the
/// connector decides, and the UI only reads this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemKind {
    /// A regular on-demand video.
    Video,
    /// A short vertical video, shown in a dedicated player.
    Short,
    /// A live stream in progress.
    Live,
    /// A scheduled premiere, played like a live once it starts.
    Premiere,
    /// A scheduled live or premiere that has not started yet.
    Upcoming,
}

/// A playable item as listings show it: enough for a card, a row or the top of the watch page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    /// Where the item lives.
    pub r#ref: ItemRef,
    /// What the item is.
    pub kind: ItemKind,
    /// Title as the service shows it.
    pub title: String,
    /// Full description, when the listing carries it.
    pub description: Option<String>,
    /// Duration; `None` for lives, and for listings that do not give it.
    pub duration: Option<Seconds>,
    /// Publication time.
    pub published: Option<Published>,
    /// Start time of an upcoming live or premiere.
    pub scheduled_start: Option<Timestamp>,
    /// Publisher of the item.
    pub channel: Option<ChannelSummary>,
    /// Thumbnails, any order; pick with [`Thumbnail::best_for_width`].
    pub thumbnails: Vec<Thumbnail>,
    /// Frame size of the video, when known: tells portrait videos apart for the UI.
    pub dimensions: Option<Dimensions>,
    /// View count, or current viewers for a live.
    pub views: Option<u64>,
    /// Public reaction counts.
    pub reactions: ReactionCounts,
    /// Sensitive-content and restriction markers.
    pub flags: ContentFlags,
    /// Page of the item on its service, to open in a browser or share.
    pub web_url: Option<Url>,
    /// Where the signed-in user stands with this item, when the service or the local history knows.
    pub watch: Option<WatchState>,
}

/// Width and height of a video frame, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dimensions {
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

impl Dimensions {
    /// Taller than wide.
    #[must_use]
    pub fn is_portrait(self) -> bool {
        self.height > self.width
    }
}

/// One thumbnail image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Thumbnail {
    /// Image URL.
    pub url: Url,
    /// Width in pixels, when known.
    pub width: Option<u32>,
    /// Height in pixels, when known.
    pub height: Option<u32>,
}

impl Thumbnail {
    /// The smallest thumbnail at least `width` pixels wide, or the widest one if none is.
    /// Thumbnails of unknown width rank below every known one.
    #[must_use]
    pub fn best_for_width(thumbnails: &[Self], width: u32) -> Option<&Self> {
        let wide_enough = thumbnails
            .iter()
            .filter(|t| t.width.is_some_and(|w| w >= width))
            .min_by_key(|t| t.width);
        wide_enough.or_else(|| thumbnails.iter().max_by_key(|t| t.width))
    }
}

/// Public reaction counts. Services that hide dislikes leave `dislikes` at `None`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactionCounts {
    /// Likes, or the service's positive reaction.
    pub likes: Option<u64>,
    /// Dislikes, or the service's negative reaction.
    pub dislikes: Option<u64>,
    /// Comment count.
    pub comments: Option<u64>,
}

/// The signed-in user's own reaction to an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reaction {
    /// No reaction.
    None,
    /// Liked.
    Like,
    /// Disliked.
    Dislike,
}

/// Sensitive-content markers. `None` everywhere means the service said nothing, not "safe".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentFlags {
    /// The service, the feed or a resolver marked the item as mature or sensitive.
    pub sensitive: Option<bool>,
    /// The service restricts the item by age (sign-in required to play it).
    pub age_restricted: Option<bool>,
    /// The item is for members or subscribers only.
    pub members_only: Option<bool>,
}

/// Playback progress of the signed-in user on one item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progress {
    /// Last position reached.
    pub position: Seconds,
    /// Duration the position relates to, when known.
    pub duration: Option<Seconds>,
    /// When this progress was recorded.
    pub updated_at: Timestamp,
}

/// What the signed-in user did with an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchState {
    /// Where playback stopped; `None` when only "watched" is known.
    pub progress: Option<Progress>,
    /// The item counts as watched; `None` when the service does not say.
    pub watched: Option<bool>,
    /// The user's reaction, when the service tells it.
    pub reaction: Option<Reaction>,
}

/// What a playlist is for. Lets the UI find the built-in lists without matching titles.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlaylistKind {
    /// A playlist a user made.
    #[default]
    Regular,
    /// The account's "watch later" list.
    WatchLater,
    /// The account's liked items.
    Liked,
}

/// A playlist, as listings show it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Playlist {
    /// Where the playlist lives.
    pub r#ref: PlaylistRef,
    /// Built-in list or user playlist.
    pub kind: PlaylistKind,
    /// Title.
    pub title: String,
    /// Owner, when it is a channel.
    pub channel: Option<ChannelSummary>,
    /// Number of items, when known.
    pub item_count: Option<u64>,
    /// Thumbnails.
    pub thumbnails: Vec<Thumbnail>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thumb(width: Option<u32>) -> Thumbnail {
        Thumbnail {
            url: Url::parse("https://example.org/t.jpg").unwrap(),
            width,
            height: None,
        }
    }

    #[test]
    fn picks_smallest_thumbnail_wide_enough() {
        let all = [
            thumb(Some(120)),
            thumb(Some(480)),
            thumb(Some(1280)),
            thumb(None),
        ];
        assert_eq!(
            Thumbnail::best_for_width(&all, 400).unwrap().width,
            Some(480)
        );
    }

    #[test]
    fn falls_back_to_widest_thumbnail() {
        let all = [thumb(Some(120)), thumb(Some(320))];
        assert_eq!(
            Thumbnail::best_for_width(&all, 1000).unwrap().width,
            Some(320)
        );
    }

    #[test]
    fn unknown_counts_stay_unknown_through_serde() {
        let counts = ReactionCounts {
            likes: Some(0),
            dislikes: None,
            comments: None,
        };
        let json = serde_json::to_string(&counts).unwrap();
        let back: ReactionCounts = serde_json::from_str(&json).unwrap();
        assert_eq!(back.likes, Some(0));
        assert_eq!(back.dislikes, None);
    }
}
