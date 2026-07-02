//! Plan-only writers for high-value `common/*` systems.
//!
//! These commands produce structured edit plans before any low-level writer is
//! allowed to touch a Clausewitz file. The main job is to make missing symbols
//! and ambiguous targets loud instead of letting generated code silently fail.

#[allow(unused_imports)]
use crate::*;

#[derive(Clone)]
struct SourceRoot {
    role: &'static str,
    root: PathBuf,
}

#[derive(Clone)]
struct SymbolMatch {
    role: &'static str,
    root: PathBuf,
    file: PathBuf,
    line: usize,
}

pub(crate) fn cmd_on_action_insert_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let target_root = resolve_mod_root(&mod_root)?.root;
    let on_action = require_value(&map, "on-action")?;
    let event = require_value(&map, "event")?;
    let target_file = value(&map, "target-file").map(str::to_string);
    let event_command = value(&map, "event-command").unwrap_or("country_event");

    if !matches!(
        event_command,
        "country_event" | "news_event" | "state_event" | "unit_event" | "operative_event"
    ) {
        return Err(format!(
            "--event-command {event_command} is not a known HOI4 event command"
        ));
    }

    let roots = source_roots(&map, &target_root)?;
    let on_action_matches = collect_on_action_matches(&roots, &on_action)?;
    let event_matches = collect_event_id_matches(&roots, &event)?;

    let mut blockers = Vec::new();
    let mut questions = Vec::new();
    if on_action_matches.is_empty() {
        blockers.push(format!(
            "on_action `{on_action}` is not registered in target, parent, or game common/on_actions"
        ));
    }
    if event_matches.is_empty() {
        blockers.push(format!(
            "event `{event}` is not registered in target, parent, or game events"
        ));
    }

    let selected_on_action = select_on_action_match(
        &on_action_matches,
        target_file.as_deref(),
        &target_root,
        &mut blockers,
        &mut questions,
    );

    let mut operations = Vec::new();
    if blockers.is_empty() {
        if let Some(selected) = selected_on_action.as_ref() {
            operations.push(format!(
                "insert `{event_command} = {{ id = {event} }}` under `{on_action}` in {}",
                relative_slash_path(&selected.root, &selected.file)
            ));
        }
    }

    let ok = blockers.is_empty();
    let json = on_action_insert_plan_json(OnActionInsertReport {
        ok,
        target_root: &target_root,
        on_action: &on_action,
        event: &event,
        event_command,
        target_file: target_file.as_deref(),
        roots: &roots,
        on_action_matches: &on_action_matches,
        event_matches: &event_matches,
        operations: &operations,
        blockers: &blockers,
        questions: &questions,
    });
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

struct CommonSystemSpec {
    command: &'static str,
    common_dir: &'static str,
    id_args: &'static [&'static str],
    value_keys: &'static [&'static str],
    default_file_suffix: &'static str,
    template_rule: &'static str,
}

#[derive(Clone, Copy)]
struct CommonWriterRegistrySpec {
    system: &'static str,
    common_dir: &'static str,
    writer_status: &'static str,
    allowed_containers: &'static [&'static str],
    required_scope: &'static str,
    symbol_kinds: &'static [&'static str],
    apply_mode: &'static str,
    rollback_mode: &'static str,
    note: &'static str,
}

