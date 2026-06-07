//! HOI4 error.log parser and repair hint reporter.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_analyze_error_log(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let text = read_utf8_lossy(&input)?;
    let diagnostics = analyze_error_log(&text, mod_root.as_deref());
    let json = error_log_report_json(&input, mod_root.as_deref(), &diagnostics);
    write_or_print(&json, value(&map, "output"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ErrorLogDiagnostic {
    pub(crate) severity: String,
    pub(crate) category: String,
    pub(crate) file: Option<String>,
    pub(crate) resolved_file: Option<String>,
    pub(crate) line: Option<i64>,
    pub(crate) message: String,
    pub(crate) suggestion: String,
    pub(crate) raw: String,
}

pub(crate) fn analyze_error_log(text: &str, mod_root: Option<&Path>) -> Vec<ErrorLogDiagnostic> {
    let mut diagnostics = Vec::new();
    for raw in text.lines() {
        let raw = raw.trim();
        if raw.is_empty() || !is_error_log_diagnostic_line(raw) {
            continue;
        }
        let file = extract_log_file_path(raw);
        let line = extract_log_line_number(raw, file.as_deref());
        let message = clean_log_message(raw);
        let category = classify_error_log_line(raw, &message);
        let severity = classify_error_log_severity(raw);
        let suggestion = error_log_suggestion(&category, file.as_deref(), line);
        let resolved_file = file
            .as_deref()
            .and_then(|file| resolve_log_file(mod_root, file))
            .map(|path| path.display().to_string());
        diagnostics.push(ErrorLogDiagnostic {
            severity,
            category,
            file,
            resolved_file,
            line,
            message,
            suggestion,
            raw: raw.to_string(),
        });
    }
    diagnostics
}

pub(crate) fn is_error_log_diagnostic_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("error")
        || lower.contains("warning")
        || lower.contains("failed")
        || lower.contains("exception")
        || lower.contains("could not")
        || lower.contains("malformed")
        || lower.contains("unknown ")
        || lower.contains("invalid ")
        || lower.contains("missing ")
}

pub(crate) fn classify_error_log_severity(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    if lower.contains("warning") {
        "warning".to_string()
    } else {
        "error".to_string()
    }
}

pub(crate) fn classify_error_log_line(raw: &str, message: &str) -> String {
    let lower = format!("{raw}\n{message}").to_ascii_lowercase();
    if lower.contains("localisation")
        || lower.contains("localization")
        || lower.contains("missing loc")
        || lower.contains("missing localisation")
    {
        "localisation".to_string()
    } else if lower.contains("gfx")
        || lower.contains("sprite")
        || lower.contains("texturefile")
        || lower.contains("texture")
    {
        "gfx".to_string()
    } else if lower.contains("namespace")
        || (lower.contains("event") && lower.contains(" id "))
        || lower.contains("event id")
    {
        "event_namespace".to_string()
    } else if lower.contains("unknown effect")
        || lower.contains("unknown trigger")
        || lower.contains("invalid effect")
        || lower.contains("invalid trigger")
        || lower.contains("unexpected effect")
        || lower.contains("unexpected trigger")
    {
        "script_command".to_string()
    } else if lower.contains("modifier") {
        "modifier".to_string()
    } else if lower.contains("malformed token")
        || lower.contains("unexpected token")
        || lower.contains("unexpected end")
        || lower.contains("expected token")
        || lower.contains("syntax")
        || lower.contains("parser")
    {
        "syntax".to_string()
    } else if lower.contains("focus") {
        "national_focus".to_string()
    } else if lower.contains("descriptor") || lower.contains(".mod") {
        "descriptor".to_string()
    } else if lower.contains("could not find")
        || lower.contains("missing file")
        || lower.contains("no such file")
    {
        "missing_file".to_string()
    } else {
        "general".to_string()
    }
}

pub(crate) fn error_log_suggestion(
    category: &str,
    file: Option<&str>,
    line: Option<i64>,
) -> String {
    let location = match (file, line) {
        (Some(file), Some(line)) => format!(" Check `{file}` near line {line}."),
        (Some(file), None) => format!(" Check `{file}`."),
        (None, Some(line)) => format!(" Check the referenced file near line {line}."),
        (None, None) => String::new(),
    };
    let base = match category {
        "syntax" => "Check braces, quotes, and assignment syntax around the reported token.",
        "gfx" => "Check interface/*.gfx sprite names and texturefile paths; run icon-preview or validate.",
        "localisation" => {
            "Add or fix the missing localisation key under localisation/simp_chinese with l_simp_chinese:."
        }
        "event_namespace" => "Declare add_namespace = <ns> at the top level before event bodies; event ids should use <ns>.<number> with number 1..=200000, plus matching title/desc/option localisation.",
        "script_command" => "Verify whether the command belongs in an effect or trigger context and check its spelling against game documentation.",
        "modifier" => "Verify modifier names against local documentation or nearby working mod files.",
        "national_focus" => "Check focus id, prerequisite, mutually_exclusive, icon, and completion_reward structure.",
        "descriptor" => "Check descriptor.mod and launcher .mod metadata, path, dependencies, and supported_version.",
        "missing_file" => "Check that referenced files exist and that dependency mods are passed with --mod-path.",
        _ => "Inspect the reported file and rerun validate with --game-root and --mod-path dependencies.",
    };
    format!("{base}{location}")
}

pub(crate) fn extract_log_file_path(line: &str) -> Option<String> {
    for marker in ["file:", "in file:", "File:", "In file:", "path:", "Path:"] {
        if let Some(value) = extract_quoted_after(line, marker) {
            if looks_like_log_path(&value) {
                return Some(trim_log_path_line_suffix(&value).to_string());
            }
        }
    }
    for value in quoted_values(line) {
        if looks_like_log_path(&value) {
            return Some(trim_log_path_line_suffix(&value).to_string());
        }
    }
    extract_unquoted_log_path(line)
}

pub(crate) fn extract_quoted_after(line: &str, marker: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let marker_lower = marker.to_ascii_lowercase();
    let idx = lower.find(&marker_lower)?;
    let rest = &line[idx + marker.len()..];
    quoted_values(rest).into_iter().next()
}

pub(crate) fn extract_unquoted_log_path(line: &str) -> Option<String> {
    for token in line.split_whitespace() {
        let token = token.trim_matches(|ch: char| {
            matches!(
                ch,
                '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';'
            )
        });
        let lower = token.to_ascii_lowercase();
        for ext in [
            ".txt", ".yml", ".yaml", ".gfx", ".gui", ".mod", ".asset", ".jsonl", ".json",
        ] {
            if let Some(idx) = lower.find(ext) {
                let end = idx + ext.len();
                let candidate = &token[..end];
                if looks_like_log_path(candidate) {
                    return Some(candidate.to_string());
                }
            }
        }
    }
    None
}

pub(crate) fn looks_like_log_path(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let has_known_ext = [
        ".txt", ".yml", ".yaml", ".gfx", ".gui", ".mod", ".asset", ".jsonl", ".json",
    ]
    .iter()
    .any(|ext| lower.contains(ext));
    has_known_ext
        && (value.contains('/')
            || value.contains('\\')
            || lower.starts_with("common")
            || lower.starts_with("events")
            || lower.starts_with("history")
            || lower.starts_with("interface")
            || lower.starts_with("localisation")
            || lower.starts_with("localization"))
}

pub(crate) fn trim_log_path_line_suffix(value: &str) -> &str {
    let lower = value.to_ascii_lowercase();
    for ext in [
        ".txt", ".yml", ".yaml", ".gfx", ".gui", ".mod", ".asset", ".jsonl", ".json",
    ] {
        if let Some(idx) = lower.find(ext) {
            return &value[..idx + ext.len()];
        }
    }
    value
}

pub(crate) fn extract_log_line_number(line: &str, file: Option<&str>) -> Option<i64> {
    for marker in ["near line:", "line:", "line "] {
        if let Some(value) = first_i64_after_marker(line, marker) {
            return Some(value);
        }
    }
    if let Some(file) = file {
        if let Some(idx) = line.find(file) {
            let rest = &line[idx + file.len()..];
            if let Some(after_colon) = rest.strip_prefix(':') {
                return leading_i64(after_colon);
            }
        }
    }
    None
}

pub(crate) fn first_i64_after_marker(line: &str, marker: &str) -> Option<i64> {
    let lower = line.to_ascii_lowercase();
    let idx = lower.find(&marker.to_ascii_lowercase())?;
    leading_i64_after_noise(&line[idx + marker.len()..])
}

pub(crate) fn leading_i64_after_noise(text: &str) -> Option<i64> {
    let start = text
        .char_indices()
        .find(|(_, ch)| ch.is_ascii_digit())
        .map(|(idx, _)| idx)?;
    leading_i64(&text[start..])
}

pub(crate) fn leading_i64(text: &str) -> Option<i64> {
    let digits = text
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    digits.parse().ok()
}

pub(crate) fn clean_log_message(line: &str) -> String {
    let mut message = line;
    if let Some(idx) = message.rfind("]:") {
        message = &message[idx + 2..];
    }
    let message = message.trim();
    let message = message
        .strip_prefix("Error:")
        .or_else(|| message.strip_prefix("Warning:"))
        .unwrap_or(message)
        .trim();
    message.trim_matches('"').to_string()
}

pub(crate) fn resolve_log_file(mod_root: Option<&Path>, file: &str) -> Option<PathBuf> {
    let normalized = file.replace('/', "\\");
    let path = PathBuf::from(&normalized);
    if path.is_absolute() && path.exists() {
        return Some(path);
    }
    let root = mod_root?;
    let candidate = root.join(normalized.trim_start_matches('\\'));
    candidate.exists().then_some(candidate)
}

pub(crate) fn error_log_report_json(
    input: &Path,
    mod_root: Option<&Path>,
    diagnostics: &[ErrorLogDiagnostic],
) -> String {
    let mut severity_counts: BTreeMap<String, i64> = BTreeMap::new();
    let mut category_counts: BTreeMap<String, i64> = BTreeMap::new();
    for diagnostic in diagnostics {
        *severity_counts
            .entry(diagnostic.severity.clone())
            .or_default() += 1;
        *category_counts
            .entry(diagnostic.category.clone())
            .or_default() += 1;
    }
    let items = diagnostics
        .iter()
        .map(error_log_diagnostic_json)
        .collect::<Vec<_>>()
        .join(",\n    ");
    format!(
        "{{\n  \"input\": {},\n  \"mod_root\": {},\n  \"diagnostics\": [\n    {}\n  ],\n  \"counts\": {{\"total\": {}, \"by_severity\": {}, \"by_category\": {}}}\n}}\n",
        json_str(&input.display().to_string()),
        mod_root
            .map(|path| json_str(&path.display().to_string()))
            .unwrap_or_else(|| "null".to_string()),
        items,
        diagnostics.len(),
        json_i64_object(&severity_counts),
        json_i64_object(&category_counts)
    )
}

pub(crate) fn error_log_diagnostic_json(diagnostic: &ErrorLogDiagnostic) -> String {
    format!(
        "{{\"severity\": {}, \"category\": {}, \"file\": {}, \"resolved_file\": {}, \"line\": {}, \"message\": {}, \"suggestion\": {}, \"raw\": {}}}",
        json_str(&diagnostic.severity),
        json_str(&diagnostic.category),
        json_optional_str(diagnostic.file.as_deref()),
        json_optional_str(diagnostic.resolved_file.as_deref()),
        json_optional_i64(diagnostic.line),
        json_str(&diagnostic.message),
        json_str(&diagnostic.suggestion),
        json_str(&diagnostic.raw)
    )
}
