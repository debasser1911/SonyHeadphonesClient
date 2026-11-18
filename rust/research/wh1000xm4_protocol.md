# WH-1000-XM4 Sony MDR Protocol v1 Research

## Sources
- Local repo: `Client/Constants.h`, `Client/CommandSerializer.cpp`, `Client/BluetoothWrapper.cpp`, `Client/Headphones.cpp`
- Upstream PR #103 (`Plutoberth/SonyHeadphonesClient` - "Added various features for WH-1000XM4")
- Community implementations (`mos9527/SonyHeadphonesClient`, `ilyaszk/xm4-control`)

## Transport & Framing
- Service UUID: `96CC203E-5068-46ad-B32D-E316F5E069BA` (RFCOMM SPP)
- Frame wire layout:
  `[0x3E '>'] ESCAPE([data_type: u8][seq_number: u8][payload_len_be: u32][payload: ...][checksum: u8]) [0x3C '<']`
- Byte escaping (`0x3D` sentry):
  - `0x3C` (60) -> `0x3D 0x2C` (61, 44)
  - `0x3D` (61) -> `0x3D 0x2D` (61, 45)
  - `0x3E` (62) -> `0x3D 0x2E` (61, 46)
- Checksum: `sum(unescaped_bytes_before_checksum) & 0xFF`.
- Sequence number: alternates `0x00` / `0x01`. Client tracks the sequence number reported in incoming ACKs (`seq`), and when ACKing incoming `DATA_MDR`/`DATA_MDR_NO2` packets, responds with `DataType::Ack` (`0x01`), empty payload, and `seq ^ 0x01`.

## Bug in Original `unpackBtMessage`
In `Client/CommandSerializer.cpp:139-152`, `_unescapeSpecials(src)` returns `unescaped`, but lines 141-152 inspect `src` instead of `unescaped`. Whenever an XM4 packet contains `0x3C`, `0x3D`, or `0x3E` (common in multipoint MAC strings/lengths/capabilities), the checksum check fails with `"Invalid checksum!"`.
**Fix**: Check length >= 7, checksum, `dataType`, `seqNumber`, and big-endian payload length on `unescaped`.

## WH-1000-XM4 Specific Commands & Differences from XM3
1. **NC / ASM (`COMMAND_TYPE::NCASM_SET_PARAM = 0x68`)**:
   - WH-1000-XM3 uses `MAX_STEPS = 19` (`0..=19`, where 0 = Dual NC, 1 = Single/Wind NC, 2..=19 = Ambient Sound levels).
   - WH-1000-XM4 supports `MAX_STEPS = 20` (`0..=20`: 0 = NC On, 1 = Wind Noise Reduction, 2..=20 = Ambient Sound 1..20 or slider 0..=20 per PR #103 review). Using `max_steps = 20` for WH-1000-XM4 prevents `"Exceeded max steps"` errors at step 20.
   - Query/Response/Notify opcodes: `0x66` (`NCASM_GET_PARAM`), `0x67` (`NCASM_RET_PARAM`), `0x69` (`NCASM_NTFY_PARAM`).
2. **Initialization & Capabilities (`DATA_MDR`)**:
   - Init: `[0x00, 0x00]`, MAC query: `[0x02, 0x00]`
   - Capability query: `[0x06, 0x01]`, response opcode `0x07` containing `FUNCTION_TYPE` bytes:
     - `0x38` (56): `DEVICE_MANAGEMENT` -> `MULTI_POINT`
     - `0x41` (65): `VPT` -> `VPT`
     - `0x81` (-127): `OPTIMIZER` -> `OPTIMIZER`
     - `0x62` (98): `NC_ASM` -> `NC_ASM`
     - `0xF5` (-11): `SMART_TALKING_MODE` -> `SPEAK_TO_CHAT`
3. **ANC Optimizer (`0x84` / `0x85`, `DATA_MDR`)**:
   - Set: `[0x84, 0x01, 0x00, state]` (`0x00` = Idle, `0x01` = Optimizing).
   - Response (`0x85`): `bytes[2] != 0` indicates optimizing.
4. **Speak-to-Chat (`0xF6`..`0xFD`, `DATA_MDR`)**:
   - Toggle query/response/set/notify: `0xF6`, `0xF7`, `0xF8`, `0xF9`.
     - Set payload: `[0xF8, 0x05, 0x01, toggle]` (`0x00` = Inactive, `0x01` = Active).
   - Options query/response/set/notify: `0xFA`, `0xFB`, `0xFC`, `0xFD`.
     - Set payload: `[0xFC, 0x05, 0x00, sensitivity, voice_passthrough, off_time]`.
     - Sensitivity: `0` = Auto, `1` = High, `2` = Low.
     - Auto-close time: `0` = Short, `1` = Standard, `2` = Long, `3` = Off.
5. **Multipoint Device Management (`DATA_MDR_NO2 = 0x0E`)**:
   - Query: `[0x36, 0x01]` on `DataType::DataMdrNo2`.
   - Response (`0x37` / `0x39`): `[cmd, 0x01, num_devices, (mac[17], slot_number, name_len, name[name_len])*]`.
   - Set (`0x3C`): `[0x3C, 0x01, 0x00, action, mac_ascii_bytes...]` (`0x00` = Disconnect, `0x01` = Connect, `0x02` = Unpair) on `DataType::DataMdrNo2`.