pub(crate) fn cmd_common_writer_registry(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let target_root = if let Some(raw) = value(&map, "mod-root") {
        resolve_mod_root(&normalize_path(raw)?)?.root
    } else {
        PathBuf::from(".")
    };
    let roots = source_roots(&map, &target_root)?;
    let mut blockers = Vec::new();
    if roots.is_empty() {
        blockers.push("no local source roots were provided".to_string());
    }
    let specs = common_writer_registry_specs();
    let observed = specs
        .iter()
        .map(|spec| common_writer_registry_row(spec, &roots))
        .collect::<Result<Vec<_>, _>>()?;
    let ok = blockers.is_empty()
        && specs.iter().all(|spec| {
            !spec.allowed_containers.is_empty()
                && !spec.required_scope.is_empty()
                && !spec.symbol_kinds.is_empty()
                && !spec.apply_mode.is_empty()
                && !spec.rollback_mode.is_empty()
        });
    let json = common_writer_registry_json(&roots, &observed, &blockers, ok);
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_scripted_effect_plan(args: &[String]) -> Result<(), String> {
    cmd_common_definition_plan(
        args,
        CommonSystemSpec {
            command: "scripted-effect-plan",
            common_dir: "scripted_effects",
            id_args: &["effect", "id", "name"],
            value_keys: &[],
            default_file_suffix: "_scripted_effects.txt",
            template_rule: "scripted effects are top-level keyed blocks; the body must be assembled from indexed effect templates and scope/container gates",
        },
    )
}

pub(crate) fn cmd_scripted_trigger_plan(args: &[String]) -> Result<(), String> {
    cmd_common_definition_plan(
        args,
        CommonSystemSpec {
            command: "scripted-trigger-plan",
            common_dir: "scripted_triggers",
            id_args: &["trigger", "id", "name"],
            value_keys: &[],
            default_file_suffix: "_scripted_triggers.txt",
            template_rule: "scripted triggers are top-level keyed blocks; trigger contents must come from indexed trigger templates",
        },
    )
}

pub(crate) fn cmd_scripted_localisation_plan(args: &[String]) -> Result<(), String> {
    cmd_common_definition_plan(
        args,
        CommonSystemSpec {
            command: "scripted-localisation-plan",
            common_dir: "scripted_localisation",
            id_args: &["key", "id", "name"],
            value_keys: &["name"],
            default_file_suffix: "_scripted_loc.txt",
            template_rule: "scripted localisation ids are discovered from `name = <id>`; create a dedicated defined_text block only through a later writer",
        },
    )
}

pub(crate) fn cmd_opinion_modifier_plan(args: &[String]) -> Result<(), String> {
    cmd_common_definition_plan(
        args,
        CommonSystemSpec {
            command: "opinion-modifier-plan",
            common_dir: "opinion_modifiers",
            id_args: &["modifier", "id"],
            value_keys: &[],
            default_file_suffix: "_opinion_modifiers.txt",
            template_rule:
                "opinion modifiers are top-level keyed blocks; duplicate keys are blockers",
        },
    )
}

pub(crate) fn cmd_game_rule_plan(args: &[String]) -> Result<(), String> {
    cmd_common_definition_plan(
        args,
        CommonSystemSpec {
            command: "game-rule-plan",
            common_dir: "game_rules",
            id_args: &["rule", "id", "name"],
            value_keys: &["name"],
            default_file_suffix: "_game_rules.txt",
            template_rule: "game rules are discovered from `name = <id>` inside game_rule blocks; options need a later schema-aware writer",
        },
    )
}

pub(crate) fn cmd_bookmark_plan(args: &[String]) -> Result<(), String> {
    cmd_common_definition_plan(
        args,
        CommonSystemSpec {
            command: "bookmark-plan",
            common_dir: "bookmarks",
            id_args: &["bookmark", "id", "name"],
            value_keys: &["name"],
            default_file_suffix: "_bookmarks.txt",
            template_rule: "bookmarks are discovered from `name = <id>`; country entries and dates need a later schema-aware writer",
        },
    )
}

pub(crate) fn cmd_bop_plan(args: &[String]) -> Result<(), String> {
    cmd_common_definition_plan(
        args,
        CommonSystemSpec {
            command: "bop-plan",
            common_dir: "bop",
            id_args: &["bop", "id"],
            value_keys: &["id"],
            default_file_suffix: "_bop.txt",
            template_rule: "balance-of-power objects are discovered from `id = <id>`; range sides need a later schema-aware writer",
        },
    )
}

pub(crate) fn cmd_ai_strategy_definition_plan(args: &[String]) -> Result<(), String> {
    cmd_common_definition_plan(
        args,
        CommonSystemSpec {
            command: "ai-strategy-definition-plan",
            common_dir: "ai_strategy",
            id_args: &["strategy", "id", "name"],
            value_keys: &["type", "id"],
            default_file_suffix: "_ai_strategy.txt",
            template_rule: "AI strategy entries must use indexed strategy types, ids, and target scopes; route logic must be checked before apply",
        },
    )
}

pub(crate) fn cmd_ai_strategy_plan_file(args: &[String]) -> Result<(), String> {
    cmd_common_definition_plan(
        args,
        CommonSystemSpec {
            command: "ai-strategy-plan-file",
            common_dir: "ai_strategy_plans",
            id_args: &["strategy-id", "plan", "id"],
            value_keys: &["id"],
            default_file_suffix: "_ai_strategy_plan.txt",
            template_rule: "AI strategy plans are discovered from `id = <id>`; strategy contents must use indexed AI strategy types",
        },
    )
}

pub(crate) fn cmd_character_common_plan(args: &[String]) -> Result<(), String> {
    cmd_common_definition_plan(
        args,
        CommonSystemSpec {
            command: "character-common-plan",
            common_dir: "characters",
            id_args: &["character", "id"],
            value_keys: &["id"],
            default_file_suffix: "_characters.txt",
            template_rule: "characters require portrait, traits, ideology, advisor/leader roles, localisation, and history recruit evidence before a schema writer may apply",
        },
    )
}

pub(crate) fn cmd_country_leader_common_plan(args: &[String]) -> Result<(), String> {
    cmd_common_definition_plan(
        args,
        CommonSystemSpec {
            command: "country-leader-common-plan",
            common_dir: "country_leader",
            id_args: &["leader", "id", "name"],
            value_keys: &["id", "name"],
            default_file_suffix: "_country_leader.txt",
            template_rule: "legacy country leaders are dependency-style sensitive; prefer indexed parent syntax and block if the target uses common/characters",
        },
    )
}

pub(crate) fn cmd_unit_common_plan(args: &[String]) -> Result<(), String> {
    cmd_common_definition_plan(
        args,
        CommonSystemSpec {
            command: "unit-common-plan",
            common_dir: "units",
            id_args: &["unit", "id", "name"],
            value_keys: &["type", "id"],
            default_file_suffix: "_units.txt",
            template_rule: "unit definitions must be resolved through unit-taxonomy-build; unknown parent-mod units remain questions, not guessed aliases",
        },
    )
}

pub(crate) fn cmd_technology_common_plan(args: &[String]) -> Result<(), String> {
    cmd_common_definition_plan(
        args,
        CommonSystemSpec {
            command: "technology-common-plan",
            common_dir: "technologies",
            id_args: &["technology", "tech", "id"],
            value_keys: &["id"],
            default_file_suffix: "_technologies.txt",
            template_rule: "technologies require indexed folder/tree/category/equipment references and cannot be created from a name alone",
        },
    )
}

pub(crate) fn cmd_mio_common_plan(args: &[String]) -> Result<(), String> {
    cmd_common_definition_plan(
        args,
        CommonSystemSpec {
            command: "mio-common-plan",
            common_dir: "military_industrial_organization",
            id_args: &["mio", "organization", "id"],
            value_keys: &["id"],
            default_file_suffix: "_mio.txt",
            template_rule: "MIO content must stay in MIO containers; MIO modifiers are blockers if routed into ideas, states, or country history",
        },
    )
}

pub(crate) fn cmd_common_writer_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let system = require_value(&map, "system")?;
    match normalize_common_writer_system(&system).as_str() {
        "on_actions" => cmd_on_action_insert_plan(args),
        "scripted_effects" => cmd_scripted_effect_plan(args),
        "scripted_triggers" => cmd_scripted_trigger_plan(args),
        "scripted_localisation" => cmd_scripted_localisation_plan(args),
        "opinion_modifiers" => cmd_opinion_modifier_plan(args),
        "bookmarks" => cmd_bookmark_plan(args),
        "game_rules" => cmd_game_rule_plan(args),
        "bop" => cmd_bop_plan(args),
        "ai_strategy" => cmd_ai_strategy_definition_plan(args),
        "ai_strategy_plans" => cmd_ai_strategy_plan_file(args),
        "characters" => cmd_character_common_plan(args),
        "country_leader" => cmd_country_leader_common_plan(args),
        "units" => cmd_unit_common_plan(args),
        "technologies" => cmd_technology_common_plan(args),
        "military_industrial_organization" => cmd_mio_common_plan(args),
        other => Err(format!(
            "unsupported common writer system `{other}`; run common-writer-registry for supported systems and writer policies"
        )),
    }
}

