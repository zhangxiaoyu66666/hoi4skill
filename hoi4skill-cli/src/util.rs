//! Small formatting and identifier helpers shared by the crate.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn slugify(value: &str, fallback: &str) -> String {
    let mut out = String::new();
    let mut last_us = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_us = false;
        } else if !last_us {
            out.push('_');
            last_us = true;
        }
    }
    let out = out.trim_matches('_').to_string();
    if out.is_empty() {
        fallback.to_string()
    } else {
        out
    }
}

pub(crate) fn hoi4_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "/").replace('"', "\\\""))
}

pub(crate) fn suggestions_json(values: &[Suggestion]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|s| {
                format!(
                    "{{\"kind\": {}, \"code\": {}, \"source\": {}, \"note\": {}}}",
                    json_str(&s.kind),
                    json_str(&s.code),
                    json_str(&s.source),
                    json_str(&s.note)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(crate) fn comma(out: &mut String, idx: usize, indent: &str) {
    if idx > 0 {
        out.push_str(",\n");
    }
    out.push_str(indent);
}
