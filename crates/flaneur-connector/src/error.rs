// SPDX-License-Identifier: AGPL-3.0-or-later

/// Result of a connector operation.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Why a connector operation failed, in terms the UI can act on.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The connector does not implement this operation.
    #[error("not supported by this service")]
    Unsupported,
    /// The operation needs a session, or the session expired.
    #[error("sign-in required")]
    SignInRequired,
    /// The item, channel or playlist does not exist or was removed.
    #[error("not found")]
    NotFound,
    /// The item exists but cannot be played here (region, age, members only, private).
    #[error("unavailable: {0}")]
    Unavailable(String),
    /// The service is rate-limiting or challenging this client.
    #[error("rate limited")]
    RateLimited {
        /// Seconds to wait, when the service says.
        retry_after: Option<u32>,
    },
    /// The network failed.
    #[error("network error: {0}")]
    Network(String),
    /// The service answered something the connector does not understand (it probably changed).
    #[error("unexpected response: {0}")]
    Parse(String),
    /// Any other error the service reported.
    #[error("{0}")]
    Service(String),
}
