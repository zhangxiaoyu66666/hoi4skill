//! One-sentence and mixed-card workflows that orchestrate generation and validation.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_run_workflow(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let tag = value(&map, "tag").unwrap_or("TAG");
    let prefix = value(&map, "prefix").unwrap_or("mod");
    let sheet = value(&map, "sheet");
    let tree_id = value(&map, "tree-id");
    let dry_run = map.flags.contains("dry-run") || mod_root.is_none();
    let dependency_mods = dependency_mod_roots(&map)?;
    let game_index = value(&map, "game-root")
        .map(normalize_path)
        .transpose()?
        .map(|path| build_game_index_with_mod_paths(&path, &dependency_mods))
        .transpose()?;
    if game_index.is_none() && !dependency_mods.is_empty() {
        return Err("--mod-path requires --game-root during workflow generation".to_string());
    }
    let text = workflow_input_text_from_path(&input, sheet, tag, prefix)?;
    let json = run_workflow_json(
        &text,
        mod_root.as_deref(),
        tag,
        prefix,
        tree_id,
        dry_run,
        game_index.as_ref(),
    )?;
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn workflow_input_text_from_path(
    input: &Path,
    sheet: Option<&str>,
    tag: &str,
    prefix: &str,
) -> Result<String, String> {
    let extension = input
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    if matches!(extension.as_str(), "xlsx" | "xls" | "xlsm" | "xlsb" | "ods") {
        return render_focus_excel_workflow_input(input, sheet, tag, prefix);
    }
    read_utf8_lossy(input)
}

pub(crate) fn render_focus_excel_workflow_input(
    input: &Path,
    sheet: Option<&str>,
    tag: &str,
    prefix: &str,
) -> Result<String, String> {
    let markdown = render_focus_excel_markdown(input, sheet, tag, prefix)?;
    let (_sheet_name, imported) = read_focus_excel_import(input, sheet)?;
    let sketch = render_excel_focus_import_sketch(&imported)?;
    Ok(format!("{markdown}\n\n国策树：\n{sketch}"))
}

pub(crate) fn render_excel_focus_import_sketch(
    imported: &ExcelFocusImport,
) -> Result<String, String> {
    let mut row_tokens: BTreeMap<usize, BTreeMap<usize, String>> = BTreeMap::new();
    for cell in &imported.cells {
        let token = cell
            .id_hint
            .as_deref()
            .map(|hint| format!("{} | {}", cell.title, hint))
            .unwrap_or_else(|| cell.title.clone());
        row_tokens
            .entry(cell.row)
            .or_default()
            .insert(cell.column, token);
    }
    for (row, column) in &imported.mutual_markers {
        row_tokens
            .entry(*row)
            .or_default()
            .insert(*column, "互斥".to_string());
    }
    if row_tokens.is_empty() {
        return Err("worksheet did not contain any focus cells".to_string());
    }

    let mut out = String::new();
    for columns in row_tokens.values() {
        let line = columns
            .values()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("    ");
        if !line.trim().is_empty() {
            out.push_str(&line);
            out.push('\n');
        }
    }
    Ok(out)
}

pub(crate) fn cmd_generate_mod(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = one_sentence_input_text(&map)?;
    let source_roots = generate_mod_source_roots(&map)?;
    let country = infer_country_from_sources(&text, &source_roots)?
        .or_else(|| infer_country_from_text(&text));
    let tag = value(&map, "tag")
        .map(|value| sanitize_identifier_part(value, "TAG").to_ascii_uppercase())
        .or_else(|| country.as_ref().map(|country| country.tag.clone()))
        .unwrap_or_else(|| "TAG".to_string());
    let title = infer_one_sentence_title(&text);
    let prefix = value(&map, "prefix")
        .map(|value| sanitize_identifier_part(value, "mod"))
        .unwrap_or_else(|| infer_one_sentence_prefix(&tag, &title));
    let name = value(&map, "name")
        .map(str::to_string)
        .unwrap_or_else(|| infer_one_sentence_mod_name(country.as_ref(), &title));
    let output = value(&map, "output")
        .map(normalize_path)
        .transpose()?
        .unwrap_or_else(|| {
            env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(format!("{}_mod", sanitize_identifier_part(&prefix, "mod")))
        });
    let tags = value(&map, "tags").unwrap_or("Alternative History");
    let version = value(&map, "version").unwrap_or("0.1.0");
    let supported_version = value(&map, "supported-version").unwrap_or("*");
    let dry_run = map.flags.contains("dry-run");
    let launcher_file = map.flags.contains("launcher-file");
    let request = GenerateModRequest {
        text: &text,
        mod_root: &output,
        name: &name,
        tag: &tag,
        prefix: &prefix,
        tags,
        version,
        supported_version,
        launcher_file,
        dry_run,
        country_source: country.as_ref().map(|country| country.source.as_str()),
    };
    let json = generate_mod_json(&request)?;
    write_or_print(&json, value(&map, "report"))
}

pub(crate) struct GenerateModRequest<'a> {
    pub(crate) text: &'a str,
    pub(crate) mod_root: &'a Path,
    pub(crate) name: &'a str,
    pub(crate) tag: &'a str,
    pub(crate) prefix: &'a str,
    pub(crate) tags: &'a str,
    pub(crate) version: &'a str,
    pub(crate) supported_version: &'a str,
    pub(crate) launcher_file: bool,
    pub(crate) dry_run: bool,
    pub(crate) country_source: Option<&'a str>,
}

pub(crate) const FEATURE_CARD_HEADERS: &[&str] = &[
    "决议",
    "民族精神",
    "科技",
    "独有科技",
    "特殊科技",
    "特殊GUI",
    "特殊 GUI",
    "GUI",
    "界面",
    "脚本效果",
    "scripted_effect",
    "scripted effect",
    "脚本触发",
    "scripted_trigger",
    "scripted trigger",
    "州效果",
    "州编辑",
    "州改动",
    "省份效果",
    "地区效果",
    "state_effect",
    "state edit",
    "state_edit",
];

