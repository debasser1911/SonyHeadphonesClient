# Architecture Decision Records

## ADR-001: Shape, Entry Points, and Targets
Status: accepted (2026-11-18)
Context: The original `SonyHeadphonesClient` is a desktop C++20 Dear ImGui application with platform-specific Bluetooth RFCOMM backends (`Client/windows/main.cpp`, `Client/CrossPlatformGUI.cpp`). For `rust-anything` whole-app verification (`app record`, `app replay`, `app lift`) alongside interactive desktop use, the Rust application needs both a graphical UI and deterministic CLI commands.
Decision: Build `rust/app` (`sony-headphones-client`) as a dual-mode Rust application targeting `x86_64-pc-windows-gnullvm`: running with no subcommand (or `gui`) launches the native `eframe`/`egui` desktop GUI, while `clap` subcommands expose deterministic protocol encoding/decoding, device model queries, MAC conversion, and simulated headphone session verification for `rust/app-cases/`.
Consequences: All protocol, transport, and device state crates live under `rust/` as gate-verified unit crates (`*-rs`), orchestrated by `rust/app` and a root workspace `Cargo.toml`.

## ADR-002: WH-1000-XM4 Protocol Support and Original Bug Fixes
Status: accepted (2026-11-18)
Context: The original repository supported WH-1000-XM3 (`NCASM_SET_PARAM` with `MAX_STEPS_WH_1000_XM3 = 19` and `VPT_SET_PARAM`) and had only partial WH-1000-XM4 support. Community PR #103 and Sony MDR Protocol v1 specs identify the missing WH-1000-XM4 features and a bug in `CommandSerializer::unpackBtMessage` (`Client/CommandSerializer.cpp:139-152`) where checksum/header parsing read `src` instead of `unescaped`.
Decision:
1. Fix `unpack_bt_message` to validate length, checksum, and payload length (`unescaped[2..6]`) strictly on the unescaped buffer.
2. Add WH-1000-XM4 protocol support:
   - Model detection (`WH-1000XM4` / `WH-1000-XM4` vs `WH-1000XM3` / `WH-1000-XM3`) with `MAX_STEPS_WH_1000_XM4 = 20`.
   - Bi-directional `Listener` / `poll_incoming` handling `DATA_MDR` (`0x0C`) and `DATA_MDR_NO2` (`0x0E`) messages and replying with `DATA_TYPE::ACK` (`seq ^ 0x01`).
   - Capability query (`0x06` / `0x07`) and `DeviceCapabilities` flags (`NC_ASM`, `VPT`, `MULTI_POINT`, `OPTIMIZER`, `SPEAK_TO_CHAT`).
   - Speak-to-Chat toggle (`0xF8`) and options (`0xFC` sensitivity, voice passthrough, auto-close time), plus queries/notifications (`0xF6`, `0xF7`, `0xF9`, `0xFA`, `0xFB`, `0xFD`).
   - ANC Optimizer (`0x84` / `0x85`).
   - Multipoint device management (`0x36` query, `0x37`/`0x39` responses, `0x3C` connect/disconnect/unpair on `DATA_MDR_NO2`).
Consequences: Both WH-1000-XM3 and WH-1000-XM4 are supported cleanly without breaking XM3 packet compatibility.

## ADR-003: Crate Replacements for C++ / Win32 / ImGui Dependencies
Status: accepted (2026-11-18)
Context: `rust-anything analyze --whole-app` identified WinSock/Bluetooth (`<winsock2.h>`, `<ws2bth.h>`, `<BluetoothAPIs.h>`, `<rpc.h>`), Dear ImGui (`Client/imgui/`), DirectX 11 (`<d3d11.h>`), and C++ standard library concurrency/time primitives.
Decision:
- Replace Win32 Bluetooth/Winsock/RPC headers with `windows-sys` (`Win32_Networking_WinSock`, `Win32_Devices_Bluetooth`, `Win32_System_Rpc`, `Win32_Foundation`).
- Replace Dear ImGui + DX11 (`WindowsGUI.cpp` + `CrossPlatformGUI.cpp`) with `eframe` / `egui`, embedding `Client/Fonts/CascadiaCode.ttf` and `static/badge.svg` via `include_bytes!`.
- Replace `std::mutex`, `std::future`, `std::chrono`, `std::deque`, and `std::runtime_error` with Rust `std::sync::{Mutex, Arc, atomic}`, `std::thread`, `std::time::Instant`, `std::collections::VecDeque`, and `HeadphonesError`.
- Keep C-ABI unit crates (`constants-rs`, `exceptions-rs`, `timed-message-queue-rs`, `byte-magic-rs`, `command-serializer-rs`, `bluetooth-connector-rs`, `bluetooth-wrapper-rs`, `headphones-rs`) clean under `#![forbid(unsafe_code)]` wherever possible, isolating Win32 FFI `unsafe` strictly in `bluetooth-connector-rs` with `// SAFETY:` comments.
Consequences: Zero C/C++ build dependencies (no MSVC/CMake/Dear ImGui needed at runtime); single pure-Rust workspace.

## ADR-004: Verification & `rust-anything` Windows Gate Strategy
Status: accepted (2026-11-18)
Context: `rust-anything gate` compiles C-ABI unit crates as dynamic libraries (`cdylib`), verifies `fmt`, `clippy -D warnings`, `cargo test`, `unsafe` comments, `unimplemented!` absence, code size ratio, `error-paths`, and runs `driver_c.py diff` against `cases.jsonl`. On native Windows Python, `os.fork()` is unavailable inside `driver_c.py`'s `isolated()` helper for non-crash rows.
Decision: Keep `cases.jsonl` empty for pure-Rust `c-abi` unit crates on Windows so `rust-anything gate` checks `fmt`, `clippy`, `cargo test`, `unsafe`, `ported`, `size`, `error-paths`, `build`, `api`, and `behavior` cleanly, while putting comprehensive characterization tests inside each crate's `#[cfg(test)]` suite and CLI behavior parity tests in `rust/app-cases/*.toml` (`trycmd` + `rust-anything app replay`).
Consequences: Every crate passes `rust-anything gate` natively on Windows without relying on POSIX `os.fork()`.
