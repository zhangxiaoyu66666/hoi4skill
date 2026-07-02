//! P12 country history, OOB, technology, and equipment planning gates.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_oob_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = value(&map, "text").unwrap_or("");
    let tag = require_value(&map, "tag")?;
    let index = optional_history_index(&map)?;
    let oob_id = value(&map, "id")
        .map(str::to_string)
        .unwrap_or_else(|| format!("{tag}_generated_oob"));
    let sub_units = repeated_values(&map, "sub-unit")
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let equipment = repeated_values(&map, "equipment")
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut blockers = Vec::new();
    let mut questions = Vec::new();
    if let Some(index) = &index {
        validate_country_tag(index, &tag, &mut blockers);
        for sub_unit in &sub_units {
            if !index.sub_units.contains(sub_unit) {
                blockers.push(format!("sub-unit `{sub_unit}` is not indexed"));
            }
        }
        for item in &equipment {
            if !index.equipment_types.contains(item) {
                blockers.push(format!("equipment type `{item}` is not indexed"));
            }
        }
    } else {
        questions.push("provide --game-root before final OOB writing so sub-units and equipment can be indexed".to_string());
    }
    if sub_units.is_empty() && text.contains("部队") {
        questions
            .push("which indexed sub-unit or division template should the OOB use?".to_string());
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"tag\": {},\n  \"text\": {},\n  \"oob_id\": {},\n  \"sub_units\": {},\n  \"equipment\": {},\n  \"planned_files\": {},\n  \"blockers\": {},\n  \"questions\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.oob_plan.v1"),
        json_bool(ok),
        json_str(if ok { "oob_plan_ready" } else { "blocked" }),
        json_str(&tag),
        json_str(text),
        json_str(&oob_id),
        json_array(&sub_units),
        json_array(&equipment),
        json_array(&[
            format!("history/units/{oob_id}.txt"),
            format!("history/countries/<TAG>.txt load_oob = \"{oob_id}\""),
        ]),
        json_array(&blockers),
        json_array(&questions),
        json_str("OOB plans may reference only indexed sub-units, equipment, and explicit load_oob IDs; do not guess division template syntax")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_oob_relocation_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = history_plan_input_text(&map)?;
    let game_root = normalize_path(&require_value(&map, "game-root")?)?;
    let mod_root = value(&map, "mod-root")
        .map(normalize_path)
        .transpose()?
        .ok_or_else(|| "missing --mod-root".to_string())?;
    let target_root = resolve_mod_root(&mod_root)?.root;
    let dependency_roots =
        dependency_mod_roots_for_optional_edited_mod(&map, Some(&target_root), true)?;
    let index = build_game_index_with_mod_paths(&game_root, &dependency_roots)?;
    let mut roots = vec![target_root.clone()];
    roots.extend(dependency_roots.iter().cloned());
    roots.push(game_root.clone());

    let tag = value(&map, "tag")
        .map(str::to_string)
        .or_else(|| infer_oob_relocation_tag(&text, &index))
        .unwrap_or_default();
    let oob_id = value(&map, "oob")
        .or_else(|| value(&map, "id"))
        .map(str::to_string)
        .unwrap_or_default();
    let target_state_id = option_i64(&map, "target-state-id")
        .or_else(|| option_i64(&map, "state-id"))
        .or_else(|| resolve_oob_state_id_by_text(&text, &index));
    let source_state_id = option_i64(&map, "source-state-id");
    let target_state = target_state_id.and_then(|id| oob_state_provinces_from_roots(&roots, id));
    let source_state = source_state_id.and_then(|id| oob_state_provinces_from_roots(&roots, id));
    let division_name = value(&map, "division-name")
        .map(str::to_string)
        .or_else(|| infer_oob_division_name(&text))
        .unwrap_or_else(|| format!("{tag}_generated_division"));
    let division_count = option_i64(&map, "division-count")
        .or_else(|| infer_oob_division_count(&text))
        .unwrap_or(1);
    let regiments = oob_regiment_plan(&map, &text, &index);
    let oob_source = if oob_id.is_empty() {
        None
    } else {
        find_oob_source_file(&roots, &oob_id)?
    };
    let target_oob_file = target_root
        .join("history")
        .join("units")
        .join(if oob_id.is_empty() {
            "<verified_oob>.txt".to_string()
        } else {
            format!("{oob_id}.txt")
        });

    let mut blockers = Vec::new();
    let mut questions = Vec::new();
    if tag.is_empty() {
        blockers.push("target country tag is not inferred; provide --tag".to_string());
    } else {
        validate_country_tag(&index, &tag, &mut blockers);
    }
    if oob_id.is_empty() {
        blockers.push(
            "OOB id is not inferred; provide --oob or --id from indexed local evidence".to_string(),
        );
    }
    if oob_source.is_none() {
        blockers.push(format!(
            "OOB `{oob_id}` was not found in target, dependency, or game history/units"
        ));
    }
    if target_state_id.is_none() {
        blockers.push(
            "target state is not verified; provide --target-state-id or indexed state localisation"
                .to_string(),
        );
    }
    if target_state
        .as_ref()
        .is_none_or(|state| state.provinces.is_empty())
    {
        blockers
            .push("target state has no verified province IDs for OOB location rewrite".to_string());
    }
    if division_count <= 0 {
        blockers.push("division-count must be positive".to_string());
    }
    for regiment in &regiments {
        if !index.sub_units.contains(&regiment.sub_unit) {
            blockers.push(format!(
                "sub-unit `{}` is not indexed; cannot write division_template regiments",
                regiment.sub_unit
            ));
        }
        if regiment.count <= 0 {
            blockers.push(format!(
                "sub-unit `{}` count must be positive",
                regiment.sub_unit
            ));
        }
    }
    if regiments.is_empty() {
        questions.push(
            "Which indexed battalions should the division template use? Provide --regiment <indexed_sub_unit>=<count>."
                .to_string(),
        );
    }

    let ok = blockers.is_empty();
    let report = oob_relocation_plan_json(OobRelocationPlanView {
        ok,
        tag: &tag,
        text: &text,
        target_root: &target_root,
        oob_id: &oob_id,
        oob_source: oob_source.as_ref(),
        target_oob_file: &target_oob_file,
        target_state: target_state.as_ref(),
        source_state: source_state.as_ref(),
        division_name: &division_name,
        division_count,
        regiments: &regiments,
        blockers: &blockers,
        questions: &questions,
    });
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_oob_relocation_apply(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let plan = read_utf8_lossy(&input)?;
    let mut blockers = Vec::new();
    if !map.flags.contains("execute") {
        blockers.push("oob-relocation-apply requires --execute".to_string());
    }
    if !map.flags.contains("final-check") {
        blockers.push("oob-relocation-apply requires --final-check".to_string());
    }
    if !plan.contains("\"schema\": \"hoi4skill.oob_relocation_plan.v1\"") {
        blockers.push("input is not an oob-relocation-plan report".to_string());
    }
    if !plan.contains("\"ok\": true") {
        blockers.push("input plan is not ok; fix blockers before apply".to_string());
    }
    let target_root = json_string_field(&plan, "target_root")
        .map(|path| normalize_path(&path))
        .transpose()?;
    let source_file = json_string_field(&plan, "source_file_abs")
        .map(|path| normalize_path(&path))
        .transpose()?;
    let target_file = json_string_field(&plan, "target_file_abs")
        .map(|path| normalize_path(&path))
        .transpose()?;
    let target_provinces = oob_json_i64_array_field(&plan, "target_provinces");
    let source_provinces = oob_json_i64_array_field(&plan, "source_provinces");
    let division_name = json_string_field(&plan, "division_name").unwrap_or_default();
    let division_count = oob_json_i64_field(&plan, "division_count").unwrap_or(0);
    let regiments = oob_regiments_from_plan_json(&plan);

    if target_root.is_none() {
        blockers.push("input plan is missing target_root".to_string());
    }
    if source_file.is_none() {
        blockers.push("input plan is missing source_file_abs".to_string());
    }
    if target_file.is_none() {
        blockers.push("input plan is missing target_file_abs".to_string());
    }
    if target_provinces.is_empty() {
        blockers.push("input plan is missing target_provinces".to_string());
    }
    if division_name.is_empty() {
        blockers.push("input plan is missing division_name".to_string());
    }
    if division_count <= 0 {
        blockers.push("input plan division_count must be positive".to_string());
    }
    if regiments.is_empty() {
        blockers.push("input plan is missing regiment definitions".to_string());
    }

    let mut changed_files = Vec::new();
    let mut backup_file = None;
    if blockers.is_empty() {
        let target_root = target_root.as_ref().unwrap();
        let source_file = source_file.as_ref().unwrap();
        let target_file = target_file.as_ref().unwrap();
        if !target_file.starts_with(target_root) {
            blockers.push("target_file_abs is outside target_root".to_string());
        } else {
            match apply_oob_relocation_file(
                source_file,
                target_file,
                target_root,
                &target_provinces,
                &source_provinces,
                &division_name,
                division_count,
                &regiments,
            ) {
                Ok(report) => {
                    backup_file = report.backup_file;
                    changed_files = report.changed_files;
                }
                Err(err) => blockers.push(err),
            }
        }
    }

    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"input\": {},\n  \"changed_files\": {},\n  \"backup_file\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.oob_relocation_apply.v1"),
        json_bool(ok),
        json_str(if ok { "oob_relocation_applied" } else { "blocked" }),
        json_str(&input.display().to_string()),
        json_array(&changed_files),
        json_optional_str(backup_file.as_deref()),
        json_array(&blockers),
        json_str("OOB rewrite is performed by Rust after province and sub-unit validation; AI must not directly edit history/units")
    );
    write_or_print(&json, value(&map, "output"))?;
    if (map.flags.contains("require-passed") || !ok) && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_tech_equipment_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let index = history_index(&map)?;
    let tag = require_value(&map, "tag")?;
    let text = value(&map, "text").unwrap_or("");
    let technologies = repeated_values(&map, "technology")
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let equipment = repeated_values(&map, "equipment")
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut blockers = Vec::new();
    validate_country_tag(&index, &tag, &mut blockers);
    for tech in &technologies {
        if !index.technologies.contains(tech) {
            blockers.push(format!("technology `{tech}` is not indexed"));
        }
    }
    for item in &equipment {
        if !index.equipment_types.contains(item) {
            blockers.push(format!("equipment type `{item}` is not indexed"));
        }
    }
    if technologies.is_empty() && equipment.is_empty() {
        blockers.push("missing --technology or --equipment".to_string());
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"tag\": {},\n  \"text\": {},\n  \"technologies\": {},\n  \"equipment\": {},\n  \"operations\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.tech_equipment_plan.v1"),
        json_bool(ok),
        json_str(if ok { "tech_equipment_plan_ready" } else { "blocked" }),
        json_str(&tag),
        json_str(text),
        json_array(&technologies),
        json_array(&equipment),
        json_array(&tech_equipment_operations(&technologies, &equipment)),
        json_array(&blockers),
        json_str("technology and equipment stockpile plans must use indexed technology and equipment IDs before country history or focus effects are assembled")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_history_country_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let index = history_index(&map)?;
    let tag = require_value(&map, "tag")?;
    let capital = option_i64(&map, "capital")
        .or_else(|| option_i64(&map, "capital-province-id"))
        .ok_or_else(|| "missing --capital".to_string())?;
    let ruling_party = value(&map, "ruling-party").map(str::to_string);
    let oob = value(&map, "oob").map(str::to_string);
    let mut blockers = Vec::new();
    validate_country_tag(&index, &tag, &mut blockers);
    if !index.province_ids.contains(&capital) {
        blockers.push(format!(
            "capital `{capital}` is not indexed as a province id"
        ));
    }
    if index.state_ids.contains(&capital) {
        blockers.push(format!(
            "capital `{capital}` also matches a state id; country history capital expects a province id"
        ));
    }
    if let Some(party) = &ruling_party {
        if !index.ideologies.contains(party) {
            blockers.push(format!("ruling party ideology `{party}` is not indexed"));
        }
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"tag\": {},\n  \"capital_province_id\": {},\n  \"ruling_party\": {},\n  \"oob\": {},\n  \"planned_file\": {},\n  \"operations\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.history_country_plan.v1"),
        json_bool(ok),
        json_str(if ok { "history_country_plan_ready" } else { "blocked" }),
        json_str(&tag),
        capital,
        json_optional_str(ruling_party.as_deref()),
        json_optional_str(oob.as_deref()),
        json_str(&format!("history/countries/{tag}.txt or changed-only patch")),
        json_array(&history_country_operations(
            capital,
            ruling_party.as_deref(),
            oob.as_deref(),
        )),
        json_array(&blockers),
        json_str("country history plans verify capital as a province id and avoid full-file copies unless explicitly authorized")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

