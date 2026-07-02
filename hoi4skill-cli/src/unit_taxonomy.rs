//! P24 dynamic unit/OOB taxonomy gates.

#[allow(unused_imports)]
use crate::*;

#[derive(Clone)]
struct UnitTaxonomyEntry {
    id: String,
    class: String,
    aliases: Vec<String>,
    source_file: String,
    source_kind: String,
    evidence: Vec<String>,
}

#[derive(Clone)]
struct DivisionUnitSpec {
    sub_unit: String,
    count: i64,
    class: String,
}

pub(crate) fn cmd_unit_taxonomy_build(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let game_root = normalize_path(&require_value(&map, "game-root")?)?;
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let dependency_roots =
        dependency_mod_roots_for_optional_edited_mod(&map, mod_root.as_deref(), true)?;
    let index = build_game_index_with_mod_paths(&game_root, &dependency_roots)?;
    let mut roots = vec![game_root.clone()];
    roots.extend(dependency_roots);
    let entries = build_unit_taxonomy_entries(&roots, &index)?;
    let blockers = unit_taxonomy_blockers(&entries);
    let ok = blockers.is_empty();
    let json = render_unit_taxonomy_json(&game_root, mod_root.as_deref(), &entries, &blockers);
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_unit_intent_classify(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let taxonomy_path = normalize_path(&require_value(&map, "taxonomy")?)?;
    let text = history_plan_input_text(&map)?;
    let taxonomy = read_utf8_lossy(&taxonomy_path)?;
    let entries = parse_unit_taxonomy_entries(&taxonomy);
    let matches = classify_unit_intent_matches(&entries, &text);
    let blockers = unit_intent_blockers(&matches);
    let ok = blockers.is_empty();
    let json = render_unit_intent_json(&taxonomy_path, &text, &matches, &blockers);
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_oob_template_resolve(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = history_plan_input_text(&map)?;
    let (taxonomy_source, entries) = if let Some(raw) = value(&map, "taxonomy") {
        let taxonomy_path = normalize_path(raw)?;
        let taxonomy = read_utf8_lossy(&taxonomy_path)?;
        if !taxonomy.contains("\"schema\": \"hoi4skill.unit_taxonomy.v1\"") {
            return Err("input is not a unit taxonomy report".to_string());
        }
        (
            taxonomy_path.display().to_string(),
            parse_unit_taxonomy_entries(&taxonomy),
        )
    } else {
        let game_root = normalize_path(&require_value(&map, "game-root")?)?;
        let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
        let dependency_roots =
            dependency_mod_roots_for_optional_edited_mod(&map, mod_root.as_deref(), true)?;
        let index = build_game_index_with_mod_paths(&game_root, &dependency_roots)?;
        let mut roots = vec![game_root.clone()];
        roots.extend(dependency_roots);
        (
            format!("built_from:{}", game_root.display()),
            build_unit_taxonomy_entries(&roots, &index)?,
        )
    };
    let kind = classify_oob_kind_from_taxonomy(&entries, &text);
    let matches = classify_unit_intent_matches(&entries, &text);
    let specs = division_specs_from_text(&entries, &text);
    let regiments = specs
        .iter()
        .filter(|spec| matches!(spec.class.as_str(), "line_battalion" | "special_forces"))
        .cloned()
        .collect::<Vec<_>>();
    let support = specs
        .iter()
        .filter(|spec| spec.class == "support_company")
        .cloned()
        .collect::<Vec<_>>();
    let unknown_matches = matches
        .iter()
        .filter(|entry| entry.class == "special_or_unknown")
        .map(|entry| entry.id.clone())
        .collect::<Vec<_>>();
    let mut blockers = Vec::new();
    let mut questions = Vec::new();
    if matches.is_empty() {
        blockers.push("no indexed unit aliases matched request text".to_string());
        questions.push(
            "Which indexed parent/target unit or equipment template should this OOB use?"
                .to_string(),
        );
    }
    if !unknown_matches.is_empty() {
        blockers.push(format!(
            "matched units require user classification before OOB writing: {}",
            unknown_matches.join(", ")
        ));
        questions.push("Classify matched special units as line_battalion, support_company, air_wing, or naval_ship before apply.".to_string());
    }
    if kind == "land" && regiments.is_empty() {
        blockers.push(
            "land OOB template requires at least one indexed line battalion or special forces unit"
                .to_string(),
        );
    }
    if kind == "air" {
        questions.push(
            "Run air-oob-plan with indexed aircraft equipment, province, and amount.".to_string(),
        );
    } else if kind == "naval" {
        questions.push(
            "Run naval-oob-plan with indexed ship equipment, base province, and amount."
                .to_string(),
        );
    } else if kind == "unknown" {
        blockers
            .push("OOB kind is ambiguous; choose land, air, or naval before writing".to_string());
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"taxonomy\": {},\n  \"text\": {},\n  \"kind\": {},\n  \"matches\": {},\n  \"regiments\": {},\n  \"support\": {},\n  \"questions\": {},\n  \"blockers\": {},\n  \"next_commands\": {},\n  \"rules\": {}\n}}\n",
        json_str("hoi4skill.oob_template_resolve.v1"),
        json_bool(ok),
        json_str(if ok { "oob_template_resolved" } else { "needs_user_confirmation" }),
        json_str(&taxonomy_source),
        json_str(&text),
        json_str(&kind),
        unit_entries_json(&matches),
        division_specs_json(&regiments),
        division_specs_json(&support),
        json_array(&questions),
        json_array(&blockers),
        json_array(&oob_template_next_commands(&kind)),
        json_array(&[
            "unit aliases are read from indexed game/parent/target common/units and localisation evidence".to_string(),
            "do not hard-code parent-mod unit names; unknown or ambiguous custom units require user classification".to_string(),
            "air and naval OOB requests must use typed OOB plans, not land division-template writers".to_string(),
        ])
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_unit_taxonomy_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let taxonomy_path = normalize_path(&require_value(&map, "taxonomy")?)?;
    let taxonomy = read_utf8_lossy(&taxonomy_path)?;
    let entries = parse_unit_taxonomy_entries(&taxonomy);
    let mut blockers = Vec::new();
    if !taxonomy.contains("\"schema\": \"hoi4skill.unit_taxonomy.v1\"") {
        blockers.push("input is not a unit taxonomy report".to_string());
    }
    if entries.is_empty() {
        blockers.push("unit taxonomy has no indexed sub-units".to_string());
    }
    blockers.extend(unit_taxonomy_blockers(&entries));
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"taxonomy\": {},\n  \"unit_count\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.unit_taxonomy_audit.v1"),
        json_bool(ok),
        json_str(if ok { "unit_taxonomy_ok" } else { "blocked" }),
        json_str(&taxonomy_path.display().to_string()),
        entries.len(),
        json_array(&blockers),
        json_str("unit taxonomy must be built from indexed game/parent/target unit definitions; unknown classes require user confirmation before OOB writing")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_division_template_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let taxonomy_path = normalize_path(&require_value(&map, "taxonomy")?)?;
    let text = history_plan_input_text(&map)?;
    let taxonomy = read_utf8_lossy(&taxonomy_path)?;
    let entries = parse_unit_taxonomy_entries(&taxonomy);
    let name = value(&map, "name")
        .map(str::to_string)
        .or_else(|| infer_division_template_name(&text))
        .unwrap_or_else(|| "generated_division_template".to_string());
    let mut blockers = Vec::new();
    if !taxonomy.contains("\"schema\": \"hoi4skill.unit_taxonomy.v1\"") {
        blockers.push("input is not a unit taxonomy report".to_string());
    }
    let specs = division_specs_from_text(&entries, &text);
    let regiments = specs
        .iter()
        .filter(|spec| matches!(spec.class.as_str(), "line_battalion" | "special_forces"))
        .cloned()
        .collect::<Vec<_>>();
    let support = specs
        .iter()
        .filter(|spec| spec.class == "support_company")
        .cloned()
        .collect::<Vec<_>>();
    let unknown = specs
        .iter()
        .filter(|spec| spec.class == "special_or_unknown")
        .map(|spec| spec.sub_unit.clone())
        .collect::<Vec<_>>();
    if specs.is_empty() {
        blockers.push("no indexed unit aliases matched division template text".to_string());
    }
    if regiments.is_empty() {
        blockers.push(
            "division template requires at least one line battalion or special forces battalion"
                .to_string(),
        );
    }
    if !unknown.is_empty() {
        blockers.push(format!(
            "cannot write unknown unit classes into a division template: {}",
            unknown.join(", ")
        ));
    }
    let regiment_slots: i64 = regiments.iter().map(|spec| spec.count).sum();
    let support_slots: i64 = support.iter().map(|spec| spec.count).sum();
    if regiment_slots > 25 {
        blockers.push(format!(
            "division template has {regiment_slots} regiment slots; maximum supported writer layout is 25"
        ));
    }
    if support_slots > 5 {
        blockers.push(format!(
            "division template has {support_slots} support companies; maximum supported writer layout is 5"
        ));
    }
    let ok = blockers.is_empty();
    let json = render_division_template_plan_json(
        &taxonomy_path,
        &text,
        &name,
        &regiments,
        &support,
        &blockers,
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_division_template_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let plan = read_utf8_lossy(&input)?;
    let mut blockers = Vec::new();
    if !plan.contains("\"schema\": \"hoi4skill.division_template_plan.v1\"") {
        blockers.push("input is not a division-template-plan report".to_string());
    }
    if !plan.contains("\"ok\": true") {
        blockers.push("division template plan is not ok".to_string());
    }
    let regiments = division_specs_from_plan_json(&plan, "regiments");
    let support = division_specs_from_plan_json(&plan, "support");
    if regiments.is_empty() {
        blockers.push("division template plan has no regiments".to_string());
    }
    if regiments.iter().any(|spec| spec.class == "support_company") {
        blockers.push("support company appears in regiments".to_string());
    }
    if support
        .iter()
        .any(|spec| matches!(spec.class.as_str(), "line_battalion" | "special_forces"))
    {
        blockers.push("line battalion appears in support".to_string());
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"input\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.division_template_audit.v1"),
        json_bool(ok),
        json_str(if ok { "division_template_ok" } else { "blocked" }),
        json_str(&input.display().to_string()),
        json_array(&blockers),
        json_str("regiments and support must remain separated before division_template code is assembled")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_division_template_apply(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let oob = require_value(&map, "oob")?;
    let plan = read_utf8_lossy(&input)?;
    let mut blockers = Vec::new();
    if !map.flags.contains("execute") {
        blockers.push("division-template-apply requires --execute".to_string());
    }
    if !map.flags.contains("final-check") {
        blockers.push("division-template-apply requires --final-check".to_string());
    }
    if !plan.contains("\"schema\": \"hoi4skill.division_template_plan.v1\"") {
        blockers.push("input is not a division-template-plan report".to_string());
    }
    if !plan.contains("\"ok\": true") {
        blockers.push("division template plan is not ok".to_string());
    }
    let name = json_string_field(&plan, "division_name").unwrap_or_default();
    let regiments = division_specs_from_plan_json(&plan, "regiments");
    let support = division_specs_from_plan_json(&plan, "support");
    if name.is_empty() {
        blockers.push("division template plan is missing division_name".to_string());
    }
    if regiments.is_empty() {
        blockers.push("division template plan is missing regiments".to_string());
    }
    let target_file = mod_root
        .join("history")
        .join("units")
        .join(format!("{oob}.txt"));
    let mut changed_files = Vec::new();
    if blockers.is_empty() {
        if target_file.exists() {
            let existing = read_utf8_lossy(&target_file)?;
            if existing.contains(&format!("name = \"{name}\""))
                || existing.contains(&format!("division_template = \"{name}\""))
            {
                blockers.push(format!(
                    "division template `{name}` already appears in {}",
                    target_file.display()
                ));
            }
        }
        if blockers.is_empty() {
            fs::create_dir_all(target_file.parent().unwrap())
                .map_err(|e| format!("create {}: {e}", target_file.display()))?;
            let mut text = if target_file.exists() {
                read_utf8_lossy(&target_file)?
            } else {
                String::new()
            };
            if !text.ends_with('\n') && !text.is_empty() {
                text.push('\n');
            }
            text.push('\n');
            text.push_str(&render_division_template_code(&name, &regiments, &support));
            fs::write(&target_file, text)
                .map_err(|e| format!("write {}: {e}", target_file.display()))?;
            changed_files.push(target_file.display().to_string());
        }
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"input\": {},\n  \"changed_files\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.division_template_apply.v1"),
        json_bool(ok),
        json_str(if ok { "division_template_applied" } else { "blocked" }),
        json_str(&input.display().to_string()),
        json_array(&changed_files),
        json_array(&blockers),
        json_str("Rust writer assembles division_template only after taxonomy class separation and final-check gate")
    );
    write_or_print(&json, value(&map, "output"))?;
    if (map.flags.contains("require-passed") || !ok) && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_oob_kind_classify(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = history_plan_input_text(&map)?;
    let taxonomy = value(&map, "taxonomy").map(normalize_path).transpose()?;
    let kind = if let Some(path) = &taxonomy {
        let entries = parse_unit_taxonomy_entries(&read_utf8_lossy(path)?);
        classify_oob_kind_from_taxonomy(&entries, &text)
    } else {
        classify_oob_kind_from_text(&text)
    };
    let blockers = if kind == "unknown" {
        vec!["OOB kind is ambiguous; choose land, air, or naval before writing".to_string()]
    } else {
        Vec::new()
    };
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"text\": {},\n  \"kind\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.oob_kind_classify.v1"),
        json_bool(ok),
        json_str(if ok { "oob_kind_classified" } else { "needs_user_confirmation" }),
        json_str(&text),
        json_str(&kind),
        json_array(&blockers),
        json_str("air and naval OOB requests must not be routed to land division writers")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_air_oob_plan(args: &[String]) -> Result<(), String> {
    cmd_typed_oob_plan(args, "air")
}

pub(crate) fn cmd_naval_oob_plan(args: &[String]) -> Result<(), String> {
    cmd_typed_oob_plan(args, "naval")
}

pub(crate) fn cmd_oob_kind_apply(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let output_dir = value(&map, "output-dir")
        .map(normalize_path)
        .transpose()?
        .unwrap_or_else(|| PathBuf::from(".hoi4skill").join("oob_kind_apply"));
    let plan = read_utf8_lossy(&input)?;
    let mut blockers = Vec::new();
    if !map.flags.contains("execute") {
        blockers.push("oob-kind-apply requires --execute".to_string());
    }
    if !map.flags.contains("final-check") {
        blockers.push("oob-kind-apply requires --final-check".to_string());
    }
    if !plan.contains("\"schema\": \"hoi4skill.typed_oob_plan.v1\"")
        && !plan.contains("\"schema\": \"hoi4skill.oob_kind_classify.v1\"")
    {
        blockers.push("input is not an OOB kind or typed OOB plan report".to_string());
    }
    if !plan.contains("\"ok\": true") {
        blockers.push("input OOB plan is not ok".to_string());
    }
    let mut changed_files = Vec::new();
    if blockers.is_empty() {
        fs::create_dir_all(&output_dir)
            .map_err(|e| format!("create {}: {e}", output_dir.display()))?;
        let readme = output_dir.join("README.md");
        fs::write(
            &readme,
            "OOB kind apply gate passed. Air/naval OOB still requires a dedicated changed-file writer before runtime use.\n",
        )
        .map_err(|e| format!("write {}: {e}", readme.display()))?;
        changed_files.push(readme.display().to_string());
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"input\": {},\n  \"changed_files\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.oob_kind_apply.v1"),
        json_bool(ok),
        json_str(if ok { "oob_kind_apply_pack_written" } else { "blocked" }),
        json_str(&input.display().to_string()),
        json_array(&changed_files),
        json_array(&blockers),
        json_str("P26 apply writes a review pack only; do not paste air/naval OOB into land division files")
    );
    write_or_print(&json, value(&map, "output"))?;
    if (map.flags.contains("require-passed") || !ok) && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_parent_oob_compat_smoke(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = history_plan_input_text(&map)?;
    let game_root = normalize_path(&require_value(&map, "game-root")?)?;
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let parent_roots =
        dependency_mod_roots_for_optional_edited_mod(&map, mod_root.as_deref(), false)?;
    let mut blockers = Vec::new();
    if parent_roots.is_empty() {
        blockers.push(
            "parent OOB compat smoke requires at least one --mod-path or --dependency-mod root"
                .to_string(),
        );
    }
    let index = build_game_index_with_mod_paths(&game_root, &parent_roots)?;
    let mut roots = vec![game_root.clone()];
    roots.extend(parent_roots.iter().cloned());
    if let Some(root) = &mod_root {
        roots.push(root.clone());
    }
    let evidence_roots = parent_compat_evidence_roots(&parent_roots, mod_root.as_deref());
    let entries = build_unit_taxonomy_entries(&evidence_roots, &index)?;
    let matches = classify_unit_intent_matches(&entries, &text);
    let kind = classify_oob_kind_from_taxonomy(&entries, &text);
    if entries.is_empty() {
        blockers
            .push("no indexed sub_units were found in game or parent mod common/units".to_string());
    }
    if matches.is_empty() {
        blockers.push("request did not match any indexed parent/game sub_unit alias".to_string());
    }
    for entry in &matches {
        if entry.class == "special_or_unknown" {
            blockers.push(format!(
                "matched unit `{}` has unknown class; ask the user before writing OOB",
                entry.id
            ));
        }
    }
    let questions = parent_oob_questions(&matches, &blockers);
    let snippets = parent_unit_snippets(&evidence_roots, &matches)?;
    let ok = blockers.is_empty();
    let json = parent_oob_compat_smoke_json(ParentOobCompatReport {
        ok,
        game_root: &game_root,
        mod_root: mod_root.as_deref(),
        parent_roots: &parent_roots,
        text: &text,
        kind: &kind,
        matches: &matches,
        snippets: &snippets,
        questions: &questions,
        blockers: &blockers,
    });
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_parent_history_compat_smoke(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = history_plan_input_text(&map)?;
    let game_root = normalize_path(&require_value(&map, "game-root")?)?;
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let parent_roots =
        dependency_mod_roots_for_optional_edited_mod(&map, mod_root.as_deref(), false)?;
    let mut blockers = Vec::new();
    if parent_roots.is_empty() {
        blockers.push(
            "parent history compat smoke requires at least one --mod-path or --dependency-mod root"
                .to_string(),
        );
    }
    let index = build_game_index_with_mod_paths(&game_root, &parent_roots)?;
    let evidence_roots = parent_compat_evidence_roots(&parent_roots, mod_root.as_deref());
    let country_files = parent_history_file_count(&evidence_roots, "history/countries")?;
    let state_files = parent_history_file_count(&evidence_roots, "history/states")?;
    let unit_files = parent_history_file_count(&evidence_roots, "history/units")?;
    let diplomacy_files = parent_history_file_count(&evidence_roots, "history/diplomacy")?;
    if index.country_tags.is_empty() {
        blockers.push("no indexed country tags from game or parent mod".to_string());
    }
    if state_files == 0 {
        blockers.push("no history/states files found in game or parent mod".to_string());
    }
    if unit_files == 0 {
        blockers.push("no history/units files found in game or parent mod".to_string());
    }
    if parent_compat_text_requests_war(&text) && diplomacy_files == 0 {
        blockers.push("war request has no observed history/diplomacy template".to_string());
    }
    let snippets = parent_history_snippets(&evidence_roots, &text)?;
    let questions = parent_history_questions(&blockers, &snippets);
    let ok = blockers.is_empty();
    let json = parent_history_compat_smoke_json(ParentHistoryCompatReport {
        ok,
        game_root: &game_root,
        mod_root: mod_root.as_deref(),
        parent_roots: &parent_roots,
        text: &text,
        country_files,
        state_files,
        unit_files,
        diplomacy_files,
        snippets: &snippets,
        questions: &questions,
        blockers: &blockers,
    });
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_parent_compat_release_gate(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let inputs = repeated_values(&map, "input")
        .into_iter()
        .map(normalize_path)
        .collect::<Result<Vec<_>, _>>()?;
    let mut blockers = Vec::new();
    if inputs.is_empty() {
        blockers.push(
            "parent-compat-release-gate requires at least one --input smoke report".to_string(),
        );
    }
    let mut reports = Vec::new();
    for input in &inputs {
        let text = read_utf8_lossy(input)?;
        let schema_ok = text.contains("\"schema\": \"hoi4skill.parent_oob_compat_smoke.v1\"")
            || text.contains("\"schema\": \"hoi4skill.parent_history_compat_smoke.v1\"");
        if !schema_ok {
            blockers.push(format!(
                "{} is not a P29 parent compat smoke report",
                input.display()
            ));
        }
        if !text.contains("\"ok\": true") {
            blockers.push(format!(
                "{} did not pass parent compat smoke",
                input.display()
            ));
        }
        reports.push(input.display().to_string());
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"inputs\": {},\n  \"blockers\": {},\n  \"required_final_checks\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.parent_compat_release_gate.v1"),
        json_bool(ok),
        json_str(if ok { "parent_compat_ready" } else { "blocked" }),
        json_array(&reports),
        json_array(&blockers),
        json_array(&[
            "validate --strict-code-index with every parent --mod-path".to_string(),
            "runtime-error-regression after applying changed-only patch packs".to_string(),
            "core-capability-audit --phase all --require-passed".to_string(),
        ]),
        json_str("parent compatibility release requires passing OOB and history smoke reports before generated patches can be considered release candidates")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

struct ParentOobCompatReport<'a> {
    ok: bool,
    game_root: &'a Path,
    mod_root: Option<&'a Path>,
    parent_roots: &'a [PathBuf],
    text: &'a str,
    kind: &'a str,
    matches: &'a [UnitTaxonomyEntry],
    snippets: &'a [String],
    questions: &'a [String],
    blockers: &'a [String],
}

struct ParentHistoryCompatReport<'a> {
    ok: bool,
    game_root: &'a Path,
    mod_root: Option<&'a Path>,
    parent_roots: &'a [PathBuf],
    text: &'a str,
    country_files: usize,
    state_files: usize,
    unit_files: usize,
    diplomacy_files: usize,
    snippets: &'a [String],
    questions: &'a [String],
    blockers: &'a [String],
}

fn parent_oob_questions(matches: &[UnitTaxonomyEntry], blockers: &[String]) -> Vec<String> {
    let mut questions = Vec::new();
    for entry in matches {
        if entry.class == "special_or_unknown" {
            questions.push(format!(
                "Classify parent unit `{}` from `{}` before writing a division or OOB.",
                entry.id, entry.source_file
            ));
        }
    }
    if blockers
        .iter()
        .any(|blocker| blocker.contains("did not match"))
    {
        questions.push(
            "Which exact parent MOD sub_unit/localisation alias should this request use?"
                .to_string(),
        );
    }
    questions
}

fn parent_history_questions(blockers: &[String], snippets: &[String]) -> Vec<String> {
    let mut questions = Vec::new();
    if blockers
        .iter()
        .any(|blocker| blocker.contains("history/diplomacy"))
    {
        questions.push(
            "Provide an observed parent/game start-war diplomacy template before writing war history."
                .to_string(),
        );
    }
    if snippets.is_empty() {
        questions.push(
            "No related parent history snippet was found; provide exact tag/state/OOB ids or broaden the request."
                .to_string(),
        );
    }
    questions
}

fn parent_compat_evidence_roots(parent_roots: &[PathBuf], mod_root: Option<&Path>) -> Vec<PathBuf> {
    let mut roots = parent_roots.to_vec();
    if let Some(root) = mod_root {
        roots.push(root.to_path_buf());
    }
    roots
}

fn parent_unit_snippets(
    roots: &[PathBuf],
    matches: &[UnitTaxonomyEntry],
) -> Result<Vec<String>, String> {
    let mut snippets = Vec::new();
    for entry in matches.iter().take(8) {
        for root in roots {
            let path = root.join(entry.source_file.replace('/', "\\"));
            if !path.exists() {
                continue;
            }
            let text = read_utf8_lossy(&path)?;
            if let Some(snippet) = snippet_around(&text, &entry.id, 10) {
                snippets.push(format!(
                    "{} :: {}",
                    relative_slash_path(root, &path),
                    snippet
                ));
                break;
            }
        }
    }
    Ok(snippets)
}

fn parent_history_snippets(roots: &[PathBuf], text: &str) -> Result<Vec<String>, String> {
    let terms = parent_history_terms(text);
    let mut snippets = Vec::new();
    for root in roots {
        for dir in [
            "history/countries",
            "history/states",
            "history/units",
            "history/diplomacy",
        ] {
            for file in txt_files(root, dir)? {
                let content = read_utf8_lossy(&file)?;
                let hit = terms.iter().find(|term| content.contains(term.as_str()));
                if let Some(term) = hit {
                    if let Some(snippet) = snippet_around(&content, term, 8) {
                        snippets.push(format!(
                            "{} :: {}",
                            relative_slash_path(root, &file),
                            snippet
                        ));
                    }
                }
                if snippets.len() >= 12 {
                    return Ok(snippets);
                }
            }
        }
    }
    Ok(snippets)
}

fn parent_history_terms(text: &str) -> Vec<String> {
    let mut terms = Vec::new();
    for raw in text
        .split(|ch: char| !ch.is_alphanumeric() && !('\u{4e00}'..='\u{9fff}').contains(&ch))
        .map(str::trim)
        .filter(|part| part.len() >= 2)
    {
        terms.push(raw.to_string());
    }
    terms.extend(
        ["owner", "controller", "oob", "war"]
            .iter()
            .map(|s| s.to_string()),
    );
    dedup_parent_terms(terms)
}

fn dedup_parent_terms(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            out.push(value);
        }
    }
    out
}

fn parent_history_file_count(roots: &[PathBuf], dir: &str) -> Result<usize, String> {
    let mut count = 0usize;
    for root in roots {
        count += txt_files(root, dir)?.len();
    }
    Ok(count)
}

fn parent_compat_text_requests_war(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    text.contains("战争")
        || text.contains("开战")
        || text.contains("宣战")
        || lower.contains("war")
        || lower.contains("wargoal")
}

fn snippet_around(text: &str, needle: &str, radius: usize) -> Option<String> {
    let lines = text.lines().collect::<Vec<_>>();
    let hit = lines.iter().position(|line| line.contains(needle))?;
    let start = hit.saturating_sub(radius / 2);
    let end = (hit + radius / 2 + 1).min(lines.len());
    Some(
        lines[start..end]
            .join(" ")
            .chars()
            .take(900)
            .collect::<String>(),
    )
}

fn parent_roots_json(roots: &[PathBuf]) -> String {
    json_array(
        &roots
            .iter()
            .map(|root| root.display().to_string())
            .collect::<Vec<_>>(),
    )
}

fn parent_oob_compat_smoke_json(report: ParentOobCompatReport<'_>) -> String {
    format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"game_root\": {},\n  \"mod_root\": {},\n  \"parent_roots\": {},\n  \"text\": {},\n  \"oob_kind\": {},\n  \"matched_units\": {},\n  \"related_code_snippets\": {},\n  \"questions\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.parent_oob_compat_smoke.v1"),
        json_bool(report.ok),
        json_str(if report.ok { "parent_oob_compat_ready" } else { "blocked" }),
        json_str(&report.game_root.display().to_string()),
        json_optional_str(report.mod_root.map(|root| root.display().to_string()).as_deref()),
        parent_roots_json(report.parent_roots),
        json_str(report.text),
        json_str(report.kind),
        unit_entries_json(report.matches),
        json_array(report.snippets),
        json_array(report.questions),
        json_array(report.blockers),
        json_str("parent MOD OOB compatibility must classify dynamic sub_units from indexed parent/target code and ask about unknown custom structures instead of hardcoding vanilla aliases; official game root may be indexed for validation but must not be emitted as related code snippets")
    )
}

fn parent_history_compat_smoke_json(report: ParentHistoryCompatReport<'_>) -> String {
    format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"game_root\": {},\n  \"mod_root\": {},\n  \"parent_roots\": {},\n  \"text\": {},\n  \"history_file_counts\": {{\"countries\": {}, \"states\": {}, \"units\": {}, \"diplomacy\": {}}},\n  \"related_code_snippets\": {},\n  \"questions\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.parent_history_compat_smoke.v1"),
        json_bool(report.ok),
        json_str(if report.ok { "parent_history_compat_ready" } else { "blocked" }),
        json_str(&report.game_root.display().to_string()),
        json_optional_str(report.mod_root.map(|root| root.display().to_string()).as_deref()),
        parent_roots_json(report.parent_roots),
        json_str(report.text),
        report.country_files,
        report.state_files,
        report.unit_files,
        report.diplomacy_files,
        json_array(report.snippets),
        json_array(report.questions),
        json_array(report.blockers),
        json_str("parent history smoke verifies inherited parent/target history file families and emits related snippets/questions without copying parent files into target output; official game root may be indexed for validation but must not be emitted as related code snippets")
    )
}

