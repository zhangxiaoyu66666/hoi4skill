//! One-sentence and mixed-card workflows that orchestrate generation and validation.

#[allow(unused_imports)]
use crate::*;

pub(crate) struct WorkflowInput {
    pub(crate) text: String,
    pub(crate) focus_layout: Option<FocusLayout>,
}

#[derive(Clone)]
pub(crate) struct RequirementScopeContract {
    pub(crate) authorized_systems: Vec<String>,
    pub(crate) minimum_events: Option<usize>,
    pub(crate) minimum_ideas: Option<usize>,
    pub(crate) planned_files: Vec<String>,
    pub(crate) forbidden_without_explicit_request: Vec<String>,
    pub(crate) rules: Vec<String>,
}

pub(crate) fn cmd_resolve_country_tag(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = one_sentence_input_text(&map)?;
    let game_root = normalize_path(&require_value(&map, "game-root")?)?;
    let dependency_mods = dependency_mod_roots(&map)?;
    let game_index = build_country_tag_index_with_mod_paths(&game_root, &dependency_mods)?;
    let inferred = infer_country_from_sources(&text, &generate_mod_source_roots(&map)?)?
        .or_else(|| infer_country_from_text(&text));
    let resolution = resolve_country_tag(
        &text,
        value(&map, "tag"),
        inferred,
        Some(&game_index),
        map.flags.contains("allow-new-tag"),
    )?;
    write_or_print(
        &country_tag_resolution_json(&text, &resolution),
        value(&map, "output"),
    )
}

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
    let validation_options = validation_options_from_args(&map);
    let mut workflow_input = workflow_input_from_path(&input, sheet, tag, prefix)?;
    append_explicit_request(&mut workflow_input, value(&map, "request"));
    enforce_tag_request_contract(&map, tag, game_index.as_ref())?;
    let json = run_workflow_json_with_focus_layout_options(
        &workflow_input.text,
        workflow_input.focus_layout.as_ref(),
        mod_root.as_deref(),
        tag,
        prefix,
        tree_id,
        dry_run,
        game_index.as_ref(),
        validation_options,
    )?;
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn append_explicit_request(input: &mut WorkflowInput, request: Option<&str>) {
    let Some(request) = request.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    input.text.push_str(
        "\n\n# Explicit User Requirement Contract\n\
# The following text is the user's literal scope. It may authorize additional systems, but it does not authorize unrelated systems.\n# ",
    );
    input
        .text
        .push_str(&request.lines().collect::<Vec<_>>().join("\n# "));
    input.text.push('\n');
}

pub(crate) fn workflow_input_from_path(
    input: &Path,
    sheet: Option<&str>,
    tag: &str,
    prefix: &str,
) -> Result<WorkflowInput, String> {
    let extension = input
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    if matches!(extension.as_str(), "xlsx" | "xls" | "xlsm" | "xlsb" | "ods") {
        let markdown = render_focus_excel_markdown(input, sheet, tag, prefix)?;
        let text = format!(
            "{markdown}\n\n## Immutable Excel Import Contract\n\n\
- Every imported focus title is a literal value. Do not rename, paraphrase, split, merge, add, or remove focuses.\n\
- Preserve worksheet rows, columns, blank-column spacing, and explicit mutual-exclusion markers.\n\
- Do not reconstruct coordinates from prose. Apply the structured focus layout supplied by the workflow.\n\
- All non-opening focuses are anchored to the opening focus with relative offsets. Never combine parent-relative anchors with absolute worksheet x/y coordinates.\n"
        );
        let focus_layout = read_focus_excel_layout(input, sheet, tag, prefix)?;
        return Ok(WorkflowInput {
            text,
            focus_layout: Some(focus_layout),
        });
    }
    let text = read_text_document(input)?;
    Ok(WorkflowInput {
        text: normalise_single_work_package_request_text(&text, None),
        focus_layout: None,
    })
}

pub(crate) fn workflow_input_from_text(text: &str) -> WorkflowInput {
    WorkflowInput {
        text: normalise_single_work_package_request_text(text, None),
        focus_layout: None,
    }
}

pub(crate) fn workflow_dynamic_modifier_intents(text: &str) -> Vec<String> {
    let labels = [
        "动态修正",
        "dynamic_modifier",
        "dynamic modifier",
        "dynamic modifiers",
    ];
    let mut out = Vec::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some((key, value)) = split_field(trimmed) {
            let normalised = normalise_workflow_lane_label(key);
            if labels
                .iter()
                .any(|label| normalised == normalise_workflow_lane_label(label))
            {
                if let Some(value) = current.take() {
                    out.push(value);
                }
                let value = value.trim();
                if !value.is_empty() {
                    current = Some(value.to_string());
                }
                continue;
            }
            if workflow_line_starts_new_lane(&normalised) {
                if let Some(value) = current.take() {
                    out.push(value);
                }
                continue;
            }
        }
        if let Some(current_value) = current.as_mut() {
            if workflow_dynamic_modifier_continuation(trimmed) {
                current_value.push_str(" | ");
                current_value.push_str(trimmed);
            }
        }
    }
    if let Some(value) = current {
        out.push(value);
    }
    out.sort();
    out.dedup();
    out
}

pub(crate) fn workflow_localisation_entries(text: &str) -> Vec<String> {
    workflow_labeled_entries(text, &["本地化", "localisation", "localization", "loc"])
}

pub(crate) fn workflow_localisation_token_issues(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for entry in workflow_localisation_entries(text) {
        let Some(value) = workflow_localisation_visible_value(&entry) else {
            continue;
        };
        let compiled = compile_author_localisation_placeholders_without_index(value);
        let (_, issues) = extract_localisation_tokens(&compiled);
        for issue in issues {
            out.push(format!(
                "{}: {}: {}",
                workflow_localisation_entry_label(&entry),
                issue.kind,
                issue.message
            ));
        }
    }
    out.sort();
    out.dedup();
    out
}

