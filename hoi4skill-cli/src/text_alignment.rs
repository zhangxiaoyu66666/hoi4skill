//! Simple text-preservation checks between user input and generated mod output.

#[allow(unused_imports)]
use crate::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExpectedText {
    pub(crate) text: String,
    pub(crate) source: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TextHit {
    pub(crate) value: String,
    pub(crate) source: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TextAlignmentItem {
    pub(crate) expected: ExpectedText,
    pub(crate) matches: Vec<TextHit>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TextAlignmentReport {
    pub(crate) mod_root: PathBuf,
    pub(crate) expected: Vec<TextAlignmentItem>,
}

impl TextAlignmentReport {
    pub(crate) fn missing(&self) -> Vec<&TextAlignmentItem> {
        self.expected
            .iter()
            .filter(|item| item.matches.is_empty())
            .collect()
    }

    pub(crate) fn matched_count(&self) -> usize {
        self.expected
            .iter()
            .filter(|item| !item.matches.is_empty())
            .count()
    }
}

pub(crate) fn cmd_check_text_alignment(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let report = text_alignment_report_from_args(&mod_root, &map)?;
    let json = text_alignment_report_json(&report);
    write_or_print(&json, value(&map, "output"))?;
    if report.missing().is_empty() {
        Ok(())
    } else {
        Err("text alignment failed".to_string())
    }
}

pub(crate) fn check_text_alignment_from_validate_args(
    root: &Path,
    map: &ArgMap,
    reporter: &mut Reporter,
) -> Result<(), String> {
    if !has_text_alignment_args(map) {
        return Ok(());
    }
    let report = text_alignment_report_from_args(root, map)?;
    for item in report.missing() {
        reporter.error(format!(
            "text alignment missing user-provided text `{}` from {}",
            item.expected.text, item.expected.source
        ));
    }
    Ok(())
}

pub(crate) fn has_text_alignment_args(map: &ArgMap) -> bool {
    ["text-source", "source-file", "input"]
        .iter()
        .any(|key| !repeated_values(map, key).is_empty())
        || !repeated_values(map, "expect-title").is_empty()
        || !repeated_values(map, "expect-text").is_empty()
}

pub(crate) fn text_alignment_report_from_args(
    mod_root: &Path,
    map: &ArgMap,
) -> Result<TextAlignmentReport, String> {
    let tag = value(map, "tag").unwrap_or("TAG");
    let prefix = value(map, "prefix").unwrap_or("mod");
    let sheet = value(map, "sheet");
    let mut expected = Vec::new();

    for key in ["text-source", "source-file", "input"] {
        for raw in repeated_values(map, key) {
            let path = normalize_path(raw)?;
            expected.extend(expected_texts_from_path(&path, sheet, tag, prefix)?);
        }
    }
    for title in repeated_values(map, "expect-title") {
        add_expected_text(
            &mut expected,
            title,
            format!("explicit --expect-title `{title}`"),
        );
    }
    for text in repeated_values(map, "expect-text") {
        add_expected_text(
            &mut expected,
            text,
            format!("explicit --expect-text `{text}`"),
        );
    }
    text_alignment_report(mod_root, expected)
}

pub(crate) fn text_alignment_report(
    mod_root: &Path,
    expected: Vec<ExpectedText>,
) -> Result<TextAlignmentReport, String> {
    let actual = collect_mod_alignment_texts(mod_root)?;
    let mut items = Vec::new();
    for expected in dedupe_expected_texts(expected) {
        let key = normalize_text_for_alignment(&expected.text);
        let matches = actual.get(&key).cloned().unwrap_or_default();
        items.push(TextAlignmentItem { expected, matches });
    }
    Ok(TextAlignmentReport {
        mod_root: mod_root.to_path_buf(),
        expected: items,
    })
}

pub(crate) fn expected_texts_from_path(
    input: &Path,
    sheet: Option<&str>,
    tag: &str,
    prefix: &str,
) -> Result<Vec<ExpectedText>, String> {
    let extension = input
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    if matches!(extension.as_str(), "xlsx" | "xls" | "xlsm" | "xlsb" | "ods") {
        let workflow_input = workflow_input_from_path(input, sheet, tag, prefix)?;
        let mut expected = expected_texts_from_workflow_input(
            &workflow_input.text,
            workflow_input.focus_layout.as_ref(),
        );
        for item in &mut expected {
            item.source = format!("{}:{}", input.display(), item.source);
        }
        return Ok(expected);
    }
    let text = read_utf8_lossy(input)?;
    let mut expected = expected_texts_from_workflow_input(&text, None);
    for item in &mut expected {
        item.source = format!("{}:{}", input.display(), item.source);
    }
    Ok(expected)
}

pub(crate) fn expected_texts_from_workflow_input(
    text: &str,
    focus_layout: Option<&FocusLayout>,
) -> Vec<ExpectedText> {
    let mut out = Vec::new();
    if let Some(layout) = focus_layout {
        for focus in &layout.focuses {
            add_expected_text(&mut out, &focus.title, "layout:focus".to_string());
        }
    } else {
        let focus_text = extract_focus_layout_text(text);
        if !focus_text.trim().is_empty() {
            let layout = parse_focus_layout_with_rewards(&focus_text, "TAG", "text_check");
            for focus in &layout.focuses {
                add_expected_text(&mut out, &focus.title, "layout:focus".to_string());
            }
        }
    }

    let feature_text = extract_card_text(text, FEATURE_CARD_HEADERS);
    for card in parse_cards(&feature_text, FEATURE_CARD_HEADERS) {
        if is_player_visible_card_kind(&card.kind) {
            add_expected_text(&mut out, &card.title, format!("card:{}", card.kind));
        }
    }
    let event_text = extract_card_text(text, &["事件"]);
    for card in parse_cards(&event_text, &["事件"]) {
        add_expected_text(&mut out, &card.title, "card:事件".to_string());
        collect_option_text_expectations(&card, &mut out);
    }
    collect_field_text_expectations(text, &mut out);
    dedupe_expected_texts(out)
}

pub(crate) fn collect_option_text_expectations(card: &Card, out: &mut Vec<ExpectedText>) {
    for (key, value) in &card.fields {
        if key.starts_with("选项") && !key.contains("效果") && !key.contains("ai") {
            add_expected_text(out, value, format!("field:{key}"));
        }
    }
}

pub(crate) fn collect_field_text_expectations(text: &str, out: &mut Vec<ExpectedText>) {
    for line in text.lines() {
        let trimmed = line.trim();
        let Some((key, value)) = split_field(trimmed) else {
            continue;
        };
        if is_expected_text_field_key(key) {
            add_expected_text(out, value, format!("field:{key}"));
        }
    }
}

pub(crate) fn is_player_visible_card_kind(kind: &str) -> bool {
    matches!(
        feature_card_type(kind),
        Some("decision" | "idea" | "technology" | "gui")
    )
}

pub(crate) fn is_expected_text_field_key(key: &str) -> bool {
    matches!(
        key,
        "国策"
            | "国策名"
            | "国策名称"
            | "focus"
            | "focus_name"
            | "focus name"
            | "事件"
            | "事件名"
            | "事件名称"
            | "event"
            | "event_name"
            | "event name"
            | "民族精神"
            | "民族精神名"
            | "民族精神名称"
            | "national_spirit"
            | "national spirit"
            | "决议"
            | "决议名"
            | "决议名称"
            | "decision"
            | "decision_name"
            | "decision name"
            | "标题"
            | "title"
    )
}

pub(crate) fn add_expected_text(out: &mut Vec<ExpectedText>, raw: &str, source: String) {
    let mut text = raw.trim().trim_matches('"').trim().to_string();
    if let Some(first_line) = text.lines().next() {
        text = first_line.trim().to_string();
    }
    let (title, _) = parse_focus_token(&text);
    let text = title.trim();
    if !is_alignment_text_candidate(text) {
        return;
    }
    out.push(ExpectedText {
        text: text.to_string(),
        source,
    });
}

pub(crate) fn is_alignment_text_candidate(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() || text == "互斥" {
        return false;
    }
    let chars = text.chars().count();
    (2..=80).contains(&chars)
}

pub(crate) fn dedupe_expected_texts(values: Vec<ExpectedText>) -> Vec<ExpectedText> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for value in values {
        let key = normalize_text_for_alignment(&value.text);
        if key.is_empty() || !seen.insert(key) {
            continue;
        }
        out.push(value);
    }
    out
}

pub(crate) fn collect_mod_alignment_texts(
    mod_root: &Path,
) -> Result<BTreeMap<String, Vec<TextHit>>, String> {
    let mut out: BTreeMap<String, Vec<TextHit>> = BTreeMap::new();
    if !mod_root.exists() {
        return Err(format!("{}: path does not exist", mod_root.display()));
    }
    for file in collect_files(mod_root)? {
        let ext = file
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        let rel = relative_slash_path(mod_root, &file);
        if matches!(ext.as_str(), "yml" | "yaml") && slash_path(&file).contains("/localisation/") {
            for (key, value) in localisation_entries_with_values(&read_utf8_lossy(&file)?) {
                add_actual_hit(&mut out, &value, format!("{rel}:{key}"));
            }
        } else if matches!(ext.as_str(), "txt" | "gui" | "gfx" | "asset" | "mod") {
            let text = strip_comments(&read_utf8_lossy(&file)?);
            for key in ["id", "name"] {
                for value in assignment_values_in_text(&text, key) {
                    add_actual_hit(&mut out, &value, format!("{rel}:{key}"));
                }
            }
        }
    }
    Ok(out)
}

pub(crate) fn add_actual_hit(
    values: &mut BTreeMap<String, Vec<TextHit>>,
    raw: &str,
    source: String,
) {
    let value = raw.trim().trim_matches('"').trim();
    let key = normalize_text_for_alignment(value);
    if key.is_empty() {
        return;
    }
    values.entry(key).or_default().push(TextHit {
        value: value.to_string(),
        source,
    });
}

pub(crate) fn localisation_entries_with_values(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start_matches('\u{feff}').trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("l_") {
            continue;
        }
        let Some((key, rest)) = trimmed.split_once(':') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let value = parse_localisation_line_value(rest);
        if !value.trim().is_empty() {
            out.push((key.to_string(), value));
        }
    }
    out
}

pub(crate) fn parse_localisation_line_value(rest: &str) -> String {
    let mut value = rest.trim_start();
    if let Some(after_zero) = value.strip_prefix('0') {
        value = after_zero.trim_start();
    }
    if let Some(after_quote) = value.strip_prefix('"') {
        let mut out = String::new();
        let mut escape = false;
        for ch in after_quote.chars() {
            if ch == '"' && !escape {
                break;
            }
            if escape {
                out.push(ch);
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else {
                out.push(ch);
            }
        }
        out
    } else {
        value.to_string()
    }
}

pub(crate) fn normalize_text_for_alignment(value: &str) -> String {
    value
        .trim_start_matches('\u{feff}')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

pub(crate) fn text_alignment_report_json(report: &TextAlignmentReport) -> String {
    let mut out = String::new();
    let missing = report.missing();
    out.push_str("{\n");
    out.push_str("  \"schema\": \"hoi4skill.text_alignment.v1\",\n");
    out.push_str(&format!(
        "  \"mod_root\": {},\n",
        json_str(&report.mod_root.display().to_string())
    ));
    out.push_str(&format!("  \"ok\": {},\n", json_bool(missing.is_empty())));
    out.push_str(&format!(
        "  \"expected_count\": {},\n  \"matched_count\": {},\n  \"missing_count\": {},\n",
        report.expected.len(),
        report.matched_count(),
        missing.len()
    ));
    out.push_str("  \"expected\": [\n");
    for (index, item) in report.expected.iter().enumerate() {
        comma(&mut out, index, "    ");
        out.push_str(&text_alignment_item_json(item));
    }
    out.push_str("\n  ],\n  \"missing\": [\n");
    for (index, item) in missing.iter().enumerate() {
        comma(&mut out, index, "    ");
        out.push_str(&format!(
            "{{\"text\": {}, \"source\": {}}}",
            json_str(&item.expected.text),
            json_str(&item.expected.source)
        ));
    }
    out.push_str("\n  ]\n}\n");
    out
}

pub(crate) fn text_alignment_item_json(item: &TextAlignmentItem) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{{\"text\": {}, \"source\": {}, \"matched\": {}, \"matches\": [",
        json_str(&item.expected.text),
        json_str(&item.expected.source),
        json_bool(!item.matches.is_empty())
    ));
    for (index, hit) in item.matches.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!(
            "{{\"value\": {}, \"source\": {}}}",
            json_str(&hit.value),
            json_str(&hit.source)
        ));
    }
    out.push_str("]}");
    out
}
