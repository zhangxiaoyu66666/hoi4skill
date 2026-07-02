//! HOI4 localisation token extraction and comparison.
//!
//! Localisation values are player-visible text mixed with engine-side control
//! fragments. These helpers keep translation and audit flows from treating
//! `$STATE|Y$`, `[ROOT.GetName]`, `§Y...§!`, and `£pol_power` as ordinary prose.

#[allow(unused_imports)]
use crate::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LocalisationToken {
    pub(crate) kind: String,
    pub(crate) text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LocalisationTokenIssue {
    pub(crate) kind: String,
    pub(crate) message: String,
}

#[derive(Clone, Debug)]
pub(crate) struct LocalisationTokenComparison {
    pub(crate) missing: Vec<LocalisationToken>,
    pub(crate) extra: Vec<LocalisationToken>,
    pub(crate) source_issues: Vec<LocalisationTokenIssue>,
    pub(crate) translated_issues: Vec<LocalisationTokenIssue>,
}

#[derive(Clone, Debug)]
pub(crate) struct LocalisationTokenEntry {
    pub(crate) key: String,
    pub(crate) value: String,
    pub(crate) file: String,
    pub(crate) line: usize,
}

pub(crate) fn compile_author_localisation_placeholders(value: &str) -> String {
    compile_author_localisation_placeholders_without_index(value)
}

pub(crate) fn compile_author_localisation_placeholders_without_index(value: &str) -> String {
    let chars = value.char_indices().collect::<Vec<_>>();
    let mut out = String::with_capacity(value.len());
    let mut cursor = 0usize;
    let mut i = 0usize;
    while i < chars.len() {
        let (start, ch) = chars[i];
        if ch != '【' {
            i += 1;
            continue;
        }
        out.push_str(&value[cursor..start]);
        let Some(end_i) = find_author_placeholder_end(&chars, i) else {
            out.push_str(&value[start..]);
            return out;
        };
        let body_start = start + '【'.len_utf8();
        let body_end = chars[end_i].0;
        let name = value[body_start..body_end].trim();
        if let Some((color, body)) = author_color_placeholder(name) {
            out.push_str(color);
            out.push_str(&compile_author_localisation_placeholders_without_index(
                body,
            ));
            out.push_str("§!");
        } else if let Some(replacement) = author_localisation_placeholder_replacement(name) {
            out.push_str(replacement);
        } else {
            out.push_str(&value[start..body_end + '】'.len_utf8()]);
        }
        cursor = body_end + '】'.len_utf8();
        i = end_i + 1;
    }
    out.push_str(&value[cursor..]);
    out
}

pub(crate) fn compile_author_localisation_placeholders_with_index(
    value: &str,
    index: &GameIndex,
) -> Result<String, String> {
    let chars = value.char_indices().collect::<Vec<_>>();
    let mut out = String::with_capacity(value.len());
    let mut cursor = 0usize;
    let mut i = 0usize;
    while i < chars.len() {
        let (start, ch) = chars[i];
        if ch != '【' {
            i += 1;
            continue;
        }
        out.push_str(&value[cursor..start]);
        let Some(end_i) = find_author_placeholder_end(&chars, i) else {
            return Err(
                "authoring placeholder starts with `【` but has no closing `】`".to_string(),
            );
        };
        let body_start = start + '【'.len_utf8();
        let body_end = chars[end_i].0;
        let name = value[body_start..body_end].trim();
        let placeholder = &value[start..body_end + '】'.len_utf8()];
        if let Some((color, body)) = author_color_placeholder(name) {
            out.push_str(color);
            out.push_str(&compile_author_localisation_placeholders_with_index(
                body, index,
            )?);
            out.push_str("§!");
        } else if let Some(replacement) = author_localisation_placeholder_replacement(name) {
            out.push_str(replacement);
        } else if let Some(replacement) =
            country_author_placeholder_replacement(name, placeholder, index)?
        {
            out.push_str(&replacement);
        } else if let Some(replacement) =
            icon_author_placeholder_replacement(name, placeholder, index)?
        {
            out.push_str(&replacement);
        } else {
            return Err(format!(
                "authoring placeholder `{placeholder}` has no built-in HOI4 mapping and no unique country localisation match; use a known scope placeholder like `【对方领导人】`, a scripted localisation token like `[ROOT.GetLeader]`, or add an indexed country localisation entry"
            ));
        }
        cursor = body_end + '】'.len_utf8();
        i = end_i + 1;
    }
    out.push_str(&value[cursor..]);
    Ok(out)
}

pub(crate) fn find_author_placeholder_end(
    chars: &[(usize, char)],
    start_i: usize,
) -> Option<usize> {
    let mut depth = 0usize;
    for (idx, (_, ch)) in chars.iter().enumerate().skip(start_i) {
        match ch {
            '【' => depth += 1,
            '】' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn author_color_placeholder(value: &str) -> Option<(&'static str, &str)> {
    let (name, body) = value.split_once('：').or_else(|| value.split_once(':'))?;
    let color = match name.trim() {
        "红" | "红色" | "赤色" => "§R",
        "绿" | "绿色" => "§G",
        "黄" | "黄色" => "§Y",
        "蓝" | "蓝色" => "§B",
        "白" | "白色" => "§W",
        "灰" | "灰色" | "黑" | "黑色" => "§g",
        "橙" | "橙色" => "§O",
        _ => return None,
    };
    Some((color, body.trim()))
}

pub(crate) fn author_localisation_placeholder_replacement(name: &str) -> Option<&'static str> {
    match name.trim() {
        "我党领导人" | "本党领导人" | "我国领导人" | "本国领导人" | "我方领导人" => {
            Some("[ROOT.GetLeader]")
        }
        "对方领导人" | "该国领导人" | "来源国领导人" | "发起国领导人" => {
            Some("[FROM.GetLeader]")
        }
        "我国" | "本国" | "我方" => Some("[ROOT.GetName]"),
        "我国形容词" | "本国形容词" | "我方形容词" => Some("[ROOT.GetAdjective]"),
        "我国国旗" | "本国国旗" | "我方国旗" | "我国旗子" | "本国旗子" | "我方旗子"
        | "我党党旗" | "本党党旗" | "我国党旗" | "本国党旗" | "我方党旗" => {
            Some("[ROOT.GetFlag]")
        }
        "对方" | "该国" | "来源国" | "发起国" => Some("[FROM.GetName]"),
        "对方形容词" | "该国形容词" | "来源国形容词" | "发起国形容词" => {
            Some("[FROM.GetAdjective]")
        }
        "对方国旗" | "该国国旗" | "来源国国旗" | "发起国国旗" | "对方旗子" | "该国旗子"
        | "来源国旗子" | "发起国旗子" | "对方党旗" | "该国党旗" | "来源国党旗" | "发起国党旗" => {
            Some("[FROM.GetFlag]")
        }
        _ => None,
    }
}

pub(crate) fn country_author_placeholder_replacement(
    name: &str,
    placeholder: &str,
    index: &GameIndex,
) -> Result<Option<String>, String> {
    let (country_name, accessor) =
        author_placeholder_country_accessor(name).unwrap_or((name.trim(), "GetName"));
    if country_name.is_empty() {
        return Ok(None);
    }
    let Some(tags) = index.country_name_tags.get(country_name) else {
        return Ok(None);
    };
    if tags.len() != 1 {
        return Err(format!(
            "authoring placeholder `{placeholder}` matches multiple country tags for `{country_name}`: {}; provide an explicit scripted localisation token",
            tags.iter().cloned().collect::<Vec<_>>().join(", ")
        ));
    }
    let tag = tags.iter().next().expect("checked one tag");
    Ok(Some(format!("[{tag}.{accessor}]")))
}

pub(crate) fn icon_author_placeholder_replacement(
    name: &str,
    placeholder: &str,
    index: &GameIndex,
) -> Result<Option<String>, String> {
    let Some(icon_name) = author_placeholder_icon_name(name) else {
        return Ok(None);
    };
    if icon_name.is_empty() {
        return Ok(None);
    }
    let explicit = icon_name.trim_start_matches('£');
    if is_explicit_localisation_icon_token(explicit) {
        if indexed_localisation_icon_exists(explicit, index) {
            return Ok(Some(format!("£{explicit}")));
        }
        return Err(format!(
            "authoring placeholder `{placeholder}` references icon `{explicit}`, but that sprite was not found in the indexed interface/*.gfx files"
        ));
    }
    let Some(icons) = index.localisation_icon_names.get(icon_name) else {
        return Err(format!(
            "authoring placeholder `{placeholder}` asks for icon `{icon_name}`, but no matching localisation icon was found in indexed localisation. Ask the user whether this concept has an existing icon; if yes, provide it as an explicit placeholder like `【GFX_xxx图标】`. If not, register or add the icon first, then rerun with --game-root/--mod-path so it can be indexed"
        ));
    };
    if icons.len() != 1 {
        return Err(format!(
            "authoring placeholder `{placeholder}` matches multiple localisation icons for `{icon_name}`: {}; provide an explicit icon token like `【GFX_xxx图标】`",
            icons.iter().cloned().collect::<Vec<_>>().join(", ")
        ));
    }
    let icon = icons.iter().next().expect("checked one icon");
    Ok(Some(format!("£{icon}")))
}

pub(crate) fn author_placeholder_icon_name(name: &str) -> Option<&str> {
    let trimmed = name.trim();
    ["图标", "党徽", "徽标", "标志", "icon"]
        .iter()
        .find_map(|suffix| trimmed.strip_suffix(suffix).map(str::trim))
        .filter(|value| !value.is_empty())
}

pub(crate) fn author_placeholder_country_accessor(name: &str) -> Option<(&str, &'static str)> {
    let trimmed = name.trim();
    [
        ("领导人", "GetLeader"),
        ("形容词", "GetAdjective"),
        ("国旗", "GetFlag"),
        ("旗子", "GetFlag"),
        ("旗帜", "GetFlag"),
        ("党旗", "GetFlag"),
    ]
    .iter()
    .find_map(|(suffix, accessor)| {
        trimmed
            .strip_suffix(suffix)
            .map(str::trim)
            .filter(|country| !country.is_empty())
            .map(|country| (country, *accessor))
    })
}

pub(crate) fn is_explicit_localisation_icon_token(value: &str) -> bool {
    value.starts_with("GFX_")
        || value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
            && value.contains('_')
}

pub(crate) fn indexed_localisation_icon_exists(value: &str, index: &GameIndex) -> bool {
    index.sprites.contains(value) || index.sprites.contains(&format!("GFX_{value}"))
}

impl LocalisationTokenComparison {
    pub(crate) fn ok(&self) -> bool {
        self.missing.is_empty()
            && self.extra.is_empty()
            && self.source_issues.is_empty()
            && self.translated_issues.is_empty()
    }
}

pub(crate) fn cmd_localisation_token_report(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = value(&map, "input")
        .or_else(|| map.positionals.first().map(String::as_str))
        .ok_or_else(|| "missing --input <localisation-file-or-dir>".to_string())?;
    let input = normalize_path(input)?;
    let files = collect_localisation_token_files(&input)?;
    let max_items = parse_usize_option(&map, "max-items", usize::MAX)?;
    let json = localisation_token_report_json(&input, &files, max_items)?;
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_localisation_token_check(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let source = value(&map, "source")
        .or_else(|| value(&map, "input"))
        .ok_or_else(|| "missing --source <source-localisation-file-or-dir>".to_string())?;
    let translated = value(&map, "translated")
        .or_else(|| value(&map, "target"))
        .ok_or_else(|| "missing --translated <translated-localisation-file-or-dir>".to_string())?;
    let source = normalize_path(source)?;
    let translated = normalize_path(translated)?;
    let max_items = parse_usize_option(&map, "max-items", usize::MAX)?;
    let report = localisation_token_check_json(&source, &translated, max_items)?;
    write_or_print(&report.json, value(&map, "output"))?;
    if map.flags.contains("strict") && !report.ok {
        return Err(format!(
            "localisation token check failed: {} mismatch(es), {} missing translation(s), {} extra translation key(s)",
            report.mismatch_count, report.missing_translation_count, report.extra_translation_count
        ));
    }
    Ok(())
}

pub(crate) struct LocalisationTokenCheckReport {
    pub(crate) json: String,
    pub(crate) ok: bool,
    pub(crate) mismatch_count: usize,
    pub(crate) missing_translation_count: usize,
    pub(crate) extra_translation_count: usize,
}

pub(crate) fn cmd_author_placeholder_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = author_placeholder_text_from_args(&map)?;
    let dependency_mods = dependency_mod_roots(&map)?;
    let game_root = normalize_path(
        value(&map, "game-root")
            .ok_or_else(|| "author-placeholder-plan requires --game-root".to_string())?,
    )?;
    let index = build_game_index_with_mod_paths(&game_root, &dependency_mods)?;
    let report = author_placeholder_plan_json(&text, &index);
    write_or_print(&report.json, value(&map, "output"))?;
    if report.ok {
        Ok(())
    } else {
        Err("author placeholder plan blocked unresolved placeholders; answer questions before final localisation".to_string())
    }
}

pub(crate) struct AuthorPlaceholderPlanReport {
    pub(crate) json: String,
    pub(crate) ok: bool,
}

pub(crate) fn author_placeholder_text_from_args(map: &ArgMap) -> Result<String, String> {
    if let Some(text) = value(map, "text").or_else(|| value(map, "copy")) {
        return Ok(text.to_string());
    }
    if let Some(input) = value(map, "input") {
        return read_text_document(&normalize_path(input)?);
    }
    Err("author-placeholder-plan requires --text or --input".to_string())
}

pub(crate) fn author_placeholder_plan_json(
    text: &str,
    index: &GameIndex,
) -> AuthorPlaceholderPlanReport {
    let placeholders = collect_author_placeholders(text);
    let mut items = Vec::new();
    let mut questions = Vec::new();
    let mut asset_questions = Vec::new();
    let mut country_questions = Vec::new();
    let mut blockers = Vec::new();
    let mut resolved_count = 0usize;
    for placeholder in &placeholders {
        let resolution = author_placeholder_resolution_json(placeholder, index);
        if resolution.resolved {
            resolved_count += 1;
        }
        if let Some(question) = resolution.question {
            if is_author_icon_placeholder_name(&placeholder.name) {
                asset_questions.push(question.clone());
            } else if is_author_country_placeholder_name(&placeholder.name, index) {
                country_questions.push(question.clone());
            }
            questions.push(question);
        }
        if let Some(blocker) = resolution.blocker {
            blockers.push(blocker);
        }
        items.push(resolution.json);
    }
    let compiled_text = if blockers.is_empty() {
        compile_author_localisation_placeholders_with_index(text, index).ok()
    } else {
        None
    };
    let token_issues = extract_localisation_tokens(text).1;
    let (input_tokens, _) = extract_localisation_tokens(text);
    for issue in &token_issues {
        if issue.kind == "unclosed_author_placeholder" {
            blockers.push(issue.message.clone());
            questions.push("补全缺失的 `】`，或删除未闭合的作者占位符后重新运行。".to_string());
        }
    }
    blockers.sort();
    blockers.dedup();
    questions.sort();
    questions.dedup();
    asset_questions.sort();
    asset_questions.dedup();
    country_questions.sort();
    country_questions.dedup();
    let ok = blockers.is_empty();
    let rules = vec![
        "Do not guess icons, country tags, cosmetic tags, leaders, or flag tokens when no unique indexed match exists.".to_string(),
        "If a question asks for a GFX sprite, the user must provide an existing indexed sprite or approve adding/registering one first.".to_string(),
        "If a country/cosmetic name is ambiguous, the user must provide the exact tag or scripted localisation token.".to_string(),
        "Only compiled_text may be written to final localisation; raw 【...】 author placeholders are not final HOI4 localisation.".to_string(),
    ];
    let token_rules = vec![
        "Preserve scripted localisation tokens such as [ROOT.GetNameDef], [FROM.GetLeader], and [?var|Y0%].".to_string(),
        "Preserve variable tokens such as $STATE|Y$ and engine icon tokens such as £pol_power.".to_string(),
        "Preserve colour wrappers as balanced §X ... §! pairs; nested author colour placeholders must compile before writing.".to_string(),
        "Block raw 【...】 author placeholders in final localisation unless this plan resolves them into compiled_text.".to_string(),
    ];
    let compiled_tokens = compiled_text
        .as_deref()
        .map(|value| extract_localisation_tokens(value).0)
        .unwrap_or_default();
    let alias_graph = author_placeholder_alias_graph_json(&placeholders, index);
    let json = format!(
        "{{\n  \"schema\": \"hoi4skill.author_placeholder_plan.v1\",\n  \"ok\": {},\n  \"status\": {},\n  \"input\": {},\n  \"compiled_text\": {},\n  \"placeholder_count\": {},\n  \"resolved_count\": {},\n  \"question_count\": {},\n  \"asset_question_count\": {},\n  \"country_question_count\": {},\n  \"blocker_count\": {},\n  \"placeholders\": [{}],\n  \"questions\": {},\n  \"asset_questions\": {},\n  \"country_questions\": {},\n  \"blockers\": {},\n  \"cosmetic_alias_graph\": {},\n  \"input_token_inventory\": {},\n  \"compiled_token_inventory\": {},\n  \"token_preservation_rules\": {},\n  \"anti_hallucination_rules\": {}\n}}\n",
        json_bool(ok),
        json_str(if ok { "ok" } else { "questions_required" }),
        json_str(text),
        compiled_text
            .as_deref()
            .map(json_str)
            .unwrap_or_else(|| "null".to_string()),
        placeholders.len(),
        resolved_count,
        questions.len(),
        asset_questions.len(),
        country_questions.len(),
        blockers.len(),
        items.join(", "),
        json_array(&questions),
        json_array(&asset_questions),
        json_array(&country_questions),
        json_array(&blockers),
        alias_graph,
        localisation_tokens_json(&input_tokens),
        localisation_tokens_json(&compiled_tokens),
        json_array(&token_rules),
        json_array(&rules)
    );
    AuthorPlaceholderPlanReport { json, ok }
}

fn is_author_icon_placeholder_name(name: &str) -> bool {
    author_placeholder_icon_name(name).is_some()
}

fn is_author_country_placeholder_name(name: &str, index: &GameIndex) -> bool {
    let trimmed = name.trim();
    if trimmed.ends_with("领导人")
        || trimmed.ends_with("形容词")
        || author_placeholder_country_accessor(trimmed)
            .is_some_and(|(_, accessor)| accessor == "GetFlag")
    {
        return true;
    }
    let country_name = author_placeholder_country_name(trimmed);
    index.country_name_tags.contains_key(&country_name)
}

#[derive(Clone)]
pub(crate) struct AuthorPlaceholder {
    pub(crate) text: String,
    pub(crate) name: String,
}

pub(crate) struct AuthorPlaceholderResolution {
    pub(crate) json: String,
    pub(crate) resolved: bool,
    pub(crate) question: Option<String>,
    pub(crate) blocker: Option<String>,
}

pub(crate) fn collect_author_placeholders(value: &str) -> Vec<AuthorPlaceholder> {
    let mut out = Vec::new();
    collect_author_placeholders_into(value, &mut out);
    out
}

fn collect_author_placeholders_into(value: &str, out: &mut Vec<AuthorPlaceholder>) {
    let chars = value.char_indices().collect::<Vec<_>>();
    let mut i = 0usize;
    while i < chars.len() {
        let (start, ch) = chars[i];
        if ch != '【' {
            i += 1;
            continue;
        }
        let Some(end_i) = find_author_placeholder_end(&chars, i) else {
            break;
        };
        let body_start = start + '【'.len_utf8();
        let body_end = chars[end_i].0;
        let name = value[body_start..body_end].trim().to_string();
        let text = value[start..body_end + '】'.len_utf8()].to_string();
        out.push(AuthorPlaceholder {
            text: text.clone(),
            name: name.clone(),
        });
        if let Some((_, body)) = author_color_placeholder(&name) {
            collect_author_placeholders_into(body, out);
        }
        i = end_i + 1;
    }
}

pub(crate) fn author_placeholder_resolution_json(
    placeholder: &AuthorPlaceholder,
    index: &GameIndex,
) -> AuthorPlaceholderResolution {
    let name = placeholder.name.trim();
    let kind = author_placeholder_kind(name, index);
    let mut status = "resolved".to_string();
    let mut replacement = None;
    let mut question = None;
    let mut blocker = None;

    if let Some((color, _body)) = author_color_placeholder(name) {
        replacement = Some(color.to_string());
    } else if let Some(value) = author_localisation_placeholder_replacement(name) {
        replacement = Some(value.to_string());
    } else {
        match country_author_placeholder_replacement(name, &placeholder.text, index) {
            Ok(Some(value)) => replacement = Some(value),
            Ok(None) => match icon_author_placeholder_replacement(name, &placeholder.text, index) {
                Ok(Some(value)) => replacement = Some(value),
                Ok(None) => {
                    status = "question_required".to_string();
                    let q = author_placeholder_question(name, &placeholder.text, index);
                    blocker = Some(q.clone());
                    question = Some(q);
                }
                Err(err) => {
                    status = "question_required".to_string();
                    question = Some(author_placeholder_error_question(
                        name,
                        &placeholder.text,
                        &err,
                    ));
                    blocker = Some(err);
                }
            },
            Err(err) => {
                status = "question_required".to_string();
                question = Some(author_placeholder_error_question(
                    name,
                    &placeholder.text,
                    &err,
                ));
                blocker = Some(err);
            }
        }
    }

    let candidates = author_placeholder_candidates(name, index);
    let json = format!(
        "{{\"placeholder\": {}, \"name\": {}, \"kind\": {}, \"status\": {}, \"replacement\": {}, \"candidates\": {}, \"question\": {}, \"blocker\": {}}}",
        json_str(&placeholder.text),
        json_str(name),
        json_str(&kind),
        json_str(&status),
        replacement
            .as_deref()
            .map(json_str)
            .unwrap_or_else(|| "null".to_string()),
        json_array(&candidates),
        question
            .as_deref()
            .map(json_str)
            .unwrap_or_else(|| "null".to_string()),
        blocker
            .as_deref()
            .map(json_str)
            .unwrap_or_else(|| "null".to_string())
    );
    AuthorPlaceholderResolution {
        json,
        resolved: status == "resolved",
        question,
        blocker,
    }
}

pub(crate) fn author_placeholder_kind(name: &str, index: &GameIndex) -> String {
    if author_color_placeholder(name).is_some() {
        return "color_span".to_string();
    }
    if author_localisation_placeholder_replacement(name).is_some() {
        return "scope_scripted_loc".to_string();
    }
    if author_placeholder_icon_name(name).is_some() {
        return "icon".to_string();
    }
    if let Some((_, accessor)) = author_placeholder_country_accessor(name) {
        return match accessor {
            "GetLeader" => "country_or_cosmetic_leader_alias",
            "GetAdjective" => "country_or_cosmetic_adjective_alias",
            "GetFlag" => "country_or_cosmetic_flag_alias",
            _ => "country_or_cosmetic_alias",
        }
        .to_string();
    }
    let country_name = author_placeholder_country_name(name);
    if index.country_name_tags.contains_key(&country_name) {
        "country_scripted_loc".to_string()
    } else {
        "unknown".to_string()
    }
}

pub(crate) fn author_placeholder_alias_graph_json(
    placeholders: &[AuthorPlaceholder],
    index: &GameIndex,
) -> String {
    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();
    for placeholder in placeholders {
        let name = placeholder.name.trim();
        if author_color_placeholder(name).is_some()
            || author_localisation_placeholder_replacement(name).is_some()
            || author_placeholder_icon_name(name).is_some()
        {
            continue;
        }
        let alias = author_placeholder_country_name(name);
        if alias.is_empty() || !seen.insert(alias.clone()) {
            continue;
        }
        let tags = index
            .country_name_tags
            .get(&alias)
            .map(|values| values.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        let status = match tags.len() {
            0 => "missing",
            1 => "unique",
            _ => "ambiguous",
        };
        rows.push(format!(
            "{{\"alias\": {}, \"status\": {}, \"tag_candidates\": {}}}",
            json_str(&alias),
            json_str(status),
            json_array(&tags)
        ));
    }
    format!("[{}]", rows.join(", "))
}

pub(crate) fn author_placeholder_country_name(name: &str) -> String {
    author_placeholder_country_accessor(name)
        .map(|(country, _)| country)
        .unwrap_or(name)
        .trim()
        .to_string()
}

pub(crate) fn author_placeholder_candidates(name: &str, index: &GameIndex) -> Vec<String> {
    if let Some(icon_name) = author_placeholder_icon_name(name) {
        if let Some(icons) = index.localisation_icon_names.get(icon_name) {
            return icons.iter().map(|icon| format!("£{icon}")).collect();
        }
        return Vec::new();
    }
    let country_name = author_placeholder_country_name(name);
    index
        .country_name_tags
        .get(&country_name)
        .map(|tags| tags.iter().cloned().collect())
        .unwrap_or_default()
}

pub(crate) fn author_placeholder_question(
    name: &str,
    placeholder: &str,
    index: &GameIndex,
) -> String {
    if let Some(icon_name) = author_placeholder_icon_name(name) {
        let explicit = icon_name.trim_start_matches('£');
        if is_explicit_localisation_icon_token(explicit) {
            return format!(
                "{placeholder} 指向显式图标 `{explicit}`，但索引里没有这个 sprite。这个图标是否已经存在？如果存在，请提供 interface/*.gfx 中的正确 sprite 名；如果不存在，请先注册/添加图标。"
            );
        }
        return format!(
            "{placeholder} 需要图标 `{icon_name}`，但索引里没有唯一匹配。这个概念是否已有图标？如果有，请提供 GFX sprite 名；如果没有，请先创建/注册图标后再生成。"
        );
    }
    let country_name = author_placeholder_country_name(name);
    if !country_name.is_empty() {
        if let Some(tags) = index.country_name_tags.get(&country_name) {
            return format!(
                "{placeholder} 匹配多个国家/cosmetic tag：{}。请指定确切 tag，或直接提供 `[TAG.GetLeader]` / `[TAG.GetName]` / `[TAG.GetFlag]` 这样的 scripted localisation。",
                tags.iter().cloned().collect::<Vec<_>>().join(", ")
            );
        }
        return format!(
            "{placeholder} 找不到国家或 cosmetic tag 别名 `{country_name}`。请说明它对应哪个 tag/cosmetic tag，或提供明确 scripted localisation token。"
        );
    }
    format!("{placeholder} 不是已知作者占位符。请提供明确的 HOI4 scripted localisation token 或改写为已支持占位符。")
}

pub(crate) fn author_placeholder_error_question(
    name: &str,
    placeholder: &str,
    err: &str,
) -> String {
    if err.contains("icon") || author_placeholder_icon_name(name).is_some() {
        return author_placeholder_question(name, placeholder, &GameIndex::default());
    }
    if err.contains("multiple country tags") {
        return format!(
            "{placeholder} 匹配多个国家/cosmetic tag。请指定确切 tag，或直接提供 `[TAG.GetLeader]` / `[TAG.GetName]` / `[TAG.GetFlag]`。"
        );
    }
    format!("{err}。请补充明确映射后重新运行。")
}

pub(crate) fn collect_localisation_token_files(input: &Path) -> Result<Vec<PathBuf>, String> {
    if input.is_file() {
        return Ok(vec![input.to_path_buf()]);
    }
    if !input.exists() {
        return Err(format!(
            "localisation input does not exist: {}",
            input.display()
        ));
    }
    let mut files = collect_files(input)?
        .into_iter()
        .filter(|path| is_localisation_yml(path))
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

pub(crate) fn collect_localisation_token_entries(
    input: &Path,
) -> Result<BTreeMap<String, LocalisationTokenEntry>, String> {
    let mut entries = BTreeMap::new();
    for file in collect_localisation_token_files(input)? {
        let text = read_utf8_lossy(&file)?;
        for (line_no, line) in text.lines().enumerate() {
            let Some((key, value)) = parse_localisation_line(line) else {
                continue;
            };
            entries
                .entry(key.clone())
                .or_insert(LocalisationTokenEntry {
                    key,
                    value,
                    file: slash_path(&file),
                    line: line_no + 1,
                });
        }
    }
    Ok(entries)
}

pub(crate) fn localisation_token_check_json(
    source: &Path,
    translated: &Path,
    max_items: usize,
) -> Result<LocalisationTokenCheckReport, String> {
    let source_entries = collect_localisation_token_entries(source)?;
    let translated_entries = collect_localisation_token_entries(translated)?;
    let mut mismatches = Vec::new();
    let mut missing_translation = Vec::new();
    let mut extra_translation = Vec::new();
    for (key, source_entry) in &source_entries {
        let Some(translated_entry) = translated_entries.get(key) else {
            missing_translation.push(key.clone());
            continue;
        };
        let comparison = compare_localisation_tokens(&source_entry.value, &translated_entry.value);
        if !comparison.ok() {
            mismatches.push(format!(
                "{{\"key\": {}, \"source\": {}, \"translated\": {}, \"comparison\": {}}}",
                json_str(key),
                localisation_token_entry_json(source_entry),
                localisation_token_entry_json(translated_entry),
                localisation_token_comparison_json(&comparison)
            ));
        }
    }
    for key in translated_entries.keys() {
        if !source_entries.contains_key(key) {
            extra_translation.push(key.clone());
        }
    }
    let mismatch_count = mismatches.len();
    let missing_translation_count = missing_translation.len();
    let extra_translation_count = extra_translation.len();
    let ok = mismatch_count == 0 && missing_translation_count == 0;
    let json = format!(
        "{{\n  \"schema\": \"hoi4skill.localisation_token_check.v1\",\n  \"source\": {},\n  \"translated\": {},\n  \"status\": {},\n  \"source_keys_total\": {},\n  \"translated_keys_total\": {},\n  \"mismatch_count\": {},\n  \"missing_translation_count\": {},\n  \"extra_translation_count\": {},\n  \"mismatches\": [{}],\n  \"missing_translation\": {},\n  \"extra_translation\": {}\n}}\n",
        json_str(&source.display().to_string()),
        json_str(&translated.display().to_string()),
        json_str(if ok { "ok" } else { "blocked" }),
        source_entries.len(),
        translated_entries.len(),
        mismatch_count,
        missing_translation_count,
        extra_translation_count,
        mismatches
            .iter()
            .take(max_items)
            .cloned()
            .collect::<Vec<_>>()
            .join(", "),
        json_array(
            &missing_translation
                .iter()
                .take(max_items)
                .cloned()
                .collect::<Vec<_>>()
        ),
        json_array(
            &extra_translation
                .iter()
                .take(max_items)
                .cloned()
                .collect::<Vec<_>>()
        )
    );
    Ok(LocalisationTokenCheckReport {
        json,
        ok,
        mismatch_count,
        missing_translation_count,
        extra_translation_count,
    })
}

pub(crate) fn localisation_token_report_json(
    input: &Path,
    files: &[PathBuf],
    max_items: usize,
) -> Result<String, String> {
    let mut entries = Vec::new();
    let mut token_count = 0usize;
    let mut issue_count = 0usize;
    for file in files {
        let text = read_utf8_lossy(file)?;
        for (line_no, line) in text.lines().enumerate() {
            let Some((key, value)) = parse_localisation_line(line) else {
                continue;
            };
            let (tokens, issues) = extract_localisation_tokens(&value);
            token_count += tokens.len();
            issue_count += issues.len();
            if entries.len() < max_items {
                entries.push(format!(
                    "{{\"file\": {}, \"line\": {}, \"key\": {}, \"value\": {}, \"tokens\": {}, \"issues\": {}}}",
                    json_str(&slash_path(file)),
                    line_no + 1,
                    json_str(&key),
                    json_str(&value),
                    localisation_tokens_json(&tokens),
                    localisation_token_issues_json(&issues)
                ));
            }
        }
    }
    Ok(format!(
        "{{\n  \"schema\": \"hoi4skill.localisation_tokens.v1\",\n  \"input\": {},\n  \"files_total\": {},\n  \"entries_reported\": {},\n  \"tokens_total\": {},\n  \"issues_total\": {},\n  \"entries\": [\n    {}\n  ]\n}}\n",
        json_str(&input.display().to_string()),
        files.len(),
        entries.len(),
        token_count,
        issue_count,
        entries.join(",\n    ")
    ))
}

pub(crate) fn localisation_token_entry_json(entry: &LocalisationTokenEntry) -> String {
    format!(
        "{{\"file\": {}, \"line\": {}, \"key\": {}, \"value\": {}}}",
        json_str(&entry.file),
        entry.line,
        json_str(&entry.key),
        json_str(&entry.value)
    )
}

pub(crate) fn extract_localisation_tokens(
    value: &str,
) -> (Vec<LocalisationToken>, Vec<LocalisationTokenIssue>) {
    let chars = value.char_indices().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut issues = Vec::new();
    let mut color_active = false;
    let mut i = 0usize;
    while i < chars.len() {
        let (start, ch) = chars[i];
        match ch {
            '§' => {
                if let Some((end_idx, marker)) = chars.get(i + 1).copied() {
                    let end = end_idx + marker.len_utf8();
                    let text = value[start..end].to_string();
                    if marker == '!' {
                        if !color_active {
                            issues.push(LocalisationTokenIssue {
                                kind: "orphan_color_reset".to_string(),
                                message: format!(
                                    "colour reset `{text}` has no preceding colour code"
                                ),
                            });
                        } else {
                            color_active = false;
                        }
                        tokens.push(LocalisationToken {
                            kind: "color_reset".to_string(),
                            text,
                        });
                    } else {
                        color_active = true;
                        tokens.push(LocalisationToken {
                            kind: "color".to_string(),
                            text,
                        });
                    }
                    i += 2;
                } else {
                    issues.push(LocalisationTokenIssue {
                        kind: "dangling_color_marker".to_string(),
                        message: "dangling `§` at end of localisation value".to_string(),
                    });
                    i += 1;
                }
            }
            '$' => {
                if let Some(end_i) = find_next_char(&chars, i + 1, '$') {
                    let end = chars[end_i].0 + '$'.len_utf8();
                    tokens.push(LocalisationToken {
                        kind: "variable".to_string(),
                        text: value[start..end].to_string(),
                    });
                    i = end_i + 1;
                } else {
                    issues.push(LocalisationTokenIssue {
                        kind: "unclosed_variable".to_string(),
                        message: "localisation variable starts with `$` but has no closing `$`"
                            .to_string(),
                    });
                    i += 1;
                }
            }
            '[' => {
                if let Some(end_i) = find_next_char(&chars, i + 1, ']') {
                    let end = chars[end_i].0 + ']'.len_utf8();
                    let text = value[start..end].to_string();
                    let kind = if text.starts_with("[?") {
                        "numeric_variable"
                    } else {
                        "scripted_loc"
                    };
                    tokens.push(LocalisationToken {
                        kind: kind.to_string(),
                        text,
                    });
                    i = end_i + 1;
                } else {
                    issues.push(LocalisationTokenIssue {
                        kind: "unclosed_scripted_loc".to_string(),
                        message: "scripted localisation starts with `[` but has no closing `]`"
                            .to_string(),
                    });
                    i += 1;
                }
            }
            '【' => {
                if let Some(end_i) = find_next_char(&chars, i + 1, '】') {
                    let end = chars[end_i].0 + '】'.len_utf8();
                    let text = value[start..end].to_string();
                    let body_start = start + '【'.len_utf8();
                    let name = value[body_start..chars[end_i].0].trim();
                    tokens.push(LocalisationToken {
                        kind: "author_placeholder".to_string(),
                        text: text.clone(),
                    });
                    let message = if let Some(replacement) =
                        author_localisation_placeholder_replacement(name)
                    {
                        format!(
                                "authoring placeholder `{text}` must be compiled to `{replacement}` before final localisation"
                            )
                    } else {
                        format!(
                                "authoring placeholder `{text}` has no built-in HOI4 mapping; replace it with a scripted localisation token like `[ROOT.GetLeader]` or provide an explicit mapping before final localisation"
                            )
                    };
                    issues.push(LocalisationTokenIssue {
                        kind: "unresolved_author_placeholder".to_string(),
                        message,
                    });
                    i = end_i + 1;
                } else {
                    issues.push(LocalisationTokenIssue {
                        kind: "unclosed_author_placeholder".to_string(),
                        message: "authoring placeholder starts with `【` but has no closing `】`"
                            .to_string(),
                    });
                    i += 1;
                }
            }
            '£' => {
                let mut end_i = i + 1;
                while let Some((_, next)) = chars.get(end_i) {
                    if next.is_ascii_alphanumeric() || matches!(next, '_' | '.' | '-') {
                        end_i += 1;
                    } else {
                        break;
                    }
                }
                if end_i == i + 1 {
                    issues.push(LocalisationTokenIssue {
                        kind: "empty_icon".to_string(),
                        message: "icon marker `£` has no icon name".to_string(),
                    });
                    i += 1;
                } else {
                    let end = if let Some((idx, _)) = chars.get(end_i) {
                        *idx
                    } else {
                        value.len()
                    };
                    tokens.push(LocalisationToken {
                        kind: "icon".to_string(),
                        text: value[start..end].to_string(),
                    });
                    i = end_i;
                }
            }
            '\\' => {
                if chars.get(i + 1).is_some_and(|(_, next)| *next == 'n') {
                    let end = chars[i + 1].0 + 'n'.len_utf8();
                    tokens.push(LocalisationToken {
                        kind: "newline".to_string(),
                        text: value[start..end].to_string(),
                    });
                    i += 2;
                } else {
                    i += 1;
                }
            }
            '\n' => {
                tokens.push(LocalisationToken {
                    kind: "newline".to_string(),
                    text: "\\n".to_string(),
                });
                i += 1;
            }
            '^' => {
                tokens.push(LocalisationToken {
                    kind: "control".to_string(),
                    text: "^".to_string(),
                });
                i += 1;
            }
            '%' => {
                tokens.push(LocalisationToken {
                    kind: "percent".to_string(),
                    text: "%".to_string(),
                });
                i += 1;
            }
            _ => i += 1,
        }
    }
    if color_active {
        issues.push(LocalisationTokenIssue {
            kind: "unclosed_color".to_string(),
            message: "colour code is missing `§!`".to_string(),
        });
    }
    (tokens, issues)
}

pub(crate) fn compare_localisation_tokens(
    source: &str,
    translated: &str,
) -> LocalisationTokenComparison {
    let (source_tokens, source_issues) = extract_localisation_tokens(source);
    let (translated_tokens, translated_issues) = extract_localisation_tokens(translated);
    let source_counts = localisation_token_counts(&source_tokens);
    let translated_counts = localisation_token_counts(&translated_tokens);
    let mut missing = Vec::new();
    let mut extra = Vec::new();
    for (token, count) in &source_counts {
        let translated_count = translated_counts.get(token).copied().unwrap_or(0);
        for _ in 0..count.saturating_sub(translated_count) {
            missing.push(token.clone());
        }
    }
    for (token, count) in &translated_counts {
        let source_count = source_counts.get(token).copied().unwrap_or(0);
        for _ in 0..count.saturating_sub(source_count) {
            extra.push(token.clone());
        }
    }
    LocalisationTokenComparison {
        missing,
        extra,
        source_issues,
        translated_issues,
    }
}

pub(crate) fn localisation_token_counts(
    tokens: &[LocalisationToken],
) -> BTreeMap<LocalisationToken, usize> {
    let mut counts = BTreeMap::new();
    for token in tokens {
        *counts.entry(token.clone()).or_insert(0) += 1;
    }
    counts
}

impl Ord for LocalisationToken {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.kind
            .cmp(&other.kind)
            .then_with(|| self.text.cmp(&other.text))
    }
}

impl PartialOrd for LocalisationToken {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub(crate) fn find_next_char(chars: &[(usize, char)], start: usize, needle: char) -> Option<usize> {
    (start..chars.len()).find(|idx| chars[*idx].1 == needle)
}

pub(crate) fn localisation_tokens_json(tokens: &[LocalisationToken]) -> String {
    format!(
        "[{}]",
        tokens
            .iter()
            .map(|token| {
                format!(
                    "{{\"kind\": {}, \"text\": {}}}",
                    json_str(&token.kind),
                    json_str(&token.text)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn localisation_token_issues_json(issues: &[LocalisationTokenIssue]) -> String {
    format!(
        "[{}]",
        issues
            .iter()
            .map(|issue| {
                format!(
                    "{{\"kind\": {}, \"message\": {}}}",
                    json_str(&issue.kind),
                    json_str(&issue.message)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn localisation_token_comparison_json(
    comparison: &LocalisationTokenComparison,
) -> String {
    format!(
        "{{\"missing\": {}, \"extra\": {}, \"source_issues\": {}, \"translated_issues\": {}}}",
        localisation_tokens_json(&comparison.missing),
        localisation_tokens_json(&comparison.extra),
        localisation_token_issues_json(&comparison.source_issues),
        localisation_token_issues_json(&comparison.translated_issues)
    )
}
