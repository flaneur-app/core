// SPDX-License-Identifier: AGPL-3.0-or-later

//! The normalized model of Flâneur.
//!
//! Every connector maps its service's data into these types, and every view-model and app only
//! ever sees these types. Two rules hold throughout:
//!
//! - **Absent is not false.** A value the service did not give is `None`, never `0`, `false` or an
//!   empty string. A view count of `None` means "unknown", `Some(0)` means "nobody watched it".
//! - **Units are fixed.** Durations and positions are whole seconds, timestamps are Unix seconds
//!   (UTC), sizes are bytes, bitrates are bits per second.

mod channel;
mod comment;
mod ids;
mod item;
mod page;
mod stream;
mod time;

pub use channel::{Channel, ChannelSummary};
pub use comment::{Comment, CommentAuthor};
pub use ids::{ChannelRef, ItemRef, PlaylistRef, ServiceId};
pub use item::{
    ContentFlags, Dimensions, Item, ItemKind, Playlist, PlaylistKind, Progress, Reaction,
    ReactionCounts, Thumbnail, WatchState,
};
pub use page::{Cursor, Page};
pub use stream::{AudioTrack, Container, Playable, Stream, StreamKind, VideoCodec};
pub use time::{Precision, Published, Seconds, Timestamp};