pub(crate) fn cmd_common_writer_apply(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let text = read_utf8_lossy(&input)?;
    let mut blockers = Vec::new();
    if !text.contains("\"schema\": \"hoi4skill.common_definition_plan.v1\"")
        && !text.contains("\"schema\": \"hoi4skill.on_action_insert_plan.v1\"")
    {
        blockers.push("input is not a supported common writer plan".to_string());
    }
    if !text.contains("\"ok\": true") {
        blockers.push("common writer plan is not ok".to_string());
    }
    if !map.flags.contains("execute") {
        blockers.push("common-writer-apply requires --execute".to_string());
    }
    if !map.flags.contains("final-check") {
        blockers.push("common-writer-apply requires --final-check".to_string());
    }
    if !map.flags.contains("atomic") {
        blockers.push("common-writer-apply requires --atomic".to_string());
    }
    let changed_files = common_writer_plan_changed_files(&text);
    if changed_files.is_empty() {
        blockers.push("common writer plan has no target changed_files".to_string());
    }
    let output_dir = value(&map, "output-dir")
        .map(normalize_path)
        .transpose()?
        .unwrap_or_else(|| PathBuf::from(".hoi4skill").join("common_writer_apply"));
    fs::create_dir_all(&output_dir).map_err(|e| format!("create {}: {e}", output_dir.display()))?;
    let changed_path = output_dir.join("changed_files.txt");
    fs::write(&changed_path, changed_files.join("\n"))
        .map_err(|e| format!("write {}: {e}", changed_path.display()))?;
    let rollback_path = output_dir.join("rollback_plan.md");
    fs::write(
        &rollback_path,
        common_writer_rollback_markdown(&input, &changed_files),
    )
    .map_err(|e| format!("write {}: {e}", rollback_path.display()))?;
    let ok = blockers.is_empty();
    let report = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"input\": {},\n  \"execute\": {},\n  \"final_check\": {},\n  \"atomic\": {},\n  \"changed_files\": {},\n  \"changed_files_report\": {},\n  \"rollback_plan\": {},\n  \"blockers\": {},\n  \"rules\": {}\n}}\n",
        json_str("hoi4skill.common_writer_apply.v1"),
        json_bool(ok),
        json_str(if ok { "common_writer_review_pack_ready" } else { "blocked" }),
        json_str(&input.display().to_string()),
        json_bool(map.flags.contains("execute")),
        json_bool(map.flags.contains("final-check")),
        json_bool(map.flags.contains("atomic")),
        json_array(&changed_files),
        json_str(&changed_path.display().to_string()),
        json_str(&rollback_path.display().to_string()),
        json_array(&blockers),
        json_array(&[
            "common-writer-apply is the P73 apply gate; schema-specific writers must consume this review pack before mutating common files".to_string(),
            "final release still requires validate --strict-code-index and runtime-evidence-gate".to_string(),
        ])
    );
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

