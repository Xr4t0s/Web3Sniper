# web3sniper

Watches EVM chains over WebSocket for token **launches** and **graduations**,
decodes and enriches the matching logs (token facts, curve/pool state, dev buy),
and publishes typed events on an internal bus. Stages consume that stream: the
logger renders it now; a tracker, opportunity scorer, trader, dashboard feed and
chat alerts plug into the same bus later.

`SPECS.md` is the design contract (envelope, data model, pipeline). `CLAUDE.md`
is the working guide.

```
watchlist.yaml → listener ─(enrich)─▶ bus ─▶ logger  (+ tracker · opportunist · trader · dashboard)
```

## Run

```sh
cargo run                  # human-readable coloured logs
DEBUG=true cargo run | jq   # one JSON envelope per line
```

`NO_COLOR=1` or a non-TTY stdout disables colour and the banner.

## Configure — `watchlist.yaml`

```yaml
chains:
  <chain-id>:
    wss_rpc_url:   "wss://…"      # subscriptions
    https_rpc_url: "https://…"    # enrichment view calls
    targets:
      - kind: launch              # launch | graduation
        name: "PonsFamily V2"     # shown in logs / as `protocol`
        address: "0x…"            # contract that emits the event
        event: "Launched(address,address,address,address,uint256,uint256)"
```

Several targets may share a `kind` (coexisting factory / pool versions). The
`event` signature accepts an optional leading `event ` keyword.

### Adding an event type

`alloy::sol!` ABIs live in [`src/chain/contracts.rs`](src/chain/contracts.rs):

1. Add the `sol!` event with correct `indexed` markers (must match the deployed
   ABI — indexed fields are topics, the rest is `data`).
2. Add a `DecodedEvent` variant + an arm in `decode()`.
3. Handle it in [`src/chain/enrich.rs`](src/chain/enrich.rs) to fill the
   `Detection`.
4. Reference the signature in `watchlist.yaml`.

A signature / `indexed` mismatch surfaces as an `Undecoded` event, never a panic.

## Layout

| module     | responsibility                                                   |
|------------|-----------------------------------------------------------------|
| `config`   | parse `watchlist.yaml`                                           |
| `chain`    | RPC providers, `sol!` ABIs + log decode, log → `Detection` enrich |
| `bus`      | the `Bus`, `Envelope`, `Dec` numeric wrapper                     |
| `events`   | typed payloads (`Event`, `Detection`), topic / level / trace_id  |
| `stages`   | bus stages: `listener` (producer), `logger` (sink)               |

## Status

Detection, enrichment and logging work against live chains. Not built yet:
tracker, opportunity scorer, trader, and the non-terminal sinks. PonsV1
(`TokenLaunched`) enrichment still needs the `dexId` → venue/router lookup.