pub(crate) fn feature_card_type(kind: &str) -> Option<&'static str> {
    match kind {
        "决议" => Some("decision"),
        "民族精神" => Some("idea"),
        "科技" | "独有科技" | "特殊科技" => Some("technology"),
        "特殊GUI" | "特殊 GUI" | "GUI" | "界面" => Some("gui"),
        "脚本效果" | "scripted_effect" | "scripted effect" => Some("scripted_effect"),
        "脚本触发" | "scripted_trigger" | "scripted trigger" => Some("scripted_trigger"),
        "州效果" | "州编辑" | "州改动" | "省份效果" | "地区效果" | "state_effect"
        | "state edit" | "state_edit" => Some("state_effect"),
        _ => None,
    }
}

pub(crate) fn is_feature_card_header(kind: &str) -> bool {
    feature_card_type(kind).is_some()
}

pub(crate) fn is_technology_card(kind: &str) -> bool {
    feature_card_type(kind) == Some("technology")
}

pub(crate) fn is_gui_card(kind: &str) -> bool {
    feature_card_type(kind) == Some("gui")
}

pub(crate) fn is_scripted_effect_card(kind: &str) -> bool {
    feature_card_type(kind) == Some("scripted_effect")
}

pub(crate) fn is_scripted_trigger_card(kind: &str) -> bool {
    feature_card_type(kind) == Some("scripted_trigger")
}

pub(crate) fn is_state_effect_card(kind: &str) -> bool {
    feature_card_type(kind) == Some("state_effect")
}

pub(crate) fn generate_mod_json(request: &GenerateModRequest<'_>) -> Result<String, String> {
    let synthesized = synthesize_one_sentence_workflow(request.text, request.tag, request.prefix);
    let created = if request.dry_run {
        Vec::new()
    } else {
        scaffold_mod(
            request.mod_root,
            request.name,
            request.version,
            request.supported_version,
            request.tags,
            request.launcher_file,
        )?
    };
    let workflow = run_workflow_json(
        &synthesized,
        (!request.dry_run).then_some(request.mod_root),
        request.tag,
        request.prefix,
        None,
        request.dry_run,
        None,
    )?;
    let created_files = created
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();

    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"schema\": \"hoi4skill.one_sentence_mod.v1\",\n");
    out.push_str(&format!("  \"name\": {},\n", json_str(request.name)));
    out.push_str(&format!("  \"tag\": {},\n", json_str(request.tag)));
    out.push_str(&format!("  \"prefix\": {},\n", json_str(request.prefix)));
    out.push_str(&format!(
        "  \"country_source\": {},\n",
        request
            .country_source
            .map(json_str)
            .unwrap_or_else(|| "null".to_string())
    ));
    out.push_str(&format!(
        "  \"mod_root\": {},\n",
        json_str(&request.mod_root.display().to_string())
    ));
    out.push_str(&format!("  \"dry_run\": {},\n", json_bool(request.dry_run)));
    out.push_str(&format!("  \"source_text\": {},\n", json_str(request.text)));
    out.push_str(&format!(
        "  \"synthesized_input\": {},\n",
        json_str(&synthesized)
    ));
    out.push_str(&format!(
        "  \"scaffold_created\": {},\n",
        json_array(&created_files)
    ));
    out.push_str(&format!("  \"workflow\": {}\n", workflow.trim()));
    out.push_str("}\n");
    Ok(out)
}

#[derive(Clone)]
pub(crate) struct CountryGuess {
    pub(crate) tag: String,
    pub(crate) name: String,
    pub(crate) source: String,
}

pub(crate) fn one_sentence_input_text(map: &ArgMap) -> Result<String, String> {
    if let Some(text) = value(map, "text").or_else(|| value(map, "sentence")) {
        return Ok(text.trim().to_string());
    }
    if let Some(input) = value(map, "input") {
        return read_utf8_lossy(&normalize_path(input)?);
    }
    let text = map.positionals.join(" ");
    if text.trim().is_empty() {
        Err("missing --text, --input, or positional sentence".to_string())
    } else {
        Ok(text.trim().to_string())
    }
}

pub(crate) fn generate_mod_source_roots(map: &ArgMap) -> Result<Vec<PathBuf>, String> {
    let mut roots = Vec::new();

    for raw in repeated_values(map, "game-root") {
        for path in split_path_option(raw) {
            roots.push(normalize_path(path)?);
        }
    }

    for key in ["source-root", "source-mod", "source-mod-root"] {
        for raw in repeated_values(map, key) {
            for path in split_path_option(raw) {
                let path = normalize_path(path)?;
                roots.push(resolve_mod_root(&path)?.root);
            }
        }
    }

    roots.extend(dependency_mod_roots(map)?);
    Ok(dedupe_paths(roots))
}

