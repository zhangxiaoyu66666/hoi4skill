//! P18 system-package gates.
//!
//! Complex HOI4 systems should be planned as connected packages instead of
//! isolated files. This module is deliberately conservative: it records package
//! boundaries, dependency edges, template evidence, and runtime checks before a
//! later writer is allowed to run.

#[allow(unused_imports)]
use crate::*;

#[derive(Clone, Copy)]
struct SystemPackSpec {
    id: &'static str,
    title: &'static str,
    common_dirs: &'static [&'static str],
    definition_roles: &'static [&'static str],
    reference_roles: &'static [&'static str],
    summary: &'static str,
    runtime_check: &'static str,
}

pub(crate) fn cmd_system_pack_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let pack = require_pack_spec(value(&map, "pack"))?;
    system_pack_plan(args, pack)
}

pub(crate) fn cmd_intelligence_system_pack_plan(args: &[String]) -> Result<(), String> {
    system_pack_plan(args, system_pack_spec("intelligence_operations")?)
}

pub(crate) fn cmd_ai_behavior_system_pack_plan(args: &[String]) -> Result<(), String> {
    system_pack_plan(args, system_pack_spec("ai_behavior")?)
}

pub(crate) fn cmd_technology_depth_system_pack_plan(args: &[String]) -> Result<(), String> {
    system_pack_plan(args, system_pack_spec("technology_depth")?)
}

pub(crate) fn cmd_occupation_resistance_system_pack_plan(args: &[String]) -> Result<(), String> {
    system_pack_plan(args, system_pack_spec("occupation_resistance")?)
}

