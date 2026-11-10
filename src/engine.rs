#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expansion {
    pub remove_chars: usize,
    pub text: String,
    pub abbreviation: String,
    pub snippet_type: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Character(char),
    Backspace,
    Trigger,
    Delimiter(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    Expand(usize),
    Launch,
}

pub struct Expander {
    snippets: Vec<crate::Snippet>,
    buffer: String,
    expansion: Option<Expansion>,
}

impl Expander {
    pub fn new(snippets: Vec<crate::Snippet>) -> Self {
        let mut snippets = snippets;
        snippets.sort_by(|a, b| b.abbreviation.chars().count().cmp(&a.abbreviation.chars().count()));
        Self {
            snippets,
            buffer: String::new(),
            expansion: None,
        }
    }

    pub fn buffer(&self) -> &str {
        &self.buffer
    }

    pub fn snippet_count(&self) -> usize {
        self.snippets.len()
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
        self.expansion = None;
    }

    pub fn feed(&mut self, event: Event) -> Action {
        self.expansion = None;
        match event {
            Event::Character(c) => {
                self.buffer.push(c);
                Action::None
            }
            Event::Backspace => {
                self.buffer.pop();
                Action::None
            }
            Event::Delimiter(c) => self.delimiter_or_none(c),
            Event::Trigger => match self.find_match(true) {
                Some(m) => {
                    let launcher = self.snippets[m].snippet_type == 1
                        || self.snippets[m].snippet_type == 2;
                    self.apply(m);
                    if launcher {
                        Action::Launch
                    } else {
                        Action::Expand(self.expansion.as_ref().map(|e| e.remove_chars).unwrap_or(0))
                    }
                }
                None => Action::None,
            },
        }
    }

    pub fn take_expansion(&mut self) -> Option<Expansion> {
        self.expansion.take()
    }

    fn delimiter_or_none(&mut self, c: char) -> Action {
        if let Some(m) = self.find_match(false) {
            let action = if self.snippets[m].snippet_type == 1 || self.snippets[m].snippet_type == 2 {
                Action::Launch
            } else {
                Action::Expand(self.snippets[m].abbreviation.chars().count())
            };
            self.apply(m);
            let _ = c;
            action
        } else {
            Action::None
        }
    }

    fn find_match(&self, trigger: bool) -> Option<usize> {
        let typed_len = self.buffer.chars().count();
        for (i, snip) in self.snippets.iter().enumerate() {
            let abbr_len = snip.abbreviation.chars().count();
            if abbr_len == 0 || abbr_len > typed_len {
                continue;
            }
            let matches_mode = match snip.mode {
                2 => trigger,
                _ => !trigger,
            };
            if !matches_mode {
                continue;
            }
            let cut = typed_len - abbr_len;
            let prefix: String = self.buffer.chars().take(cut).collect();
            let tail: String = self.buffer.chars().skip(cut).collect();
            if snip.is_case_sensitive() {
                if tail != snip.abbreviation {
                    continue;
                }
            } else if tail.to_lowercase() != snip.abbreviation.to_lowercase() {
                continue;
            }
            if let Some(last) = prefix.chars().last() {
                if last.is_alphanumeric() {
                    continue;
                }
            }
            return Some(i);
        }
        None
    }

    fn apply(&mut self, idx: usize) {
        let snip = &self.snippets[idx];
        let abbr_len = snip.abbreviation.chars().count();
        let typed_len = self.buffer.chars().count();
        let typed_tail: String = self.buffer.chars().skip(typed_len - abbr_len).collect();
        let text = convert_case(&crate::macros::expand_macros(&snip.expansion), &typed_tail);
        self.expansion = Some(Expansion {
            remove_chars: abbr_len,
            text,
            abbreviation: snip.abbreviation.clone(),
            snippet_type: snip.snippet_type,
        });
        self.buffer.clear();
    }
}

fn convert_case(expansion: &str, typed: &str) -> String {
    if typed.chars().all(|c| c.is_uppercase() || !c.is_alphabetic()) {
        if typed.chars().any(|c| c.is_alphabetic()) {
            return expansion.to_uppercase();
        }
    }
    let mut chars = typed.chars();
    if let Some(first) = chars.next() {
        if first.is_uppercase() {
            let mut out: String = expansion
                .chars()
                .take(1)
                .flat_map(|c| c.to_uppercase())
                .collect();
            out.push_str(&expansion.chars().skip(1).collect::<String>());
            return out;
        }
    }
    expansion.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Snippet;

    fn snip(abbr: &str, text: &str, mode: u8) -> Snippet {
        Snippet {
            abbreviation: abbr.into(),
            expansion: text.into(),
            label: String::new(),
            mode,
            snippet_type: 0,
            uuid: String::new(),
            case_sensitive: false,
        }
    }

    #[test]
    fn trigger_mode_expands() {
        let mut e = Expander::new(vec![snip("addr", "1 Main St", 2)]);
        for c in "addr".chars() {
            assert_eq!(e.feed(Event::Character(c)), Action::None);
        }
        assert_eq!(e.feed(Event::Trigger), Action::Expand(4));
        let ex = e.take_expansion().unwrap();
        assert_eq!(ex.text, "1 Main St");
        assert_eq!(ex.remove_chars, 4);
    }

    #[test]
    fn no_match_inside_word() {
        let mut e = Expander::new(vec![snip("addr", "x", 2)]);
        for c in "myaddr".chars() {
            e.feed(Event::Character(c));
        }
        assert_eq!(e.feed(Event::Trigger), Action::None);
    }

    #[test]
    fn longest_match_wins() {
        let mut e = Expander::new(vec![snip("ab", "short", 2), snip("abc", "long", 2)]);
        for c in "abc".chars() {
            e.feed(Event::Character(c));
        }
        e.feed(Event::Trigger);
        assert_eq!(e.take_expansion().unwrap().text, "long");
    }

    #[test]
    fn case_preserved() {
        let mut e = Expander::new(vec![snip("sig", "regards", 2)]);
        for c in "SIG".chars() {
            e.feed(Event::Character(c));
        }
        e.feed(Event::Trigger);
        assert_eq!(e.take_expansion().unwrap().text, "REGARDS");
    }

    #[test]
    fn word_ending_mode_expands_on_delimiter() {
        let mut e = Expander::new(vec![snip("btw", "by the way", 1)]);
        for c in "btw".chars() {
            e.feed(Event::Character(c));
        }
        assert_eq!(e.feed(Event::Delimiter(' ')), Action::Expand(3));
    }
}