pub(crate) fn synthesize_one_sentence_workflow(text: &str, tag: &str, prefix: &str) -> String {
    if has_workflow_card_header(text) || !extract_focus_layout_text(text).trim().is_empty() {
        return text.to_string();
    }
    let title = infer_one_sentence_title(text);
    let effects = infer_one_sentence_effects(text);
    let lower = text.to_ascii_lowercase();
    let long_term_effect = one_sentence_requires_idea(text, &effects);
    let idea_title = infer_one_sentence_idea_title(text, &title);
    let wants_decision = text.contains("决议");
    let wants_idea = text.contains("民族精神")
        || lower.contains("buff")
        || lower.contains("debuff")
        || long_term_effect;
    let wants_event = text.contains("事件") || text.contains("新闻");
    let wants_technology = text.contains("科技") || text.contains("技术");
    let wants_gui = lower.contains("gui")
        || text.contains("界面")
        || text.contains("面板")
        || text.contains("按钮");
    let wants_focus = text.contains("国策")
        || (!wants_decision && !wants_idea && !wants_event && !wants_technology && !wants_gui);

    let mut out = String::new();
    if wants_focus {
        out.push_str("国策树：\n");
        if wants_default_focus_tree_template(text) {
            out.push_str(&default_focus_tree_template(&title));
        } else {
            out.push_str(&title);
            out.push('\n');
        }
        if !effects.trim().is_empty() {
            if long_term_effect {
                out.push_str(&format!("# completion_reward: 添加民族精神 {idea_title}\n"));
            } else {
                out.push_str(&format!("# completion_reward: {effects}\n"));
            }
        }
        out.push('\n');
    }
    if wants_decision
        || (!wants_focus && !wants_idea && !wants_event && !wants_technology && !wants_gui)
    {
        out.push_str(&format!(
            "决议：{}\n目标：{}\n分类：国家事务\n花费：50政治点\n效果：{}\n描述：{}\n\n",
            title,
            tag,
            effects,
            one_sentence_description(text)
        ));
    }
    if wants_idea {
        let removal = if long_term_effect
            && contains_any(text, &["临时", "暂时", "持续到", "结束", "移除"])
        {
            "需要在结束国策/事件/决议中移除"
        } else {
            "不可手动移除"
        };
        out.push_str(&format!(
            "民族精神：{}\n目标：{}\n效果：{}\n移除：{}\n描述：{}\n\n",
            idea_title,
            tag,
            effects,
            removal,
            one_sentence_description(text)
        ));
    }
    if wants_technology {
        out.push_str(&format!(
            "独有科技：{}\n目标：{}\n分类：special_forces\n效果：{}\n描述：{}\n\n",
            title,
            tag,
            effects,
            one_sentence_description(text)
        ));
    }
    if wants_gui {
        out.push_str(&format!(
            "特殊GUI：{}\n目标：{}\n用途：{}\n描述：{}\n\n",
            title,
            tag,
            one_sentence_description(text),
            one_sentence_description(text)
        ));
    }
    if wants_event {
        let event_type = if text.contains("新闻") {
            "新闻事件"
        } else {
            "国家事件"
        };
        out.push_str(&format!(
            "事件：{}\n类型：{}\n目标：{}\n命名空间：{}\n标题：{}\n描述：{}\n选项A：继续\n效果A：{}\n\n",
            infer_one_sentence_event_title(text, &title),
            event_type,
            tag,
            prefix,
            infer_one_sentence_event_title(text, &title),
            one_sentence_description(text),
            effects
        ));
    }
    out
}

pub(crate) fn wants_default_focus_tree_template(text: &str) -> bool {
    contains_any(
        text,
        &[
            "国策树",
            "一套国策",
            "一条国策",
            "一条路线",
            "路线国策",
            "多个国策",
            "系列国策",
            "完整国策",
        ],
    )
}

pub(crate) fn default_focus_tree_template(title: &str) -> String {
    let title = title.trim();
    let opening = if title.is_empty() {
        "确立路线"
    } else {
        title
    };
    format!(
        "{opening} | opening_focus\n整顿行政机关 | reorganize_administration    扩大工业基础 | expand_industry    稳定社会秩序 | stabilize_society\n第一阶段成果 | first_phase_result\n深化制度建设 | deepen_institutions    强化动员体系 | strengthen_mobilisation    巩固地方执行 | consolidate_local_execution\n完成路线收束 | complete_route\n"
    )
}

#[derive(Clone)]
pub(crate) struct CountryTagRecord {
    pub(crate) tag_file: PathBuf,
    pub(crate) country_path: String,
}

pub(crate) struct CountryVerification {
    pub(crate) tag_file: PathBuf,
    pub(crate) country_file: PathBuf,
}

#[derive(Clone)]
pub(crate) struct CountryLocCandidate {
    pub(crate) tag: String,
    pub(crate) name: String,
    pub(crate) key: String,
    pub(crate) file: PathBuf,
    pub(crate) rank: u8,
}

pub(crate) fn infer_country_from_sources(
    text: &str,
    roots: &[PathBuf],
) -> Result<Option<CountryGuess>, String> {
    for root in roots {
        let tag_records = collect_country_tag_records(root)?;
        if tag_records.is_empty() {
            continue;
        }

        let loc_candidates = collect_country_localisation_candidates(root)?;
        let mut best_match: Option<(CountryLocCandidate, CountryVerification)> = None;
        for candidate in &loc_candidates {
            if !country_name_matches_text(text, &candidate.name) {
                continue;
            }
            let Some(record) = tag_records.get(&candidate.tag) else {
                continue;
            };
            let Some(verification) = verify_country_record(root, record) else {
                continue;
            };
            let replace = best_match.as_ref().is_none_or(|(old, _)| {
                candidate.rank < old.rank
                    || (candidate.rank == old.rank
                        && candidate.name.chars().count() > old.name.chars().count())
            });
            if replace {
                best_match = Some((candidate.clone(), verification));
            }
        }

        if let Some((candidate, verification)) = best_match {
            return Ok(Some(CountryGuess {
                tag: candidate.tag.clone(),
                name: candidate.name.clone(),
                source: format_country_localisation_source(root, &candidate, &verification),
            }));
        }

        let preferred_names = preferred_country_names(&loc_candidates);
        for (tag, record) in &tag_records {
            if !contains_ascii_token(text, tag) {
                continue;
            }
            let Some(verification) = verify_country_record(root, record) else {
                continue;
            };
            let name = preferred_names
                .get(tag)
                .cloned()
                .unwrap_or_else(|| tag.clone());
            return Ok(Some(CountryGuess {
                tag: tag.clone(),
                name: name.clone(),
                source: format_country_tag_source(root, tag, &name, &verification),
            }));
        }
    }
    Ok(None)
}

