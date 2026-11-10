//! Layout-independent character mapping shared by the Windows and Linux hooks.

/// The character produced when `c` is typed with Shift held on a US layout.
pub fn shifted(c: char) -> char {
    match c {
        '0' => ')',
        '1' => '!',
        '2' => '@',
        '3' => '#',
        '4' => '$',
        '5' => '%',
        '6' => '^',
        '7' => '&',
        '8' => '*',
        '9' => '(',
        '.' => '>',
        ',' => '<',
        ';' => ':',
        '=' => '+',
        '-' => '_',
        '/' => '?',
        '`' => '~',
        '[' => '{',
        '\\' => '|',
        ']' => '}',
        '\'' => '"',
        c => c.to_ascii_uppercase(),
    }
}

/// Apply Shift/Caps Lock the way a US keyboard would.
///
/// Letters follow Shift XOR Caps Lock; punctuation follows Shift only.
pub fn apply_shift(c: char, shift: bool, caps: bool) -> char {
    if c.is_ascii_alphabetic() {
        if shift ^ caps {
            c.to_ascii_uppercase()
        } else {
            c.to_ascii_lowercase()
        }
    } else if shift {
        shifted(c)
    } else {
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digits_shift_to_symbols() {
        assert_eq!(apply_shift('1', true, false), '!');
        assert_eq!(apply_shift('9', true, false), '(');
        assert_eq!(apply_shift('1', false, false), '1');
    }

    #[test]
    fn punctuation_shifts() {
        assert_eq!(apply_shift('.', true, false), '>');
        assert_eq!(apply_shift('\'', true, false), '"');
        assert_eq!(apply_shift('/', true, false), '?');
        assert_eq!(apply_shift(',', false, false), ',');
    }

    #[test]
    fn letters_follow_shift_xor_caps() {
        assert_eq!(apply_shift('a', true, false), 'A');
        assert_eq!(apply_shift('a', false, true), 'A');
        assert_eq!(apply_shift('a', true, true), 'a');
        assert_eq!(apply_shift('a', false, false), 'a');
    }

    #[test]
    fn punctuation_ignores_caps() {
        assert_eq!(apply_shift('1', false, true), '1');
        assert_eq!(apply_shift('.', false, true), '.');
    }
}