fn cmd_common_definition_plan(args: &[String], spec: CommonSystemSpec) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let target_root = resolve_mod_root(&mod_root)?.root;
    let id = first_present_value(&map, spec.id_args)
        .ok_or_else(|| format!("missing one of {}", spec.id_args.join(", ")))?;
    let target_file = value(&map, "target-file")
        .map(str::to_string)
        .unwrap_or_else(|| {
            format!(
                "common/{}/{}{}",
                spec.common_dir,
                sanitize_common_file_stem(&id),
                spec.default_file_suffix
            )
        });
    let allow_existing = map.flags.contains("allow-existing");
    let roots = source_roots(&map, &target_root)?;
    let matches = collect_common_definition_matches(&roots, spec.common_dir, &id, spec.value_keys)?;

    let mut blockers = Vec::new();
    if !allow_existing && !matches.is_empty() {
        blockers.push(format!(
            "{} `{id}` already exists; pass --allow-existing only when intentionally extending it",
            spec.common_dir
        ));
    }
    if !target_file.starts_with(&format!("common/{}/", spec.common_dir)) {
        blockers.push(format!(
            "--target-file must stay under common/{}/",
            spec.common_dir
        ));
    }

    let operations = if blockers.is_empty() {
        vec![format!(
            "plan {} `{id}` in {target_file}; final writer must use a schema-specific template",
            spec.common_dir
        )]
    } else {
        Vec::new()
    };
    let ok = blockers.is_empty();
    let json = common_definition_plan_json(CommonDefinitionReport {
        ok,
        command: spec.command,
        common_dir: spec.common_dir,
        target_root: &target_root,
        id: &id,
        target_file: &target_file,
        roots: &roots,
        matches: &matches,
        operations: &operations,
        blockers: &blockers,
        template_rule: spec.template_rule,
    });
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

fn source_roots(map: &ArgMap, target_root: &Path) -> Result<Vec<SourceRoot>, String> {
    let mut roots = Vec::new();
    roots.push(SourceRoot {
        role: "target",
        root: target_root.to_path_buf(),
    });
    for raw in repeated_values(map, "mod-path") {
        let root = resolve_mod_root(&normalize_path(raw)?)?.root;
        if !roots.iter().any(|entry| entry.root == root) {
            roots.push(SourceRoot {
                role: "parent",
                root,
            });
        }
    }
    if let Some(raw) = value(map, "game-root") {
        let root = normalize_path(raw)?;
        if !roots.iter().any(|entry| entry.root == root) {
            roots.push(SourceRoot { role: "game", root });
        }
    }
    Ok(roots)
}

fn first_present_value(map: &ArgMap, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value(map, key).map(str::to_string))
}

fn collect_common_definition_matches(
    roots: &[SourceRoot],
    common_dir: &str,
    id: &str,
    value_keys: &[&str],
) -> Result<Vec<SymbolMatch>, String> {
    let mut matches = Vec::new();
    for source in roots {
        let dir = source.root.join("common").join(common_dir);
        if !dir.is_dir() {
            continue;
        }
        for file in collect_files(&dir)? {
            if file.extension().and_then(OsStr::to_str) != Some("txt") {
                continue;
            }
            let text = read_utf8_lossy(&file)?;
            for (idx, line) in text.lines().enumerate() {
                let code = line.split('#').next().unwrap_or("").trim();
                let direct_key_match = assignment_key(code) == Some(id);
                let value_match = value_keys.iter().any(|key| {
                    assignment_value(code, key)
                        .map(unquote_token)
                        .is_some_and(|value| value == id)
                });
                if direct_key_match || value_match {
                    matches.push(SymbolMatch {
                        role: source.role,
                        root: source.root.clone(),
                        file: file.clone(),
                        line: idx + 1,
                    });
                }
            }
        }
    }
    Ok(matches)
}

fn sanitize_common_file_stem(value: &str) -> String {
    let stem = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if stem.is_empty() {
        "generated".to_string()
    } else {
        stem
    }
}

fn normalize_common_writer_system(system: &str) -> String {
    match system.to_ascii_lowercase().replace('-', "_").as_str() {
        "on_action" | "on_actions" => "on_actions".to_string(),
        "scripted_effect" | "scripted_effects" => "scripted_effects".to_string(),
        "scripted_trigger" | "scripted_triggers" => "scripted_triggers".to_string(),
        "scripted_localisation" | "scripted_localization" | "scripted_loc" => {
            "scripted_localisation".to_string()
        }
        "opinion" | "opinion_modifier" | "opinion_modifiers" => "opinion_modifiers".to_string(),
        "bookmark" | "bookmarks" => "bookmarks".to_string(),
        "game_rule" | "game_rules" | "gamerule" | "gamerules" => "game_rules".to_string(),
        "balance_of_power" | "bop" => "bop".to_string(),
        "ai_strategy" => "ai_strategy".to_string(),
        "ai_strategy_plan" | "ai_strategy_plans" => "ai_strategy_plans".to_string(),
        "character" | "characters" => "characters".to_string(),
        "country_leader" | "country_leaders" | "legacy_leader" => "country_leader".to_string(),
        "unit" | "units" => "units".to_string(),
        "technology" | "technologies" | "tech" => "technologies".to_string(),
        "mio" | "military_industrial_organization" | "military_industrial_organizations" => {
            "military_industrial_organization".to_string()
        }
        other => other.to_string(),
    }
}