pub(crate) fn collect_country_tag_records(
    root: &Path,
) -> Result<BTreeMap<String, CountryTagRecord>, String> {
    let mut records = BTreeMap::new();
    let tag_root = root.join("common").join("country_tags");
    if !tag_root.exists() {
        return Ok(records);
    }
    for file in collect_files(&tag_root)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        for line in strip_comments(&text).lines() {
            let trimmed = line.trim();
            let Some(tag) = assignment_key(trimmed) else {
                continue;
            };
            if !looks_like_tag(tag) {
                continue;
            }
            let Some(country_path) = assignment_value(trimmed, tag) else {
                continue;
            };
            records.entry(tag.to_string()).or_insert(CountryTagRecord {
                tag_file: file.clone(),
                country_path: country_path.to_string(),
            });
        }
    }
    Ok(records)
}

pub(crate) fn collect_country_localisation_candidates(
    root: &Path,
) -> Result<Vec<CountryLocCandidate>, String> {
    let mut candidates = Vec::new();
    let loc_root = root.join("localisation");
    if !loc_root.exists() {
        return Ok(candidates);
    }
    for file in collect_files(&loc_root)? {
        let ext = file
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(ext.as_str(), "yml" | "yaml") {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        for line in text.lines() {
            let Some((key, value)) = parse_localisation_line(line) else {
                continue;
            };
            let Some((tag, rank)) = country_localisation_key_tag(&key) else {
                continue;
            };
            if !is_meaningful_country_name(&value) {
                continue;
            }
            candidates.push(CountryLocCandidate {
                tag,
                name: value,
                key,
                file: file.clone(),
                rank,
            });
        }
    }
    Ok(candidates)
}

pub(crate) fn country_localisation_key_tag(key: &str) -> Option<(String, u8)> {
    if looks_like_tag(key) {
        return Some((key.to_string(), 0));
    }
    for (suffix, rank) in [("_DEF", 1), ("_ADJ", 2)] {
        if let Some(tag) = key.strip_suffix(suffix) {
            if looks_like_tag(tag) {
                return Some((tag.to_string(), rank));
            }
        }
    }
    None
}

pub(crate) fn verify_country_record(
    root: &Path,
    record: &CountryTagRecord,
) -> Option<CountryVerification> {
    resolve_country_file(root, &record.country_path).map(|country_file| CountryVerification {
        tag_file: record.tag_file.clone(),
        country_file,
    })
}

pub(crate) fn resolve_country_file(root: &Path, country_path: &str) -> Option<PathBuf> {
    let cleaned = country_path.trim().replace('/', "\\");
    let rel_text = cleaned.trim_start_matches(['\\', '/']);
    if rel_text.is_empty() {
        return None;
    }
    let rel = PathBuf::from(rel_text);
    let mut candidates = Vec::new();
    if rel.is_absolute() {
        candidates.push(rel.clone());
    } else {
        candidates.push(root.join(&rel));
        candidates.push(root.join("common").join(&rel));
        if let Some(file_name) = rel.file_name() {
            candidates.push(root.join("common").join("countries").join(file_name));
        }
    }
    candidates.into_iter().find(|path| path.is_file())
}

pub(crate) fn preferred_country_names(
    candidates: &[CountryLocCandidate],
) -> BTreeMap<String, String> {
    let mut names: BTreeMap<String, (u8, String)> = BTreeMap::new();
    for candidate in candidates {
        names
            .entry(candidate.tag.clone())
            .and_modify(|(rank, name)| {
                if candidate.rank < *rank
                    || (candidate.rank == *rank
                        && candidate.name.chars().count() > name.chars().count())
                {
                    *rank = candidate.rank;
                    *name = candidate.name.clone();
                }
            })
            .or_insert((candidate.rank, candidate.name.clone()));
    }
    names
        .into_iter()
        .map(|(tag, (_, name))| (tag, name))
        .collect()
}

pub(crate) fn country_name_matches_text(text: &str, name: &str) -> bool {
    let text = searchable_country_text(text);
    let name = searchable_country_text(name);
    is_meaningful_country_name_norm(&name) && text.contains(&name)
}

pub(crate) fn is_meaningful_country_name(value: &str) -> bool {
    is_meaningful_country_name_norm(&searchable_country_text(value))
}

pub(crate) fn is_meaningful_country_name_norm(value: &str) -> bool {
    let count = value.chars().count();
    if value.is_ascii() {
        count >= 3
    } else {
        count >= 2
    }
}

pub(crate) fn searchable_country_text(value: &str) -> String {
    let mut out = String::new();
    let mut skip_section_code = false;
    let mut in_variable = false;
    for ch in value.chars() {
        if skip_section_code {
            skip_section_code = false;
            continue;
        }
        if ch == '§' {
            skip_section_code = true;
            continue;
        }
        if ch == '$' {
            in_variable = !in_variable;
            continue;
        }
        if in_variable {
            continue;
        }
        if ch.is_whitespace()
            || matches!(
                ch,
                '，' | '。'
                    | '、'
                    | '：'
                    | ':'
                    | ';'
                    | '；'
                    | ','
                    | '.'
                    | '!'
                    | '！'
                    | '?'
                    | '？'
                    | '"'
                    | '\''
                    | '“'
                    | '”'
                    | '‘'
                    | '’'
                    | '「'
                    | '」'
                    | '『'
                    | '』'
                    | '('
                    | ')'
                    | '（'
                    | '）'
                    | '['
                    | ']'
                    | '【'
                    | '】'
            )
        {
            continue;
        }
        for lower in ch.to_lowercase() {
            out.push(lower);
        }
    }
    out
}

pub(crate) fn contains_ascii_token(text: &str, token: &str) -> bool {
    let mut rest = text;
    while let Some(idx) = rest.find(token) {
        let before_ok = idx == 0
            || rest[..idx]
                .chars()
                .last()
                .is_none_or(|ch| !(ch.is_ascii_alphanumeric() || ch == '_'));
        let after = &rest[idx + token.len()..];
        let after_ok = after
            .chars()
            .next()
            .is_none_or(|ch| !(ch.is_ascii_alphanumeric() || ch == '_'));
        if before_ok && after_ok {
            return true;
        }
        rest = after;
    }
    false
}

pub(crate) fn format_country_localisation_source(
    root: &Path,
    candidate: &CountryLocCandidate,
    verification: &CountryVerification,
) -> String {
    format!(
        "{}:{} '{}' -> {} via {} -> {}",
        relative_slash_path(root, &candidate.file),
        candidate.key,
        candidate.name,
        candidate.tag,
        relative_slash_path(root, &verification.tag_file),
        relative_slash_path(root, &verification.country_file)
    )
}

pub(crate) fn format_country_tag_source(
    root: &Path,
    tag: &str,
    name: &str,
    verification: &CountryVerification,
) -> String {
    format!(
        "text tag {} '{}' verified by {} -> {}",
        tag,
        name,
        relative_slash_path(root, &verification.tag_file),
        relative_slash_path(root, &verification.country_file)
    )
}

pub(crate) fn target_localisation_file_name(tag: &str) -> String {
    let tag = sanitize_identifier_part(tag, "TAG").to_ascii_uppercase();
    format!("{tag}_l_simp_chinese.yml")
}

pub(crate) fn target_localisation_relative_path(tag: &str) -> String {
    format!(
        "localisation/simp_chinese/{}",
        target_localisation_file_name(tag)
    )
}

pub(crate) fn target_localisation_path(root: &Path, tag: &str) -> PathBuf {
    root.join("localisation")
        .join("simp_chinese")
        .join(target_localisation_file_name(tag))
}

pub(crate) fn infer_country_from_text(text: &str) -> Option<CountryGuess> {
    for (name, tag, aliases) in country_guess_table().iter().copied() {
        if text.contains(tag)
            || text.contains(name)
            || aliases.iter().any(|alias| text.contains(alias))
        {
            return Some(CountryGuess {
                tag: tag.to_string(),
                name: name.to_string(),
                source: format!("built-in country table: {name} -> {tag}"),
            });
        }
    }
    None
}

pub(crate) fn country_guess_table(
) -> &'static [(&'static str, &'static str, &'static [&'static str])] {
    &[
        ("远东铁路共和国", "FER", &["远东铁路", "远东"]),
        ("德国", "GER", &["德意志"]),
        ("苏联", "SOV", &["苏维埃", "俄罗斯"]),
        ("意大利", "ITA", &["意呆利"]),
        ("日本", "JAP", &[]),
        ("美国", "USA", &["美利坚"]),
        ("英国", "ENG", &["不列颠", "英格兰"]),
        ("法国", "FRA", &[]),
        ("中国", "CHI", &["中华民国", "国民政府"]),
        ("中共", "PRC", &["共产党中国", "红色中国"]),
        ("波兰", "POL", &[]),
        ("奥地利", "AUS", &[]),
        ("西班牙", "SPR", &[]),
        ("匈牙利", "HUN", &[]),
        ("罗马尼亚", "ROM", &[]),
    ]
}

pub(crate) fn one_sentence_requires_idea(text: &str, effects: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let combined = format!("{text} {effects}");
    lower.contains("buff")
        || lower.contains("debuff")
        || contains_any(
            &combined,
            &[
                "长期",
                "长期有效",
                "持续",
                "永久",
                "常驻",
                "修正",
                "消费品",
                "建造速度",
                "建设速度",
                "工厂产出",
                "生产效率",
                "生产速度",
                "征兵人口",
                "适役人口",
                "科研速度",
                "研究速度",
                "每日",
                "每周",
            ],
        )
}

pub(crate) fn infer_one_sentence_idea_title(text: &str, focus_title: &str) -> String {
    if text.contains("新经济政策") || text.contains("奈普") {
        "新经济政策复兴".to_string()
    } else if text.contains("工业")
        || text.contains("建造速度")
        || text.contains("建设速度")
        || text.contains("工厂")
    {
        "工业动员体系".to_string()
    } else if text.contains("消费品") || text.contains("市场") || text.contains("经济") {
        "经济管制调整".to_string()
    } else if text.contains("军") || text.contains("动员") || text.contains("战争") {
        "国家动员体制".to_string()
    } else {
        format!("{focus_title}的长期影响")
    }
}

pub(crate) fn infer_one_sentence_title(text: &str) -> String {
    if let Some(quoted) = first_quoted_phrase(text) {
        return quoted;
    }
    if text.contains("海军") || text.contains("舰队") {
        "整训舰队".to_string()
    } else if text.contains("军工") || text.contains("军用工厂") {
        "扩建军工体系".to_string()
    } else if text.contains("民用工厂") || text.contains("工业") || text.contains("工厂") {
        "推动工业建设".to_string()
    } else if text.contains("铁路") {
        "重整铁路网络".to_string()
    } else if text.contains("新经济政策") || text.contains("奈普") {
        "延续新经济政策".to_string()
    } else if text.contains("改革") {
        "推进国家改革".to_string()
    } else if text.contains("战争") || text.contains("动员") {
        "动员国家力量".to_string()
    } else {
        "新的国家方针".to_string()
    }
}

pub(crate) fn first_quoted_phrase(text: &str) -> Option<String> {
    for (left, right) in [('“', '”'), ('「', '」'), ('『', '』'), ('"', '"')] {
        let Some(start) = text.find(left) else {
            continue;
        };
        let rest = &text[start + left.len_utf8()..];
        let Some(end) = rest.find(right) else {
            continue;
        };
        let value = rest[..end].trim();
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

pub(crate) fn infer_one_sentence_effects(text: &str) -> String {
    let mut effects = split_cn_list(text)
        .into_iter()
        .filter(|part| {
            part.contains("政治点")
                || part.contains("政治力量")
                || part.contains("稳定")
                || part.contains("战争支持")
                || part.contains("战争支援")
                || part.contains("海军经验")
                || part.contains("陆军经验")
                || part.contains("空军经验")
                || part.contains("军工")
                || part.contains("军用工厂")
                || part.contains("民工")
                || part.contains("民用工厂")
                || part.contains("消费品")
                || part.contains("建造速度")
                || part.contains("建设速度")
                || part.contains("设置旗标")
        })
        .map(clean_one_sentence_effect)
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>();
    if effects.is_empty() {
        effects.push("政治点+50".to_string());
    }
    effects.join("，")
}

pub(crate) fn clean_one_sentence_effect(text: &str) -> String {
    let mut out = flatten_ws(text);
    for prefix in [
        "完成后",
        "并",
        "同时",
        "效果是",
        "效果为",
        "获得",
        "得到",
        "增加",
    ] {
        out = out.trim_start_matches(prefix).trim().to_string();
    }
    out
}

pub(crate) fn one_sentence_description(text: &str) -> String {
    let text = text.trim().trim_end_matches(['。', '.', '！', '!']);
    if text.is_empty() {
        "这项措施将为国家开辟新的政治方向。".to_string()
    } else {
        format!("{text}。")
    }
}

pub(crate) fn infer_one_sentence_event_title(text: &str, fallback: &str) -> String {
    if text.contains("新闻") {
        format!("{fallback}的消息")
    } else {
        fallback.to_string()
    }
}

pub(crate) fn infer_one_sentence_prefix(tag: &str, title: &str) -> String {
    let fragment = english_focus_fragment(title).unwrap_or_else(|| "feature".to_string());
    format!("{}_{}", tag.to_ascii_lowercase(), fragment)
}

pub(crate) fn infer_one_sentence_mod_name(country: Option<&CountryGuess>, title: &str) -> String {
    if let Some(country) = country {
        format!("{}：{}", country.name, title)
    } else {
        format!("HOI4 一句话 MOD：{title}")
    }
}

pub(crate) fn run_workflow_json(
    text: &str,
    mod_root: Option<&Path>,
    tag: &str,
    prefix: &str,
    tree_id: Option<&str>,
    dry_run: bool,
    game_index: Option<&GameIndex>,
) -> Result<String, String> {
    let feature_text = extract_card_text(text, FEATURE_CARD_HEADERS);
    let event_text = extract_card_text(text, &["事件"]);
    let focus_text = extract_focus_layout_text(text);
    let feature_cards = parse_cards(&feature_text, FEATURE_CARD_HEADERS);
    let event_cards = parse_cards(&event_text, &["事件"]);
    let has_focus_layout = !focus_text.trim().is_empty();
    let focus_plan = has_focus_layout.then(|| {
        let mut layout = parse_focus_layout_with_rewards(&focus_text, tag, prefix);
        if let Some(tree_id) = tree_id {
            layout.tree_id = tree_id.to_string();
        }
        focus_layout_json(&layout, tag, prefix)
    });
    let feature_plan = (!feature_cards.is_empty())
        .then(|| parse_decision_idea_cards_json(&feature_text, tag, prefix));
    let event_plan =
        (!event_cards.is_empty()).then(|| parse_event_cards_json(&event_text, tag, prefix));

    let mut changed = Vec::new();
    if let Some(root) = mod_root {
        if !dry_run {
            if has_focus_layout {
                let mut layout = parse_focus_layout_with_rewards(&focus_text, tag, prefix);
                if let Some(tree_id) = tree_id {
                    layout.tree_id = tree_id.to_string();
                }
                changed.extend(apply_focus_layout_to_mod_with_index(
                    root, &layout, tag, prefix, game_index,
                )?);
            }
            if !feature_cards.is_empty() {
                changed.extend(apply_feature_cards_to_mod(
                    root,
                    &feature_cards,
                    tag,
                    prefix,
                )?);
            }
            if !event_cards.is_empty() {
                changed.extend(apply_event_cards_to_mod(root, &event_cards, tag, prefix)?);
            }
        }
    }
    changed.sort();
    changed.dedup();
    let changed_files = changed
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();

    let validation = if let Some(root) = mod_root {
        Some(validate_mod(root, game_index)?)
    } else {
        None
    };
    let next_steps = workflow_next_steps(
        mod_root.is_some(),
        dry_run,
        validation.as_ref(),
        feature_cards.len() + event_cards.len() + usize::from(has_focus_layout),
    );

    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"schema\": \"hoi4skill.copy_to_code_workflow.v1\",\n");
    out.push_str(&format!("  \"tag\": {},\n", json_str(tag)));
    out.push_str(&format!("  \"prefix\": {},\n", json_str(prefix)));
    out.push_str(&format!("  \"dry_run\": {},\n", json_bool(dry_run)));
    out.push_str(&format!(
        "  \"mod_root\": {},\n",
        json_optional_str(mod_root.map(|path| path.to_string_lossy()).as_deref())
    ));
    out.push_str(&format!(
        "  \"detected\": {{\"focus_layout\": {}, \"feature_cards\": {}, \"event_cards\": {}}},\n",
        json_bool(has_focus_layout),
        feature_cards.len(),
        event_cards.len()
    ));
    out.push_str(&format!(
        "  \"plans\": {{\"focus_layout\": {}, \"feature_cards\": {}, \"event_cards\": {}}},\n",
        json_optional_raw(focus_plan.as_deref()),
        json_optional_raw(feature_plan.as_deref()),
        json_optional_raw(event_plan.as_deref())
    ));
    out.push_str(&format!(
        "  \"changed_files\": {},\n",
        json_array(&changed_files)
    ));
    out.push_str(&format!(
        "  \"validation\": {},\n",
        workflow_validation_json(validation.as_ref())
    ));
    out.push_str(&format!("  \"next_steps\": {}\n", json_array(&next_steps)));
    out.push_str("}\n");
    Ok(out)
}

pub(crate) fn focus_layout_json(layout: &FocusLayout, tag: &str, prefix: &str) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"tag\": {},\n", json_str(tag)));
    out.push_str(&format!("  \"prefix\": {},\n", json_str(prefix)));
    out.push_str(&format!("  \"tree_id\": {},\n", json_str(&layout.tree_id)));
    out.push_str("  \"rows\": [\n");
    for (i, row) in layout.rows.iter().enumerate() {
        comma(&mut out, i, "    ");
        out.push_str(&format!(
            "{{\"y\": {}, \"tokens\": {}, \"focuses\": {}}}",
            row.y,
            json_array(&row.tokens),
            json_array(&row.focus_ids)
        ));
    }
    out.push_str("\n  ],\n  \"focuses\": [\n");
    for (i, f) in layout.focuses.iter().enumerate() {
        comma(&mut out, i, "    ");
        out.push_str(&format!(
            "{{\"title\": {}, \"id\": {}, \"icon\": {}, \"x\": {}, \"y\": {}, \"relative_position_id\": {}, \"row\": {}, \"column\": {}, \"prerequisite\": {}, \"mutually_exclusive\": {}, \"completion_reward\": {}}}",
            json_str(&f.title),
            json_str(&f.id),
            json_optional_str(f.icon.as_deref()),
            f.x,
            f.y,
            json_optional_str(f.relative_position_id.as_deref()),
            f.row,
            f.column,
            json_array(&f.prerequisite),
            json_array(&f.mutually_exclusive),
            json_array(&f.completion_reward)
        ));
    }
    out.push_str("\n  ],\n  \"mutually_exclusive\": [\n");
    for (i, (left, right, row)) in layout.mutuals.iter().enumerate() {
        comma(&mut out, i, "    ");
        out.push_str(&format!(
            "{{\"left\": {}, \"right\": {}, \"row\": {}}}",
            json_str(left),
            json_str(right),
            row
        ));
    }
    out.push_str("\n  ]\n}\n");
    out
}