pub(crate) fn cmd_system_pack_apply(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let plan = read_utf8_lossy(&input)?;
    let mut blockers = Vec::new();
    if !map.flags.contains("execute") {
        blockers.push("system-pack-apply requires --execute".to_string());
    }
    if !map.flags.contains("final-check") {
        blockers.push("system-pack-apply requires --final-check".to_string());
    }
    if !plan.contains("\"schema\": \"hoi4skill.system_pack_plan.v1\"") {
        blockers.push("input is not a system-pack-plan report".to_string());
    }
    if !plan.contains("\"ok\": true") {
        blockers.push("input plan is not ok; fix blockers before apply".to_string());
    }
    let pack_id = json_string_field(&plan, "pack").unwrap_or_default();
    let pack = system_pack_spec(&pack_id).ok();
    if pack.is_none() {
        blockers.push(format!(
            "input plan pack `{pack_id}` is not a supported P18 system pack"
        ));
    }
    let prefix = json_string_field(&plan, "prefix").unwrap_or_else(|| "system_pack".to_string());
    let target_root = json_string_field(&plan, "target_root")
        .map(|path| normalize_path(&path))
        .transpose()?;
    if target_root.is_none() {
        blockers.push("input plan is missing target_root".to_string());
    }
    let target_files = json_string_array_field(&plan, "target_files");
    if target_files.is_empty() {
        blockers.push("input plan is missing target_files".to_string());
    }

    let mut write_plan = Vec::new();
    if let (Some(pack), Some(target_root)) = (pack, target_root.as_ref()) {
        for relative in &target_files {
            match system_pack_target_path(target_root, relative) {
                Ok(path) => {
                    if path.exists() {
                        blockers.push(format!(
                            "transaction target already exists and will not be overwritten: {}",
                            path.display()
                        ));
                    }
                    write_plan.push((
                        relative.clone(),
                        path,
                        system_pack_skeleton(pack, &prefix, relative),
                    ));
                }
                Err(err) => blockers.push(err),
            }
        }
    }

    let mut changed_files = Vec::new();
    let mut rollback_blockers = Vec::new();
    if blockers.is_empty() {
        match write_system_pack_transaction(&write_plan) {
            Ok(changed) => changed_files = changed,
            Err((err, changed)) => {
                rollback_blockers.push(err);
                rollback_blockers.extend(rollback_system_pack_files(&changed));
                blockers.push("transaction write failed and rollback was attempted".to_string());
                changed_files = changed
                    .into_iter()
                    .map(|path| path.display().to_string())
                    .collect();
            }
        }
    }

    let ok = blockers.is_empty();
    let report = system_pack_apply_json(&input, ok, &changed_files, &blockers, &rollback_blockers);
    write_or_print(&report, value(&map, "output"))?;
    if (map.flags.contains("require-passed") || !blockers.is_empty()) && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

fn system_pack_plan(args: &[String], pack: SystemPackSpec) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let target_root = resolve_mod_root(&mod_root)?.root;
    let parent_roots = repeated_values(&map, "mod-path")
        .into_iter()
        .map(|path| resolve_mod_root(&normalize_path(path)?).map(|resolved| resolved.root))
        .collect::<Result<Vec<_>, String>>()?;
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    let prefix = value(&map, "prefix")
        .or_else(|| value(&map, "id"))
        .unwrap_or(pack.id);

    let mut blockers = Vec::new();
    let references = requested_references(&map);
    if !references.is_empty() && game_root.is_none() && parent_roots.is_empty() {
        blockers.push("reference validation needs --game-root or --mod-path evidence".to_string());
    }
    let source_roots = system_pack_source_roots(&target_root, &parent_roots, game_root.as_deref());
    let missing_refs = references
        .iter()
        .filter(|(_, id)| !symbol_text_exists(&source_roots, id).unwrap_or(false))
        .map(|(kind, id)| format!("{kind}:{id}"))
        .collect::<Vec<_>>();
    if !missing_refs.is_empty() {
        blockers.push(format!(
            "unindexed package references: {}",
            missing_refs.join(", ")
        ));
    }

    let template_evidence = pack
        .common_dirs
        .iter()
        .map(|dir| system_pack_template_evidence(dir, &source_roots))
        .collect::<Result<Vec<_>, String>>()?;
    let has_parent_or_target_template = template_evidence
        .iter()
        .any(|row| row.contains("\"role\": \"target\"") || row.contains("\"role\": \"parent\""));
    let template_mode = if has_parent_or_target_template {
        "reuse_parent_or_target_template"
    } else {
        "conservative_skeleton_only"
    };
    let target_files = pack
        .common_dirs
        .iter()
        .map(|dir| {
            format!(
                "common/{dir}/{}_{}.txt",
                sanitize_pack_file_stem(prefix),
                dir.replace('/', "_")
            )
        })
        .collect::<Vec<_>>();
    let dependency_edges = system_pack_dependency_edges(pack);
    let ok = blockers.is_empty();
    let json = system_pack_plan_json(SystemPackReport {
        ok,
        pack,
        target_root: &target_root,
        prefix,
        template_mode,
        target_files: &target_files,
        template_evidence: &template_evidence,
        dependency_edges: &dependency_edges,
        references: &references,
        blockers: &blockers,
    });
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

fn require_pack_spec(raw: Option<&str>) -> Result<SystemPackSpec, String> {
    let raw = raw.ok_or_else(|| "missing --pack".to_string())?;
    system_pack_spec(raw)
}

fn system_pack_spec(raw: &str) -> Result<SystemPackSpec, String> {
    match raw.replace('-', "_").as_str() {
        "intelligence" | "intelligence_operations" | "operations" => Ok(SystemPackSpec {
            id: "intelligence_operations",
            title: "intelligence and operations package",
            common_dirs: &[
                "intelligence_agencies",
                "intelligence_agency_upgrades",
                "operations",
                "operation_phases",
                "operation_tokens",
            ],
            definition_roles: &["agency", "upgrade", "operation", "phase", "token"],
            reference_roles: &["operation -> phase", "operation/phase -> token"],
            summary: "Creates or extends spy agencies, upgrades, operations, phases, and tokens as one connected gameplay system.",
            runtime_check: "Open the intelligence agency UI, start an operation, and compare error.log against the pre-change baseline.",
        }),
        "ai" | "ai_behavior" => Ok(SystemPackSpec {
            id: "ai_behavior",
            title: "AI behavior package",
            common_dirs: &["ai_strategy", "ai_strategy_plans", "ai_focuses", "ai_templates"],
            definition_roles: &["ai strategy", "strategy plan", "focus preference", "division template"],
            reference_roles: &["strategy plan -> ai strategy", "ai focuses -> focus ids", "ai templates -> units/equipment"],
            summary: "Connects AI route choice, focus preference, production, and template behavior instead of writing one isolated AI file.",
            runtime_check: "Run hands-off AI smoke, inspect selected focus/strategy behavior, and diff error.log from baseline.",
        }),
        "technology" | "technology_depth" | "tech_depth" => Ok(SystemPackSpec {
            id: "technology_depth",
            title: "technology depth package",
            common_dirs: &[
                "technologies",
                "technology_tags",
                "technology_sharing",
                "equipment_groups",
                "special_projects",
            ],
            definition_roles: &["technology", "technology tag", "sharing group", "equipment group", "special project"],
            reference_roles: &["special project -> technology/building/facility", "equipment group -> equipment", "technology sharing -> technology tag"],
            summary: "Keeps research tree, tags, sharing groups, equipment grouping, and special projects in a single dependency graph.",
            runtime_check: "Open research and special-project UI, verify icons/folders, then diff error.log from baseline.",
        }),
        "occupation" | "occupation_resistance" | "resistance" => Ok(SystemPackSpec {
            id: "occupation_resistance",
            title: "occupation and resistance package",
            common_dirs: &[
                "occupation_laws",
                "resistance_activity",
                "resistance_compliance_modifiers",
            ],
            definition_roles: &["occupation law", "resistance activity", "compliance modifier"],
            reference_roles: &["law -> modifiers", "activity -> resistance/compliance effects"],
            summary: "Bundles occupation laws, resistance activity, and compliance modifiers so balance values and scopes are reviewed together.",
            runtime_check: "Load an occupied state, switch occupation laws, inspect resistance/compliance changes, then diff error.log.",
        }),
        _ => Err(format!(
            "unknown --pack {raw}; expected intelligence_operations, ai_behavior, technology_depth, or occupation_resistance"
        )),
    }
}

fn requested_references(map: &ArgMap) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for key in [
        "focus",
        "technology",
        "equipment",
        "building",
        "facility",
        "unit",
        "modifier",
        "sprite",
    ] {
        for value in repeated_values(map, key) {
            out.push((key.to_string(), value.to_string()));
        }
    }
    out
}