fn build_unit_taxonomy_entries(
    roots: &[PathBuf],
    index: &GameIndex,
) -> Result<Vec<UnitTaxonomyEntry>, String> {
    let mut by_id = BTreeMap::<String, UnitTaxonomyEntry>::new();
    for root in roots {
        for file in txt_files(root, "common/units")? {
            if slash_path(&file).contains("/common/units/equipment/") {
                continue;
            }
            let text = strip_comments(&read_utf8_lossy(&file)?);
            for wrapper in blocks_named(&text, "sub_units") {
                for unit_id in direct_block_keys(&wrapper) {
                    if !index.sub_units.contains(&unit_id) {
                        continue;
                    }
                    let Some(unit_block) = first_direct_named_block(&wrapper, &unit_id) else {
                        continue;
                    };
                    let class = classify_unit_block(&unit_id, &unit_block);
                    let aliases = unit_aliases(index, &unit_id);
                    let source_file = rel_slash(root, &file);
                    let source_kind = if root == &index.game_root {
                        "game".to_string()
                    } else {
                        "dependency_or_target_mod".to_string()
                    };
                    let evidence = unit_class_evidence(&unit_block);
                    by_id.insert(
                        unit_id.clone(),
                        UnitTaxonomyEntry {
                            id: unit_id,
                            class,
                            aliases,
                            source_file,
                            source_kind,
                            evidence,
                        },
                    );
                }
            }
        }
    }
    for sub_unit in &index.sub_units {
        by_id
            .entry(sub_unit.clone())
            .or_insert_with(|| UnitTaxonomyEntry {
                id: sub_unit.clone(),
                class: "special_or_unknown".to_string(),
                aliases: unit_aliases(index, sub_unit),
                source_file: "<indexed_without_unit_block>".to_string(),
                source_kind: "index".to_string(),
                evidence: vec!["indexed_sub_unit_without_parseable_common_units_block".to_string()],
            });
    }
    Ok(by_id.into_values().collect())
}

