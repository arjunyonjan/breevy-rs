//! Linux global keyboard listener: kernel-level evdev capture + uinput injection.
//! Works on X11, Wayland and consoles. Needs read access to /dev/input/event*
//! and write access to /dev/uinput (usually: add the user to the `input` group).

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use breevy_core::engine::{Event, Expansion};
use breevy_core::{parse_bvy, Expander};
use evdev::uinput::VirtualDeviceBuilder;
use evdev::{AttributeSet, Device, EventType, InputEvent, KeyCode};

static ENGINE: OnceLock<Mutex<Expander>> = OnceLock::new();
static INJECT: OnceLock<Mutex<VirtualDevice>> = OnceLock::new();
static SUPPRESS_UNTIL: OnceLock<Mutex<Instant>> = OnceLock::new();

use evdev::uinput::VirtualDevice;

const VIRTUAL_NAME: &str = "breevy-rs virtual keyboard";

fn suppress_for(d: Duration) {
    let lock = SUPPRESS_UNTIL.get_or_init(|| Mutex::new(Instant::now()));
    if let Ok(mut until) = lock.lock() {
        *until = Instant::now() + d;
    }
}

fn suppressed() -> bool {
    let lock = SUPPRESS_UNTIL.get_or_init(|| Mutex::new(Instant::now()));
    match lock.lock() {
        Ok(until) => Instant::now() < *until,
        Err(_) => false,
    }
}

fn event_for(key: KeyCode) -> Option<Event> {
    let event = match key {
        KeyCode::KEY_BACKSPACE => Event::Backspace,
        KeyCode::KEY_LEFTCTRL | KeyCode::KEY_RIGHTCTRL => Event::Trigger,
        KeyCode::KEY_SPACE => Event::Delimiter(' '),
        KeyCode::KEY_ENTER => Event::Delimiter('\n'),
        KeyCode::KEY_TAB => Event::Delimiter('\t'),
        KeyCode::KEY_MINUS => Event::Delimiter('-'),
        KeyCode::KEY_EQUAL => Event::Delimiter('='),
        KeyCode::KEY_LEFTBRACE => Event::Delimiter('['),
        KeyCode::KEY_RIGHTBRACE => Event::Delimiter(']'),
        KeyCode::KEY_BACKSLASH => Event::Delimiter('\\'),
        KeyCode::KEY_SEMICOLON => Event::Delimiter(';'),
        KeyCode::KEY_APOSTROPHE => Event::Delimiter('\''),
        KeyCode::KEY_COMMA => Event::Delimiter(','),
        KeyCode::KEY_DOT => Event::Delimiter('.'),
        KeyCode::KEY_SLASH => Event::Delimiter('/'),
        KeyCode::KEY_GRAVE => Event::Delimiter('`'),
        KeyCode::KEY_A => Event::Character('a'),
        KeyCode::KEY_B => Event::Character('b'),
        KeyCode::KEY_C => Event::Character('c'),
        KeyCode::KEY_D => Event::Character('d'),
        KeyCode::KEY_E => Event::Character('e'),
        KeyCode::KEY_F => Event::Character('f'),
        KeyCode::KEY_G => Event::Character('g'),
        KeyCode::KEY_H => Event::Character('h'),
        KeyCode::KEY_I => Event::Character('i'),
        KeyCode::KEY_J => Event::Character('j'),
        KeyCode::KEY_K => Event::Character('k'),
        KeyCode::KEY_L => Event::Character('l'),
        KeyCode::KEY_M => Event::Character('m'),
        KeyCode::KEY_N => Event::Character('n'),
        KeyCode::KEY_O => Event::Character('o'),
        KeyCode::KEY_P => Event::Character('p'),
        KeyCode::KEY_Q => Event::Character('q'),
        KeyCode::KEY_R => Event::Character('r'),
        KeyCode::KEY_S => Event::Character('s'),
        KeyCode::KEY_T => Event::Character('t'),
        KeyCode::KEY_U => Event::Character('u'),
        KeyCode::KEY_V => Event::Character('v'),
        KeyCode::KEY_W => Event::Character('w'),
        KeyCode::KEY_X => Event::Character('x'),
        KeyCode::KEY_Y => Event::Character('y'),
        KeyCode::KEY_Z => Event::Character('z'),
        KeyCode::KEY_0 => Event::Character('0'),
        KeyCode::KEY_1 => Event::Character('1'),
        KeyCode::KEY_2 => Event::Character('2'),
        KeyCode::KEY_3 => Event::Character('3'),
        KeyCode::KEY_4 => Event::Character('4'),
        KeyCode::KEY_5 => Event::Character('5'),
        KeyCode::KEY_6 => Event::Character('6'),
        KeyCode::KEY_7 => Event::Character('7'),
        KeyCode::KEY_8 => Event::Character('8'),
        KeyCode::KEY_9 => Event::Character('9'),
        _ => return None,
    };
    Some(event)
}

