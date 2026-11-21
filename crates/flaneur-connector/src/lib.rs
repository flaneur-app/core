// SPDX-License-Identifier: AGPL-3.0-or-later

//! What a Flâneur connector implements.
//!
//! A connector is one [`Connector`] value. Beyond the required [`Player`], it exposes optional
//! **facets** ([`Feeds`], [`Search`], [`Channels`], [`Library`], [`Subscriptions`], [`Reactions`],
//! [`Comments`]): a facet that returns `None` is not implemented at all. Within an implemented
//! facet, [`Capabilities`] says which operations work and under which conditions (anonymous,
//! signed in, or kept locally by Flâneur), so the UI can hide or disable actions before calling
//! them, and explain why.
//!
//! Connectors never store anything themselves: the session secret is handed to the app, which
//! keeps it in the OS keyring and gives it back with [`Connector::restore`].

mod auth;
mod capabilities;
mod error;
mod facets;

pub use auth::{Account, AuthMethod, Credentials, Session};
pub use capabilities::{Capabilities, Support};
pub use error::{Error, Result};
pub use facets::{
    ChannelTab, Channels, CommentSort, Comments, FeedKind, Feeds, Library, PlaybackEvent,
    PlaybackReport, Player, Reactions, Search, SearchFilters, SearchResult, Subscriptions,
};

use async_trait::async_trait;
use flaneur_model::{ChannelRef, ItemRef, PlaylistRef, ServiceId};
use url::Url;

/// Static description of a connector.
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    /// Stable id, stored in the database.
    pub id: ServiceId,
    /// Display name ("Odysee", "RSS & Atom").
    pub name: String,
    /// Ways to sign in, best first; empty when the service has no accounts.
    pub auth_methods: Vec<AuthMethod>,
}

/// What a URL points to, once a connector recognized it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A playable item.
    Item(ItemRef),
    /// A channel.
    Channel(ChannelRef),
    /// A playlist.
    Playlist(PlaylistRef),
}

/// One service, as Flâneur sees it.
#[async_trait]
pub trait Connector: Send + Sync {
    /// Static description.
    fn info(&self) -> &ServiceInfo;

    /// What works right now. Changes with the session: sign-in turns `SignedIn` operations on.
    fn capabilities(&self) -> Capabilities;

    /// Signs in and returns the session to keep in the keyring.
    async fn sign_in(&self, credentials: Credentials) -> Result<Session>;

    /// Restores a session kept from an earlier [`Connector::sign_in`].
    async fn restore(&self, session: &Session) -> Result<()>;

    /// Forgets the session. Does not revoke anything on the service.
    fn sign_out(&self);

    /// Recognizes a URL of this service (shared links, `open with`).
    fn parse_url(&self, url: &Url) -> Option<Target>;

    /// Stream resolution and related items: every connector can play what it lists.
    fn player(&self) -> &dyn Player;

    /// Home, subscription feed, trending.
    fn feeds(&self) -> Option<&dyn Feeds> {
        None
    }

    /// Search.
    fn search(&self) -> Option<&dyn Search> {
        None
    }

    /// Channel pages.
    fn channels(&self) -> Option<&dyn Channels> {
        None
    }

    /// History, watch later, playlists, playback reporting.
    fn library(&self) -> Option<&dyn Library> {
        None
    }

    /// Followed channels.
    fn subscriptions(&self) -> Option<&dyn Subscriptions> {
        None
    }

    /// Likes and dislikes.
    fn reactions(&self) -> Option<&dyn Reactions> {
        None
    }

    /// Comments.
    fn comments(&self) -> Option<&dyn Comments> {
        None
    }
}