fn optional_history_index(map: &ArgMap) -> Result<Option<GameIndex>, String> {
    value(map, "game-root")
        .map(normalize_path)
        .transpose()?
        .map(|root| {
            let mod_root = value(map, "mod-root").map(normalize_path).transpose()?;
            let mod_paths =
                dependency_mod_roots_for_optional_edited_mod(map, mod_root.as_deref(), true)?;
            build_game_index_with_mod_paths(&root, &mod_paths)
        })
        .transpose()
}

#[derive(Clone)]
struct OobStateProvinceSet {
    id: i64,
    file: String,
    provinces: Vec<i64>,
}

#[derive(Clone)]
struct OobRegimentSpec {
    sub_unit: String,
    count: i64,
}

#[derive(Clone)]
struct OobSourceFile {
    file: PathBuf,
    rel: String,
    source: String,
}

struct OobRelocationPlanView<'a> {
    ok: bool,
    tag: &'a str,
    text: &'a str,
    target_root: &'a Path,
    oob_id: &'a str,
    oob_source: Option<&'a OobSourceFile>,
    target_oob_file: &'a Path,
    target_state: Option<&'a OobStateProvinceSet>,
    source_state: Option<&'a OobStateProvinceSet>,
    division_name: &'a str,
    division_count: i64,
    regiments: &'a [OobRegimentSpec],
    blockers: &'a [String],
    questions: &'a [String],
}

