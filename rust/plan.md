# Rust migration plan: SonyHeadphonesClient (C)

2 packages, 45 source files (12407 lines), 0 test files, 0 UI files, grouped into 19 units.

| Ready | Caution | Blocked | Keep in source language |
| ---: | ---: | ---: | ---: |
| 2 | 15 | 0 | 2 |

> 40 files had parse errors; their units are marked partial.

## Start here

No unit is a safe first port yet. See the blocked units and dependencies below.

## Order by package

Packages come after the workspace packages they use. Inside a package, port by wave: a unit's imports are in earlier waves.

### 1. Client (`Client`, portable, 0 test files)

| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 0 | `Client/CascadiaCodeFont.h` | ready | 4 | 98 | C ABI | - |
| 0 | `Client/Exceptions.h` | caution | 10 | 87 | C ABI | - |
| 0 | `Client/Constants.h` | caution | 111 | 86 | C ABI | - |
| 0 | `Client/imgui/misc/freetype/` | caution | 43 | 76 | C ABI | Pointers and manual memory (9); Integer width and overflow (10) |
| 0 | `Client/imgui/misc/cpp/` | caution | 17 | 75 | C ABI | Pointers and manual memory (10); Null references (6) |
| 0 | `Client/SingleInstanceFuture.h` | caution | 45 | 72 | C ABI | Pointers and manual memory (3) |
| 0 | `Client/TimedMessageQueue.h` | keep | 39 | 69 | C ABI | Platform-specific C (asm, volatile, bitfields, packing) (1) |
| 0 | `Client/windows/` | caution | 60 | 46 | C ABI | Pointers and manual memory (3) |
| 0 | `Client/imgui/backends/` | caution | 565 | 44 | C ABI | Pointers and manual memory (66); Division and modulo (6) |
| 0 | `Client/imgui/` | keep | 11180 | 0 | C ABI | Pointers and manual memory (2153); Platform-specific C (asm, volatile, bitfields, packing) (17) |
| 1 | `Client/imgui/misc/single_file/` | ready | 16 | 100 | C ABI | - |
| 1 | `Client/IBluetoothConnector.h` | caution | 39 | 76 | C ABI | Pointers and manual memory (2); Integer width and overflow (1) |
| 1 | `Client/ByteMagic.h` | caution | 13 | 74 | C ABI | - |
| 2 | `Client/CommandSerializer.h` | caution | 38 | 80 | C ABI | Pointers and manual memory (1) |
| 2 | `Client/linux/` | caution | 35 | 62 | C ABI | Pointers and manual memory (17) |
| 2 | `Client/macos/` | caution | 75 | 54 | C ABI | Pointers and manual memory (20); Integer width and overflow (1) |
| 3 | `Client/BluetoothWrapper.h` | caution | 29 | 78 | C ABI | Integer width and overflow (1) |
| 4 | `Client/Headphones.h` | caution | 47 | 93 | C ABI | - |
| 5 | `Client/CrossPlatformGUI.h` | caution | 41 | 91 | C ABI | Integer width and overflow (1); Division and modulo (1) |

## system libraries dependencies

| Package | Units | Rust | Fit | Note |
| --- | ---: | --- | --- | --- |
| `<string>` | 7 | ? | unknown |  |
| `<vector>` | 6 | ? | unknown |  |
| `<stdexcept>` | 4 | ? | unknown |  |
| `<atomic>` | 3 | ? | unknown |  |
| `<future>` | 3 | ? | unknown |  |
| `<mutex>` | 3 | ? | unknown |  |
| `<BluetoothAPIs.h>` | 1 | ? | unknown |  |
| `<algorithm>` | 1 | ? | unknown |  |
| `<chrono>` | 1 | ? | unknown |  |
| `<condition_variable>` | 1 | ? | unknown |  |
| `<cstddef>` | 1 | ? | unknown |  |
| `<cstdint>` | 1 | ? | unknown |  |
| `<d3d11.h>` | 1 | ? | unknown |  |
| `<dbus/dbus.h>` | 1 | ? | unknown |  |
| `<deque>` | 1 | ? | unknown |  |
| `<dinput.h>` | 1 | ? | unknown |  |
| `<functional>` | 1 | ? | unknown |  |
| `<iomanip>` | 1 | ? | unknown |  |
| `<iostream>` | 1 | ? | unknown |  |
| `<memory>` | 1 | ? | unknown |  |
| `<rpc.h>` | 1 | ? | unknown |  |
| `<sstream>` | 1 | ? | unknown |  |
| `<tchar.h>` | 1 | ? | unknown |  |
| `<thread>` | 1 | ? | unknown |  |
| `<utility>` | 1 | ? | unknown |  |
| `<vulkan/vulkan.h>` | 1 | ? | unknown |  |
| `<webgpu/webgpu.h>` | 1 | ? | unknown |  |
| `<winsock2.h>` | 1 | ? | unknown |  |
| `<ws2bth.h>` | 1 | ? | unknown |  |

