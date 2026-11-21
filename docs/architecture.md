# Flâneur core architecture

This document describes the structure of the shared core and the contract between the core, the
connectors and the native apps. It is the reference for structural changes: update it in the same
pull request as the code it describes.

## Goals, in order

1. **Speed.** Every screen shows something within one network round-trip; nothing blocks the UI
   thread; playback starts on the best stream the GPU can decode.
2. **Aggregation.** Several services side by side, and merged where that makes sense (home,
   subscriptions, history), without hiding where each item comes from.
3. **Account compatibility.** What the user does in Flâneur (watching, liking, following,
   commenting) lands in their account on the service, as if they had used the official site.

## Repositories

```text
flaneur-app/core                 this repository: model, connector traits, view-models, storage
flaneur-app/connector-youtube    implements the traits with RustyPipe (InnerTube)
flaneur-app/connector-odysee     implements the traits with the Odysee backend and Commentron
flaneur-app/connector-rss        implements the traits for RSS/Atom feeds, with page resolvers
flaneur-app/app-gnome            GTK4/libadwaita app, Flatpak; depends on all of the above
flaneur-app/app-windows          later: WinUI 3 app in C#, over the core through UniFFI
```

Connectors depend on the core, never on each other and never on an app. Apps pick the connectors
they ship at build time; there is no plugin loading at runtime.

## Crates of this repository

```text
flaneur-model       pure data: Item, Channel, Comment, Playable, Page... (serde only)
      ▲
flaneur-connector   traits: Connector + facets, Capabilities, Session, Error
      ▲
flaneur-store       SQLite: history and resume positions, local subscriptions, settings, outbox
      ▲
flaneur-vm          view-models: lists, play queue, stream selection, aggregation, registry
      ▲
  apps (gtk4-rs directly on GNOME; UniFFI bindings for Windows later)
```

`flaneur-model` and `flaneur-connector` are all a connector needs: no SQLite, no tokio runtime,
no UI. `flaneur-store` and `flaneur-vm` are what apps need on top.

## The model (`flaneur-model`)

Two rules apply to every type:

- **Absent is not false.** A value the service did not give is `None`, never `0`, `false` or `""`.
  `views: None` is "unknown", `views: Some(0)` is "nobody". Booleans that services often omit are
  `Option<bool>`.
- **Fixed units.** Durations and positions in whole seconds (`Seconds = u32`), timestamps in Unix
  seconds UTC, bitrates in bit/s, sizes in bytes.

Key types:

- `ItemRef { service, id }`, `ChannelRef`, `PlaylistRef`: the service's own id, opaque to everyone
  but its connector, namespaced by `ServiceId`. Cross-service deduplication (the same video on
  YouTube and Odysee) is possible later with an extra `canonical` key; nothing depends on it now.
- `Item` with `kind: ItemKind` (`Video`, `Short`, `Live`, `Premiere`, `Upcoming`). The connector
  decides the kind; the UI picks the player and the card from it.
- `Published { at, precision }`: listings often only say "3 days ago"; `precision` keeps the UI and
  the sorting honest about it.
- `ContentFlags { sensitive, age_restricted, members_only }`: sensitive-content marking, filled by
  services, feed metadata or page resolvers.
- `WatchState` and `Progress`: the user's own state for an item (resume position, watched,
  reaction).
