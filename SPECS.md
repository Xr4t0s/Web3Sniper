# SPECS — event bus, data model & pipeline

Draft for review. `CLAUDE.md` has the mechanics; this file locks the semantics:
what a bus message looks like, what data each stage needs, and the rules for
emitting and interpreting events.

**Built (2026-09-02):** envelope (§2), decimal-string numbers (`bus::num::Dec`),
Detection payload (§4), process-lifetime `Bus` + `CancellationToken` shutdown
(§8.1), module layout (§8.2), reconnect + heartbeat, `listener` enrichment via
concurrent view calls guarded by an immutable-fact cache (`chain/cache.rs`) and
a `MAX_CONCURRENT` semaphore.
**Open:** Multicall3 batching (§6), USD pricing, tracker/opportunist/executor (stub built).

## 1. Pipeline

`listener → tracker → opportunist → executor`, plus passive consumers (dashboard,
JSONL, alerts). Each stage subscribes to the bus, does its job, and **republishes
new facts under its own topic prefix**. Enrichment is additive and immutable: a
stage never edits a prior event, it emits a new one linked by `trace_id`.

```
                 ┌──────────┐
 chain logs ───▶ │ listener │  listener.<c>.<kind>.<Event>
                 └────┬─────┘  Detection = identity + launch economics + market snapshot
                      │
        ┌─────────────┼──────────────┐
        ▼             ▼              ▼
   ┌─────────┐  ┌───────────┐   passive: dashboard · jsonl · alerts
   │ tracker │  │opportunist│
   └────┬────┘  └─────┬─────┘
 tracker.<c>.<tok>.snapshot   opportunity.<c>.<tok>
   (price/liq/vol/holders)      (score + gates + entry plan)
        │              │
        └──────┬───────┘
               ▼
          ┌────────┐
          │ executor │  executor.<c>.<tok>.intent | .result
          └────────┘
```

- **listener** — read the WS log stream; per log, decode it and spawn a detached
  enrichment task that does a round of **concurrent view calls** (immutable facts
  cached, a semaphore caps burst rate), builds the Detection, and emits it. Hard
  timeout; a failed call → that field `null`, event still emitted. No external
  HTTP, no simulation, no time-series. The stream reader never blocks. Follow-up:
  collapse the calls into one Multicall3 round-trip (§6).
- **tracker** — owns a token over time: subscribes to its curve/pool, accumulates
  price / liquidity / volume / holders, emits snapshots on a timer and on
  material moves.
- **opportunist** — consumes Detection + snapshots, runs risk sims + heuristics,
  emits a decision.
- **executor** — consumes a decision, builds and submits the buy, reports the result.

## 2. Envelope

Every bus message is an envelope with the payload flattened in:

```jsonc
{
  "schema": 1,
  "ts": "2026-09-02T21:55:57.462+02:00",  // emitter wall clock
  "subsystem": "listener",                 // == topic's first segment
  "topic": "listener.rh.launch.Launched",
  "level": "info",
  "trace_id": "rh:0xf9e339…e656",          // <chain>:<token lowercased>
  "event_id": "rh:0xf585…5845:74",         // <chain>:<tx>:<log_index> — detections only
  "caused_by": "rh:0xf585…5845:74",        // event_id this was derived from (enrichment stages)
  "type": "detection",
  "block_number": 52837125,                 // block the on-chain reads are as-of
  // …payload…
}
```

Rules:
- `trace_id = <chain>:<token>` — deterministic, every stage computes it. Primary
  join key and per-token dedup key. Chain-scoped (same address on two chains ≠).
- `event_id` — dedup identity for detections; the bus may redeliver after a
  reconnect.
- **Numbers**: token/quote amounts are decimal strings in base units
  (`"1000000000000000000"`); bps, block numbers, counts are integers; prices are
  decimal strings. **No hex for numeric values** (needs a serde wrapper over
  alloy `U256`).
- Addresses checksummed `0x`; hashes lowercase `0x`.
- `null` = applicable but unknown / call failed. Absent = not applicable to this
  variant or venue.
- Additive only: new fields appended, `schema` bumps on a breaking change,
  consumers ignore unknown fields and unknown topics.

## 3. Data inventory

