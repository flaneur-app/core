// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};

/// A duration or a position in a media, in whole seconds.
pub type Seconds = u32;

/// A point in time, in Unix seconds (UTC).
pub type Timestamp = i64;

/// When an item was published, and how precisely the service said so.
///
/// Services rarely give exact dates in listings: YouTube shows "3 days ago", feeds give a full
/// date. Keeping the precision lets the UI write "3 days ago" without pretending to know the hour,
/// and lets sorting treat approximate dates as such.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Published {
    /// Best estimate of the publication time.
    pub at: Timestamp,
    /// How far `at` can be from the real time.
    pub precision: Precision,
}

/// Precision of a [`Published`] time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Precision {
    /// The service gave a full timestamp.
    Exact,
    /// The service gave a date without a time.
    Day,
    /// The service gave a relative text ("3 weeks ago"), parsed into an estimate.
    Relative,
}
