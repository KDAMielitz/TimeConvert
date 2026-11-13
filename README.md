# Time Convert

A compact Windows desktop app for comparing times across two or more time zones. Row A is the input: use its Enter button or press Enter in its date or time field to convert, or use Now to use the current time. All other rows display the converted results. Add or remove zones, swap A and B, optionally keep the window on top, and show analog clocks for the selected time.

The timezone picker searches by country, city, or IANA zone ID. Conversions account for daylight saving time and show a note for ambiguous or nonexistent local times. Enter 24-hour times with a colon (`09:30`) or without one (`930` or `1930`).

## Build and run

Install the Rust toolchain, then run:

```sh
cargo run --release
```

The release executable is built at `target/release/timeconvert.exe`. Run tests with `cargo test`.

Settings are saved to `%APPDATA%\TimeConvert\settings.json`. The bundled `zone.tab` and `iso3166.tab` files are public-domain data from the IANA time zone database.

The application icon is embedded at build time. To regenerate it, install Python and Pillow, then run `python tools/generate_icon.py`.