## Risk guide

- **String offsets and slicing** (High): JS indexes strings in UTF-16 code units, Rust in UTF-8 bytes. Offsets that cross the boundary must be converted, or non-ASCII text gets wrong ranges silently.
- **Dynamic code (eval, new Function, computed import)** (High): eval, new Function and computed imports have no Rust equivalent. Keep this part in JS or replace it with an explicit registry.
- **Reflection and prototypes** (Medium): Proxy, Reflect, Symbol and prototype tricks need a redesign, not a translation.
- **Integer width and overflow** (Medium): Python ints never overflow; C, Go and Java ints wrap or are undefined on overflow. Pick i64, i128 or a bignum crate deliberately, and use wrapping_* where the source wraps.
- **Division and modulo** (Medium): Python `//` and `%` floor toward negative infinity; Rust `/` and `%` truncate: use `div_euclid`/`rem_euclid` or explicit floor math. C division by zero is undefined.
- **Pointers and manual memory** (High): Pointer arithmetic and manual memory become slices, indices and owned types. Code that exposes pointers across the boundary needs `unsafe`, isolated and explained.
- **Null references** (Medium): Any reference may be null in the source: decide per parameter whether it is `Option` and what null means, instead of mapping every value to `Option`.
- **Platform-specific C (asm, volatile, bitfields, packing)** (High): This C depends on its target: register access, memory layout, the compiler's type sizes or its assembly. A recording on this machine says nothing about the target, and battle-tested driver or firmware code gains little from a rewrite: keep it in C and port the portable logic around it.

## Whole app

Shape: **cli**.

| Entry point | Kind | Found from |
| --- | --- | --- |
| `Client/imgui/imstb_truetype.h` | cli | `int main(` at Client/imgui/imstb_truetype.h:335 |

### Every dependency

| Dependency | Declared | Used by | Rust | Fit | Note |
| --- | --- | --- | --- | --- | --- |
| `<string>` | no | 8 | ? | unknown |  |
| `<vector>` | no | 7 | ? | unknown |  |
| `<stdexcept>` | no | 4 | ? | unknown |  |
| `<atomic>` | no | 3 | ? | unknown |  |
| `<future>` | no | 3 | ? | unknown |  |
| `<mutex>` | no | 3 | ? | unknown |  |
| `<chrono>` | no | 2 | ? | unknown |  |
| `<deque>` | no | 2 | ? | unknown |  |
| `<utility>` | no | 2 | ? | unknown |  |
| `<BluetoothAPIs.h>` | no | 1 | ? | unknown |  |
| `<IMGUI_USER_CONFIG>` | no | 1 | ? | unknown |  |
| `<algorithm>` | no | 1 | ? | unknown |  |
| `<condition_variable>` | no | 1 | ? | unknown |  |
| `<cstddef>` | no | 1 | ? | unknown |  |
| `<cstdint>` | no | 1 | ? | unknown |  |
| `<d3d11.h>` | no | 1 | ? | unknown |  |
| `<dbus/dbus.h>` | no | 1 | ? | unknown |  |
| `<dinput.h>` | no | 1 | ? | unknown |  |
| `<functional>` | no | 1 | ? | unknown |  |
| `<iomanip>` | no | 1 | ? | unknown |  |
| `<iostream>` | no | 1 | ? | unknown |  |
| `<limits>` | no | 1 | ? | unknown |  |
| `<memory>` | no | 1 | ? | unknown |  |
| `<rpc.h>` | no | 1 | ? | unknown |  |
| `<sstream>` | no | 1 | ? | unknown |  |
| `<tchar.h>` | no | 1 | ? | unknown |  |
| `<thread>` | no | 1 | ? | unknown |  |
| `<vulkan/vulkan.h>` | no | 1 | ? | unknown |  |
| `<webgpu/webgpu.h>` | no | 1 | ? | unknown |  |
| `<winsock2.h>` | no | 1 | ? | unknown |  |
| `<ws2bth.h>` | no | 1 | ? | unknown |  |