pub(crate) fn extract_card_text(text: &str, allowed: &[&str]) -> String {
    let mut out = String::new();
    let mut active = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some((key, _)) = split_field(trimmed) {
            if allowed.contains(&key) {
                active = true;
            } else if is_workflow_card_header(key) {
                active = false;
            }
        }
        if active {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

pub(crate) fn extract_focus_layout_text(text: &str) -> String {
    let mut out = String::new();
    let mut active = false;
    let mut saw_marker = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some((key, value)) = split_field(trimmed) {
            if matches!(key, "国策树" | "国策布局" | "国策路线" | "国策草图") {
                active = true;
                saw_marker = true;
                if !value.trim().is_empty() {
                    out.push_str(value.trim());
                    out.push('\n');
                }
                continue;
            }
            if active && is_workflow_card_header(key) {
                active = false;
            }
        }
        if active {
            out.push_str(line);
            out.push('\n');
        }
    }
    if saw_marker {
        return out;
    }
    if !has_workflow_card_header(text) && looks_like_focus_layout(text) {
        text.to_string()
    } else {
        String::new()
    }
}

pub(crate) fn is_workflow_card_header(key: &str) -> bool {
    is_feature_card_header(key) || key == "事件"
}

pub(crate) fn has_workflow_card_header(text: &str) -> bool {
    text.lines().any(|line| {
        split_field(line.trim())
            .map(|(key, _)| is_workflow_card_header(key))
            .unwrap_or(false)
    })
}

pub(crate) fn looks_like_focus_layout(text: &str) -> bool {
    let mut non_empty = 0usize;
    let mut structured = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if split_field(trimmed).is_some() {
            return false;
        }
        non_empty += 1;
        let tokens = split_focus_line(trimmed);
        if tokens.len() > 1 || tokens.iter().any(|token| is_mutual_token(token)) {
            structured = true;
        }
    }
    non_empty >= 2 && structured
}

