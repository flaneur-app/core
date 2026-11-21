// SPDX-License-Identifier: AGPL-3.0-or-later

//! View-models of Flâneur: everything a screen needs except drawing it.
//!
//! Each native app (GTK on GNOME, `WinUI` on Windows) only binds widgets to these types. A
//! view-model owns its state, publishes it through a [`tokio::sync::watch`] channel the UI
//! observes, and exposes commands as methods. Nothing here depends on a UI toolkit, so the same
//! logic runs, and is tested, everywhere.

mod feed;
mod paginator;
mod queue;
mod registry;
mod selector;

pub use feed::{FeedSource, interleave};
pub use paginator::{ListState, Paginator};
pub use queue::PlayQueue;
pub use registry::Registry;
pub use selector::{Selection, SelectionPrefs, select_streams};