fn common_writer_plan_changed_files(text: &str) -> Vec<String> {
    let mut files = json_string_array_field(text, "changed_files");
    if files.is_empty() {
        if let Some(target_file) = common_writer_json_string_field(text, "target_file") {
            if !target_file.trim().is_empty() && target_file != "null" {
                files.push(target_file);
            }
        }
    }
    files.sort();
    files.dedup();
    files
}

fn common_writer_json_string_field(text: &str, key: &str) -> Option<String> {
    let marker = format!("\"{key}\":");
    let start = text.find(&marker)?;
    let rest = text[start + marker.len()..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let mut out = String::new();
    let mut escaped = false;
    for ch in rest.chars() {
        if escaped {
            out.push(match ch {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '"' => '"',
                '\\' => '\\',
                other => other,
            });
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(out);
        } else {
            out.push(ch);
        }
    }
    None
}

fn common_writer_rollback_markdown(input: &Path, changed_files: &[String]) -> String {
    let mut out = String::new();
    out.push_str("# Common Writer Rollback Plan\n\n");
    out.push_str(&format!("- input: `{}`\n", input.display()));
    out.push_str(
        "- action: restore changed common files from VCS or backup if any final gate fails.\n\n",
    );
    out.push_str("## Changed Files\n\n");
    for file in changed_files {
        out.push_str(&format!("- `{file}`\n"));
    }
    out
}

fn collect_on_action_matches(
    roots: &[SourceRoot],
    on_action: &str,
) -> Result<Vec<SymbolMatch>, String> {
    let mut matches = Vec::new();
    for source in roots {
        let dir = source.root.join("common").join("on_actions");
        if !dir.is_dir() {
            continue;
        }
        for file in collect_files(&dir)? {
            if file.extension().and_then(OsStr::to_str) != Some("txt") {
                continue;
            }
            let text = read_utf8_lossy(&file)?;
            for (idx, line) in text.lines().enumerate() {
                let code = line.split('#').next().unwrap_or("").trim();
                if assignment_key(code) == Some(on_action) {
                    matches.push(SymbolMatch {
                        role: source.role,
                        root: source.root.clone(),
                        file: file.clone(),
                        line: idx + 1,
                    });
                }
            }
        }
    }
    Ok(matches)
}

fn collect_event_id_matches(roots: &[SourceRoot], event: &str) -> Result<Vec<SymbolMatch>, String> {
    let mut matches = Vec::new();
    for source in roots {
        let dir = source.root.join("events");
        if !dir.is_dir() {
            continue;
        }
        for file in collect_files(&dir)? {
            if file.extension().and_then(OsStr::to_str) != Some("txt") {
                continue;
            }
            let text = read_utf8_lossy(&file)?;
            for (idx, line) in text.lines().enumerate() {
                let code = line.split('#').next().unwrap_or("").trim();
                if assignment_value(code, "id")
                    .map(unquote_token)
                    .is_some_and(|value| value == event)
                {
                    matches.push(SymbolMatch {
                        role: source.role,
                        root: source.root.clone(),
                        file: file.clone(),
                        line: idx + 1,
                    });
                }
            }
        }
    }
    Ok(matches)
}

fn select_on_action_match(
    matches: &[SymbolMatch],
    target_file: Option<&str>,
    target_root: &Path,
    blockers: &mut Vec<String>,
    questions: &mut Vec<String>,
) -> Option<SymbolMatch> {
    if matches.is_empty() {
        return None;
    }
    if let Some(target_file) = target_file {
        let normalized = target_file.replace('\\', "/");
        let selected = matches
            .iter()
            .find(|entry| relative_slash_path(&entry.root, &entry.file) == normalized)
            .cloned();
        if selected.is_none() {
            blockers.push(format!(
                "--target-file {target_file} does not contain the requested on_action"
            ));
        }
        return selected;
    }

    let target_matches = matches
        .iter()
        .filter(|entry| entry.root == target_root)
        .cloned()
        .collect::<Vec<_>>();
    if target_matches.len() == 1 {
        return target_matches.into_iter().next();
    }
    if target_matches.len() > 1 {
        blockers.push(
            "multiple target on_action definitions matched; pass --target-file to choose one"
                .to_string(),
        );
        return None;
    }
    if matches.len() == 1 {
        questions.push(
            "on_action exists only outside target mod; writer should create an override or patch file explicitly before applying"
                .to_string(),
        );
        return matches.first().cloned();
    }
    blockers
        .push("multiple parent/game on_action definitions matched; pass --target-file".to_string());
    None
}

fn unquote_token(value: &str) -> &str {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim_end_matches('{')
        .trim()
}

struct OnActionInsertReport<'a> {
    ok: bool,
    target_root: &'a Path,
    on_action: &'a str,
    event: &'a str,
    event_command: &'a str,
    target_file: Option<&'a str>,
    roots: &'a [SourceRoot],
    on_action_matches: &'a [SymbolMatch],
    event_matches: &'a [SymbolMatch],
    operations: &'a [String],
    blockers: &'a [String],
    questions: &'a [String],
}

