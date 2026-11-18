# SonyHeadphonesClient Rust Migration Map

Goal: Rewrite `Plutoberth/SonyHeadphonesClient` in pure Rust under `rust/` using `rust-anything`, and add full support for **Sony WH-1000-XM4** alongside **WH-1000-XM3**.

## Decisions & Research
- [DECISIONS.md](DECISIONS.md) — ADRs 001..004
- [GLOSSARY.md](GLOSSARY.md) — C++ to Rust domain glossary
- [plan.md](plan.md) — `rust-anything analyze --whole-app` plan
- [research/wh1000xm4_protocol.md](research/wh1000xm4_protocol.md) — MDR v1 framing, XM4 commands, checksum fix
- [research/bluetooth_rfcomm.md](research/bluetooth_rfcomm.md) — Win32 Winsock/Bluetooth RFCOMM via `windows-sys`
- [research/gui_egui.md](research/gui_egui.md) — `eframe`/`egui` replacement for Dear ImGui + DX11

## Units by Wave

| Wave | Crate | Original Unit(s) | State | Gate | Notes |
| ---: | --- | --- | --- | --- | --- |
| 0 | `rust/constants-rs` | `Client/Constants.h` | done | pass | Adds WH-1000-XM4 enums & capabilities |
| 0 | `rust/exceptions-rs` | `Client/Exceptions.h` | done | pass | `HeadphonesError` |
| 0 | `rust/timed-message-queue-rs` | `Client/TimedMessageQueue.h` | done | pass | Expiring UI message queue |
| 1 | `rust/byte-magic-rs` | `Client/ByteMagic.h` | done | pass | BE serialization & MAC conversion |
| 1/2 | `rust/bluetooth-connector-rs` | `Client/IBluetoothConnector.h`, `Client/windows/` | done | pass | `windows-sys` RFCOMM + `MockBluetoothConnector` |
| 2 | `rust/command-serializer-rs` | `Client/CommandSerializer.cpp` | done | pass | Fixes `unpack_bt_message` bug + XM4 serializers |
| 3 | `rust/bluetooth-wrapper-rs` | `Client/BluetoothWrapper.cpp` | done | pass | Bi-directional framing & ACK sync |
| 4 | `rust/headphones-rs` | `Client/Headphones.cpp` | done | pass | XM3 & WH-1000-XM4 state machine |
| 5 | `rust/app` | `Client/CrossPlatformGUI.cpp`, `Client/windows/` | done | pass | `eframe`/`egui` GUI + `clap` CLI (`12/12` replay) |

## Out of Scope
- `Client/imgui/` (replaced by `eframe`/`egui` per ADR-003)
- `Client/linux/`, `Client/macos/` native backends (target is Windows `x86_64-pc-windows-gnullvm`; `IBluetoothConnector` trait remains cross-platform extensible)
