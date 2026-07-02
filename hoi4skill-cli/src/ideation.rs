//! P5/P6 planning utilities for idea sketches, export plans, batch state edits,
//! console help, and route guidance.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_focus_ideation_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = require_value(&map, "text")?;
    let prefix = value(&map, "prefix").unwrap_or("idea");
    let themes = focus_idea_titles(&text);
    let nodes = themes
        .iter()
        .enumerate()
        .map(|(idx, title)| {
            format!(
                "{{\"id\": {}, \"title\": {}, \"x\": {}, \"y\": {}, \"prerequisite\": {}}}",
                json_str(&format!("{}_{}", prefix, slugify(title, "focus"))),
                json_str(title),
                idx * 10,
                idx / 3,
                if idx == 0 {
                    "null".to_string()
                } else {
                    json_str(&format!(
                        "{}_{}",
                        prefix,
                        slugify(&themes[idx - 1], "focus")
                    ))
                }
            )
        })
        .collect::<Vec<_>>();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": true,\n  \"status\": {},\n  \"text\": {},\n  \"node_count\": {},\n  \"nodes\": [{}],\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.focus_ideation_plan.v1"),
        json_str("focus_sketch_ready"),
        json_str(&text),
        nodes.len(),
        nodes.join(", "),
        json_str("this is a structured sketch for UI review; final focus code still needs registered IDs, localisation, effects, and validation")
    );
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_export_mod(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let root = ideation_mod_root(&map)?;
    let output_dir = value(&map, "output-dir")
        .map(normalize_path)
        .transpose()?
        .unwrap_or_else(default_export_mod_dir);
    let execute = map.flags.contains("execute");
    let overwrite = map.flags.contains("overwrite");
    let descriptor = root.join("descriptor.mod");
    let descriptor_text = if descriptor.exists() {
        Some(read_utf8_lossy(&descriptor)?)
    } else {
        None
    };
    let export_id = value(&map, "id")
        .map(str::to_string)
        .or_else(|| {
            descriptor_text
                .as_deref()
                .and_then(|text| export_descriptor_scalar_value(text, "name"))
        })
        .unwrap_or_else(|| {
            root.file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("hoi4skill_export")
                .to_string()
        });
    let folder_name = slugify_ascii(&export_id, "hoi4skill_export");
    let destination_root = output_dir.join(&folder_name);
    let launcher_descriptor = output_dir.join(format!("{folder_name}.mod"));
    let runtime_release_gate = value(&map, "runtime-release-gate")
        .or_else(|| value(&map, "release-gate"))
        .map(normalize_path)
        .transpose()?;
    let runtime_release_ready = runtime_release_gate
        .as_deref()
        .map(export_runtime_release_gate_ready)
        .transpose()?
        .unwrap_or(false);
    let source_files = export_mod_source_files(&root)?;
    let mut blockers = Vec::new();
    if !root.is_dir() {
        blockers.push(format!("mod root `{}` does not exist", root.display()));
    }
    if descriptor_text.is_none() {
        blockers.push("descriptor.mod is missing".to_string());
    }
    if execute && runtime_release_gate.is_none() {
        blockers.push("export execute requires --runtime-release-gate from P109".to_string());
    }
    if execute && !runtime_release_ready {
        blockers.push("runtime-release-gate report is missing or not passing".to_string());
    }
    if execute && destination_root.exists() && !overwrite {
        blockers.push(format!(
            "destination `{}` already exists; rerun with --overwrite after reviewing backup policy",
            destination_root.display()
        ));
    }
    if execute && launcher_descriptor.exists() && !overwrite {
        blockers.push(format!(
            "launcher descriptor `{}` already exists; rerun with --overwrite after reviewing backup policy",
            launcher_descriptor.display()
        ));
    }
    let backup_dir = output_dir
        .join(".hoi4skill_export_backups")
        .join(format!("{}_backup", folder_name));
    let manifest_dir = output_dir.join(".hoi4skill_export_manifests");
    let manifest = manifest_dir.join(format!("{folder_name}_manifest.json"));
    let rollback = manifest_dir.join(format!("{folder_name}_rollback.json"));
    let mut written_files = Vec::new();
    if execute && blockers.is_empty() {
        fs::create_dir_all(&output_dir)
            .map_err(|e| format!("create export output dir {}: {e}", output_dir.display()))?;
        fs::create_dir_all(&manifest_dir)
            .map_err(|e| format!("create export manifest dir {}: {e}", manifest_dir.display()))?;
        if overwrite && (destination_root.exists() || launcher_descriptor.exists()) {
            fs::create_dir_all(&backup_dir)
                .map_err(|e| format!("create export backup dir {}: {e}", backup_dir.display()))?;
            if destination_root.exists() {
                copy_dir_recursive(
                    &destination_root,
                    &backup_dir.join(&folder_name),
                    &mut Vec::new(),
                )?;
            }
            if launcher_descriptor.exists() {
                fs::copy(
                    &launcher_descriptor,
                    backup_dir.join(format!("{folder_name}.mod")),
                )
                .map_err(|e| format!("backup launcher descriptor: {e}"))?;
            }
        }
        copy_dir_recursive(&root, &destination_root, &mut written_files)?;
        let descriptor_body = export_launcher_descriptor(&export_id, &destination_root);
        fs::write(&launcher_descriptor, descriptor_body).map_err(|e| {
            format!(
                "write launcher descriptor {}: {e}",
                launcher_descriptor.display()
            )
        })?;
        written_files.push(launcher_descriptor.display().to_string());
        let rollback_json = export_mod_rollback_json(
            &destination_root,
            &launcher_descriptor,
            &backup_dir,
            overwrite,
        );
        fs::write(&rollback, &rollback_json)
            .map_err(|e| format!("write rollback plan {}: {e}", rollback.display()))?;
        let manifest_json = export_mod_manifest_json(
            &root,
            &output_dir,
            &destination_root,
            &launcher_descriptor,
            runtime_release_gate.as_deref(),
            &source_files,
            &written_files,
            &rollback,
        );
        fs::write(&manifest, &manifest_json)
            .map_err(|e| format!("write export manifest {}: {e}", manifest.display()))?;
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"mod_root\": {},\n  \"output_dir\": {},\n  \"export_id\": {},\n  \"destination_root\": {},\n  \"launcher_descriptor\": {},\n  \"execute\": {},\n  \"overwrite\": {},\n  \"descriptor_exists\": {},\n  \"runtime_release_gate\": {},\n  \"runtime_release_ready\": {},\n  \"source_file_count\": {},\n  \"written_file_count\": {},\n  \"manifest\": {},\n  \"rollback_plan\": {},\n  \"backup_dir\": {},\n  \"blockers\": {},\n  \"next_commands\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.export_mod.v1"),
        json_bool(ok),
        json_str(if ok && execute {
            "export_applied"
        } else if ok {
            "export_plan_ready"
        } else {
            "blocked"
        }),
        json_str(&root.display().to_string()),
        json_str(&output_dir.display().to_string()),
        json_str(&export_id),
        json_str(&destination_root.display().to_string()),
        json_str(&launcher_descriptor.display().to_string()),
        json_bool(execute),
        json_bool(overwrite),
        json_bool(descriptor_text.is_some()),
        json_optional_str(
            runtime_release_gate
                .as_ref()
                .map(|path| path.display().to_string())
                .as_deref(),
        ),
        json_bool(runtime_release_ready),
        source_files.len(),
        written_files.len(),
        json_str(&manifest.display().to_string()),
        json_str(&rollback.display().to_string()),
        json_str(&backup_dir.display().to_string()),
        json_array(&blockers),
        json_array(&[
            "hoi4skill runtime-release-gate --mod-root <mod> --game-root <HOI4 root> --validation validation.json --error-regression runtime_regression.json --runtime-evidence runtime_evidence_gate.json --require-p101-p108 --require-passed".to_string(),
            "hoi4skill export-mod --mod-root <mod> --output-dir <Documents/Paradox Interactive/Hearts of Iron IV/mod> --runtime-release-gate runtime_release_gate.json --execute --output export_report.json".to_string(),
        ]),
        json_str("P110 export requires descriptor.mod plus P109 runtime-release-gate evidence before --execute writes the launcher descriptor, exported mod folder, manifest, and rollback plan.")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

fn default_export_mod_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Documents")
        .join("Paradox Interactive")
        .join("Hearts of Iron IV")
        .join("mod")
}

