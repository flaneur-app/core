// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};

/// Opaque continuation token. Only the connector that produced it can read it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Cursor(pub String);

/// One page of a listing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page<T> {
    /// The items of this page, in service order.
    pub items: Vec<T>,
    /// Token for the next page; `None` at the end.
    pub next: Option<Cursor>,
    /// Total count across all pages, when the service gives it.
    pub total: Option<u64>,
}

impl<T> Page<T> {
    /// A last page with no total.
    #[must_use]
    pub fn last(items: Vec<T>) -> Self {
        Self {
            items,
            next: None,
            total: None,
        }
    }

    /// Maps the items, keeping the cursor and the total.
    #[must_use]
    pub fn map<U>(self, f: impl FnMut(T) -> U) -> Page<U> {
        Page {
            items: self.items.into_iter().map(f).collect(),
            next: self.next,
            total: self.total,
        }
    }
}
