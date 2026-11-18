# Domain Glossary: SonyHeadphonesClient (C++ -> Rust)

| C++ Identifier / Concept | Meaning | Rust Type / Module / Function |
| --- | --- | --- |
| `Buffer` (`std::vector<char>`) | Raw byte buffer for Bluetooth frames and payloads | `Vec<u8>` (`&[u8]` for borrowed slices) |
| `DATA_TYPE` | MDR packet framing type (`DATA_MDR = 12`, `ACK = 1`, `DATA_MDR_NO2 = 14`, etc.) | `constants_rs::port::DataType` |
| `COMMAND_TYPE` | MDR command opcode (`VPT_SET_PARAM = 72`, `NCASM_SET_PARAM = 104`, XM4 opcodes) | `constants_rs::port::CommandType` |
| `NC_ASM_*` / `ASM_*` | Noise Cancelling & Ambient Sound Mode settings, effect, inquired type, ID | `NcAsmInquiredType`, `NcAsmEffect`, `NcAsmSettingType`, `AsmSettingType`, `AsmId`, `NcDualSingleValue` |
| `VPT_PRESET_ID` / `SOUND_POSITION_PRESET` | Virtual Phone Technology surround preset and sound position enum | `VptPresetId`, `SoundPositionPreset`, `VptInquiredType` |
| `OPTIMIZER_STATE` | WH-1000-XM4 ANC Optimizer state (`Idle = 0`, `Optimizing = 1`) | `OptimizerState` in `constants-rs` |
| `S2C_TOGGLE` / S2C Options | WH-1000-XM4 Speak-to-Chat toggle (`Inactive = 0`, `Active = 1`) & sensitivity/voice/auto-close | `S2cToggle`, `S2cSensitivity`, `S2cAutoOff` |
| `MULTI_POINT_COMMANDS` | WH-1000-XM4 multipoint actions (`Disconnect = 0`, `Connect = 1`, `Unpair = 2`) | `MultiPointCommand` in `constants-rs` |
| `FUNCTION_TYPE` / `DEVICE_CAPABILITIES` | Capability query item IDs and bitmask flags | `FunctionType`, `DeviceCapabilities` in `constants-rs` |
| `HeadphoneModel` | Detected or selected headset family (`WH-1000XM3`, `WH-1000XM4`, `Generic`) | `HeadphoneModel` in `constants-rs` |
| `RecoverableException` | Recoverable UI-facing error that may require socket disconnect | `HeadphonesError::Recoverable { message, should_disconnect }` in `exceptions-rs` |
| `TimedMessageQueue` / `Message` | Bounded FIFO queue of expiring UI error notifications | `TimedMessageQueue`, `TimedMessage` in `timed-message-queue-rs` |
| `ByteMagic` | Endian swap, BE int serialization, MAC address string/integer conversion | `byte_order_swap`, `int_to_bytes_be`, `mac_string_to_long`, `mac_bytes_to_string` in `byte-magic-rs` |
| `CommandSerializer` / `BtMessage` | Packet escaping (`0x3D`), checksum, framing (`>`..`<`), payload serializers/parsers | `BtMessage`, `package_data_for_bt`, `unpack_bt_message`, `serialize_*` in `command-serializer-rs` |
| `IBluetoothConnector` / `BluetoothDevice` | Platform RFCOMM socket & Bluetooth radio discovery abstraction | `IBluetoothConnector`, `BluetoothDevice`, `WindowsBluetoothConnector`, `MockBluetoothConnector` in `bluetooth-connector-rs` |
| `BluetoothWrapper` / `Listener` | Thread-safe framed command sender, ACK synchronization, sequence tracking, packet reader | `BluetoothWrapper` in `bluetooth-wrapper-rs` |
| `Property<T>` | Tracked setting with `current` vs `desired` state and `fulfill()` / `is_fulfilled()` | `Property<T>` in `headphones-rs` |
| `Headphones` | High-level headset state machine supporting XM3 and WH-1000-XM4 | `Headphones` in `headphones-rs` |
| `CrossPlatformGUI` / `WindowsGUI` | Desktop GUI window and controls | `SonyHeadphonesApp` (`eframe::App`) in `rust/app/src/gui.rs` |