fn workflow_localisation_entry_label(entry: &str) -> String {
    if let Some((key, _)) = entry.split_once('=') {
        return key.trim().trim_matches('"').trim().to_string();
    }
    if let Some((key, _)) = split_field(entry) {
        return key.trim().trim_matches('"').trim().to_string();
    }
    truncate_chars(entry.trim(), 40)
}

fn workflow_labeled_entries(text: &str, labels: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let Some((key, value)) = split_field(trimmed) else {
            continue;
        };
        let normalised = normalise_workflow_lane_label(key);
        if labels
            .iter()
            .any(|label| normalised == normalise_workflow_lane_label(label))
        {
            let value = value.trim();
            if !value.is_empty() {
                out.push(value.to_string());
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn normalise_workflow_lane_label(value: &str) -> String {
    value
        .trim()
        .trim_matches('`')
        .trim()
        .chars()
        .filter(|ch| !matches!(ch, ' ' | '\t' | '_' | '-' | ':' | '：'))
        .collect::<String>()
        .to_ascii_lowercase()
}

fn workflow_line_starts_new_lane(normalised_label: &str) -> bool {
    [
        "国策",
        "focus",
        "事件",
        "event",
        "决议",
        "decision",
        "民族精神",
        "nationalspirit",
        "idea",
        "本地化",
        "localisation",
        "localization",
        "loc",
    ]
    .iter()
    .any(|label| normalised_label == normalise_workflow_lane_label(label))
}

fn workflow_dynamic_modifier_continuation(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.starts_with("custom_effect_tooltip")
        || lower.starts_with("set_temp_variable")
        || lower.starts_with("change_")
        || lower.starts_with("描述：")
        || lower.starts_with("描述:")
        || lower.starts_with("effect:")
        || lower.starts_with("效果：")
        || lower.starts_with("效果:")
}

pub(crate) fn cmd_generate_mod(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = one_sentence_input_text(&map)?;
    let source_roots = generate_mod_source_roots(&map)?;
    let explicit_source_roots = generate_mod_explicit_source_roots(&map)?;
    let country = if let Some(country) = infer_country_from_sources(&text, &explicit_source_roots)?
    {
        Some(country)
    } else if let Some(country) = infer_country_from_sources(&text, &source_roots)? {
        Some(country)
    } else {
        infer_country_from_text(&text)
    };
    let dependency_mods = dependency_mod_roots(&map)?;
    let game_index = value(&map, "game-root")
        .map(normalize_path)
        .transpose()?
        .map(|path| build_game_index_with_mod_paths(&path, &dependency_mods))
        .transpose()?;
    if game_index.is_none() && !dependency_mods.is_empty() {
        return Err(
            "--mod-path requires --game-root during one-sentence mod generation".to_string(),
        );
    }
    let validation_options = validation_options_from_args(&map);
    if validation_options.strict_code_index && game_index.is_none() {
        return Err(
            "strict generate-mod requires --game-root before accepting generated files".to_string(),
        );
    }
    let resolution = resolve_country_tag(
        &text,
        value(&map, "tag"),
        country.clone(),
        game_index.as_ref(),
        map.flags.contains("allow-new-tag"),
    )?;
    let tag = resolution.tag;
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
        country_source: Some(&resolution.source),
        game_index: game_index.as_ref(),
        validation_options,
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
    pub(crate) game_index: Option<&'a GameIndex>,
    pub(crate) validation_options: ValidationOptions,
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
    let workflow = run_workflow_json_with_focus_layout_options(
        &synthesized,
        None,
        (!request.dry_run).then_some(request.mod_root),
        request.tag,
        request.prefix,
        None,
        request.dry_run,
        request.game_index,
        request.validation_options,
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

#[derive(Clone)]
pub(crate) struct CountryTagResolution {
    pub(crate) tag: String,
    pub(crate) name: String,
    pub(crate) source: String,
    pub(crate) exists_in_index: Option<bool>,
    pub(crate) new_tag_authorized: bool,
    pub(crate) decision: &'static str,
}

pub(crate) fn resolve_country_tag(
    request: &str,
    explicit_tag: Option<&str>,
    inferred: Option<CountryGuess>,
    game_index: Option<&GameIndex>,
    allow_new_tag: bool,
) -> Result<CountryTagResolution, String> {
    let explicit_tag = explicit_tag
        .map(|tag| sanitize_identifier_part(tag, "TAG").to_ascii_uppercase())
        .filter(|tag| tag != "TAG");
    let new_tag_authorized = request_explicitly_creates_country_tag(request) && allow_new_tag;

    if let (Some(explicit), Some(guess)) = (explicit_tag.as_ref(), inferred.as_ref()) {
        if explicit != &guess.tag && !new_tag_authorized {
            return Err(format!(
                "country TAG conflict: request resolves to existing {} ({}) from {}, but --tag {} was supplied. Reuse {}. A prefix, faction, committee, government, or revolutionary organisation name is not authorization to create a country TAG",
                guess.tag, guess.name, guess.source, explicit, guess.tag
            ));
        }
    }

    let (tag, name, source) = if new_tag_authorized {
        let tag = explicit_tag.ok_or_else(|| {
            "--allow-new-tag requires an explicit --tag and a literal request to create a new country/TAG"
                .to_string()
        })?;
        let name = inferred
            .as_ref()
            .map(|guess| guess.name.clone())
            .unwrap_or_else(|| tag.clone());
        (
            tag,
            name,
            "explicit user request plus --allow-new-tag".to_string(),
        )
    } else if let Some(guess) = inferred {
        (guess.tag, guess.name, guess.source)
    } else if let Some(tag) = explicit_tag {
        (tag.clone(), tag, "explicit --tag".to_string())
    } else {
        return Err(
            "country TAG is unknown. Build/read the local game or mod knowledge base and resolve an existing TAG; do not invent one from the mod name or prefix"
                .to_string(),
        );
    };

    let exists_in_index = game_index.map(|index| index.country_tags.contains(&tag));
    if exists_in_index.is_none() && source == "explicit --tag" && !new_tag_authorized {
        return Err(format!(
            "country TAG {tag} was supplied without local game/dependency/source-mod evidence. Run resolve-country-tag with --game-root first; a bare --tag is not proof that the country exists"
        ));
    }
    if exists_in_index == Some(false) && !new_tag_authorized {
        return Err(format!(
            "country TAG {tag} is not present in the indexed game/dependency knowledge base. Creating common/country_tags, common/countries, or history/countries is forbidden unless the user literally requests a new country/TAG and --allow-new-tag is supplied"
        ));
    }

    Ok(CountryTagResolution {
        tag,
        name,
        source,
        exists_in_index,
        new_tag_authorized,
        decision: if new_tag_authorized {
            "create_new_tag"
        } else {
            "reuse_existing_tag"
        },
    })
}

pub(crate) fn request_explicitly_creates_country_tag(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    contains_any(
        text,
        &[
            "创建新国家",
            "创建一个新国家",
            "新建国家",
            "新建一个国家",
            "建立新国家",
            "建立一个新国家",
            "创建国家TAG",
            "创建国家tag",
            "新建TAG",
            "新建tag",
            "创建新TAG",
            "创建新tag",
            "建立新TAG",
            "建立新tag",
            "建立一个新TAG",
            "建立一个新tag",
            "自定义TAG",
            "自定义tag",
        ],
    ) || lower.contains("create a new country")
        || lower.contains("create new country")
        || lower.contains("create a new tag")
        || lower.contains("create new tag")
}

pub(crate) fn enforce_tag_request_contract(
    map: &ArgMap,
    tag: &str,
    game_index: Option<&GameIndex>,
) -> Result<(), String> {
    let Some(request) = value(map, "request") else {
        return Ok(());
    };
    let inferred = if let Some(index) = game_index {
        infer_country_from_sources(request, &index.indexed_roots)?
            .or_else(|| infer_country_from_text(request))
    } else {
        infer_country_from_text(request)
    };
    resolve_country_tag(
        request,
        Some(tag),
        inferred,
        game_index,
        map.flags.contains("allow-new-tag"),
    )
    .map(|_| ())
}

pub(crate) fn country_tag_resolution_json(
    request: &str,
    resolution: &CountryTagResolution,
) -> String {
    let indexed = resolution.exists_in_index.map(json_bool).unwrap_or("null");
    format!(
        "{{\n  \"request\": {},\n  \"resolved_tag\": {},\n  \"country_name\": {},\n  \"source\": {},\n  \"exists_in_index\": {},\n  \"new_tag_authorized\": {},\n  \"decision\": {},\n  \"forbidden_files_when_reusing\": [\"common/country_tags/*\", \"common/countries/*\", \"history/countries/*\"]\n}}\n",
        json_str(request),
        json_str(&resolution.tag),
        json_str(&resolution.name),
        json_str(&resolution.source),
        indexed,
        json_bool(resolution.new_tag_authorized),
        json_str(resolution.decision),
    )
}

pub(crate) fn one_sentence_input_text(map: &ArgMap) -> Result<String, String> {
    if let Some(text) = value(map, "text").or_else(|| value(map, "sentence")) {
        return Ok(text.trim().to_string());
    }
    if let Some(input) = value(map, "input") {
        return read_text_document(&normalize_path(input)?);
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

    roots.extend(generate_mod_explicit_source_roots(map)?);
    Ok(dedupe_paths(roots))
}

pub(crate) fn generate_mod_explicit_source_roots(map: &ArgMap) -> Result<Vec<PathBuf>, String> {
    let mut roots = Vec::new();
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
    let wants_event_chain = wants_event && one_sentence_wants_event_chain(text);
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
        let event_title = infer_one_sentence_event_title(text, &title);
        if wants_event_chain {
            let final_effect = if effects.trim().is_empty() {
                "增加政治权力25".to_string()
            } else {
                effects.clone()
            };
            let chain_key_base = sanitize_identifier_part(prefix, "event_chain");
            let opening_key = format!("{chain_key_base}_opening");
            let reaction_key = format!("{chain_key_base}_reaction");
            let resolution_key = format!("{chain_key_base}_resolution");
            out.push_str(&format!(
                "事件：{event_title}：序幕\n事件键：{opening_key}\n类型：{event_type}\n目标：{tag}\n命名空间：{prefix}\n标题：{event_title}：序幕\n描述：{}\n选项A：继续观察\n后续事件A：{reaction_key}\n延迟A：3\n\n",
                one_sentence_description(text)
            ));
            out.push_str(&format!(
                "事件：{event_title}：各方反应\n事件键：{reaction_key}\n类型：{event_type}\n目标：{tag}\n命名空间：{prefix}\n标题：{event_title}：各方反应\n描述：围绕“{}”的消息扩散后，各派力量开始重新估量局势。\n选项A：推动进程\n后续事件A：{resolution_key}\n延迟A：7\n\n",
                one_sentence_description(text)
            ));
            out.push_str(&format!(
                "事件：{event_title}：最终定局\n事件键：{resolution_key}\n类型：{event_type}\n目标：{tag}\n命名空间：{prefix}\n标题：{event_title}：最终定局\n描述：事态进入收束阶段，国家机器开始把临时应对转化为新的政治现实。\n选项A：确认结果\n效果A：{final_effect}\n\n"
            ));
        } else {
            out.push_str(&format!(
                "事件：{}\n类型：{}\n目标：{}\n命名空间：{}\n标题：{}\n描述：{}\n选项A：继续\n效果A：{}\n\n",
                event_title,
                event_type,
                tag,
                prefix,
                event_title,
                one_sentence_description(text),
                effects
            ));
        }
    }
    out
}

pub(crate) fn one_sentence_wants_event_chain(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    contains_any(
        text,
        &[
            "事件链",
            "连续事件",
            "连锁事件",
            "系列事件",
            "多段事件",
            "多事件",
            "一串事件",
            "后续事件",
            "链式事件",
            "三段事件",
        ],
    ) || lower.contains("event chain")
        || lower.contains("chain of events")
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

        if let Some(alias_guess) = infer_country_alias_from_text(text, root, &tag_records) {
            return Ok(Some(alias_guess));
        }

        let loc_candidates = collect_country_localisation_candidates(root)?;
        let mut best_match: Option<(i64, CountryLocCandidate, CountryVerification)> = None;
        for candidate in &loc_candidates {
            let Some(score) = country_target_match_score(text, &candidate.name, candidate.rank)
            else {
                continue;
            };
            let Some(record) = tag_records.get(&candidate.tag) else {
                continue;
            };
            let Some(verification) = verify_country_record(root, record) else {
                continue;
            };
            let replace = best_match.as_ref().is_none_or(|(old_score, old, _)| {
                score > *old_score
                    || (score == *old_score
                        && candidate.name.chars().count() > old.name.chars().count())
            });
            if replace {
                best_match = Some((score, candidate.clone(), verification));
            }
        }

        if let Some((_, candidate, verification)) = best_match {
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

pub(crate) fn infer_country_alias_from_text(
    text: &str,
    root: &Path,
    tag_records: &BTreeMap<String, CountryTagRecord>,
) -> Option<CountryGuess> {
    for (tag, name, aliases) in [(
        "PRC",
        "中国共产党",
        &[
            "中国共产党",
            "中共",
            "共产党中国",
            "Chinese Communist Party",
            "CCP",
        ][..],
    )] {
        let matched = aliases.iter().any(|alias| {
            if alias.is_ascii() {
                contains_ascii_token(text, alias)
            } else {
                text.contains(alias)
            }
        });
        if !matched {
            continue;
        }
        let record = tag_records.get(tag)?;
        let verification = verify_country_record(root, record)?;
        return Some(CountryGuess {
            tag: tag.to_string(),
            name: name.to_string(),
            source: format_country_tag_source(root, tag, name, &verification),
        });
    }
    None
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
        if root.join("hoi4.exe").is_file() && !is_game_country_localisation_file(&file) {
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

pub(crate) fn is_game_country_localisation_file(file: &Path) -> bool {
    let name = file
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    name.contains("countries") || name.contains("country")
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
    let tag = key.split('_').next()?;
    if !looks_like_tag(tag) {
        return None;
    }
    let rank = if key.ends_with("_DEF") {
        4
    } else if key.ends_with("_ADJ") {
        5
    } else {
        3
    };
    Some((tag.to_string(), rank))
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

pub(crate) fn country_target_match_score(text: &str, name: &str, rank: u8) -> Option<i64> {
    let text = searchable_country_text(text);
    let name = searchable_country_text(name);
    if !is_meaningful_country_name_norm(&name) {
        return None;
    }
    let mut best = None;
    for (position, _) in text.match_indices(&name) {
        let before_tail = char_tail(&text[..position], 12);
        let after_head = char_head(&text[position + name.len()..], 12);
        let mut score = 1_000i64 - position.min(800) as i64;
        score += (name.chars().count().min(20) * 8) as i64;
        score -= i64::from(rank) * 20;
        if ends_with_any(
            &before_tail,
            &["给", "为", "替", "让", "依据", "按照", "面向", "针对"],
        ) {
            score += 900;
        }
        if starts_with_any(
            &after_head,
            &[
                "制作",
                "生成",
                "创建",
                "建立",
                "添加",
                "设计",
                "写",
                "的mod",
                "mod",
                "国策",
                "事件",
                "民族精神",
            ],
        ) {
            score += 450;
        }
        if ends_with_any(
            &before_tail,
            &[
                "反抗", "抵抗", "对抗", "抗击", "击败", "进攻", "攻击", "入侵", "摆脱", "防御",
                "防范", "反对", "驱逐",
            ],
        ) {
            score -= 1_500;
        }
        best = Some(best.map_or(score, |old: i64| old.max(score)));
    }
    best
}

pub(crate) fn char_tail(value: &str, count: usize) -> String {
    value
        .chars()
        .rev()
        .take(count)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

pub(crate) fn char_head(value: &str, count: usize) -> String {
    value.chars().take(count).collect()
}

pub(crate) fn ends_with_any(value: &str, suffixes: &[&str]) -> bool {
    suffixes.iter().any(|suffix| value.ends_with(suffix))
}

pub(crate) fn starts_with_any(value: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| value.starts_with(prefix))
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
    text.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .find(|token| looks_like_tag(token))
        .map(|tag| CountryGuess {
            tag: tag.to_string(),
            name: tag.to_string(),
            source: "literal country TAG in request".to_string(),
        })
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

#[allow(dead_code)]
pub(crate) fn run_workflow_json(
    text: &str,
    mod_root: Option<&Path>,
    tag: &str,
    prefix: &str,
    tree_id: Option<&str>,
    dry_run: bool,
    game_index: Option<&GameIndex>,
) -> Result<String, String> {
    run_workflow_json_with_focus_layout_options(
        text,
        None,
        mod_root,
        tag,
        prefix,
        tree_id,
        dry_run,
        game_index,
        ValidationOptions::default(),
    )
}

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
pub(crate) fn run_workflow_json_with_focus_layout(
    text: &str,
    supplied_focus_layout: Option<&FocusLayout>,
    mod_root: Option<&Path>,
    tag: &str,
    prefix: &str,
    tree_id: Option<&str>,
    dry_run: bool,
    game_index: Option<&GameIndex>,
) -> Result<String, String> {
    run_workflow_json_with_focus_layout_options(
        text,
        supplied_focus_layout,
        mod_root,
        tag,
        prefix,
        tree_id,
        dry_run,
        game_index,
        ValidationOptions::default(),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn run_workflow_json_with_focus_layout_options(
    text: &str,
    supplied_focus_layout: Option<&FocusLayout>,
    mod_root: Option<&Path>,
    tag: &str,
    prefix: &str,
    tree_id: Option<&str>,
    dry_run: bool,
    game_index: Option<&GameIndex>,
    validation_options: ValidationOptions,
) -> Result<String, String> {
    let feature_text = extract_card_text(text, FEATURE_CARD_HEADERS);
    let event_text = extract_card_text(text, &["事件"]);
    let focus_text = extract_focus_layout_text(text);
    let feature_cards = parse_cards(&feature_text, FEATURE_CARD_HEADERS);
    let event_cards = parse_cards(&event_text, &["事件"]);
    let dynamic_modifier_intents = workflow_dynamic_modifier_intents(text);
    let localisation_entries = workflow_localisation_entries(text);
    let localisation_token_issues = workflow_localisation_token_issues(text);
    let mut focus_layout = supplied_focus_layout.cloned().or_else(|| {
        (!focus_text.trim().is_empty())
            .then(|| parse_focus_layout_with_rewards(&focus_text, tag, prefix))
    });
    if let Some(layout) = &mut focus_layout {
        if let Some(tree_id) = tree_id {
            layout.tree_id = tree_id.to_string();
        }
        if let Some(root) = mod_root {
            assign_indexed_focus_icons(layout, root, game_index, tag)?;
        } else if let Some(index) = game_index {
            for focus in &mut layout.focuses {
                if focus.icon.is_none() {
                    let semantic_title = focus_icon_semantic_title(tag, &focus.title);
                    focus.icon =
                        choose_focus_icon_from_catalog(&semantic_title, &index.focus_goal_sprites);
                }
            }
        }
    }
    let has_focus_layout = focus_layout.is_some();
    let scope_contract = requirement_scope_contract(text, has_focus_layout, tag, prefix);
    let focus_plan = focus_layout
        .as_ref()
        .map(|layout| focus_layout_json(layout, tag, prefix));
    let feature_plan = (!feature_cards.is_empty())
        .then(|| parse_decision_idea_cards_json(&feature_text, tag, prefix));
    let event_plan =
        (!event_cards.is_empty()).then(|| parse_event_cards_json(&event_text, tag, prefix));
    let mut safety_blockers = workflow_safety_blockers(
        focus_layout.as_ref(),
        &feature_cards,
        &event_cards,
        tag,
        prefix,
    );
    safety_blockers.extend(workflow_strict_gate_blockers(
        validation_options,
        mod_root,
        focus_layout.as_ref(),
        &feature_cards,
        &event_cards,
        tag,
        prefix,
        game_index,
    ));
    safety_blockers.extend(
        localisation_token_issues
            .iter()
            .map(|issue| format!("localisation token preflight: {issue}")),
    );
    let safety = workflow_safety_json_from_blockers(&safety_blockers);

    let mut changed = Vec::new();
    if let Some(root) = mod_root {
        if !dry_run {
            if let Some(layout) = &focus_layout {
                enforce_strict_focus_layout_gate_with_options(
                    validation_options,
                    root,
                    layout,
                    tag,
                    game_index,
                )?;
                changed.extend(apply_focus_layout_to_mod_with_index(
                    root, layout, tag, prefix, game_index,
                )?);
            }
            if !feature_cards.is_empty() {
                enforce_strict_feature_card_gate_with_options(
                    validation_options,
                    &feature_cards,
                    tag,
                    prefix,
                    game_index,
                )?;
                changed.extend(apply_feature_cards_to_mod_with_index(
                    root,
                    &feature_cards,
                    tag,
                    prefix,
                    game_index,
                )?);
            }
            if !event_cards.is_empty() {
                enforce_strict_event_card_gate_with_options(
                    validation_options,
                    &event_cards,
                    game_index,
                )?;
                changed.extend(apply_event_cards_to_mod_with_index(
                    root,
                    &event_cards,
                    tag,
                    prefix,
                    game_index,
                )?);
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
        Some(validate_mod_with_options(
            root,
            game_index,
            validation_options,
        )?)
    } else {
        None
    };
    let text_alignment = if let Some(root) = mod_root {
        Some(text_alignment_report(
            root,
            expected_texts_from_workflow_input(text, focus_layout.as_ref()),
        )?)
    } else {
        None
    };
    let next_steps = workflow_next_steps(
        mod_root.is_some(),
        dry_run,
        validation.as_ref(),
        feature_cards.len()
            + event_cards.len()
            + dynamic_modifier_intents.len()
            + localisation_entries.len()
            + usize::from(has_focus_layout),
        !safety_blockers.is_empty(),
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
        "  \"detected\": {{\"focus_layout\": {}, \"feature_cards\": {}, \"event_cards\": {}, \"dynamic_modifier_intents\": {}, \"localisation_entries\": {}, \"localisation_token_issues\": {}}},\n",
        json_bool(has_focus_layout),
        feature_cards.len(),
        event_cards.len(),
        dynamic_modifier_intents.len(),
        localisation_entries.len(),
        localisation_token_issues.len()
    ));
    out.push_str(&format!(
        "  \"scope_contract\": {},\n",
        requirement_scope_contract_json(&scope_contract)
    ));
    out.push_str(&format!("  \"safety\": {},\n", safety));
    out.push_str(&format!(
        "  \"plans\": {{\"focus_layout\": {}, \"feature_cards\": {}, \"event_cards\": {}}},\n",
        json_optional_raw(focus_plan.as_deref()),
        json_optional_raw(feature_plan.as_deref()),
        json_optional_raw(event_plan.as_deref())
    ));
    out.push_str(&format!(
        "  \"auxiliary_inputs\": {{\"dynamic_modifier_intents\": {}, \"localisation_entries\": {}, \"localisation_token_issues\": {}}},\n",
        json_array(&dynamic_modifier_intents),
        json_array(&localisation_entries),
        json_array(&localisation_token_issues)
    ));
    out.push_str(&format!(
        "  \"changed_files\": {},\n",
        json_array(&changed_files)
    ));
    out.push_str(&format!(
        "  \"validation\": {},\n",
        workflow_validation_json(validation.as_ref())
    ));
    out.push_str(&format!(
        "  \"text_alignment\": {},\n",
        workflow_text_alignment_json(text_alignment.as_ref())
    ));
    out.push_str(&format!("  \"next_steps\": {}\n", json_array(&next_steps)));
    out.push_str("}\n");
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn workflow_strict_gate_blockers(
    validation_options: ValidationOptions,
    mod_root: Option<&Path>,
    focus_layout: Option<&FocusLayout>,
    feature_cards: &[Card],
    event_cards: &[Card],
    tag: &str,
    prefix: &str,
    game_index: Option<&GameIndex>,
) -> Vec<String> {
    if !validation_options.strict_code_index {
        return Vec::new();
    }
    let mut blockers = Vec::new();
    let Some(root) = mod_root else {
        blockers.push(
            "strict dry-run gate requires --mod-root so focus icons and local references can be checked"
                .to_string(),
        );
        return blockers;
    };
    if game_index.is_none() {
        blockers.push(
            "strict dry-run gate requires --game-root so generated code can be checked against the code index"
                .to_string(),
        );
        return blockers;
    }
    if let Some(layout) = focus_layout {
        if let Err(err) = enforce_strict_focus_layout_gate_with_options(
            validation_options,
            root,
            layout,
            tag,
            game_index,
        ) {
            blockers.push(format!("focus_layout strict gate: {err}"));
        }
    }
    if !feature_cards.is_empty() {
        if let Err(err) = enforce_strict_feature_card_gate_with_options(
            validation_options,
            feature_cards,
            tag,
            prefix,
            game_index,
        ) {
            blockers.push(format!("feature_cards strict gate: {err}"));
        }
    }
    if !event_cards.is_empty() {
        if let Err(err) =
            enforce_strict_event_card_gate_with_options(validation_options, event_cards, game_index)
        {
            blockers.push(format!("event_cards strict gate: {err}"));
        }
    }
    blockers
}

pub(crate) fn workflow_safety_blockers(
    focus_layout: Option<&FocusLayout>,
    feature_cards: &[Card],
    event_cards: &[Card],
    tag: &str,
    prefix: &str,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if let Some(layout) = focus_layout {
        blockers.extend(
            focus_node_safety_blockers_for_layout(layout)
                .into_iter()
                .map(|blocker| format!("focus_layout: {blocker}")),
        );
    }
    blockers.extend(
        suggestions_safety_blockers(&feature_cards_safety_suggestions(
            feature_cards,
            tag,
            prefix,
        ))
        .into_iter()
        .map(|blocker| format!("feature_cards: {blocker}")),
    );
    blockers.extend(
        suggestions_safety_blockers(&event_cards_safety_suggestions(event_cards))
            .into_iter()
            .map(|blocker| format!("event_cards: {blocker}")),
    );
    blockers
}

pub(crate) fn workflow_safety_json_from_blockers(blockers: &[String]) -> String {
    let status = if blockers.is_empty() {
        "verified_shape"
    } else {
        "blocked"
    };
    format!(
        "{{\"status\": {}, \"final_code_allowed\": {}, \"blockers\": {}}}",
        json_str(status),
        json_bool(blockers.is_empty()),
        json_array(blockers)
    )
}

pub(crate) fn focus_node_safety_blockers_for_layout(layout: &FocusLayout) -> Vec<String> {
    layout
        .focuses
        .iter()
        .flat_map(focus_node_safety_blockers)
        .collect()
}

pub(crate) fn requirement_scope_contract(
    text: &str,
    has_focus_layout: bool,
    tag: &str,
    prefix: &str,
) -> RequirementScopeContract {
    let lower = text.to_ascii_lowercase();
    let wants_focus = has_focus_layout || text.contains("国策") || lower.contains("focus");
    let wants_events = text.contains("事件") || lower.contains("event");
    let wants_ideas =
        text.contains("民族精神") || lower.contains("national spirit") || lower.contains("idea");
    let wants_country_creation = request_explicitly_creates_country_tag(text);
    let wants_country_history = contains_any(text, &["国家历史", "history/countries", "开局政治"]);
    let wants_units = contains_any(
        text,
        &["初始军队", "初始部队", "部队编制", "history/units", "oob"],
    );
    let wants_characters = contains_any(
        text,
        &["创建领袖", "创建领导人", "创建人物", "common/characters"],
    );
    let wants_english = contains_any(text, &["英文本地化", "英文翻译", "localisation/english"]);
    let wants_states = contains_any(text, &["history/states", "州历史", "修改州", "修改省份"]);
    let wants_decisions = text.contains("决议") || lower.contains("decision");
    let wants_technology = text.contains("科技") || lower.contains("technology");
    let wants_gui = lower.contains("gui") || text.contains("特殊界面");

    let mut authorized_systems = Vec::new();
    let mut planned_files = Vec::new();
    if text.contains("mod") || text.contains("MOD") || text.contains("模组") {
        authorized_systems.push("new_mod_descriptor".to_string());
        planned_files.push("descriptor.mod (and launcher-side .mod when requested)".to_string());
    }
    if wants_focus {
        authorized_systems.push("national_focus".to_string());
        planned_files.push(format!("common/national_focus/{prefix}_focus.txt"));
    }
    if wants_ideas {
        authorized_systems.push("national_spirits".to_string());
        planned_files.push(format!("common/ideas/{prefix}_ideas.txt"));
    }
    if wants_events {
        authorized_systems.push("events".to_string());
        planned_files.push(format!("events/{prefix}_events.txt"));
    }
    if wants_focus || wants_ideas || wants_events {
        authorized_systems.push("simplified_chinese_localisation".to_string());
        planned_files.push(format!(
            "localisation/simp_chinese/{}_l_simp_chinese.yml",
            tag.to_ascii_uppercase()
        ));
    }
    for (wanted, system, file) in [
        (
            wants_country_creation,
            "country_definition",
            "common/country_tags and common/countries",
        ),
        (
            wants_country_history,
            "country_history",
            "history/countries",
        ),
        (wants_units, "initial_units", "history/units"),
        (wants_characters, "characters", "common/characters"),
        (
            wants_english,
            "english_localisation",
            "localisation/english",
        ),
        (wants_states, "state_history", "history/states"),
        (wants_decisions, "decisions", "common/decisions"),
        (wants_technology, "technologies", "common/technologies"),
        (
            wants_gui,
            "custom_gui",
            "common/scripted_guis and interface",
        ),
    ] {
        if wanted {
            authorized_systems.push(system.to_string());
            planned_files.push(file.to_string());
        }
    }

    let mut forbidden_without_explicit_request = Vec::new();
    for (wanted, path) in [
        (
            wants_country_creation,
            "common/country_tags and common/countries (do not redefine an existing vanilla tag)",
        ),
        (wants_country_history, "history/countries"),
        (wants_units, "history/units"),
        (wants_characters, "common/characters"),
        (wants_english, "localisation/english"),
        (wants_states, "history/states"),
        (wants_decisions, "common/decisions"),
        (wants_technology, "common/technologies"),
        (wants_gui, "common/scripted_guis and interface/*.gui"),
    ] {
        if !wanted {
            forbidden_without_explicit_request.push(path.to_string());
        }
    }

    authorized_systems.sort();
    authorized_systems.dedup();
    planned_files.sort();
    planned_files.dedup();
    forbidden_without_explicit_request.sort();
    forbidden_without_explicit_request.dedup();

    RequirementScopeContract {
        authorized_systems,
        minimum_events: requested_minimum(text, &["事件", "event", "events"]),
        minimum_ideas: requested_minimum(
            text,
            &[
                "民族精神",
                "national spirit",
                "national spirits",
                "idea",
                "ideas",
            ],
        ),
        planned_files,
        forbidden_without_explicit_request,
        rules: vec![
            "A new mod authorizes a new folder, not every HOI4 subsystem.".to_string(),
            "Create only files required by explicit requirements or unavoidable runtime wiring."
                .to_string(),
            "Do not create empty placeholder files or speculative country/history/unit/character files."
                .to_string(),
            "Do not rename, paraphrase, add, remove, or aesthetically rearrange spreadsheet focuses."
                .to_string(),
            "Every referenced sprite, modifier, technology, equipment, sub-unit, state, and province must be locally observed or game/dependency indexed."
                .to_string(),
            "Warnings about unresolved game resources are unfinished work; do not report validation success until indexed validation is clean."
                .to_string(),
            "LLMs must submit structured focus, decision, event, and national-spirit inputs to Rust generators; they must not handwrite common/national_focus, common/decisions, common/ideas, or events scripts unless the user explicitly requests direct manual Clausewitz/file editing."
                .to_string(),
            "General requests such as create a mod, make it complete, fix it, or add content are not permission for manual script editing; only the user's explicit request to handwrite or directly edit those script files is an exception."
                .to_string(),
            "Without that explicit user exception, if a requested field is not supported by the generator, extend the generator first instead of bypassing it with manual Clausewitz code."
                .to_string(),
        ],
    }
}

fn requested_minimum(text: &str, nouns: &[&str]) -> Option<usize> {
    let lower = text.to_ascii_lowercase();
    for noun in nouns {
        let noun = noun.to_ascii_lowercase();
        let Some(position) = lower.find(&noun) else {
            continue;
        };
        let before = lower[..position]
            .chars()
            .rev()
            .take(16)
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>();
        let after = lower[position + noun.len()..]
            .chars()
            .take(24)
            .collect::<String>();
        let after_digits = after
            .split(|ch: char| !ch.is_ascii_digit())
            .filter(|part| !part.is_empty())
            .filter_map(|part| part.parse::<usize>().ok())
            .collect::<Vec<_>>();
        if let Some(value) = after_digits.first() {
            return Some(*value);
        }
        let before_digits = before
            .split(|ch: char| !ch.is_ascii_digit())
            .filter(|part| !part.is_empty())
            .filter_map(|part| part.parse::<usize>().ok())
            .collect::<Vec<_>>();
        if let Some(value) = before_digits.last() {
            return Some(*value);
        }
    }
    None
}

pub(crate) fn requirement_scope_contract_json(scope: &RequirementScopeContract) -> String {
    format!(
        "{{\"minimal_modification\": true, \"authorized_systems\": {}, \"minimums\": {{\"events\": {}, \"national_spirits\": {}}}, \"planned_files\": {}, \"forbidden_without_explicit_request\": {}, \"rules\": {}}}",
        json_array(&scope.authorized_systems),
        scope
            .minimum_events
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string()),
        scope
            .minimum_ideas
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string()),
        json_array(&scope.planned_files),
        json_array(&scope.forbidden_without_explicit_request),
        json_array(&scope.rules)
    )
}

pub(crate) fn focus_layout_json(layout: &FocusLayout, tag: &str, prefix: &str) -> String {
    let focus_blocking_safety_count = layout
        .focuses
        .iter()
        .flat_map(focus_node_safety_blockers)
        .count();
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"schema\": \"hoi4skill.focus_layout.v1\",\n");
    out.push_str(&format!("  \"tag\": {},\n", json_str(tag)));
    out.push_str(&format!("  \"prefix\": {},\n", json_str(prefix)));
    out.push_str(&format!(
        "  \"focus_blocking_safety_count\": {},\n",
        focus_blocking_safety_count
    ));
    out.push_str(&format!("  \"tree_id\": {},\n", json_str(&layout.tree_id)));
    out.push_str(&format!(
        "  \"safety\": {},\n",
        focus_layout_safety_json(layout)
    ));
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
            "{{\"title\": {}, \"id\": {}, \"icon\": {}, \"x\": {}, \"y\": {}, \"worksheet_x\": {}, \"worksheet_y\": {}, \"relative_position_id\": {}, \"row\": {}, \"column\": {}, \"prerequisite\": {}, \"mutually_exclusive\": {}, \"completion_reward\": {}, \"safety\": {}}}",
            json_str(&f.title),
            json_str(&f.id),
            json_optional_str(f.icon.as_deref()),
            f.relative_x.unwrap_or(f.x),
            f.relative_y.unwrap_or(f.y),
            f.x,
            f.y,
            json_optional_str(f.relative_position_id.as_deref()),
            f.row,
            f.column,
            json_array(&f.prerequisite),
            json_array(&f.mutually_exclusive),
            json_array(&f.completion_reward),
            focus_node_safety_json(f)
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

pub(crate) fn focus_layout_safety_json(layout: &FocusLayout) -> String {
    let blockers = layout
        .focuses
        .iter()
        .flat_map(focus_node_safety_blockers)
        .collect::<Vec<_>>();
    focus_safety_json_from_blockers(&blockers)
}

pub(crate) fn focus_node_safety_json(focus: &FocusNode) -> String {
    let blockers = focus_node_safety_blockers(focus);
    focus_safety_json_from_blockers(&blockers)
}

pub(crate) fn focus_node_safety_blockers(focus: &FocusNode) -> Vec<String> {
    let mut blockers = Vec::new();
    for line in &focus.completion_reward {
        if let Some(marker) = unresolved_generation_marker(line) {
            blockers.push(format!(
                "focus `{}` completion_reward contains unresolved marker `{marker}`: {}",
                focus.title, line
            ));
        } else if line.contains('<') || line.contains('>') {
            blockers.push(format!(
                "focus `{}` completion_reward contains unresolved placeholder: {}",
                focus.title, line
            ));
        }
    }
    blockers
}

pub(crate) fn focus_safety_json_from_blockers(blockers: &[String]) -> String {
    let status = if blockers.is_empty() {
        "verified_shape"
    } else {
        "blocked"
    };
    format!(
        "{{\"status\": {}, \"final_code_allowed\": {}, \"blockers\": {}}}",
        json_str(status),
        json_bool(blockers.is_empty()),
        json_array(blockers)
    )
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
            if active && is_focus_layout_explanatory_field(key, value) {
                continue;
            }
        }
        if active && !is_focus_layout_noise_line(trimmed) {
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

pub(crate) fn is_focus_layout_explanatory_field(key: &str, value: &str) -> bool {
    let key = key.trim();
    if matches!(
        key,
        "说明" | "注释" | "备注" | "注意" | "解释" | "输出" | "代码"
    ) {
        return true;
    }
    let value = value.trim();
    value.is_empty()
        && matches!(
            key,
            "下面" | "以下" | "如下" | "国策树代码" | "国策布局说明"
        )
}

pub(crate) fn is_focus_layout_noise_line(trimmed: &str) -> bool {
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.starts_with("```") || trimmed == "~~~" || trimmed.starts_with("~~~") {
        return true;
    }
    if trimmed.starts_with('#') && !is_focus_layout_reward_comment(trimmed) {
        return true;
    }
    if contains_any(
        trimmed,
        &[
            "下面是按",
            "下面是你要的",
            "下面是国策",
            "以下是按",
            "以下是你要的",
            "以下是国策",
            "如你所说",
            "我将为",
            "我会为",
        ],
    ) && !trimmed.contains('|')
        && split_focus_line(trimmed).len() <= 1
    {
        return true;
    }
    false
}

pub(crate) fn is_focus_layout_reward_comment(trimmed: &str) -> bool {
    let Some(comment) = trimmed.trim_start().strip_prefix('#') else {
        return false;
    };
    let comment = comment.trim_start();
    comment.starts_with("completion_reward:")
        || comment.starts_with("completion_reward：")
        || comment.starts_with("国策效果:")
        || comment.starts_with("国策效果：")
}

pub(crate) fn is_workflow_card_header(key: &str) -> bool {
    is_feature_card_header(key)
        || matches!(
            normalise_workflow_lane_label(key).as_str(),
            "事件"
                | "event"
                | "动态修正"
                | "dynamicmodifier"
                | "本地化"
                | "localisation"
                | "localization"
                | "loc"
        )
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

pub(crate) fn workflow_text_alignment_json(report: Option<&TextAlignmentReport>) -> String {
    if let Some(report) = report {
        text_alignment_report_json(report).trim().to_string()
    } else {
        "{\"ran\": false}".to_string()
    }
}

pub(crate) fn workflow_next_steps(
    has_mod_root: bool,
    dry_run: bool,
    reporter: Option<&Reporter>,
    detected_sections: usize,
    safety_blocked: bool,
) -> Vec<String> {
    let mut steps = Vec::new();
    if safety_blocked {
        steps.push(
            "先解决 safety.blockers：把 raw/placeholder 意图映射到 code-catalog 中已验证的 effect、trigger、modifier 或资源。"
                .to_string(),
        );
    }
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