pub(crate) fn json_optional_raw(value: Option<&str>) -> String {
    value
        .map(|raw| raw.trim().to_string())
        .unwrap_or_else(|| "null".to_string())
}

pub(crate) fn workflow_validation_json(reporter: Option<&Reporter>) -> String {
    if let Some(reporter) = reporter {
        let ok = reporter.errors.is_empty() && reporter.warnings.is_empty();
        let status = if reporter.errors.is_empty() {
            if reporter.warnings.is_empty() {
                json_str("ok")
            } else {
                json_str("warnings")
            }
        } else {
            json_str("errors")
        };
        format!(
            "{{\"ran\": true, \"ok\": {}, \"status\": {}, \"errors\": {}, \"warnings\": {}}}",
            json_bool(ok),
            status,
            json_array(&reporter.errors),
            json_array(&reporter.warnings)
        )
    } else {
        "{\"ran\": false, \"ok\": null, \"status\": null, \"errors\": [], \"warnings\": []}"
            .to_string()
    }
}

pub(crate) fn workflow_next_steps(
    has_mod_root: bool,
    dry_run: bool,
    reporter: Option<&Reporter>,
    detected_sections: usize,
) -> Vec<String> {
    let mut steps = Vec::new();
    if detected_sections == 0 {
        steps.push(
            "未识别到国策树、决议/民族精神卡片或事件卡片；请把文案改成带中文字段的 Feature Plan。"
                .to_string(),
        );
    }
    if !has_mod_root {
        steps.push("确认计划后带上 --mod-root 重新运行以写入 MOD 文件。".to_string());
    } else if dry_run {
        steps.push("去掉 --dry-run 后重新运行以写入 MOD 文件。".to_string());
    }
    if let Some(reporter) = reporter {
        if !reporter.errors.is_empty() {
            steps.push("先修复 validation.errors 中的静态错误，再进游戏测试。".to_string());
        } else if !reporter.warnings.is_empty() {
            steps.push("先逐条核对 validation.warnings，不要把它当成通过；确认可接受后再用 -debug 启动游戏测试。".to_string());
        } else if has_mod_root && !dry_run {
            steps.push("静态校验通过；用 -debug 启动 HOI4 并检查 error.log。".to_string());
        }
    }
    if steps.is_empty() {
        steps.push("计划已生成，可以继续补效果、图标和本地化细节。".to_string());
    }
    steps
}