fn classify_unit_block(unit_id: &str, block: &str) -> String {
    let id = unit_id.to_ascii_lowercase();
    let group = block_assignment(block, "group").unwrap_or_default();
    let type_value = block_assignment(block, "type").unwrap_or_default();
    let priority = block_assignment(block, "priority").unwrap_or_default();
    let text = format!(
        "{} {} {} {}",
        id,
        group.to_ascii_lowercase(),
        type_value.to_ascii_lowercase(),
        priority.to_ascii_lowercase()
    );
    if text.contains("support") || id.contains("company") || id.contains("engineer") {
        "support_company".to_string()
    } else if text.contains("special")
        || id.contains("mountaineer")
        || id.contains("marine")
        || id.contains("paratrooper")
    {
        "special_forces".to_string()
    } else if text.contains("air") || id.contains("fighter") || id.contains("bomber") {
        "air_wing".to_string()
    } else if text.contains("naval") || text.contains("ship") || id.contains("ship") {
        "naval_ship".to_string()
    } else if has_line_battalion_evidence(&text, block) {
        "line_battalion".to_string()
    } else {
        "special_or_unknown".to_string()
    }
}

fn has_line_battalion_evidence(text: &str, block: &str) -> bool {
    text.contains("infantry")
        || text.contains("armor")
        || text.contains("armour")
        || text.contains("artillery")
        || text.contains("cavalry")
        || text.contains("motorized")
        || text.contains("mechanized")
        || block_assignment(block, "sprite").is_some()
}

