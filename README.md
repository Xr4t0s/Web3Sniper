# web3sniper

Watches EVM chains over WebSocket for token **launches** and **graduations**,
decodes the matching logs, enriches them with token metadata, and publishes typed
events on an internal bus. A logger renders them to the terminal; other consumers
(dashboard feed, spreadsheet, chat alerts, trade execution) plug into the same
stream.

```
                         ┌──────────────┐
 watchlist.yaml ───────▶ │   listener   │  one task per chain,
                         │  sublistener │  one WS subscription per target
                         └──────┬───────┘
                                │  Event
                       ┌────────▼─────────┐   broadcast::channel<Event>
                       │       bus        │
                       └───┬────────┬─────┘
                           │        │
                     ┌─────▼──┐  ┌──▼─────────────┐
                     │ logger │  │ future consumers│  ws feed · xlsx · discord
                     └────────┘  └────────────────┘
```

## Run

```sh
cargo run
```

Set `NO_COLOR=1` or pipe the output to disable ANSI colour.

## Configure — `watchlist.yaml`

```yaml
chains:
  <chain-id>:
    wss_rpc_url:   "wss://…"      # subscriptions
    https_rpc_url: "https://…"    # metadata / eth_call
    targets:
      - kind: launch              # launch | graduation
        name: "PonsFamily V2"     # shown in logs
        address: "0x…"            # contract that emits the event
        event: "Launched(address,address,address,address,uint256,uint256)"
```

Several targets may share a `kind` (coexisting factory / pool versions). The
`event` signature accepts an optional leading `event ` keyword.

### Adding an event type

Event decoding is typed via `alloy::sol!` in [`src/contracts/mod.rs`](src/contracts/mod.rs).
To watch a new event:

1. Add its `sol!` definition with the correct `indexed` markers (they must match
   the deployed ABI — indexed fields live in topics, the rest in `data`).
2. Add a `DecodedEvent` variant and wire it into `decode()`.
3. Reference it from `watchlist.yaml`.

A signature or `indexed` mismatch surfaces as an `Undecoded` event in the logs,
never a panic.

## Layout

| module        | responsibility                                             |
|---------------|-----------------------------------------------------------|
| `config`      | parse `watchlist.yaml`                                     |
| `provider`    | one HTTP + WS `Provider` per chain                         |
| `contracts`   | `sol!` event ABIs, log decoding, ERC-20 metadata          |
| `event`       | the bus and the `Event` type (all `Serialize`)            |
| `listener`    | fan out to one task per chain, one subscription per target |
| `consumer`    | bus sinks; `logger` is the only one so far                |

## Status

Detection and logging work. Not implemented yet: trade execution, and the
non-terminal consumers (dashboard socket, spreadsheet, Discord/Telegram).
The two `launch` event ABIs are inferred and marked `TODO` in `contracts`.