pub(crate) fn cmd_country_localisation_template(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let tag = require_value(&map, "tag")?;
    let name = require_value(&map, "name")?;
    let default_prefix = tag.to_ascii_lowercase();
    let prefix = value(&map, "prefix").unwrap_or(&default_prefix);
    let template = country_localisation_template(&map, &tag, &name, prefix);
    write_or_print(&template, value(&map, "output"))
}

pub(crate) fn country_localisation_template(
    map: &ArgMap,
    tag: &str,
    name: &str,
    prefix: &str,
) -> String {
    let tag = sanitize_identifier_part(tag, "TAG").to_ascii_uppercase();
    let prefix = sanitize_identifier_part(prefix, "mod");
    let def_name = value(map, "def").unwrap_or(name);
    let adjective = value(map, "adj").unwrap_or(name);
    let cosmetic_id = value(map, "cosmetic-id")
        .map(|value| sanitize_identifier_part(value, "cosmetic"))
        .unwrap_or_else(|| format!("{}_{}_cosmetic", tag, prefix));
    let cosmetic_name = value(map, "cosmetic-name").unwrap_or(name);
    let cosmetic_def = value(map, "cosmetic-def").unwrap_or(cosmetic_name);
    let cosmetic_adj = value(map, "cosmetic-adj").unwrap_or(adjective);

    let focuses = localisation_item_specs(map, &["focus", "focus-id"]);
    let ideas = localisation_item_specs(map, &["idea", "idea-id"])
        .into_iter()
        .map(|(id, title)| (ensure_idea_localisation_key_suffix(&id), title))
        .collect::<Vec<_>>();
    let decisions = localisation_item_specs(map, &["decision", "decision-id"]);
    let events = localisation_item_specs(map, &["event", "event-id"]);
    let technologies = localisation_item_specs(map, &["tech", "tech-id", "technology"]);
    let gui = localisation_item_specs(map, &["gui", "gui-key"]);

    let mut out = String::new();
    out.push('\u{feff}');
    out.push_str("l_simp_chinese:\n");
    out.push_str("  # ===== 国家 tag / 国家名 =====\n");
    out.push_str(&format!("  {tag}:0 \"{}\"\n", localisation_value(name)));
    out.push_str(&format!(
        "  {tag}_DEF:0 \"{}\"\n",
        localisation_value(def_name)
    ));
    out.push_str(&format!(
        "  {tag}_ADJ:0 \"{}\"\n",
        localisation_value(adjective)
    ));
    out.push_str("\n  # ===== 国家 cosmetic 名 =====\n");
    out.push_str(&format!(
        "  {cosmetic_id}:0 \"{}\"\n",
        localisation_value(cosmetic_name)
    ));
    out.push_str(&format!(
        "  {cosmetic_id}_DEF:0 \"{}\"\n",
        localisation_value(cosmetic_def)
    ));
    out.push_str(&format!(
        "  {cosmetic_id}_ADJ:0 \"{}\"\n",
        localisation_value(cosmetic_adj)
    ));

    out.push_str("\n  # ===== 国策树 =====\n");
    out.push_str(&format!(
        "  {prefix}_{tag}_focus_tree:0 \"{}国策树\"\n",
        localisation_value(name)
    ));
    push_localisation_title_desc_section(&mut out, &focuses, "focus");

    out.push_str("\n  # ===== 民族精神 =====\n");
    push_localisation_title_desc_section(&mut out, &ideas, "idea");

    out.push_str("\n  # ===== 决议 =====\n");
    push_localisation_title_desc_section(&mut out, &decisions, "decision");

    out.push_str("\n  # ===== 事件 =====\n");
    if events.is_empty() {
        out.push_str("  # TODO: event id example: namespace.1.t / namespace.1.d / namespace.1.a\n");
    } else {
        for (id, title) in events {
            out.push_str(&format!("  {id}.t:0 \"{}\"\n", localisation_value(&title)));
            out.push_str(&format!(
                "  {id}.d:0 \"{}\"\n",
                localisation_value(&format!("{title}。"))
            ));
            out.push_str(&format!("  {id}.a:0 \"好的。\"\n"));
        }
    }

    out.push_str("\n  # ===== 独有特殊科技 =====\n");
    push_localisation_title_desc_section(&mut out, &technologies, "technology");

    out.push_str("\n  # ===== 特殊 GUI =====\n");
    if gui.is_empty() {
        out.push_str("  # TODO: GUI_KEY:0 \"界面文本\"\n");
    } else {
        for (key, title) in gui {
            out.push_str(&format!("  {key}:0 \"{}\"\n", localisation_value(&title)));
        }
    }
    out
}

