//! P21 UI/profile/cosmetic common-system plan gate.
//!
//! These systems affect polish more than core gameplay. The gate checks assets,
//! sprite registration, parent/template evidence, and visual/runtime smoke
//! requirements without pretending to generate complex UI logic.

#[allow(unused_imports)]
use crate::*;

const UI_COSMETIC_COMMON_DIRS: &[&str] = &[
    "profile_backgrounds",
    "profile_pictures",
    "ribbons",
    "medals",
    "unit_medals",
    "map_modes",
    "focus_inlay_windows",
    "frontend",
];

pub(crate) fn cmd_ui_cosmetic_common_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let target_root = resolve_mod_root(&mod_root)?.root;
    let common_dir = require_value(&map, "common-dir")
        .or_else(|_| require_value(&map, "kind"))?
        .replace('\\', "/");
    if !UI_COSMETIC_COMMON_DIRS.contains(&common_dir.as_str()) {
        return Err(format!(
            "--common-dir {common_dir} is not in the P21 UI/cosmetic whitelist"
        ));
    }
    let id = value(&map, "id").unwrap_or("generated_ui_cosmetic");
    let asset = value(&map, "asset").map(str::to_string);
    let sprite = value(&map, "sprite").map(str::to_string);
    let parent_roots = repeated_values(&map, "mod-path")
        .into_iter()
        .map(|path| resolve_mod_root(&normalize_path(path)?).map(|resolved| resolved.root))
        .collect::<Result<Vec<_>, String>>()?;
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    let roots = ui_cosmetic_roots(&target_root, &parent_roots, game_root.as_deref());
    let template_evidence = ui_cosmetic_template_evidence(&roots, &common_dir)?;

    let mut blockers = Vec::new();
    let mut todos = Vec::new();
    if template_evidence
        .iter()
        .all(|entry| entry.starts_with("none:"))
    {
        todos.push(format!(
            "no parent/game template found for common/{common_dir}; output TODO only and do not execute"
        ));
    }
    let asset_status = asset
        .as_deref()
        .map(|asset| ui_cosmetic_asset_status(&roots, asset))
        .transpose()?;
    if let Some(status) = asset_status.as_ref() {
        if !status.exists {
            blockers.push(format!("asset `{}` does not exist", status.requested));
        }
        if !matches!(
            status.extension.as_deref(),
            Some("dds" | "tga" | "png" | "jpg" | "jpeg" | "webp")
        ) {
            blockers.push(format!(
                "asset `{}` has unsupported extension",
                status.requested
            ));
        }
    }
    let sprite_registered = if let Some(sprite) = sprite.as_deref() {
        if let Some(game_root) = game_root.as_ref() {
            let mut mod_paths = parent_roots.clone();
            mod_paths.push(target_root.clone());
            let index = build_game_index_with_mod_paths(game_root, &mod_paths)?;
            let found = index.sprites.contains(sprite);
            if !found {
                blockers.push(format!(
                    "sprite `{sprite}` is not registered in indexed roots"
                ));
            }
            Some(found)
        } else {
            todos.push("sprite registration check needs --game-root".to_string());
            None
        }
    } else {
        todos.push("no --sprite provided; resource registration remains TODO".to_string());
        None
    };

    let ok = blockers.is_empty();
    let json = ui_cosmetic_plan_json(UiCosmeticReport {
        ok,
        target_root: &target_root,
        common_dir: &common_dir,
        id,
        asset_status: asset_status.as_ref(),
        sprite: sprite.as_deref(),
        sprite_registered,
        template_evidence: &template_evidence,
        todos: &todos,
        blockers: &blockers,
    });
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_ui_cosmetic_common_apply(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let plan = read_utf8_lossy(&input)?;
    let mut blockers = Vec::new();
    if !map.flags.contains("execute") {
        blockers.push("ui-cosmetic-common-apply requires --execute".to_string());
    }
    if !map.flags.contains("final-check") {
        blockers.push("ui-cosmetic-common-apply requires --final-check".to_string());
    }
    if !plan.contains("\"schema\": \"hoi4skill.ui_cosmetic_common_plan.v1\"") {
        blockers.push("input is not a ui-cosmetic-common-plan report".to_string());
    }
    if !plan.contains("\"ok\": true") {
        blockers.push("input UI/cosmetic plan is not ok".to_string());
    }
    let todos = json_string_array_field(&plan, "todos");
    if !todos.is_empty() {
        blockers.push(format!(
            "input plan still has TODOs and must not execute: {}",
            todos.join("; ")
        ));
    }
    if !plan.contains("\"sprite_registered\": true") {
        blockers.push("input plan must prove sprite_registered=true before apply".to_string());
    }
    let target_root = json_string_field(&plan, "target_root")
        .map(|path| normalize_path(&path))
        .transpose()?;
    let common_dir = json_string_field(&plan, "common_dir").unwrap_or_default();
    if !UI_COSMETIC_COMMON_DIRS.contains(&common_dir.as_str()) {
        blockers.push(format!(
            "common_dir `{common_dir}` is not in the P21 UI/cosmetic whitelist"
        ));
    }
    let id = json_string_field(&plan, "id").unwrap_or_else(|| "generated_ui_cosmetic".to_string());
    let sprite = json_string_field(&plan, "sprite").unwrap_or_default();
    let asset = json_string_field(&plan, "requested").unwrap_or_default();
    if target_root.is_none() {
        blockers.push("input plan is missing target_root".to_string());
    }
    let mut write_plan = Vec::new();
    if let Some(target_root) = target_root.as_ref() {
        let relative = format!(
            "common/{}/hoi4skill_{}_{}.txt",
            common_dir,
            sanitize_ui_cosmetic_file_stem(&id),
            common_dir
        );
        let path = target_root.join(Path::new(&relative));
        if path.exists() {
            blockers.push(format!(
                "transaction target already exists and will not be overwritten: {}",
                path.display()
            ));
        }
        write_plan.push((
            relative,
            path,
            ui_cosmetic_skeleton(&common_dir, &id, &sprite, &asset),
        ));
    }

    let mut changed_files = Vec::new();
    let mut rollback_blockers = Vec::new();
    if blockers.is_empty() {
        match write_ui_cosmetic_transaction(&write_plan) {
            Ok(changed) => changed_files = changed,
            Err((err, changed)) => {
                rollback_blockers.push(err);
                rollback_blockers.extend(rollback_ui_cosmetic_files(&changed));
                blockers
                    .push("UI/cosmetic transaction failed and rollback was attempted".to_string());
                changed_files = changed
                    .into_iter()
                    .map(|path| path.display().to_string())
                    .collect();
            }
        }
    }

    let ok = blockers.is_empty();
    let report = ui_cosmetic_apply_json(
        &input,
        ok,
        &common_dir,
        &id,
        &changed_files,
        &blockers,
        &rollback_blockers,
    );
    write_or_print(&report, value(&map, "output"))?;
    if (map.flags.contains("require-passed") || !blockers.is_empty()) && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

fn ui_cosmetic_roots(
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

fn ui_cosmetic_template_evidence(
    roots: &[(&'static str, PathBuf)],
    common_dir: &str,
) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for (role, root) in roots {
        let dir = root.join("common").join(common_dir);
        if !dir.is_dir() {
            continue;
        }
        let count = collect_files(&dir)?
            .into_iter()
            .filter(|file| file.extension().and_then(OsStr::to_str) == Some("txt"))
            .count();
        if count > 0 {
            out.push(format!("{role}:common/{common_dir}:files={count}"));
        }
    }
    if out.is_empty() {
        out.push(format!("none:common/{common_dir}:files=0"));
    }
    Ok(out)
}

struct UiCosmeticAssetStatus {
    requested: String,
    resolved: Option<String>,
    exists: bool,
    extension: Option<String>,
}

fn ui_cosmetic_asset_status(
    roots: &[(&'static str, PathBuf)],
    asset: &str,
) -> Result<UiCosmeticAssetStatus, String> {
    let path = PathBuf::from(asset);
    let candidates = if path.is_absolute() {
        vec![path]
    } else {
        roots.iter().map(|(_, root)| root.join(asset)).collect()
    };
    let resolved = candidates.into_iter().find(|path| path.is_file());
    let extension = resolved
        .as_ref()
        .unwrap_or(&PathBuf::from(asset))
        .extension()
        .and_then(OsStr::to_str)
        .map(|value| value.to_ascii_lowercase());
    Ok(UiCosmeticAssetStatus {
        requested: asset.to_string(),
        resolved: resolved.as_ref().map(|path| path.display().to_string()),
        exists: resolved.is_some(),
        extension,
    })
}

struct UiCosmeticReport<'a> {
    ok: bool,
    target_root: &'a Path,
    common_dir: &'a str,
    id: &'a str,
    asset_status: Option<&'a UiCosmeticAssetStatus>,
    sprite: Option<&'a str>,
    sprite_registered: Option<bool>,
    template_evidence: &'a [String],
    todos: &'a [String],
    blockers: &'a [String],
}

fn ui_cosmetic_plan_json(report: UiCosmeticReport<'_>) -> String {
    let mut map = BTreeMap::new();
    map.insert(
        "schema".to_string(),
        json_str("hoi4skill.ui_cosmetic_common_plan.v1"),
    );
    map.insert("ok".to_string(), json_bool(report.ok).to_string());
    map.insert(
        "status".to_string(),
        json_str(if report.ok {
            "ui_cosmetic_plan_ready"
        } else {
            "ui_cosmetic_plan_blocked"
        }),
    );
    map.insert("common_dir".to_string(), json_str(report.common_dir));
    map.insert("id".to_string(), json_str(report.id));
    map.insert(
        "target_root".to_string(),
        json_str(&report.target_root.display().to_string()),
    );
    map.insert(
        "template_evidence".to_string(),
        json_array(report.template_evidence),
    );
    map.insert(
        "asset".to_string(),
        ui_cosmetic_asset_json(report.asset_status),
    );
    map.insert("sprite".to_string(), json_optional_str(report.sprite));
    map.insert(
        "sprite_registered".to_string(),
        json_optional_bool(report.sprite_registered),
    );
    map.insert(
        "visual_runtime_smoke".to_string(),
        json_array(&[
            "gui-visual-smoke or gui-runtime-visual-probe for display changes".to_string(),
            "runtime-error-regression before release gate".to_string(),
            "release gate should warn but not block core content for cosmetic-only TODOs"
                .to_string(),
        ]),
    );
    map.insert("todos".to_string(), json_array(report.todos));
    map.insert("blockers".to_string(), json_array(report.blockers));
    map.insert(
        "rules".to_string(),
        json_array(&[
            "P21 only registers resources, learns templates, and validates references".to_string(),
            "missing template means TODO/plan only, no execute".to_string(),
            "cosmetic risk must be visible in release gate but should not block core content generation".to_string(),
        ]),
    );
    json_raw_object(&map) + "\n"
}

fn ui_cosmetic_asset_json(status: Option<&UiCosmeticAssetStatus>) -> String {
    let Some(status) = status else {
        return "null".to_string();
    };
    format!(
        "{{\"requested\": {}, \"resolved\": {}, \"exists\": {}, \"extension\": {}}}",
        json_str(&status.requested),
        json_optional_str(status.resolved.as_deref()),
        json_bool(status.exists),
        json_optional_str(status.extension.as_deref())
    )
}

fn sanitize_ui_cosmetic_file_stem(value: &str) -> String {
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
        "ui_cosmetic".to_string()
    } else {
        stem
    }
}

fn ui_cosmetic_skeleton(common_dir: &str, id: &str, sprite: &str, asset: &str) -> String {
    format!(
        "# Generated by hoi4skill P21 ui-cosmetic-common-apply.\n# common_dir = {common_dir}\n# id = {id}\n# sprite = {sprite}\n# asset = {asset}\n# This is an inert registration skeleton. Fill it from parent/game template evidence,\n# then run visual/runtime smoke and runtime-error-regression before release.\n"
    )
}

fn write_ui_cosmetic_transaction(
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

fn rollback_ui_cosmetic_files(changed: &[PathBuf]) -> Vec<String> {
    let mut blockers = Vec::new();
    for path in changed.iter().rev() {
        if let Err(err) = fs::remove_file(path) {
            blockers.push(format!("rollback remove {}: {err}", path.display()));
        }
    }
    blockers
}

fn ui_cosmetic_apply_json(
    input: &Path,
    ok: bool,
    common_dir: &str,
    id: &str,
    changed_files: &[String],
    blockers: &[String],
    rollback_blockers: &[String],
) -> String {
    let mut map = BTreeMap::new();
    map.insert(
        "schema".to_string(),
        json_str("hoi4skill.ui_cosmetic_common_apply.v1"),
    );
    map.insert("ok".to_string(), json_bool(ok).to_string());
    map.insert(
        "status".to_string(),
        json_str(if ok {
            "ui_cosmetic_applied"
        } else {
            "ui_cosmetic_apply_blocked"
        }),
    );
    map.insert("input".to_string(), json_str(&input.display().to_string()));
    map.insert("common_dir".to_string(), json_str(common_dir));
    map.insert("id".to_string(), json_str(id));
    map.insert(
        "transaction".to_string(),
        json_str(if ok {
            "committed_ui_cosmetic_skeleton_files"
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
        json_str("run gui-visual-smoke or gui-runtime-visual-probe plus runtime-error-regression after filling cosmetic skeletons"),
    );
    json_raw_object(&map) + "\n"
}