### Pieces, in port order

- wave 0: `Client/CascadiaCodeFont.h` (4 lines)
- wave 0: `Client/Constants.h` (111 lines)
- wave 0: `Client/Exceptions.h` (10 lines)
- wave 0: `Client/SingleInstanceFuture.h` (45 lines)
- wave 0: `Client/imgui/backends/` (565 lines)
- wave 0: `Client/imgui/misc/cpp/` (17 lines)
- wave 0: `Client/imgui/misc/freetype/` (43 lines)
- wave 0: `Client/windows/` (60 lines)
- wave 1: `Client/ByteMagic.h` (13 lines)
- wave 1: `Client/IBluetoothConnector.h` (39 lines)
- wave 1: `Client/imgui/misc/single_file/` (16 lines)
- wave 2: `Client/CommandSerializer.h` (38 lines)
- wave 2: `Client/linux/` (35 lines)
- wave 2: `Client/macos/` (75 lines)
- wave 3: `Client/BluetoothWrapper.h` (29 lines)
- wave 4: `Client/Headphones.h` (47 lines)
- wave 5: `Client/CrossPlatformGUI.h` (41 lines)

### Decisions for the user

**D1** (shape): The app is a cli, entered through Client/imgui/imstb_truetype.h. Port every entry point, in that shape?

> Yes: a clap front door (`app front-door`) takes every command over first

**D2** (keep Client/TimedMessageQueue.h): Client/TimedMessageQueue.h stays in the source language (Depends on the target (hardware registers, memory layout, inline assembly): keep in C, since a recording on this machine cannot vouch for the target). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D3** (keep Client/imgui/): Client/imgui/ stays in the source language (Depends on the target (hardware registers, memory layout, inline assembly): keep in C, since a recording on this machine cannot vouch for the target). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D4** (dependency <string>): Which crate replaces <string> (used by 8 units)?

> Research it against primary sources into rust/research/<string>.md before porting what uses it

**D5** (dependency <vector>): Which crate replaces <vector> (used by 7 units)?

> Research it against primary sources into rust/research/<vector>.md before porting what uses it

**D6** (dependency <stdexcept>): Which crate replaces <stdexcept> (used by 4 units)?

> Research it against primary sources into rust/research/<stdexcept>.md before porting what uses it

**D7** (dependency <atomic>): Which crate replaces <atomic> (used by 3 units)?

> Research it against primary sources into rust/research/<atomic>.md before porting what uses it

**D8** (dependency <future>): Which crate replaces <future> (used by 3 units)?

> Research it against primary sources into rust/research/<future>.md before porting what uses it

**D9** (dependency <mutex>): Which crate replaces <mutex> (used by 3 units)?

> Research it against primary sources into rust/research/<mutex>.md before porting what uses it

**D10** (dependency <chrono>): Which crate replaces <chrono> (used by 2 units)?

> Research it against primary sources into rust/research/<chrono>.md before porting what uses it

**D11** (dependency <deque>): Which crate replaces <deque> (used by 2 units)?

> Research it against primary sources into rust/research/<deque>.md before porting what uses it

**D12** (dependency <utility>): Which crate replaces <utility> (used by 2 units)?

> Research it against primary sources into rust/research/<utility>.md before porting what uses it

