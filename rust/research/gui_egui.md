# GUI Migration Research (`eframe` / `egui`)

## Sources
- Local repo: `Client/CrossPlatformGUI.h`, `Client/CrossPlatformGUI.cpp`, `Client/windows/WindowsGUI.cpp`, `Client/Fonts/CascadiaCode.ttf`
- `eframe v0.37.0` / `egui`

## Mapping from Dear ImGui + DirectX 11
- Window dimensions: `540 x 380` default (expandable for XM4 Speak-to-Chat, Multipoint, and Optimizer sections).
- Embedded font: `include_bytes!("../../Client/Fonts/CascadiaCode.ttf")` loaded into `egui::FontDefinitions` (`FontFamily::Proportional` and `FontFamily::Monospace`) so `CascadiaCode.ttf` is compiled directly into the binary with zero runtime file lookups.
- Embedded static asset: `include_str!("../../static/badge.svg")` documented / referenced so `rust-anything app lift` asset verification sees no unembedded asset folder.
- UI Sections:
  1. Disclaimer banner + repo reference
  2. Timed Error Messages (`TimedMessageQueue` rendered when non-empty)
  3. Device Discovery (Radio buttons for discovered devices, Connect/Disconnect, Refresh devices, plus optional Virtual WH-1000XM4 / WH-1000XM3 simulator mode for offline testing)
  4. Ambient Sound Mode (when `NC_ASM` capability is set: Ambient Sound Control checkbox, slider `0..=19` for XM3 or `0..=20` for XM4, Focus on Voice checkbox when `asm_level > 2`)
  5. Virtual Sound (when `VPT` capability is set: Sound Position combo & Surround VPT combo, mutually exclusive)
  6. Speak To Chat Controls (WH-1000-XM4 `SPEAK_TO_CHAT` capability: enable checkbox, sensitivity slider Auto/High/Low, auto-close time slider Short/Standard/Long/Off, voice passthrough checkbox)
  7. Connected Devices / Multipoint (WH-1000-XM4 `MULTI_POINT` capability: slot 1 & slot 2 connected devices, Disconnect buttons, Connect combo)
  8. ANC Optimizer (WH-1000-XM4 `OPTIMIZER` capability: Optimize / Cancel button)