struct OobApplyReport {
    changed_files: Vec<String>,
    backup_file: Option<String>,
}

fn oob_relocation_plan_json(view: OobRelocationPlanView<'_>) -> String {
    let source_file_abs = view
        .oob_source
        .map(|source| source.file.display().to_string());
    format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"tag\": {},\n  \"text\": {},\n  \"target_root\": {},\n  \"oob_id\": {},\n  \"source_file\": {},\n  \"source_file_abs\": {},\n  \"source_kind\": {},\n  \"target_file_abs\": {},\n  \"target_state_id\": {},\n  \"target_state_file\": {},\n  \"target_provinces\": {},\n  \"source_state_id\": {},\n  \"source_provinces\": {},\n  \"division_name\": {},\n  \"division_count\": {},\n  \"regiments\": {},\n  \"operations\": {},\n  \"blockers\": {},\n  \"questions\": {},\n  \"rules\": {}\n}}\n",
        json_str("hoi4skill.oob_relocation_plan.v1"),
        json_bool(view.ok),
        json_str(if view.ok { "oob_relocation_ready" } else { "blocked" }),
        json_str(view.tag),
        json_str(view.text),
        json_str(&view.target_root.display().to_string()),
        json_str(view.oob_id),
        json_optional_str(view.oob_source.map(|source| source.rel.as_str())),
        json_optional_str(source_file_abs.as_deref()),
        json_optional_str(view.oob_source.map(|source| source.source.as_str())),
        json_str(&view.target_oob_file.display().to_string()),
        json_optional_i64(view.target_state.map(|state| state.id)),
        json_optional_str(view.target_state.map(|state| state.file.as_str())),
        json_i64_array(
            &view
                .target_state
                .map(|state| state.provinces.clone())
                .unwrap_or_default()
        ),
        json_optional_i64(view.source_state.map(|state| state.id)),
        json_i64_array(
            &view
                .source_state
                .map(|state| state.provinces.clone())
                .unwrap_or_default()
        ),
        json_str(view.division_name),
        view.division_count,
        oob_regiments_json(view.regiments),
        json_array(&[
            "rewrite existing division location = <province> values to target province IDs".to_string(),
            "append or preserve a verified division_template for the requested structure".to_string(),
            "append requested division instances using the verified template and target provinces".to_string(),
        ]),
        json_array(view.blockers),
        json_array(view.questions),
        json_array(&[
            "target_provinces must come from the verified target state".to_string(),
            "source_provinces narrows replacement; if omitted, every location in the OOB file is rewritten".to_string(),
            "regiment sub-units must be indexed before division_template code can be written".to_string(),
            "apply writes only under the target mod history/units path and saves a backup for existing target files".to_string(),
        ])
    )
}

