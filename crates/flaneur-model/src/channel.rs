// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use url::Url;

use crate::{ChannelRef, Thumbnail};

/// The few channel fields an item card shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelSummary {
    /// Where the channel lives.
    pub r#ref: ChannelRef,
    /// Display name.
    pub name: String,
    /// Avatars.
    pub avatars: Vec<Thumbnail>,
    /// The service marks the channel as verified.
    pub verified: Option<bool>,
}

/// A channel page header. Its items are listed separately, one tab at a time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Channel {
    /// Name, avatars, verification.
    pub summary: ChannelSummary,
    /// Handle, such as `@name`.
    pub handle: Option<String>,
    /// About text.
    pub description: Option<String>,
    /// Subscriber or follower count.
    pub subscribers: Option<u64>,
    /// Banner images.
    pub banners: Vec<Thumbnail>,
    /// The signed-in user follows this channel.
    pub subscribed: Option<bool>,
    /// Page of the channel on its service.
    pub web_url: Option<Url>,
}