fn slugify_ascii(value: &str, fallback: &str) -> String {
    let slug = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    let slug = slug
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    if slug.is_empty() {
        fallback.to_string()
    } else {
        slug
    }
}

fn export_runtime_release_gate_ready(path: &Path) -> Result<bool, String> {
    let text = read_utf8_lossy(path)?;
    Ok(export_json_report_contains_marker(
        &text,
        "\"schema\": \"hoi4skill.runtime_release_gate.v1\"",
    ) && export_json_report_contains_marker(&text, "\"ok\": true")
        && text.contains("runtime_release_ready"))
}

fn export_json_report_contains_marker(text: &str, marker: &str) -> bool {
    if text.contains(marker) {
        return true;
    }
    let compact_text = text
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    let compact_marker = marker
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    compact_text.contains(&compact_marker)
}

fn export_descriptor_scalar_value(text: &str, key: &str) -> Option<String> {
    let marker = format!("{key}=");
    text.lines().find_map(|line| {
        let line = line.trim();
        if line.starts_with('#') || !line.starts_with(&marker) {
            return None;
        }
        let value = line[marker.len()..].trim();
        Some(value.trim_matches('"').to_string()).filter(|value| !value.is_empty())
    })
}

fn export_mod_source_files(root: &Path) -> Result<Vec<String>, String> {
    let mut files = collect_files(root)?
        .into_iter()
        .filter(|path| path.is_file())
        .filter(|path| !path_has_component(path, ".git"))
        .map(|path| {
            path.strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

fn path_has_component(path: &Path, component: &str) -> bool {
    path.components().any(|part| {
        part.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(component)
    })
}

fn copy_dir_recursive(src: &Path, dst: &Path, written: &mut Vec<String>) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("create {}: {e}", dst.display()))?;
    for entry in fs::read_dir(src).map_err(|e| format!("read dir {}: {e}", src.display()))? {
        let entry = entry.map_err(|e| format!("read dir entry {}: {e}", src.display()))?;
        let path = entry.path();
        if path_has_component(&path, ".git") {
            continue;
        }
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &target, written)?;
        } else if path.is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("create {}: {e}", parent.display()))?;
            }
            fs::copy(&path, &target)
                .map_err(|e| format!("copy {} to {}: {e}", path.display(), target.display()))?;
            written.push(target.display().to_string());
        }
    }
    Ok(())
}

