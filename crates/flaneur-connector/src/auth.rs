// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;

use flaneur_model::{ServiceId, Thumbnail};
use serde::{Deserialize, Serialize};
use url::Url;

/// A way to sign in to a service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMethod {
    /// Paste or import the browser cookies of a signed-in session (Netscape `cookies.txt`).
    Cookies,
    /// Sign in inside an embedded web view; the app captures the cookies at the end.
    WebLogin {
        /// Page that starts the sign-in.
        start: Url,
    },
    /// Email (or user name) and password.
    Password,
    /// A sign-in link sent by email: [`Connector::send_sign_in_link`](crate::Connector::send_sign_in_link),
    /// then [`Connector::poll_sign_in_link`](crate::Connector::poll_sign_in_link) until the user
    /// opened it in any browser.
    EmailLink,
    /// OAuth 2.0 with PKCE, through the system browser.
    OAuth {
        /// Authorization endpoint.
        authorize: Url,
    },
}

/// What the user gave to sign in.
pub enum Credentials {
    /// Netscape `cookies.txt` content.
    Cookies(String),
    /// Email or user name, and password.
    Password {
        /// Email or user name.
        login: String,
        /// Password; never logged, never stored.
        password: String,
    },
    /// OAuth authorization code and PKCE verifier.
    OAuthCode {
        /// Authorization code.
        code: String,
        /// PKCE code verifier.
        verifier: String,
    },
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self {
            Self::Cookies(_) => "Cookies",
            Self::Password { .. } => "Password",
            Self::OAuthCode { .. } => "OAuthCode",
        };
        write!(f, "Credentials::{kind}(<redacted>)")
    }
}

/// A sign-in link sent and not opened yet.
#[derive(Clone)]
pub struct PendingSignIn {
    /// Opaque state the connector needs to poll; never logged.
    pub state: String,
    /// What to tell the user ("Open the link we sent to a…@example.org").
    pub message: Option<String>,
}

impl fmt::Debug for PendingSignIn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PendingSignIn")
            .field("state", &"<redacted>")
            .field("message", &self.message)
            .finish()
    }
}

/// The signed-in account, as shown in the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    /// Display name.
    pub name: String,
    /// Handle or email, when the service gives it.
    pub handle: Option<String>,
    /// Avatars.
    pub avatars: Vec<Thumbnail>,
}

/// A signed-in session.
///
/// `secret` is everything the connector needs to restore the session (cookies, tokens), in a
/// format only that connector reads. The app stores it in the OS keyring and nowhere else.
#[derive(Clone, Serialize, Deserialize)]
pub struct Session {
    /// The service the session belongs to.
    pub service: ServiceId,
    /// The account, when known.
    pub account: Option<Account>,
    /// Opaque secret; never logged.
    pub secret: String,
}

impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session")
            .field("service", &self.service)
            .field("account", &self.account)
            .field("secret", &"<redacted>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_never_contains_secrets() {
        let session = Session {
            service: ServiceId::new("youtube"),
            account: None,
            secret: "SAPISID=very-secret".into(),
        };
        assert!(!format!("{session:?}").contains("very-secret"));
        let creds = Credentials::Password {
            login: "me".into(),
            password: "hunter2".into(),
        };
        assert!(!format!("{creds:?}").contains("hunter2"));
    }
}