fn on_action_insert_plan_json(report: OnActionInsertReport<'_>) -> String {
    let mut map = BTreeMap::new();
    map.insert(
        "schema".to_string(),
        json_str("hoi4skill.on_action_insert_plan.v1"),
    );
    map.insert("ok".to_string(), json_bool(report.ok).to_string());
    map.insert(
        "status".to_string(),
        json_str(if report.ok {
            "on_action_insert_plan_ready"
        } else {
            "on_action_insert_plan_blocked"
        }),
    );
    map.insert(
        "target_root".to_string(),
        json_str(&report.target_root.display().to_string()),
    );
    map.insert("on_action".to_string(), json_str(report.on_action));
    map.insert("event".to_string(), json_str(report.event));
    map.insert("event_command".to_string(), json_str(report.event_command));
    map.insert(
        "target_file".to_string(),
        json_optional_str(report.target_file),
    );
    map.insert("source_roots".to_string(), source_roots_json(report.roots));
    map.insert(
        "on_action_matches".to_string(),
        symbol_matches_json(report.on_action_matches),
    );
    map.insert(
        "event_matches".to_string(),
        symbol_matches_json(report.event_matches),
    );
    map.insert("operations".to_string(), json_array(report.operations));
    map.insert("blockers".to_string(), json_array(report.blockers));
    map.insert("questions".to_string(), json_array(report.questions));
    map.insert(
        "rules".to_string(),
        json_array(&[
            "do not invent on_action ids; missing id is a blocker".to_string(),
            "do not invent event ids; missing event is a blocker".to_string(),
            "this command is plan-only and must be followed by final validate before writing"
                .to_string(),
        ]),
    );
    json_raw_object(&map) + "\n"
}

struct CommonDefinitionReport<'a> {
    ok: bool,
    command: &'a str,
    common_dir: &'a str,
    target_root: &'a Path,
    id: &'a str,
    target_file: &'a str,
    roots: &'a [SourceRoot],
    matches: &'a [SymbolMatch],
    operations: &'a [String],
    blockers: &'a [String],
    template_rule: &'a str,
}

fn common_definition_plan_json(report: CommonDefinitionReport<'_>) -> String {
    let mut map = BTreeMap::new();
    map.insert(
        "schema".to_string(),
        json_str("hoi4skill.common_definition_plan.v1"),
    );
    map.insert("ok".to_string(), json_bool(report.ok).to_string());
    map.insert(
        "status".to_string(),
        json_str(if report.ok {
            "common_definition_plan_ready"
        } else {
            "common_definition_plan_blocked"
        }),
    );
    map.insert("command".to_string(), json_str(report.command));
    map.insert("common_dir".to_string(), json_str(report.common_dir));
    map.insert(
        "target_root".to_string(),
        json_str(&report.target_root.display().to_string()),
    );
    map.insert("id".to_string(), json_str(report.id));
    map.insert("target_file".to_string(), json_str(report.target_file));
    map.insert(
        "changed_files".to_string(),
        json_array(&[report.target_file.to_string()]),
    );
    map.insert("source_roots".to_string(), source_roots_json(report.roots));
    map.insert("matches".to_string(), symbol_matches_json(report.matches));
    map.insert("operations".to_string(), json_array(report.operations));
    map.insert("blockers".to_string(), json_array(report.blockers));
    map.insert(
        "rules".to_string(),
        json_array(&[
            "existing definitions are blockers unless --allow-existing is explicit".to_string(),
            "target files must stay inside the expected common directory".to_string(),
            report.template_rule.to_string(),
        ]),
    );
    json_raw_object(&map) + "\n"
}

struct CommonWriterRegistryRow {
    spec: CommonWriterRegistrySpec,
    observations: Vec<String>,
}