fn emit(key: KeyCode, value: i32) -> bool {
    let Some(lock) = INJECT.get() else { return false };
    let Ok(mut dev) = lock.lock() else { return false };
    dev.emit(&[
        InputEvent::new(EventType::KEY.0, key.code(), value),
        InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
    ])
    .is_ok()
}

fn tap(key: KeyCode) {
    emit(key, 1);
    emit(key, 0);
}

fn key_for_char(c: char) -> Option<(KeyCode, bool)> {
    let (key, shift) = match c {
        'a'..='z' => (KeyCode::new((c as u16 - 'a' as u16) + KeyCode::KEY_A.code()), false),
        'A'..='Z' => (
            KeyCode::new((c as u16 - 'A' as u16) + KeyCode::KEY_A.code()),
            true,
        ),
        '0'..='9' => (
            KeyCode::new((c as u16 - '0' as u16) + KeyCode::KEY_0.code()),
            false,
        ),
        ' ' => (KeyCode::KEY_SPACE, false),
        '\n' => (KeyCode::KEY_ENTER, false),
        '\t' => (KeyCode::KEY_TAB, false),
        '-' => (KeyCode::KEY_MINUS, false),
        '_' => (KeyCode::KEY_MINUS, true),
        '=' => (KeyCode::KEY_EQUAL, false),
        '+' => (KeyCode::KEY_EQUAL, true),
        '[' => (KeyCode::KEY_LEFTBRACE, false),
        '{' => (KeyCode::KEY_LEFTBRACE, true),
        ']' => (KeyCode::KEY_RIGHTBRACE, false),
        '}' => (KeyCode::KEY_RIGHTBRACE, true),
        '\\' => (KeyCode::KEY_BACKSLASH, false),
        '|' => (KeyCode::KEY_BACKSLASH, true),
        ';' => (KeyCode::KEY_SEMICOLON, false),
        ':' => (KeyCode::KEY_SEMICOLON, true),
        '\'' => (KeyCode::KEY_APOSTROPHE, false),
        '"' => (KeyCode::KEY_APOSTROPHE, true),
        ',' => (KeyCode::KEY_COMMA, false),
        '<' => (KeyCode::KEY_COMMA, true),
        '.' => (KeyCode::KEY_DOT, false),
        '>' => (KeyCode::KEY_DOT, true),
        '/' => (KeyCode::KEY_SLASH, false),
        '?' => (KeyCode::KEY_SLASH, true),
        '`' => (KeyCode::KEY_GRAVE, false),
        '~' => (KeyCode::KEY_GRAVE, true),
        '!' => (KeyCode::KEY_1, true),
        '@' => (KeyCode::KEY_2, true),
        '#' => (KeyCode::KEY_3, true),
        '$' => (KeyCode::KEY_4, true),
        '%' => (KeyCode::KEY_5, true),
        '^' => (KeyCode::KEY_6, true),
        '&' => (KeyCode::KEY_7, true),
        '*' => (KeyCode::KEY_8, true),
        '(' => (KeyCode::KEY_9, true),
        ')' => (KeyCode::KEY_0, true),
        _ => return None,
    };
    Some((key, shift))
}

