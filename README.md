# renamer-tui

A keyboard-driven terminal UI for batch-renaming video series files. Point it at
a folder, preview the rename, confirm — done.

![Rust](https://img.shields.io/badge/rust-2024-orange?logo=rust)
![License](https://img.shields.io/badge/license-MIT-blue)

## What it does

Scans a folder for video files (`.mp4`, `.mkv`, `.avi`, …) and subtitle files
(`.srt`, `.ass`, `.vtt`, …), strips codec and quality tags from the filenames
(`1080p`, `x265`, `BluRay`, `WEBRip`, etc.), and reduces each name to just its
episode number and extension.

```sh
My.Show.S01E03.1080p.BluRay.x265.mkv  →  3.mkv
Series Name - 12 [WEBRip][AAC].mp4    →  12.mp4
episode.07.srt                         →  7.srt
```

Files are sorted numerically before display. Files that can't be parsed are
flagged with ⚠️ and left untouched.

## Install

### Pre-built binaries (recommended)

Download the latest release for your platform from the [Releases page](../../releases/latest).

#### Linux x86_64

```bash
tar xzf renamer-tui-<version>-x86_64-unknown-linux-gnu.tar.gz
sudo mv renamer-tui-<version>-x86_64-unknown-linux-gnu /usr/local/bin/renamer-tui
```

#### macOS (Apple Silicon)

```bash
tar xzf renamer-tui-<version>-aarch64-apple-darwin.tar.gz
sudo mv renamer-tui-<version>-aarch64-apple-darwin /usr/local/bin/renamer-tui
```

#### macOS (Intel)

```bash
tar xzf renamer-tui-<version>-x86_64-apple-darwin.tar.gz
sudo mv renamer-tui-<version>-x86_64-apple-darwin /usr/local/bin/renamer-tui
```

#### Windows x86_64

Extract `renamer-tui-<version>-x86_64-pc-windows-msvc.zip` and
place `renamer-tui.exe` somewhere on your `PATH`.

Each archive includes a `.sha256` checksum file you can use to verify the download.

### From source

Requires Rust 1.85+ (edition 2024).

```bash
cargo install --path .
# or
cargo build --release && ./target/release/renamer-tui
```

## Usage

```md
renamer-tui
```

| Key                        | Action                                      |
| -------------------------- | ------------------------------------------- |
| `Ctrl+O`                   | Open native folder picker                   |
| `Enter` (path bar)         | Scan the typed path                         |
| `Tab`                      | Switch focus between path bar and file list |
| `↑ / ↓` or `j / k`         | Navigate files                              |
| `Space`                    | Toggle skip or selected file                |
| `Enter` or `r` (file list) | Open rename confirmation                    |
| `y` / `Enter`              | Confirm rename                              |
| `n` / `Esc`                | Cancel                                      |
| `q` / `Ctrl+C`             | Quit                                        |

## Dependencies

| Crate                                                  | Purpose                           |
| ------------------------------------------------------ | --------------------------------- |
| [ratatui](https://ratatui.rs)                          | TUI framework                     |
| [crossterm](https://github.com/crossterm-rs/crossterm) | Cross-platform terminal backend   |
| [tui-input](https://github.com/sayanarijit/tui-input)  | Text input widget with cursor     |
| [rfd](https://github.com/PolyMeilex/rfd)               | Native async folder picker dialog |
| [tokio](https://tokio.rs)                              | Async runtime                     |
| [color-eyre](https://github.com/eyre-rs/color-eyre)    | Error reporting                   |

## License

Copyright (c) Dantescur <daniel@thedaniweb.eu.org>

Licensed under the [MIT license](./LICENSE).
