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
                    "{{\"kind\": {}, \"code\": {}, \"source\": {}, \"effect_strategy\": {}, \"note\": {}}}",
                    json_str(&s.kind),
                    json_str(&s.code),
                    json_str(&s.source),
                    json_str(suggestion_effect_strategy(s)),
                    json_str(&s.note)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn suggestions_safety_json(values: &[Suggestion]) -> String {
    suggestions_safety_json_with_extra_blockers(values, &[])
}

pub(crate) fn suggestions_safety_json_with_extra_blockers(
    values: &[Suggestion],
    extra_blockers: &[String],
) -> String {
    let mut blockers = suggestions_safety_blockers(values);
    blockers.extend(extra_blockers.iter().cloned());
    let requires_mapping = values.iter().any(|suggestion| {
        suggestion.kind == "raw_effect"
            || suggestion.kind == "raw_trigger"
            || suggestion
                .note
                .contains("Needs Codex mapping before final code")
    });
    let has_placeholder = values
        .iter()
        .any(|suggestion| suggestion.code.contains('<') || suggestion.code.contains('>'));
    let status = if blockers.is_empty() {
        "verified_shape"
    } else {
        "blocked"
    };
    format!(
        "{{\"status\": {}, \"final_code_allowed\": {}, \"requires_mapping\": {}, \"has_placeholder\": {}, \"blockers\": {}}}",
        json_str(status),
        json_bool(blockers.is_empty()),
        json_bool(requires_mapping),
        json_bool(has_placeholder),
        json_array(&blockers)
    )
}

pub(crate) fn suggestions_safety_blockers(values: &[Suggestion]) -> Vec<String> {
    let mut blockers = Vec::new();
    for suggestion in values {
        if suggestion.kind == "raw_effect" || suggestion.kind == "raw_trigger" {
            blockers.push(format!(
                "{} `{}` must be mapped to a verified code-catalog entry",
                suggestion.kind, suggestion.source
            ));
        }
        if suggestion.code.contains('<') || suggestion.code.contains('>') {
            blockers.push(format!(
                "`{}` contains unresolved placeholder code `{}`",
                suggestion.source, suggestion.code
            ));
        }
        if suggestion
            .note
            .contains("Needs Codex mapping before final code")
        {
            blockers.push(format!(
                "`{}` needs Codex mapping before final code",
                suggestion.source
            ));
        }
    }
    blockers
}

pub(crate) fn apply_writer_report_json(
    schema: &str,
    input: &Path,
    mod_root: &Path,
    tag: &str,
    prefix: &str,
    count_key: &str,
    count: usize,
    changed_files: &[PathBuf],
) -> String {
    let changed = changed_files
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    format!(
        "{{\n  \"schema\": {},\n  \"ok\": true,\n  \"input\": {},\n  \"mod_root\": {},\n  \"tag\": {},\n  \"prefix\": {},\n  \"{}\": {},\n  \"changed_file_count\": {},\n  \"changed_files\": {}\n}}\n",
        json_str(schema),
        json_str(&input.display().to_string()),
        json_str(&mod_root.display().to_string()),
        json_str(tag),
        json_str(prefix),
        count_key,
        count,
        changed.len(),
        json_array(&changed)
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
