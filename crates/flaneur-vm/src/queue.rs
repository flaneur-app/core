// SPDX-License-Identifier: AGPL-3.0-or-later

//! The play queue: what plays now, what plays next, and autoplay.

use std::collections::VecDeque;

use flaneur_model::Item;

/// The play queue.
///
/// Items the user queued play first, in order. When they run out and autoplay is on, the
/// player view-model fills the queue with the first related item of the current one.
#[derive(Debug, Clone, Default)]
pub struct PlayQueue {
    history: Vec<Item>,
    current: Option<Item>,
    upcoming: VecDeque<Item>,
    /// Play a related item when the queue runs out.
    pub autoplay: bool,
}

impl PlayQueue {
    /// An empty queue with autoplay on.
    #[must_use]
    pub fn new() -> Self {
        Self {
            autoplay: true,
            ..Self::default()
        }
    }

    /// The item playing now.
    #[must_use]
    pub fn current(&self) -> Option<&Item> {
        self.current.as_ref()
    }

    /// The items queued after the current one.
    pub fn upcoming(&self) -> impl Iterator<Item = &Item> {
        self.upcoming.iter()
    }

    /// Plays an item now; the current one goes to the history, the queue is kept.
    pub fn play_now(&mut self, item: Item) {
        if let Some(previous) = self.current.replace(item) {
            self.history.push(previous);
        }
    }

    /// Queues an item right after the current one.
    pub fn play_next(&mut self, item: Item) {
        self.upcoming.push_front(item);
    }

    /// Queues an item at the end.
    pub fn enqueue(&mut self, item: Item) {
        self.upcoming.push_back(item);
    }

    /// Moves to the next queued item; `None` when the queue is empty, and then the caller
    /// decides on autoplay.
    pub fn advance(&mut self) -> Option<&Item> {
        let next = self.upcoming.pop_front()?;
        self.play_now(next);
        self.current.as_ref()
    }

    /// Goes back to the previous item; the current one becomes the next.
    pub fn back(&mut self) -> Option<&Item> {
        let previous = self.history.pop()?;
        if let Some(current) = self.current.replace(previous) {
            self.upcoming.push_front(current);
        }
        self.current.as_ref()
    }

    /// Picks the item autoplay should play from the related items: the first one not already
    /// played in this session.
    #[must_use]
    pub fn autoplay_candidate<'a>(&self, related: &'a [Item]) -> Option<&'a Item> {
        if !self.autoplay {
            return None;
        }
        related.iter().find(|candidate| {
            self.current
                .as_ref()
                .is_none_or(|c| c.r#ref != candidate.r#ref)
                && !self.history.iter().any(|h| h.r#ref == candidate.r#ref)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flaneur_model::{ContentFlags, ItemKind, ItemRef, ReactionCounts, ServiceId};

    fn item(id: &str) -> Item {
        Item {
            r#ref: ItemRef::new(ServiceId::new("test"), id),
            kind: ItemKind::Video,
            title: id.into(),
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
        }
    }

    fn id(item: Option<&Item>) -> Option<&str> {
        item.map(|i| i.r#ref.id.as_str())
    }

    #[test]
    fn play_next_goes_before_enqueued_items() {
        let mut q = PlayQueue::new();
        q.play_now(item("a"));
        q.enqueue(item("c"));
        q.play_next(item("b"));
        assert_eq!(id(q.advance()), Some("b"));
        assert_eq!(id(q.advance()), Some("c"));
        assert_eq!(id(q.advance()), None);
    }

    #[test]
    fn back_then_advance_returns_to_the_same_item() {
        let mut q = PlayQueue::new();
        q.play_now(item("a"));
        q.play_now(item("b"));
        assert_eq!(id(q.back()), Some("a"));
        assert_eq!(id(q.advance()), Some("b"));
    }

    #[test]
    fn autoplay_skips_items_already_played() {
        let mut q = PlayQueue::new();
        q.play_now(item("a"));
        q.play_now(item("b"));
        let related = [item("b"), item("a"), item("c")];
        assert_eq!(id(q.autoplay_candidate(&related)), Some("c"));
        q.autoplay = false;
        assert_eq!(id(q.autoplay_candidate(&related)), None);
    }
}