fn unit_class_evidence(block: &str) -> Vec<String> {
    let mut evidence = Vec::new();
    for key in ["group", "type", "priority", "sprite", "map_icon_category"] {
        if let Some(value) = block_assignment(block, key) {
            evidence.push(format!("{key}={value}"));
        }
    }
    if evidence.is_empty() {
        evidence.push("no_known_class_fields".to_string());
    }
    evidence
}

fn unit_aliases(index: &GameIndex, unit_id: &str) -> Vec<String> {
    let mut aliases = Vec::new();
    push_unique_unit_alias(&mut aliases, unit_id);
    push_unique_unit_alias(&mut aliases, &unit_id.replace('_', " "));
    if let Some(localized) = index.localisation_entries.get(unit_id) {
        push_unique_unit_alias(&mut aliases, localized);
        push_unique_unit_alias(&mut aliases, &clean_localisation_icon_label(localized));
    }
    aliases
}

fn push_unique_unit_alias(aliases: &mut Vec<String>, value: &str) {
    let value = value.trim();
    if value.chars().count() < 2 {
        return;
    }
    if !aliases.iter().any(|existing| existing == value) {
        aliases.push(value.to_string());
    }
}

fn first_direct_named_block(text: &str, key: &str) -> Option<String> {
    let pattern = key;
    let mut rest = text;
    while let Some(idx) = rest.find(pattern) {
        let before_ok = idx == 0
            || rest[..idx]
                .chars()
                .last()
                .is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'));
        let after = &rest[idx + pattern.len()..];
        let after_trimmed = after.trim_start();
        if before_ok && after_trimmed.starts_with('=') {
            let after_eq = after_trimmed[1..].trim_start();
            if let Some(block) = after_eq.strip_prefix('{') {
                let wrapped = format!("wrapper = {{{block}");
                return blocks_named(&wrapped, "wrapper").into_iter().next();
            }
        }
        rest = after;
    }
    None
}

