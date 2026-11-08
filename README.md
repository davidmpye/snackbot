
**Snackbot** is Makerspace's vending machine. It started life as a Polyvend
machine with a dead original controller (or at least it died while being experimented with...)
it's now a hybrid of a Raspberry Pi (running the user interface) and two RP2040 Pi Pico boards (driving the
hardware), all written in Rust.

This repository contains everything needed to reproduce the conversion:
firmware, PCB designs, the Pi-side application, and the 3D-printed / laser-cut
fixings.

Of note, it supports an MDB credit card reader, allowing contactless card payments

> 📷 *Photos to follow.*

---

## Architecture

```
                 ┌──────────────────────────────────────────┐
                 │ Raspberry Pi + Hyperpixel 480x800 display │
                 │          vmc-host (GTK4 app)              │
                 └───────────┬──────────────────┬───────────┘
                     USB HID │ + postcard-rpc   │ USB postcard-rpc
                             │                  │
            ┌────────────────┴───┐      ┌───────┴────────────────────────┐
            │  matrix-keyboard   │      │       vmc (RP2040 + PCB)        │
            │  (RP2040 + PCB)    │      │                                 │
            │                    │      │  • dispenser motor matrix       │
            │  • 3x8 keypad →    │      │  • MDB bus: coin acceptor &     │
            │    USB keyboard    │      │    cashless (card) reader       │
            │  • optional I2C    │      │  • chiller control + thermistor │
            │    text LCD        │      │  • hardware watchdog            │
            └────────────────────┘      └─────────────────────────────────┘
```

1. The customer types a selection on the machine's original membrane keypad
   (e.g. `A` then `4`). The **matrix-keyboard** Pico presents this to the Pi as
   an ordinary USB keyboard.
2. **vmc-host** shows the item, picture and price on the display and asks for
   confirmation (✔ / ✖).
3. On confirmation the host sends a `Vend` command to the **vmc** Pico, which
   collects payment over MDB (coins or contactless), drives the appropriate
   spiral/can motor, checks for a successful drop, and reports back.
4. The host shows success/failure and returns to idle.

