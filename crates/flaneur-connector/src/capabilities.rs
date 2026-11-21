// SPDX-License-Identifier: AGPL-3.0-or-later

/// Whether, and how, one operation works.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Support {
    /// The service or the connector cannot do it.
    #[default]
    Unsupported,
    /// Flâneur does it on its own, in the local database: the service knows nothing of it
    /// (history and subscriptions of RSS feeds, for instance). The connector does not implement
    /// the operation: the view-models do, with the store. A `Local` subscription feed is the
    /// merge, by date, of the channel items of every locally followed channel.
    Local,
    /// Works without an account.
    Anonymous,
    /// Works once signed in, and is then synced with the service account.
    SignedIn,
}

impl Support {
    /// The operation can run now, given whether a session is active.
    #[must_use]
    pub fn available(self, signed_in: bool) -> bool {
        match self {
            Self::Unsupported => false,
            Self::Local | Self::Anonymous => true,
            Self::SignedIn => signed_in,
        }
    }
}

/// Every operation the UI may offer, one [`Support`] each.
///
/// Reads and writes are separate: a service may show its history but not accept new entries
/// from a third-party client.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Capabilities {
    /// A session is active.
    pub signed_in: bool,

    /// Personalized or editorial home.
    pub home: Support,
    /// Chronological feed of the followed channels.
    pub subscription_feed: Support,
    /// Trending or popular items.
    pub trending: Support,
    /// Short vertical videos feed.
    pub shorts: Support,

    /// Search.
    pub search: Support,
    /// Channel pages.
    pub channels: Support,
    /// Related items, for autoplay.
    pub related: Support,

    /// Read the watch history.
    pub history_read: Support,
    /// Report playback (watched state, resume position) to the history.
    pub history_write: Support,
    /// Open any public playlist by its reference.
    pub public_playlists: Support,
    /// List the account's own playlists, watch later and liked items.
    pub playlists_read: Support,
    /// Add to and remove from the account's playlists and watch later.
    pub playlists_write: Support,

    /// List the followed channels.
    pub subscriptions_read: Support,
    /// Follow and unfollow channels.
    pub subscriptions_write: Support,

    /// Like and dislike.
    pub reactions_write: Support,
    /// Read comments.
    pub comments_read: Support,
    /// Post top-level comments.
    pub comments_write: Support,
    /// Reply to comments.
    pub comment_replies_write: Support,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_in_operations_need_a_session() {
        assert!(!Support::SignedIn.available(false));
        assert!(Support::SignedIn.available(true));
        assert!(Support::Local.available(false));
        assert!(!Support::Unsupported.available(true));
    }
}