fn unit_taxonomy_blockers(entries: &[UnitTaxonomyEntry]) -> Vec<String> {
    let mut blockers = Vec::new();
    if entries
        .iter()
        .any(|entry| entry.class == "special_or_unknown")
    {
        blockers.push("unit taxonomy contains special_or_unknown entries; classify with user confirmation before OOB writing".to_string());
    }
    blockers.extend(unit_alias_collisions(entries));
    blockers
}

fn unit_alias_collisions(entries: &[UnitTaxonomyEntry]) -> Vec<String> {
    let mut aliases = BTreeMap::<String, BTreeSet<String>>::new();
    for entry in entries {
        for alias in &entry.aliases {
            let normalized = normalize_unit_alias(alias);
            if !normalized.is_empty() {
                aliases
                    .entry(normalized)
                    .or_default()
                    .insert(entry.id.clone());
            }
        }
    }
    aliases
        .into_iter()
        .filter(|(_, ids)| ids.len() > 1)
        .take(20)
        .map(|(alias, ids)| {
            format!(
                "ambiguous unit alias `{alias}` matches {}",
                ids.into_iter().collect::<Vec<_>>().join(", ")
            )
        })
        .collect()
}

fn normalize_unit_alias(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || ('\u{4e00}'..='\u{9fff}').contains(ch))
        .flat_map(char::to_lowercase)
        .collect()
}

