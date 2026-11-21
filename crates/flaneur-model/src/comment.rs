// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};

use crate::{ChannelRef, Published, Reaction, Thumbnail};

/// A comment, or a reply to one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    /// The service's own id of the comment.
    pub id: String,
    /// Who wrote it.
    pub author: CommentAuthor,
    /// Plain text; links and timestamps stay as text, the UI linkifies them.
    pub text: String,
    /// When it was posted.
    pub published: Option<Published>,
    /// Likes.
    pub likes: Option<u64>,
    /// Number of replies; fetch them with the connector's `replies`.
    pub reply_count: Option<u64>,
    /// Opaque token the connector needs to list the replies.
    pub replies_token: Option<String>,
    /// Pinned by the item's publisher.
    pub pinned: bool,
    /// Liked by the item's publisher.
    pub hearted: bool,
    /// The signed-in user's reaction.
    pub my_reaction: Option<Reaction>,
}

/// Author of a comment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentAuthor {
    /// Display name.
    pub name: String,
    /// The author's channel, when they have one.
    pub channel: Option<ChannelRef>,
    /// Avatars.
    pub avatars: Vec<Thumbnail>,
    /// The author is the item's publisher.
    pub is_publisher: bool,
}