The two Picos talk to the Pi using [postcard-rpc](https://github.com/jamesmunns/postcard-rpc)
over USB. The wire protocols are defined in small shared `*-icd` crates so the
host and firmware can never disagree about message formats.

---

## Repository layout

| Path | What it is |
|------|------------|
| [`vmc/fw`](vmc/fw) | **Vending Machine Controller firmware** – `no_std` Rust on RP2040 using [Embassy](https://embassy.dev). Drives motors, MDB peripherals, chiller and watchdog; exposes a postcard-rpc server over USB. |
| [`vmc/vmc-icd`](vmc/vmc-icd) | Interface Control Document crate for the VMC – shared request/response types and endpoint definitions. |
| [`vmc/pcb`](vmc/pcb) | KiCad project for the VMC board (power, level shifters, MDB interface, motor driver). Gerber job file included. |
| [`matrix-keyboard/fw`](matrix-keyboard/fw) | Firmware for the keypad board – USB HID keyboard + postcard-rpc endpoints for an optional I2C LCD. |
| [`matrix-keyboard/keyboard-icd`](matrix-keyboard/keyboard-icd) | ICD crate for the keypad board. |
| [`matrix-keyboard/pcb`](matrix-keyboard/pcb) | KiCad project for the keypad interface board (includes 3V3→5V level shifting for 5V LCDs). |
| [`vmc-host`](vmc-host) | The GTK4 application that runs on the Raspberry Pi – UI, state machine, stock list, and USB clients for both Picos. |
| [`fixings`](fixings) | OpenSCAD sources for the display mount, retaining tabs, LCD frame and laser-cut acrylic windows. See [`fixings/README.md`](fixings/README.md). |

---

## Components in detail

### `vmc` – Vending Machine Controller

The VMC firmware runs a set of Embassy tasks:

- **Motor driver** – drives the dispenser motor matrix via an 8-bit data bus
  plus three clock lines, OE and CLR (GPIO 0–12). Detects home position,
  stuck motors, "one or no cans left" on the can vendor, and whether a drop
  was detected.
- **MDB (Multi-Drop Bus)** – the 9-bit serial bus used by vending peripherals,
  implemented with a PIO-backed 9-bit UART on GPIO 20/21. Two poll tasks handle
  the **coin acceptor** and a **cashless (card) device**.
- **Chiller** – reads a thermistor on ADC0 (GPIO 26) and publishes
  `ChillerStatus` to the host.
- **Watchdog** – hardware watchdog with a heartbeat LED on GPIO 22.
- **USB** – enumerates as a composite device, manufacturer `Snackbot`,
  product `vmc` (VID `0xDEAD`, PID `0xBEEF`).

The postcard-rpc API (from `vmc-icd`):

| Endpoint | Request | Response | Purpose |
|----------|---------|----------|---------|
| `/vmc/itemavailable` | `VendCommand` | `VendResult` | Check an item can be vended |
| `/vmc/vend` | `VendCommand` | `VendResult` | Take payment, then dispense |
| `/vmc/forcedispense` | `VendCommand` | `VendResult` | Dispense **without charging** (maintenance) |
| `/vmc/cancelvend` | `()` | `VendResult` | Cancel an in-progress vend *(not yet implemented)* |
| `/vmc/collectpayment` | `VendCommand` | `VendResult` | Charge without dispensing – lets people pay for e.g. filament |

| Topic (device → host) | Payload |
|-----------------------|---------|
| `/vmc/vend_progress` | `AwaitingPayment` / `Dispensing` |
| `/vmc/status/chiller` | `ChillerStatus` |

`VendCommand` is `{ row: u8, col: u8, price: u16 }` with the price in unscaled
pence. Failures are reported as a `VendError` (`MotorNotPresent`,
`MotorNotHome`, `MotorStuckHome`, `MotorStuckNotHome`, `OneOrNoCansLeft`,
`NoDropDetected`, `InvalidAddress`, `Cancelled`, `PaymentFailed`, `CommsFault`).

### `matrix-keyboard` – keypad interface

A small PCB that plugs into the vending machine's original 3×8 matrix keypad
header. The firmware scans the matrix and appears to the Pi as a **USB HID
keyboard**, so no custom input driver is needed on the Pi side.

It also exposes postcard-rpc endpoints for an optional **I2C character LCD**
(useful as a cheap secondary display, or as the *only* display if you don't
fit the Hyperpixel):

| Endpoint | Request | Purpose |
|----------|---------|---------|
| `setText` | `[[u8; 32]; 2]` | Set two 32-character lines of text |
| `setBacklight` | `bool` | Backlight on/off |

Topic `serviceMode` (`bool`) is published to the host.

### `vmc-host` – the Pi application

A GTK4 app (`uk.org.makerspace.snackbot`) sized for a 480×800 portrait display
(Pimoroni Hyperpixel 4). It is a simple event-driven state machine:

```
Idle ─▶ AwaitingConfirmation ─▶ AwaitingPayment ─▶ Dispensing ─▶ VendSuccess
  ▲            │                      │                 │              │
  │            ✖ cancel               ✖ cancel          └──▶ VendFailed │
  └────────────┴──────────────────────┴───────────── timeout / 2s ─────┘
```

- Letters `A`–`H` select a row, digits `0`–`9` a column; `Return` is ✔ confirm,
  `Escape` is ✖ cancel.
- Returns to idle automatically after 30 s of inactivity.
- Each screen is a GTK widget in a `Stack` (`make_selection_box`,
  `confirm_item_box`, `make_payment_box`, `vend_in_progress_box`, etc.).
- The stock list (name, image, price, row/col) currently lives in
  [`vmc-host/src/stock_info.rs`](vmc-host/src/stock_info.rs); product images
  are in `vmc-host/src/images/`.
- `rpc_shim.rs` spawns a Tokio runtime hosting the postcard-rpc clients for the
  VMC and the LCD, bridged to the GLib main loop with `async-channel`.

### `fixings` – mechanical parts

OpenSCAD sources for:

- `hyperpixel_mount.scad` / `hyperpixel_retaining_tabs.scad` – 3D-printed
  bracket and tabs to hold the Pi + Hyperpixel behind the fascia.
- `hyperpixel_lcd_window.scad` – laser-cut 3 mm clear acrylic window.
- `i2c_lcd_frame.scad` / `i2c_lcd_window.scad` – frame and window for the
  optional I2C text display.

---

## Building

### Prerequisites

- Rust toolchain (the firmware crates pin their version via
  `rust-toolchain.toml`; `rustup` will pick it up automatically).
- For firmware: the `thumbv6m-none-eabi` target and
  [`probe-rs`](https://probe.rs) (or `elf2uf2-rs` if you'd rather flash over
  USB bootloader).
- For the host: GTK4 development libraries (`libgtk-4-dev` on
  Debian/Raspberry Pi OS) and `libudev`.

### Firmware (either Pico)

```bash
cd vmc/fw            # or matrix-keyboard/fw
cargo run --release  # flashes via probe-rs (see .cargo/config.toml)
```

To produce a UF2 instead, swap the runner in `.cargo/config.toml` for the
commented-out `elf2uf2-rs -d` line, hold BOOTSEL while plugging in the Pico,
and `cargo run --release` again.

Logging is via `defmt` over RTT; set `DEFMT_LOG=debug` for more chatter.

The VMC firmware depends on two companion crates by the same author:
[`mdb-async`](https://github.com/davidmpye/mdb-async) and
[`pio-9bit-uart-async`](https://github.com/davidmpye/pio-9bit-uart-async).

### Host application

```bash
cd vmc-host
cargo build --release
./target/release/vmc-host
```

The app opens USB devices by product string (`vmc` and `matrix-keyboard`), so
you'll want a udev rule granting your user access to them, e.g.:

```
# /etc/udev/rules.d/99-snackbot.rules
SUBSYSTEM=="usb", ATTR{idVendor}=="dead", ATTR{idProduct}=="beef", MODE="0666"
```

(adjust for the keyboard board's VID/PID as well), then
`sudo udevadm control --reload && sudo udevadm trigger`.

For a kiosk deployment, run it under a minimal Wayland/X session on boot with
the Hyperpixel in portrait orientation.

### Hardware

Open the `.kicad_pro` files in `vmc/pcb` and `matrix-keyboard/pcb` with
KiCad 8+. Render the `.scad` files in `fixings/` with OpenSCAD and export STL
(for printing) or DXF (for laser cutting).

---

## Status / TODO

- [ ] `CancelVend` endpoint on the VMC
- [ ] Move stock list out of `stock_info.rs` into a config file
- [ ] Hook up `ItemAvailable` / `CollectPaymentOnly` in the host UI
- [ ] Photos and wiring diagrams
- [ ] Chiller status display on the host

---

## License

GPL-3.0 – see [LICENSE](LICENSE).