Origin: `log` (decoded event) · `call` (view call, listener budget) ·
`sim` (simulation) · `series` (accumulated over time) · `ext` (external, e.g.
USD rate). Columns: **T** tracker · **O** opportunist · **X** executor.

### 3.1 Provenance — listener
| field | origin | T | O | X |
|---|---|:-:|:-:|:-:|
| chain, protocol, kind, event | log | ✓ | ✓ | ✓ |
| token address | log | ✓ | ✓ | ✓ |
| tx_hash, block_number, log_index | log | ✓ | ✓ | ✓ |
| block_timestamp | call | ✓ | ✓ | ✓ |
| observed_at | local | ✓ | ✓ | ✓ |

### 3.2 Token — listener
| field | origin | T | O | X |
|---|---|:-:|:-:|:-:|
| name, symbol, decimals | call | ✓ | ✓ | ✓ |
| total_supply | call | ✓ | ✓ | · |
| creator (dev) | log | ✓ | ✓ | · |
| launcher / launch router | log | · | ✓ | · |

### 3.3 Launch economics — listener
| field | origin | T | O | X |
|---|---|:-:|:-:|:-:|
| dev_buy_tokens, dev_buy_quote | log | · | ✓ | · |
| dev_buy_pct (of supply) | derived | · | ✓ | · |
| quote_token {address, symbol, decimals} | log+call | ✓ | ✓ | ✓ |
| graduation_threshold (quote) | call | ✓ | ✓ | · |
| phantom_quote | call | ✓ | · | · |
| curve_fee_bps, creator_tax_bps | call | · | ✓ | ✓ |
| anti_snipe {max_wallet_bps, max_tx_bps, restriction_end_block} | call | · | ✓ | ✓ |
| snipe_tax {start_bps, decay_secs, launched_at} | call+log | · | ✓ | ✓ |

### 3.4 Market state at detection — listener (`market: "bonding_curve" | "pool"`)

bonding_curve:
| field | origin | T | O | X |
|---|---|:-:|:-:|:-:|
| curve_address | log | ✓ | ✓ | ✓ |
| real_quote_reserve, token_reserve | call | ✓ | ✓ | ✓ |
| progress_bps (reserve / threshold) | derived | ✓ | ✓ | · |
| price_native | derived | ✓ | ✓ | ✓ |

pool:
| field | origin | T | O | X |
|---|---|:-:|:-:|:-:|
| venue (dex name / `uniswap-v4`) | call | ✓ | ✓ | ✓ |
| pool_address or pool_id | log/derived | ✓ | ✓ | ✓ |
| pool_key {currency0, currency1, fee, tick_spacing, hooks} | derived | · | · | ✓ |
| position_id | log | · | · | · |
| router_address | call | · | · | ✓ |
| reserve_token, reserve_quote  *(or sqrt_price_x96 + liquidity)* | call | ✓ | ✓ | ✓ |
| price_native | derived | ✓ | ✓ | ✓ |

### 3.5 Valuation — listener (native) / tracker (USD)
| field | origin | T | O | X |
|---|---|:-:|:-:|:-:|
| price_native | derived | ✓ | ✓ | ✓ |
| liquidity_native | derived | ✓ | ✓ | · |
| fdv_native = price × supply | derived | · | ✓ | · |
| quote_usd_rate | ext | ✓ | ✓ | · |
| price_usd, liquidity_usd, fdv_usd | derived | ✓ | ✓ | · |

### 3.6 Activity — tracker only, `series` (windows: 1m / 5m / 1h / since_launch)
| field | T | O | dash |
|---|:-:|:-:|:-:|
| buys, sells, quote_volume, unique_buyers per window | ✓ | ✓ | ✓ |
| price_change_bps per window | ✓ | ✓ | ✓ |
| liquidity adds / removes | ✓ | ✓ | ✓ |
| holder_count, top10_concentration_bps | ✓ | ✓ | ✓ |
| curve_progress_bps over time, graduation_eta | ✓ | ✓ | ✓ |
| age_blocks, age_secs | ✓ | ✓ | ✓ |