fn classify_unit_intent_matches(
    entries: &[UnitTaxonomyEntry],
    text: &str,
) -> Vec<UnitTaxonomyEntry> {
    let normalized_text = normalize_unit_alias(text);
    entries
        .iter()
        .filter(|entry| {
            entry.aliases.iter().any(|alias| {
                let alias = normalize_unit_alias(alias);
                !alias.is_empty() && normalized_text.contains(&alias)
            })
        })
        .cloned()
        .collect()
}

fn unit_intent_blockers(matches: &[UnitTaxonomyEntry]) -> Vec<String> {
    let mut blockers = Vec::new();
    if matches.is_empty() {
        blockers.push("no indexed unit alias matched request text".to_string());
    }
    let unknown = matches
        .iter()
        .filter(|entry| entry.class == "special_or_unknown")
        .map(|entry| entry.id.clone())
        .collect::<Vec<_>>();
    if !unknown.is_empty() {
        blockers.push(format!(
            "matched units require user classification before OOB writing: {}",
            unknown.join(", ")
        ));
    }
    blockers
}

fn infer_division_template_name(text: &str) -> Option<String> {
    for raw in text.split(|ch: char| {
        ch.is_whitespace()
            || matches!(
                ch,
                '，' | ',' | '。' | ';' | '；' | ':' | '：' | '\n' | '\r' | '\t'
            )
    }) {
        let value = raw
            .trim_matches(|ch: char| {
                matches!(ch, '“' | '”' | '"' | '\'' | '《' | '》' | '「' | '」')
            })
            .trim();
        if value.contains('师') && !value.contains("个师") && !value.contains("共") {
            return Some(
                value
                    .trim_start_matches("创建")
                    .trim_start_matches("建立")
                    .trim_start_matches("新增")
                    .trim_start_matches("组建")
                    .to_string(),
            );
        }
    }
    None
}

fn division_specs_from_text(entries: &[UnitTaxonomyEntry], text: &str) -> Vec<DivisionUnitSpec> {
    let mut specs = Vec::new();
    for entry in entries {
        let mut matched = None;
        for alias in &entry.aliases {
            if let Some(count) = first_number_before_unit_marker(text, alias) {
                matched = Some(count);
                break;
            }
            if entry.class == "support_company" && unit_text_contains_alias(text, alias) {
                matched = Some(1);
                break;
            }
        }
        if let Some(count) = matched {
            push_or_add_division_spec(
                &mut specs,
                DivisionUnitSpec {
                    sub_unit: entry.id.clone(),
                    count,
                    class: entry.class.clone(),
                },
            );
        }
    }
    specs
}

fn unit_text_contains_alias(text: &str, alias: &str) -> bool {
    let text = normalize_unit_alias(text);
    let alias = normalize_unit_alias(alias);
    !alias.is_empty() && text.contains(&alias)
}

fn push_or_add_division_spec(specs: &mut Vec<DivisionUnitSpec>, spec: DivisionUnitSpec) {
    if let Some(existing) = specs
        .iter_mut()
        .find(|existing| existing.sub_unit == spec.sub_unit && existing.class == spec.class)
    {
        existing.count += spec.count;
    } else {
        specs.push(spec);
    }
}

fn first_number_before_unit_marker(text: &str, marker: &str) -> Option<i64> {
    let lower = text.to_ascii_lowercase();
    let marker = marker.to_ascii_lowercase();
    if marker.trim().is_empty() {
        return None;
    }
    let mut offset = 0usize;
    while let Some(relative_idx) = lower[offset..].find(&marker) {
        let idx = offset + relative_idx;
        if let Some(count) = number_at_end_before_unit_marker(&text[..idx]) {
            return Some(count);
        }
        offset = idx + marker.len();
    }
    None
}

fn number_at_end_before_unit_marker(prefix: &str) -> Option<i64> {
    let trimmed = prefix.trim_end();
    let mut start = trimmed.len();
    for (idx, ch) in trimmed.char_indices().rev() {
        if ch.is_ascii_digit() || is_simple_unit_chinese_number_char(ch) {
            start = idx;
        } else {
            break;
        }
    }
    if start == trimmed.len() || trimmed[..start].chars().last() == Some('第') {
        return None;
    }
    let value = &trimmed[start..];
    value
        .parse::<i64>()
        .ok()
        .or_else(|| parse_simple_unit_chinese_number(value))
}

fn is_simple_unit_chinese_number_char(ch: char) -> bool {
    matches!(
        ch,
        '零' | '一' | '二' | '两' | '三' | '四' | '五' | '六' | '七' | '八' | '九' | '十'
    )
}

fn parse_simple_unit_chinese_number(value: &str) -> Option<i64> {
    if value == "十" {
        return Some(10);
    }
    if let Some((left, right)) = value.split_once('十') {
        let tens = if left.is_empty() {
            1
        } else {
            unit_chinese_digit(left.chars().next()?)?
        };
        let ones = if right.is_empty() {
            0
        } else {
            unit_chinese_digit(right.chars().next()?)?
        };
        return Some(tens * 10 + ones);
    }
    if value.chars().count() == 1 {
        return unit_chinese_digit(value.chars().next()?);
    }
    None
}

fn unit_chinese_digit(ch: char) -> Option<i64> {
    match ch {
        '零' => Some(0),
        '一' => Some(1),
        '二' | '两' => Some(2),
        '三' => Some(3),
        '四' => Some(4),
        '五' => Some(5),
        '六' => Some(6),
        '七' => Some(7),
        '八' => Some(8),
        '九' => Some(9),
        _ => None,
    }
}

fn render_division_template_plan_json(
    taxonomy_path: &Path,
    text: &str,
    name: &str,
    regiments: &[DivisionUnitSpec],
    support: &[DivisionUnitSpec],
    blockers: &[String],
) -> String {
    let ok = blockers.is_empty();
    format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"taxonomy\": {},\n  \"text\": {},\n  \"division_name\": {},\n  \"regiments\": {},\n  \"support\": {},\n  \"blockers\": {},\n  \"rules\": {}\n}}\n",
        json_str("hoi4skill.division_template_plan.v1"),
        json_bool(ok),
        json_str(if ok { "division_template_ready" } else { "blocked" }),
        json_str(&taxonomy_path.display().to_string()),
        json_str(text),
        json_str(name),
        division_specs_json(regiments),
        division_specs_json(support),
        json_array(blockers),
        json_array(&[
            "line_battalion and special_forces units must be written under regiments".to_string(),
            "support_company units must be written under support".to_string(),
            "special_or_unknown units require user confirmation before writing".to_string(),
        ])
    )
}

