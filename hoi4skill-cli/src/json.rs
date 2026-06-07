//! Minimal JSON rendering helpers for the current zero-dependency CLI.
//!
//! Command reports still render JSON as strings. Keeping all escaping and small
//! object helpers here gives the later serde migration a single boundary.

use std::collections::BTreeMap;

pub(crate) fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub(crate) fn json_array(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|s| json_str(s))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn json_bool(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

pub(crate) fn json_optional_str(value: Option<&str>) -> String {
    value.map(json_str).unwrap_or_else(|| "null".to_string())
}

pub(crate) fn json_optional_i64(value: Option<i64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

pub(crate) fn json_i64_array(values: &[i64]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(i64::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn json_i64_object(values: &BTreeMap<String, i64>) -> String {
    format!(
        "{{{}}}",
        values
            .iter()
            .map(|(k, v)| format!("{}: {}", json_str(k), v))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn json_object(values: &BTreeMap<String, String>) -> String {
    format!(
        "{{{}}}",
        values
            .iter()
            .map(|(k, v)| format!("{}: {}", json_str(k), json_str(v)))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn json_raw_object(values: &BTreeMap<String, String>) -> String {
    format!(
        "{{{}}}",
        values
            .iter()
            .map(|(k, v)| format!("{}: {}", json_str(k), v))
            .collect::<Vec<_>>()
            .join(", ")
    )
}
