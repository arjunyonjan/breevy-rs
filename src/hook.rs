//! Windows global keyboard hook: buffer keystrokes, match a Breevy snippet,
//! erase the typed abbreviation and type the expansion.

use std::ptr;
use std::sync::{Mutex, OnceLock};

use breevy_core::engine::{Event, Expansion};
use breevy_core::{parse_bvy, Expander};
use winapi::shared::minwindef::{HINSTANCE, LPARAM, LRESULT, UINT, WPARAM};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::winuser::*;

static ENGINE: OnceLock<Mutex<Expander>> = OnceLock::new();

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
        if kb.flags & LLKHF_INJECTED == 0
            && (wparam as u32 == WM_KEYDOWN || wparam as u32 == WM_SYSKEYDOWN)
        {
            handle_vk(kb.vkCode as i32);
        }
    }
    CallNextHookEx(ptr::null_mut(), code, wparam, lparam)
}

fn event_for(vk: i32) -> Option<Event> {
    let event = match vk {
        VK_BACK => Event::Backspace,
        VK_CONTROL | VK_LCONTROL | VK_RCONTROL => Event::Trigger,
        VK_SPACE => Event::Delimiter(' '),
        VK_RETURN => Event::Delimiter('\n'),
        VK_TAB => Event::Delimiter('\t'),
        VK_OEM_PERIOD => Event::Delimiter('.'),
        VK_OEM_COMMA => Event::Delimiter(','),
        VK_OEM_1 => Event::Delimiter(';'),
        VK_OEM_PLUS => Event::Delimiter('='),
        VK_OEM_MINUS => Event::Delimiter('-'),
        VK_OEM_2 => Event::Delimiter('/'),
        VK_OEM_3 => Event::Delimiter('`'),
        VK_OEM_4 => Event::Delimiter('['),
        VK_OEM_5 => Event::Delimiter('\\'),
        VK_OEM_6 => Event::Delimiter(']'),
        VK_OEM_7 => Event::Delimiter('\''),
        _ => Event::Character(vk_char(vk)?),
    };
    Some(event)
}

fn handle_vk(vk: i32) {
    let Some(event) = event_for(vk) else { return };
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
        unsafe {
            for _ in 0..remove_chars {
                send_vk(VK_BACK);
            }
            send_text(&text);
        }
    }
}

fn vk_char(vk: i32) -> Option<char> {
    match vk {
        0x41..=0x5A => Some((b'a' + (vk - 0x41) as u8) as char),
        0x30..=0x39 => Some((b'0' + (vk - 0x30) as u8) as char),
        VK_OEM_PERIOD => Some('.'),
        VK_OEM_COMMA => Some(','),
        VK_OEM_1 => Some(';'),
        VK_OEM_PLUS => Some('='),
        VK_OEM_MINUS => Some('-'),
        VK_OEM_2 => Some('/'),
        VK_OEM_3 => Some('`'),
        VK_OEM_4 => Some('['),
        VK_OEM_5 => Some('\\'),
        VK_OEM_6 => Some(']'),
        VK_OEM_7 => Some('\''),
        _ => None,
    }
}

fn send_input(input: &mut INPUT) -> UINT {
    unsafe { SendInput(1, input, std::mem::size_of::<INPUT>() as i32) }
}

unsafe fn send_vk(vk: i32) {
    let mut ki: KEYBDINPUT = std::mem::zeroed();
    ki.wVk = vk as u16;
    ki.wScan = MapVirtualKeyW(vk as u32, 0) as u16;
    let mut input: INPUT = std::mem::zeroed();
    input.type_ = INPUT_KEYBOARD;
    *input.u.ki_mut() = ki;
    send_input(&mut input);
    ki.dwFlags = KEYEVENTF_KEYUP;
    let mut input: INPUT = std::mem::zeroed();
    input.type_ = INPUT_KEYBOARD;
    *input.u.ki_mut() = ki;
    send_input(&mut input);
}

unsafe fn send_text(text: &str) {
    for unit in text.encode_utf16() {
        for flags in [KEYEVENTF_UNICODE, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP] {
            let mut ki: KEYBDINPUT = std::mem::zeroed();
            ki.wScan = unit;
            ki.dwFlags = flags;
            let mut input: INPUT = std::mem::zeroed();
            input.type_ = INPUT_KEYBOARD;
            *input.u.ki_mut() = ki;
            send_input(&mut input);
        }
    }
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
    let expander = Expander::new(snippets);
    let _ = ENGINE.set(Mutex::new(expander));
    unsafe {
        let module: HINSTANCE = GetModuleHandleW(ptr::null());
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), module, 0);
        if hook.is_null() {
            eprintln!(
                "SetWindowsHookExW failed: {}",
                std::io::Error::last_os_error()
            );
            std::process::exit(1);
        }
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    std::process::exit(0)
}
