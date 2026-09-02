# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

`SPECS.md` is the design contract (event envelope, data model, pipeline). Read it
before changing anything the bus carries.

## Commands

```sh
cargo run                    # start the watcher — reads ./watchlist.yaml
cargo build
cargo clippy --all-targets   # keep clean; CI-equivalent gate
cargo fmt
```

No test suite yet.

- `NO_COLOR=1` or a non-TTY stdout disables ANSI colour and the startup banner.
- `DEBUG=true` makes the logger print the raw envelope as one JSON object per
  line instead of human text, e.g. `DEBUG=true cargo run | jq`.

## Big picture

One Tokio process, one in-process broadcast bus of `Arc<Envelope>`. Every
subsystem is a **stage**: it subscribes to the whole stream, and some also emit.
The pipeline is `listener → tracker → opportunist → trader`, plus passive stages
(logger now; dashboard / jsonl / alerts later). Enrichment is additive — a stage
never mutates an event, it publishes a new one linked by `trace_id`.

```
watchlist.yaml → listener → bus → logger (+ future stages)
```

### Lifecycle (`main`)

- `main` builds one `Bus` and one `CancellationToken`, held for the whole process.
- Each stage is a task spawned via `stages::drive(stage, bus.subscribe(), shutdown)`
  (or, for the listener, `stages::listener::run(...)`).
- `Ctrl-C` → `shutdown.cancel()` → every stage's `select!` loop exits → `main`
  joins and exits. Shutdown is explicit; it does **not** depend on Senders
  dropping, so stages may hold a `Bus` and emit freely.

### Producer side — `stages/listener.rs` + `listener_sub.rs`

- One `SubListener` task per chain; inside it one **reconnecting** subscription
  per `Target` (capped exponential backoff, `Resubscribed` / `WatchStopped`
  events) plus a 30 s `Alive` heartbeat.
- Per log: `chain::contracts::decode` dispatches by `topic0`.
  - match → spawn a **detached** `chain::enrich::enrich` task that does one round
    of concurrent view calls (token facts, curve getters, block timestamp) under
    a time budget, builds the `Detection`, and emits it. The stream reader never
    blocks on enrichment.
  - no match → `Event::Undecoded`.
- Never panics — a wrong ABI or a failed call degrades to `Undecoded` / a `null`
  field, so ABIs are safe to iterate against live data.

### Consuming the bus — `stages/`

- `Stage` trait: `async fn on_event(&mut self, &Envelope)` + optional `on_lag`.
- `stages::drive(stage, rx, shutdown)` pumps a `Receiver` until shutdown or the
  channel closes; `RecvError::Lagged` → `on_lag`.
- `logger` is the only stage so far and the only code that writes stdout.

## Adding a stage

1. Implement `Stage`. `on_event` gets every `Envelope` by shared ref; match
   `env.payload` or filter on `env.topic`.
2. To emit, take a `Bus` in the constructor and call `bus.emit(event)` /
   `bus.emit_caused(event, caused_by)`. Add the payload as a new `events::Event`
   variant (see normalization rules below).
3. In `main`, `tasks.spawn(stages::drive(MyStage::new(bus.clone()), bus.subscribe(), shutdown.clone()))`.
   **Subscribe before the first `bus.emit`** or early events are missed.
4. A `broadcast::Receiver` only delivers from subscribe time — a stage that needs
   history keeps its own ring buffer.
5. Slow stages lag rather than block producers (buffer `bus::CAPACITY`);
   implement `on_lag` if a gap matters.

## Message normalization (SPECS §2) — keep new events consistent

- One `Event` variant per **fact**, not per log line. Fill structured fields;
  rendering to text is the consumer's job.
- `topic()` = `<subsystem>.<chain>.<kind>.<detail>`; the first segment is the
  emitting subsystem (`listener.*` now, later `tracker.*` / `opportunity.*` /
  `trader.*`, never reusing `listener`). `app.*` / `bus.*` are process-level. Add
  the `topic()`, `level()`, `subsystem()` arms when you add a variant.
- The envelope carries `trace_id` (`<chain>:<token>`, the per-token join key) and,
  for detections, `event_id` (`<chain>:<tx>:<log_index>`, the dedup key).
- Numbers cross the bus as decimal strings in base units (`bus::num::Dec` over
  `U256`), never hex. Addresses/hashes as `0x` strings. Raw `alloy` types stay
  inside `chain/`.
- `TargetKind` (`launch` | `graduation`) is the routing axis shared by
  `watchlist.yaml`, `Target`, `Detection`, and every `topic()`.

## chain/

- `contracts.rs` — `alloy::sol!` event ABIs + `decode()`, and the getter
  interfaces enrichment reads. `indexed` markers must match the deployed ABI. All
  three events are from verified source. New event: add the `sol!` def, a
  `DecodedEvent` variant, an arm in `decode()`, then the signature in
  `watchlist.yaml`.
- `enrich.rs` — decoded log → full `Detection`. Concurrent view calls under a
  budget; failures leave fields `None`. RPC guards: immutable token facts come
  from `cache.rs` (so a graduation / relaunch / recurring quote token is one
  lookup), and a `MAX_CONCURRENT` semaphore caps burst call rate. Follow-ups:
  one Multicall3 round-trip instead of N calls; PonsV1 `dexId` → venue/router.
- `cache.rs` — process-wide `name/symbol/decimals/totalSupply` cache, keyed by
  address, two-generation eviction.
- `provider.rs` — one HTTP + WS `Provider` per chain, connected independently
  (a dead endpoint → `ChainDown`, the others still run).

## Where to spend effort

Thin by design, don't expand: `stages/style.rs` (ANSI), `banner.rs`, hex
shortening. The substance is the bus — the event taxonomy, the envelope contract,
the `topic()` scheme, the stage lifecycle, and keeping cross-stage messages
normalized per `SPECS.md`.