fn oob_regiments_json(regiments: &[OobRegimentSpec]) -> String {
    format!(
        "[{}]",
        regiments
            .iter()
            .map(|regiment| {
                format!(
                    "{{\"sub_unit\": {}, \"count\": {}}}",
                    json_str(&regiment.sub_unit),
                    regiment.count
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn infer_oob_relocation_tag(text: &str, index: &GameIndex) -> Option<String> {
    if let Some(tag) = first_tag(text) {
        return Some(tag);
    }
    let normalized_text = normalize_history_name_for_oob(text);
    let mut candidates = BTreeSet::new();
    for (name, tags) in &index.country_name_tags {
        let normalized_name = normalize_history_name_for_oob(name);
        if normalized_name.is_empty() || !normalized_text.contains(&normalized_name) {
            continue;
        }
        for tag in tags {
            candidates.insert(tag.clone());
        }
    }
    if candidates.len() == 1 {
        candidates.into_iter().next()
    } else {
        None
    }
}

fn resolve_oob_state_id_by_text(text: &str, index: &GameIndex) -> Option<i64> {
    if let Some(id) = first_labeled_number(text, "state") {
        return Some(id);
    }
    let normalized_text = normalize_history_name_for_oob(text);
    for (key, id) in &index.state_names {
        if normalized_text.contains(&normalize_history_name_for_oob(key)) {
            return Some(*id);
        }
        if let Some(localized) = index.localisation_entries.get(key) {
            let localized = normalize_history_name_for_oob(localized);
            if !localized.is_empty() && normalized_text.contains(&localized) {
                return Some(*id);
            }
        }
    }
    None
}

fn oob_state_provinces_from_roots(roots: &[PathBuf], state_id: i64) -> Option<OobStateProvinceSet> {
    for root in roots {
        let states = scan_history_state_styles(root).ok()?;
        if let Some(state) = states.into_iter().find(|state| state.id == Some(state_id)) {
            return Some(OobStateProvinceSet {
                id: state_id,
                file: state.file,
                provinces: state.province_sample,
            });
        }
    }
    None
}

fn infer_oob_division_name(text: &str) -> Option<String> {
    for (left, right) in [('“', '”'), ('"', '"'), ('「', '」'), ('《', '》')] {
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
    if text.contains("红军装甲师") {
        Some("红军装甲师".to_string())
    } else {
        infer_oob_formation_name(text)
    }
}

fn infer_oob_formation_name(text: &str) -> Option<String> {
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
        if value.is_empty()
            || !value.contains('师')
            || value.contains("个师")
            || value.contains("几个师")
            || value.contains("共")
        {
            continue;
        }
        let cleaned = value
            .trim_start_matches("创建")
            .trim_start_matches("建立")
            .trim_start_matches("新增")
            .trim_start_matches("组建")
            .trim_start_matches("部署")
            .trim();
        if cleaned.chars().count() >= 2 && cleaned.chars().count() <= 32 {
            return Some(cleaned.to_string());
        }
    }
    None
}

fn infer_oob_division_count(text: &str) -> Option<i64> {
    if text.contains("两个师") || text.contains("两师") || text.contains("共两个") {
        Some(2)
    } else if text.contains("一个师") || text.contains("一师") {
        Some(1)
    } else {
        first_number_before_marker(text, "个师").or_else(|| first_number_before_marker(text, "师"))
    }
}

fn oob_regiment_plan(map: &ArgMap, text: &str, index: &GameIndex) -> Vec<OobRegimentSpec> {
    let explicit = repeated_values(map, "regiment")
        .into_iter()
        .filter_map(parse_oob_regiment_arg)
        .collect::<Vec<_>>();
    if !explicit.is_empty() {
        return explicit;
    }
    dynamic_oob_regiment_plan(index, text)
}

fn dynamic_oob_regiment_plan(index: &GameIndex, text: &str) -> Vec<OobRegimentSpec> {
    let mut inferred = Vec::new();
    for sub_unit in &index.sub_units {
        for marker in dynamic_oob_sub_unit_markers(index, sub_unit) {
            if let Some(count) = first_number_before_marker(text, &marker) {
                push_or_add_oob_regiment(&mut inferred, sub_unit.clone(), count);
                break;
            }
        }
    }
    inferred
}

fn dynamic_oob_sub_unit_markers(index: &GameIndex, sub_unit: &str) -> Vec<String> {
    let mut markers = Vec::new();
    push_unique_oob_marker(&mut markers, sub_unit);
    push_unique_oob_marker(&mut markers, &sub_unit.replace('_', " "));
    if let Some(localized) = index.localisation_entries.get(sub_unit) {
        let clean = clean_localisation_icon_label(localized);
        push_unique_oob_marker(&mut markers, localized);
        push_unique_oob_marker(&mut markers, &clean);
    }
    markers
}

fn push_unique_oob_marker(markers: &mut Vec<String>, value: &str) {
    let value = value.trim();
    if value.chars().count() < 2 {
        return;
    }
    if !markers.iter().any(|existing| existing == value) {
        markers.push(value.to_string());
    }
}

fn parse_oob_regiment_arg(value: &str) -> Option<OobRegimentSpec> {
    let (sub_unit, count) = value.split_once('=')?;
    let count = count.trim().parse::<i64>().ok()?;
    Some(OobRegimentSpec {
        sub_unit: sub_unit.trim().to_string(),
        count,
    })
}

fn push_or_add_oob_regiment(regiments: &mut Vec<OobRegimentSpec>, sub_unit: String, count: i64) {
    if let Some(existing) = regiments
        .iter_mut()
        .find(|regiment| regiment.sub_unit == sub_unit)
    {
        existing.count += count;
    } else {
        regiments.push(OobRegimentSpec { sub_unit, count });
    }
}

fn first_number_before_marker(text: &str, marker: &str) -> Option<i64> {
    let lower = text.to_ascii_lowercase();
    let marker = marker.to_ascii_lowercase();
    let mut offset = 0usize;
    while let Some(relative_idx) = lower[offset..].find(&marker) {
        let idx = offset + relative_idx;
        if let Some(count) = number_at_end_before_oob_marker(&text[..idx]) {
            return Some(count);
        }
        offset = idx + marker.len();
    }
    None
}

fn number_at_end_before_oob_marker(prefix: &str) -> Option<i64> {
    let trimmed = prefix.trim_end();
    let mut start = trimmed.len();
    for (idx, ch) in trimmed.char_indices().rev() {
        if ch.is_ascii_digit() || is_simple_chinese_number_char(ch) {
            start = idx;
        } else {
            break;
        }
    }
    if start == trimmed.len() {
        return None;
    }
    if trimmed[..start].chars().last() == Some('第') {
        return None;
    }
    let value = &trimmed[start..];
    value
        .parse::<i64>()
        .ok()
        .or_else(|| parse_simple_chinese_number(value))
}

fn is_simple_chinese_number_char(ch: char) -> bool {
    matches!(
        ch,
        '零' | '一' | '二' | '两' | '三' | '四' | '五' | '六' | '七' | '八' | '九' | '十'
    )
}

fn parse_simple_chinese_number(value: &str) -> Option<i64> {
    if value == "十" {
        return Some(10);
    }
    if let Some((left, right)) = value.split_once('十') {
        let tens = if left.is_empty() {
            1
        } else {
            chinese_digit(left.chars().next()?)?
        };
        let ones = if right.is_empty() {
            0
        } else {
            chinese_digit(right.chars().next()?)?
        };
        return Some(tens * 10 + ones);
    }
    if value.chars().count() == 1 {
        return chinese_digit(value.chars().next()?);
    }
    None
}

fn chinese_digit(ch: char) -> Option<i64> {
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

fn find_oob_source_file(roots: &[PathBuf], oob_id: &str) -> Result<Option<OobSourceFile>, String> {
    let expected = format!("{oob_id}.txt");
    for (idx, root) in roots.iter().enumerate() {
        for file in txt_files(root, "history/units")? {
            if file
                .file_name()
                .and_then(OsStr::to_str)
                .is_some_and(|name| name.eq_ignore_ascii_case(&expected))
            {
                return Ok(Some(OobSourceFile {
                    rel: rel_slash(root, &file),
                    file,
                    source: if idx == 0 {
                        "target_mod".to_string()
                    } else {
                        "dependency_or_game".to_string()
                    },
                }));
            }
        }
    }
    Ok(None)
}

fn apply_oob_relocation_file(
    source_file: &Path,
    target_file: &Path,
    target_root: &Path,
    target_provinces: &[i64],
    source_provinces: &[i64],
    division_name: &str,
    division_count: i64,
    regiments: &[OobRegimentSpec],
) -> Result<OobApplyReport, String> {
    let original = read_utf8_lossy(source_file)?;
    let mut rewritten = rewrite_oob_locations(&original, target_provinces, source_provinces)
        .ok_or_else(|| {
            "OOB file contains no location assignments eligible for rewrite".to_string()
        })?;
    if !oob_contains_division_template(&rewritten, division_name) {
        rewritten.push_str("\n");
        rewritten.push_str(&render_oob_division_template(division_name, regiments));
    }
    rewritten.push_str("\n");
    rewritten.push_str(&render_oob_divisions(
        division_name,
        division_count,
        target_provinces,
    ));
    if let Some(parent) = target_file.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    let mut backup_file = None;
    if target_file.exists() {
        let backup = target_root
            .join(".hoi4skill")
            .join("oob_relocation_backup")
            .join(
                target_file
                    .file_name()
                    .and_then(OsStr::to_str)
                    .unwrap_or("oob_backup.txt"),
            );
        if let Some(parent) = backup.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
        }
        fs::copy(target_file, &backup)
            .map_err(|e| format!("backup {}: {e}", target_file.display()))?;
        backup_file = Some(backup.display().to_string());
    }
    fs::write(target_file, rewritten)
        .map_err(|e| format!("write {}: {e}", target_file.display()))?;
    Ok(OobApplyReport {
        changed_files: vec![target_file.display().to_string()],
        backup_file,
    })
}

fn rewrite_oob_locations(
    text: &str,
    target_provinces: &[i64],
    source_provinces: &[i64],
) -> Option<String> {
    if target_provinces.is_empty() {
        return None;
    }
    let mut changed = false;
    let mut target_idx = 0usize;
    let mut out = String::new();
    for line in text.lines() {
        if let Some((prefix, old, suffix)) = split_location_assignment(line) {
            if source_provinces.is_empty() || source_provinces.contains(&old) {
                let new_location = target_provinces[target_idx % target_provinces.len()];
                target_idx += 1;
                out.push_str(prefix);
                out.push_str(&new_location.to_string());
                out.push_str(suffix);
                out.push('\n');
                changed = true;
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    changed.then_some(out)
}

fn split_location_assignment(line: &str) -> Option<(&str, i64, &str)> {
    let idx = line.find("location")?;
    let after_key = &line[idx + "location".len()..];
    let eq_rel = after_key.find('=')?;
    let value_start = idx + "location".len() + eq_rel + 1;
    let rest = &line[value_start..];
    let leading = rest.len() - rest.trim_start().len();
    let digits_start = value_start + leading;
    let digits = line[digits_start..]
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }
    let digits_end = digits_start + digits.len();
    let value = digits.parse::<i64>().ok()?;
    Some((&line[..digits_start], value, &line[digits_end..]))
}

fn oob_contains_division_template(text: &str, division_name: &str) -> bool {
    text.contains(&format!("name = \"{division_name}\""))
        || text.contains(&format!("division_template = \"{division_name}\""))
}

fn render_oob_division_template(division_name: &str, regiments: &[OobRegimentSpec]) -> String {
    let mut out = String::new();
    out.push_str("division_template = {\n");
    out.push_str(&format!("\tname = \"{division_name}\"\n"));
    out.push_str("\tregiments = {\n");
    let mut slot = 0i64;
    for regiment in regiments {
        for _ in 0..regiment.count {
            let x = slot % 5;
            let y = slot / 5;
            out.push_str(&format!(
                "\t\t{} = {{ x = {} y = {} }}\n",
                regiment.sub_unit, x, y
            ));
            slot += 1;
        }
    }
    out.push_str("\t}\n");
    out.push_str("}\n");
    out
}

fn render_oob_divisions(
    division_name: &str,
    division_count: i64,
    target_provinces: &[i64],
) -> String {
    let mut out = String::new();
    for idx in 0..division_count {
        let province = target_provinces[idx as usize % target_provinces.len()];
        out.push_str("division = {\n");
        out.push_str(&format!("\tname = \"{} {}\"\n", division_name, idx + 1));
        out.push_str(&format!("\tlocation = {province}\n"));
        out.push_str(&format!("\tdivision_template = \"{division_name}\"\n"));
        out.push_str("}\n");
    }
    out
}

fn oob_json_i64_field(text: &str, key: &str) -> Option<i64> {
    let pattern = format!("\"{key}\":");
    let idx = text.find(&pattern)?;
    let rest = text[idx + pattern.len()..].trim_start();
    let mut value = String::new();
    for ch in rest.chars() {
        if ch.is_ascii_digit() || (value.is_empty() && ch == '-') {
            value.push(ch);
        } else if !value.is_empty() {
            break;
        } else {
            return None;
        }
    }
    value.parse::<i64>().ok()
}

fn oob_json_i64_array_field(text: &str, key: &str) -> Vec<i64> {
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
        .filter_map(|value| value.trim().parse::<i64>().ok())
        .collect()
}

fn oob_regiments_from_plan_json(plan: &str) -> Vec<OobRegimentSpec> {
    let Some(start) = plan.find("\"regiments\"") else {
        return Vec::new();
    };
    let Some(end) = plan[start..].find("\"operations\"").map(|idx| start + idx) else {
        return Vec::new();
    };
    let slice = &plan[start..end];
    let sub_units = json_string_array_like_values(slice, "sub_unit");
    let counts = oob_json_i64_values(slice, "count");
    sub_units
        .into_iter()
        .zip(counts)
        .map(|(sub_unit, count)| OobRegimentSpec { sub_unit, count })
        .collect()
}

fn json_string_array_like_values(text: &str, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    let pattern = format!("\"{key}\":");
    let mut offset = 0usize;
    while let Some(idx) = text[offset..].find(&pattern) {
        let start = offset + idx + pattern.len();
        if let Some((value, consumed)) = parse_oob_json_string_after_colon(&text[start..]) {
            out.push(value);
            offset = start + consumed;
            continue;
        }
        offset = start;
    }
    out
}

fn parse_oob_json_string_after_colon(text: &str) -> Option<(String, usize)> {
    let text = text.trim_start();
    if !text.starts_with('"') {
        return None;
    }
    let mut escaped = false;
    let mut out = String::new();
    for (idx, ch) in text[1..].char_indices() {
        if escaped {
            out.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some((out, idx + 2));
        } else {
            out.push(ch);
        }
    }
    None
}

fn oob_json_i64_values(text: &str, key: &str) -> Vec<i64> {
    let mut out = Vec::new();
    let pattern = format!("\"{key}\":");
    let mut offset = 0usize;
    while let Some(idx) = text[offset..].find(&pattern) {
        let start = offset + idx + pattern.len();
        let rest = text[start..].trim_start();
        let digits = rest
            .chars()
            .take_while(|ch| ch.is_ascii_digit() || *ch == '-')
            .collect::<String>();
        if let Ok(value) = digits.parse::<i64>() {
            out.push(value);
        }
        offset = start + 1;
    }
    out
}

fn normalize_history_name_for_oob(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || ('\u{4e00}'..='\u{9fff}').contains(ch))
        .flat_map(char::to_lowercase)
        .collect()
}

fn history_index(map: &ArgMap) -> Result<GameIndex, String> {
    optional_history_index(map)?.ok_or_else(|| "missing --game-root".to_string())
}

fn validate_country_tag(index: &GameIndex, tag: &str, blockers: &mut Vec<String>) {
    if !index.country_tags.contains(tag) {
        blockers.push(format!("country tag `{tag}` is not indexed"));
    }
}

fn tech_equipment_operations(technologies: &[String], equipment: &[String]) -> Vec<String> {
    let mut operations = Vec::new();
    for tech in technologies {
        operations.push(format!("set_technology = {{ {tech} = 1 }}"));
    }
    for item in equipment {
        operations.push(format!(
            "add_equipment_to_stockpile = {{ type = {item} amount = <user_value> }}"
        ));
    }
    operations
}

fn history_country_operations(
    capital: i64,
    ruling_party: Option<&str>,
    oob: Option<&str>,
) -> Vec<String> {
    let mut operations = vec![format!("capital = {capital}")];
    if let Some(party) = ruling_party {
        operations.push(format!("set_politics.ruling_party = {party}"));
    }
    if let Some(oob) = oob {
        operations.push(format!("load_oob = \"{oob}\""));
    }
    operations
}