fn system_pack_source_roots(
    target_root: &Path,
    parent_roots: &[PathBuf],
    game_root: Option<&Path>,
) -> Vec<(&'static str, PathBuf)> {
    let mut roots = vec![("target", target_root.to_path_buf())];
    for root in parent_roots {
        roots.push(("parent", root.clone()));
    }
    if let Some(root) = game_root {
        roots.push(("game", root.to_path_buf()));
    }
    roots
}

fn symbol_text_exists(roots: &[(&'static str, PathBuf)], symbol: &str) -> Result<bool, String> {
    for (_, root) in roots {
        for base in ["common", "events", "interface"] {
            let dir = root.join(base);
            if !dir.is_dir() {
                continue;
            }
            for file in collect_files(&dir)? {
                if file.extension().and_then(OsStr::to_str) != Some("txt")
                    && file.extension().and_then(OsStr::to_str) != Some("gfx")
                    && file.extension().and_then(OsStr::to_str) != Some("gui")
                {
                    continue;
                }
                if read_utf8_lossy(&file)?.contains(symbol) {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

fn system_pack_template_evidence(
    common_dir: &str,
    roots: &[(&'static str, PathBuf)],
) -> Result<String, String> {
    let mut parts = Vec::new();
    for (role, root) in roots {
        let dir = root.join("common").join(common_dir);
        let count = if dir.is_dir() {
            collect_files(&dir)?
                .into_iter()
                .filter(|file| file.extension().and_then(OsStr::to_str) == Some("txt"))
                .count()
        } else {
            0
        };
        if count > 0 {
            parts.push(format!(
                "{{\"role\": {}, \"common_dir\": {}, \"file_count\": {}}}",
                json_str(role),
                json_str(common_dir),
                count
            ));
        }
    }
    if parts.is_empty() {
        parts.push(format!(
            "{{\"role\": \"none\", \"common_dir\": {}, \"file_count\": 0}}",
            json_str(common_dir)
        ));
    }
    Ok(format!("[{}]", parts.join(", ")))
}

fn system_pack_dependency_edges(pack: SystemPackSpec) -> Vec<String> {
    pack.reference_roles
        .iter()
        .map(|role| role.to_string())
        .collect()
}

fn sanitize_pack_file_stem(value: &str) -> String {
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
        "system_pack".to_string()
    } else {
        stem
    }
}

struct SystemPackReport<'a> {
    ok: bool,
    pack: SystemPackSpec,
    target_root: &'a Path,
    prefix: &'a str,
    template_mode: &'a str,
    target_files: &'a [String],
    template_evidence: &'a [String],
    dependency_edges: &'a [String],
    references: &'a [(String, String)],
    blockers: &'a [String],
}

fn system_pack_plan_json(report: SystemPackReport<'_>) -> String {
    let mut map = BTreeMap::new();
    map.insert(
        "schema".to_string(),
        json_str("hoi4skill.system_pack_plan.v1"),
    );
    map.insert("ok".to_string(), json_bool(report.ok).to_string());
    map.insert(
        "status".to_string(),
        json_str(if report.ok {
            "system_pack_plan_ready"
        } else {
            "system_pack_plan_blocked"
        }),
    );
    map.insert("pack".to_string(), json_str(report.pack.id));
    map.insert("title".to_string(), json_str(report.pack.title));
    map.insert("prefix".to_string(), json_str(report.prefix));
    map.insert(
        "target_root".to_string(),
        json_str(&report.target_root.display().to_string()),
    );
    map.insert(
        "common_dirs".to_string(),
        json_array(
            &report
                .pack
                .common_dirs
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>(),
        ),
    );
    map.insert(
        "definition_roles".to_string(),
        json_array(
            &report
                .pack
                .definition_roles
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>(),
        ),
    );
    map.insert(
        "dependency_graph".to_string(),
        json_array(report.dependency_edges),
    );
    map.insert("target_files".to_string(), json_array(report.target_files));
    map.insert("template_mode".to_string(), json_str(report.template_mode));
    map.insert(
        "template_evidence".to_string(),
        format!("[{}]", report.template_evidence.join(", ")),
    );
    map.insert(
        "requested_references".to_string(),
        reference_pairs_json(report.references),
    );
    map.insert("user_summary".to_string(), json_str(report.pack.summary));
    map.insert(
        "runtime_checks".to_string(),
        json_array(&[
            "validate --strict-code-index".to_string(),
            "runtime-error-baseline before change".to_string(),
            report.pack.runtime_check.to_string(),
        ]),
    );
    map.insert(
        "apply_command".to_string(),
        json_str("hoi4skill system-pack-apply --input <plan.json> --execute --final-check"),
    );
    map.insert("blockers".to_string(), json_array(report.blockers));
    json_raw_object(&map) + "\n"
}

fn reference_pairs_json(references: &[(String, String)]) -> String {
    format!(
        "[{}]",
        references
            .iter()
            .map(|(kind, id)| format!("{{\"kind\": {}, \"id\": {}}}", json_str(kind), json_str(id)))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn system_pack_target_path(target_root: &Path, relative: &str) -> Result<PathBuf, String> {
    let normalized = relative.replace('\\', "/");
    if !normalized.starts_with("common/") || normalized.contains("..") {
        return Err(format!(
            "unsafe system pack target path `{relative}`; expected common/<dir>/<file>.txt"
        ));
    }
    Ok(target_root.join(Path::new(&normalized)))
}

fn system_pack_skeleton(pack: SystemPackSpec, prefix: &str, relative: &str) -> String {
    let dir = relative
        .replace('\\', "/")
        .split('/')
        .nth(1)
        .unwrap_or("common")
        .to_string();
    let role = system_pack_dir_role(pack, &dir);
    format!(
        "# Generated by hoi4skill P18 system-pack-apply.\n# pack = {}\n# prefix = {}\n# common_dir = {}\n# role = {}\n#\n# This conservative skeleton is intentionally inert. Fill this file through\n# the matching schema-specific planner, then rerun validate --strict-code-index\n# and runtime-error-regression before release.\n# dependency_graph = {}\n",
        pack.id,
        sanitize_pack_file_stem(prefix),
        dir,
        role,
        pack.reference_roles.join("; ")
    )
}

fn system_pack_dir_role(pack: SystemPackSpec, dir: &str) -> &'static str {
    pack.common_dirs
        .iter()
        .position(|candidate| *candidate == dir)
        .and_then(|idx| pack.definition_roles.get(idx).copied())
        .unwrap_or("system component")
}

fn write_system_pack_transaction(
    write_plan: &[(String, PathBuf, String)],
) -> Result<Vec<String>, (String, Vec<PathBuf>)> {
    let mut changed = Vec::new();
    for (_, path, content) in write_plan {
        if let Some(parent) = path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                return Err((format!("create {}: {err}", parent.display()), changed));
            }
        }
        if let Err(err) = fs::write(path, content) {
            return Err((format!("write {}: {err}", path.display()), changed));
        }
        changed.push(path.clone());
    }
    Ok(changed
        .iter()
        .map(|path| path.display().to_string())
        .collect())
}

fn rollback_system_pack_files(changed: &[PathBuf]) -> Vec<String> {
    let mut blockers = Vec::new();
    for path in changed.iter().rev() {
        if let Err(err) = fs::remove_file(path) {
            blockers.push(format!("rollback remove {}: {err}", path.display()));
        }
    }
    blockers
}

fn system_pack_apply_json(
    input: &Path,
    ok: bool,
    changed_files: &[String],
    blockers: &[String],
    rollback_blockers: &[String],
) -> String {
    let mut map = BTreeMap::new();
    map.insert(
        "schema".to_string(),
        json_str("hoi4skill.system_pack_apply.v1"),
    );
    map.insert("ok".to_string(), json_bool(ok).to_string());
    map.insert(
        "status".to_string(),
        json_str(if ok {
            "system_pack_applied"
        } else {
            "system_pack_apply_blocked"
        }),
    );
    map.insert("input".to_string(), json_str(&input.display().to_string()));
    map.insert(
        "transaction".to_string(),
        json_str(if ok {
            "committed_schema_skeleton_files"
        } else if changed_files.is_empty() {
            "not_started_no_files_changed"
        } else {
            "rollback_attempted"
        }),
    );
    map.insert("changed_files".to_string(), json_array(changed_files));
    map.insert(
        "rollback_ok".to_string(),
        json_bool(rollback_blockers.is_empty()).to_string(),
    );
    map.insert(
        "rollback_blockers".to_string(),
        json_array(rollback_blockers),
    );
    map.insert("blockers".to_string(), json_array(blockers));
    map.insert(
        "final_check".to_string(),
        json_str("rerun validate --strict-code-index and runtime-error-regression after filling generated skeletons"),
    );
    json_raw_object(&map) + "\n"
}