fn export_launcher_descriptor(name: &str, destination_root: &Path) -> String {
    format!(
        "name=\"{}\"\npath=\"{}\"\nsupported_version=\"*\"\n",
        name.replace('"', "'"),
        destination_root.display().to_string().replace('\\', "/")
    )
}

fn export_mod_rollback_json(
    destination_root: &Path,
    launcher_descriptor: &Path,
    backup_dir: &Path,
    overwrite: bool,
) -> String {
    format!(
        "{{\n  \"schema\": {},\n  \"manual_rollback\": {},\n  \"destination_root\": {},\n  \"launcher_descriptor\": {},\n  \"backup_dir\": {},\n  \"overwrite_backup_created\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.export_mod_rollback.v1"),
        json_array(&[
            format!("disable the launcher descriptor {}", launcher_descriptor.display()),
            format!("remove exported folder {}", destination_root.display()),
            format!("if overwrite was used, restore files from {}", backup_dir.display()),
        ]),
        json_str(&destination_root.display().to_string()),
        json_str(&launcher_descriptor.display().to_string()),
        json_str(&backup_dir.display().to_string()),
        json_bool(overwrite),
        json_str("rollback is explicit and reviewable; export-mod never deletes user files automatically")
    )
}

fn export_mod_manifest_json(
    root: &Path,
    output_dir: &Path,
    destination_root: &Path,
    launcher_descriptor: &Path,
    runtime_release_gate: Option<&Path>,
    source_files: &[String],
    written_files: &[String],
    rollback: &Path,
) -> String {
    format!(
        "{{\n  \"schema\": {},\n  \"mod_root\": {},\n  \"output_dir\": {},\n  \"destination_root\": {},\n  \"launcher_descriptor\": {},\n  \"runtime_release_gate\": {},\n  \"source_files\": {},\n  \"written_files\": {},\n  \"rollback_plan\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.export_mod_manifest.v1"),
        json_str(&root.display().to_string()),
        json_str(&output_dir.display().to_string()),
        json_str(&destination_root.display().to_string()),
        json_str(&launcher_descriptor.display().to_string()),
        json_optional_str(runtime_release_gate.map(|path| path.display().to_string()).as_deref()),
        json_array(source_files),
        json_array(written_files),
        json_str(&rollback.display().to_string()),
        json_str("manifest records local paths and generated reports only; it does not embed game or parent-mod source code")
    )
}

