# ble-touch

ESP32-C3 BLE HID touch bridge. Receives JSON commands over serial (CDC-ACM), synthesizes touch gestures, and sends them as HID reports over Bluetooth Low Energy via the HID-over-GATT-Profile (HOGP) service.

## What It Does

Acts as a programmable touch screen for any BLE HID host (phone, tablet, PC). You send gesture commands over serial from a host controller app, and the ESP32 presents them as real touchscreen input to the paired device.

### Serial Command Format

```json
{"cmd":"tap","x":512,"y":768}
{"cmd":"swipe","x1":100,"y1":3000,"x2":100,"y2":100}
{"cmd":"long_press","x":512,"y":768","duration_ms":1000}
{"cmd":"drag","points":[{"x":100,"y":2000},{"x":900,"y":2000}]}
{"cmd":"config","window":{"width":1080,"height":1920}}
{"cmd":"get_settings"}
```

Full command schema: see [`src/cmd.rs`](src/cmd.rs).

### Coordinate Pipeline

Source pixels → scale/offset transform → physical screen pixels → normalized 0–4095 HID range. Configurable via `window` and `screen` settings.

## Architecture

Single Rust package with implicit library + explicit binary:

| File | Role |
|------|------|
| `src/lib.rs` | Pure logic — commands, coordinates, gestures, HID reports, settings (testable on any host) |
| `src/main.rs` | ESP32 entry point (conditional on `esp32` feature) |
| `src/ble.rs` | NimBLE stack init + HOGP GATT service |
| `src/serial.rs` | Serial I/O, JSON command parsing loop |
| `src/storage.rs` | NVS flash persistence (stubbed, pending) |

### Module Breakdown

- **`cmd`** — JSON command enum with serde tagged serialization (`{"cmd":"tap",...}`)
- **`coords`** — Source-to-HID coordinate transformation pipeline
- **`gesture`** — Gesture synthesis (tap, swipe, long press, drag) with interpolation
- **`hid`** — Single-touch HID descriptor + 8-byte report builder
- **`settings`** — Device config with per-field PATCH merge semantics

## Building

### Prerequisites

- Rust toolchain (`esp` channel via `rustup`)
- ESP-IDF v5.3.4 toolchain (installed via `espup`)
- System deps: `flex`, `bison`, `gperf`, `ninja-build`

### Host Testing

```bash
cargo test --target x86_64-unknown-linux-gnu
```

49 unit tests cover coordinate transforms, gesture synthesis, HID report building, settings merge, and command parsing.

### ESP32-C3 Build

```bash
cargo build --features esp32
```

Flashed via `espflash` (configured as cargo runner in `.cargo/config.toml`).

### Known Dependency Patch

This project uses a forked version of `esp-idf-sys` (branch `patched-main`) to fix missing NimBLE host API headers for ESP-IDF v5.3.x. See [`Cargo.toml`](Cargo.toml) `[patch.crates-io]` section. Fork: https://github.com/RichardHopperProGrammar/esp-idf-sys

## Hardware

- **MCU:** ESP32-C3 (RISC-V, 400 MHz)
- **BLE:** NimBLE HOGP (HID Device profile, single-touch screen)
- **Serial:** CDC-ACM over USB (115200 baud)
- **Storage:** NVS (stubbed, pending implementation)

## Pairing

BLE pairing uses a fixed passkey (`123456`). Initiate via serial:

```json
{"cmd":"pair"}
```

Use `{"cmd":"list_bonds"}` to see bonded devices, `{"cmd":"unpair","address":"XX:XX:XX:XX:XX:XX"}` to remove.

## Status

- ✅ Library logic (tested, 49/49 pass)
- ✅ ESP32 build compiles clean
- ✅ BLE stack initialization
- ✅ Serial command loop
- ✅ HOGP GATT service construction (BLEHIDDevice helper)
- ✅ NVS flash storage (EspDefaultNvsPartition + EspNvs)
- ✅ HID report sending over BLE (input characteristic notify)
- 🚧 Hardware testing (pending ESP32-C3 board)
- 📋 HOGP descriptor validation (needs BLE scanner/analysis tool)
