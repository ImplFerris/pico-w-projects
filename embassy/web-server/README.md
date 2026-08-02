# Pico W Template

This template provides a starting point for Raspberry Pi Pico W projects using Embassy.

It is part of the **Embedded Rust with Raspberry Pi Pico** book:

[https://rp2040.implrust.com/](https://rp2040.implrust.com/)

Create a new project with:

```bash
cargo generate --git https://github.com/ImplFerris/pico-w-template.git --tag TAG_VALUE
```

For example:

```bash
cargo generate --git https://github.com/ImplFerris/pico-w-template.git --tag v0.1.1
```

## Building

Set your Wi-Fi credentials before building.

```bash
SSID=YOUR_WIFI PASSWORD=YOUR_WIFI_PASSWORD cargo build
```

Alternatively, export the variables in your shell before running `cargo build`.
