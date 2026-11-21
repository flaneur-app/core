// SPDX-License-Identifier: AGPL-3.0-or-later

//! The connectors the app was built with.

use std::collections::BTreeMap;
use std::sync::Arc;

use flaneur_connector::Connector;
use flaneur_model::ServiceId;

/// The connectors available to the view-models, by service id.
#[derive(Clone, Default)]
pub struct Registry {
    connectors: BTreeMap<ServiceId, Arc<dyn Connector>>,
}

impl Registry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a connector; a connector with the same id is replaced.
    pub fn register(&mut self, connector: Arc<dyn Connector>) {
        self.connectors
            .insert(connector.info().id.clone(), connector);
    }

    /// The connector of a service.
    #[must_use]
    pub fn get(&self, service: &ServiceId) -> Option<&Arc<dyn Connector>> {
        self.connectors.get(service)
    }

    /// Every connector, ordered by id.
    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn Connector>> {
        self.connectors.values()
    }

    /// The connector that recognizes a URL, for shared links.
    #[must_use]
    pub fn resolve_url(&self, url: &str) -> Option<(ServiceId, flaneur_connector::Target)> {
        self.iter()
            .find_map(|c| c.parse_url(url).map(|t| (c.info().id.clone(), t)))
    }
}