fn common_writer_registry_specs() -> Vec<CommonWriterRegistrySpec> {
    vec![
        CommonWriterRegistrySpec {
            system: "on_actions",
            common_dir: "on_actions",
            writer_status: "implemented_plan_only",
            allowed_containers: &["common/on_actions"],
            required_scope: "event_dispatch",
            symbol_kinds: &["on_action", "event_id"],
            apply_mode: "review_pack_then_schema_writer",
            rollback_mode: "changed_file_manifest",
            note: "registered on_actions and event ids are required before insertion",
        },
        CommonWriterRegistrySpec {
            system: "scripted_effects",
            common_dir: "scripted_effects",
            writer_status: "implemented_plan_only",
            allowed_containers: &["common/scripted_effects"],
            required_scope: "effect_scope_declared_by_indexed_template",
            symbol_kinds: &["effect", "scripted_effect", "modifier", "variable"],
            apply_mode: "review_pack_then_scope_gate",
            rollback_mode: "changed_file_manifest",
            note: "effects may be shared only when the local code evidence proves compatible scopes",
        },
        CommonWriterRegistrySpec {
            system: "scripted_triggers",
            common_dir: "scripted_triggers",
            writer_status: "implemented_plan_only",
            allowed_containers: &["common/scripted_triggers"],
            required_scope: "trigger_scope_declared_by_indexed_template",
            symbol_kinds: &["trigger", "scripted_trigger"],
            apply_mode: "review_pack_then_scope_gate",
            rollback_mode: "changed_file_manifest",
            note: "trigger syntax must come from the local trigger index",
        },
        CommonWriterRegistrySpec {
            system: "scripted_localisation",
            common_dir: "scripted_localisation",
            writer_status: "implemented_plan_only",
            allowed_containers: &["common/scripted_localisation", "localisation"],
            required_scope: "localisation_token_scope",
            symbol_kinds: &["scripted_localisation", "loc_key", "token"],
            apply_mode: "review_pack_then_token_gate",
            rollback_mode: "changed_file_manifest",
            note: "scripted localisation must preserve ROOT/FROM variables, color tokens, and icon tokens",
        },
        CommonWriterRegistrySpec {
            system: "opinion_modifiers",
            common_dir: "opinion_modifiers",
            writer_status: "implemented_plan_only",
            allowed_containers: &["common/opinion_modifiers"],
            required_scope: "country_relation",
            symbol_kinds: &["opinion_modifier", "tag"],
            apply_mode: "review_pack_then_schema_writer",
            rollback_mode: "changed_file_manifest",
            note: "opinion modifiers must be used only by country-scope diplomacy effects",
        },
        CommonWriterRegistrySpec {
            system: "bookmarks",
            common_dir: "bookmarks",
            writer_status: "implemented_plan_only",
            allowed_containers: &["common/bookmarks"],
            required_scope: "start_date",
            symbol_kinds: &["bookmark", "tag", "history", "portrait", "loc_key"],
            apply_mode: "review_pack_then_scenario_gate",
            rollback_mode: "changed_file_manifest",
            note: "bookmarks require matching history, localisation, and start-date evidence",
        },
        CommonWriterRegistrySpec {
            system: "game_rules",
            common_dir: "game_rules",
            writer_status: "implemented_plan_only",
            allowed_containers: &["common/game_rules"],
            required_scope: "global_rule",
            symbol_kinds: &["game_rule", "loc_key", "trigger"],
            apply_mode: "review_pack_then_schema_writer",
            rollback_mode: "changed_file_manifest",
            note: "game rule ids must be indexed before focus/event/decision triggers reference them",
        },
        CommonWriterRegistrySpec {
            system: "bop",
            common_dir: "bop",
            writer_status: "implemented_plan_only",
            allowed_containers: &["common/bop"],
            required_scope: "country_balance_of_power",
            symbol_kinds: &["bop", "bop_side", "modifier", "loc_key"],
            apply_mode: "review_pack_then_schema_writer",
            rollback_mode: "changed_file_manifest",
            note: "BOP sides, ranges, modifiers, and decision/event mutation points must be connected",
        },
        CommonWriterRegistrySpec {
            system: "ai_strategy",
            common_dir: "ai_strategy",
            writer_status: "implemented_plan_only",
            allowed_containers: &["common/ai_strategy"],
            required_scope: "country_ai",
            symbol_kinds: &["ai_strategy", "tag", "trigger"],
            apply_mode: "review_pack_then_route_gate",
            rollback_mode: "changed_file_manifest",
            note: "AI strategy must be tied to route logic and indexed strategy types",
        },
        CommonWriterRegistrySpec {
            system: "ai_strategy_plans",
            common_dir: "ai_strategy_plans",
            writer_status: "implemented_plan_only",
            allowed_containers: &["common/ai_strategy_plans"],
            required_scope: "country_ai",
            symbol_kinds: &["ai_strategy_plan", "tag", "trigger"],
            apply_mode: "review_pack_then_route_gate",
            rollback_mode: "changed_file_manifest",
            note: "strategy plans cannot invent route ids or target tags",
        },
        CommonWriterRegistrySpec {
            system: "characters",
            common_dir: "characters",
            writer_status: "implemented_plan_only",
            allowed_containers: &["common/characters", "history/countries"],
            required_scope: "country_character",
            symbol_kinds: &["character", "portrait", "trait", "ideology", "loc_key"],
            apply_mode: "review_pack_then_scenario_gate",
            rollback_mode: "changed_file_manifest",
            note: "characters must be recruited from history only when the user authorizes leader/history edits",
        },
        CommonWriterRegistrySpec {
            system: "country_leader",
            common_dir: "country_leader",
            writer_status: "legacy_plan_only",
            allowed_containers: &["common/country_leader"],
            required_scope: "legacy_country_leader",
            symbol_kinds: &["leader", "ideology", "trait", "portrait", "loc_key"],
            apply_mode: "review_pack_then_dependency_style_gate",
            rollback_mode: "changed_file_manifest",
            note: "legacy leader syntax must match the dependency style; modern character syntax is preferred when indexed",
        },
        CommonWriterRegistrySpec {
            system: "units",
            common_dir: "units",
            writer_status: "implemented_plan_only",
            allowed_containers: &["common/units", "history/units"],
            required_scope: "unit_taxonomy",
            symbol_kinds: &["unit", "sub_unit", "equipment", "division_template"],
            apply_mode: "review_pack_then_unit_taxonomy_gate",
            rollback_mode: "changed_file_manifest",
            note: "parent-mod unit aliases must come from unit-taxonomy-build, not hard-coded names",
        },
        CommonWriterRegistrySpec {
            system: "technologies",
            common_dir: "technologies",
            writer_status: "implemented_plan_only",
            allowed_containers: &["common/technologies", "history/countries"],
            required_scope: "technology_tree",
            symbol_kinds: &["technology", "equipment", "folder", "category"],
            apply_mode: "review_pack_then_technology_gate",
            rollback_mode: "changed_file_manifest",
            note: "technology ids must be indexed before history grants or focus rewards reference them",
        },
        CommonWriterRegistrySpec {
            system: "military_industrial_organization",
            common_dir: "military_industrial_organization",
            writer_status: "implemented_plan_only",
            allowed_containers: &["common/military_industrial_organization"],
            required_scope: "mio",
            symbol_kinds: &["mio", "mio_policy", "mio_trait", "modifier"],
            apply_mode: "review_pack_then_mio_scope_gate",
            rollback_mode: "changed_file_manifest",
            note: "MIO modifiers must not be routed into national spirits, states, or whole-country history",
        },
    ]
}