**D13** (dependency <BluetoothAPIs.h>): Which crate replaces <BluetoothAPIs.h> (used by 1 unit)?

> Research it against primary sources into rust/research/<BluetoothAPIs.h>.md before porting what uses it

**D14** (dependency <IMGUI_USER_CONFIG>): Which crate replaces <IMGUI_USER_CONFIG> (used by 1 unit)?

> Research it against primary sources into rust/research/<IMGUI_USER_CONFIG>.md before porting what uses it

**D15** (dependency <algorithm>): Which crate replaces <algorithm> (used by 1 unit)?

> Research it against primary sources into rust/research/<algorithm>.md before porting what uses it

**D16** (dependency <condition_variable>): Which crate replaces <condition_variable> (used by 1 unit)?

> Research it against primary sources into rust/research/<condition_variable>.md before porting what uses it

**D17** (dependency <cstddef>): Which crate replaces <cstddef> (used by 1 unit)?

> Research it against primary sources into rust/research/<cstddef>.md before porting what uses it

**D18** (dependency <cstdint>): Which crate replaces <cstdint> (used by 1 unit)?

> Research it against primary sources into rust/research/<cstdint>.md before porting what uses it

**D19** (dependency <d3d11.h>): Which crate replaces <d3d11.h> (used by 1 unit)?

> Research it against primary sources into rust/research/<d3d11.h>.md before porting what uses it

**D20** (dependency <dbus/dbus.h>): Which crate replaces <dbus/dbus.h> (used by 1 unit)?

> Research it against primary sources into rust/research/<dbus-dbus.h>.md before porting what uses it

**D21** (dependency <dinput.h>): Which crate replaces <dinput.h> (used by 1 unit)?

> Research it against primary sources into rust/research/<dinput.h>.md before porting what uses it

**D22** (dependency <functional>): Which crate replaces <functional> (used by 1 unit)?

> Research it against primary sources into rust/research/<functional>.md before porting what uses it

**D23** (dependency <iomanip>): Which crate replaces <iomanip> (used by 1 unit)?

> Research it against primary sources into rust/research/<iomanip>.md before porting what uses it

**D24** (dependency <iostream>): Which crate replaces <iostream> (used by 1 unit)?

> Research it against primary sources into rust/research/<iostream>.md before porting what uses it

**D25** (dependency <limits>): Which crate replaces <limits> (used by 1 unit)?

> Research it against primary sources into rust/research/<limits>.md before porting what uses it

**D26** (dependency <memory>): Which crate replaces <memory> (used by 1 unit)?

> Research it against primary sources into rust/research/<memory>.md before porting what uses it

**D27** (dependency <rpc.h>): Which crate replaces <rpc.h> (used by 1 unit)?

> Research it against primary sources into rust/research/<rpc.h>.md before porting what uses it

**D28** (dependency <sstream>): Which crate replaces <sstream> (used by 1 unit)?

> Research it against primary sources into rust/research/<sstream>.md before porting what uses it

**D29** (dependency <tchar.h>): Which crate replaces <tchar.h> (used by 1 unit)?

> Research it against primary sources into rust/research/<tchar.h>.md before porting what uses it

**D30** (dependency <thread>): Which crate replaces <thread> (used by 1 unit)?

> Research it against primary sources into rust/research/<thread>.md before porting what uses it

**D31** (dependency <vulkan/vulkan.h>): Which crate replaces <vulkan/vulkan.h> (used by 1 unit)?

> Research it against primary sources into rust/research/<vulkan-vulkan.h>.md before porting what uses it

**D32** (dependency <webgpu/webgpu.h>): Which crate replaces <webgpu/webgpu.h> (used by 1 unit)?

> Research it against primary sources into rust/research/<webgpu-webgpu.h>.md before porting what uses it

**D33** (dependency <winsock2.h>): Which crate replaces <winsock2.h> (used by 1 unit)?

> Research it against primary sources into rust/research/<winsock2.h>.md before porting what uses it

**D34** (dependency <ws2bth.h>): Which crate replaces <ws2bth.h> (used by 1 unit)?

> Research it against primary sources into rust/research/<ws2bth.h>.md before porting what uses it

