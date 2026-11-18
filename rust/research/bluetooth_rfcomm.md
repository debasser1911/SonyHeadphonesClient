# Win32 Bluetooth RFCOMM Research (`windows-sys`)

## Sources
- Local repo: `Client/windows/WindowsBluetoothConnector.h`, `Client/windows/WindowsBluetoothConnector.cpp`, `Client/ByteMagic.cpp`
- `windows-sys v0.61.2` (`Win32_Networking_WinSock`, `Win32_Devices_Bluetooth`, `Win32_System_Rpc`, `Win32_Foundation`)

## Implementation Mapping
1. **Winsock Initialization**: `WSAStartup(0x0202, &mut wsa_data)` guarded via `std::sync::OnceLock` so it initializes once per process.
2. **RFCOMM Socket**:
   - `socket(AF_BTH as i32, SOCK_STREAM as i32, BTHPROTO_RFCOMM as i32)`
   - `setsockopt(sock, SOL_RFCOMM as i32, SO_BTH_AUTHENTICATE as i32, &enable, 4)`
   - `setsockopt(sock, SOL_RFCOMM as i32, SO_BTH_ENCRYPT as i32, &enable, 4)`
3. **SOCKADDR_BTH & Service UUID**:
   - `addressFamily = AF_BTH as u16`
   - `serviceClassId = GUID::from_u128(0x96CC203E_5068_46ad_B32D_E316F5E069BA)` (exact byte order matching `UuidFromStringA("96CC203E-5068-46ad-B32D-E316F5E069BA", ...)`: `data1 = 0x96cc203e`, `data2 = 0x5068`, `data3 = 0x46ad`, `data4 = [0xb3, 0x2d, 0xe3, 0x16, 0xf5, 0xe0, 0x69, 0xba]`).
   - `btAddr = mac_string_to_long(&addr)?`
4. **Device Enumeration**:
   - `BluetoothFindFirstRadio` -> `BluetoothFindNextRadio` -> `BluetoothFindRadioClose`
   - `BLUETOOTH_DEVICE_SEARCH_PARAMS` with `fReturnConnected = 1`, `cTimeoutMultiplier = 15`, `hRadio = radio`
   - `BluetoothFindFirstDevice` -> `BluetoothFindNextDevice` -> `BluetoothFindDeviceClose`
   - UTF-16 `szName` converted to `String` via `String::from_utf16_lossy` trimming trailing `\0`.
   - `Address.Anonymous.rgBytes` formatted via `mac_bytes_to_string`.

## Semantic Traps
- `closesocket` and `shutdown(SD_BOTH)` must be invoked cleanly on `disconnect()` and `Drop`.
- Because Windows `recv` blocks by default, `poll_incoming` / background listeners should handle timeout or disconnect cleanly without panicking.
- All `unsafe` FFI calls must be localized in `WindowsBluetoothConnector` and documented with `// SAFETY:` comments for `rust-anything gate`.
