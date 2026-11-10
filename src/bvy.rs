use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snippet {
    pub abbreviation: String,
    pub expansion: String,
    pub label: String,
    pub mode: u8,
    pub snippet_type: u8,
    pub uuid: String,
    pub case_sensitive: bool,
}

impl Snippet {
    pub fn is_case_sensitive(&self) -> bool {
        self.case_sensitive
    }
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let tail = &rest[amp..];
        let (decoded, consumed) = match tail.get(..4) {
            Some("&lt;") => (Some('<'), 4),
            Some("&gt;") => (Some('>'), 4),
            _ => match tail.get(..5) {
                Some("&amp;") => (Some('&'), 5),
                Some("&quot") if tail.get(..6) == Some("&quot;") => (Some('"'), 6),
                Some("&apos") if tail.get(..6) == Some("&apos;") => (Some('\''), 6),
                _ => (None, 0),
            },
        };
        if let Some(ch) = decoded {
            out.push(ch);
            rest = &tail[consumed..];
        } else if let Some(num_start) = tail.find("#x").map(|i| i + 2) {
            let end = tail.find(';');
            if let Some(end) = end {
                let hex = &tail[num_start..end];
                if let Ok(code) = u32::from_str_radix(hex, 16) {
                    if let Some(ch) = char::from_u32(code) {
                        out.push(ch);
                        rest = &tail[end + 1..];
                        continue;
                    }
                }
            }
            out.push('&');
            rest = &tail[1..];
        } else if let Some(num_start) = tail.find('#').map(|i| i + 1) {
            let end = tail.find(';');
            if let Some(end) = end {
                let dec = &tail[num_start..end];
                if let Ok(code) = dec.parse::<u32>() {
                    if let Some(ch) = char::from_u32(code) {
                        out.push(ch);
                        rest = &tail[end + 1..];
                        continue;
                    }
                }
            }
            out.push('&');
            rest = &tail[1..];
        } else {
            out.push('&');
            rest = &tail[1..];
        }
    }
    out.push_str(rest);
    out
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let pat = format!(" {}=\"", name);
    let start = tag.find(&pat)? + pat.len();
    let end = tag[start..].find('"')? + start;
    Some(unescape(&tag[start..end]))
}

fn dict_pairs(block: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let mut idx = 0;
    while let Some(key_open) = block[idx..].find("<key>") {
        let key_start = idx + key_open + 5;
        let Some(key_end_rel) = block[key_start..].find("</key>") else {
            break;
        };
        let key_end = key_start + key_end_rel;
        let key = unescape(&block[key_start..key_end]);
        let after = key_end + 6;
        let mut val_start = after;
        while block[val_start..].starts_with('\n') || block[val_start..].starts_with(' ') || block[val_start..].starts_with('\t') {
            val_start += 1;
        }
        let (value, next) = if block[val_start..].starts_with("<string>") {
            let start = val_start + 8;
            let Some(end_rel) = block[start..].find("</string>") else {
                break;
            };
            (unescape(&block[start..start + end_rel]), start + end_rel + 9)
        } else if block[val_start..].starts_with("<integer>") {
            let start = val_start + 9;
            let Some(end_rel) = block[start..].find("</integer>") else {
                break;
            };
            (block[start..start + end_rel].trim().to_string(), start + end_rel + 10)
        } else if block[val_start..].starts_with("<date>") {
            let start = val_start + 6;
            let Some(end_rel) = block[start..].find("</date>") else {
                break;
            };
            (block[start..start + end_rel].to_string(), start + end_rel + 7)
        } else {
            (String::new(), val_start)
        };
        map.insert(key, value);
        idx = next;
        if idx >= block.len() {
            break;
        }
    }
    map
}

pub fn parse_bvy(input: &str) -> Vec<Snippet> {
    let mut snippets = Vec::new();
    let mut idx = 0;
    while let Some(open_rel) = input[idx..].find("<a a=\"") {
        let tag_start = idx + open_rel;
        let Some(tag_end_rel) = input[tag_start..].find('>') else {
            break;
        };
        let tag_end = tag_start + tag_end_rel;
        let tag = &input[tag_start..tag_end];
        let self_closing = tag.trim_end().ends_with('/');
        let next_open = input[tag_end + 1..]
            .find("<a a=\"")
            .map(|r| tag_end + 1 + r);
        let (block, next_idx) = if self_closing {
            (None, tag_end + 1)
        } else {
            let close = input[tag_end..].find("</a>").map(|r| tag_end + r);
            match close {
                Some(c) if next_open.map_or(true, |n| c < n) => {
                    (Some(&input[tag_end + 1..c]), c + 4)
                }
                _ => {
                    let end = next_open.unwrap_or(input.len());
                    (Some(&input[tag_end + 1..end]), end)
                }
            }
        };
        let dict = block.map(dict_pairs).unwrap_or_default();
        let abbreviation = dict
            .get("abbreviation")
            .cloned()
            .or_else(|| attr(tag, "a"))
            .unwrap_or_default();
        let expansion = dict
            .get("plainText")
            .cloned()
            .or_else(|| attr(tag, "e"))
            .unwrap_or_default();
        let c_attr = attr(tag, "c").and_then(|v| v.parse::<u8>().ok());
        if !abbreviation.is_empty() {
            snippets.push(Snippet {
                abbreviation,
                expansion,
                label: dict
                    .get("label")
                    .cloned()
                    .or_else(|| attr(tag, "d"))
                    .unwrap_or_default(),
                mode: dict
                    .get("abbreviationMode")
                    .and_then(|v| v.parse().ok())
                    .or(c_attr)
                    .unwrap_or(2),
                snippet_type: dict
                    .get("snippetType")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0),
                uuid: dict.get("uuidString").cloned().unwrap_or_default(),
                case_sensitive: c_attr == Some(1),
            });
        }
        idx = next_idx;
    }
    snippets
}

fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\n' => out.push_str("&#10;"),
            '\t' => out.push_str("&#9;"),
            c => out.push(c),
        }
    }
    out
}

pub fn record_xml(s: &Snippet) -> String {
    let c = if s.case_sensitive { 1 } else { 2 };
    format!(
        "<a a=\"{a}\" d=\"{d}\" e=\"{e}\" c=\"{c}\" t=\"4\" r=\"0\"><textexpander><dict><key>abbreviation</key><string>{a}</string><key>abbreviationMode</key><integer>{m}</integer><key>label</key><string>{d}</string><key>plainText</key><string>{e}</string><key>snippetType</key><integer>{st}</integer><key>uuidString</key><string>{u}</string></dict></textexpander><textexpanderBS><dict><key>triggerMode</key><integer>2</integer><key>uuidString</key><string>{u}</string></dict></textexpanderBS></a>\n",
        a = escape_xml(&s.abbreviation),
        d = escape_xml(&s.label),
        e = escape_xml(&s.expansion),
        c = c,
        m = s.mode,
        st = s.snippet_type,
        u = escape_xml(&s.uuid),
    )
}

pub fn write_bvy(snippets: &[Snippet]) -> String {
    let mut out = String::from(
        "<Breevy n=\"All Abbreviations / Folders\" t=\"0\" d=\"0\" w=\"0\" m=\"1\">\n",
    );
    for s in snippets {
        out.push_str(&record_xml(s));
    }
    out.push_str("</Breevy>\n");
    out
}

fn record_span(xml: &str, uuid: &str) -> Option<(usize, usize)> {
    let needle = format!("<string>{}</string>", escape_xml(uuid));
    let mut from = 0usize;
    while let Some(i) = xml[from..].find("<a ") {
        let start = from + i;
        let tag_end = xml[start..].find('>')? + start;
        let self_closing = xml[start..=tag_end].trim_end().ends_with("/>");
        let end = if self_closing {
            tag_end + 1
        } else {
            xml[tag_end..].find("</a>")? + tag_end + 4
        };
        let block = &xml[start..end];
        if block.contains(&needle) {
            return Some((start, end));
        }
        from = end;
    }
    None
}

fn insert_record(xml: &str, s: &Snippet) -> String {
    let mut out = xml.to_string();
    let rec = record_xml(s);
    match out.rfind("</Breevy>") {
        Some(pos) => out.insert_str(pos, &rec),
        None => out.push_str(&rec),
    }
    out
}

pub fn update_bvy_record(xml: &str, s: &Snippet) -> String {
    match record_span(xml, &s.uuid) {
        Some((start, end)) => {
            let mut out = xml.to_string();
            out.replace_range(start..end, &record_xml(s));
            out
        }
        None => insert_record(xml, s),
    }
}

pub fn remove_bvy_record(xml: &str, uuid: &str) -> String {
    match record_span(xml, uuid) {
        Some((start, end)) => {
            let mut out = xml.to_string();
            out.replace_range(start..end, "");
            out
        }
        None => xml.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<Breevy n="All" t="0"><textexpander><dict><key>Characters Saved</key><integer>1517</integer></dict><a a="addr" e="123 Main&#10;Suite 4" c="2" t="4"><textexpander><dict><key>abbreviation</key><string>addr</string><key>abbreviationMode</key><integer>2</integer><key>label</key><string>Street address</string><key>plainText</key><string>123 Apple Street&#10;Suite #254&#10;Sometown, CA</string><key>snippetType</key><integer>0</integer><key>uuidString</key><string>ABC-123</string></dict></textexpander></a><a a="sig" e="Regards"><textexpander><dict><key>abbreviation</key><string>sig</string><key>abbreviationMode</key><integer>2</integer><key>plainText</key><string>Regards &amp; thanks</string></dict></textexpander></a><a a="zzz" d="Self close" e="self close text" c="1"/>"#;

    #[test]
    fn parses_records() {
        let got = parse_bvy(SAMPLE);
        assert_eq!(got.len(), 3);
        assert_eq!(got[0].abbreviation, "addr");
        assert_eq!(got[0].label, "Street address");
        assert_eq!(got[0].uuid, "ABC-123");
        assert!(got[0].expansion.contains("Suite #254"));
        assert_eq!(got[1].expansion, "Regards & thanks");
        assert_eq!(got[2].abbreviation, "zzz");
        assert_eq!(got[2].expansion, "self close text");
        assert!(got[2].is_case_sensitive());
    }

    #[test]
    fn decodes_entities() {
        assert_eq!(unescape("a&#10;b&amp;c&lt;d&quot;e"), "a\nb&c<d\"e");
        assert_eq!(unescape("&#x41;&#66;"), "AB");
    }

    #[test]
    fn truncation_is_safe() {
        let cut = &SAMPLE[..300];
        let got = parse_bvy(cut);
        assert!(got.len() <= 2);
    }

    #[test]
    fn updates_record_in_place() {
        let xml = SAMPLE.to_string();
        let mut sn = parse_bvy(SAMPLE)[0].clone();
        sn.expansion = "NEW EXPANSION".into();
        let out = update_bvy_record(&xml, &sn);
        assert!(out.contains("NEW EXPANSION"));
        assert!(out.contains("<a a=\"sig\""));
        assert_eq!(parse_bvy(&out).len(), 3);
    }

    #[test]
    fn removes_record_in_place() {
        let xml = SAMPLE.to_string();
        let out = remove_bvy_record(&xml, "ABC-123");
        assert!(!out.contains("ABC-123"));
        assert_eq!(parse_bvy(&out).len(), 2);
        assert!(out.contains("<a a=\"sig\""));
    }
}