fn division_specs_json(specs: &[DivisionUnitSpec]) -> String {
    let mut out = String::from("[");
    for (idx, spec) in specs.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!(
            "{{\"sub_unit\": {}, \"count\": {}, \"class\": {}}}",
            json_str(&spec.sub_unit),
            spec.count,
            json_str(&spec.class)
        ));
    }
    out.push(']');
    out
}

fn division_specs_from_plan_json(plan: &str, key: &str) -> Vec<DivisionUnitSpec> {
    let Some(start) = plan.find(&format!("\"{key}\"")) else {
        return Vec::new();
    };
    let rest = &plan[start..];
    let Some(array_start) = rest.find('[') else {
        return Vec::new();
    };
    let Some(array_end) = rest[array_start + 1..].find(']') else {
        return Vec::new();
    };
    let slice = &rest[array_start + 1..array_start + 1 + array_end];
    let mut out = Vec::new();
    for object in slice.split("},") {
        let sub_unit = json_string_field(object, "sub_unit").unwrap_or_default();
        let class = json_string_field(object, "class").unwrap_or_default();
        let count = unit_json_i64_field(object, "count").unwrap_or(0);
        if !sub_unit.is_empty() && count > 0 {
            out.push(DivisionUnitSpec {
                sub_unit,
                count,
                class,
            });
        }
    }
    out
}

fn unit_json_i64_field(text: &str, key: &str) -> Option<i64> {
    let pattern = format!("\"{key}\":");
    let idx = text.find(&pattern)?;
    let rest = text[idx + pattern.len()..].trim_start();
    let digits = rest
        .chars()
        .take_while(|ch| ch.is_ascii_digit() || *ch == '-')
        .collect::<String>();
    digits.parse::<i64>().ok()
}

fn render_division_template_code(
    name: &str,
    regiments: &[DivisionUnitSpec],
    support: &[DivisionUnitSpec],
) -> String {
    let mut out = String::new();
    out.push_str("division_template = {\n");
    out.push_str(&format!("\tname = \"{name}\"\n"));
    out.push_str("\tregiments = {\n");
    let mut slot = 0i64;
    for spec in regiments {
        for _ in 0..spec.count {
            let x = slot % 5;
            let y = slot / 5;
            out.push_str(&format!(
                "\t\t{} = {{ x = {} y = {} }}\n",
                spec.sub_unit, x, y
            ));
            slot += 1;
        }
    }
    out.push_str("\t}\n");
    if !support.is_empty() {
        out.push_str("\tsupport = {\n");
        let mut support_slot = 0i64;
        for spec in support {
            for _ in 0..spec.count {
                out.push_str(&format!(
                    "\t\t{} = {{ x = {} y = 0 }}\n",
                    spec.sub_unit, support_slot
                ));
                support_slot += 1;
            }
        }
        out.push_str("\t}\n");
    }
    out.push_str("}\n");
    out
}

fn classify_oob_kind_from_taxonomy(entries: &[UnitTaxonomyEntry], text: &str) -> String {
    let matches = classify_unit_intent_matches(entries, text);
    if matches.iter().any(|entry| entry.class == "air_wing") {
        "air".to_string()
    } else if matches.iter().any(|entry| entry.class == "naval_ship") {
        "naval".to_string()
    } else if matches.iter().any(|entry| {
        matches!(
            entry.class.as_str(),
            "line_battalion" | "support_company" | "special_forces"
        )
    }) {
        "land".to_string()
    } else {
        classify_oob_kind_from_text(text)
    }
}

fn classify_oob_kind_from_text(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    if [
        "战斗机",
        "轰炸机",
        "联队",
        "air wing",
        "fighter",
        "bomber",
        "cas",
    ]
    .iter()
    .any(|marker| lower.contains(&marker.to_ascii_lowercase()))
    {
        "air".to_string()
    } else if [
        "舰队",
        "驱逐舰",
        "巡洋舰",
        "战列舰",
        "潜艇",
        "fleet",
        "destroyer",
        "submarine",
    ]
    .iter()
    .any(|marker| lower.contains(&marker.to_ascii_lowercase()))
    {
        "naval".to_string()
    } else if ["师", "步兵", "炮兵", "坦克", "division", "infantry"]
        .iter()
        .any(|marker| lower.contains(&marker.to_ascii_lowercase()))
    {
        "land".to_string()
    } else {
        "unknown".to_string()
    }
}

fn oob_template_next_commands(kind: &str) -> Vec<String> {
    match kind {
        "land" => vec![
            "hoi4skill division-template-plan --taxonomy <unit_taxonomy.json> --text <request> --require-passed".to_string(),
            "hoi4skill oob-relocation-plan --game-root <HOI4 root> --mod-root <mod> --text <request> --require-passed".to_string(),
        ],
        "air" => vec![
            "hoi4skill air-oob-plan --game-root <HOI4 root> --tag <TAG> --equipment <indexed_air_equipment> --province <province_id> --require-passed".to_string(),
        ],
        "naval" => vec![
            "hoi4skill naval-oob-plan --game-root <HOI4 root> --tag <TAG> --equipment <indexed_ship_equipment> --province <province_id> --require-passed".to_string(),
        ],
        _ => vec!["hoi4skill unit-intent-classify --taxonomy <unit_taxonomy.json> --text <request>".to_string()],
    }
}