fn common_writer_registry_row(
    spec: &CommonWriterRegistrySpec,
    roots: &[SourceRoot],
) -> Result<CommonWriterRegistryRow, String> {
    let mut observations = Vec::new();
    for root in roots {
        let dir = root.root.join("common").join(spec.common_dir);
        if !dir.is_dir() {
            observations.push(format!("{}:missing", root.role));
            continue;
        }
        let file_count = collect_files(&dir)?
            .into_iter()
            .filter(|path| path.extension().and_then(OsStr::to_str) == Some("txt"))
            .count();
        observations.push(format!("{}:{}_txt_files", root.role, file_count));
    }
    Ok(CommonWriterRegistryRow {
        spec: *spec,
        observations,
    })
}

fn common_writer_registry_json(
    roots: &[SourceRoot],
    rows: &[CommonWriterRegistryRow],
    blockers: &[String],
    ok: bool,
) -> String {
    let mut map = BTreeMap::new();
    map.insert(
        "schema".to_string(),
        json_str("hoi4skill.common_writer_registry.v1"),
    );
    map.insert("ok".to_string(), json_bool(ok).to_string());
    map.insert(
        "status".to_string(),
        json_str(if ok {
            "common_writer_registry_ready"
        } else {
            "common_writer_registry_blocked"
        }),
    );
    map.insert("source_roots".to_string(), source_roots_json(roots));
    map.insert(
        "systems".to_string(),
        format!(
            "[{}]",
            rows.iter()
                .map(common_writer_registry_row_json)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    );
    map.insert("blockers".to_string(), json_array(blockers));
    map.insert(
        "rules".to_string(),
        json_array(&[
            "registry stores writer contracts and source-layer observations, not game or parent-mod source code".to_string(),
            "unknown, ambiguous, or unsupported common systems must stop at review pack".to_string(),
            "shared symbols require local evidence for every allowed container before apply".to_string(),
        ]),
    );
    json_raw_object(&map) + "\n"
}

fn common_writer_registry_row_json(row: &CommonWriterRegistryRow) -> String {
    format!(
        "{{\"system\": {}, \"common_dir\": {}, \"writer_status\": {}, \"allowed_containers\": {}, \"required_scope\": {}, \"symbol_kinds\": {}, \"apply_mode\": {}, \"rollback_mode\": {}, \"source_observations\": {}, \"note\": {}}}",
        json_str(row.spec.system),
        json_str(row.spec.common_dir),
        json_str(row.spec.writer_status),
        json_array(&row.spec.allowed_containers.iter().map(|value| value.to_string()).collect::<Vec<_>>()),
        json_str(row.spec.required_scope),
        json_array(&row.spec.symbol_kinds.iter().map(|value| value.to_string()).collect::<Vec<_>>()),
        json_str(row.spec.apply_mode),
        json_str(row.spec.rollback_mode),
        json_array(&row.observations),
        json_str(row.spec.note)
    )
}

fn source_roots_json(roots: &[SourceRoot]) -> String {
    format!(
        "[{}]",
        roots
            .iter()
            .map(|entry| {
                format!(
                    "{{\"role\": {}, \"root\": {}}}",
                    json_str(entry.role),
                    json_str(&entry.root.display().to_string())
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn symbol_matches_json(matches: &[SymbolMatch]) -> String {
    format!(
        "[{}]",
        matches
            .iter()
            .map(|entry| {
                format!(
                    "{{\"role\": {}, \"root\": {}, \"file\": {}, \"relative_file\": {}, \"line\": {}}}",
                    json_str(entry.role),
                    json_str(&entry.root.display().to_string()),
                    json_str(&entry.file.display().to_string()),
                    json_str(&relative_slash_path(&entry.root, &entry.file)),
                    entry.line
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}
