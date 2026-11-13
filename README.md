# TimeConvert Time Zone Converter

TimeConvert is a compact, open-source Windows and Linux desktop app for comparing times across two or more time zones. Row A is the input: use its Enter button or press Enter in its date or time field to convert, or use Now to use the current time. All other rows display the converted results. Add or remove zones, swap A and B, optionally keep the window on top, and show analog clocks for the selected time.

The timezone picker searches by country, city, or IANA zone ID. Conversions account for daylight saving time and show a note for ambiguous or nonexistent local times. Enter 24-hour times with a colon (`09:30`) or without one (`930` or `1930`).

## Download

Download the latest Windows ZIP or Linux x86_64 archive from [GitHub Releases](https://github.com/KDAMielitz/TimeConvert/releases). The first launch of a new unsigned build may be slow while Windows scans it.

## Build from source

Install the Rust toolchain, then run:

```sh
cargo run --release
```

On Debian/Ubuntu, install build dependencies first:

```sh
sudo apt-get install pkg-config libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libgl1-mesa-dev libx11-dev libxi-dev libxcursor-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

The executable is `target/release/timeconvert.exe` on Windows and `target/release/timeconvert` on Linux. Run tests with `cargo test --locked`. To install a Linux desktop shortcut, place the binary on your `PATH`, copy `timeconvert.desktop` to `~/.local/share/applications/`, and copy `timeconvert.png` to `~/.local/share/icons/hicolor/256x256/apps/`. For a source checkout, the icon is `assets/icon.png`. Always-on-top depends on the window manager and may not be honored on Wayland.

Settings are saved to `%APPDATA%\TimeConvert\settings.json` on Windows and `$XDG_CONFIG_HOME/timeconvert/settings.json` (or `~/.config/timeconvert/settings.json`) on Linux. The bundled `zone.tab` and `iso3166.tab` files are public-domain data from the IANA time zone database.

The application icon is embedded at build time. To regenerate it, install Python and Pillow, then run `python tools/generate_icon.py`.

## Releasing

Versions use `major.milestone.production.testing`: the first two numbers identify major and smaller milestones; the third identifies a production release; the fourth is a testing revision. Production tags end in `.0` (for example, `v1.2.1.0`); testing tags use a nonzero revision and `a` for experimental alpha or `b` for relatively safe beta (for example, `v1.2.1.4a` or `v1.2.1.5b`). Testing builds are Actions artifacts for 14 days and do not create a GitHub Release. Production builds publish automatically.

Update the three-part `Cargo.toml` version and `Cargo.lock` when the production version changes, then commit and push the matching tag:

```sh
git tag v1.2.1.0
git push origin v1.2.1.0
```

See [versioning details](docs/VERSIONING.md), including the Windows executable version resource.

## Privacy

TimeConvert does not collect, transmit, or share any data and makes no network connections. It only saves your selected time zones and display options in a local settings file. See [PRIVACY.md](PRIVACY.md).

## License

Copyright (c) 2026 Boozted Hosting LLC and Kaden Mielitz.

Licensed under either the [MIT License](LICENSE-MIT) or the [Apache License, Version 2.0](LICENSE-APACHE), at your option. Unless stated otherwise, contributions are dual-licensed under the same terms.
