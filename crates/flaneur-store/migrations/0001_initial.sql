-- SPDX-License-Identifier: AGPL-3.0-or-later
-- Local state of Flâneur. Secrets (cookies, tokens) never go here: they live in the OS keyring.

CREATE TABLE settings (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL -- JSON
) STRICT;

-- Last known state of every item the user played, whatever the service. For services that sync
-- history (YouTube, Odysee) it is a cache that makes resume instant and works offline; for local
-- services (RSS) it is the only history.
CREATE TABLE history (
    service    TEXT NOT NULL,
    item_id    TEXT NOT NULL,
    item       TEXT,              -- JSON flaneur_model::Item, for listing the history offline
    position   INTEGER NOT NULL,  -- seconds
    duration   INTEGER,           -- seconds, NULL when unknown
    watched    INTEGER NOT NULL,  -- 0 or 1
    updated_at INTEGER NOT NULL,  -- Unix seconds
    PRIMARY KEY (service, item_id)
) STRICT;
CREATE INDEX history_by_date ON history (updated_at DESC);

-- Channels followed in Flâneur itself, for services without accounts (RSS feeds).
CREATE TABLE local_subscriptions (
    service    TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    channel    TEXT NOT NULL,     -- JSON flaneur_model::ChannelSummary
    added_at   INTEGER NOT NULL,
    PRIMARY KEY (service, channel_id)
) STRICT;

-- Writes to a service that failed (offline, rate limited) and will be retried, oldest first.
CREATE TABLE outbox (
    id         INTEGER PRIMARY KEY,
    service    TEXT NOT NULL,
    operation  TEXT NOT NULL,     -- JSON, defined by the view-model that queues it
    created_at INTEGER NOT NULL,
    attempts   INTEGER NOT NULL DEFAULT 0
) STRICT;
