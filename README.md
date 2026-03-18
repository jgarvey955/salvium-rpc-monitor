# Salvium Monitor

To Donate SAL1= SC11UA22DFrAQerDwJwcf8Yh2ySTb7ipaFL8qSEX26tqUDdPf1RQBmmRuZG4SnRd8DNpp5vE1zDHnKNStiFDQsce49Q7fyp8Yp

Rust desktop monitor for Salvium daemon RPC and wallet RPC.

The app is built with `iced` and opens a native window titled `Salvium Monitor`. It provides:

- A `Home` view with daemon and wallet status cards
- Separate `Daemon RPC` and `Wallet RPC` tabs
- A `Preferences` page for connection settings
- Manual method polling with captured output panes
- Automatic status polling on a configurable interval

## Current Behavior

It currently supports:

- HTTP and HTTPS transport selection for daemon and wallet RPC
- RPC login enable/disable for daemon and wallet
- Digest-auth retry handling when the server challenges with Digest
- HTTPS operation without a CA-cert field, matching the current `curl -k` style workflow
- Method and parameter-template loading from [rpc.output](rpc.output) and [walletrpc.output](walletrpc.output)
- Labeled request fields for methods that require input
- Separate daemon and wallet status badges
- Separate daemon and wallet last-poll timestamps
- Restricted daemon mode filtering

## Build

```bash
cargo build --release
```

The binary is written to:

```bash
target/release/salvium-monitor
```

## Run

```bash
./target/release/salvium-monitor
```

You can also run:

```bash
cargo run
```

## First Run Defaults

If `settings.json` does not exist, the app starts with neutral defaults:

- Daemon IP: `127.0.0.1`
- Daemon port: `19081`
- Wallet IP: `127.0.0.1`
- Wallet port: `19092`
- Transports: blank until chosen
- Daemon login: disabled
- Wallet login: disabled
- Wallet RPC: disabled
- Poll frequency: `10` seconds

Settings are saved to [settings.json](settings.json).

## Preferences

`Save Settings` does not blindly write the file. It first validates the entered settings by polling the configured endpoints. If verification succeeds, the settings are written and the app reconnects.

Daemon settings:

- One IP field
- One port field
- One transport selector: `http` or `https`
- `Daemon Restricted Mode` toggle
- Optional RPC login

Wallet settings:

- Enable/disable toggle
- One IP field
- One port field
- One transport selector: `http` or `https`
- Optional RPC login

## Polling Model

There are two different polling paths:

- Automatic status polling: always checks daemon base status on the configured interval
- Manual RPC polling: the `Poll` button in the daemon or wallet tab sends the currently selected method with the currently entered parameters

Changing the method or template dropdown clears that tab’s output pane immediately. The output only repopulates after you press `Poll`.

## RPC Tabs

The `Daemon RPC` and `Wallet RPC` tabs both use:

- A method dropdown
- A parameter-template dropdown
- Generated input fields when a method needs arguments
- A manual `Poll` button
- A scrollable output pane for the returned payload or failure data

The daemon tab uses [rpc.output](rpc.output). The wallet tab uses [walletrpc.output](walletrpc.output).

## Authentication And TLS

The app currently behaves like the tested Salvium RPC setup in this workspace:

- Requests are sent to the selected HTTP or HTTPS endpoint
- If the server returns `401` with a Digest challenge, the client retries with Digest auth
- If there is no Digest challenge and login is enabled, the client can fall back to Basic auth
- HTTPS certificate verification is currently relaxed to match `curl -k` style testing

Important:

- There is no CA-cert field in the UI now
- Wallet HTTPS works with Digest auth in the current setup
- If you want strict certificate verification later, that needs to be added back explicitly

## Restricted Daemon Mode

Restricted mode is handled separately from normal daemon mode.

When `Daemon Restricted Mode` is enabled:

- The daemon method dropdown is filtered to methods that are appropriate for restricted operation
- Methods that are still gated by the rpc-payment client-signature flow are hidden from the daemon dropdown
- The base daemon status poll uses restricted-safe methods instead of `get_info`
- Unavailable daemon summary fields show `Restricted mode active` instead of `Unknown`

The app keeps one daemon IP field and one daemon port field. In the current local setup, restricted mode uses the same daemon IP with a different port.

## Local Tested Configuration

This repo has been tested in the current environment with:

- Normal daemon: `192.168.0.30:19081`
- Restricted daemon: `192.168.0.30:19089`
- Wallet RPC: `192.168.0.30:19092`

Current observed behavior of the restricted daemon in this environment:

- `get_version` works
- `get_block_count` works
- `get_info` is signature-gated and does not work as a base poll
- Some rpc-payment methods remain unavailable without client-signature support

## Notes

- The wallet balance display is formatted in SAL units with 8 decimal places
- Long wallet addresses are displayed in a full-width single-line field
- The app is designed for desktop use and expects a graphical session

## Development Notes

Useful files:

- [src/app.rs](src/app.rs): UI, polling flow, view logic
- [src/rpc.rs](src/rpc.rs): HTTP/HTTPS RPC client, Digest handling
- [src/settings.rs](src/settings.rs): saved settings model
- [src/inventory.rs](src/inventory.rs): RPC inventory parsing and method/template generation
- [rpc.output](rpc.output): daemon RPC extraction
- [walletrpc.output](walletrpc.output): wallet RPC extraction
