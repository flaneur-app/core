// SPDX-License-Identifier: AGPL-3.0-or-later

//! The aggregated home: one feed made of the feeds of every signed-in service.

use flaneur_model::{Item, ServiceId};

/// The items one service contributed to an aggregated feed, in its own order.
#[derive(Debug, Clone)]
pub struct FeedSource {
    /// The service.
    pub service: ServiceId,
    /// Its items, best first, as the service ranked them.
    pub items: Vec<Item>,
    /// Relative share of this service in the result (1 for equal shares).
    pub weight: u32,
}

/// Merges ranked feeds into one, keeping each service's own ranking.
///
/// Recommendation feeds cannot be sorted by date: each service already ranked them. The merge
/// takes items round-robin, `weight` items from a source per round, so a service with more
/// content does not drown the others and each one's best items stay near the top. Items whose
/// reference already appeared are skipped.
#[must_use]
pub fn interleave(sources: Vec<FeedSource>) -> Vec<Item> {
    let mut seen = std::collections::HashSet::new();
    let mut iters: Vec<_> = sources
        .into_iter()
        .map(|s| (s.weight.max(1), s.items.into_iter()))
        .collect();
    let mut out = Vec::new();
    loop {
        let mut progressed = false;
        for (weight, items) in &mut iters {
            for _ in 0..*weight {
                let Some(item) = items.next() else { break };
                progressed = true;
                if seen.insert(item.r#ref.clone()) {
                    out.push(item);
                }
            }
        }
        if !progressed {
            return out;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flaneur_model::{ContentFlags, ItemKind, ItemRef, ReactionCounts};

    fn item(service: &str, id: &str) -> Item {
        Item {
            r#ref: ItemRef::new(ServiceId::new(service), id),
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

    fn source(service: &str, ids: &[&str], weight: u32) -> FeedSource {
        FeedSource {
            service: ServiceId::new(service),
            items: ids.iter().map(|id| item(service, id)).collect(),
            weight,
        }
    }

    #[test]
    fn interleaves_by_weight_and_drains_the_longest_source() {
        let merged = interleave(vec![
            source("yt", &["y1", "y2", "y3", "y4", "y5"], 2),
            source("od", &["o1", "o2"], 1),
        ]);
        let ids: Vec<_> = merged.iter().map(|i| i.r#ref.id.as_str()).collect();
        assert_eq!(ids, ["y1", "y2", "o1", "y3", "y4", "o2", "y5"]);
    }

    #[test]
    fn skips_duplicates() {
        let merged = interleave(vec![source("yt", &["a", "a", "b"], 1)]);
        assert_eq!(merged.len(), 2);
    }
}
