# breevy-rs

Fast, modern text expander compatible with Breevy `.bvy` files.

## What it is

- Zero-dependency `.bvy` parser (Breevy's XML plist format), writer/export.
- Matching engine with trigger-key and word-ending modes, boundary checks,
  case-insensitive matching with case-preserving expansion.
- Insertion macros: `%(Year) %(Month) %(Day) %(MonthName) %(WeekdayName)
  %(#Hour) %(Minute) %(Second) %(AMorPM) %(clipboard) %(cursor)` plus
  `%(AddDays 1)` / `%(SubMonths 2)` family (also Add/SubtractSeconds, Minutes,
  Hours, Years).
- Native GUI editor that mirrors Breevy's window (812x614): folder tree,
  snippet list, replacement editor, Apply/OK saving **in place** — folders,
  groups and unrelated records are preserved.
- CLI for inspection, testing and export.
- Windows build: global low-level keyboard hook (`--hook`) that erases the
  typed abbreviation and types the expansion with `SendInput`.
- Linux build: kernel-level evdev capture + uinput injection (`--hook`), works on
  X11, Wayland and consoles (no X libraries needed).
- macOS: core library + CLI only.

## GUI

```
breevy-rs                                   # opens the editor (abbreviations.bvy)
breevy-rs --gui /path/to/abbreviations.bvy
```

The editor reads the real `.bvy`, edits snippets, and writes changes back into
the original XML record (no flattening). Build without GUI:
`cargo build --release --no-default-features`.

## CLI

```
breevy-rs <file.bvy> [--list] [--match TEXT] [--trigger] [--export OUT.bvy]
breevy-rs --hook <file.bvy>        (Windows / Linux)
breevy-rs --gui <file.bvy>
```

`--match addr --trigger` shows what would be typed and how many characters
would be removed. `--export` writes a flattened `.bvy` (folders are not
preserved; export path only).

## Windows hook

1. Copy `breevy-rs.exe` somewhere permanent, e.g.
   `%LOCALAPPDATA%\BreevyRs\breevy-rs.exe`.
2. Your Breevy data file lives at
   `%APPDATA%\Breevy\abbreviations.bvy` (Breevy's own folder).
3. Run `breevy-rs.exe --hook "%APPDATA%\Breevy\abbreviations.bvy"`.
4. Type an abbreviation, then press **Ctrl** (trigger) or a delimiter
   (space, period, comma...) depending on the snippet's mode.

Notes:

- US keyboard layout with Shift and Caps Lock tracked, so uppercase letters and
  shifted symbols (`! @ # $ % ... { } | : " < > ?`) are captured correctly.
- Trigger is configurable in Preferences (Ctrl / Alt / Shift).
- Injected keystrokes are ignored, so no feedback loop.

## Linux hook

Requires read access to `/dev/input/event*` and write access to `/dev/uinput`.
On Ubuntu/Debian the `input` group covers both:

```
sudo usermod -aG input $USER   # log out and back in
breevy-rs --hook ~/.config/breevy/abbreviations.bvy
```

Notes:

- Types US-layout ASCII (letters, digits, punctuation, Shift symbols);
  non-ASCII is typed via Ctrl+Shift+U unicode input.
- Shift and Caps Lock are tracked during capture, so uppercase abbreviations
  and shifted symbols match exactly.
- Kernel-level, so it works in X11 and Wayland sessions alike.
- Trigger is configurable in Preferences (Ctrl / Alt / Shift).

## Build

```
cargo test --release
cargo build --release                                  # Linux GUI + CLI
cargo build --release --target x86_64-pc-windows-gnu   # Windows exe
```

Cross-compiling needs `mingw-w64` and the Rust target
`x86_64-pc-windows-gnu`.

## Releases

Tagged builds (`v*`) are produced by GitHub Actions:
`breevy-rs-linux-x86_64.zip` and `breevy-rs-win64.zip` on the Releases page.

## Layout

- `src/bvy.rs` — parser, entities, writer, in-place record update/remove.
- `src/engine.rs` — snippet model + expander state machine.
- `src/macros.rs` — Breevy insertion macros.
- `src/gui.rs` — egui editor (Breevy layout).
- `src/hook.rs` — Windows hook (winapi).
- `src/hook_linux.rs` — Linux hook (evdev/uinput).
- `src/main.rs` — CLI entry.