- `Playable`: streams, HLS/DASH manifests, expiry, start position, and an opaque `tracking`
  token the connector needs to report playback (YouTube's playback tracking URLs). Resolved right
  before playback, never stored: stream URLs expire and can be tied to the IP that resolved them.

## Connectors (`flaneur-connector`)

A connector is one value implementing `Connector`:

- **Required:** `info()` (id, name, sign-in methods), `capabilities()`, `sign_in` / `restore` /
  `sign_out`, `parse_url` (shared links), and `player()` (stream resolution and related items):
  every connector plays what it lists.
- **Optional facets**, `None` by default: `feeds()` (home, subscription feed, trending, shorts),
  `search()`, `channels()`, `library()` (history read and playback reporting, playlists, watch
  later), `subscriptions()`, `reactions()`, `comments()`.

Facets say what is *implemented*; `Capabilities` says what *works now*, operation by operation,
as a `Support`:

| `Support` | Meaning | Example |
| --- | --- | --- |
| `Unsupported` | The service cannot do it | dislikes on RSS |
| `Local` | Flâneur does it alone, in its database; the service never knows | RSS subscriptions and history |
| `Anonymous` | Works without an account | YouTube search |
| `SignedIn` | Needs a session, and then syncs with the account | YouTube history, likes |

Reads and writes are separate entries (`history_read` / `history_write`, `comments_read` /
`comments_write`...): a service may let a client read the history but not write it. The UI hides
what is `Unsupported`, disables what needs a session with a "sign in to..." hint, and never calls
an operation it was told does not work.

Errors are an enum the UI can act on: `SignInRequired`, `RateLimited { retry_after }`,
`Unavailable(reason)`, `NotFound`, `Network`, `Parse` (the service changed: report a bug),
`Service`.

Async: facets use `async-trait` so connectors stay object-safe (`Arc<dyn Connector>`). Connectors
use their own HTTP client and must not assume a particular executor beyond tokio's reactor being
available (reqwest/RustyPipe need it).

## Accounts and secrets

- A connector never stores anything. `sign_in` returns a `Session { service, account, secret }`;
  the app stores `secret` in the OS keyring (libsecret / the Secret portal on GNOME, Credential
  Manager on Windows) and hands it back with `restore` at start-up.
- `Session` and `Credentials` redact their secrets in `Debug`, so they cannot leak into logs.
- No secret ever reaches the SQLite file, a log, a crash report, or any host but the service it
  belongs to. There is no Flâneur server.
- A future OAuth helper (brig·id) may manage *official API* OAuth tokens. Browser cookies of a
  Google session never leave the machine.

## Storage (`flaneur-store`)

One SQLite file (bundled SQLite, WAL), migrated with `PRAGMA user_version`:

- `history`: last known position, duration and watched state of every played item, whatever the
  service. For services that sync history it is a cache that makes resume instant and offline; for
  local services it is the history. An older record never overwrites a newer one, so a late sync
  never moves the resume position back. "Watched" sticks once set.
- `local_subscriptions`: channels followed in Flâneur itself (RSS feeds).
- `settings`: JSON values by key.
- `outbox`: writes to a service that failed (offline, rate-limited), retried oldest first. A like
  or a history report made offline reaches the service later.

## View-models (`flaneur-vm`)

Everything a screen needs except drawing it. A view-model owns its state, publishes it on a
`tokio::sync::watch` channel the UI observes, and exposes commands as methods.

- `Paginator<T>`: infinite lists over any `Page<T>` source, with loading/error/end state;
  concurrent "load more" calls collapse into one.
- `PlayQueue`: current item, "play next", "add to queue", back/forward, and the autoplay pick from
  related items (never an item already played in the session).
- `select_streams`: picks what to play. **Hardware-decodable codecs rank first**, then resolution
  (capped by the user's maximum), then the user's codec order, fps and bitrate; audio by language,
  then original track, then bitrate; falls back to muxed streams, then HLS. The audio mode picks
  audio alone. This is where most of the "fast on modest PCs" comes from: a codec the GPU cannot
  decode falls back to the CPU and stutters.
- `interleave`: merges ranked feeds of several services (home) round-robin with weights, keeping
  each service's own ranking and skipping duplicates. Chronological feeds (subscriptions) merge by
  date instead.
- `Registry`: the connectors the app was built with, and URL recognition across them.

Next view-models, built on these: `HomeVm`, `SubscriptionsVm`, `SearchVm`, `ChannelVm`, `WatchVm`
(details, streams, comments, related, reactions, playback reporting at start, every 30 s, on pause
and at the end), `HistoryVm`, `AccountsVm`, `SettingsVm`.

## Threading

- The core owns a multi-thread tokio runtime for network and SQLite work.
- On GNOME, the app calls the view-models directly in Rust (gtk4-rs, no FFI). It observes each
  `watch::Receiver` from the GTK main loop (`glib::MainContext::spawn_local` awaiting
  `changed()`), so widgets are only ever touched on the main thread and never wait on the network.
- On Windows, UniFFI exposes the view-models with async methods and callback interfaces; the
  `watch` channels map to a listener per view-model.

## Licensing

Everything in `flaneur-app` is AGPL-3.0-or-later. Some dependencies are GPL-3.0: RustyPipe
(GPL-3.0-only) in `connector-youtube`, and Pipeline's code (GPL-3.0-or-later) if `app-gnome`
reuses it. Section 13 of both the GPL-3.0 and the AGPL-3.0 allows combining them; a binary that
includes them is distributed under the terms of both. The core itself depends on none of them.

## Not now

- Audio mode beyond `audio_only` stream selection.
- Cross-service deduplication.
- Any declarative query layer: the view-models call connectors directly.