fn cmd_typed_oob_plan(args: &[String], kind: &str) -> Result<(), String> {
    let map = parse_args(args);
    let game_root = normalize_path(&require_value(&map, "game-root")?)?;
    let index = history_index_from_game_and_mods(&map, &game_root)?;
    let tag = require_value(&map, "tag")?;
    let text = value(&map, "text").unwrap_or("");
    let equipment = require_value(&map, "equipment")?;
    let province = option_i64(&map, "province")
        .or_else(|| option_i64(&map, "province-id"))
        .ok_or_else(|| "missing --province".to_string())?;
    let amount = option_i64(&map, "amount")
        .or_else(|| option_i64(&map, "size"))
        .or_else(|| typed_oob_amount_from_text(text, kind))
        .unwrap_or(1);
    let mut blockers = Vec::new();
    validate_typed_oob_country_tag(&index, &tag, &mut blockers);
    if !index.equipment_types.contains(&equipment) {
        blockers.push(format!("equipment `{equipment}` is not indexed"));
    }
    if !index.province_ids.contains(&province) {
        blockers.push(format!("province `{province}` is not indexed"));
    }
    let required_building = if kind == "air" {
        "air_base"
    } else {
        "naval_base"
    };
    if !index.buildings.contains(required_building) {
        blockers.push(format!(
            "building `{required_building}` is not indexed; cannot verify {kind} base placement"
        ));
    }
    if amount <= 0 {
        blockers.push("amount must be positive".to_string());
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"kind\": {},\n  \"tag\": {},\n  \"text\": {},\n  \"equipment\": {},\n  \"province\": {},\n  \"amount\": {},\n  \"required_building\": {},\n  \"operations\": {},\n  \"blockers\": {},\n  \"rules\": {}\n}}\n",
        json_str("hoi4skill.typed_oob_plan.v1"),
        json_bool(ok),
        json_str(if ok { "typed_oob_plan_ready" } else { "blocked" }),
        json_str(kind),
        json_str(&tag),
        json_str(text),
        json_str(&equipment),
        province,
        amount,
        json_str(required_building),
        json_array(&typed_oob_operations(kind, &tag, &equipment, province, amount)),
        json_array(&blockers),
        json_array(&[
            "air OOB requests must use air-wing writers, not land division templates".to_string(),
            "naval OOB requests must use fleet/task-force writers, not land division templates".to_string(),
            "base province and equipment must be indexed before any writer runs".to_string(),
        ])
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

fn history_index_from_game_and_mods(map: &ArgMap, game_root: &Path) -> Result<GameIndex, String> {
    let mod_root = value(map, "mod-root").map(normalize_path).transpose()?;
    let mod_paths = dependency_mod_roots_for_optional_edited_mod(map, mod_root.as_deref(), true)?;
    build_game_index_with_mod_paths(game_root, &mod_paths)
}

fn validate_typed_oob_country_tag(index: &GameIndex, tag: &str, blockers: &mut Vec<String>) {
    if tag.len() != 3 || !tag.chars().all(|ch| ch.is_ascii_uppercase()) {
        blockers.push(format!(
            "tag `{tag}` must be a 3-letter uppercase country tag"
        ));
    } else if !index.country_tags.contains(tag) {
        blockers.push(format!("tag `{tag}` is not indexed"));
    }
}

fn typed_oob_amount_from_text(text: &str, kind: &str) -> Option<i64> {
    if kind == "air" {
        first_number_before_unit_marker(text, "架")
            .or_else(|| first_number_before_unit_marker(text, "联队"))
    } else {
        first_number_before_unit_marker(text, "艘")
            .or_else(|| first_number_before_unit_marker(text, "舰队"))
    }
}

fn typed_oob_operations(
    kind: &str,
    tag: &str,
    equipment: &str,
    province: i64,
    amount: i64,
) -> Vec<String> {
    if kind == "air" {
        vec![format!(
            "plan air wing for {tag}: equipment={equipment}, province={province}, amount={amount}"
        )]
    } else {
        vec![format!(
            "plan naval fleet for {tag}: ship_equipment={equipment}, base_province={province}, amount={amount}"
        )]
    }
}

fn render_unit_taxonomy_json(
    game_root: &Path,
    mod_root: Option<&Path>,
    entries: &[UnitTaxonomyEntry],
    blockers: &[String],
) -> String {
    let ok = blockers.is_empty();
    format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"game_root\": {},\n  \"mod_root\": {},\n  \"unit_count\": {},\n  \"units\": {},\n  \"blockers\": {},\n  \"rules\": {}\n}}\n",
        json_str("hoi4skill.unit_taxonomy.v1"),
        json_bool(ok),
        json_str(if ok { "unit_taxonomy_ready" } else { "needs_user_confirmation" }),
        json_str(&game_root.display().to_string()),
        json_optional_str(mod_root.map(|path| path.display().to_string()).as_deref()),
        entries.len(),
        unit_entries_json(entries),
        json_array(blockers),
        json_array(&[
            "sub_units must come from indexed common/units evidence".to_string(),
            "localised parent-mod unit names are aliases, not hard-coded CLI vocabulary".to_string(),
            "special_or_unknown units require user confirmation before OOB writing".to_string(),
        ])
    )
}

fn render_unit_intent_json(
    taxonomy_path: &Path,
    text: &str,
    matches: &[UnitTaxonomyEntry],
    blockers: &[String],
) -> String {
    let ok = blockers.is_empty();
    format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"taxonomy\": {},\n  \"text\": {},\n  \"matches\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.unit_intent_classify.v1"),
        json_bool(ok),
        json_str(if ok { "unit_intent_classified" } else { "needs_user_confirmation" }),
        json_str(&taxonomy_path.display().to_string()),
        json_str(text),
        unit_entries_json(matches),
        json_array(blockers),
        json_str("AI may choose from classified indexed units only; ambiguous or unknown unit classes must be resolved by the user")
    )
}

fn unit_entries_json(entries: &[UnitTaxonomyEntry]) -> String {
    let mut out = String::from("[");
    for (idx, entry) in entries.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!(
            "{{\"id\": {}, \"class\": {}, \"aliases\": {}, \"source_file\": {}, \"source_kind\": {}, \"evidence\": {}}}",
            json_str(&entry.id),
            json_str(&entry.class),
            json_array(&entry.aliases),
            json_str(&entry.source_file),
            json_str(&entry.source_kind),
            json_array(&entry.evidence),
        ));
    }
    out.push(']');
    out
}

fn parse_unit_taxonomy_entries(text: &str) -> Vec<UnitTaxonomyEntry> {
    let Some(units_start) = text.find("\"units\"") else {
        return Vec::new();
    };
    let Some(blockers_start) = text[units_start..]
        .find("\"blockers\"")
        .map(|idx| units_start + idx)
    else {
        return Vec::new();
    };
    let slice = &text[units_start..blockers_start];
    let mut entries = Vec::new();
    let mut offset = 0usize;
    while let Some(id_idx) = slice[offset..].find("\"id\":") {
        let start = offset + id_idx;
        let end = slice[start..]
            .find("}")
            .map(|idx| start + idx)
            .unwrap_or(slice.len());
        let object = &slice[start..end];
        let id = json_string_field(object, "id").unwrap_or_default();
        let class =
            json_string_field(object, "class").unwrap_or_else(|| "special_or_unknown".to_string());
        let aliases = parse_unit_json_string_array(object, "aliases");
        let source_file = json_string_field(object, "source_file").unwrap_or_default();
        let source_kind = json_string_field(object, "source_kind").unwrap_or_default();
        let evidence = parse_unit_json_string_array(object, "evidence");
        if !id.is_empty() {
            entries.push(UnitTaxonomyEntry {
                id,
                class,
                aliases,
                source_file,
                source_kind,
                evidence,
            });
        }
        offset = end + 1;
    }
    entries
}

fn parse_unit_json_string_array(text: &str, key: &str) -> Vec<String> {
    let pattern = format!("\"{key}\":");
    let Some(idx) = text.find(&pattern) else {
        return Vec::new();
    };
    let rest = &text[idx + pattern.len()..];
    let Some(start) = rest.find('[') else {
        return Vec::new();
    };
    let Some(end) = rest[start + 1..].find(']') else {
        return Vec::new();
    };
    rest[start + 1..start + 1 + end]
        .split(',')
        .filter_map(|part| {
            let value = part.trim();
            value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .map(json_unescape_minimal)
        })
        .collect()
}

fn json_unescape_minimal(value: &str) -> String {
    value
        .replace("\\\"", "\"")
        .replace("\\\\", "\\")
        .replace("\\n", "\n")
}
