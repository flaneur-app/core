// SPDX-License-Identifier: AGPL-3.0-or-later

//! Infinite lists: load the first page, load more on scroll, refresh.

use std::future::Future;
use std::sync::Arc;

use flaneur_connector::Error;
use flaneur_model::{Cursor, Page};
use tokio::sync::{Mutex, watch};

/// State of a paginated list, as the UI draws it.
#[derive(Debug, Clone)]
pub struct ListState<T> {
    /// Items loaded so far.
    pub items: Arc<Vec<T>>,
    /// A page is loading.
    pub loading: bool,
    /// The last load failed; the UI shows it with a retry button.
    pub error: Option<String>,
    /// More pages remain.
    pub has_more: bool,
}

impl<T> Default for ListState<T> {
    fn default() -> Self {
        Self {
            items: Arc::new(Vec::new()),
            loading: false,
            error: None,
            has_more: true,
        }
    }
}

/// Which page loads next.
#[derive(Debug, Clone)]
enum Next {
    /// The first page.
    First,
    /// The page after this cursor.
    After(Cursor),
    /// None: the list is complete.
    End,
}

/// A paginated list over any page source.
///
/// `fetch` gets the cursor of the next page (`None` for the first one). Concurrent
/// `load_more` calls collapse into one: scrolling fast never loads the same page twice.
pub struct Paginator<T, F> {
    fetch: F,
    next: Mutex<Next>,
    state: watch::Sender<ListState<T>>,
}

impl<T, F, Fut> Paginator<T, F>
where
    T: Clone + Send + Sync,
    F: Fn(Option<Cursor>) -> Fut + Send + Sync,
    Fut: Future<Output = Result<Page<T>, Error>> + Send,
{
    /// A list that loads nothing until [`Paginator::load_more`].
    pub fn new(fetch: F) -> Self {
        Self {
            fetch,
            next: Mutex::new(Next::First),
            state: watch::Sender::new(ListState::default()),
        }
    }

    /// Observes the state.
    pub fn subscribe(&self) -> watch::Receiver<ListState<T>> {
        self.state.subscribe()
    }

    /// The current state.
    pub fn state(&self) -> ListState<T> {
        self.state.borrow().clone()
    }

    /// Loads the next page, if any and if none is loading.
    pub async fn load_more(&self) {
        let Ok(mut next) = self.next.try_lock() else {
            return; // a load is already running
        };
        let cursor = match &*next {
            Next::First => None,
            Next::After(cursor) => Some(cursor.clone()),
            Next::End => return,
        };
        self.state.send_modify(|s| {
            s.loading = true;
            s.error = None;
        });
        match (self.fetch)(cursor).await {
            Ok(page) => {
                let has_more = page.next.is_some();
                *next = page.next.map_or(Next::End, Next::After);
                self.state.send_modify(|s| {
                    let mut items = Vec::clone(&s.items);
                    items.extend(page.items);
                    s.items = Arc::new(items);
                    s.loading = false;
                    s.has_more = has_more;
                });
            }
            Err(error) => {
                tracing::warn!(%error, "page load failed");
                self.state.send_modify(|s| {
                    s.loading = false;
                    s.error = Some(error.to_string());
                });
            }
        }
    }

    /// Drops everything and loads the first page again.
    pub async fn refresh(&self) {
        {
            let mut next = self.next.lock().await;
            *next = Next::First;
            self.state.send_replace(ListState::default());
        }
        self.load_more().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn numbers(cursor: Option<Cursor>) -> Result<Page<u32>, Error> {
        let start: u32 = cursor.map_or(0, |c| c.0.parse().unwrap());
        if start == 4 {
            return Err(Error::Network("offline".into()));
        }
        let next = (start + 2 < 6).then(|| Cursor((start + 2).to_string()));
        Ok(Page {
            items: vec![start, start + 1],
            next,
            total: None,
        })
    }

    #[tokio::test]
    async fn loads_pages_until_the_end_and_reports_errors() {
        let list = Paginator::new(numbers);
        list.load_more().await;
        list.load_more().await;
        assert_eq!(*list.state().items, [0, 1, 2, 3]);
        assert!(list.state().has_more);

        list.load_more().await; // page 4 fails
        let state = list.state();
        assert_eq!(state.error.as_deref(), Some("network error: offline"));
        assert_eq!(state.items.len(), 4);

        list.refresh().await;
        assert_eq!(*list.state().items, [0, 1]);
    }
}
