// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;

use serde::{Deserialize, Serialize};

/// Stable identifier of a connector, such as `youtube`, `odysee` or `rss`.
///
/// It is stored in the local database, so a connector never changes its id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ServiceId(pub String);

impl ServiceId {
    /// Builds a service id from a static name.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl fmt::Display for ServiceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

macro_rules! service_ref {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name {
            /// The service the object lives on.
            pub service: ServiceId,
            /// The service's own id, opaque to everything but its connector.
            pub id: String,
        }

        impl $name {
            /// Builds a reference from a service and the service's own id.
            #[must_use]
            pub fn new(service: ServiceId, id: impl Into<String>) -> Self {
                Self { service, id: id.into() }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}:{}", self.service, self.id)
            }
        }
    };
}

service_ref!(
    /// Reference to a playable item (video, short, live...) on one service.
    ItemRef
);
service_ref!(
    /// Reference to a channel (or feed, for RSS) on one service.
    ChannelRef
);
service_ref!(
    /// Reference to a playlist on one service; "watch later" is a playlist too.
    PlaylistRef
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_as_service_colon_id() {
        let r = ItemRef::new(ServiceId::new("youtube"), "dQw4w9WgXcQ");
        assert_eq!(r.to_string(), "youtube:dQw4w9WgXcQ");
    }
}