pub(crate) fn cmd_state_batch_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let index = value(&map, "game-root")
        .map(normalize_path)
        .transpose()?
        .map(|root| build_game_index_with_mod_paths(&root, &[]))
        .transpose()?;
    let states = repeated_values(&map, "state")
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if states.is_empty() {
        return Err("missing --state".to_string());
    }
    let owner = value(&map, "owner");
    let controller = value(&map, "controller");
    let resources = repeated_values(&map, "resource")
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let population = value(&map, "population");
    let victory_points = repeated_values(&map, "victory-point")
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut blockers = Vec::new();
    if let Some(index) = &index {
        for state in &states {
            match state.parse::<i64>() {
                Ok(id) if index.state_ids.contains(&id) => {}
                Ok(id) => blockers.push(format!("state id `{id}` is not indexed")),
                Err(_) => blockers.push(format!("state id `{state}` is not an integer")),
            }
        }
        for tag in [owner, controller].into_iter().flatten() {
            if !index.country_tags.contains(tag) {
                blockers.push(format!("country tag `{tag}` is not indexed"));
            }
        }
        for resource in &resources {
            let name = resource
                .split_once('=')
                .map(|(name, _)| name)
                .unwrap_or(resource)
                .trim();
            if !index.resources.contains(name) {
                blockers.push(format!("resource `{name}` is not indexed"));
            }
        }
        for vp in &victory_points {
            let province = vp.split_once('=').map(|(id, _)| id).unwrap_or(vp).trim();
            match province.parse::<i64>() {
                Ok(id) if index.province_ids.contains(&id) => {}
                Ok(id) => blockers.push(format!("victory point province id `{id}` is not indexed")),
                Err(_) => blockers.push(format!(
                    "victory point province `{province}` is not an integer"
                )),
            }
        }
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"indexed_validation\": {},\n  \"states\": {},\n  \"owner\": {},\n  \"controller\": {},\n  \"resources\": {},\n  \"population\": {},\n  \"victory_points\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.state_batch_plan.v1"),
        json_bool(ok),
        json_str(if ok { "state_batch_plan_ready" } else { "blocked" }),
        json_bool(index.is_some()),
        json_array(&states),
        json_optional_str(owner),
        json_optional_str(controller),
        json_array(&resources),
        json_optional_str(population),
        json_array(&victory_points),
        json_array(&blockers),
        json_str("state edits are scoped to history/states and must be validated against indexed state IDs before execution")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_state_batch_apply(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let plan = read_utf8_lossy(&input)?;
    let execute = map.flags.contains("execute");
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": true,\n  \"status\": {},\n  \"input\": {},\n  \"execute\": {},\n  \"plan_schema_detected\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.state_batch_apply.v1"),
        json_str(if execute { "apply_requested" } else { "plan_checked" }),
        json_str(&input.display().to_string()),
        json_bool(execute),
        json_bool(plan.contains("hoi4skill.state_batch_plan.v1")),
        json_str("P6 apply is a guarded entry point; final state file mutation should stay changed-file scoped and validated")
    );
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_console_command_help(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = require_value(&map, "text")?;
    let command = if text.contains("所有科技") || text.to_ascii_lowercase().contains("all tech")
    {
        "research all"
    } else if text.contains("政治点") {
        "pp 100"
    } else if text.contains("吞并") || text.to_ascii_lowercase().contains("annex") {
        "annex <TAG>"
    } else {
        "help"
    };
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": true,\n  \"status\": {},\n  \"text\": {},\n  \"command\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.console_command_help.v1"),
        json_str("console_command_ready"),
        json_str(&text),
        json_str(command),
        json_str("console suggestions are gameplay help, not mod code")
    );
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_gameplay_guide(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = value(&map, "text").unwrap_or("requested route");
    let route = value(&map, "route").unwrap_or("default");
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": true,\n  \"status\": {},\n  \"route\": {},\n  \"text\": {},\n  \"steps\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.gameplay_guide.v1"),
        json_str("guide_plan_ready"),
        json_str(route),
        json_str(text),
        json_array(&[
            "read indexed focus/event route evidence".to_string(),
            "list required choices and focus order".to_string(),
            "include useful console commands only when the user asks for gameplay help".to_string(),
        ]),
        json_str("guide output must cite route evidence when available; do not invent hidden event IDs")
    );
    write_or_print(&json, value(&map, "output"))
}

fn focus_idea_titles(text: &str) -> Vec<String> {
    if text.contains("大明") || text.contains("明朝") {
        vec![
            "整顿锦衣卫".to_string(),
            "重启郑和下西洋".to_string(),
            "研发神机营火器".to_string(),
            "整饬江南税粮".to_string(),
            "重建辽东边防".to_string(),
        ]
    } else {
        vec![
            "确立新路线".to_string(),
            "整顿国家机器".to_string(),
            "动员工业基础".to_string(),
            "召开特别会议".to_string(),
            "宣布长期目标".to_string(),
        ]
    }
}

fn ideation_mod_root(map: &ArgMap) -> Result<PathBuf, String> {
    let input = map
        .positionals
        .first()
        .cloned()
        .or_else(|| value(map, "mod-root").map(str::to_string))
        .ok_or_else(|| "missing mod root".to_string())?;
    Ok(resolve_mod_root(&normalize_path(&input)?)?.root)
}