### 3.7 Risk — opportunist, `sim` + heuristic
| field | O | X | dash |
|---|:-:|:-:|:-:|
| can_sell (honeypot sim) | ✓ | ✓ | ✓ |
| buy_tax_bps, sell_tax_bps (sim) | ✓ | ✓ | ✓ |
| lp_locked | ✓ | ✓ | ✓ |
| ownership_renounced, has_mint, has_blacklist, is_proxy | ✓ | ✓ | ✓ |
| creator_prior_launches, creator_rug_count | ✓ | · | ✓ |
| contract_verified | ✓ | · | ✓ |

### 3.8 Decision — opportunist output
| field | X | dash |
|---|:-:|:-:|
| decision (watch / enter / skip) | ✓ | ✓ |
| score 0–100, gates{…}, reasons[] | ✓ | ✓ |
| used_snapshot (tracker event_id) | · | ✓ |
| entry_plan {max_quote_in, min_tokens_out, max_price_native, valid_until_block} | ✓ | ✓ |

### 3.9 Execution — executor output
| field | dash | T |
|---|:-:|:-:|
| intent {venue, target_address, amount_in, min_out, route, gas caps, deadline} | ✓ | ✓ |
| result {tx_hash, status, block, filled_in, filled_out, effective_price_native, gas_used, error} | ✓ | ✓ |

## 4. Detection payload (listener output — the one we build next)

```jsonc
{
  "type": "detection",
  "chain": "rh",
  "protocol": "PonsFamily V2",
  "kind": "launch",                        // launch | graduation
  "event": "Launched",
  "token": {
    "address": "0x…",
    "name": "snob", "symbol": "SNOB", "decimals": 18,
    "total_supply": "1000000000000000000000000000"
  },
  "creator": "0x…",                        // dev
  "launcher": "0x…",                       // tx sender / launch router
  "dev_buy": { "tokens": "…", "quote": "…", "pct_bps": 42 },
  "quote_token": { "address": "0x…", "symbol": "WETH", "decimals": 18 },
  "economics": {
    "graduation_threshold": "…", "phantom_quote": "…",
    "curve_fee_bps": 100, "creator_tax_bps": 0,
    "anti_snipe": { "max_wallet_bps": null, "max_tx_bps": null, "restriction_end_block": null },
    "snipe_tax": { "start_bps": 9900, "decay_secs": 15, "launched_at": 1725310000 }
  },
  "market": "bonding_curve",               // bonding_curve | pool
  "curve": {                               // when market == bonding_curve
    "address": "0x…",
    "real_quote_reserve": "…", "token_reserve": "…",
    "progress_bps": 0, "price_native": "0.000000123"
  },
  "pool": null,                            // when market == pool: { venue, pool_id/address,
                                           //   pool_key{…}, position_id, router_address,
                                           //   reserve_token, reserve_quote, price_native }
  "valuation": { "price_native": "…", "liquidity_native": "…", "fdv_native": "…" }
}
```

`pool` and `curve` are mutually exclusive per `market`. PonsV2 `Launched` →
`bonding_curve`; PonsV2 `PoolGraduated` and PonsV1 `TokenLaunched` → `pool`.

Downstream payloads (tracker snapshot, opportunity, trade intent/result) follow
the §3 field groups; schema them when each stage is built.

## 5. Correlation & dedup
- Join everything on `trace_id` (`<chain>:<token>`).
- Dedup detections on `event_id` (`<chain>:<tx>:<log_index>`).
- A launch and its later graduation share a `trace_id`; `kind` separates them.
- Same token relaunched under another protocol → still one `trace_id`;
  `protocol` separates.

## 6. Open questions

- **Multicall3 batching.** Enrichment currently fires ~8 view calls concurrently
  under a 1.5 s budget (`chain/enrich.rs`). Collapse to one `Multicall3` call
  (`0xcA11…CA11`, native alloy support) with a batch-RPC fallback — confirm the
  contract is deployed on `rh` first.
- **Native quote token.** A native-quoted launch shows `quote_token` as address
  zero with no symbol/decimals. Special-case it to the chain's native
  (`ETH` / 18) — needs a per-chain native symbol in config.
- **USD pricing.** Who owns `quote_usd_rate` — the tracker off a reference pool,
  or a small shared price stage publishing `price.<chain>.<symbol>`?
- **Snapshot cadence.** Timer interval + material-move thresholds (price ±X bps,
  liquidity ±Y%).
