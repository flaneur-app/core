// SPDX-License-Identifier: AGPL-3.0-or-later

//! Optional facets of a connector. Each method maps to one [`Capabilities`](crate::Capabilities)
//! entry; calling an operation whose support is `Unsupported` returns
//! [`Error::Unsupported`](crate::Error::Unsupported).

use async_trait::async_trait;
use flaneur_model::{
    Channel, ChannelRef, ChannelSummary, Comment, Cursor, Item, ItemRef, Page, Playable, Playlist,
    PlaylistRef, Reaction, Seconds,
};

use crate::Result;

/// Stream resolution and related items.
#[async_trait]
pub trait Player: Send + Sync {
    /// Resolves the streams and full details of an item, right before playback.
    async fn resolve(&self, item: &ItemRef) -> Result<Playable>;

    /// Items to play next (autoplay) or to show beside the player.
    async fn related(&self, item: &ItemRef, cursor: Option<&Cursor>) -> Result<Page<Item>>;
}

/// The feeds a service offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeedKind {
    /// Personalized when signed in, editorial otherwise.
    Home,
    /// Latest items of the followed channels.
    Subscriptions,
    /// Trending or popular.
    Trending,
    /// Short vertical videos.
    Shorts,
}

/// Home, subscription feed, trending.
#[async_trait]
pub trait Feeds: Send + Sync {
    /// One page of a feed.
    async fn feed(&self, kind: FeedKind, cursor: Option<&Cursor>) -> Result<Page<Item>>;
}

/// A search hit.
#[derive(Debug, Clone, PartialEq)]
pub enum SearchResult {
    /// A playable item.
    Item(Box<Item>),
    /// A channel.
    Channel(ChannelSummary),
    /// A playlist.
    Playlist(Playlist),
}

/// Search filters; `None` means "the service's default".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchFilters {
    /// Only items of this kind.
    pub kind: Option<flaneur_model::ItemKind>,
    /// Only items published after this time.
    pub published_after: Option<flaneur_model::Timestamp>,
    /// Exclude items flagged sensitive.
    pub safe: Option<bool>,
}

/// Search.
#[async_trait]
pub trait Search: Send + Sync {
    /// One page of results.
    async fn search(
        &self,
        query: &str,
        filters: &SearchFilters,
        cursor: Option<&Cursor>,
    ) -> Result<Page<SearchResult>>;

    /// Query completions while typing. Empty when the service has none.
    async fn suggestions(&self, prefix: &str) -> Result<Vec<String>> {
        let _ = prefix;
        Ok(Vec::new())
    }
}

/// Tabs of a channel page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelTab {
    /// On-demand videos, newest first.
    Videos,
    /// Shorts.
    Shorts,
    /// Lives, past and upcoming.
    Lives,
    /// The channel's playlists, as items of a page of their own (see [`Channels::playlists`]).
    Playlists,
}

/// Channel pages.
#[async_trait]
pub trait Channels: Send + Sync {
    /// Header of the channel page.
    async fn channel(&self, channel: &ChannelRef) -> Result<Channel>;

    /// One page of a tab of items.
    async fn items(
        &self,
        channel: &ChannelRef,
        tab: ChannelTab,
        cursor: Option<&Cursor>,
    ) -> Result<Page<Item>>;

    /// One page of the channel's playlists.
    async fn playlists(
        &self,
        channel: &ChannelRef,
        cursor: Option<&Cursor>,
    ) -> Result<Page<Playlist>>;
}

/// What happened in the player, for the service's history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackEvent {
    /// Playback started (counts the view).
    Started,
    /// Periodic progress while playing.
    Progress,
    /// Paused or closed before the end.
    Stopped,
    /// Reached the end.
    Ended,
}

/// A playback report, sent by the player view-model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackReport {
    /// The item being played.
    pub item: ItemRef,
    /// What happened.
    pub event: PlaybackEvent,
    /// Current position.
    pub position: Seconds,
    /// Seconds actually watched since the previous report (seeks excluded).
    pub watched_since_last: Seconds,
    /// The `tracking` token of the [`Playable`].
    pub tracking: Option<String>,
}

/// History, watch later, playlists.
#[async_trait]
pub trait Library: Send + Sync {
    /// One page of the watch history, most recent first.
    async fn history(&self, cursor: Option<&Cursor>) -> Result<Page<Item>>;

    /// Reports playback, so the service's history and resume position follow.
    async fn report_playback(&self, report: &PlaybackReport) -> Result<()>;

    /// The signed-in user's playlists, watch later included.
    async fn playlists(&self, cursor: Option<&Cursor>) -> Result<Page<Playlist>>;

    /// The watch later playlist, when the service has one.
    fn watch_later(&self) -> Option<PlaylistRef> {
        None
    }

    /// One page of a playlist.
    async fn playlist_items(
        &self,
        playlist: &PlaylistRef,
        cursor: Option<&Cursor>,
    ) -> Result<Page<Item>>;

    /// Adds an item to a playlist.
    async fn add_to_playlist(&self, playlist: &PlaylistRef, item: &ItemRef) -> Result<()>;

    /// Removes an item from a playlist.
    async fn remove_from_playlist(&self, playlist: &PlaylistRef, item: &ItemRef) -> Result<()>;
}

/// Followed channels.
#[async_trait]
pub trait Subscriptions: Send + Sync {
    /// One page of the followed channels.
    async fn list(&self, cursor: Option<&Cursor>) -> Result<Page<ChannelSummary>>;

    /// Follows or unfollows a channel.
    async fn set_subscribed(&self, channel: &ChannelRef, subscribed: bool) -> Result<()>;
}

/// Likes and dislikes.
#[async_trait]
pub trait Reactions: Send + Sync {
    /// Sets the signed-in user's reaction; [`Reaction::None`] removes it.
    async fn react(&self, item: &ItemRef, reaction: Reaction) -> Result<()>;
}

/// Comment ordering.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CommentSort {
    /// The service's default ("top").
    #[default]
    Top,
    /// Newest first.
    Newest,
}

/// Comments.
#[async_trait]
pub trait Comments: Send + Sync {
    /// One page of the top-level comments of an item.
    async fn comments(
        &self,
        item: &ItemRef,
        sort: CommentSort,
        cursor: Option<&Cursor>,
    ) -> Result<Page<Comment>>;

    /// One page of the replies to a comment, from its `replies_token`.
    async fn replies(
        &self,
        item: &ItemRef,
        replies_token: &str,
        cursor: Option<&Cursor>,
    ) -> Result<Page<Comment>>;

    /// Posts a comment, or a reply when `reply_to` is the id of a comment.
    async fn post(&self, item: &ItemRef, text: &str, reply_to: Option<&str>) -> Result<Comment>;
}