fn type_char(c: char) {
    let Some((key, shift)) = key_for_char(c) else {
        return;
    };
    if shift {
        emit(KeyCode::KEY_LEFTSHIFT, 1);
    }
    tap(key);
    if shift {
        emit(KeyCode::KEY_LEFTSHIFT, 0);
    }
}

fn inject(remove_chars: usize, text: &str) {
    suppress_for(Duration::from_millis(400));
    for _ in 0..remove_chars {
        tap(KeyCode::KEY_BACKSPACE);
    }
    for c in text.chars() {
        type_char(c);
        std::thread::sleep(Duration::from_millis(2));
    }
    suppress_for(Duration::from_millis(400));
}

fn handle_key(key: KeyCode) {
    if suppressed() {
        return;
    }
    let Some(event) = event_for(key) else { return };
    let expansion = {
        let Some(lock) = ENGINE.get() else { return };
        let Ok(mut ex) = lock.lock() else { return };
        let _ = ex.feed(event);
        ex.take_expansion()
    };
    if let Some(Expansion {
        remove_chars, text, ..
    }) = expansion
    {
        inject(remove_chars, &text);
    }
}

fn is_keyboard(dev: &Device) -> bool {
    match dev.supported_keys() {
        Some(keys) => keys.contains(KeyCode::KEY_A) && keys.contains(KeyCode::KEY_Z),
        None => false,
    }
}

fn read_device(mut dev: Device) {
    loop {
        match dev.fetch_events() {
            Ok(events) => {
                for ev in events {
                    if ev.event_type() != EventType::KEY {
                        continue;
                    }
                    if ev.value() != 1 {
                        continue;
                    }
                    handle_key(KeyCode::new(ev.code()));
                }
            }
            Err(e) => {
                eprintln!("input device read failed: {e}");
                return;
            }
        }
    }
}

fn build_virtual() -> std::io::Result<VirtualDevice> {
    let mut keys = AttributeSet::<KeyCode>::new();
    for code in 1..=255u16 {
        let key = KeyCode::new(code);
        keys.insert(key);
    }
    VirtualDeviceBuilder::new()?
        .name(VIRTUAL_NAME)
        .with_keys(&keys)?
        .build()
}

pub fn run(path: &str) -> ! {
    let data = match std::fs::read_to_string(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("cannot read {path}: {e}");
            std::process::exit(1);
        }
    };
    let snippets = parse_bvy(&data);
    eprintln!("breevy-rs hook: {} snippets from {path}", snippets.len());
    let _ = ENGINE.set(Mutex::new(Expander::new(snippets)));

    let virtual_dev = match build_virtual() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("cannot open /dev/uinput: {e}");
            eprintln!("fix: sudo usermod -aG input $USER  (then log out and back in)");
            std::process::exit(1);
        }
    };
    let _ = INJECT.set(Mutex::new(virtual_dev));

    let mut opened = HashSet::new();
    let entries = match std::fs::read_dir("/dev/input") {
        Ok(e) => e,
        Err(e) => {
            eprintln!("cannot read /dev/input: {e}");
            std::process::exit(1);
        }
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("event") {
            continue;
        }
        let dev = match Device::open(entry.path()) {
            Ok(d) => d,
            Err(_) => continue,
        };
        if dev.name().map(|n| n.contains("breevy-rs")) == Some(true) {
            continue;
        }
        if !is_keyboard(&dev) {
            continue;
        }
        eprintln!(
            "watching {} ({})",
            entry.path().display(),
            dev.name().unwrap_or("unnamed")
        );
        opened.insert(name);
        std::thread::spawn(move || read_device(dev));
    }
    if opened.is_empty() {
        eprintln!("no keyboard devices found (need read access to /dev/input/event*)");
        std::process::exit(1);
    }
    eprintln!("breevy-rs listening (Ctrl expands, Ctrl+C in your shell is still your shell)");
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}