- **Backfill for late consumers.** Ring buffer inside the WS/dashboard stage, or
  a small shared replay log?

## 7. Non-goals (now)
- Bus persistence / replay.
- Multi-process / network bus (the dashboard WS consumer covers external fan-out).
- Config hot-reload.

## 8. Architecture (end-to-end)

### 8.1 Lifecycle (decided)

`main` owns the canonical bus `Sender` for the whole process. Shutdown is
explicit, not "last Sender dropped":

```
main
 ├─ bus: Sender            (held until main returns)
 ├─ shutdown: CancellationToken
 ├─ spawn logger      (Receiver)
 ├─ spawn listener    (Sender + shutdown)          producers
 ├─ spawn tracker     (Sender + Receiver + shutdown)  transformers
 ├─ spawn opportunist (Sender + Receiver + shutdown)
 ├─ spawn executor      (Sender + Receiver + shutdown)
 └─ tokio::signal::ctrl_c → shutdown.cancel() → join all → exit
```

Every stage loop is `select! { _ = shutdown.cancelled() => break, msg = rx.recv() => … }`.
Consumers may now hold a `Sender` and emit freely — this is what makes the
enricher, tracker, opportunist and executor possible. **Rewrites CLAUDE.md §3.**

### 8.2 Module map (proposed)

```
src/
  main.rs              wiring + lifecycle
  banner.rs
  config/              watchlist.yaml
  bus/
    mod.rs             Sender/Receiver, Envelope, trace_id/event_id, channel()
    num.rs             serde newtype over U256/I256 → decimal strings
  events/
    mod.rs             Event enum (tags the payloads)
    detection.rs       §4
    tracker.rs         snapshot payload            (later)
    opportunity.rs     decision payload            (later)
    trade.rs           intent / result payloads    (later)
  chain/
    provider.rs        RPC providers per chain     (was provider/)
    contracts.rs       sol! ABIs + log decode
    cache.rs           immutable token-fact cache
    enrich.rs          decoded log → Detection (concurrent calls)
  stages/
    mod.rs             Stage trait + driver + shutdown plumbing
    logger.rs          stdout, human or JSON
    listener.rs        WS subs + per-log detached enrich
    executor.rs        stub: Detection → gate → TradeIntent / TradeSkipped
    tracker.rs         (later)
    opportunist.rs     (later)
```

Rename rationale: `consumer` → `stages` (some are producers/transformers, not
just sinks); `event.rs` → `bus/` + `events/` (payload types will multiply);
`provider` + `contracts` → `chain/` (all the on-chain surface in one place).
Not yet done — execute after this section is approved.

### 8.3 Stage responsibilities

| stage | consumes | emits | holds |
|---|---|---|---|
| listener | — (WS) | `listener.<c>.<kind>.<Event>` (Detection) | `Sender`, `Providers` |
| tracker | Detection | `tracker.<c>.<tok>.snapshot` | `Sender`, `Receiver`, `Providers` |
| opportunist | Detection, snapshot | `opportunity.<c>.<tok>` | `Sender`, `Receiver`, `Providers` |
| executor | opportunity | `executor.<c>.<tok>.{intent,result}` | `Sender`, `Receiver`, `Providers` |
| logger | all | — (stdout) | `Receiver` |
| dashboard | all | — (WS out) | `Receiver` |

### 8.4 One launch, end to end

```
WS log ─▶ listener: decode → spawn enrich task
                       │
             enrich: Multicall3 (token + curve getters + block ts), ≤500ms
                       │
                       ▼  emit
     listener.rh.launch.Launched  {trace_id: rh:0xTOKEN, …Detection}
        │                              │
        ▼ tracker                      ▼ opportunist
   subscribe to curve,           risk sims + heuristics, wait for
   accumulate price/vol/…        first snapshot, score
        │                              │
        ▼ (timer / move)               ▼ (decision = enter)
   tracker.rh.0xTOKEN.snapshot    opportunity.rh.0xTOKEN {entry_plan}
                                       │
                                       ▼ executor
                              build buy from Detection.pool/curve + entry_plan,
                              submit, then emit executor.rh.0xTOKEN.result
```
All five topics share `trace_id: rh:0xTOKEN`; the dashboard joins them into one
token view.