pub(crate) fn localisation_item_specs(map: &ArgMap, keys: &[&str]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for key in keys {
        for raw in repeated_values(map, key) {
            out.push(localisation_item_spec(raw));
        }
    }
    out
}

pub(crate) fn localisation_item_spec(raw: &str) -> (String, String) {
    if let Some((id, title)) = raw.split_once('=') {
        (
            sanitize_localisation_key(id.trim(), "key"),
            title.trim().to_string(),
        )
    } else {
        let id = sanitize_localisation_key(raw.trim(), "key");
        (id, "TODO".to_string())
    }
}

pub(crate) fn sanitize_localisation_key(value: &str, fallback: &str) -> String {
    let mut out = String::new();
    let mut last_us = false;
    for ch in value.trim().chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | ':') {
            out.push(ch);
            last_us = false;
        } else if !out.is_empty() && !last_us {
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

pub(crate) fn push_localisation_title_desc_section(
    out: &mut String,
    items: &[(String, String)],
    kind: &str,
) {
    if items.is_empty() {
        out.push_str(&format!("  # TODO: add {kind} localisation keys here\n"));
        return;
    }
    for (key, title) in items {
        out.push_str(&format!(
            "  {}:0 \"{}\"\n",
            sanitize_localisation_key(key, kind),
            title.replace('"', "\\\"")
        ));
        out.push_str(&format!(
            "  {}_desc:0 \"{}描述\"\n",
            sanitize_localisation_key(key, kind),
            title.replace('"', "\\\"")
        ));
    }
}
