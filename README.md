<p align="center">
  <h1 align="center">Sony Headphones Client (Rust Port)</h1>

  A pure Rust PC alternative for the mobile-only Sony Headphones app, ported from [Plutoberth/SonyHeadphonesClient](https://github.com/Plutoberth/SonyHeadphonesClient) with added **WH-1000-XM4** support.
  <br/><br/>
  <img width="556" src="static/showcase.gif" alt="Program Showcase">
</p>

## Table of Contents

* [Disclaimer](#disclaimer)
* [What Changed in This Fork](#what-changed-in-this-fork)
* [Features](#features)
* [Supported Platforms And Headsets](#supported-platforms-and-headsets)
* [Building & Running](#building--running)
* [Architecture](#architecture)
* [Credits & Contributors](#credits--contributors)
* [License](#license)

## Disclaimer

### THIS PROGRAM IS NOT AFFILIATED WITH SONY. YOU ARE RESPONSIBLE FOR ANY DAMAGE THAT MAY OCCUR WHILE USING THIS PROGRAM.

## What Changed in This Fork

- **Ported from C++ to pure Rust** under [`rust/`](rust/).
- Replaced the Dear ImGui + CMake/MSVC/Xcode build with [`eframe`](https://crates.io/crates/eframe)/`egui` (`glow`) and Cargo.
- Added **WH-1000-XM4** protocol support (20-step Ambient Sound Control, Speak-to-Chat, Multipoint, and Optimizer) alongside **WH-1000-XM3**.
- Added a `clap`-based CLI alongside the desktop GUI (`cargo run -- --help`).

## Features

- [x] Ambient Sound Control (XM3 19-step & XM4 20-step)
- [x] Disabling noise cancelling
- [x] Virtual Sound — VPT and Sound Position
- [x] WH-1000-XM4 Speak-to-Chat, Multipoint, and Optimizer
- [x] Desktop GUI (`eframe`/`egui`) & CLI subcommands (`gui`, `models`, packet encoding/simulation)

## Supported Platforms And Headsets

* **WH-1000-XM3**: Supported
* **WH-1000-XM4**: Supported
* **Windows**: Supported (native Winsock/Bluetooth RFCOMM via `windows-sys`)
* **Linux / macOS**: Not yet implemented in the Rust backend (`IBluetoothConnector` trait in `rust/bluetooth-connector-rs` is extensible for future backends)

## Building & Running

Requires a working [Rust toolchain](https://rustup.rs/) (edition 2024).

```sh
git clone https://github.com/debasser1911/SonyHeadphonesClient.git
cd SonyHeadphonesClient/rust/app
cargo build --release
cargo run
```

To run the test suite:

```sh
cd rust/app
cargo test
```

For CLI usage:

```sh
cargo run -- --help
```

## Architecture

See [`rust/MAP.md`](rust/MAP.md) for the full crate breakdown:

| Crate | Purpose |
| --- | --- |
| `rust/constants-rs` | Protocol constants, XM3/XM4 enums & capabilities |
| `rust/exceptions-rs` | `HeadphonesError` error types |
| `rust/timed-message-queue-rs` | Expiring UI message queue |
| `rust/byte-magic-rs` | Big-endian serialization & MAC address conversion |
| `rust/bluetooth-connector-rs` | `IBluetoothConnector` trait, Win32 RFCOMM backend & mock |
| `rust/command-serializer-rs` | MDR v1 framing, checksums & XM3/XM4 command serializers |
| `rust/bluetooth-wrapper-rs` | Bi-directional framing & ACK synchronization |
| `rust/headphones-rs` | WH-1000-XM3 & WH-1000-XM4 state machine |
| `rust/app` | `sony-headphones-client` binary (`eframe`/`egui` GUI + `clap` CLI) |

## Credits & Contributors

Ported to Rust in this fork; original C++ reverse-engineering and client by:

* [Plutoberth](https://github.com/Plutoberth) - Original Work and Windows Version
* [Mr-M33533K5](https://github.com/Mr-M33533K5) - BT Interface and Other Help
* [semvis123](https://github.com/semvis123) - macOS Version
* [jimzrt](https://github.com/jimzrt) - Linux Version
* [guilhermealbm](https://github.com/guilhermealbm) - Noise Cancelling Switch

## License

Distributed under the [MIT License](LICENSE). See `LICENSE` for more information.
