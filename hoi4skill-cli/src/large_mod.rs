//! Large-mod production planning commands.
//!
//! These commands create planning artifacts and project scaffolds only. They do
//! not create country tags, history, map data, or gameplay scripts.

#[allow(unused_imports)]
use crate::*;

#[derive(Clone, Debug)]
pub(crate) struct LargeModBlueprint {
    pub(crate) name: String,
    pub(crate) acronym: String,
    pub(crate) default_language: String,
    pub(crate) summary: String,
    pub(crate) countries: Vec<BlueprintItem>,
    pub(crate) regions: Vec<BlueprintItem>,
    pub(crate) systems: Vec<BlueprintItem>,
    pub(crate) milestones: Vec<String>,
    pub(crate) asset_needs: Vec<String>,
    pub(crate) localisation_languages: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct BlueprintItem {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) priority: String,
}

pub(crate) fn cmd_plan_large_mod(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let source = large_mod_source_text(&map)?;
    let name = value(&map, "name")
        .map(str::to_string)
        .unwrap_or_else(|| infer_mod_name(&source));
    let acronym = value(&map, "acronym")
        .map(str::to_string)
        .unwrap_or_else(|| infer_acronym(&name));
    let default_language = value(&map, "language").unwrap_or("simp_chinese");
    let blueprint = plan_large_mod_blueprint(&source, &name, &acronym, default_language);
    write_or_print(&blueprint.to_yaml(), value(&map, "output"))
}

pub(crate) fn cmd_init_large_mod(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let blueprint_path = normalize_path(&require_value(&map, "blueprint")?)?;
    let output = normalize_path(&require_value(&map, "output")?)?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let version = value(&map, "version").unwrap_or("0.1.0");
    let supported_version = value(&map, "supported-version").unwrap_or("*");
    let tags = value(&map, "tags").unwrap_or("Alternative History");

    let mut created = scaffold_mod(
        &output,
        &blueprint.name,
        version,
        supported_version,
        tags,
        map.flags.contains("launcher-file"),
    )?;

    for dir in large_mod_project_dirs(&blueprint) {
        let path = output.join(dir);
        if !path.exists() {
            fs::create_dir_all(&path).map_err(|e| format!("create {}: {e}", path.display()))?;
            created.push(path);
        }
    }

    let hoi4skill_dir = output.join(".hoi4skill");
    fs::create_dir_all(&hoi4skill_dir)
        .map_err(|e| format!("create {}: {e}", hoi4skill_dir.display()))?;
    let blueprint_target = hoi4skill_dir.join("large_mod_blueprint.yml");
    if write_if_missing(&blueprint_target, blueprint.to_yaml().as_bytes())? {
        created.push(blueprint_target.clone());
    }

    let project_json = large_mod_project_json(&blueprint, value(&map, "game-root"));
    let project_path = hoi4skill_dir.join("project.json");
    if write_if_missing(&project_path, project_json.as_bytes())? {
        created.push(project_path);
    }

    let readme_path = output.join("README.large-mod.md");
    if write_if_missing(&readme_path, large_mod_readme(&blueprint).as_bytes())? {
        created.push(readme_path);
    }

    println!("Large mod root: {}", output.display());
    println!("Blueprint: {}", blueprint_target.display());
    if created.is_empty() {
        println!("No new files or directories were needed.");
    } else {
        println!("Created:");
        for path in created {
            println!("  {}", path.display());
        }
    }
    Ok(())
}

pub(crate) fn cmd_split_work_packages(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let blueprint_path = normalize_path(&require_value(&map, "blueprint")?)?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let output = if let Some(output) = value(&map, "output") {
        normalize_path(output)?
    } else if let Some(root) = value(&map, "mod-root") {
        normalize_path(root)?
            .join(".hoi4skill")
            .join("work_packages")
    } else {
        normalize_path("work_packages")?
    };
    let packages = split_large_mod_work_packages(&blueprint);
    fs::create_dir_all(&output).map_err(|e| format!("create {}: {e}", output.display()))?;

    let mut created = Vec::new();
    for package in &packages {
        let path = output.join(format!("{}.md", package.id));
        if write_if_missing(&path, package.to_markdown(&blueprint).as_bytes())? {
            created.push(path);
        }
    }
    let manifest = output.join("manifest.json");
    if write_if_missing(
        &manifest,
        work_package_manifest_json(&blueprint, &packages).as_bytes(),
    )? {
        created.push(manifest);
    }

    println!("Work package root: {}", output.display());
    println!("Packages: {}", packages.len());
    if created.is_empty() {
        println!("No new files were needed.");
    } else {
        println!("Created:");
        for path in created {
            println!("  {}", path.display());
        }
    }
    Ok(())
}

pub(crate) fn cmd_generate_work_package(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    if !map.flags.contains("dry-run") {
        return Err(
            "generate-work-package is dry-run only in this version; pass --dry-run".to_string(),
        );
    }
    let package_id = require_value(&map, "package")?;
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let package = packages
        .iter()
        .find(|package| package.id == package_id)
        .ok_or_else(|| {
            format!(
                "unknown package `{}`; available packages: {}",
                package_id,
                packages
                    .iter()
                    .map(|package| package.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
    let json = work_package_plan_json(&blueprint, package, &blueprint_path, mod_root.as_deref());
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_work_package_start_brief(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let package_id = require_value(&map, "package")?;
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let package = packages
        .iter()
        .find(|package| package.id == package_id)
        .ok_or_else(|| {
            format!(
                "unknown package `{}`; available packages: {}",
                package_id,
                packages
                    .iter()
                    .map(|package| package.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
    let markdown = work_package_start_brief_markdown(
        &blueprint,
        package,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
    );
    write_or_print(&markdown, value(&map, "output"))
}

pub(crate) fn cmd_work_package_start_briefs(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let output_dir = if let Some(output_dir) = value(&map, "output-dir") {
        normalize_path(output_dir)?
    } else if let Some(root) = mod_root.as_deref() {
        root.join(".hoi4skill").join("start_briefs")
    } else {
        return Err("missing --output-dir when --mod-root is not provided".to_string());
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let manifest = write_work_package_start_briefs(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &output_dir,
        map.flags.contains("ready-only"),
    )?;
    write_or_print(&manifest, value(&map, "output"))
}

pub(crate) fn cmd_work_package_authoring_pack(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let package_id = require_value(&map, "package")?;
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let output_dir = if let Some(output_dir) = value(&map, "output-dir") {
        normalize_path(output_dir)?
    } else if let Some(root) = mod_root.as_deref() {
        root.join(".hoi4skill")
            .join("authoring")
            .join(package_id.as_str())
    } else {
        return Err("missing --output-dir when --mod-root is not provided".to_string());
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let package = packages
        .iter()
        .find(|package| package.id == package_id)
        .ok_or_else(|| {
            format!(
                "unknown package `{}`; available packages: {}",
                package_id,
                packages
                    .iter()
                    .map(|package| package.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
    let manifest = write_work_package_authoring_pack(
        &blueprint,
        package,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &output_dir,
    )?;
    write_or_print(&manifest, value(&map, "output"))
}

pub(crate) fn cmd_work_package_claim(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let package_id = require_value(&map, "package")?;
    let assignee = require_value(&map, "assignee")?;
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let output = if let Some(output) = value(&map, "output") {
        normalize_path(output)?
    } else if let Some(root) = mod_root.as_deref() {
        root.join(".hoi4skill")
            .join("claims")
            .join(format!("claim_{package_id}.json"))
    } else {
        return Err("missing --output when --mod-root is not provided".to_string());
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let package = packages
        .iter()
        .find(|package| package.id == package_id)
        .ok_or_else(|| {
            format!(
                "unknown package `{}`; available packages: {}",
                package_id,
                packages
                    .iter()
                    .map(|package| package.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
    let claim = work_package_claim_json(
        &blueprint,
        package,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &assignee,
        &output,
    );
    let start_state = work_package_start_state(package, &packages, mod_root.as_deref());
    if start_state.state == "blocked_by_dependencies" && !map.flags.contains("allow-blocked") {
        return Err(format!(
            "package `{package_id}` is blocked by dependencies: {}; pass --allow-blocked to record a blocked claim",
            start_state.blocked_by.join(", ")
        ));
    }
    if output.exists() && !map.flags.contains("force") {
        return Err(format!(
            "claim already exists at {}; pass --force to replace it",
            output.display()
        ));
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    fs::write(&output, claim).map_err(|e| format!("write {}: {e}", output.display()))?;
    println!("{}", output.display());
    Ok(())
}

pub(crate) fn cmd_work_package_release_claim(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let package_id = require_value(&map, "package")?;
    let reason = require_value(&map, "reason")?;
    if reason.trim().is_empty() {
        return Err("missing non-empty --reason".to_string());
    }
    let released_by = value(&map, "released-by")
        .map(str::to_string)
        .unwrap_or_else(|| "unknown".to_string());
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let claims_dir = if let Some(claims_dir) = value(&map, "claims-dir") {
        normalize_path(claims_dir)?
    } else if let Some(root) = mod_root.as_deref() {
        root.join(".hoi4skill").join("claims")
    } else {
        return Err("missing --claims-dir when --mod-root is not provided".to_string());
    };
    let claim_path = if let Some(claim_path) = value(&map, "claim") {
        normalize_path(claim_path)?
    } else {
        claims_dir.join(format!("claim_{package_id}.json"))
    };
    if !claim_path.exists() {
        return Err(format!("no active claim found at {}", claim_path.display()));
    }

    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let package = packages
        .iter()
        .find(|package| package.id == package_id)
        .ok_or_else(|| {
            format!(
                "unknown package `{}`; available packages: {}",
                package_id,
                packages
                    .iter()
                    .map(|package| package.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;

    let claim_text = read_utf8_lossy(&claim_path)?;
    if !claim_text.contains("\"schema\": \"hoi4skill.work_package_claim.v1\"") {
        return Err(format!(
            "{} is not a hoi4skill work package claim",
            claim_path.display()
        ));
    }
    let package_marker = format!("\"id\": {}", json_str(&package_id));
    if !claim_text.contains(&package_marker) {
        return Err(format!(
            "{} does not belong to package `{package_id}`",
            claim_path.display()
        ));
    }
    let output = if let Some(output) = value(&map, "output") {
        normalize_path(output)?
    } else if let Some(root) = mod_root.as_deref() {
        root.join(".hoi4skill")
            .join("claim_releases")
            .join(format!("release_{package_id}.json"))
    } else {
        claims_dir.join(format!("release_{package_id}.json"))
    };
    if output == claim_path {
        return Err("release output must not be the active claim path".to_string());
    }
    let release = work_package_claim_release_json(
        &blueprint,
        package,
        &blueprint_path,
        mod_root.as_deref(),
        &claim_path,
        &output,
        &claim_text,
        &released_by,
        &reason,
    );
    if output.exists() && !map.flags.contains("force") {
        return Err(format!(
            "release record already exists at {}; pass --force to replace it",
            output.display()
        ));
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    fs::write(&output, release).map_err(|e| format!("write {}: {e}", output.display()))?;
    fs::remove_file(&claim_path).map_err(|e| format!("remove {}: {e}", claim_path.display()))?;
    println!("{}", output.display());
    Ok(())
}

pub(crate) fn cmd_work_package_claims(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let claims_dir = if let Some(claims_dir) = value(&map, "claims-dir") {
        normalize_path(claims_dir)?
    } else if let Some(root) = mod_root.as_deref() {
        root.join(".hoi4skill").join("claims")
    } else {
        return Err("missing --claims-dir when --mod-root is not provided".to_string());
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let json = work_package_claims_json(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &claims_dir,
    );
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_work_package_dispatch_board(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let claims_dir = if let Some(claims_dir) = value(&map, "claims-dir") {
        normalize_path(claims_dir)?
    } else if let Some(root) = mod_root.as_deref() {
        root.join(".hoi4skill").join("claims")
    } else {
        return Err("missing --claims-dir when --mod-root is not provided".to_string());
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let markdown = work_package_dispatch_board_markdown(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &claims_dir,
    );
    write_or_print(&markdown, value(&map, "output"))
}

pub(crate) fn cmd_asset_pack_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let package_id = require_value(&map, "package")?;
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let package = packages
        .iter()
        .find(|package| package.id == package_id)
        .ok_or_else(|| {
            format!(
                "unknown package `{}`; available packages: {}",
                package_id,
                packages
                    .iter()
                    .map(|package| package.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
    let markdown =
        asset_pack_plan_markdown(&blueprint, package, &blueprint_path, mod_root.as_deref());
    write_or_print(&markdown, value(&map, "output"))
}

pub(crate) fn cmd_work_package_status(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let package_filter = value(&map, "package");
    let mut packages = split_large_mod_work_packages(&blueprint);
    if let Some(filter) = package_filter {
        packages.retain(|package| package.id == filter);
        if packages.is_empty() {
            return Err(format!(
                "unknown package `{}`; available packages: {}",
                filter,
                split_large_mod_work_packages(&blueprint)
                    .iter()
                    .map(|package| package.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    let reports = collect_work_package_status_reports(mod_root.as_deref(), &map)?;
    let json = work_package_status_json(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &reports,
    )?;
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_check_work_package_boundary(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let package_id = require_value(&map, "package")?;
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let package = packages
        .iter()
        .find(|package| package.id == package_id)
        .ok_or_else(|| {
            format!(
                "unknown package `{}`; available packages: {}",
                package_id,
                packages
                    .iter()
                    .map(|package| package.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
    let changed = collect_boundary_changed_paths(&map)?;
    let json = work_package_boundary_json(
        &blueprint,
        package,
        &blueprint_path,
        mod_root.as_deref(),
        &changed,
        map.flags.contains("strict-names"),
    );
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_ci_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let package_filter = value(&map, "package");
    let mut packages = split_large_mod_work_packages(&blueprint);
    if let Some(filter) = package_filter {
        packages.retain(|package| package.id == filter);
        if packages.is_empty() {
            return Err(format!(
                "unknown package `{}`; available packages: {}",
                filter,
                split_large_mod_work_packages(&blueprint)
                    .iter()
                    .map(|package| package.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    let json = large_mod_ci_plan_json(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        value(&map, "game-root"),
        map.flags.contains("strict-names"),
    );
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_release_gate(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let reports = collect_large_mod_release_gate_reports(mod_root.as_deref(), &map)?;
    let json = large_mod_release_gate_json(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &reports,
    )?;
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_dispatch_gate(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let claims_dir = if let Some(claims_dir) = value(&map, "claims-dir") {
        normalize_path(claims_dir)?
    } else if let Some(root) = mod_root.as_deref() {
        root.join(".hoi4skill").join("claims")
    } else {
        return Err("missing --claims-dir when --mod-root is not provided".to_string());
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let json = large_mod_dispatch_gate_json(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &claims_dir,
        map.flags.contains("allow-unclaimed"),
    );
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_identify_work_packages(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let changed = collect_boundary_changed_paths(&map)?;
    let json = identify_work_packages_json(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &changed,
        map.flags.contains("strict-names"),
    );
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_split_changed_work_packages(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let output_dir = if let Some(output_dir) = value(&map, "output-dir") {
        normalize_path(output_dir)?
    } else if let Some(root) = mod_root.as_deref() {
        root.join(".hoi4skill")
    } else {
        return Err("missing --output-dir when --mod-root is not provided".to_string());
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let changed = collect_boundary_changed_paths(&map)?;
    let json = split_changed_work_packages_json(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &output_dir,
        &changed,
        map.flags.contains("strict-names"),
    )?;
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_work_package_readiness(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let package_filter = value(&map, "package");
    let mut packages = split_large_mod_work_packages(&blueprint);
    if let Some(filter) = package_filter {
        packages.retain(|package| package.id == filter);
        if packages.is_empty() {
            return Err(format!(
                "unknown package `{}`; available packages: {}",
                filter,
                split_large_mod_work_packages(&blueprint)
                    .iter()
                    .map(|package| package.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    let json =
        work_package_readiness_json(&blueprint, &packages, &blueprint_path, mod_root.as_deref())?;
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_work_package_handoff(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let package_id = require_value(&map, "package")?;
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let package = packages
        .iter()
        .find(|package| package.id == package_id)
        .ok_or_else(|| {
            format!(
                "unknown package `{}`; available packages: {}",
                package_id,
                packages
                    .iter()
                    .map(|package| package.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
    let markdown =
        work_package_handoff_markdown(&blueprint, package, &blueprint_path, mod_root.as_deref())?;
    write_or_print(&markdown, value(&map, "output"))
}

pub(crate) fn cmd_work_package_review_checklist(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let package_id = require_value(&map, "package")?;
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let claims_dir = if let Some(claims_dir) = value(&map, "claims-dir") {
        normalize_path(claims_dir)?
    } else if let Some(root) = mod_root.as_deref() {
        root.join(".hoi4skill").join("claims")
    } else {
        PathBuf::from(".hoi4skill").join("claims")
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let package = packages
        .iter()
        .find(|package| package.id == package_id)
        .ok_or_else(|| {
            format!(
                "unknown package `{}`; available packages: {}",
                package_id,
                packages
                    .iter()
                    .map(|package| package.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
    let markdown = work_package_review_checklist_markdown(
        &blueprint,
        package,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &claims_dir,
    )?;
    write_or_print(&markdown, value(&map, "output"))
}

pub(crate) fn cmd_work_package_merge_gate(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let package_id = require_value(&map, "package")?;
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let claims_dir = if let Some(claims_dir) = value(&map, "claims-dir") {
        normalize_path(claims_dir)?
    } else if let Some(root) = mod_root.as_deref() {
        root.join(".hoi4skill").join("claims")
    } else {
        PathBuf::from(".hoi4skill").join("claims")
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let package = packages
        .iter()
        .find(|package| package.id == package_id)
        .ok_or_else(|| {
            format!(
                "unknown package `{}`; available packages: {}",
                package_id,
                packages
                    .iter()
                    .map(|package| package.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
    let json = work_package_merge_gate_json(
        &blueprint,
        package,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &claims_dir,
    )?;
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_work_package_merge_gates(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let claims_dir = if let Some(claims_dir) = value(&map, "claims-dir") {
        normalize_path(claims_dir)?
    } else if let Some(root) = mod_root.as_deref() {
        root.join(".hoi4skill").join("claims")
    } else {
        PathBuf::from(".hoi4skill").join("claims")
    };
    let output_dir = if let Some(output_dir) = value(&map, "output-dir") {
        normalize_path(output_dir)?
    } else if let Some(root) = mod_root.as_deref() {
        root.join(".hoi4skill").join("merge_gates")
    } else {
        return Err("missing --output-dir when --mod-root is not provided".to_string());
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let manifest = write_work_package_merge_gates(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &claims_dir,
        &output_dir,
    )?;
    write_or_print(&manifest, value(&map, "output"))
}

pub(crate) fn cmd_work_package_playtest_report(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let package_id = require_value(&map, "package")?;
    let result = normalize_playtest_result(&require_value(&map, "result")?)?;
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let package = packages
        .iter()
        .find(|package| package.id == package_id)
        .ok_or_else(|| {
            format!(
                "unknown package `{}`; available packages: {}",
                package_id,
                packages
                    .iter()
                    .map(|package| package.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
    let findings = repeated_values(&map, "finding")
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if result == "passed" && !findings.is_empty() {
        return Err("--finding requires --result needs_review".to_string());
    }
    let json = work_package_playtest_report_json(
        &blueprint,
        package,
        &blueprint_path,
        mod_root.as_deref(),
        &map,
        &result,
        &findings,
    );
    let output = value(&map, "output").map(str::to_string).or_else(|| {
        mod_root.as_ref().map(|root| {
            root.join(".hoi4skill")
                .join(format!("playtest_{}.json", package.id))
                .display()
                .to_string()
        })
    });
    write_or_print(&json, output.as_deref())
}

pub(crate) fn cmd_large_mod_merge_gate(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let claims_dir = if let Some(claims_dir) = value(&map, "claims-dir") {
        normalize_path(claims_dir)?
    } else if let Some(root) = mod_root.as_deref() {
        root.join(".hoi4skill").join("claims")
    } else {
        PathBuf::from(".hoi4skill").join("claims")
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let json = large_mod_merge_gate_json(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &claims_dir,
    );
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_review_queue(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let claims_dir = if let Some(claims_dir) = value(&map, "claims-dir") {
        normalize_path(claims_dir)?
    } else if let Some(root) = mod_root.as_deref() {
        root.join(".hoi4skill").join("claims")
    } else {
        PathBuf::from(".hoi4skill").join("claims")
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let json = large_mod_review_queue_json(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &claims_dir,
    );
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_dashboard(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let reports = collect_large_mod_release_gate_reports(mod_root.as_deref(), &map)?;
    let markdown = large_mod_dashboard_markdown(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &reports,
    )?;
    write_or_print(&markdown, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_next_actions(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let reports = collect_large_mod_release_gate_reports(mod_root.as_deref(), &map)?;
    let json = large_mod_next_actions_json(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &reports,
    )?;
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_production_snapshot(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let claims_dir = if let Some(claims_dir) = value(&map, "claims-dir") {
        normalize_path(claims_dir)?
    } else if let Some(root) = &mod_root {
        root.join(".hoi4skill").join("claims")
    } else {
        PathBuf::from(".hoi4skill").join("claims")
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let reports = collect_large_mod_release_gate_reports(mod_root.as_deref(), &map)?;
    let snapshot = large_mod_production_snapshot_state(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &claims_dir,
        &reports,
    )?;
    let json = large_mod_production_snapshot_json(&snapshot);
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_production_brief(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let claims_dir = if let Some(claims_dir) = value(&map, "claims-dir") {
        normalize_path(claims_dir)?
    } else if let Some(root) = &mod_root {
        root.join(".hoi4skill").join("claims")
    } else {
        PathBuf::from(".hoi4skill").join("claims")
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let reports = collect_large_mod_release_gate_reports(mod_root.as_deref(), &map)?;
    let snapshot = large_mod_production_snapshot_state(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &claims_dir,
        &reports,
    )?;
    let markdown = large_mod_production_brief_markdown(&snapshot);
    write_or_print(&markdown, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_fix_queue(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let reports = collect_large_mod_fix_queue_reports(mod_root.as_deref(), &map)?;
    let json = large_mod_fix_queue_json(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &reports,
    )?;
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_regression_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let reports = collect_large_mod_fix_queue_reports(mod_root.as_deref(), &map)?;
    let json = large_mod_regression_plan_json(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &reports,
    )?;
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_regression_gate(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let reports = collect_large_mod_fix_queue_reports(mod_root.as_deref(), &map)?;
    let plan_path = if let Some(plan) = value(&map, "plan") {
        normalize_path(plan)?
    } else if let Some(root) = mod_root.as_deref() {
        root.join(".hoi4skill").join("regression_plan.json")
    } else {
        PathBuf::from(".hoi4skill").join("regression_plan.json")
    };
    let json = large_mod_regression_gate_json(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &reports,
        &plan_path,
    )?;
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_regression_brief(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let reports = collect_large_mod_fix_queue_reports(mod_root.as_deref(), &map)?;
    let plan_path = if let Some(plan) = value(&map, "plan") {
        normalize_path(plan)?
    } else if let Some(root) = mod_root.as_deref() {
        root.join(".hoi4skill").join("regression_plan.json")
    } else {
        PathBuf::from(".hoi4skill").join("regression_plan.json")
    };
    let markdown = large_mod_regression_brief_markdown(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &reports,
        &plan_path,
    )?;
    write_or_print(&markdown, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_risk_register(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let claims_dir = if let Some(claims_dir) = value(&map, "claims-dir") {
        normalize_path(claims_dir)?
    } else if let Some(root) = mod_root.as_deref() {
        root.join(".hoi4skill").join("claims")
    } else {
        PathBuf::from(".hoi4skill").join("claims")
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let reports = collect_large_mod_release_gate_reports(mod_root.as_deref(), &map)?;
    let json = large_mod_risk_register_json(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &reports,
        &claims_dir,
    )?;
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_ownership_map(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let json =
        large_mod_ownership_map_json(&blueprint, &packages, &blueprint_path, mod_root.as_deref());
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_dependency_graph(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let json = large_mod_dependency_graph_json(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
    );
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_milestone_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let json =
        large_mod_milestone_plan_json(&blueprint, &packages, &blueprint_path, mod_root.as_deref());
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_execution_queue(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let json =
        large_mod_execution_queue_json(&blueprint, &packages, &blueprint_path, mod_root.as_deref());
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_evidence_pack(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let reports = collect_large_mod_release_gate_reports(mod_root.as_deref(), &map)?;
    let json = large_mod_evidence_pack_json(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &reports,
    )?;
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_review_brief(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let reports = collect_large_mod_release_gate_reports(mod_root.as_deref(), &map)?;
    let markdown = large_mod_review_brief_markdown(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &reports,
    )?;
    write_or_print(&markdown, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_release_bundle(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let reports = collect_large_mod_release_gate_reports(mod_root.as_deref(), &map)?;
    let json = large_mod_release_bundle_json(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &reports,
    )?;
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_release_brief(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let reports = collect_large_mod_release_gate_reports(mod_root.as_deref(), &map)?;
    let markdown = large_mod_release_brief_markdown(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &reports,
    )?;
    write_or_print(&markdown, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_release_notes(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let reports = collect_large_mod_playtest_reports(mod_root.as_deref(), &map)?;
    let markdown = large_mod_release_notes_markdown(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &reports,
    )?;
    write_or_print(&markdown, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_playtest_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let json =
        large_mod_playtest_plan_json(&blueprint, &packages, &blueprint_path, mod_root.as_deref());
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_playtest_gate(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let reports = collect_large_mod_playtest_reports(mod_root.as_deref(), &map)?;
    let json = large_mod_playtest_gate_json(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &reports,
    )?;
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_large_mod_playtest_brief(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = if let Some(root) = value(&map, "mod-root") {
        Some(normalize_path(root)?)
    } else {
        None
    };
    let blueprint_path = large_mod_blueprint_path_from_args(&map, mod_root.as_deref())?;
    let blueprint = read_large_mod_blueprint(&blueprint_path)?;
    let packages = split_large_mod_work_packages(&blueprint);
    let reports = collect_large_mod_playtest_reports(mod_root.as_deref(), &map)?;
    let markdown = large_mod_playtest_brief_markdown(
        &blueprint,
        &packages,
        &blueprint_path,
        mod_root.as_deref(),
        &reports,
    )?;
    write_or_print(&markdown, value(&map, "output"))
}

fn large_mod_source_text(map: &ArgMap) -> Result<String, String> {
    if let Some(text) = value(map, "text") {
        return Ok(text.to_string());
    }
    if let Some(input) = value(map, "input") {
        let path = normalize_path(input)?;
        return read_utf8_lossy(&path);
    }
    Err("missing --text or --input".to_string())
}

fn large_mod_blueprint_path_from_args(
    map: &ArgMap,
    mod_root: Option<&Path>,
) -> Result<PathBuf, String> {
    if let Some(blueprint) = value(map, "blueprint") {
        return normalize_path(blueprint);
    }
    if let Some(root) = mod_root {
        return Ok(root.join(".hoi4skill").join("large_mod_blueprint.yml"));
    }
    Err("missing --blueprint or --mod-root".to_string())
}

pub(crate) fn plan_large_mod_blueprint(
    source: &str,
    name: &str,
    acronym: &str,
    default_language: &str,
) -> LargeModBlueprint {
    let countries = infer_blueprint_items(
        source,
        &["countries", "country", "tags", "nations"],
        "country",
    );
    let regions = infer_blueprint_items(source, &["regions", "region", "areas"], "region");
    let systems = infer_blueprint_items(
        source,
        &["systems", "system", "mechanics", "features"],
        "system",
    );
    let milestones = vec![
        "prototype_playable_country".to_string(),
        "regional_content_pass".to_string(),
        "cross_system_integration".to_string(),
        "localisation_and_asset_pass".to_string(),
        "playtest_regression".to_string(),
    ];
    let asset_needs = vec![
        "focus_icons".to_string(),
        "event_pictures".to_string(),
        "idea_icons".to_string(),
        "portraits".to_string(),
    ];
    let mut localisation_languages = vec![default_language.to_string()];
    if default_language != "english" {
        localisation_languages.push("english".to_string());
    }
    LargeModBlueprint {
        name: name.to_string(),
        acronym: acronym.to_string(),
        default_language: default_language.to_string(),
        summary: compact_summary(source),
        countries: if countries.is_empty() {
            vec![BlueprintItem::new("core_country", "Core Country", "major")]
        } else {
            countries
        },
        regions: if regions.is_empty() {
            vec![BlueprintItem::new("core_region", "Core Region", "major")]
        } else {
            regions
        },
        systems: if systems.is_empty() {
            vec![
                BlueprintItem::new("political_paths", "Political Paths", "major"),
                BlueprintItem::new("economic_system", "Economic System", "major"),
                BlueprintItem::new("regional_crisis", "Regional Crisis", "supporting"),
            ]
        } else {
            systems
        },
        milestones,
        asset_needs,
        localisation_languages,
    }
}

impl BlueprintItem {
    fn new(id: &str, name: &str, priority: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            priority: priority.to_string(),
        }
    }
}

impl LargeModBlueprint {
    pub(crate) fn to_yaml(&self) -> String {
        let mut out = String::new();
        out.push_str("schema: \"hoi4skill.large_mod_blueprint.v1\"\n");
        out.push_str(&format!("name: {}\n", json_str(&self.name)));
        out.push_str(&format!("acronym: {}\n", json_str(&self.acronym)));
        out.push_str(&format!(
            "default_language: {}\n",
            json_str(&self.default_language)
        ));
        out.push_str("summary: |-\n");
        for line in self.summary.lines() {
            out.push_str("  ");
            out.push_str(line);
            out.push('\n');
        }
        push_item_list(&mut out, "countries", &self.countries);
        push_item_list(&mut out, "regions", &self.regions);
        push_item_list(&mut out, "systems", &self.systems);
        push_string_list(&mut out, "milestones", &self.milestones);
        push_string_list(&mut out, "asset_needs", &self.asset_needs);
        push_string_list(
            &mut out,
            "localisation_languages",
            &self.localisation_languages,
        );
        out
    }
}

#[derive(Clone, Debug)]
struct WorkPackage {
    id: String,
    kind: String,
    title: String,
    allowed_paths: Vec<String>,
    deliverables: Vec<String>,
    validation_steps: Vec<String>,
}

impl WorkPackage {
    fn to_markdown(&self, blueprint: &LargeModBlueprint) -> String {
        let mut out = String::new();
        out.push_str(&format!("# Work Package: {}\n\n", self.title));
        out.push_str(&format!("- id: `{}`\n", self.id));
        out.push_str(&format!("- kind: `{}`\n", self.kind));
        out.push_str(&format!("- mod: `{}`\n", blueprint.name));
        out.push_str(&format!("- blueprint: `{}`\n\n", blueprint.acronym));
        out.push_str("## Allowed Edit Surface\n\n");
        for path in &self.allowed_paths {
            out.push_str(&format!("- `{path}`\n"));
        }
        out.push_str("\n## Deliverables\n\n");
        for item in &self.deliverables {
            out.push_str(&format!("- {item}\n"));
        }
        out.push_str("\n## Validation\n\n");
        for step in &self.validation_steps {
            out.push_str(&format!("- `{step}`\n"));
        }
        out.push_str("\n## Stop Conditions\n\n");
        out.push_str("- Do not create new country tags, state history, map data, GUI, or technologies unless the blueprint and user request explicitly authorize them.\n");
        out.push_str("- Before writing gameplay script, build local game/dependency evidence and run final validation.\n");
        out
    }
}

fn split_large_mod_work_packages(blueprint: &LargeModBlueprint) -> Vec<WorkPackage> {
    let mut packages = Vec::new();
    for country in &blueprint.countries {
        packages.push(WorkPackage {
            id: format!("country_{}", country.id),
            kind: "country".to_string(),
            title: format!("{} Country Content", country.name),
            allowed_paths: vec![
                "common/national_focus".to_string(),
                "common/ideas".to_string(),
                "common/decisions".to_string(),
                "events".to_string(),
                format!("localisation/{}", blueprint.default_language),
                "interface".to_string(),
                "gfx/interface".to_string(),
            ],
            deliverables: vec![
                "focus tree plan".to_string(),
                "event chain cards".to_string(),
                "decision and national spirit cards".to_string(),
                "localisation keys".to_string(),
                "asset requirement list".to_string(),
            ],
            validation_steps: vec![
                "hoi4skill feature-context --tag <TAG>".to_string(),
                "hoi4skill reserve-id --kind event --namespace <namespace>".to_string(),
                "hoi4skill validate <mod-root> --changed-only --strict-code-index".to_string(),
            ],
        });
    }
    for region in &blueprint.regions {
        packages.push(WorkPackage {
            id: format!("region_{}", region.id),
            kind: "region".to_string(),
            title: format!("{} Regional Integration", region.name),
            allowed_paths: vec![
                "events".to_string(),
                "common/decisions".to_string(),
                "common/scripted_effects".to_string(),
                "common/scripted_triggers".to_string(),
                format!("localisation/{}", blueprint.default_language),
            ],
            deliverables: vec![
                "regional crisis/event flow".to_string(),
                "cross-country trigger/effect contracts".to_string(),
                "shared localisation keys".to_string(),
            ],
            validation_steps: vec![
                "hoi4skill impact --git-diff".to_string(),
                "hoi4skill validate <mod-root> --changed-only --strict-code-index".to_string(),
            ],
        });
    }
    for system in &blueprint.systems {
        packages.push(WorkPackage {
            id: format!("system_{}", system.id),
            kind: "system".to_string(),
            title: format!("{} System", system.name),
            allowed_paths: vec![
                "common/scripted_effects".to_string(),
                "common/scripted_triggers".to_string(),
                "common/on_actions".to_string(),
                "common/decisions".to_string(),
                format!("localisation/{}", blueprint.default_language),
            ],
            deliverables: vec![
                "system contract".to_string(),
                "scripted helper cards".to_string(),
                "integration test scenario".to_string(),
            ],
            validation_steps: vec![
                "hoi4skill query-symbol --symbol <system helper>".to_string(),
                "hoi4skill validate <mod-root> --changed-only --strict-code-index".to_string(),
            ],
        });
    }
    packages
}

fn read_large_mod_blueprint(path: &Path) -> Result<LargeModBlueprint, String> {
    parse_large_mod_blueprint(&read_utf8_lossy(path)?)
}

pub(crate) fn parse_large_mod_blueprint(text: &str) -> Result<LargeModBlueprint, String> {
    let name = parse_scalar(text, "name").unwrap_or_else(|| "Large HOI4 Mod".to_string());
    let acronym = parse_scalar(text, "acronym").unwrap_or_else(|| infer_acronym(&name));
    let default_language =
        parse_scalar(text, "default_language").unwrap_or_else(|| "simp_chinese".to_string());
    let summary = parse_block_scalar(text, "summary").unwrap_or_else(|| name.clone());
    let countries = parse_item_list(text, "countries");
    let regions = parse_item_list(text, "regions");
    let systems = parse_item_list(text, "systems");
    Ok(LargeModBlueprint {
        name,
        acronym,
        default_language,
        summary,
        countries,
        regions,
        systems,
        milestones: parse_string_list(text, "milestones"),
        asset_needs: parse_string_list(text, "asset_needs"),
        localisation_languages: parse_string_list(text, "localisation_languages"),
    })
}

fn parse_scalar(text: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    text.lines().find_map(|line| {
        let trimmed = line.trim();
        trimmed
            .strip_prefix(&prefix)
            .map(|value| unquote_yaml(value.trim()))
            .filter(|value| !value.is_empty())
    })
}

fn parse_block_scalar(text: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    let mut in_block = false;
    let mut out = String::new();
    for line in text.lines() {
        if !in_block {
            let trimmed = line.trim();
            if trimmed.starts_with(&prefix) && trimmed.contains('|') {
                in_block = true;
            }
            continue;
        }
        if line.starts_with("  ") {
            out.push_str(line.trim_start());
            out.push('\n');
        } else {
            break;
        }
    }
    let out = out.trim().to_string();
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn parse_item_list(text: &str, key: &str) -> Vec<BlueprintItem> {
    let mut items = Vec::new();
    let mut in_list = false;
    let mut current: Option<BlueprintItem> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if !in_list {
            if trimmed == format!("{key}:") {
                in_list = true;
            }
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }
        if !line.starts_with("  ") {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("- id:") {
            if let Some(item) = current.take() {
                items.push(item);
            }
            let id = unquote_yaml(value.trim());
            current = Some(BlueprintItem::new(&id, &id, "supporting"));
        } else if let Some(value) = trimmed.strip_prefix("name:") {
            if let Some(item) = &mut current {
                item.name = unquote_yaml(value.trim());
            }
        } else if let Some(value) = trimmed.strip_prefix("priority:") {
            if let Some(item) = &mut current {
                item.priority = unquote_yaml(value.trim());
            }
        }
    }
    if let Some(item) = current {
        items.push(item);
    }
    items
}

fn parse_string_list(text: &str, key: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut in_list = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if !in_list {
            if trimmed == format!("{key}:") {
                in_list = true;
            }
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }
        if !line.starts_with("  ") {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("- ") {
            values.push(unquote_yaml(value.trim()));
        }
    }
    values
}

fn unquote_yaml(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        value[1..value.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
    } else {
        value.to_string()
    }
}

fn push_item_list(out: &mut String, key: &str, items: &[BlueprintItem]) {
    out.push_str(&format!("{key}:\n"));
    for item in items {
        out.push_str(&format!("  - id: {}\n", json_str(&item.id)));
        out.push_str(&format!("    name: {}\n", json_str(&item.name)));
        out.push_str(&format!("    priority: {}\n", json_str(&item.priority)));
    }
}

fn push_string_list(out: &mut String, key: &str, values: &[String]) {
    out.push_str(&format!("{key}:\n"));
    for value in values {
        out.push_str(&format!("  - {}\n", json_str(value)));
    }
}

fn infer_mod_name(source: &str) -> String {
    for line in source.lines() {
        let trimmed = line.trim();
        for prefix in ["name:", "mod:", "title:"] {
            if let Some(value) = trimmed.strip_prefix(prefix) {
                let value = value.trim();
                if !value.is_empty() {
                    return value.to_string();
                }
            }
        }
    }
    "Large HOI4 Mod".to_string()
}

fn infer_acronym(name: &str) -> String {
    let letters: String = name
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.chars().next())
        .map(|ch| ch.to_ascii_uppercase())
        .collect();
    if letters.is_empty() {
        "LMD".to_string()
    } else {
        letters.chars().take(8).collect()
    }
}

fn compact_summary(source: &str) -> String {
    let mut lines = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(12)
        .collect::<Vec<_>>();
    if lines.is_empty() {
        lines.push("Large HOI4 mod production blueprint.");
    }
    lines.join("\n")
}

fn infer_blueprint_items(source: &str, keys: &[&str], fallback_prefix: &str) -> Vec<BlueprintItem> {
    let mut out = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        let mut matched = None;
        for key in keys {
            if let Some((_, value)) = lower.split_once(&format!("{key}:")) {
                let start = lower.len() - value.len();
                matched = Some(&trimmed[start..]);
                break;
            }
            if let Some((_, value)) = trimmed.split_once(&format!("{key}：")) {
                matched = Some(value);
                break;
            }
        }
        if let Some(values) = matched {
            for (idx, raw) in split_listish(values).into_iter().enumerate() {
                let id = slugify(&raw, &format!("{fallback_prefix}_{}", idx + 1));
                out.push(BlueprintItem::new(
                    &id,
                    &raw,
                    if idx == 0 { "major" } else { "supporting" },
                ));
            }
        }
    }
    dedupe_items(out)
}

fn split_listish(value: &str) -> Vec<String> {
    value
        .split([',', ';', '，', '；', '、', '|'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn dedupe_items(items: Vec<BlueprintItem>) -> Vec<BlueprintItem> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for item in items {
        if seen.insert(item.id.clone()) {
            out.push(item);
        }
    }
    out
}

fn large_mod_project_dirs(blueprint: &LargeModBlueprint) -> Vec<PathBuf> {
    let mut dirs = vec![
        ".hoi4skill".into(),
        ".hoi4skill/work_packages".into(),
        "common/national_focus".into(),
        "common/ideas".into(),
        "common/decisions".into(),
        "common/scripted_effects".into(),
        "common/scripted_triggers".into(),
        "common/on_actions".into(),
        "events".into(),
        "interface".into(),
        "gfx/interface/goals".into(),
        "gfx/event_pictures".into(),
    ];
    for language in &blueprint.localisation_languages {
        dirs.push(PathBuf::from("localisation").join(language));
    }
    dirs
}

fn large_mod_project_json(blueprint: &LargeModBlueprint, game_root: Option<&str>) -> String {
    format!(
        "{{\n  \"schema\": \"hoi4skill.large_mod_project.v1\",\n  \"name\": {},\n  \"acronym\": {},\n  \"default_language\": {},\n  \"game_root\": {},\n  \"countries\": {},\n  \"regions\": {},\n  \"systems\": {}\n}}\n",
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&blueprint.default_language),
        json_optional_str(game_root),
        json_array(&blueprint.countries.iter().map(|item| item.id.clone()).collect::<Vec<_>>()),
        json_array(&blueprint.regions.iter().map(|item| item.id.clone()).collect::<Vec<_>>()),
        json_array(&blueprint.systems.iter().map(|item| item.id.clone()).collect::<Vec<_>>()),
    )
}

fn large_mod_readme(blueprint: &LargeModBlueprint) -> String {
    format!(
        "# {}\n\nThis project was initialized from a `hoi4skill` large-mod blueprint.\n\n- Acronym: `{}`\n- Default language: `{}`\n- Countries: {}\n- Regions: {}\n- Systems: {}\n\nUse `.hoi4skill/large_mod_blueprint.yml` as the source of truth before generating gameplay content.\n",
        blueprint.name,
        blueprint.acronym,
        blueprint.default_language,
        blueprint.countries.len(),
        blueprint.regions.len(),
        blueprint.systems.len()
    )
}

fn work_package_manifest_json(blueprint: &LargeModBlueprint, packages: &[WorkPackage]) -> String {
    let package_json = packages
        .iter()
        .map(|package| {
            format!(
                "{{\"id\": {}, \"kind\": {}, \"title\": {}}}",
                json_str(&package.id),
                json_str(&package.kind),
                json_str(&package.title)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{{\n  \"schema\": \"hoi4skill.large_mod_work_packages.v1\",\n  \"mod\": {},\n  \"acronym\": {},\n  \"package_count\": {},\n  \"packages\": [{}]\n}}\n",
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        packages.len(),
        package_json
    )
}

fn work_package_plan_json(
    blueprint: &LargeModBlueprint,
    package: &WorkPackage,
    blueprint_path: &Path,
    mod_root: Option<&Path>,
) -> String {
    let package_token = package_token(package);
    let tag = package_tag(package);
    let namespace = package_namespace(package, blueprint);
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let preflight_commands =
        work_package_preflight_commands(package, &root, tag.as_deref(), &namespace);
    let recommended_generators = work_package_recommended_generators(package, tag.as_deref());
    let planned_files = work_package_planned_files(package, &root, tag.as_deref(), &package_token);
    let code_authoring_contract = work_package_code_authoring_contract_json();
    let stop_conditions = vec![
        "Do not create country tags, country history, state history, initial units, characters, English localisation, GUI, technologies, or map data unless the literal user request authorizes them.".to_string(),
        "Before writing gameplay script, gather local game/dependency evidence and keep final validation strict-code-index clean.".to_string(),
        "Missing user-provided player-visible text is unfinished work, not an acceptable partial result.".to_string(),
        "Use Rust writers such as apply-focus-layout, apply-feature-cards, and apply-event-cards for final script emission.".to_string(),
    ];

    format!(
        "{{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true,\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"package\": {{\n    \"id\": {},\n    \"kind\": {},\n    \"title\": {},\n    \"token\": {},\n    \"tag\": {},\n    \"namespace\": {}\n  }},\n  \"allowed_paths\": {},\n  \"deliverables\": {},\n  \"code_authoring_contract\": {},\n  \"preflight_commands\": {},\n  \"recommended_generators\": {},\n  \"planned_files\": {},\n  \"validation_steps\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        json_str(&package.id),
        json_str(&package.kind),
        json_str(&package.title),
        json_str(&package_token),
        json_optional_str(tag.as_deref()),
        json_str(&namespace),
        json_array(&package.allowed_paths),
        json_array(&package.deliverables),
        code_authoring_contract,
        json_array(&preflight_commands),
        json_array(&recommended_generators),
        json_array(&planned_files),
        json_array(&package.validation_steps),
        json_array(&stop_conditions),
    )
}

fn work_package_code_authoring_contract_json() -> String {
    let allowed_model_outputs = vec![
        "intent text".to_string(),
        "structured focus layout".to_string(),
        "feature cards".to_string(),
        "event cards".to_string(),
        "player-visible localisation text".to_string(),
    ];
    let forbidden_model_outputs = vec![
        "raw Clausewitz blocks not produced by Rust writers".to_string(),
        "unindexed effects, triggers, modifiers, buildings, resources, technologies, sprites, or tags".to_string(),
        "placeholder ids such as <idea id>, <event id>, <number>, or TODO code".to_string(),
        "new country tags, history files, units, characters, GUI, technologies, or English localisation without literal authorization".to_string(),
    ];
    let required_commands = vec![
        "hoi4skill code-catalog --game-root <HOI4 root> [--mod-path <dependency>] --output .hoi4skill/code_catalog.json".to_string(),
        "hoi4skill compile-intent --text <intent> --kind auto --game-root <HOI4 root> [--mod-path <dependency>] --strict-code-index".to_string(),
        "hoi4skill apply-focus-layout|apply-feature-cards|apply-event-cards ... --game-root <HOI4 root> [--mod-path <dependency>] --final-check".to_string(),
        "hoi4skill validate <mod-root> --game-root <HOI4 root> [--mod-path <dependency>] --strict-code-index --changed-only".to_string(),
    ];
    let blocking_conditions = vec![
        "safety.final_code_allowed is false".to_string(),
        "safety.blockers is non-empty".to_string(),
        "code index category for a required effect, trigger, modifier, resource, building, technology, sprite, or tag is empty".to_string(),
        "check-code-symbol reports ok=false for any generated symbol".to_string(),
        "text alignment misses user-provided player-visible titles or descriptions".to_string(),
    ];
    format!(
        "{{\"schema\": {}, \"final_code_allowed\": false, \"model_role\": {}, \"writer_role\": {}, \"allowed_model_outputs\": {}, \"forbidden_model_outputs\": {}, \"required_commands\": {}, \"blocking_conditions\": {}}}",
        json_str("hoi4skill.code_authoring_contract.v1"),
        json_str("Produce intent, structure, and player-facing text only."),
        json_str("Rust hoi4skill writers assemble Clausewitz code after local code-catalog and strict final checks."),
        json_array(&allowed_model_outputs),
        json_array(&forbidden_model_outputs),
        json_array(&required_commands),
        json_array(&blocking_conditions),
    )
}

fn work_package_start_brief_markdown(
    blueprint: &LargeModBlueprint,
    package: &WorkPackage,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
) -> String {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let package_token = package_token(package);
    let tag = package_tag(package);
    let namespace = package_namespace(package, blueprint);
    let start_state = work_package_start_state(package, packages, mod_root);
    let preflight_commands =
        work_package_preflight_commands(package, &root, tag.as_deref(), &namespace);
    let generator_commands = work_package_recommended_generators(package, tag.as_deref());
    let planned_files = work_package_planned_files(package, &root, tag.as_deref(), &package_token);

    let mut out = String::new();
    out.push_str(&format!(
        "# Work Package Start Brief: {}\n\n",
        package.title
    ));
    out.push_str("- schema: `hoi4skill.work_package_start_brief.v1`\n");
    out.push_str(&format!("- state: `{}`\n", start_state.state));
    out.push_str(&format!("- package: `{}`\n", package.id));
    out.push_str(&format!("- kind: `{}`\n", package.kind));
    out.push_str(&format!("- mod: `{}`\n", blueprint.name));
    out.push_str(&format!("- acronym: `{}`\n", blueprint.acronym));
    out.push_str(&format!("- mod_root: `{}`\n", root));
    out.push_str(&format!("- blueprint: `{}`\n", blueprint_path.display()));
    out.push_str(&format!("- token: `{}`\n", package_token));
    out.push_str(&format!("- tag: `{}`\n", tag.as_deref().unwrap_or("none")));
    out.push_str(&format!("- namespace: `{}`\n", namespace));

    out.push_str("\n## Dependency Gate\n\n");
    if start_state.dependencies.is_empty() {
        out.push_str("- No package dependencies.\n");
    } else {
        out.push_str(&format!(
            "- depends_on: `{}`\n",
            start_state.dependencies.join("`, `")
        ));
    }
    if start_state.blocked_by.is_empty() {
        out.push_str("- dependency_state: `clear`\n");
    } else {
        out.push_str(&format!(
            "- dependency_state: `blocked` by `{}`\n",
            start_state.blocked_by.join("`, `")
        ));
    }

    out.push_str("\n## Allowed Edit Surface\n\n");
    for path in work_package_boundary_allowed_prefixes(package) {
        out.push_str(&format!("- `{path}`\n"));
    }

    out.push_str("\n## Identity Terms\n\n");
    for term in work_package_identity_terms(package, blueprint) {
        out.push_str(&format!("- `{term}`\n"));
    }

    out.push_str("\n## Deliverables\n\n");
    for item in &package.deliverables {
        out.push_str(&format!("- {item}\n"));
    }

    out.push_str("\n## Planned Files\n\n");
    for path in planned_files {
        out.push_str(&format!("- `{path}`\n"));
    }

    out.push_str("\n## Preflight Commands\n\n");
    for command in preflight_commands {
        out.push_str(&format!("- `{command}`\n"));
    }

    out.push_str("\n## Generator Commands\n\n");
    for command in generator_commands {
        out.push_str(&format!("- `{command}`\n"));
    }

    out.push_str("\n## Code Authoring Contract\n\n");
    out.push_str("- AI outputs intent, structured layouts/cards, and player-visible text only.\n");
    out.push_str("- Rust `hoi4skill` writers emit final Clausewitz script.\n");
    out.push_str("- Run `hoi4skill code-catalog --game-root <HOI4 root> [--mod-path <dependency>]` before accepting generated code symbols.\n");
    out.push_str("- Use `hoi4skill compile-intent --kind auto --game-root <HOI4 root> --strict-code-index` for shorthand effects before writing.\n");
    out.push_str("- Stop if `safety.final_code_allowed` is false, `safety.blockers` is non-empty, a code-index category is empty, or `check-code-symbol` returns `ok=false`.\n");

    out.push_str("\n## Package Commands\n\n");
    for command in large_mod_execution_queue_package_commands(package, &root) {
        out.push_str(&format!("- `{command}`\n"));
    }

    out.push_str("\n## Stop Conditions\n\n");
    out.push_str("- Do not start while `state` is `blocked_by_dependencies`.\n");
    out.push_str(
        "- Do not write outside the allowed edit surface without updating boundary evidence.\n",
    );
    out.push_str("- Do not create country tags, country history, state history, initial units, characters, English localisation, GUI, technologies, or map data unless the literal user request authorizes them.\n");
    out.push_str("- Before final script output, run changed-only validation and keep strict-code-index clean.\n");
    out
}

struct WorkPackageStartState {
    state: String,
    dependencies: Vec<String>,
    blocked_by: Vec<String>,
}

fn work_package_start_state(
    package: &WorkPackage,
    packages: &[WorkPackage],
    mod_root: Option<&Path>,
) -> WorkPackageStartState {
    let dependencies = work_package_dependency_ids(package, packages);
    let blocked_by = dependencies
        .iter()
        .filter(|dependency| {
            packages
                .iter()
                .find(|candidate| &candidate.id == *dependency)
                .map(|dependency_package| {
                    let summary = work_package_readiness_summary(dependency_package, mod_root);
                    !work_package_completion_ready(&summary)
                })
                .unwrap_or(true)
        })
        .cloned()
        .collect::<Vec<_>>();
    let summary = work_package_readiness_summary(package, mod_root);
    let state = if !blocked_by.is_empty() {
        "blocked_by_dependencies"
    } else if work_package_completion_ready(&summary) {
        "already_handed_off"
    } else if !summary.blocking.is_empty() {
        "needs_review"
    } else {
        "ready_to_start"
    };
    WorkPackageStartState {
        state: state.to_string(),
        dependencies,
        blocked_by,
    }
}

fn write_work_package_start_briefs(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    output_dir: &Path,
    ready_only: bool,
) -> Result<String, String> {
    fs::create_dir_all(output_dir).map_err(|e| format!("create {}: {e}", output_dir.display()))?;
    let mut ordered = packages.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        work_package_layer(&left.kind)
            .cmp(&work_package_layer(&right.kind))
            .then_with(|| left.id.cmp(&right.id))
    });
    let mut entries = Vec::new();
    let mut generated_count = 0usize;
    let mut skipped_count = 0usize;
    for package in ordered {
        let start_state = work_package_start_state(package, packages, mod_root);
        let generated = !ready_only || start_state.state == "ready_to_start";
        let path = output_dir.join(format!("start_{}.md", package.id));
        if generated {
            let markdown = work_package_start_brief_markdown(
                blueprint,
                package,
                packages,
                blueprint_path,
                mod_root,
            );
            fs::write(&path, markdown).map_err(|e| format!("write {}: {e}", path.display()))?;
            generated_count += 1;
        } else {
            skipped_count += 1;
        }
        entries.push(format!(
            "{{\n      \"id\": {},\n      \"kind\": {},\n      \"title\": {},\n      \"state\": {},\n      \"generated\": {},\n      \"path\": {},\n      \"blocked_by\": {}\n    }}",
            json_str(&package.id),
            json_str(&package.kind),
            json_str(&package.title),
            json_str(&start_state.state),
            json_bool(generated),
            json_str(&path.display().to_string()),
            json_array(&start_state.blocked_by),
        ));
    }
    let manifest = format!(
        "{{\n  \"schema\": \"hoi4skill.work_package_start_briefs.v1\",\n  \"mod\": {},\n  \"acronym\": {},\n  \"blueprint\": {},\n  \"output_dir\": {},\n  \"ready_only\": {},\n  \"package_count\": {},\n  \"generated_count\": {},\n  \"skipped_count\": {},\n  \"briefs\": [\n{}\n  ],\n  \"stop_conditions\": {}\n}}\n",
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&blueprint_path.display().to_string()),
        json_str(&output_dir.display().to_string()),
        json_bool(ready_only),
        packages.len(),
        generated_count,
        skipped_count,
        entries.join(",\n"),
        json_array(&work_package_start_briefs_stop_conditions()),
    );
    let manifest_path = output_dir.join("manifest.json");
    fs::write(&manifest_path, &manifest)
        .map_err(|e| format!("write {}: {e}", manifest_path.display()))?;
    Ok(manifest)
}

fn work_package_start_briefs_stop_conditions() -> Vec<String> {
    vec![
        "Do not dispatch skipped packages while --ready-only filtering is active.".to_string(),
        "Do not start packages whose generated brief reports blocked_by_dependencies.".to_string(),
        "Treat generated briefs as task boundaries, not permission to write outside package ownership.".to_string(),
    ]
}

fn write_work_package_authoring_pack(
    blueprint: &LargeModBlueprint,
    package: &WorkPackage,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    output_dir: &Path,
) -> Result<String, String> {
    fs::create_dir_all(output_dir).map_err(|e| format!("create {}: {e}", output_dir.display()))?;
    let start_path = output_dir.join("start.md");
    let plan_path = output_dir.join("plan.json");
    let assets_path = output_dir.join("assets.md");
    let context_path = output_dir.join("context.md");
    let manifest_path = output_dir.join("manifest.json");

    fs::write(
        &start_path,
        work_package_start_brief_markdown(blueprint, package, packages, blueprint_path, mod_root),
    )
    .map_err(|e| format!("write {}: {e}", start_path.display()))?;
    fs::write(
        &plan_path,
        work_package_plan_json(blueprint, package, blueprint_path, mod_root),
    )
    .map_err(|e| format!("write {}: {e}", plan_path.display()))?;
    fs::write(
        &assets_path,
        asset_pack_plan_markdown(blueprint, package, blueprint_path, mod_root),
    )
    .map_err(|e| format!("write {}: {e}", assets_path.display()))?;
    fs::write(
        &context_path,
        work_package_authoring_context_markdown(
            blueprint,
            package,
            packages,
            blueprint_path,
            mod_root,
            output_dir,
        ),
    )
    .map_err(|e| format!("write {}: {e}", context_path.display()))?;

    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let start_state = work_package_start_state(package, packages, mod_root);
    let files = [
        ("start_brief", &start_path),
        ("generation_plan", &plan_path),
        ("asset_plan", &assets_path),
        ("authoring_context", &context_path),
    ]
    .iter()
    .map(|(kind, path)| {
        format!(
            "{{\"kind\": {}, \"path\": {}}}",
            json_str(kind),
            json_str(&path.display().to_string())
        )
    })
    .collect::<Vec<_>>()
    .join(", ");
    let commands = vec![
        format!("hoi4skill work-package-authoring-pack --mod-root {root} --package {} --output-dir {}", package.id, output_dir.display()),
        format!("hoi4skill check-work-package-boundary --mod-root {root} --package {} --changed-file .hoi4skill/changed_{}.txt --strict-names --output .hoi4skill/boundary_{}.json", package.id, package.id, package.id),
        format!("hoi4skill work-package-status --mod-root {root} --package {} --output .hoi4skill/status_{}.json", package.id, package.id),
        format!("hoi4skill work-package-handoff --mod-root {root} --package {} --output .hoi4skill/handoff_{}.md", package.id, package.id),
    ];
    let manifest = format!(
        "{{\n  \"schema\": \"hoi4skill.work_package_authoring_pack.v1\",\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"output_dir\": {},\n  \"package\": {{\n    \"id\": {},\n    \"kind\": {},\n    \"title\": {},\n    \"state\": {},\n    \"blocked_by\": {}\n  }},\n  \"files\": [{}],\n  \"next_commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        json_str(&output_dir.display().to_string()),
        json_str(&package.id),
        json_str(&package.kind),
        json_str(&package.title),
        json_str(&start_state.state),
        json_array(&start_state.blocked_by),
        files,
        json_array(&commands),
        json_array(&work_package_authoring_pack_stop_conditions()),
    );
    fs::write(&manifest_path, &manifest)
        .map_err(|e| format!("write {}: {e}", manifest_path.display()))?;
    Ok(manifest)
}

fn work_package_authoring_context_markdown(
    blueprint: &LargeModBlueprint,
    package: &WorkPackage,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    output_dir: &Path,
) -> String {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let start_state = work_package_start_state(package, packages, mod_root);
    let tag = package_tag(package);
    let namespace = package_namespace(package, blueprint);
    let mut out = String::new();
    out.push_str(&format!(
        "# Work Package Authoring Context: {}\n\n",
        package.id
    ));
    out.push_str("- schema: `hoi4skill.work_package_authoring_context.v1`\n");
    out.push_str(&format!("- mod: `{}`\n", blueprint.name));
    out.push_str(&format!("- acronym: `{}`\n", blueprint.acronym));
    out.push_str(&format!("- mod_root: `{}`\n", root));
    out.push_str(&format!("- blueprint: `{}`\n", blueprint_path.display()));
    out.push_str(&format!("- output_dir: `{}`\n", output_dir.display()));
    out.push_str(&format!("- package: `{}`\n", package.id));
    out.push_str(&format!("- kind: `{}`\n", package.kind));
    out.push_str(&format!("- state: `{}`\n", start_state.state));
    out.push_str(&format!("- namespace: `{}`\n", namespace));
    if let Some(tag) = &tag {
        out.push_str(&format!("- tag: `{tag}`\n"));
    }

    out.push_str("\n## Allowed Edit Surface\n\n");
    for path in work_package_boundary_allowed_prefixes(package) {
        out.push_str(&format!("- `{path}`\n"));
    }

    out.push_str("\n## Identity Terms\n\n");
    for term in work_package_identity_terms(package, blueprint) {
        out.push_str(&format!("- `{term}`\n"));
    }

    out.push_str("\n## Dependencies\n\n");
    if start_state.dependencies.is_empty() {
        out.push_str("- No package dependencies.\n");
    } else {
        for dependency in &start_state.dependencies {
            let state = if start_state.blocked_by.contains(dependency) {
                "blocking"
            } else {
                "ready"
            };
            out.push_str(&format!("- `{dependency}`: `{state}`\n"));
        }
    }

    out.push_str("\n## Authoring Files\n\n");
    for file in [
        "start.md",
        "plan.json",
        "assets.md",
        "context.md",
        "manifest.json",
    ] {
        out.push_str(&format!("- `{}`\n", output_dir.join(file).display()));
    }

    out.push_str("\n## Suggested Commands\n\n");
    for command in work_package_preflight_commands(package, &root, tag.as_deref(), &namespace) {
        out.push_str(&format!("- `{command}`\n"));
    }
    out.push_str(&format!(
        "- `hoi4skill check-work-package-boundary --mod-root {root} --package {} --changed-file .hoi4skill/changed_{}.txt --strict-names --output .hoi4skill/boundary_{}.json`\n",
        package.id, package.id, package.id
    ));
    out.push_str(&format!(
        "- `hoi4skill work-package-handoff --mod-root {root} --package {} --output .hoi4skill/handoff_{}.md`\n",
        package.id, package.id
    ));

    out.push_str("\n## Stop Conditions\n\n");
    for condition in work_package_authoring_pack_stop_conditions() {
        out.push_str(&format!("- {condition}\n"));
    }
    out
}

fn work_package_authoring_pack_stop_conditions() -> Vec<String> {
    vec![
        "Do not write outside the allowed edit surface listed in context.md.".to_string(),
        "Do not start authoring while the manifest package state is blocked_by_dependencies."
            .to_string(),
        "Do not treat authoring pack generation as gameplay validation or handoff approval."
            .to_string(),
        "Run boundary, status, handoff, and final validation gates after changing package files."
            .to_string(),
    ]
}

fn work_package_claim_json(
    blueprint: &LargeModBlueprint,
    package: &WorkPackage,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    assignee: &str,
    output: &Path,
) -> String {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let start_state = work_package_start_state(package, packages, mod_root);
    let can_start = start_state.state == "ready_to_start";
    let commands = vec![
        format!(
            "hoi4skill work-package-start-brief --mod-root {root} --package {} --output .hoi4skill/start_{}.md",
            package.id, package.id
        ),
        format!(
            "hoi4skill generate-work-package --mod-root {root} --package {} --dry-run --output .hoi4skill/plan_{}.json",
            package.id, package.id
        ),
        format!(
            "hoi4skill work-package-handoff --mod-root {root} --package {} --output .hoi4skill/handoff_{}.md",
            package.id, package.id
        ),
        format!(
            "hoi4skill work-package-release-claim --mod-root {root} --package {} --released-by <assignee> --reason <reason> --output .hoi4skill/claim_releases/release_{}.json",
            package.id, package.id
        ),
    ];
    format!(
        "{{\n  \"schema\": \"hoi4skill.work_package_claim.v1\",\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"claim_path\": {},\n  \"assignee\": {},\n  \"can_start\": {},\n  \"state\": {},\n  \"package\": {{\n    \"id\": {},\n    \"kind\": {},\n    \"title\": {}\n  }},\n  \"depends_on\": {},\n  \"blocked_by\": {},\n  \"allowed_paths\": {},\n  \"identity_terms\": {},\n  \"commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        json_str(&output.display().to_string()),
        json_str(assignee),
        json_bool(can_start),
        json_str(&start_state.state),
        json_str(&package.id),
        json_str(&package.kind),
        json_str(&package.title),
        json_array(&start_state.dependencies),
        json_array(&start_state.blocked_by),
        json_array(&work_package_boundary_allowed_prefixes(package)),
        json_array(&work_package_identity_terms(package, blueprint)),
        json_array(&commands),
        json_array(&work_package_claim_stop_conditions()),
    )
}

fn work_package_claim_stop_conditions() -> Vec<String> {
    vec![
        "Do not start work when can_start is false.".to_string(),
        "Do not overwrite another active claim without explicit --force and human coordination."
            .to_string(),
        "Do not use a claim as permission to write outside package boundaries.".to_string(),
    ]
}

#[allow(clippy::too_many_arguments)]
fn work_package_claim_release_json(
    blueprint: &LargeModBlueprint,
    package: &WorkPackage,
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    claim_path: &Path,
    release_path: &Path,
    claim_text: &str,
    released_by: &str,
    reason: &str,
) -> String {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let previous_assignee = status_json_string_field(claim_text, "assignee");
    let previous_state = status_json_string_field(claim_text, "state");
    let commands = vec![
        format!(
            "hoi4skill work-package-dispatch-board --mod-root {root} --output .hoi4skill/dispatch_board.md"
        ),
        format!(
            "hoi4skill work-package-claims --mod-root {root} --output .hoi4skill/claims.json"
        ),
        format!(
            "hoi4skill work-package-claim --mod-root {root} --package {} --assignee <assignee>",
            package.id
        ),
    ];
    format!(
        "{{\n  \"schema\": \"hoi4skill.work_package_claim_release.v1\",\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"package\": {{\n    \"id\": {},\n    \"kind\": {},\n    \"title\": {}\n  }},\n  \"claim_path\": {},\n  \"release_path\": {},\n  \"previous_assignee\": {},\n  \"previous_state\": {},\n  \"released_by\": {},\n  \"reason\": {},\n  \"commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        json_str(&package.id),
        json_str(&package.kind),
        json_str(&package.title),
        json_str(&claim_path.display().to_string()),
        json_str(&release_path.display().to_string()),
        json_optional_str(previous_assignee.as_deref()),
        json_optional_str(previous_state.as_deref()),
        json_str(released_by),
        json_str(reason),
        json_array(&commands),
        json_array(&work_package_claim_release_stop_conditions()),
    )
}

fn work_package_claim_release_stop_conditions() -> Vec<String> {
    vec![
        "Do not release another assignee's claim without a recorded coordination reason."
            .to_string(),
        "Regenerate the dispatch board after claim release.".to_string(),
        "Do not treat release as package completion or handoff evidence.".to_string(),
    ]
}

struct WorkPackageClaimSummary {
    id: String,
    kind: String,
    title: String,
    claim_status: String,
    current_state: String,
    claim_state: Option<String>,
    assignee: Option<String>,
    claim_path: PathBuf,
    blocked_by: Vec<String>,
    summary: Vec<String>,
}

fn work_package_claim_summary(
    package: &WorkPackage,
    packages: &[WorkPackage],
    mod_root: Option<&Path>,
    claims_dir: &Path,
) -> WorkPackageClaimSummary {
    let claim_path = claims_dir.join(format!("claim_{}.json", package.id));
    let start_state = work_package_start_state(package, packages, mod_root);
    let mut assignee = None;
    let mut claim_state = None;
    let mut claim_status = "unclaimed".to_string();
    let mut summary = Vec::new();
    if claim_path.exists() {
        match read_utf8_lossy(&claim_path) {
            Ok(text) => {
                assignee = status_json_string_field(&text, "assignee");
                claim_state = status_json_string_field(&text, "state");
                if start_state.state == "blocked_by_dependencies"
                    || start_state.state == "needs_review"
                {
                    claim_status = "blocked_claim".to_string();
                    summary.push(format!("current_state={}", start_state.state));
                } else {
                    claim_status = "claimed".to_string();
                }
            }
            Err(err) => {
                claim_status = "needs_review".to_string();
                summary.push(err);
            }
        }
    } else {
        summary.push(format!("current_state={}", start_state.state));
    }
    WorkPackageClaimSummary {
        id: package.id.clone(),
        kind: package.kind.clone(),
        title: package.title.clone(),
        claim_status,
        current_state: start_state.state,
        claim_state,
        assignee,
        claim_path,
        blocked_by: start_state.blocked_by,
        summary,
    }
}

fn work_package_claims_json(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    claims_dir: &Path,
) -> String {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let mut claimed_count = 0usize;
    let mut unclaimed_count = 0usize;
    let mut stale_or_blocked_count = 0usize;
    let package_json = packages
        .iter()
        .map(|package| {
            let claim = work_package_claim_summary(package, packages, mod_root, claims_dir);
            if claim.claim_path.exists() {
                claimed_count += 1;
            } else {
                unclaimed_count += 1;
            }
            if claim.claim_status == "blocked_claim" || claim.claim_status == "needs_review" {
                stale_or_blocked_count += 1;
            }
            format!(
                "{{\n      \"id\": {},\n      \"kind\": {},\n      \"title\": {},\n      \"claim_status\": {},\n      \"current_state\": {},\n      \"claim_state\": {},\n      \"assignee\": {},\n      \"claim_path\": {},\n      \"blocked_by\": {},\n      \"summary\": {}\n    }}",
                json_str(&claim.id),
                json_str(&claim.kind),
                json_str(&claim.title),
                json_str(&claim.claim_status),
                json_str(&claim.current_state),
                json_optional_str(claim.claim_state.as_deref()),
                json_optional_str(claim.assignee.as_deref()),
                json_str(&claim.claim_path.display().to_string()),
                json_array(&claim.blocked_by),
                json_array(&claim.summary),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let next_commands = vec![
        format!("hoi4skill large-mod-execution-queue --mod-root {root} --output .hoi4skill/execution_queue.json"),
        format!("hoi4skill work-package-start-briefs --mod-root {root} --ready-only --output-dir .hoi4skill/start_briefs --output .hoi4skill/start_briefs_manifest.json"),
        format!("hoi4skill work-package-claims --mod-root {root} --output .hoi4skill/claims.json"),
        format!("hoi4skill work-package-release-claim --mod-root {root} --package <package_id> --released-by <assignee> --reason <reason>"),
    ];
    format!(
        "{{\n  \"schema\": \"hoi4skill.work_package_claims.v1\",\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"claims_dir\": {},\n  \"package_count\": {},\n  \"claimed_count\": {},\n  \"unclaimed_count\": {},\n  \"stale_or_blocked_count\": {},\n  \"packages\": [\n{}\n  ],\n  \"next_commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        json_str(&claims_dir.display().to_string()),
        packages.len(),
        claimed_count,
        unclaimed_count,
        stale_or_blocked_count,
        package_json,
        json_array(&next_commands),
        json_array(&work_package_claims_stop_conditions()),
    )
}

fn work_package_claims_stop_conditions() -> Vec<String> {
    vec![
        "Do not dispatch unclaimed packages without a claim artifact.".to_string(),
        "Re-check blocked_claim entries before continuing work.".to_string(),
        "Treat claim files as coordination records, not as permission to bypass package boundaries."
            .to_string(),
    ]
}

fn work_package_dispatch_board_markdown(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    claims_dir: &Path,
) -> String {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let claims = packages
        .iter()
        .map(|package| work_package_claim_summary(package, packages, mod_root, claims_dir))
        .collect::<Vec<_>>();
    let claimed_count = claims
        .iter()
        .filter(|claim| claim.claim_path.exists())
        .count();
    let unclaimed_count = claims.len().saturating_sub(claimed_count);
    let blocked_claim_count = claims
        .iter()
        .filter(|claim| {
            claim.claim_status == "blocked_claim" || claim.claim_status == "needs_review"
        })
        .count();
    let ready_unclaimed_count = claims
        .iter()
        .filter(|claim| {
            claim.claim_status == "unclaimed" && claim.current_state == "ready_to_start"
        })
        .count();
    let mut out = String::new();
    out.push_str(&format!(
        "# Work Package Dispatch Board: {}\n\n",
        blueprint.name
    ));
    out.push_str("- schema: `hoi4skill.work_package_dispatch_board.v1`\n");
    out.push_str(&format!("- acronym: `{}`\n", blueprint.acronym));
    out.push_str(&format!("- mod_root: `{}`\n", root));
    out.push_str(&format!("- blueprint: `{}`\n", blueprint_path.display()));
    out.push_str(&format!("- claims_dir: `{}`\n", claims_dir.display()));
    out.push_str(&format!(
        "- packages: `{}` claimed, `{}` unclaimed, `{}` blocked/stale, `{}` ready-unclaimed\n",
        claimed_count, unclaimed_count, blocked_claim_count, ready_unclaimed_count
    ));

    out.push_str("\n## Dispatch Table\n\n");
    out.push_str("| Package | Kind | Assignee | Claim | Current | Blocked By | Claim Path |\n");
    out.push_str("| --- | --- | --- | --- | --- | --- | --- |\n");
    for claim in &claims {
        out.push_str(&format!(
            "| `{}` | `{}` | {} | `{}` | `{}` | {} | `{}` |\n",
            claim.id,
            claim.kind,
            markdown_table_cell(claim.assignee.as_deref().unwrap_or("unassigned")),
            claim.claim_status,
            claim.current_state,
            markdown_table_cell(&claim.blocked_by.join(", ")),
            claim.claim_path.display()
        ));
    }

    out.push_str("\n## Ready To Claim\n\n");
    let ready_unclaimed = claims
        .iter()
        .filter(|claim| {
            claim.claim_status == "unclaimed" && claim.current_state == "ready_to_start"
        })
        .collect::<Vec<_>>();
    if ready_unclaimed.is_empty() {
        out.push_str("- No unclaimed package is currently ready to start.\n");
    } else {
        for claim in ready_unclaimed {
            out.push_str(&format!(
                "- `{}`: `hoi4skill work-package-claim --mod-root {} --package {} --assignee <assignee>`\n",
                claim.id, root, claim.id
            ));
        }
    }

    out.push_str("\n## Claim Maintenance\n\n");
    let claimed_packages = claims
        .iter()
        .filter(|claim| claim.claim_path.exists())
        .collect::<Vec<_>>();
    if claimed_packages.is_empty() {
        out.push_str("- No active claim needs release or reassignment.\n");
    } else {
        for claim in claimed_packages {
            out.push_str(&format!(
                "- `{}`: `hoi4skill work-package-release-claim --mod-root {} --package {} --released-by <assignee> --reason <reason>`\n",
                claim.id, root, claim.id
            ));
        }
    }

    out.push_str("\n## Next Commands\n\n");
    for command in [
        format!("hoi4skill work-package-claims --mod-root {root} --output .hoi4skill/claims.json"),
        format!("hoi4skill work-package-dispatch-board --mod-root {root} --output .hoi4skill/dispatch_board.md"),
        format!("hoi4skill large-mod-execution-queue --mod-root {root} --output .hoi4skill/execution_queue.json"),
        format!("hoi4skill work-package-release-claim --mod-root {root} --package <package_id> --released-by <assignee> --reason <reason>"),
    ] {
        out.push_str(&format!("- `{command}`\n"));
    }

    out.push_str("\n## Stop Conditions\n\n");
    out.push_str("- Do not assign a package whose current state is `blocked_by_dependencies`.\n");
    out.push_str("- Resolve `blocked_claim` rows before continuing claimed work.\n");
    out.push_str("- A dispatch board is coordination evidence, not permission to bypass package boundaries.\n");
    out
}

fn large_mod_dispatch_gate_json(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    claims_dir: &Path,
    allow_unclaimed: bool,
) -> String {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let claims = packages
        .iter()
        .map(|package| work_package_claim_summary(package, packages, mod_root, claims_dir))
        .collect::<Vec<_>>();
    let mut claim_count = 0usize;
    let mut ready_unclaimed_count = 0usize;
    let mut blocked_claim_count = 0usize;
    let mut stale_claim_count = 0usize;
    let mut needs_review_count = 0usize;
    let mut blocking_count = 0usize;
    let package_json = claims
        .iter()
        .map(|claim| {
            let has_claim = claim.claim_path.exists();
            if has_claim {
                claim_count += 1;
            }
            let mut blockers = Vec::new();
            let dispatch_status = if claim.claim_status == "needs_review" {
                needs_review_count += 1;
                blockers.push("claim_needs_review".to_string());
                "needs_review"
            } else if claim.claim_status == "blocked_claim" {
                blocked_claim_count += 1;
                blockers.push("claimed_package_is_not_ready".to_string());
                "blocked_claim"
            } else if claim.claim_status == "claimed" && claim.current_state == "already_handed_off"
            {
                stale_claim_count += 1;
                blockers.push("claim_exists_after_handoff".to_string());
                "stale_claim"
            } else if claim.claim_status == "unclaimed" && claim.current_state == "ready_to_start" {
                ready_unclaimed_count += 1;
                if !allow_unclaimed {
                    blockers.push("ready_package_unclaimed".to_string());
                }
                "ready_unclaimed"
            } else if claim.claim_status == "claimed" {
                "claimed"
            } else {
                "waiting"
            };
            blocking_count += blockers.len();
            format!(
                "{{\n      \"id\": {},\n      \"kind\": {},\n      \"title\": {},\n      \"dispatch_status\": {},\n      \"claim_status\": {},\n      \"current_state\": {},\n      \"assignee\": {},\n      \"claim_path\": {},\n      \"blocked_by\": {},\n      \"blockers\": {},\n      \"summary\": {}\n    }}",
                json_str(&claim.id),
                json_str(&claim.kind),
                json_str(&claim.title),
                json_str(dispatch_status),
                json_str(&claim.claim_status),
                json_str(&claim.current_state),
                json_optional_str(claim.assignee.as_deref()),
                json_str(&claim.claim_path.display().to_string()),
                json_array(&claim.blocked_by),
                json_array(&blockers),
                json_array(&claim.summary),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let dispatchable = blocking_count == 0;
    let next_commands = vec![
        format!("hoi4skill work-package-claims --mod-root {root} --output .hoi4skill/claims.json"),
        format!("hoi4skill work-package-dispatch-board --mod-root {root} --output .hoi4skill/dispatch_board.md"),
        format!("hoi4skill large-mod-dispatch-gate --mod-root {root} --output .hoi4skill/dispatch_gate.json"),
        format!("hoi4skill work-package-claim --mod-root {root} --package <package_id> --assignee <assignee>"),
        format!("hoi4skill work-package-release-claim --mod-root {root} --package <package_id> --released-by <assignee> --reason <reason>"),
    ];
    format!(
        "{{\n  \"schema\": \"hoi4skill.large_mod_dispatch_gate.v1\",\n  \"dispatchable\": {},\n  \"allow_unclaimed\": {},\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"claims_dir\": {},\n  \"package_count\": {},\n  \"claim_count\": {},\n  \"ready_unclaimed_count\": {},\n  \"blocked_claim_count\": {},\n  \"stale_claim_count\": {},\n  \"needs_review_count\": {},\n  \"blocking_count\": {},\n  \"packages\": [\n{}\n  ],\n  \"next_commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_bool(dispatchable),
        json_bool(allow_unclaimed),
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        json_str(&claims_dir.display().to_string()),
        packages.len(),
        claim_count,
        ready_unclaimed_count,
        blocked_claim_count,
        stale_claim_count,
        needs_review_count,
        blocking_count,
        package_json,
        json_array(&next_commands),
        json_array(&large_mod_dispatch_gate_stop_conditions()),
    )
}

fn large_mod_dispatch_gate_stop_conditions() -> Vec<String> {
    vec![
        "Do not dispatch ready packages that have no active claim unless --allow-unclaimed is explicit."
            .to_string(),
        "Do not continue blocked_claim or needs_review packages without refreshing dependencies and claims."
            .to_string(),
        "Release stale claims after package handoff before treating the dispatch board as current."
            .to_string(),
    ]
}

#[derive(Clone, Debug)]
struct WorkPackageReportStatus {
    path: PathBuf,
    schema: Option<String>,
    status: String,
    summary: Vec<String>,
}

fn collect_work_package_status_reports(
    mod_root: Option<&Path>,
    map: &ArgMap,
) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for report in repeated_values(map, "report") {
        paths.push(normalize_path(report)?);
    }
    if let Some(root) = mod_root {
        let hoi4skill_dir = root.join(".hoi4skill");
        for name in [
            "mod_index.json",
            "loc_audit.json",
            "loc_sync.json",
            "gfx_audit.json",
            "logic_audit.json",
            "error_log_report.json",
            "error_log.json",
            "validation.json",
            "regression_gate.json",
        ] {
            let path = hoi4skill_dir.join(name);
            if path.exists() {
                paths.push(path);
            }
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn collect_large_mod_release_gate_reports(
    mod_root: Option<&Path>,
    map: &ArgMap,
) -> Result<Vec<PathBuf>, String> {
    let mut paths = collect_work_package_status_reports(mod_root, map)?;
    if let Some(root) = mod_root {
        let hoi4skill_dir = root.join(".hoi4skill");
        if hoi4skill_dir.is_dir() {
            for entry in fs::read_dir(&hoi4skill_dir)
                .map_err(|e| format!("read {}: {e}", hoi4skill_dir.display()))?
            {
                let entry = entry.map_err(|e| format!("read {}: {e}", hoi4skill_dir.display()))?;
                let path = entry.path();
                let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                    continue;
                };
                if path.extension().and_then(|ext| ext.to_str()) == Some("json")
                    && (name.starts_with("boundary_") || name.starts_with("status_"))
                {
                    paths.push(path);
                }
            }
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn collect_large_mod_playtest_reports(
    mod_root: Option<&Path>,
    map: &ArgMap,
) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for report in repeated_values(map, "report") {
        paths.push(normalize_path(report)?);
    }
    if let Some(root) = mod_root {
        let hoi4skill_dir = root.join(".hoi4skill");
        if hoi4skill_dir.is_dir() {
            for entry in fs::read_dir(&hoi4skill_dir)
                .map_err(|e| format!("read {}: {e}", hoi4skill_dir.display()))?
            {
                let entry = entry.map_err(|e| format!("read {}: {e}", hoi4skill_dir.display()))?;
                let path = entry.path();
                let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                    continue;
                };
                if path.extension().and_then(|ext| ext.to_str()) == Some("json")
                    && name.starts_with("playtest_")
                {
                    paths.push(path);
                }
            }
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn collect_large_mod_fix_queue_reports(
    mod_root: Option<&Path>,
    map: &ArgMap,
) -> Result<Vec<PathBuf>, String> {
    let mut paths = collect_large_mod_release_gate_reports(mod_root, map)?;
    if let Some(root) = mod_root {
        let hoi4skill_dir = root.join(".hoi4skill");
        if hoi4skill_dir.is_dir() {
            for entry in fs::read_dir(&hoi4skill_dir)
                .map_err(|e| format!("read {}: {e}", hoi4skill_dir.display()))?
            {
                let entry = entry.map_err(|e| format!("read {}: {e}", hoi4skill_dir.display()))?;
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                    continue;
                }
                let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                    continue;
                };
                if name.starts_with("validation_")
                    || name.starts_with("error_log_")
                    || name.starts_with("playtest_")
                    || name.starts_with("merge_gate_")
                    || name.starts_with("review_")
                    || matches!(
                        name,
                        "validation.json"
                            | "error_log.json"
                            | "error_log_report.json"
                            | "loc_audit.json"
                            | "loc_sync.json"
                            | "gfx_audit.json"
                            | "logic_audit.json"
                            | "playtest_gate.json"
                            | "merge_gate.json"
                            | "release_gate.json"
                    )
                {
                    paths.push(path);
                }
            }
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn work_package_status_json(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    reports: &[PathBuf],
) -> Result<String, String> {
    let report_statuses = reports
        .iter()
        .map(|path| read_work_package_report_status(path))
        .collect::<Result<Vec<_>, _>>()?;
    let needs_review = report_statuses
        .iter()
        .any(|report| report.status == "needs_review");
    let status = if needs_review { "needs_review" } else { "ok" };
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let package_json = packages
        .iter()
        .map(|package| {
            let package_status = work_package_status(package, mod_root, needs_review);
            let next_commands = work_package_status_next_commands(package, &root);
            format!(
                "{{\n      \"id\": {},\n      \"kind\": {},\n      \"title\": {},\n      \"status\": {},\n      \"allowed_paths\": {},\n      \"deliverables\": {},\n      \"validation_steps\": {},\n      \"next_commands\": {}\n    }}",
                json_str(&package.id),
                json_str(&package.kind),
                json_str(&package.title),
                json_str(&package_status),
                json_array(&package.allowed_paths),
                json_array(&package.deliverables),
                json_array(&package.validation_steps),
                json_array(&next_commands),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let report_json = report_statuses
        .iter()
        .map(|report| {
            format!(
                "{{\n      \"path\": {},\n      \"schema\": {},\n      \"status\": {},\n      \"summary\": {}\n    }}",
                json_str(&report.path.display().to_string()),
                json_optional_str(report.schema.as_deref()),
                json_str(&report.status),
                json_array(&report.summary),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let recommended_commands = work_package_status_recommended_commands(&root);
    Ok(format!(
        "{{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": {},\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"package_count\": {},\n  \"report_count\": {},\n  \"packages\": [\n{}\n  ],\n  \"report_files\": [\n{}\n  ],\n  \"recommended_next_commands\": {}\n}}\n",
        json_str(status),
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        packages.len(),
        report_statuses.len(),
        package_json,
        report_json,
        json_array(&recommended_commands),
    ))
}

fn work_package_status(
    package: &WorkPackage,
    mod_root: Option<&Path>,
    reports_need_review: bool,
) -> String {
    if reports_need_review {
        return "needs_review".to_string();
    }
    if let Some(root) = mod_root {
        let package_path = root
            .join(".hoi4skill")
            .join("work_packages")
            .join(format!("{}.md", package.id));
        if package_path.exists() {
            return "scaffolded".to_string();
        }
    }
    "planned".to_string()
}

fn work_package_status_next_commands(package: &WorkPackage, mod_root: &str) -> Vec<String> {
    vec![
        format!(
            "hoi4skill work-package-start-brief --mod-root {mod_root} --package {} --output .hoi4skill/start_{}.md",
            package.id, package.id
        ),
        format!(
            "hoi4skill generate-work-package --mod-root {mod_root} --package {} --dry-run --output .hoi4skill/plan_{}.json",
            package.id, package.id
        ),
        format!(
            "hoi4skill asset-pack-plan --mod-root {mod_root} --package {} --output .hoi4skill/assets_{}.md",
            package.id, package.id
        ),
        format!(
            "hoi4skill validate {mod_root} --changed-only --changed <planned-file> --strict-code-index --output .hoi4skill/validation_{}.json",
            package.id
        ),
        format!(
            "hoi4skill work-package-handoff --mod-root {mod_root} --package {} --output .hoi4skill/handoff_{}.md",
            package.id, package.id
        ),
    ]
}

fn work_package_status_recommended_commands(mod_root: &str) -> Vec<String> {
    vec![
        format!("hoi4skill split-work-packages --mod-root {mod_root}"),
        format!("hoi4skill build-mod-index {mod_root} --output .hoi4skill/mod_index.json"),
        format!("hoi4skill loc-audit {mod_root} --output .hoi4skill/loc_audit.json"),
        format!("hoi4skill gfx-audit {mod_root} --output .hoi4skill/gfx_audit.json"),
        format!("hoi4skill logic-audit {mod_root} --output .hoi4skill/logic_audit.json"),
        format!(
            "hoi4skill work-package-status --mod-root {mod_root} --output .hoi4skill/work_package_status.json"
        ),
        format!("hoi4skill large-mod-dispatch-gate --mod-root {mod_root} --output .hoi4skill/dispatch_gate.json"),
    ]
}

fn collect_boundary_changed_paths(map: &ArgMap) -> Result<Vec<String>, String> {
    let mut changed = repeated_values(map, "changed")
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if let Some(changed_file) = value(map, "changed-file") {
        let path = normalize_path(changed_file)?;
        let text = read_utf8_lossy(&path)?;
        changed.extend(
            text.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(str::to_string),
        );
    }
    changed.sort();
    changed.dedup();
    if changed.is_empty() {
        return Err("missing --changed or --changed-file".to_string());
    }
    Ok(changed)
}

fn identify_work_packages_json(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    changed: &[String],
    strict_names: bool,
) -> String {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let mut assigned_count = 0usize;
    let mut unassigned_count = 0usize;
    let mut ambiguous_count = 0usize;
    let mut package_ids = BTreeSet::new();
    let changed_json = changed
        .iter()
        .map(|raw| {
            let normalized = normalize_boundary_path(raw, mod_root);
            let matches = packages
                .iter()
                .filter_map(|package| {
                    work_package_match_for_path(&normalized, package, blueprint, strict_names).map(
                        |allowed_by| {
                            package_ids.insert(package.id.clone());
                            format!(
                                "{{\"id\": {}, \"kind\": {}, \"title\": {}, \"allowed_by\": {}}}",
                                json_str(&package.id),
                                json_str(&package.kind),
                                json_str(&package.title),
                                json_str(&allowed_by)
                            )
                        },
                    )
                })
                .collect::<Vec<_>>();
            let status = match matches.len() {
                0 => {
                    unassigned_count += 1;
                    "unassigned"
                }
                1 => {
                    assigned_count += 1;
                    "assigned"
                }
                _ => {
                    ambiguous_count += 1;
                    "ambiguous"
                }
            };
            format!(
                "{{\n      \"path\": {},\n      \"normalized\": {},\n      \"status\": {},\n      \"matches\": [{}]\n    }}",
                json_str(raw),
                json_str(&normalized),
                json_str(status),
                matches.join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let next_commands = package_ids
        .iter()
        .map(|id| {
            format!(
                "hoi4skill check-work-package-boundary --mod-root {root} --package {id} --changed-file .hoi4skill/changed_{id}.txt --strict-names --output .hoi4skill/boundary_{id}.json"
            )
        })
        .collect::<Vec<_>>();
    format!(
        "{{\n  \"schema\": \"hoi4skill.changed_work_packages.v1\",\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"strict_names\": {},\n  \"package_count\": {},\n  \"changed_count\": {},\n  \"assigned_count\": {},\n  \"unassigned_count\": {},\n  \"ambiguous_count\": {},\n  \"affected_packages\": {},\n  \"changed_files\": [\n{}\n  ],\n  \"next_commands\": {}\n}}\n",
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        json_bool(strict_names),
        packages.len(),
        changed.len(),
        assigned_count,
        unassigned_count,
        ambiguous_count,
        json_array(&package_ids.into_iter().collect::<Vec<_>>()),
        changed_json,
        json_array(&next_commands),
    )
}

fn split_changed_work_packages_json(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    output_dir: &Path,
    changed: &[String],
    strict_names: bool,
) -> Result<String, String> {
    fs::create_dir_all(output_dir).map_err(|e| format!("create {}: {e}", output_dir.display()))?;
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let mut assignments: BTreeMap<String, Vec<String>> = packages
        .iter()
        .map(|package| (package.id.clone(), Vec::new()))
        .collect();
    let mut unassigned = Vec::new();
    let mut ambiguous = Vec::new();
    for raw in changed {
        let normalized = normalize_boundary_path(raw, mod_root);
        let matches = packages
            .iter()
            .filter(|package| {
                work_package_match_for_path(&normalized, package, blueprint, strict_names).is_some()
            })
            .map(|package| package.id.clone())
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [] => unassigned.push(normalized),
            [id] => assignments.entry(id.clone()).or_default().push(normalized),
            _ => ambiguous.push(format!("{} -> {}", normalized, matches.join(", "))),
        }
    }
    for files in assignments.values_mut() {
        files.sort();
        files.dedup();
    }
    unassigned.sort();
    unassigned.dedup();
    ambiguous.sort();
    ambiguous.dedup();

    let mut affected_package_count = 0usize;
    let mut generated = Vec::new();
    for package in packages {
        let files = assignments.get(&package.id).cloned().unwrap_or_default();
        if !files.is_empty() {
            affected_package_count += 1;
        }
        let path = output_dir.join(format!("changed_{}.txt", package.id));
        fs::write(&path, changed_list_text(&files))
            .map_err(|e| format!("write {}: {e}", path.display()))?;
        generated.push((package.id.clone(), path, files.len()));
    }
    let unassigned_path = output_dir.join("changed_unassigned.txt");
    fs::write(&unassigned_path, changed_list_text(&unassigned))
        .map_err(|e| format!("write {}: {e}", unassigned_path.display()))?;
    let ambiguous_path = output_dir.join("changed_ambiguous.txt");
    fs::write(&ambiguous_path, changed_list_text(&ambiguous))
        .map_err(|e| format!("write {}: {e}", ambiguous_path.display()))?;

    let generated_json = generated
        .iter()
        .map(|(id, path, count)| {
            format!(
                "{{\"package\": {}, \"path\": {}, \"changed_count\": {}}}",
                json_str(id),
                json_str(&path.display().to_string()),
                count
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let next_commands = generated
        .iter()
        .filter(|(_, _, count)| *count > 0)
        .map(|(id, path, _)| {
            format!(
                "hoi4skill check-work-package-boundary --mod-root {root} --package {id} --changed-file {} --strict-names --output .hoi4skill/boundary_{id}.json",
                path.display()
            )
        })
        .collect::<Vec<_>>();
    Ok(format!(
        "{{\n  \"schema\": \"hoi4skill.split_changed_work_packages.v1\",\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"output_dir\": {},\n  \"strict_names\": {},\n  \"package_count\": {},\n  \"changed_count\": {},\n  \"affected_package_count\": {},\n  \"unassigned_count\": {},\n  \"ambiguous_count\": {},\n  \"generated_files\": [{}],\n  \"unassigned_file\": {},\n  \"ambiguous_file\": {},\n  \"next_commands\": {}\n}}\n",
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        json_str(&output_dir.display().to_string()),
        json_bool(strict_names),
        packages.len(),
        changed.len(),
        affected_package_count,
        unassigned.len(),
        ambiguous.len(),
        generated_json,
        json_str(&unassigned_path.display().to_string()),
        json_str(&ambiguous_path.display().to_string()),
        json_array(&next_commands),
    ))
}

fn work_package_readiness_json(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
) -> Result<String, String> {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let mut ready_count = 0usize;
    let mut blocked_count = 0usize;
    let mut missing_package_count = 0usize;
    let package_json = packages
        .iter()
        .map(|package| {
            let artifacts = work_package_readiness_artifacts(package, mod_root);
            let mut missing = Vec::new();
            let mut blocking = Vec::new();
            let artifact_json = artifacts
                .iter()
                .map(|artifact| {
                    let exists = artifact.path.exists();
                    if !exists {
                        missing.push(artifact.label.to_string());
                    }
                    let mut status = if exists { "present" } else { "missing" }.to_string();
                    let mut summary = Vec::new();
                    if exists && artifact.report_like {
                        match read_work_package_report_status(&artifact.path) {
                            Ok(report) => {
                                status = report.status;
                                summary = report.summary;
                                if status == "needs_review" {
                                    blocking.push(artifact.label.to_string());
                                }
                            }
                            Err(err) => {
                                status = "needs_review".to_string();
                                summary.push(err);
                                blocking.push(artifact.label.to_string());
                            }
                        }
                    }
                    format!(
                        "{{\"label\": {}, \"path\": {}, \"exists\": {}, \"status\": {}, \"summary\": {}}}",
                        json_str(artifact.label),
                        json_str(&artifact.path.display().to_string()),
                        json_bool(exists),
                        json_str(&status),
                        json_array(&summary)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            let ready = missing.is_empty() && blocking.is_empty();
            if ready {
                ready_count += 1;
            } else {
                blocked_count += 1;
            }
            if !missing.is_empty() {
                missing_package_count += 1;
            }
            format!(
                "{{\n      \"id\": {},\n      \"kind\": {},\n      \"title\": {},\n      \"ready\": {},\n      \"missing_artifacts\": {},\n      \"blocking_artifacts\": {},\n      \"artifacts\": [{}]\n    }}",
                json_str(&package.id),
                json_str(&package.kind),
                json_str(&package.title),
                json_bool(ready),
                json_array(&missing),
                json_array(&blocking),
                artifact_json
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let next_commands = vec![
        format!("hoi4skill large-mod-ci-plan --mod-root {root} --output .hoi4skill/ci_plan.json"),
        format!("hoi4skill split-changed-work-packages --mod-root {root} --changed-file <changed-files.txt> --strict-names --output .hoi4skill/split_changed.json"),
        format!("hoi4skill work-package-readiness --mod-root {root} --output .hoi4skill/readiness.json"),
        format!("hoi4skill large-mod-dispatch-gate --mod-root {root} --output .hoi4skill/dispatch_gate.json"),
        format!("hoi4skill large-mod-release-gate --mod-root {root} --output .hoi4skill/release_gate.json"),
    ];
    Ok(format!(
        "{{\n  \"schema\": \"hoi4skill.work_package_readiness.v1\",\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"package_count\": {},\n  \"ready_count\": {},\n  \"blocked_count\": {},\n  \"missing_package_count\": {},\n  \"packages\": [\n{}\n  ],\n  \"next_commands\": {}\n}}\n",
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        packages.len(),
        ready_count,
        blocked_count,
        missing_package_count,
        package_json,
        json_array(&next_commands),
    ))
}

struct WorkPackageArtifact {
    label: &'static str,
    path: PathBuf,
    report_like: bool,
}

fn work_package_readiness_artifacts(
    package: &WorkPackage,
    mod_root: Option<&Path>,
) -> Vec<WorkPackageArtifact> {
    let base = mod_root
        .map(|root| root.join(".hoi4skill"))
        .unwrap_or_else(|| PathBuf::from(".hoi4skill"));
    vec![
        WorkPackageArtifact {
            label: "changed",
            path: base.join(format!("changed_{}.txt", package.id)),
            report_like: false,
        },
        WorkPackageArtifact {
            label: "plan",
            path: base.join(format!("plan_{}.json", package.id)),
            report_like: true,
        },
        WorkPackageArtifact {
            label: "assets",
            path: base.join(format!("assets_{}.md", package.id)),
            report_like: false,
        },
        WorkPackageArtifact {
            label: "boundary",
            path: base.join(format!("boundary_{}.json", package.id)),
            report_like: true,
        },
        WorkPackageArtifact {
            label: "status",
            path: base.join(format!("status_{}.json", package.id)),
            report_like: true,
        },
        WorkPackageArtifact {
            label: "validation",
            path: base.join(format!("validation_{}.json", package.id)),
            report_like: true,
        },
    ]
}

#[derive(Debug)]
struct WorkPackageReadinessSummary {
    id: String,
    kind: String,
    title: String,
    ready: bool,
    missing: Vec<String>,
    blocking: Vec<String>,
    changed_path: PathBuf,
    handoff_path: PathBuf,
}

fn work_package_readiness_summary(
    package: &WorkPackage,
    mod_root: Option<&Path>,
) -> WorkPackageReadinessSummary {
    let mut missing = Vec::new();
    let mut blocking = Vec::new();
    let artifacts = work_package_readiness_artifacts(package, mod_root);
    for artifact in &artifacts {
        if !artifact.path.exists() {
            missing.push(artifact.label.to_string());
            continue;
        }
        if artifact.report_like {
            match read_work_package_report_status(&artifact.path) {
                Ok(report) => {
                    if report.status == "needs_review" {
                        blocking.push(artifact.label.to_string());
                    }
                }
                Err(_) => blocking.push(artifact.label.to_string()),
            }
        }
    }
    let base = mod_root
        .map(|root| root.join(".hoi4skill"))
        .unwrap_or_else(|| PathBuf::from(".hoi4skill"));
    WorkPackageReadinessSummary {
        id: package.id.clone(),
        kind: package.kind.clone(),
        title: package.title.clone(),
        ready: missing.is_empty() && blocking.is_empty(),
        missing,
        blocking,
        changed_path: base.join(format!("changed_{}.txt", package.id)),
        handoff_path: base.join(format!("handoff_{}.md", package.id)),
    }
}

fn large_mod_dashboard_markdown(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    extra_reports: &[PathBuf],
) -> Result<String, String> {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let base = mod_root
        .map(|root| root.join(".hoi4skill"))
        .unwrap_or_else(|| PathBuf::from(".hoi4skill"));
    let package_summaries = packages
        .iter()
        .map(|package| work_package_readiness_summary(package, mod_root))
        .collect::<Vec<_>>();
    let ready_count = package_summaries
        .iter()
        .filter(|package| package.ready)
        .count();
    let blocked_count = package_summaries.len().saturating_sub(ready_count);
    let required_reports = large_mod_required_release_reports();
    let mut report_rows = Vec::new();
    let mut missing_required_count = 0usize;
    let mut blocking_report_count = 0usize;
    for name in &required_reports {
        let path = base.join(name);
        if !path.exists() {
            missing_required_count += 1;
            report_rows.push((name.clone(), path, "missing".to_string(), String::new()));
            continue;
        }
        match read_work_package_report_status(&path) {
            Ok(report) => {
                if report.status == "needs_review" {
                    blocking_report_count += 1;
                }
                report_rows.push((name.clone(), path, report.status, report.summary.join("; ")));
            }
            Err(err) => {
                blocking_report_count += 1;
                report_rows.push((name.clone(), path, "needs_review".to_string(), err));
            }
        }
    }
    let mut extra_report_rows = Vec::new();
    for path in extra_reports {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if required_reports.iter().any(|required| required == name) {
            continue;
        }
        match read_work_package_report_status(path) {
            Ok(report) => {
                if report.status == "needs_review" {
                    blocking_report_count += 1;
                }
                extra_report_rows.push((
                    name.to_string(),
                    path.to_path_buf(),
                    report.status,
                    report.summary.join("; "),
                ));
            }
            Err(err) => {
                blocking_report_count += 1;
                extra_report_rows.push((
                    name.to_string(),
                    path.to_path_buf(),
                    "needs_review".to_string(),
                    err,
                ));
            }
        }
    }
    let releasable =
        missing_required_count == 0 && blocking_report_count == 0 && blocked_count == 0;

    let mut out = String::new();
    out.push_str(&format!("# Large Mod Dashboard: {}\n\n", blueprint.name));
    out.push_str("- schema: `hoi4skill.large_mod_dashboard.v1`\n");
    out.push_str(&format!("- acronym: `{}`\n", blueprint.acronym));
    out.push_str(&format!("- mod_root: `{}`\n", root));
    out.push_str(&format!("- blueprint: `{}`\n", blueprint_path.display()));
    out.push_str(&format!(
        "- release_ready: `{}`\n",
        if releasable { "yes" } else { "no" }
    ));
    out.push_str(&format!(
        "- packages: `{}` ready, `{}` blocked, `{}` total\n",
        ready_count,
        blocked_count,
        package_summaries.len()
    ));
    out.push_str(&format!(
        "- reports: `{}` missing required, `{}` blocking\n",
        missing_required_count, blocking_report_count
    ));

    out.push_str("\n## Work Packages\n\n");
    out.push_str("| Package | Title | Kind | State | Missing | Blocking | Changed | Handoff |\n");
    out.push_str("| --- | --- | --- | --- | --- | --- | --- | --- |\n");
    for package in &package_summaries {
        out.push_str(&format!(
            "| `{}` | {} | `{}` | `{}` | {} | {} | `{}` | `{}` |\n",
            package.id,
            markdown_table_cell(&package.title),
            package.kind,
            if package.ready { "ready" } else { "blocked" },
            markdown_table_cell(&package.missing.join(", ")),
            markdown_table_cell(&package.blocking.join(", ")),
            package.changed_path.display(),
            package.handoff_path.display()
        ));
    }

    out.push_str("\n## Required Reports\n\n");
    out.push_str("| Report | Status | Path | Summary |\n");
    out.push_str("| --- | --- | --- | --- |\n");
    for (name, path, status, summary) in &report_rows {
        out.push_str(&format!(
            "| `{}` | `{}` | `{}` | {} |\n",
            name,
            status,
            path.display(),
            markdown_table_cell(summary)
        ));
    }

    if !extra_report_rows.is_empty() {
        out.push_str("\n## Package Reports\n\n");
        out.push_str("| Report | Status | Path | Summary |\n");
        out.push_str("| --- | --- | --- | --- |\n");
        for (name, path, status, summary) in &extra_report_rows {
            out.push_str(&format!(
                "| `{}` | `{}` | `{}` | {} |\n",
                name,
                status,
                path.display(),
                markdown_table_cell(summary)
            ));
        }
    }

    out.push_str("\n## Next Commands\n\n");
    for command in large_mod_dashboard_next_commands(&root) {
        out.push_str(&format!("- `{command}`\n"));
    }

    out.push_str("\n## Stop Conditions\n\n");
    out.push_str("- Do not release while any required report is missing or needs review.\n");
    out.push_str("- Do not release while any work package is blocked or missing artifacts.\n");
    out.push_str(
        "- Use package handoff files before assigning package work to another author or agent.\n",
    );
    Ok(out)
}

fn large_mod_dashboard_next_commands(mod_root: &str) -> Vec<String> {
    vec![
        format!("hoi4skill large-mod-ci-plan --mod-root {mod_root} --output .hoi4skill/ci_plan.json"),
        format!("hoi4skill large-mod-ownership-map --mod-root {mod_root} --output .hoi4skill/ownership_map.json"),
        format!("hoi4skill large-mod-dependency-graph --mod-root {mod_root} --output .hoi4skill/dependency_graph.json"),
        format!("hoi4skill large-mod-milestone-plan --mod-root {mod_root} --output .hoi4skill/milestone_plan.json"),
        format!("hoi4skill large-mod-execution-queue --mod-root {mod_root} --output .hoi4skill/execution_queue.json"),
        format!("hoi4skill work-package-status --mod-root {mod_root} --output .hoi4skill/work_package_status.json"),
        format!("hoi4skill work-package-readiness --mod-root {mod_root} --output .hoi4skill/readiness.json"),
        format!("hoi4skill large-mod-next-actions --mod-root {mod_root} --output .hoi4skill/next_actions.json"),
        format!("hoi4skill large-mod-risk-register --mod-root {mod_root} --output .hoi4skill/risk_register.json"),
        format!("hoi4skill large-mod-dispatch-gate --mod-root {mod_root} --output .hoi4skill/dispatch_gate.json"),
        format!("hoi4skill large-mod-release-gate --mod-root {mod_root} --output .hoi4skill/release_gate.json"),
        format!("hoi4skill large-mod-dashboard --mod-root {mod_root} --output .hoi4skill/dashboard.md"),
        format!("hoi4skill large-mod-evidence-pack --mod-root {mod_root} --output .hoi4skill/evidence_pack.json"),
        format!("hoi4skill large-mod-review-brief --mod-root {mod_root} --output .hoi4skill/review_brief.md"),
        format!("hoi4skill large-mod-release-bundle --mod-root {mod_root} --output .hoi4skill/release_bundle.json"),
        format!("hoi4skill large-mod-playtest-plan --mod-root {mod_root} --output .hoi4skill/playtest_plan.json"),
    ]
}

fn large_mod_dependency_graph_json(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
) -> String {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let mut packages_by_layer = packages.iter().collect::<Vec<_>>();
    packages_by_layer.sort_by(|left, right| {
        work_package_layer(&left.kind)
            .cmp(&work_package_layer(&right.kind))
            .then_with(|| left.id.cmp(&right.id))
    });
    let package_ids = packages
        .iter()
        .map(|package| package.id.clone())
        .collect::<Vec<_>>();
    let node_json = packages_by_layer
        .iter()
        .map(|package| {
            let dependencies = work_package_dependency_ids(package, packages);
            let dependents = package_ids
                .iter()
                .filter(|id| {
                    packages
                        .iter()
                        .find(|candidate| &candidate.id == *id)
                        .map(|candidate| {
                            work_package_dependency_ids(candidate, packages)
                                .iter()
                                .any(|dependency| dependency == &package.id)
                        })
                        .unwrap_or(false)
                })
                .cloned()
                .collect::<Vec<_>>();
            format!(
                "{{\n      \"id\": {},\n      \"kind\": {},\n      \"title\": {},\n      \"layer\": {},\n      \"layer_name\": {},\n      \"depends_on\": {},\n      \"unlocks\": {},\n      \"allowed_paths\": {}\n    }}",
                json_str(&package.id),
                json_str(&package.kind),
                json_str(&package.title),
                work_package_layer(&package.kind),
                json_str(&work_package_layer_name(&package.kind)),
                json_array(&dependencies),
                json_array(&dependents),
                json_array(&package.allowed_paths),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let mut edges = Vec::new();
    for package in &packages_by_layer {
        for dependency in work_package_dependency_ids(package, packages) {
            edges.push(work_package_dependency_edge_json(package, &dependency));
        }
    }
    let execution_layers = ["system", "country", "region"]
        .iter()
        .map(|kind| {
            let ids = packages_by_layer
                .iter()
                .filter(|package| package.kind == *kind)
                .map(|package| package.id.clone())
                .collect::<Vec<_>>();
            format!(
                "{{\n      \"layer\": {},\n      \"name\": {},\n      \"packages\": {}\n    }}",
                work_package_layer(kind),
                json_str(&work_package_layer_name(kind)),
                json_array(&ids),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let next_commands = vec![
        format!("hoi4skill large-mod-dependency-graph --mod-root {root} --output .hoi4skill/dependency_graph.json"),
        format!("hoi4skill large-mod-ci-plan --mod-root {root} --strict-names --output .hoi4skill/ci_plan.json"),
        format!("hoi4skill large-mod-next-actions --mod-root {root} --output .hoi4skill/next_actions.json"),
    ];
    format!(
        "{{\n  \"schema\": \"hoi4skill.large_mod_dependency_graph.v1\",\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"package_count\": {},\n  \"edge_count\": {},\n  \"cycle_count\": 0,\n  \"nodes\": [\n{}\n  ],\n  \"edges\": [\n{}\n  ],\n  \"execution_layers\": [\n{}\n  ],\n  \"next_commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        packages.len(),
        edges.len(),
        node_json,
        edges.join(",\n"),
        execution_layers,
        json_array(&next_commands),
        json_array(&large_mod_dependency_graph_stop_conditions()),
    )
}

fn work_package_layer(kind: &str) -> usize {
    match kind {
        "system" => 1,
        "country" => 2,
        "region" => 3,
        _ => 9,
    }
}

fn work_package_layer_name(kind: &str) -> String {
    match kind {
        "system" => "system_contracts",
        "country" => "country_content",
        "region" => "regional_integration",
        _ => "unknown",
    }
    .to_string()
}

fn work_package_dependency_ids(package: &WorkPackage, packages: &[WorkPackage]) -> Vec<String> {
    packages
        .iter()
        .filter(|candidate| match package.kind.as_str() {
            "country" => candidate.kind == "system",
            "region" => candidate.kind == "system" || candidate.kind == "country",
            _ => false,
        })
        .map(|candidate| candidate.id.clone())
        .collect()
}

fn work_package_dependency_edge_json(package: &WorkPackage, dependency: &str) -> String {
    let reason = match package.kind.as_str() {
        "country" => "country package depends on shared system contracts",
        "region" if dependency.starts_with("system_") => {
            "regional integration depends on shared system contracts"
        }
        "region" => "regional integration depends on country package surfaces",
        _ => "package dependency",
    };
    format!(
        "{{\n      \"package\": {},\n      \"depends_on\": {},\n      \"reason\": {}\n    }}",
        json_str(&package.id),
        json_str(dependency),
        json_str(reason),
    )
}

fn large_mod_dependency_graph_stop_conditions() -> Vec<String> {
    vec![
        "Do not schedule a package before its dependency layer has status and handoff evidence."
            .to_string(),
        "Do not use the dependency graph to expand a package edit surface.".to_string(),
        "Do not treat the graph as gameplay evidence; still run final strict-code-index validation."
            .to_string(),
    ]
}

fn large_mod_milestone_plan_json(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
) -> String {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let milestones = large_mod_effective_milestones(blueprint);
    let milestone_json = milestones
        .iter()
        .enumerate()
        .map(|(idx, milestone)| {
            let phase = large_mod_milestone_phase(idx);
            let package_ids = packages
                .iter()
                .filter(|package| large_mod_package_matches_milestone_phase(package, phase))
                .map(|package| package.id.clone())
                .collect::<Vec<_>>();
            format!(
                "{{\n      \"index\": {},\n      \"id\": {},\n      \"title\": {},\n      \"phase\": {},\n      \"packages\": {},\n      \"required_reports\": {},\n      \"commands\": {},\n      \"exit_criteria\": {}\n    }}",
                idx + 1,
                json_str(&slugify(milestone, &format!("milestone_{}", idx + 1))),
                json_str(milestone),
                json_str(phase),
                json_array(&package_ids),
                json_array(&large_mod_milestone_required_reports(phase)),
                json_array(&large_mod_milestone_commands(phase, &root, packages)),
                json_array(&large_mod_milestone_exit_criteria(phase)),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let next_commands = vec![
        format!("hoi4skill large-mod-milestone-plan --mod-root {root} --output .hoi4skill/milestone_plan.json"),
        format!("hoi4skill large-mod-dependency-graph --mod-root {root} --output .hoi4skill/dependency_graph.json"),
        format!("hoi4skill large-mod-ci-plan --mod-root {root} --strict-names --output .hoi4skill/ci_plan.json"),
    ];
    format!(
        "{{\n  \"schema\": \"hoi4skill.large_mod_milestone_plan.v1\",\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"package_count\": {},\n  \"milestone_count\": {},\n  \"milestones\": [\n{}\n  ],\n  \"next_commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        packages.len(),
        milestones.len(),
        milestone_json,
        json_array(&next_commands),
        json_array(&large_mod_milestone_stop_conditions()),
    )
}

fn large_mod_effective_milestones(blueprint: &LargeModBlueprint) -> Vec<String> {
    let defaults = [
        "system_contracts",
        "country_content_pass",
        "regional_integration_pass",
        "localisation_and_asset_pass",
        "playtest_regression",
    ];
    let mut milestones = blueprint.milestones.clone();
    if milestones.is_empty() {
        milestones = defaults.iter().map(|value| value.to_string()).collect();
    }
    while milestones.len() < defaults.len() {
        milestones.push(defaults[milestones.len()].to_string());
    }
    milestones
}

fn large_mod_milestone_phase(index: usize) -> &'static str {
    match index {
        0 => "system_contracts",
        1 => "country_content",
        2 => "regional_integration",
        3 => "localisation_assets",
        _ => "regression_release",
    }
}

fn large_mod_package_matches_milestone_phase(package: &WorkPackage, phase: &str) -> bool {
    matches!(
        (phase, package.kind.as_str()),
        ("system_contracts", "system")
            | ("country_content", "country")
            | ("regional_integration", "region")
    )
}

fn large_mod_milestone_required_reports(phase: &str) -> Vec<String> {
    match phase {
        "system_contracts" => vec![
            "ownership_map.json".to_string(),
            "dependency_graph.json".to_string(),
            "ci_plan.json".to_string(),
        ],
        "country_content" => vec![
            "work_package_status.json".to_string(),
            "readiness.json".to_string(),
        ],
        "regional_integration" => vec![
            "logic_audit.json".to_string(),
            "work_package_status.json".to_string(),
            "readiness.json".to_string(),
        ],
        "localisation_assets" => vec![
            "loc_audit.json".to_string(),
            "gfx_audit.json".to_string(),
            "evidence_pack.json".to_string(),
        ],
        _ => vec![
            "validation.json".to_string(),
            "release_gate.json".to_string(),
            "review_brief.md".to_string(),
        ],
    }
}

fn large_mod_milestone_commands(
    phase: &str,
    mod_root: &str,
    packages: &[WorkPackage],
) -> Vec<String> {
    match phase {
        "system_contracts" => vec![
            format!("hoi4skill large-mod-ownership-map --mod-root {mod_root} --output .hoi4skill/ownership_map.json"),
            format!("hoi4skill large-mod-dependency-graph --mod-root {mod_root} --output .hoi4skill/dependency_graph.json"),
            format!("hoi4skill large-mod-ci-plan --mod-root {mod_root} --strict-names --output .hoi4skill/ci_plan.json"),
        ],
        "country_content" | "regional_integration" => packages
            .iter()
            .filter(|package| large_mod_package_matches_milestone_phase(package, phase))
            .flat_map(|package| {
                vec![
                    format!(
                        "hoi4skill generate-work-package --mod-root {mod_root} --package {} --dry-run --output .hoi4skill/plan_{}.json",
                        package.id, package.id
                    ),
                    format!(
                        "hoi4skill work-package-handoff --mod-root {mod_root} --package {} --output .hoi4skill/handoff_{}.md",
                        package.id, package.id
                    ),
                ]
            })
            .chain(vec![
                format!("hoi4skill work-package-status --mod-root {mod_root} --output .hoi4skill/work_package_status.json"),
                format!("hoi4skill work-package-readiness --mod-root {mod_root} --output .hoi4skill/readiness.json"),
            ])
            .collect(),
        "localisation_assets" => vec![
            format!("hoi4skill loc-audit {mod_root} --output .hoi4skill/loc_audit.json"),
            format!("hoi4skill gfx-audit {mod_root} --output .hoi4skill/gfx_audit.json"),
            format!("hoi4skill large-mod-evidence-pack --mod-root {mod_root} --output .hoi4skill/evidence_pack.json"),
        ],
        _ => vec![
            format!("hoi4skill validate {mod_root} --strict-code-index --output .hoi4skill/validation.json"),
            format!("hoi4skill large-mod-release-gate --mod-root {mod_root} --output .hoi4skill/release_gate.json"),
            format!("hoi4skill large-mod-review-brief --mod-root {mod_root} --output .hoi4skill/review_brief.md"),
            format!("hoi4skill large-mod-release-bundle --mod-root {mod_root} --output .hoi4skill/release_bundle.json"),
            format!("hoi4skill large-mod-playtest-plan --mod-root {mod_root} --output .hoi4skill/playtest_plan.json"),
        ],
    }
}

fn large_mod_milestone_exit_criteria(phase: &str) -> Vec<String> {
    match phase {
        "system_contracts" => vec![
            "Ownership map exists before parallel package work starts.".to_string(),
            "Dependency graph has no cycles and lists every package.".to_string(),
        ],
        "country_content" => vec![
            "Every country package has plan, changed-file, boundary, status, validation, and handoff artifacts.".to_string(),
            "No country package is blocked in readiness.".to_string(),
        ],
        "regional_integration" => vec![
            "Regional packages only integrate existing country and system surfaces.".to_string(),
            "Logic audit has no blocking focus, event, or trigger reference issues.".to_string(),
        ],
        "localisation_assets" => vec![
            "Localisation and GFX audits have no missing release-blocking references.".to_string(),
            "Evidence pack includes required reports and package artifacts.".to_string(),
        ],
        _ => vec![
            "Final validation uses strict-code-index against the local game/dependency codebase.".to_string(),
            "Release gate is releasable and review brief decision is release_ready.".to_string(),
        ],
    }
}

fn large_mod_milestone_stop_conditions() -> Vec<String> {
    vec![
        "Do not advance a milestone while any listed required report is missing.".to_string(),
        "Do not schedule later package layers before dependency predecessors have handoff evidence."
            .to_string(),
        "Do not use milestone planning as permission to create unrequested countries, history, GUI, technologies, or characters.".to_string(),
    ]
}

fn large_mod_execution_queue_json(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
) -> String {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let summaries = packages
        .iter()
        .map(|package| {
            (
                package.id.clone(),
                work_package_readiness_summary(package, mod_root),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut queued = packages.iter().collect::<Vec<_>>();
    queued.sort_by(|left, right| {
        work_package_layer(&left.kind)
            .cmp(&work_package_layer(&right.kind))
            .then_with(|| left.id.cmp(&right.id))
    });
    let mut completed_count = 0usize;
    let mut ready_to_start_count = 0usize;
    let mut blocked_count = 0usize;
    let mut needs_review_count = 0usize;
    let package_json = queued
        .iter()
        .enumerate()
        .map(|(idx, package)| {
            let summary = summaries.get(&package.id).expect("summary exists");
            let dependencies = work_package_dependency_ids(package, packages);
            let blocked_by = dependencies
                .iter()
                .filter(|dependency| {
                    summaries
                        .get(*dependency)
                        .map(|summary| !work_package_completion_ready(summary))
                        .unwrap_or(true)
                })
                .cloned()
                .collect::<Vec<_>>();
            let status = if !blocked_by.is_empty() {
                blocked_count += 1;
                "blocked_by_dependencies"
            } else if work_package_completion_ready(summary) {
                completed_count += 1;
                "completed"
            } else if !summary.blocking.is_empty() {
                needs_review_count += 1;
                "needs_review"
            } else {
                ready_to_start_count += 1;
                "ready_to_start"
            };
            format!(
                "{{\n      \"queue_index\": {},\n      \"id\": {},\n      \"kind\": {},\n      \"title\": {},\n      \"layer\": {},\n      \"status\": {},\n      \"depends_on\": {},\n      \"blocked_by\": {},\n      \"missing_artifacts\": {},\n      \"blocking_artifacts\": {},\n      \"handoff\": {},\n      \"next_commands\": {}\n    }}",
                idx + 1,
                json_str(&package.id),
                json_str(&package.kind),
                json_str(&package.title),
                work_package_layer(&package.kind),
                json_str(status),
                json_array(&dependencies),
                json_array(&blocked_by),
                json_array(&summary.missing),
                json_array(&summary.blocking),
                json_str(&summary.handoff_path.display().to_string()),
                json_array(&large_mod_execution_queue_package_commands(package, &root)),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let next_commands = vec![
        format!("hoi4skill large-mod-execution-queue --mod-root {root} --output .hoi4skill/execution_queue.json"),
        format!("hoi4skill work-package-start-briefs --mod-root {root} --ready-only --output-dir .hoi4skill/start_briefs --output .hoi4skill/start_briefs_manifest.json"),
        format!("hoi4skill work-package-claims --mod-root {root} --output .hoi4skill/claims.json"),
        format!("hoi4skill work-package-dispatch-board --mod-root {root} --output .hoi4skill/dispatch_board.md"),
        format!("hoi4skill large-mod-dispatch-gate --mod-root {root} --output .hoi4skill/dispatch_gate.json"),
        format!("hoi4skill work-package-readiness --mod-root {root} --output .hoi4skill/readiness.json"),
        format!("hoi4skill large-mod-dashboard --mod-root {root} --output .hoi4skill/dashboard.md"),
    ];
    format!(
        "{{\n  \"schema\": \"hoi4skill.large_mod_execution_queue.v1\",\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"package_count\": {},\n  \"completed_count\": {},\n  \"ready_to_start_count\": {},\n  \"blocked_count\": {},\n  \"needs_review_count\": {},\n  \"packages\": [\n{}\n  ],\n  \"next_commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        packages.len(),
        completed_count,
        ready_to_start_count,
        blocked_count,
        needs_review_count,
        package_json,
        json_array(&next_commands),
        json_array(&large_mod_execution_queue_stop_conditions()),
    )
}

fn work_package_completion_ready(summary: &WorkPackageReadinessSummary) -> bool {
    summary.ready && summary.handoff_path.exists()
}

fn large_mod_execution_queue_package_commands(
    package: &WorkPackage,
    mod_root: &str,
) -> Vec<String> {
    vec![
        format!(
            "hoi4skill work-package-claim --mod-root {mod_root} --package {} --assignee <assignee> --output .hoi4skill/claims/claim_{}.json",
            package.id, package.id
        ),
        format!(
            "hoi4skill work-package-start-brief --mod-root {mod_root} --package {} --output .hoi4skill/start_{}.md",
            package.id, package.id
        ),
        format!(
            "hoi4skill generate-work-package --mod-root {mod_root} --package {} --dry-run --output .hoi4skill/plan_{}.json",
            package.id, package.id
        ),
        format!(
            "hoi4skill work-package-handoff --mod-root {mod_root} --package {} --output .hoi4skill/handoff_{}.md",
            package.id, package.id
        ),
        format!(
            "hoi4skill work-package-readiness --mod-root {mod_root} --package {} --output .hoi4skill/readiness_{}.json",
            package.id, package.id
        ),
        format!(
            "hoi4skill work-package-review-checklist --mod-root {mod_root} --package {} --output .hoi4skill/review_checklist_{}.md",
            package.id, package.id
        ),
    ]
}

fn large_mod_execution_queue_stop_conditions() -> Vec<String> {
    vec![
        "Do not start a package while status is blocked_by_dependencies.".to_string(),
        "Do not mark a dependency complete without both readiness success and handoff markdown."
            .to_string(),
        "Do not use queue order to expand a package edit surface beyond ownership and boundary reports."
            .to_string(),
    ]
}

fn large_mod_next_actions_json(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    extra_reports: &[PathBuf],
) -> Result<String, String> {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let base = mod_root
        .map(|root| root.join(".hoi4skill"))
        .unwrap_or_else(|| PathBuf::from(".hoi4skill"));
    let required_reports = large_mod_required_release_reports();
    let mut priority = 1usize;
    let mut blocking_count = 0usize;
    let mut actions = Vec::new();

    for name in &required_reports {
        let path = base.join(name);
        if !path.exists() {
            blocking_count += 1;
            actions.push(large_mod_action_json(
                priority,
                true,
                "global",
                None,
                "missing_required_report",
                &format!("required report `{name}` is missing"),
                &path,
                &large_mod_required_report_command(name, &root),
            ));
            priority += 1;
            continue;
        }
        let report = read_work_package_report_status(&path)?;
        if report.status == "needs_review" {
            blocking_count += 1;
            actions.push(large_mod_action_json(
                priority,
                true,
                "global",
                None,
                "report_needs_review",
                &report.summary.join("; "),
                &path,
                &large_mod_required_report_command(name, &root),
            ));
            priority += 1;
        }
    }

    for path in extra_reports {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if required_reports.iter().any(|required| required == name) {
            continue;
        }
        let report = read_work_package_report_status(path)?;
        if report.status == "needs_review" {
            blocking_count += 1;
            actions.push(large_mod_action_json(
                priority,
                true,
                "package_report",
                None,
                "report_needs_review",
                &report.summary.join("; "),
                path,
                &large_mod_report_rerun_command(name, &root),
            ));
            priority += 1;
        }
    }

    for package in packages {
        let summary = work_package_readiness_summary(package, mod_root);
        for missing in &summary.missing {
            let path = work_package_artifact_path(package, mod_root, missing);
            blocking_count += 1;
            actions.push(large_mod_action_json(
                priority,
                true,
                "work_package",
                Some(&package.id),
                "missing_package_artifact",
                &format!("missing `{missing}` artifact for `{}`", package.id),
                &path,
                &work_package_artifact_command(package, missing, &root),
            ));
            priority += 1;
        }
        for blocking in &summary.blocking {
            let path = work_package_artifact_path(package, mod_root, blocking);
            blocking_count += 1;
            actions.push(large_mod_action_json(
                priority,
                true,
                "work_package",
                Some(&package.id),
                "package_artifact_needs_review",
                &format!("`{blocking}` artifact needs review for `{}`", package.id),
                &path,
                &work_package_artifact_command(package, blocking, &root),
            ));
            priority += 1;
        }
        if summary.ready && !summary.handoff_path.exists() {
            actions.push(large_mod_action_json(
                priority,
                false,
                "work_package",
                Some(&package.id),
                "handoff_missing",
                &format!("ready package `{}` has no handoff markdown", package.id),
                &summary.handoff_path,
                &format!(
                    "hoi4skill work-package-handoff --mod-root {root} --package {} --output .hoi4skill/handoff_{}.md",
                    package.id, package.id
                ),
            ));
            priority += 1;
        }
    }

    if actions.is_empty() {
        actions.push(large_mod_action_json(
            priority,
            false,
            "release",
            None,
            "release_gate",
            "no blocking next actions found; run the final release gate",
            &base.join("release_gate.json"),
            &format!(
                "hoi4skill large-mod-release-gate --mod-root {root} --output .hoi4skill/release_gate.json"
            ),
        ));
    }
    let action_json = actions.join(",\n");
    Ok(format!(
        "{{\n  \"schema\": \"hoi4skill.large_mod_next_actions.v1\",\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"action_count\": {},\n  \"blocking_count\": {},\n  \"actions\": [\n{}\n  ],\n  \"stop_conditions\": {}\n}}\n",
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        actions.len(),
        blocking_count,
        action_json,
        json_array(&large_mod_next_action_stop_conditions()),
    ))
}

#[derive(Clone, Debug)]
struct LargeModProductionPackage {
    id: String,
    kind: String,
    title: String,
    stage: String,
    ready: bool,
    handoff: bool,
    claim_status: String,
    assignee: Option<String>,
    missing: Vec<String>,
    blocking: Vec<String>,
    blocked_by: Vec<String>,
}

#[derive(Clone, Debug)]
struct LargeModProductionReport {
    name: String,
    kind: String,
    path: PathBuf,
    required: bool,
    exists: bool,
    status: String,
    schema: Option<String>,
    summary: Vec<String>,
    blocking: bool,
}

#[derive(Clone, Debug)]
struct LargeModProductionSnapshot {
    mod_name: String,
    acronym: String,
    mod_root: String,
    blueprint: String,
    claims_dir: String,
    decision: String,
    package_count: usize,
    ready_package_count: usize,
    handoff_count: usize,
    claimed_count: usize,
    blocked_package_count: usize,
    report_count: usize,
    missing_required_report_count: usize,
    blocking_report_count: usize,
    blocking_count: usize,
    packages: Vec<LargeModProductionPackage>,
    reports: Vec<LargeModProductionReport>,
    next_commands: Vec<String>,
    stop_conditions: Vec<String>,
}

fn large_mod_production_snapshot_state(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    claims_dir: &Path,
    extra_reports: &[PathBuf],
) -> Result<LargeModProductionSnapshot, String> {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let base = mod_root
        .map(|root| root.join(".hoi4skill"))
        .unwrap_or_else(|| PathBuf::from(".hoi4skill"));
    let mut production_packages = Vec::new();
    let mut ready_package_count = 0usize;
    let mut handoff_count = 0usize;
    let mut claimed_count = 0usize;
    let mut blocked_package_count = 0usize;

    for package in packages {
        let readiness = work_package_readiness_summary(package, mod_root);
        let claim = work_package_claim_summary(package, packages, mod_root, claims_dir);
        let handoff = readiness.handoff_path.exists();
        if readiness.ready {
            ready_package_count += 1;
        }
        if handoff {
            handoff_count += 1;
        }
        if claim.claim_path.exists() {
            claimed_count += 1;
        }
        let package_blocking = !readiness.ready
            || claim.claim_status == "blocked_claim"
            || claim.claim_status == "needs_review";
        if package_blocking {
            blocked_package_count += 1;
        }
        let stage = if !readiness.ready {
            "blocked"
        } else if claim.claim_status == "blocked_claim" || claim.claim_status == "needs_review" {
            "claim_blocked"
        } else if handoff {
            "handoff_ready"
        } else if claim.claim_status == "claimed" {
            "in_progress"
        } else {
            "ready_unclaimed"
        };
        production_packages.push(LargeModProductionPackage {
            id: readiness.id,
            kind: readiness.kind,
            title: readiness.title,
            stage: stage.to_string(),
            ready: readiness.ready,
            handoff,
            claim_status: claim.claim_status,
            assignee: claim.assignee,
            missing: readiness.missing,
            blocking: readiness.blocking,
            blocked_by: claim.blocked_by,
        });
    }

    let required_reports = large_mod_required_release_reports();
    let required_set = required_reports.iter().cloned().collect::<BTreeSet<_>>();
    let mut report_paths = Vec::new();
    for name in large_mod_production_report_names() {
        report_paths.push(base.join(name));
    }
    for report in extra_reports {
        report_paths.push(report.to_path_buf());
    }
    report_paths.sort();
    report_paths.dedup();

    let mut production_reports = Vec::new();
    let mut missing_required_report_count = 0usize;
    let mut blocking_report_count = 0usize;
    for path in report_paths {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("<report>")
            .to_string();
        let required = required_set.contains(&name);
        let exists = path.exists();
        let (status, schema, summary, blocking) = if !exists {
            if required {
                missing_required_report_count += 1;
            }
            ("missing".to_string(), None, Vec::new(), required)
        } else {
            match read_work_package_report_status(&path) {
                Ok(report) => {
                    let blocking = report.status == "needs_review";
                    if blocking {
                        blocking_report_count += 1;
                    }
                    (report.status, report.schema, report.summary, blocking)
                }
                Err(err) => {
                    blocking_report_count += 1;
                    ("needs_review".to_string(), None, vec![err], true)
                }
            }
        };
        production_reports.push(LargeModProductionReport {
            name: name.clone(),
            kind: large_mod_production_report_kind(&name),
            path,
            required,
            exists,
            status,
            schema,
            summary,
            blocking,
        });
    }

    let blocking_count =
        blocked_package_count + missing_required_report_count + blocking_report_count;
    let decision = if blocking_count == 0 {
        "release_candidate"
    } else {
        "blocked"
    }
    .to_string();
    let next_commands = large_mod_production_next_commands(&root);
    Ok(LargeModProductionSnapshot {
        mod_name: blueprint.name.clone(),
        acronym: blueprint.acronym.clone(),
        mod_root: root.clone(),
        blueprint: blueprint_path.display().to_string(),
        claims_dir: claims_dir.display().to_string(),
        decision,
        package_count: production_packages.len(),
        ready_package_count,
        handoff_count,
        claimed_count,
        blocked_package_count,
        report_count: production_reports.len(),
        missing_required_report_count,
        blocking_report_count,
        blocking_count,
        packages: production_packages,
        reports: production_reports,
        next_commands,
        stop_conditions: large_mod_production_stop_conditions(),
    })
}

fn large_mod_production_snapshot_json(snapshot: &LargeModProductionSnapshot) -> String {
    let package_json = snapshot
        .packages
        .iter()
        .map(large_mod_production_package_json)
        .collect::<Vec<_>>()
        .join(",\n");
    let report_json = snapshot
        .reports
        .iter()
        .map(large_mod_production_report_json)
        .collect::<Vec<_>>()
        .join(",\n");
    format!(
        "{{\n  \"schema\": \"hoi4skill.large_mod_production_snapshot.v1\",\n  \"decision\": {},\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"claims_dir\": {},\n  \"package_count\": {},\n  \"ready_package_count\": {},\n  \"handoff_count\": {},\n  \"claimed_count\": {},\n  \"blocked_package_count\": {},\n  \"report_count\": {},\n  \"missing_required_report_count\": {},\n  \"blocking_report_count\": {},\n  \"blocking_count\": {},\n  \"packages\": [\n{}\n  ],\n  \"reports\": [\n{}\n  ],\n  \"next_commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_str(&snapshot.decision),
        json_str(&snapshot.mod_name),
        json_str(&snapshot.acronym),
        json_str(&snapshot.mod_root),
        json_str(&snapshot.blueprint),
        json_str(&snapshot.claims_dir),
        snapshot.package_count,
        snapshot.ready_package_count,
        snapshot.handoff_count,
        snapshot.claimed_count,
        snapshot.blocked_package_count,
        snapshot.report_count,
        snapshot.missing_required_report_count,
        snapshot.blocking_report_count,
        snapshot.blocking_count,
        package_json,
        report_json,
        json_array(&snapshot.next_commands),
        json_array(&snapshot.stop_conditions),
    )
}

fn large_mod_production_package_json(package: &LargeModProductionPackage) -> String {
    format!(
        "    {{\n      \"id\": {},\n      \"kind\": {},\n      \"title\": {},\n      \"stage\": {},\n      \"ready\": {},\n      \"handoff\": {},\n      \"claim_status\": {},\n      \"assignee\": {},\n      \"missing\": {},\n      \"blocking\": {},\n      \"blocked_by\": {}\n    }}",
        json_str(&package.id),
        json_str(&package.kind),
        json_str(&package.title),
        json_str(&package.stage),
        json_bool(package.ready),
        json_bool(package.handoff),
        json_str(&package.claim_status),
        json_optional_str(package.assignee.as_deref()),
        json_array(&package.missing),
        json_array(&package.blocking),
        json_array(&package.blocked_by),
    )
}

fn large_mod_production_report_json(report: &LargeModProductionReport) -> String {
    format!(
        "    {{\n      \"name\": {},\n      \"kind\": {},\n      \"path\": {},\n      \"required\": {},\n      \"exists\": {},\n      \"status\": {},\n      \"schema\": {},\n      \"summary\": {},\n      \"blocking\": {}\n    }}",
        json_str(&report.name),
        json_str(&report.kind),
        json_str(&report.path.display().to_string()),
        json_bool(report.required),
        json_bool(report.exists),
        json_str(&report.status),
        json_optional_str(report.schema.as_deref()),
        json_array(&report.summary),
        json_bool(report.blocking),
    )
}

fn large_mod_production_brief_markdown(snapshot: &LargeModProductionSnapshot) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# Large Mod Production Brief: {}\n\n",
        snapshot.mod_name
    ));
    out.push_str("- schema: `hoi4skill.large_mod_production_brief.v1`\n");
    out.push_str(&format!("- decision: `{}`\n", snapshot.decision));
    out.push_str(&format!("- acronym: `{}`\n", snapshot.acronym));
    out.push_str(&format!("- mod_root: `{}`\n", snapshot.mod_root));
    out.push_str(&format!("- blueprint: `{}`\n", snapshot.blueprint));
    out.push_str(&format!("- claims_dir: `{}`\n", snapshot.claims_dir));
    out.push_str(&format!(
        "- packages: `{}` ready, `{}` handoff, `{}` claimed, `{}` blocked, `{}` total\n",
        snapshot.ready_package_count,
        snapshot.handoff_count,
        snapshot.claimed_count,
        snapshot.blocked_package_count,
        snapshot.package_count
    ));
    out.push_str(&format!(
        "- reports: `{}` missing required, `{}` blocking, `{}` tracked\n",
        snapshot.missing_required_report_count,
        snapshot.blocking_report_count,
        snapshot.report_count
    ));
    out.push_str(&format!(
        "- total blockers: `{}`\n",
        snapshot.blocking_count
    ));

    out.push_str("\n## Immediate Blockers\n\n");
    if snapshot.blocking_count == 0 {
        out.push_str("- No blocking production issue found in the current snapshot.\n");
    } else {
        for report in snapshot
            .reports
            .iter()
            .filter(|report| report.required && !report.exists)
        {
            out.push_str(&format!(
                "- Missing required report: `{}` at `{}`\n",
                report.name,
                report.path.display()
            ));
        }
        for report in snapshot.reports.iter().filter(|report| report.blocking) {
            out.push_str(&format!(
                "- Blocking report: `{}` status `{}` ({})\n",
                report.name,
                report.status,
                markdown_table_cell(&report.summary.join("; "))
            ));
        }
        for package in snapshot
            .packages
            .iter()
            .filter(|package| package.stage == "blocked" || package.stage == "claim_blocked")
        {
            let details = [
                format!("missing={}", package.missing.join(",")),
                format!("blocking={}", package.blocking.join(",")),
                format!("blocked_by={}", package.blocked_by.join(",")),
            ]
            .join("; ");
            out.push_str(&format!(
                "- Package `{}` is `{}`: {}\n",
                package.id,
                package.stage,
                markdown_table_cell(&details)
            ));
        }
    }

    out.push_str("\n## Package State\n\n");
    out.push_str(
        "| Package | Kind | Stage | Ready | Handoff | Claim | Assignee | Missing | Blocking |\n",
    );
    out.push_str("| --- | --- | --- | --- | --- | --- | --- | --- | --- |\n");
    for package in &snapshot.packages {
        out.push_str(&format!(
            "| `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | {} | {} | {} |\n",
            package.id,
            package.kind,
            package.stage,
            package.ready,
            package.handoff,
            package.claim_status,
            markdown_table_cell(package.assignee.as_deref().unwrap_or("unassigned")),
            markdown_table_cell(&package.missing.join(", ")),
            markdown_table_cell(&package.blocking.join(", "))
        ));
    }

    out.push_str("\n## Report State\n\n");
    out.push_str("| Report | Kind | Required | Exists | Status | Summary |\n");
    out.push_str("| --- | --- | --- | --- | --- | --- |\n");
    for report in &snapshot.reports {
        out.push_str(&format!(
            "| `{}` | `{}` | `{}` | `{}` | `{}` | {} |\n",
            report.name,
            report.kind,
            report.required,
            report.exists,
            report.status,
            markdown_table_cell(&report.summary.join("; "))
        ));
    }

    out.push_str("\n## Next Commands\n\n");
    for command in &snapshot.next_commands {
        out.push_str(&format!("- `{command}`\n"));
    }

    out.push_str("\n## Stop Conditions\n\n");
    for condition in &snapshot.stop_conditions {
        out.push_str(&format!("- {condition}\n"));
    }
    out
}

fn large_mod_production_report_names() -> Vec<String> {
    let mut names = large_mod_required_release_reports();
    for name in [
        "ci_plan.json",
        "ownership_map.json",
        "dependency_graph.json",
        "milestone_plan.json",
        "execution_queue.json",
        "dispatch_gate.json",
        "merge_gate.json",
        "playtest_gate.json",
        "playtest_plan.json",
        "fix_queue.json",
        "regression_plan.json",
        "release_bundle.json",
        "next_actions.json",
        "risk_register.json",
        "production_snapshot.json",
    ] {
        names.push(name.to_string());
    }
    names.sort();
    names.dedup();
    names
}

fn large_mod_production_report_kind(name: &str) -> String {
    match name {
        "ci_plan.json" => "ci_plan",
        "ownership_map.json" => "ownership",
        "dependency_graph.json" => "dependency",
        "milestone_plan.json" => "milestone",
        "execution_queue.json" => "execution",
        "dispatch_gate.json" => "dispatch_gate",
        "merge_gate.json" => "merge_gate",
        "playtest_gate.json" => "playtest_gate",
        "playtest_plan.json" => "playtest_plan",
        "fix_queue.json" => "fix_queue",
        "regression_plan.json" => "regression_plan",
        "regression_gate.json" => "regression_gate",
        "release_bundle.json" => "release_bundle",
        "next_actions.json" => "next_actions",
        "risk_register.json" => "risk_register",
        "production_snapshot.json" => "production_snapshot",
        _ if name.starts_with("boundary_") => "package_boundary",
        _ if name.starts_with("status_") => "package_status",
        _ => "release_report",
    }
    .to_string()
}

fn large_mod_production_next_commands(mod_root: &str) -> Vec<String> {
    vec![
        format!("hoi4skill large-mod-next-actions --mod-root {mod_root} --output .hoi4skill/next_actions.json"),
        format!("hoi4skill large-mod-risk-register --mod-root {mod_root} --output .hoi4skill/risk_register.json"),
        format!("hoi4skill large-mod-dashboard --mod-root {mod_root} --output .hoi4skill/dashboard.md"),
        format!("hoi4skill large-mod-release-gate --mod-root {mod_root} --output .hoi4skill/release_gate.json"),
        format!("hoi4skill large-mod-production-snapshot --mod-root {mod_root} --output .hoi4skill/production_snapshot.json"),
        format!("hoi4skill large-mod-production-brief --mod-root {mod_root} --output .hoi4skill/production_brief.md"),
    ]
}

fn large_mod_production_stop_conditions() -> Vec<String> {
    vec![
        "Do not hand off production while blocking_count is greater than zero.".to_string(),
        "Do not treat claimed packages as complete without package handoff evidence.".to_string(),
        "Do not use this snapshot as a substitute for strict-code-index validation or release gate approval.".to_string(),
    ]
}

#[derive(Clone, Debug)]
struct LargeModFixQueueItem {
    severity: String,
    blocking: bool,
    package: Option<String>,
    kind: String,
    reason: String,
    evidence: PathBuf,
    source_schema: Option<String>,
    source_summary: Vec<String>,
    context: Option<String>,
    command: String,
}

fn large_mod_fix_queue_json(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    reports: &[PathBuf],
) -> Result<String, String> {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let mut items = large_mod_collect_fix_queue_items(packages, blueprint, mod_root, reports)?;
    items.sort_by(|left, right| {
        severity_rank(&left.severity)
            .cmp(&severity_rank(&right.severity))
            .then_with(|| right.blocking.cmp(&left.blocking))
            .then_with(|| left.package.cmp(&right.package))
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.evidence.cmp(&right.evidence))
            .then_with(|| left.reason.cmp(&right.reason))
    });
    let blocking_count = items.iter().filter(|item| item.blocking).count();
    let high_count = items.iter().filter(|item| item.severity == "high").count();
    let unassigned_count = items.iter().filter(|item| item.package.is_none()).count();
    let package_ids = items
        .iter()
        .filter_map(|item| item.package.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let item_json = items
        .iter()
        .enumerate()
        .map(|(idx, item)| large_mod_fix_queue_item_json(idx + 1, item))
        .collect::<Vec<_>>()
        .join(",\n");
    let next_commands = vec![
        format!("hoi4skill large-mod-fix-queue --mod-root {root} --output .hoi4skill/fix_queue.json"),
        format!("hoi4skill large-mod-risk-register --mod-root {root} --output .hoi4skill/risk_register.json"),
        format!("hoi4skill large-mod-next-actions --mod-root {root} --output .hoi4skill/next_actions.json"),
        format!("hoi4skill large-mod-release-gate --mod-root {root} --output .hoi4skill/release_gate.json"),
    ];
    Ok(format!(
        "{{\n  \"schema\": \"hoi4skill.large_mod_fix_queue.v1\",\n  \"healthy\": {},\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"report_count\": {},\n  \"item_count\": {},\n  \"blocking_count\": {},\n  \"high_count\": {},\n  \"unassigned_count\": {},\n  \"affected_packages\": {},\n  \"items\": [\n{}\n  ],\n  \"next_commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_bool(blocking_count == 0 && high_count == 0),
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        reports.len(),
        items.len(),
        blocking_count,
        high_count,
        unassigned_count,
        json_array(&package_ids),
        item_json,
        json_array(&next_commands),
        json_array(&large_mod_fix_queue_stop_conditions()),
    ))
}

fn large_mod_regression_plan_json(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    reports: &[PathBuf],
) -> Result<String, String> {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let items = large_mod_collect_fix_queue_items(packages, blueprint, mod_root, reports)?;
    let mut grouped: BTreeMap<Option<String>, Vec<LargeModFixQueueItem>> = BTreeMap::new();
    for item in items {
        grouped.entry(item.package.clone()).or_default().push(item);
    }
    let scenario_json = grouped
        .iter()
        .enumerate()
        .map(|(idx, (package_id, items))| {
            large_mod_regression_scenario_json(
                idx + 1,
                package_id.as_deref(),
                items,
                packages,
                &root,
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let package_scenario_count = grouped.keys().filter(|package| package.is_some()).count();
    let unassigned_count = grouped.get(&None).map(Vec::len).unwrap_or(0);
    let high_count = grouped
        .values()
        .flatten()
        .filter(|item| item.severity == "high")
        .count();
    let fix_item_count = grouped.values().map(Vec::len).sum::<usize>();
    let affected_packages = grouped.keys().filter_map(Clone::clone).collect::<Vec<_>>();
    let global_commands = vec![
        format!("hoi4skill large-mod-fix-queue --mod-root {root} --output .hoi4skill/fix_queue.json"),
        format!("hoi4skill validate {root} --strict-code-index --output .hoi4skill/validation.json"),
        format!("hoi4skill loc-audit {root} --output .hoi4skill/loc_audit.json"),
        format!("hoi4skill gfx-audit {root} --output .hoi4skill/gfx_audit.json"),
        format!("hoi4skill logic-audit {root} --output .hoi4skill/logic_audit.json"),
        format!("hoi4skill analyze-error-log --input <error.log> --mod-root {root} --output .hoi4skill/error_log_report.json"),
        format!("hoi4skill large-mod-regression-gate --mod-root {root} --output .hoi4skill/regression_gate.json"),
        format!("hoi4skill large-mod-regression-brief --mod-root {root} --output .hoi4skill/regression_brief.md"),
        format!("hoi4skill large-mod-playtest-gate --mod-root {root} --output .hoi4skill/playtest_gate.json"),
        format!("hoi4skill large-mod-release-gate --mod-root {root} --output .hoi4skill/release_gate.json"),
    ];
    Ok(format!(
        "{{\n  \"schema\": \"hoi4skill.large_mod_regression_plan.v1\",\n  \"healthy\": {},\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"report_count\": {},\n  \"fix_item_count\": {},\n  \"scenario_count\": {},\n  \"package_scenario_count\": {},\n  \"unassigned_count\": {},\n  \"high_count\": {},\n  \"affected_packages\": {},\n  \"scenarios\": [\n{}\n  ],\n  \"global_commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_bool(fix_item_count == 0 && high_count == 0),
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        reports.len(),
        fix_item_count,
        grouped.len(),
        package_scenario_count,
        unassigned_count,
        high_count,
        json_array(&affected_packages),
        scenario_json,
        json_array(&global_commands),
        json_array(&large_mod_regression_plan_stop_conditions()),
    ))
}

fn large_mod_regression_gate_json(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    reports: &[PathBuf],
    plan_path: &Path,
) -> Result<String, String> {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let base = mod_root
        .map(|root| root.join(".hoi4skill"))
        .unwrap_or_else(|| PathBuf::from(".hoi4skill"));
    let fix_items = large_mod_collect_fix_queue_items(packages, blueprint, mod_root, reports)?;
    let mut blocking = Vec::new();
    for item in &fix_items {
        blocking.push(large_mod_regression_gate_blocker_json(
            "fix_queue_item",
            item.package.as_deref(),
            &item.kind,
            &item.reason,
            &item.evidence,
            &item.command,
        ));
    }

    let (plan_exists, affected_packages, plan_unassigned_count) = if plan_path.exists() {
        let text = read_utf8_lossy(plan_path)?;
        let packages = parse_json_string_array_field(&text, "affected_packages")
            .into_iter()
            .collect::<Vec<_>>();
        (
            true,
            packages,
            status_json_i64_field(&text, "unassigned_count").unwrap_or(0),
        )
    } else {
        (false, Vec::new(), 0)
    };
    if !plan_exists {
        blocking.push(large_mod_regression_gate_blocker_json(
            "missing_regression_plan",
            None,
            "missing_regression_plan",
            "regression plan is missing",
            plan_path,
            &format!(
                "hoi4skill large-mod-regression-plan --mod-root {root} --output .hoi4skill/regression_plan.json"
            ),
        ));
    }
    if plan_unassigned_count > 0 {
        blocking.push(large_mod_regression_gate_blocker_json(
            "unassigned_regression_items",
            None,
            "unassigned_regression_items",
            &format!("regression plan still has {plan_unassigned_count} unassigned item(s)"),
            plan_path,
            &format!(
                "hoi4skill identify-work-packages --mod-root {root} --changed <fixed-file> --strict-names --output .hoi4skill/changed_work_packages.json"
            ),
        ));
    }

    let mut package_rows = Vec::new();
    for package_id in &affected_packages {
        let Some(package) = packages.iter().find(|package| &package.id == package_id) else {
            blocking.push(large_mod_regression_gate_blocker_json(
                "unknown_package",
                Some(package_id),
                "unknown_package",
                &format!("regression plan references unknown package `{package_id}`"),
                plan_path,
                &format!(
                    "hoi4skill large-mod-regression-plan --mod-root {root} --output .hoi4skill/regression_plan.json"
                ),
            ));
            continue;
        };
        let checks = [
            (
                "boundary",
                base.join(format!("boundary_{}.json", package.id)),
            ),
            (
                "validation",
                base.join(format!("validation_{}.json", package.id)),
            ),
            (
                "error_log",
                base.join(format!("error_log_{}.json", package.id)),
            ),
            (
                "playtest",
                base.join(format!("playtest_{}.json", package.id)),
            ),
        ];
        let mut check_rows = Vec::new();
        let mut package_blockers = 0usize;
        for (kind, path) in checks {
            let (status, schema, summary) = large_mod_regression_gate_check_status(&path);
            if status != "ok" {
                package_blockers += 1;
                blocking.push(large_mod_regression_gate_blocker_json(
                    "package_regression_check",
                    Some(&package.id),
                    kind,
                    &format!(
                        "`{kind}` regression check for `{}` is `{status}`",
                        package.id
                    ),
                    &path,
                    &large_mod_regression_gate_check_command(kind, &package.id, &root),
                ));
            }
            check_rows.push(format!(
                "{{\"kind\": {}, \"path\": {}, \"status\": {}, \"schema\": {}, \"summary\": {}}}",
                json_str(kind),
                json_str(&path.display().to_string()),
                json_str(&status),
                json_optional_str(schema.as_deref()),
                json_array(&summary),
            ));
        }
        package_rows.push(format!(
            "    {{\"id\": {}, \"kind\": {}, \"title\": {}, \"status\": {}, \"blocking_count\": {}, \"checks\": [{}]}}",
            json_str(&package.id),
            json_str(&package.kind),
            json_str(&package.title),
            json_str(if package_blockers == 0 { "passed" } else { "blocked" }),
            package_blockers,
            check_rows.join(", "),
        ));
    }

    let gate_passed = blocking.is_empty() && plan_exists;
    let next_commands = vec![
        format!("hoi4skill large-mod-regression-plan --mod-root {root} --output .hoi4skill/regression_plan.json"),
        format!("hoi4skill large-mod-regression-gate --mod-root {root} --output .hoi4skill/regression_gate.json"),
        format!("hoi4skill large-mod-regression-brief --mod-root {root} --output .hoi4skill/regression_brief.md"),
        format!("hoi4skill large-mod-playtest-gate --mod-root {root} --output .hoi4skill/playtest_gate.json"),
        format!("hoi4skill large-mod-release-gate --mod-root {root} --output .hoi4skill/release_gate.json"),
    ];
    Ok(format!(
        "{{\n  \"schema\": \"hoi4skill.large_mod_regression_gate.v1\",\n  \"regression_passed\": {},\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"plan\": {},\n  \"plan_exists\": {},\n  \"report_count\": {},\n  \"fix_item_count\": {},\n  \"affected_package_count\": {},\n  \"unassigned_count\": {},\n  \"blocking_count\": {},\n  \"packages\": [\n{}\n  ],\n  \"blocking\": [\n{}\n  ],\n  \"next_commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_bool(gate_passed),
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        json_str(&plan_path.display().to_string()),
        json_bool(plan_exists),
        reports.len(),
        fix_items.len(),
        affected_packages.len(),
        plan_unassigned_count,
        blocking.len(),
        package_rows.join(",\n"),
        blocking.join(",\n"),
        json_array(&next_commands),
        json_array(&large_mod_regression_gate_stop_conditions()),
    ))
}

fn large_mod_regression_gate_check_status(path: &Path) -> (String, Option<String>, Vec<String>) {
    if !path.exists() {
        return ("missing".to_string(), None, Vec::new());
    }
    match read_work_package_report_status(path) {
        Ok(report) => (report.status, report.schema, report.summary),
        Err(err) => ("needs_review".to_string(), None, vec![err]),
    }
}

fn large_mod_regression_gate_blocker_json(
    scope: &str,
    package: Option<&str>,
    kind: &str,
    reason: &str,
    evidence: &Path,
    command: &str,
) -> String {
    format!(
        "    {{\"scope\": {}, \"package\": {}, \"kind\": {}, \"reason\": {}, \"evidence\": {}, \"command\": {}}}",
        json_str(scope),
        json_optional_str(package),
        json_str(kind),
        json_str(reason),
        json_str(&evidence.display().to_string()),
        json_str(command),
    )
}

fn large_mod_regression_gate_check_command(kind: &str, package_id: &str, mod_root: &str) -> String {
    match kind {
        "boundary" => format!(
            "hoi4skill check-work-package-boundary --mod-root {mod_root} --package {package_id} --changed <fixed-file> --strict-names --output .hoi4skill/boundary_{package_id}.json"
        ),
        "validation" => format!(
            "hoi4skill validate {mod_root} --changed-only --changed <fixed-file> --strict-code-index --output .hoi4skill/validation_{package_id}.json"
        ),
        "error_log" => format!(
            "hoi4skill analyze-error-log --input <error.log> --mod-root {mod_root} --changed-only --changed <fixed-file> --output .hoi4skill/error_log_{package_id}.json"
        ),
        "playtest" => format!(
            "hoi4skill work-package-playtest-report --mod-root {mod_root} --package {package_id} --result passed --summary <regression-summary> --output .hoi4skill/playtest_{package_id}.json"
        ),
        _ => format!("hoi4skill large-mod-regression-plan --mod-root {mod_root}"),
    }
}

fn large_mod_regression_brief_markdown(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    reports: &[PathBuf],
    plan_path: &Path,
) -> Result<String, String> {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let base = mod_root
        .map(|root| root.join(".hoi4skill"))
        .unwrap_or_else(|| PathBuf::from(".hoi4skill"));
    let fix_items = large_mod_collect_fix_queue_items(packages, blueprint, mod_root, reports)?;
    let (plan_exists, affected_packages, plan_unassigned_count) = if plan_path.exists() {
        let text = read_utf8_lossy(plan_path)?;
        (
            true,
            parse_json_string_array_field(&text, "affected_packages")
                .into_iter()
                .collect::<Vec<_>>(),
            status_json_i64_field(&text, "unassigned_count").unwrap_or(0),
        )
    } else {
        (false, Vec::new(), 0)
    };

    let mut blocking_notes = Vec::new();
    if !plan_exists {
        blocking_notes.push("regression plan is missing".to_string());
    }
    if plan_unassigned_count > 0 {
        blocking_notes.push(format!(
            "regression plan still has {plan_unassigned_count} unassigned item(s)"
        ));
    }
    for item in &fix_items {
        blocking_notes.push(format!(
            "{}: {}",
            item.package.as_deref().unwrap_or("unassigned"),
            item.kind
        ));
    }

    let mut package_rows = Vec::new();
    for package_id in &affected_packages {
        if let Some(package) = packages.iter().find(|package| &package.id == package_id) {
            let checks = [
                (
                    "boundary",
                    base.join(format!("boundary_{}.json", package.id)),
                ),
                (
                    "validation",
                    base.join(format!("validation_{}.json", package.id)),
                ),
                (
                    "error_log",
                    base.join(format!("error_log_{}.json", package.id)),
                ),
                (
                    "playtest",
                    base.join(format!("playtest_{}.json", package.id)),
                ),
            ];
            let mut status_parts = Vec::new();
            let mut package_blockers = Vec::new();
            for (kind, path) in checks {
                let (status, _schema, summary) = large_mod_regression_gate_check_status(&path);
                status_parts.push(format!("{kind}={status}"));
                if status != "ok" {
                    package_blockers.push(format!("{kind}:{status}"));
                    blocking_notes.push(format!("{}: {kind} is {status}", package.id));
                }
                if !summary.is_empty() && status != "ok" {
                    package_blockers.push(summary.join("; "));
                }
            }
            package_rows.push((
                package.id.clone(),
                package.kind.clone(),
                if package_blockers.is_empty() {
                    "passed".to_string()
                } else {
                    "blocked".to_string()
                },
                status_parts,
                package_blockers,
            ));
        } else {
            blocking_notes.push(format!("unknown package `{package_id}` in regression plan"));
            package_rows.push((
                package_id.clone(),
                "unknown".to_string(),
                "blocked".to_string(),
                Vec::new(),
                vec!["unknown package".to_string()],
            ));
        }
    }

    let regression_passed = plan_exists && blocking_notes.is_empty();
    let mut out = String::new();
    out.push_str(&format!(
        "# Large Mod Regression Brief: {}\n\n",
        blueprint.name
    ));
    out.push_str("- schema: `hoi4skill.large_mod_regression_brief.v1`\n");
    out.push_str(&format!(
        "- decision: `{}`\n",
        if regression_passed {
            "regression_passed"
        } else {
            "blocked"
        }
    ));
    out.push_str(&format!("- acronym: `{}`\n", blueprint.acronym));
    out.push_str(&format!("- mod_root: `{}`\n", root));
    out.push_str(&format!("- blueprint: `{}`\n", blueprint_path.display()));
    out.push_str(&format!("- plan: `{}`\n", plan_path.display()));
    out.push_str(&format!(
        "- regression: `{}` package scenario(s), `{}` current fix item(s), `{}` unassigned item(s)\n",
        affected_packages.len(),
        fix_items.len(),
        plan_unassigned_count
    ));

    out.push_str("\n## Blocking Summary\n\n");
    if blocking_notes.is_empty() {
        out.push_str("- No blocking regression items found.\n");
    } else {
        for note in &blocking_notes {
            out.push_str(&format!("- `{}`\n", markdown_table_cell(note)));
        }
    }

    out.push_str("\n## Package Regression Status\n\n");
    out.push_str("| Package | Kind | Status | Checks | Blockers |\n");
    out.push_str("| --- | --- | --- | --- | --- |\n");
    for (id, kind, status, checks, blockers) in &package_rows {
        out.push_str(&format!(
            "| `{}` | `{}` | `{}` | {} | {} |\n",
            id,
            kind,
            status,
            markdown_table_cell(&checks.join(", ")),
            markdown_table_cell(&blockers.join("; "))
        ));
    }

    out.push_str("\n## Reviewer Commands\n\n");
    for command in large_mod_regression_brief_next_commands(&root) {
        out.push_str(&format!("- `{command}`\n"));
    }

    out.push_str("\n## Stop Conditions\n\n");
    out.push_str("- Do not close regression while decision is `blocked`.\n");
    out.push_str("- Do not accept missing validation, error-log, boundary, or playtest evidence as a pass.\n");
    out.push_str("- Regenerate release gate only after `large-mod-regression-gate` passes.\n");
    Ok(out)
}

fn large_mod_regression_brief_next_commands(mod_root: &str) -> Vec<String> {
    vec![
        format!("hoi4skill large-mod-regression-plan --mod-root {mod_root} --output .hoi4skill/regression_plan.json"),
        format!("hoi4skill large-mod-regression-gate --mod-root {mod_root} --output .hoi4skill/regression_gate.json"),
        format!("hoi4skill large-mod-regression-brief --mod-root {mod_root} --output .hoi4skill/regression_brief.md"),
        format!("hoi4skill large-mod-playtest-gate --mod-root {mod_root} --output .hoi4skill/playtest_gate.json"),
        format!("hoi4skill large-mod-release-gate --mod-root {mod_root} --output .hoi4skill/release_gate.json"),
    ]
}

fn large_mod_collect_fix_queue_items(
    packages: &[WorkPackage],
    blueprint: &LargeModBlueprint,
    mod_root: Option<&Path>,
    reports: &[PathBuf],
) -> Result<Vec<LargeModFixQueueItem>, String> {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let mut items = Vec::new();
    for report in reports {
        items.extend(large_mod_fix_items_for_report(
            packages, blueprint, mod_root, report, &root,
        )?);
    }
    Ok(items)
}

fn large_mod_regression_scenario_json(
    scenario_index: usize,
    package_id: Option<&str>,
    items: &[LargeModFixQueueItem],
    packages: &[WorkPackage],
    mod_root: &str,
) -> String {
    let severity = if items.iter().any(|item| item.severity == "high") {
        "high"
    } else {
        "medium"
    };
    let status = if package_id.is_some() {
        "package_regression_required"
    } else {
        "routing_required"
    };
    let contexts = items
        .iter()
        .filter_map(|item| item.context.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let kinds = items
        .iter()
        .map(|item| item.kind.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let evidence = items
        .iter()
        .map(|item| item.evidence.display().to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let commands = large_mod_regression_commands(package_id, packages, mod_root, &contexts);
    format!(
        "    {{\"scenario_index\": {}, \"status\": {}, \"severity\": {}, \"package\": {}, \"fix_count\": {}, \"kinds\": {}, \"contexts\": {}, \"evidence\": {}, \"commands\": {}}}",
        scenario_index,
        json_str(status),
        json_str(severity),
        json_optional_str(package_id),
        items.len(),
        json_array(&kinds),
        json_array(&contexts),
        json_array(&evidence),
        json_array(&commands),
    )
}

fn large_mod_regression_commands(
    package_id: Option<&str>,
    packages: &[WorkPackage],
    mod_root: &str,
    contexts: &[String],
) -> Vec<String> {
    if let Some(package_id) = package_id {
        let changed = contexts
            .iter()
            .filter_map(|context| large_mod_context_changed_path(context))
            .next()
            .unwrap_or_else(|| "<fixed-file>".to_string());
        let package_exists = packages.iter().any(|package| package.id == package_id);
        let mut commands = Vec::new();
        if package_exists {
            commands.push(format!(
                "hoi4skill work-package-start-brief --mod-root {mod_root} --package {package_id} --output .hoi4skill/start_{package_id}.md"
            ));
            commands.push(format!(
                "hoi4skill check-work-package-boundary --mod-root {mod_root} --package {package_id} --changed {changed} --strict-names --output .hoi4skill/boundary_{package_id}.json"
            ));
            commands.push(format!(
                "hoi4skill validate {mod_root} --changed-only --changed {changed} --strict-code-index --output .hoi4skill/validation_{package_id}.json"
            ));
            commands.push(format!(
                "hoi4skill analyze-error-log --input <error.log> --mod-root {mod_root} --changed-only --changed {changed} --output .hoi4skill/error_log_{package_id}.json"
            ));
            commands.push(format!(
                "hoi4skill work-package-playtest-report --mod-root {mod_root} --package {package_id} --result passed --summary <regression-summary> --output .hoi4skill/playtest_{package_id}.json"
            ));
        }
        commands
    } else {
        vec![
            format!("hoi4skill identify-work-packages --mod-root {mod_root} --changed <fixed-file> --strict-names --output .hoi4skill/changed_work_packages.json"),
            format!("hoi4skill split-changed-work-packages --mod-root {mod_root} --changed <fixed-file> --strict-names --output .hoi4skill/split_changed.json"),
            format!("hoi4skill large-mod-fix-queue --mod-root {mod_root} --output .hoi4skill/fix_queue.json"),
        ]
    }
}

fn large_mod_context_changed_path(context: &str) -> Option<String> {
    let trimmed = context.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some((path, line)) = trimmed.rsplit_once(':') {
        if !path.ends_with(':') && line.chars().all(|ch| ch.is_ascii_digit()) {
            return Some(path.to_string());
        }
    }
    Some(trimmed.to_string())
}

fn large_mod_fix_items_for_report(
    packages: &[WorkPackage],
    blueprint: &LargeModBlueprint,
    mod_root: Option<&Path>,
    path: &Path,
    root: &str,
) -> Result<Vec<LargeModFixQueueItem>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = read_utf8_lossy(path)?;
    let status = read_work_package_report_status(path)?;
    if status.status != "needs_review" {
        return Ok(Vec::new());
    }
    if status.schema.as_deref() == Some("hoi4skill.error_log_report.v1") {
        let diagnostics = large_mod_error_log_diagnostic_fix_items(
            packages, blueprint, mod_root, path, root, &text, &status,
        );
        if !diagnostics.is_empty() {
            return Ok(diagnostics);
        }
    }
    Ok(vec![large_mod_report_fix_item(
        packages, blueprint, mod_root, path, root, status,
    )])
}

fn large_mod_error_log_diagnostic_fix_items(
    packages: &[WorkPackage],
    blueprint: &LargeModBlueprint,
    mod_root: Option<&Path>,
    path: &Path,
    root: &str,
    text: &str,
    status: &WorkPackageReportStatus,
) -> Vec<LargeModFixQueueItem> {
    json_objects_in_array_field(text, "diagnostics")
        .into_iter()
        .map(|object| {
            let severity = status_json_string_field(&object, "severity")
                .unwrap_or_else(|| "error".to_string());
            let category = status_json_string_field(&object, "category")
                .unwrap_or_else(|| "error_log".to_string());
            let file = status_json_string_field(&object, "file")
                .or_else(|| status_json_string_field(&object, "resolved_file"));
            let line = status_json_i64_field(&object, "line");
            let message = status_json_string_field(&object, "message")
                .unwrap_or_else(|| status.summary.join("; "));
            let suggestion = status_json_string_field(&object, "suggestion");
            let package = file
                .as_deref()
                .and_then(|file| large_mod_package_for_path(packages, blueprint, mod_root, file))
                .or_else(|| large_mod_package_from_report_path(path));
            let context = file.as_ref().map(|file| match line {
                Some(line) => format!("{file}:{line}"),
                None => file.clone(),
            });
            let reason = if message.trim().is_empty() {
                category.clone()
            } else {
                format!("{category}: {message}")
            };
            let command = large_mod_fix_queue_command(package.as_deref(), root, file.as_deref());
            LargeModFixQueueItem {
                severity: large_mod_fix_queue_severity(&severity, &category),
                blocking: severity != "warning",
                package,
                kind: format!("error_log_{category}"),
                reason,
                evidence: path.to_path_buf(),
                source_schema: status.schema.clone(),
                source_summary: suggestion.into_iter().collect(),
                context,
                command,
            }
        })
        .collect()
}

fn large_mod_report_fix_item(
    packages: &[WorkPackage],
    blueprint: &LargeModBlueprint,
    mod_root: Option<&Path>,
    path: &Path,
    root: &str,
    status: WorkPackageReportStatus,
) -> LargeModFixQueueItem {
    let schema = status.schema.as_deref().unwrap_or("unknown");
    let mut package = large_mod_package_from_report_path(path);
    let kind = large_mod_fix_queue_kind(schema, path);
    let severity = if large_mod_fix_queue_report_is_global(path, package.as_deref()) {
        "high"
    } else {
        "medium"
    };
    let context = package
        .as_deref()
        .and_then(|package_id| packages.iter().find(|package| package.id == package_id))
        .map(|package| package.title.clone())
        .or_else(|| {
            status
                .summary
                .iter()
                .find_map(|summary| summary.strip_prefix("file=").map(str::to_string))
        });
    if package.is_none() {
        package = status
            .summary
            .iter()
            .find_map(|summary| summary.strip_prefix("file="))
            .and_then(|file| large_mod_package_for_path(packages, blueprint, mod_root, file));
    }
    let command = large_mod_fix_queue_command(package.as_deref(), root, None);
    LargeModFixQueueItem {
        severity: severity.to_string(),
        blocking: true,
        package,
        kind,
        reason: status.summary.join("; "),
        evidence: path.to_path_buf(),
        source_schema: status.schema,
        source_summary: status.summary,
        context,
        command,
    }
}

fn large_mod_fix_queue_item_json(priority: usize, item: &LargeModFixQueueItem) -> String {
    format!(
        "    {{\"priority\": {}, \"severity\": {}, \"blocking\": {}, \"package\": {}, \"kind\": {}, \"reason\": {}, \"evidence\": {}, \"source_schema\": {}, \"source_summary\": {}, \"context\": {}, \"command\": {}}}",
        priority,
        json_str(&item.severity),
        json_bool(item.blocking),
        json_optional_str(item.package.as_deref()),
        json_str(&item.kind),
        json_str(&item.reason),
        json_str(&item.evidence.display().to_string()),
        json_optional_str(item.source_schema.as_deref()),
        json_array(&item.source_summary),
        json_optional_str(item.context.as_deref()),
        json_str(&item.command),
    )
}

fn large_mod_package_from_report_path(path: &Path) -> Option<String> {
    package_id_from_report_path(path).or_else(|| package_id_from_playtest_report_path(path))
}

fn large_mod_package_for_path(
    packages: &[WorkPackage],
    blueprint: &LargeModBlueprint,
    mod_root: Option<&Path>,
    raw_path: &str,
) -> Option<String> {
    let normalized = normalize_boundary_path(raw_path, mod_root);
    let matches = packages
        .iter()
        .filter(|package| {
            work_package_match_for_path(&normalized, package, blueprint, false).is_some()
        })
        .map(|package| package.id.clone())
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        matches.into_iter().next()
    } else {
        packages
            .iter()
            .find(|package| boundary_path_matches_package_identity(&normalized, package, blueprint))
            .map(|package| package.id.clone())
    }
}

fn large_mod_fix_queue_kind(schema: &str, path: &Path) -> String {
    match schema {
        "hoi4skill.validation.v1" => "validation_failure".to_string(),
        "hoi4skill.loc_audit.v1" => "localisation_audit".to_string(),
        "hoi4skill.loc_sync.v1" => "localisation_sync".to_string(),
        "hoi4skill.gfx_audit.v1" => "gfx_audit".to_string(),
        "hoi4skill.logic_audit.v1" => "logic_audit".to_string(),
        "hoi4skill.playtest_report.v1" => "playtest_finding".to_string(),
        "hoi4skill.work_package_boundary.v1" => "boundary_violation".to_string(),
        _ => path
            .file_stem()
            .and_then(|name| name.to_str())
            .map(|name| format!("report_needs_review:{name}"))
            .unwrap_or_else(|| "report_needs_review".to_string()),
    }
}

fn large_mod_fix_queue_report_is_global(path: &Path, package: Option<&str>) -> bool {
    package.is_none() && large_mod_package_from_report_path(path).is_none()
}

fn large_mod_fix_queue_severity(raw_severity: &str, category: &str) -> String {
    let raw = raw_severity.to_ascii_lowercase();
    if raw == "warning" {
        "medium".to_string()
    } else if ["syntax", "script_command", "event_namespace"].contains(&category) {
        "high".to_string()
    } else {
        "medium".to_string()
    }
}

fn large_mod_fix_queue_command(
    package: Option<&str>,
    mod_root: &str,
    file: Option<&str>,
) -> String {
    if let Some(package) = package {
        format!(
            "hoi4skill work-package-start-brief --mod-root {mod_root} --package {package} --output .hoi4skill/start_{package}.md"
        )
    } else if let Some(file) = file {
        format!(
            "hoi4skill identify-work-packages --mod-root {mod_root} --changed {} --strict-names --output .hoi4skill/changed_work_packages.json",
            file
        )
    } else {
        format!(
            "hoi4skill large-mod-next-actions --mod-root {mod_root} --output .hoi4skill/next_actions.json"
        )
    }
}

fn json_objects_in_array_field(text: &str, key: &str) -> Vec<String> {
    let marker = format!("\"{key}\"");
    let Some(idx) = text.find(&marker) else {
        return Vec::new();
    };
    let after_key = &text[idx + marker.len()..];
    let Some(colon) = after_key.find(':') else {
        return Vec::new();
    };
    let after_colon = after_key[colon + 1..].trim_start();
    let Some(array_body) = after_colon.strip_prefix('[') else {
        return Vec::new();
    };
    let mut objects = Vec::new();
    let mut object_start = None;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 1usize;
    let mut in_string = false;
    let mut escaped = false;
    for (idx, ch) in array_body.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if in_string {
            if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '[' => bracket_depth += 1,
            ']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                if bracket_depth == 0 {
                    break;
                }
            }
            '{' => {
                if brace_depth == 0 && bracket_depth == 1 {
                    object_start = Some(idx);
                }
                brace_depth += 1;
            }
            '}' => {
                brace_depth = brace_depth.saturating_sub(1);
                if brace_depth == 0 {
                    if let Some(start) = object_start.take() {
                        objects.push(array_body[start..=idx].to_string());
                    }
                }
            }
            _ => {}
        }
    }
    objects
}

fn large_mod_fix_queue_stop_conditions() -> Vec<String> {
    vec![
        "Do not treat a fix queue item as resolved until the producing report is rerun and no longer needs review.".to_string(),
        "Do not edit outside the assigned package boundary while fixing a package-scoped item.".to_string(),
        "Unassigned error-log items must be routed through identify-work-packages before package work starts.".to_string(),
    ]
}

fn large_mod_regression_plan_stop_conditions() -> Vec<String> {
    vec![
        "Do not close a fix queue item until the package regression scenario and global gates are rerun.".to_string(),
        "Do not mark unassigned regression work complete before identify-work-packages assigns the touched files.".to_string(),
        "Do not release while the regenerated fix queue still contains high severity or blocking items.".to_string(),
    ]
}

fn large_mod_regression_gate_stop_conditions() -> Vec<String> {
    vec![
        "Do not pass regression while the regenerated fix queue still contains any item.".to_string(),
        "Do not pass regression while any affected package is missing boundary, validation, error-log, or playtest evidence.".to_string(),
        "Do not continue to release gate until regression_gate reports regression_passed=true.".to_string(),
    ]
}

#[derive(Clone, Debug)]
struct LargeModRisk {
    severity: String,
    blocking: bool,
    scope: String,
    package: Option<String>,
    kind: String,
    reason: String,
    evidence: PathBuf,
    command: String,
}

fn large_mod_risk_register_json(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    extra_reports: &[PathBuf],
    claims_dir: &Path,
) -> Result<String, String> {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let base = mod_root
        .map(|root| root.join(".hoi4skill"))
        .unwrap_or_else(|| PathBuf::from(".hoi4skill"));
    let mut risks = Vec::new();
    let required_reports = large_mod_required_release_reports();

    for name in &required_reports {
        let path = base.join(name);
        if !path.exists() {
            risks.push(LargeModRisk {
                severity: "high".to_string(),
                blocking: true,
                scope: "global".to_string(),
                package: None,
                kind: "missing_required_report".to_string(),
                reason: format!("required report `{name}` is missing"),
                evidence: path,
                command: large_mod_required_report_command(name, &root),
            });
            continue;
        }
        let report = read_work_package_report_status(&path)?;
        if report.status == "needs_review" {
            risks.push(LargeModRisk {
                severity: "high".to_string(),
                blocking: true,
                scope: "global".to_string(),
                package: None,
                kind: "required_report_needs_review".to_string(),
                reason: report.summary.join("; "),
                evidence: path,
                command: large_mod_required_report_command(name, &root),
            });
        }
    }

    for path in extra_reports {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if required_reports.iter().any(|required| required == name) {
            continue;
        }
        let report = read_work_package_report_status(path)?;
        if report.status == "needs_review" {
            risks.push(LargeModRisk {
                severity: "high".to_string(),
                blocking: true,
                scope: "package_report".to_string(),
                package: package_id_from_report_path(path),
                kind: "report_needs_review".to_string(),
                reason: report.summary.join("; "),
                evidence: path.to_path_buf(),
                command: large_mod_report_rerun_command(name, &root),
            });
        }
    }

    for package in packages {
        let summary = work_package_readiness_summary(package, mod_root);
        for missing in &summary.missing {
            let severity = if ["boundary", "status", "validation"].contains(&missing.as_str()) {
                "high"
            } else {
                "medium"
            };
            risks.push(LargeModRisk {
                severity: severity.to_string(),
                blocking: true,
                scope: "work_package".to_string(),
                package: Some(package.id.clone()),
                kind: "missing_package_artifact".to_string(),
                reason: format!("missing `{missing}` artifact for `{}`", package.id),
                evidence: work_package_artifact_path(package, mod_root, missing),
                command: work_package_artifact_command(package, missing, &root),
            });
        }
        for blocking in &summary.blocking {
            risks.push(LargeModRisk {
                severity: "high".to_string(),
                blocking: true,
                scope: "work_package".to_string(),
                package: Some(package.id.clone()),
                kind: "package_artifact_needs_review".to_string(),
                reason: format!("`{blocking}` artifact needs review for `{}`", package.id),
                evidence: work_package_artifact_path(package, mod_root, blocking),
                command: work_package_artifact_command(package, blocking, &root),
            });
        }
        if summary.ready && !summary.handoff_path.exists() {
            risks.push(LargeModRisk {
                severity: "medium".to_string(),
                blocking: false,
                scope: "work_package".to_string(),
                package: Some(package.id.clone()),
                kind: "handoff_missing".to_string(),
                reason: format!("ready package `{}` has no handoff markdown", package.id),
                evidence: summary.handoff_path.clone(),
                command: format!(
                    "hoi4skill work-package-handoff --mod-root {root} --package {} --output .hoi4skill/handoff_{}.md",
                    package.id, package.id
                ),
            });
        }

        let claim = work_package_claim_summary(package, packages, mod_root, claims_dir);
        if claim.claim_status == "needs_review" {
            risks.push(LargeModRisk {
                severity: "high".to_string(),
                blocking: true,
                scope: "dispatch".to_string(),
                package: Some(package.id.clone()),
                kind: "claim_needs_review".to_string(),
                reason: format!("claim for `{}` cannot be read", package.id),
                evidence: claim.claim_path.clone(),
                command: format!(
                    "hoi4skill work-package-release-claim --mod-root {root} --package {} --released-by <assignee> --reason <reason>",
                    package.id
                ),
            });
        } else if claim.claim_status == "blocked_claim" {
            risks.push(LargeModRisk {
                severity: "high".to_string(),
                blocking: true,
                scope: "dispatch".to_string(),
                package: Some(package.id.clone()),
                kind: "blocked_claim".to_string(),
                reason: format!(
                    "claim for `{}` exists while current_state is `{}`",
                    package.id, claim.current_state
                ),
                evidence: claim.claim_path.clone(),
                command: format!(
                    "hoi4skill work-package-release-claim --mod-root {root} --package {} --released-by <assignee> --reason <reason>",
                    package.id
                ),
            });
        } else if claim.claim_status == "claimed" && claim.current_state == "already_handed_off" {
            risks.push(LargeModRisk {
                severity: "medium".to_string(),
                blocking: true,
                scope: "dispatch".to_string(),
                package: Some(package.id.clone()),
                kind: "stale_claim_after_handoff".to_string(),
                reason: format!("claim for `{}` remains after package handoff", package.id),
                evidence: claim.claim_path.clone(),
                command: format!(
                    "hoi4skill work-package-release-claim --mod-root {root} --package {} --released-by <assignee> --reason <reason>",
                    package.id
                ),
            });
        } else if claim.claim_status == "unclaimed" && claim.current_state == "ready_to_start" {
            risks.push(LargeModRisk {
                severity: "medium".to_string(),
                blocking: false,
                scope: "dispatch".to_string(),
                package: Some(package.id.clone()),
                kind: "ready_package_unclaimed".to_string(),
                reason: format!("ready package `{}` has no active claim", package.id),
                evidence: claim.claim_path.clone(),
                command: format!(
                    "hoi4skill work-package-claim --mod-root {root} --package {} --assignee <assignee>",
                    package.id
                ),
            });
        }
    }

    risks.sort_by(|left, right| {
        severity_rank(&left.severity)
            .cmp(&severity_rank(&right.severity))
            .then_with(|| right.blocking.cmp(&left.blocking))
            .then_with(|| left.scope.cmp(&right.scope))
            .then_with(|| left.package.cmp(&right.package))
            .then_with(|| left.kind.cmp(&right.kind))
    });

    let high_count = risks.iter().filter(|risk| risk.severity == "high").count();
    let medium_count = risks
        .iter()
        .filter(|risk| risk.severity == "medium")
        .count();
    let low_count = risks.iter().filter(|risk| risk.severity == "low").count();
    let blocking_count = risks.iter().filter(|risk| risk.blocking).count();
    let risk_json = risks
        .iter()
        .enumerate()
        .map(|(idx, risk)| large_mod_risk_json(idx + 1, risk))
        .collect::<Vec<_>>()
        .join(",\n");
    let next_commands = vec![
        format!("hoi4skill large-mod-risk-register --mod-root {root} --output .hoi4skill/risk_register.json"),
        format!("hoi4skill large-mod-next-actions --mod-root {root} --output .hoi4skill/next_actions.json"),
        format!("hoi4skill large-mod-dispatch-gate --mod-root {root} --output .hoi4skill/dispatch_gate.json"),
        format!("hoi4skill large-mod-release-gate --mod-root {root} --output .hoi4skill/release_gate.json"),
    ];
    Ok(format!(
        "{{\n  \"schema\": \"hoi4skill.large_mod_risk_register.v1\",\n  \"healthy\": {},\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"claims_dir\": {},\n  \"package_count\": {},\n  \"risk_count\": {},\n  \"blocking_count\": {},\n  \"high_count\": {},\n  \"medium_count\": {},\n  \"low_count\": {},\n  \"risks\": [\n{}\n  ],\n  \"next_commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_bool(blocking_count == 0 && high_count == 0),
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        json_str(&claims_dir.display().to_string()),
        packages.len(),
        risks.len(),
        blocking_count,
        high_count,
        medium_count,
        low_count,
        risk_json,
        json_array(&next_commands),
        json_array(&large_mod_risk_register_stop_conditions()),
    ))
}

fn severity_rank(severity: &str) -> usize {
    match severity {
        "high" => 0,
        "medium" => 1,
        "low" => 2,
        _ => 3,
    }
}

fn large_mod_risk_json(priority: usize, risk: &LargeModRisk) -> String {
    format!(
        "    {{\"priority\": {}, \"severity\": {}, \"blocking\": {}, \"scope\": {}, \"package\": {}, \"kind\": {}, \"reason\": {}, \"evidence\": {}, \"command\": {}}}",
        priority,
        json_str(&risk.severity),
        json_bool(risk.blocking),
        json_str(&risk.scope),
        json_optional_str(risk.package.as_deref()),
        json_str(&risk.kind),
        json_str(&risk.reason),
        json_str(&risk.evidence.display().to_string()),
        json_str(&risk.command),
    )
}

fn large_mod_risk_register_stop_conditions() -> Vec<String> {
    vec![
        "Do not release while high severity or blocking risks remain.".to_string(),
        "Do not dispatch packages with blocked or stale claims.".to_string(),
        "Do not treat a missing boundary, status, or validation artifact as a nonblocking risk."
            .to_string(),
    ]
}

#[allow(clippy::too_many_arguments)]
fn large_mod_action_json(
    priority: usize,
    blocking: bool,
    scope: &str,
    package: Option<&str>,
    kind: &str,
    reason: &str,
    path: &Path,
    command: &str,
) -> String {
    format!(
        "    {{\"priority\": {}, \"blocking\": {}, \"scope\": {}, \"package\": {}, \"kind\": {}, \"reason\": {}, \"path\": {}, \"command\": {}}}",
        priority,
        json_bool(blocking),
        json_str(scope),
        json_optional_str(package),
        json_str(kind),
        json_str(reason),
        json_str(&path.display().to_string()),
        json_str(command),
    )
}

fn large_mod_required_report_command(name: &str, mod_root: &str) -> String {
    match name {
        "mod_index.json" => {
            format!("hoi4skill build-mod-index {mod_root} --output .hoi4skill/mod_index.json")
        }
        "loc_audit.json" => {
            format!("hoi4skill loc-audit {mod_root} --output .hoi4skill/loc_audit.json")
        }
        "gfx_audit.json" => {
            format!("hoi4skill gfx-audit {mod_root} --output .hoi4skill/gfx_audit.json")
        }
        "logic_audit.json" => {
            format!("hoi4skill logic-audit {mod_root} --output .hoi4skill/logic_audit.json")
        }
        "ownership_map.json" => {
            format!("hoi4skill large-mod-ownership-map --mod-root {mod_root} --output .hoi4skill/ownership_map.json")
        }
        "validation.json" => {
            format!("hoi4skill validate {mod_root} --strict-code-index --output .hoi4skill/validation.json")
        }
        "work_package_status.json" => {
            format!("hoi4skill work-package-status --mod-root {mod_root} --output .hoi4skill/work_package_status.json")
        }
        "readiness.json" => {
            format!("hoi4skill work-package-readiness --mod-root {mod_root} --output .hoi4skill/readiness.json")
        }
        _ => format!("hoi4skill large-mod-dashboard --mod-root {mod_root}"),
    }
}

fn large_mod_report_rerun_command(name: &str, mod_root: &str) -> String {
    if name.starts_with("boundary_") {
        let package = name
            .trim_start_matches("boundary_")
            .trim_end_matches(".json");
        format!(
            "hoi4skill check-work-package-boundary --mod-root {mod_root} --package {package} --changed-file .hoi4skill/changed_{package}.txt --strict-names --output .hoi4skill/{name}"
        )
    } else if name.starts_with("status_") {
        let package = name.trim_start_matches("status_").trim_end_matches(".json");
        format!(
            "hoi4skill work-package-status --mod-root {mod_root} --package {package} --output .hoi4skill/{name}"
        )
    } else {
        format!("inspect .hoi4skill/{name} and rerun the producing command")
    }
}

fn work_package_artifact_path(
    package: &WorkPackage,
    mod_root: Option<&Path>,
    label: &str,
) -> PathBuf {
    work_package_readiness_artifacts(package, mod_root)
        .into_iter()
        .find(|artifact| artifact.label == label)
        .map(|artifact| artifact.path)
        .unwrap_or_else(|| {
            mod_root
                .map(|root| root.join(".hoi4skill"))
                .unwrap_or_else(|| PathBuf::from(".hoi4skill"))
                .join(format!("{}_{}", label, package.id))
        })
}

fn work_package_artifact_command(package: &WorkPackage, label: &str, mod_root: &str) -> String {
    match label {
        "changed" => {
            format!("hoi4skill split-changed-work-packages --mod-root {mod_root} --changed-file <changed-files.txt> --strict-names --output .hoi4skill/split_changed.json")
        }
        "plan" => format!(
            "hoi4skill generate-work-package --mod-root {mod_root} --package {} --dry-run --output .hoi4skill/plan_{}.json",
            package.id, package.id
        ),
        "assets" => format!(
            "hoi4skill asset-pack-plan --mod-root {mod_root} --package {} --output .hoi4skill/assets_{}.md",
            package.id, package.id
        ),
        "boundary" => format!(
            "hoi4skill check-work-package-boundary --mod-root {mod_root} --package {} --changed-file .hoi4skill/changed_{}.txt --strict-names --output .hoi4skill/boundary_{}.json",
            package.id, package.id, package.id
        ),
        "status" => format!(
            "hoi4skill work-package-status --mod-root {mod_root} --package {} --output .hoi4skill/status_{}.json",
            package.id, package.id
        ),
        "validation" => format!(
            "hoi4skill validate {mod_root} --changed-only --changed <planned-file> --strict-code-index --output .hoi4skill/validation_{}.json",
            package.id
        ),
        _ => format!("hoi4skill work-package-handoff --mod-root {mod_root} --package {}", package.id),
    }
}

fn large_mod_next_action_stop_conditions() -> Vec<String> {
    vec![
        "Run blocking actions before treating the dashboard as release-ready.".to_string(),
        "Do not skip boundary or validation artifacts for a package with changed files."
            .to_string(),
        "Do not hand off package work without a package handoff markdown file.".to_string(),
    ]
}

fn large_mod_ownership_map_json(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
) -> String {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let package_json = packages
        .iter()
        .map(|package| {
            format!(
                "{{\n      \"id\": {},\n      \"kind\": {},\n      \"title\": {},\n      \"token\": {},\n      \"tag\": {},\n      \"namespace\": {},\n      \"identity_terms\": {},\n      \"allowed_paths\": {}\n    }}",
                json_str(&package.id),
                json_str(&package.kind),
                json_str(&package.title),
                json_str(&package_token(package)),
                json_optional_str(package_tag(package).as_deref()),
                json_str(&package_namespace(package, blueprint)),
                json_array(&work_package_identity_terms(package, blueprint)),
                json_array(&work_package_boundary_allowed_prefixes(package)),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let mut owners_by_path: BTreeMap<String, Vec<&WorkPackage>> = BTreeMap::new();
    for package in packages {
        for path in work_package_boundary_allowed_prefixes(package) {
            owners_by_path.entry(path).or_default().push(package);
        }
    }
    let mut shared_path_count = 0usize;
    let path_json = owners_by_path
        .iter()
        .map(|(path, owners)| {
            let owner_json = owners
                .iter()
                .map(|package| {
                    format!(
                        "{{\"id\": {}, \"kind\": {}, \"title\": {}}}",
                        json_str(&package.id),
                        json_str(&package.kind),
                        json_str(&package.title)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            let requires_identity_terms = owners.len() > 1 && !path.starts_with(".hoi4skill/");
            if owners.len() > 1 {
                shared_path_count += 1;
            }
            format!(
                "{{\n      \"path\": {},\n      \"owner_count\": {},\n      \"requires_identity_terms\": {},\n      \"owners\": [{}]\n    }}",
                json_str(path),
                owners.len(),
                json_bool(requires_identity_terms),
                owner_json
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let next_commands = vec![
        format!("hoi4skill large-mod-ownership-map --mod-root {root} --output .hoi4skill/ownership_map.json"),
        format!("hoi4skill large-mod-dependency-graph --mod-root {root} --output .hoi4skill/dependency_graph.json"),
        format!("hoi4skill large-mod-milestone-plan --mod-root {root} --output .hoi4skill/milestone_plan.json"),
        format!("hoi4skill large-mod-execution-queue --mod-root {root} --output .hoi4skill/execution_queue.json"),
        format!("hoi4skill split-changed-work-packages --mod-root {root} --changed-file <changed-files.txt> --strict-names --output .hoi4skill/split_changed.json"),
        format!("hoi4skill large-mod-ci-plan --mod-root {root} --strict-names --output .hoi4skill/ci_plan.json"),
    ];
    format!(
        "{{\n  \"schema\": \"hoi4skill.large_mod_ownership_map.v1\",\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"package_count\": {},\n  \"path_count\": {},\n  \"shared_path_count\": {},\n  \"packages\": [\n{}\n  ],\n  \"paths\": [\n{}\n  ],\n  \"next_commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        packages.len(),
        owners_by_path.len(),
        shared_path_count,
        package_json,
        path_json,
        json_array(&next_commands),
        json_array(&large_mod_ownership_stop_conditions()),
    )
}

fn large_mod_ownership_stop_conditions() -> Vec<String> {
    vec![
        "Use --strict-names for changed-file routing whenever a path has multiple owners."
            .to_string(),
        "Do not assign shared paths by directory prefix alone; require package identity terms."
            .to_string(),
        "Do not expand a package edit surface without updating the blueprint or literal user request."
            .to_string(),
    ]
}

struct EvidenceFileEntry {
    path: PathBuf,
    kind: String,
    package: Option<String>,
    required: bool,
    report_like: bool,
}

fn large_mod_evidence_pack_json(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    extra_reports: &[PathBuf],
) -> Result<String, String> {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let base = mod_root
        .map(|root| root.join(".hoi4skill"))
        .unwrap_or_else(|| PathBuf::from(".hoi4skill"));
    let mut entries = Vec::new();
    let mut seen = BTreeSet::new();
    for name in large_mod_required_release_reports() {
        push_evidence_entry(
            &mut entries,
            &mut seen,
            EvidenceFileEntry {
                path: base.join(name),
                kind: "required_report".to_string(),
                package: None,
                required: true,
                report_like: true,
            },
        );
    }
    for report in extra_reports {
        push_evidence_entry(
            &mut entries,
            &mut seen,
            EvidenceFileEntry {
                path: report.to_path_buf(),
                kind: "package_report".to_string(),
                package: package_id_from_report_path(report),
                required: false,
                report_like: true,
            },
        );
    }
    for package in packages {
        for artifact in work_package_readiness_artifacts(package, mod_root) {
            push_evidence_entry(
                &mut entries,
                &mut seen,
                EvidenceFileEntry {
                    path: artifact.path,
                    kind: format!("package_{}", artifact.label),
                    package: Some(package.id.clone()),
                    required: true,
                    report_like: artifact.report_like,
                },
            );
        }
        push_evidence_entry(
            &mut entries,
            &mut seen,
            EvidenceFileEntry {
                path: base.join(format!("handoff_{}.md", package.id)),
                kind: "package_handoff".to_string(),
                package: Some(package.id.clone()),
                required: false,
                report_like: false,
            },
        );
    }

    let mut missing_count = 0usize;
    let mut needs_review_count = 0usize;
    let file_json = entries
        .iter()
        .map(|entry| {
            let exists = entry.path.exists();
            if !exists {
                missing_count += 1;
            }
            let (status, schema, summary) = if exists && entry.report_like {
                match read_work_package_report_status(&entry.path) {
                    Ok(report) => {
                        if report.status == "needs_review" {
                            needs_review_count += 1;
                        }
                        (report.status, report.schema, report.summary)
                    }
                    Err(err) => {
                        needs_review_count += 1;
                        ("needs_review".to_string(), None, vec![err])
                    }
                }
            } else if exists {
                ("present".to_string(), None, Vec::new())
            } else {
                ("missing".to_string(), None, Vec::new())
            };
            format!(
                "{{\n      \"path\": {},\n      \"kind\": {},\n      \"package\": {},\n      \"required\": {},\n      \"exists\": {},\n      \"status\": {},\n      \"schema\": {},\n      \"summary\": {}\n    }}",
                json_str(&entry.path.display().to_string()),
                json_str(&entry.kind),
                json_optional_str(entry.package.as_deref()),
                json_bool(entry.required),
                json_bool(exists),
                json_str(&status),
                json_optional_str(schema.as_deref()),
                json_array(&summary),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let complete = missing_count == 0 && needs_review_count == 0;
    let next_commands = vec![
        format!("hoi4skill large-mod-ci-plan --mod-root {root} --output .hoi4skill/ci_plan.json"),
        format!("hoi4skill large-mod-dependency-graph --mod-root {root} --output .hoi4skill/dependency_graph.json"),
        format!("hoi4skill large-mod-milestone-plan --mod-root {root} --output .hoi4skill/milestone_plan.json"),
        format!("hoi4skill large-mod-execution-queue --mod-root {root} --output .hoi4skill/execution_queue.json"),
        format!("hoi4skill large-mod-next-actions --mod-root {root} --output .hoi4skill/next_actions.json"),
        format!("hoi4skill large-mod-risk-register --mod-root {root} --output .hoi4skill/risk_register.json"),
        format!("hoi4skill large-mod-dashboard --mod-root {root} --output .hoi4skill/dashboard.md"),
        format!("hoi4skill large-mod-production-snapshot --mod-root {root} --output .hoi4skill/production_snapshot.json"),
        format!("hoi4skill large-mod-production-brief --mod-root {root} --output .hoi4skill/production_brief.md"),
        format!("hoi4skill large-mod-evidence-pack --mod-root {root} --output .hoi4skill/evidence_pack.json"),
        format!("hoi4skill large-mod-review-brief --mod-root {root} --output .hoi4skill/review_brief.md"),
        format!("hoi4skill large-mod-release-bundle --mod-root {root} --output .hoi4skill/release_bundle.json"),
    ];
    Ok(format!(
        "{{\n  \"schema\": \"hoi4skill.large_mod_evidence_pack.v1\",\n  \"complete\": {},\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"package_count\": {},\n  \"file_count\": {},\n  \"missing_count\": {},\n  \"needs_review_count\": {},\n  \"files\": [\n{}\n  ],\n  \"next_commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_bool(complete),
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        packages.len(),
        entries.len(),
        missing_count,
        needs_review_count,
        file_json,
        json_array(&next_commands),
        json_array(&large_mod_evidence_stop_conditions()),
    ))
}

fn push_evidence_entry(
    entries: &mut Vec<EvidenceFileEntry>,
    seen: &mut BTreeSet<String>,
    entry: EvidenceFileEntry,
) {
    let key = normalize_boundary_slashes(&entry.path.display().to_string());
    if seen.insert(key) {
        entries.push(entry);
    }
}

fn package_id_from_report_path(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    if name == "error_log_report.json" {
        return None;
    }
    for prefix in [
        "boundary_",
        "status_",
        "validation_",
        "error_log_",
        "merge_gate_",
        "review_",
        "plan_",
    ] {
        if let Some(rest) = name.strip_prefix(prefix) {
            return Some(rest.trim_end_matches(".json").to_string());
        }
    }
    None
}

fn large_mod_evidence_stop_conditions() -> Vec<String> {
    vec![
        "Do not treat the evidence pack as complete while any required file is missing."
            .to_string(),
        "Do not release while any included report needs review.".to_string(),
        "Keep package handoff files with the evidence pack before cross-agent review.".to_string(),
    ]
}

fn large_mod_review_brief_markdown(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    extra_reports: &[PathBuf],
) -> Result<String, String> {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let base = mod_root
        .map(|root| root.join(".hoi4skill"))
        .unwrap_or_else(|| PathBuf::from(".hoi4skill"));
    let package_summaries = packages
        .iter()
        .map(|package| work_package_readiness_summary(package, mod_root))
        .collect::<Vec<_>>();
    let ready_count = package_summaries
        .iter()
        .filter(|package| package.ready)
        .count();
    let blocked_packages = package_summaries
        .iter()
        .filter(|package| !package.ready)
        .collect::<Vec<_>>();

    let mut required_rows = Vec::new();
    let mut missing_required = Vec::new();
    let mut blocking_reports = Vec::new();
    for name in large_mod_required_release_reports() {
        let path = base.join(&name);
        if !path.exists() {
            missing_required.push(name.clone());
            required_rows.push((name, path, "missing".to_string(), String::new()));
            continue;
        }
        let report = read_work_package_report_status(&path)?;
        if report.status == "needs_review" {
            blocking_reports.push(name.clone());
        }
        required_rows.push((name, path, report.status, report.summary.join("; ")));
    }

    let mut package_report_rows = Vec::new();
    for path in extra_reports {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if required_rows
            .iter()
            .any(|(required, _, _, _)| required == name)
        {
            continue;
        }
        let report = read_work_package_report_status(path)?;
        if report.status == "needs_review" {
            blocking_reports.push(name.to_string());
        }
        package_report_rows.push((
            name.to_string(),
            path.to_path_buf(),
            report.status,
            report.summary.join("; "),
        ));
    }

    let release_ready =
        missing_required.is_empty() && blocking_reports.is_empty() && blocked_packages.is_empty();
    let mut out = String::new();
    out.push_str(&format!("# Large Mod Review Brief: {}\n\n", blueprint.name));
    out.push_str("- schema: `hoi4skill.large_mod_review_brief.v1`\n");
    out.push_str(&format!(
        "- decision: `{}`\n",
        if release_ready {
            "release_ready"
        } else {
            "blocked"
        }
    ));
    out.push_str(&format!("- acronym: `{}`\n", blueprint.acronym));
    out.push_str(&format!("- mod_root: `{}`\n", root));
    out.push_str(&format!("- blueprint: `{}`\n", blueprint_path.display()));
    out.push_str(&format!(
        "- packages: `{}` ready, `{}` blocked, `{}` total\n",
        ready_count,
        blocked_packages.len(),
        package_summaries.len()
    ));
    out.push_str(&format!(
        "- reports: `{}` missing required, `{}` needs review\n",
        missing_required.len(),
        blocking_reports.len()
    ));

    out.push_str("\n## Review Findings\n\n");
    if release_ready {
        out.push_str("- No blocking findings found in the current report set.\n");
    } else {
        for name in &missing_required {
            out.push_str(&format!("- Missing required report: `{name}`\n"));
        }
        for name in &blocking_reports {
            out.push_str(&format!("- Report needs review: `{name}`\n"));
        }
        for package in &blocked_packages {
            let missing = if package.missing.is_empty() {
                "none".to_string()
            } else {
                package.missing.join(", ")
            };
            let blocking = if package.blocking.is_empty() {
                "none".to_string()
            } else {
                package.blocking.join(", ")
            };
            out.push_str(&format!(
                "- Package `{}` blocked: missing [{}], blocking [{}]\n",
                package.id, missing, blocking
            ));
        }
    }

    out.push_str("\n## Required Evidence\n\n");
    out.push_str("| Report | Status | Path | Summary |\n");
    out.push_str("| --- | --- | --- | --- |\n");
    for (name, path, status, summary) in &required_rows {
        out.push_str(&format!(
            "| `{}` | `{}` | `{}` | {} |\n",
            name,
            status,
            path.display(),
            markdown_table_cell(summary)
        ));
    }

    out.push_str("\n## Package Readiness\n\n");
    out.push_str("| Package | Title | State | Missing | Blocking | Handoff |\n");
    out.push_str("| --- | --- | --- | --- | --- | --- |\n");
    for package in &package_summaries {
        out.push_str(&format!(
            "| `{}` | {} | `{}` | {} | {} | `{}` |\n",
            package.id,
            markdown_table_cell(&package.title),
            if package.ready { "ready" } else { "blocked" },
            markdown_table_cell(&package.missing.join(", ")),
            markdown_table_cell(&package.blocking.join(", ")),
            package.handoff_path.display()
        ));
    }

    if !package_report_rows.is_empty() {
        out.push_str("\n## Package Reports\n\n");
        out.push_str("| Report | Status | Path | Summary |\n");
        out.push_str("| --- | --- | --- | --- |\n");
        for (name, path, status, summary) in &package_report_rows {
            out.push_str(&format!(
                "| `{}` | `{}` | `{}` | {} |\n",
                name,
                status,
                path.display(),
                markdown_table_cell(summary)
            ));
        }
    }

    out.push_str("\n## Reviewer Commands\n\n");
    for command in large_mod_review_brief_next_commands(&root) {
        out.push_str(&format!("- `{command}`\n"));
    }

    out.push_str("\n## Stop Conditions\n\n");
    out.push_str("- Do not approve while the decision is `blocked`.\n");
    out.push_str(
        "- Do not approve missing required reports or package artifacts as acceptable partial work.\n",
    );
    out.push_str("- Use the evidence pack and package handoffs as the review attachment list.\n");
    Ok(out)
}

fn large_mod_review_brief_next_commands(mod_root: &str) -> Vec<String> {
    vec![
        format!("hoi4skill large-mod-next-actions --mod-root {mod_root} --output .hoi4skill/next_actions.json"),
        format!("hoi4skill large-mod-risk-register --mod-root {mod_root} --output .hoi4skill/risk_register.json"),
        format!("hoi4skill large-mod-dependency-graph --mod-root {mod_root} --output .hoi4skill/dependency_graph.json"),
        format!("hoi4skill large-mod-milestone-plan --mod-root {mod_root} --output .hoi4skill/milestone_plan.json"),
        format!("hoi4skill large-mod-execution-queue --mod-root {mod_root} --output .hoi4skill/execution_queue.json"),
        format!("hoi4skill large-mod-evidence-pack --mod-root {mod_root} --output .hoi4skill/evidence_pack.json"),
        format!("hoi4skill large-mod-dashboard --mod-root {mod_root} --output .hoi4skill/dashboard.md"),
        format!("hoi4skill large-mod-review-brief --mod-root {mod_root} --output .hoi4skill/review_brief.md"),
        format!("hoi4skill large-mod-dispatch-gate --mod-root {mod_root} --output .hoi4skill/dispatch_gate.json"),
        format!("hoi4skill large-mod-release-gate --mod-root {mod_root} --output .hoi4skill/release_gate.json"),
        format!("hoi4skill large-mod-release-bundle --mod-root {mod_root} --output .hoi4skill/release_bundle.json"),
        format!("hoi4skill large-mod-release-notes --mod-root {mod_root} --output .hoi4skill/release_notes.md"),
    ]
}

fn large_mod_release_bundle_json(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    extra_reports: &[PathBuf],
) -> Result<String, String> {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let entries = large_mod_release_bundle_entries(packages, mod_root, extra_reports);
    let mut missing_required = Vec::new();
    let mut review_required = Vec::new();
    let mut needs_review_count = 0usize;
    let file_json = entries
        .iter()
        .map(|entry| {
            let (exists, status, schema, summary) = large_mod_release_entry_status(entry);
            if entry.required && !exists {
                missing_required.push(entry.path.display().to_string());
            }
            if status == "needs_review" {
                needs_review_count += 1;
                if entry.required {
                    review_required.push(entry.path.display().to_string());
                }
            }
            format!(
                "{{\n      \"path\": {},\n      \"kind\": {},\n      \"package\": {},\n      \"required\": {},\n      \"exists\": {},\n      \"status\": {},\n      \"schema\": {},\n      \"summary\": {}\n    }}",
                json_str(&entry.path.display().to_string()),
                json_str(&entry.kind),
                json_optional_str(entry.package.as_deref()),
                json_bool(entry.required),
                json_bool(exists),
                json_str(&status),
                json_optional_str(schema.as_deref()),
                json_array(&summary),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let release_candidate = missing_required.is_empty() && review_required.is_empty();
    let next_commands = vec![
        format!("hoi4skill large-mod-ci-plan --mod-root {root} --output .hoi4skill/ci_plan.json"),
        format!("hoi4skill large-mod-dispatch-gate --mod-root {root} --output .hoi4skill/dispatch_gate.json"),
        format!("hoi4skill work-package-merge-gates --mod-root {root} --output-dir .hoi4skill/merge_gates --output .hoi4skill/merge_gates/manifest.json"),
        format!("hoi4skill large-mod-merge-gate --mod-root {root} --output .hoi4skill/merge_gate.json"),
        format!("hoi4skill large-mod-review-queue --mod-root {root} --output .hoi4skill/review_queue.json"),
        format!("hoi4skill large-mod-risk-register --mod-root {root} --output .hoi4skill/risk_register.json"),
        format!("hoi4skill large-mod-evidence-pack --mod-root {root} --output .hoi4skill/evidence_pack.json"),
        format!("hoi4skill large-mod-review-brief --mod-root {root} --output .hoi4skill/review_brief.md"),
        format!("hoi4skill large-mod-release-gate --mod-root {root} --output .hoi4skill/release_gate.json"),
        format!("hoi4skill large-mod-release-bundle --mod-root {root} --output .hoi4skill/release_bundle.json"),
        format!("hoi4skill large-mod-release-brief --mod-root {root} --output .hoi4skill/release_brief.md"),
        format!("hoi4skill large-mod-release-notes --mod-root {root} --output .hoi4skill/release_notes.md"),
        format!("hoi4skill large-mod-playtest-plan --mod-root {root} --output .hoi4skill/playtest_plan.json"),
    ];
    Ok(format!(
        "{{\n  \"schema\": \"hoi4skill.large_mod_release_bundle.v1\",\n  \"release_candidate\": {},\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"package_count\": {},\n  \"file_count\": {},\n  \"required_count\": {},\n  \"missing_required_count\": {},\n  \"needs_review_count\": {},\n  \"missing_required\": {},\n  \"review_required\": {},\n  \"files\": [\n{}\n  ],\n  \"next_commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_bool(release_candidate),
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        packages.len(),
        entries.len(),
        entries.iter().filter(|entry| entry.required).count(),
        missing_required.len(),
        needs_review_count,
        json_array(&missing_required),
        json_array(&review_required),
        file_json,
        json_array(&next_commands),
        json_array(&large_mod_release_bundle_stop_conditions()),
    ))
}

fn large_mod_release_brief_markdown(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    extra_reports: &[PathBuf],
) -> Result<String, String> {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let entries = large_mod_release_bundle_entries(packages, mod_root, extra_reports);
    let mut missing_required = Vec::new();
    let mut review_required = Vec::new();
    let mut kind_counts: BTreeMap<String, (usize, usize, usize, usize)> = BTreeMap::new();
    let mut rows = Vec::new();

    for entry in &entries {
        let (exists, status, _schema, summary) = large_mod_release_entry_status(entry);
        let counts = kind_counts
            .entry(entry.kind.clone())
            .or_insert((0, 0, 0, 0));
        counts.0 += 1;
        if entry.required {
            counts.1 += 1;
        }
        if entry.required && !exists {
            counts.2 += 1;
            missing_required.push(entry.path.display().to_string());
        }
        if status == "needs_review" {
            counts.3 += 1;
            if entry.required {
                review_required.push(entry.path.display().to_string());
            }
        }
        rows.push((
            entry.kind.clone(),
            entry.package.clone(),
            entry.required,
            status,
            entry.path.clone(),
            summary,
        ));
    }

    let release_candidate = missing_required.is_empty() && review_required.is_empty();
    let mut out = String::new();
    out.push_str(&format!(
        "# Large Mod Release Brief: {}\n\n",
        blueprint.name
    ));
    out.push_str("- schema: `hoi4skill.large_mod_release_brief.v1`\n");
    out.push_str(&format!(
        "- decision: `{}`\n",
        if release_candidate {
            "release_candidate"
        } else {
            "blocked"
        }
    ));
    out.push_str(&format!("- acronym: `{}`\n", blueprint.acronym));
    out.push_str(&format!("- mod_root: `{}`\n", root));
    out.push_str(&format!("- blueprint: `{}`\n", blueprint_path.display()));
    out.push_str(&format!("- packages: `{}`\n", packages.len()));
    out.push_str(&format!(
        "- artifacts: `{}` total, `{}` required, `{}` missing required, `{}` required needs review\n",
        entries.len(),
        entries.iter().filter(|entry| entry.required).count(),
        missing_required.len(),
        review_required.len()
    ));

    out.push_str("\n## Release Findings\n\n");
    if release_candidate {
        out.push_str("- No blocking release-bundle findings found in the current artifact set.\n");
    } else {
        for path in &missing_required {
            out.push_str(&format!("- Missing required artifact: `{path}`\n"));
        }
        for path in &review_required {
            out.push_str(&format!("- Required artifact needs review: `{path}`\n"));
        }
    }

    out.push_str("\n## Artifact Groups\n\n");
    out.push_str("| Kind | Files | Required | Missing Required | Needs Review |\n");
    out.push_str("| --- | ---: | ---: | ---: | ---: |\n");
    for (kind, (files, required, missing, needs_review)) in &kind_counts {
        out.push_str(&format!(
            "| `{kind}` | `{files}` | `{required}` | `{missing}` | `{needs_review}` |\n"
        ));
    }

    out.push_str("\n## Required Artifacts\n\n");
    out.push_str("| Kind | Package | Status | Path | Summary |\n");
    out.push_str("| --- | --- | --- | --- | --- |\n");
    for (kind, package, _required, status, path, summary) in
        rows.iter().filter(|(_, _, required, _, _, _)| *required)
    {
        out.push_str(&format!(
            "| `{}` | `{}` | `{}` | `{}` | {} |\n",
            kind,
            package.as_deref().unwrap_or("-"),
            status,
            path.display(),
            markdown_table_cell(&summary.join("; "))
        ));
    }

    out.push_str("\n## Reviewer Commands\n\n");
    for command in large_mod_release_brief_next_commands(&root) {
        out.push_str(&format!("- `{command}`\n"));
    }

    out.push_str("\n## Stop Conditions\n\n");
    out.push_str("- Do not publish while the decision is `blocked`.\n");
    out.push_str("- Do not omit package handoff files from the release attachment set.\n");
    out.push_str(
        "- Regenerate this brief after changing any package artifact, gate, or review report.\n",
    );
    Ok(out)
}

fn large_mod_release_brief_next_commands(mod_root: &str) -> Vec<String> {
    vec![
        format!("hoi4skill large-mod-release-bundle --mod-root {mod_root} --output .hoi4skill/release_bundle.json"),
        format!("hoi4skill large-mod-release-brief --mod-root {mod_root} --output .hoi4skill/release_brief.md"),
        format!("hoi4skill large-mod-release-gate --mod-root {mod_root} --output .hoi4skill/release_gate.json"),
        format!("hoi4skill large-mod-evidence-pack --mod-root {mod_root} --output .hoi4skill/evidence_pack.json"),
        format!("hoi4skill large-mod-review-brief --mod-root {mod_root} --output .hoi4skill/review_brief.md"),
        format!("hoi4skill large-mod-playtest-plan --mod-root {mod_root} --output .hoi4skill/playtest_plan.json"),
        format!("hoi4skill large-mod-dashboard --mod-root {mod_root} --output .hoi4skill/dashboard.md"),
    ]
}

fn large_mod_release_bundle_entries(
    packages: &[WorkPackage],
    mod_root: Option<&Path>,
    extra_reports: &[PathBuf],
) -> Vec<EvidenceFileEntry> {
    let base = mod_root
        .map(|root| root.join(".hoi4skill"))
        .unwrap_or_else(|| PathBuf::from(".hoi4skill"));
    let mut entries = Vec::new();
    let mut seen = BTreeSet::new();

    for name in large_mod_required_release_reports() {
        let kind = large_mod_required_release_report_kind(&name);
        push_evidence_entry(
            &mut entries,
            &mut seen,
            EvidenceFileEntry {
                path: base.join(name),
                kind,
                package: None,
                required: true,
                report_like: true,
            },
        );
    }
    for (name, kind, report_like) in large_mod_release_bundle_artifacts() {
        push_evidence_entry(
            &mut entries,
            &mut seen,
            EvidenceFileEntry {
                path: base.join(name),
                kind,
                package: None,
                required: true,
                report_like,
            },
        );
    }
    for package in packages {
        for artifact in work_package_readiness_artifacts(package, mod_root) {
            push_evidence_entry(
                &mut entries,
                &mut seen,
                EvidenceFileEntry {
                    path: artifact.path,
                    kind: format!("package_{}", artifact.label),
                    package: Some(package.id.clone()),
                    required: true,
                    report_like: artifact.report_like,
                },
            );
        }
        push_evidence_entry(
            &mut entries,
            &mut seen,
            EvidenceFileEntry {
                path: base.join(format!("handoff_{}.md", package.id)),
                kind: "package_handoff".to_string(),
                package: Some(package.id.clone()),
                required: true,
                report_like: false,
            },
        );
    }
    for report in extra_reports {
        push_evidence_entry(
            &mut entries,
            &mut seen,
            EvidenceFileEntry {
                path: report.to_path_buf(),
                kind: "extra_report".to_string(),
                package: package_id_from_report_path(report),
                required: false,
                report_like: true,
            },
        );
    }
    entries
}

fn large_mod_required_release_report_kind(name: &str) -> String {
    match name {
        "regression_gate.json" => "regression_gate".to_string(),
        _ => "release_report".to_string(),
    }
}

fn large_mod_release_entry_status(
    entry: &EvidenceFileEntry,
) -> (bool, String, Option<String>, Vec<String>) {
    let exists = entry.path.exists();
    if exists && entry.report_like {
        match read_work_package_report_status(&entry.path) {
            Ok(report) => (exists, report.status, report.schema, report.summary),
            Err(err) => (exists, "needs_review".to_string(), None, vec![err]),
        }
    } else if exists {
        (exists, "present".to_string(), None, Vec::new())
    } else {
        (exists, "missing".to_string(), None, Vec::new())
    }
}

fn large_mod_playtest_plan_json(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
) -> String {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let summaries = packages
        .iter()
        .map(|package| (package, work_package_readiness_summary(package, mod_root)))
        .collect::<Vec<_>>();
    let ready_for_playtest_count = summaries
        .iter()
        .filter(|(_, summary)| summary.ready && summary.handoff_path.exists())
        .count();
    let handoff_pending_count = summaries
        .iter()
        .filter(|(_, summary)| summary.ready && !summary.handoff_path.exists())
        .count();
    let blocked_count = summaries
        .iter()
        .filter(|(_, summary)| !summary.ready)
        .count();
    let scenario_json = summaries
        .iter()
        .enumerate()
        .map(|(idx, (package, summary))| {
            let status = large_mod_playtest_status(summary);
            let focus = large_mod_playtest_focus(package);
            let commands = large_mod_playtest_package_commands(package, summary, &root);
            format!(
                "{{\n      \"scenario_index\": {},\n      \"id\": {},\n      \"kind\": {},\n      \"title\": {},\n      \"status\": {},\n      \"missing_artifacts\": {},\n      \"blocking_artifacts\": {},\n      \"handoff\": {},\n      \"changed_files\": {},\n      \"playtest_focus\": {},\n      \"commands\": {}\n    }}",
                idx + 1,
                json_str(&summary.id),
                json_str(&summary.kind),
                json_str(&summary.title),
                json_str(&status),
                json_array(&summary.missing),
                json_array(&summary.blocking),
                json_str(&summary.handoff_path.display().to_string()),
                json_str(&summary.changed_path.display().to_string()),
                json_array(&focus),
                json_array(&commands),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let global_commands = vec![
        format!("hoi4skill validate {root} --strict-code-index --output .hoi4skill/validation.json"),
        format!("hoi4skill loc-audit {root} --output .hoi4skill/loc_audit.json"),
        format!("hoi4skill gfx-audit {root} --output .hoi4skill/gfx_audit.json"),
        format!("hoi4skill logic-audit {root} --output .hoi4skill/logic_audit.json"),
        format!("hoi4skill analyze-error-log --input <error.log> --mod-root {root} --output .hoi4skill/error_log_report.json"),
        format!("hoi4skill large-mod-release-bundle --mod-root {root} --output .hoi4skill/release_bundle.json"),
        format!("hoi4skill large-mod-release-brief --mod-root {root} --output .hoi4skill/release_brief.md"),
        format!("hoi4skill large-mod-playtest-gate --mod-root {root} --output .hoi4skill/playtest_gate.json"),
        format!("hoi4skill large-mod-playtest-brief --mod-root {root} --output .hoi4skill/playtest_brief.md"),
    ];
    format!(
        "{{\n  \"schema\": \"hoi4skill.large_mod_playtest_plan.v1\",\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"package_count\": {},\n  \"scenario_count\": {},\n  \"ready_for_playtest_count\": {},\n  \"handoff_pending_count\": {},\n  \"blocked_count\": {},\n  \"scenarios\": [\n{}\n  ],\n  \"global_commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        packages.len(),
        summaries.len(),
        ready_for_playtest_count,
        handoff_pending_count,
        blocked_count,
        scenario_json,
        json_array(&global_commands),
        json_array(&large_mod_playtest_stop_conditions()),
    )
}

fn large_mod_playtest_status(summary: &WorkPackageReadinessSummary) -> String {
    if summary.ready && summary.handoff_path.exists() {
        "ready_for_playtest".to_string()
    } else if summary.ready {
        "handoff_pending".to_string()
    } else {
        "blocked".to_string()
    }
}

fn large_mod_playtest_focus(package: &WorkPackage) -> Vec<String> {
    match package.kind.as_str() {
        "country" => vec![
            "country_selection_smoke".to_string(),
            "focus_tree_start_and_first_branch".to_string(),
            "events_decisions_and_spirits_surface".to_string(),
            "localisation_and_goal_icons_visible".to_string(),
        ],
        "region" => vec![
            "regional_integration_smoke".to_string(),
            "cross_country_event_targets".to_string(),
            "decision_visibility_and_ai_weights".to_string(),
            "no_out_of_scope_country_state_or_tag_creation".to_string(),
        ],
        "system" => vec![
            "system_regression_smoke".to_string(),
            "scripted_effect_trigger_contracts".to_string(),
            "on_action_or_decision_entry_points".to_string(),
            "save_load_and_error_log_cleanliness".to_string(),
        ],
        _ => vec!["package_smoke".to_string()],
    }
}

fn large_mod_playtest_package_commands(
    package: &WorkPackage,
    summary: &WorkPackageReadinessSummary,
    mod_root: &str,
) -> Vec<String> {
    vec![
        format!(
            "hoi4skill work-package-review-checklist --mod-root {mod_root} --package {} --output .hoi4skill/review_checklist_{}.md",
            package.id, package.id
        ),
        format!(
            "hoi4skill work-package-merge-gate --mod-root {mod_root} --package {} --output .hoi4skill/merge_gate_{}.json",
            package.id, package.id
        ),
        format!(
            "hoi4skill validate {mod_root} --changed-only --changed-file {} --strict-code-index --output .hoi4skill/validation_{}.json",
            summary.changed_path.display(),
            package.id
        ),
        format!("hoi4skill analyze-error-log --input <error.log> --mod-root {mod_root} --changed-only --changed-file {} --output .hoi4skill/error_log_{}.json", summary.changed_path.display(), package.id),
    ]
}

fn large_mod_playtest_stop_conditions() -> Vec<String> {
    vec![
        "Do not schedule blocked packages for playtest before required artifacts are present."
            .to_string(),
        "Do not treat handoff_pending packages as ready for external QA.".to_string(),
        "Attach validation and error-log reports to every completed playtest scenario.".to_string(),
    ]
}

fn large_mod_playtest_gate_json(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    reports: &[PathBuf],
) -> Result<String, String> {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let mut reports_by_package: BTreeMap<String, PathBuf> = BTreeMap::new();
    for report in reports {
        if let Some(package) = package_id_from_playtest_report_path(report) {
            reports_by_package.insert(package, report.to_path_buf());
        }
    }

    let mut passed_count = 0usize;
    let mut missing_report_count = 0usize;
    let mut needs_review_count = 0usize;
    let mut blocked_package_count = 0usize;
    let package_json = packages
        .iter()
        .map(|package| {
            let summary = work_package_readiness_summary(package, mod_root);
            let mut blockers = Vec::new();
            if !summary.ready {
                blocked_package_count += 1;
                blockers.push("package_not_ready".to_string());
            }
            if !summary.handoff_path.exists() {
                blockers.push("missing_handoff".to_string());
            }
            let report_path = reports_by_package
                .get(&package.id)
                .cloned()
                .unwrap_or_else(|| large_mod_default_playtest_report_path(package, mod_root));
            let mut report_summary = Vec::new();
            let report_status = if report_path.exists() {
                match read_work_package_report_status(&report_path) {
                    Ok(report) => {
                        report_summary = report.summary;
                        if report.status == "needs_review" {
                            needs_review_count += 1;
                            blockers.push("playtest_needs_review".to_string());
                        }
                        report.status
                    }
                    Err(err) => {
                        needs_review_count += 1;
                        report_summary.push(err);
                        blockers.push("playtest_needs_review".to_string());
                        "needs_review".to_string()
                    }
                }
            } else {
                missing_report_count += 1;
                blockers.push("missing_playtest_report".to_string());
                "missing".to_string()
            };
            let gate_status = if blockers.is_empty() {
                passed_count += 1;
                "passed"
            } else {
                "blocked"
            };
            format!(
                "{{\n      \"id\": {},\n      \"kind\": {},\n      \"title\": {},\n      \"gate_status\": {},\n      \"readiness\": {},\n      \"handoff\": {},\n      \"playtest_report\": {},\n      \"playtest_report_status\": {},\n      \"blockers\": {},\n      \"report_summary\": {},\n      \"next_commands\": {}\n    }}",
                json_str(&package.id),
                json_str(&package.kind),
                json_str(&package.title),
                json_str(gate_status),
                json_str(if summary.ready { "ready" } else { "blocked" }),
                json_str(&summary.handoff_path.display().to_string()),
                json_str(&report_path.display().to_string()),
                json_str(&report_status),
                json_array(&blockers),
                json_array(&report_summary),
                json_array(&large_mod_playtest_gate_package_commands(package, &summary, &root)),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let playtest_complete = passed_count == packages.len();
    let next_commands = vec![
        format!("hoi4skill large-mod-playtest-plan --mod-root {root} --output .hoi4skill/playtest_plan.json"),
        format!("hoi4skill large-mod-playtest-gate --mod-root {root} --output .hoi4skill/playtest_gate.json"),
        format!("hoi4skill large-mod-playtest-brief --mod-root {root} --output .hoi4skill/playtest_brief.md"),
        format!("hoi4skill large-mod-release-bundle --mod-root {root} --output .hoi4skill/release_bundle.json"),
        format!("hoi4skill large-mod-release-brief --mod-root {root} --output .hoi4skill/release_brief.md"),
        format!("hoi4skill large-mod-release-gate --mod-root {root} --output .hoi4skill/release_gate.json"),
    ];
    Ok(format!(
        "{{\n  \"schema\": \"hoi4skill.large_mod_playtest_gate.v1\",\n  \"playtest_complete\": {},\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"package_count\": {},\n  \"passed_count\": {},\n  \"blocked_package_count\": {},\n  \"missing_report_count\": {},\n  \"needs_review_count\": {},\n  \"packages\": [\n{}\n  ],\n  \"next_commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_bool(playtest_complete),
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        packages.len(),
        passed_count,
        blocked_package_count,
        missing_report_count,
        needs_review_count,
        package_json,
        json_array(&next_commands),
        json_array(&large_mod_playtest_gate_stop_conditions()),
    ))
}

fn large_mod_default_playtest_report_path(
    package: &WorkPackage,
    mod_root: Option<&Path>,
) -> PathBuf {
    mod_root
        .map(|root| root.join(".hoi4skill"))
        .unwrap_or_else(|| PathBuf::from(".hoi4skill"))
        .join(format!("playtest_{}.json", package.id))
}

fn package_id_from_playtest_report_path(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    name.strip_prefix("playtest_")?
        .strip_suffix(".json")
        .map(str::to_string)
}

fn large_mod_playtest_gate_package_commands(
    package: &WorkPackage,
    summary: &WorkPackageReadinessSummary,
    mod_root: &str,
) -> Vec<String> {
    vec![
        format!("hoi4skill large-mod-playtest-plan --mod-root {mod_root} --output .hoi4skill/playtest_plan.json"),
        format!(
            "hoi4skill validate {mod_root} --changed-only --changed-file {} --strict-code-index --output .hoi4skill/validation_{}.json",
            summary.changed_path.display(),
            package.id
        ),
        format!("hoi4skill analyze-error-log --input <error.log> --mod-root {mod_root} --changed-only --changed-file {} --output .hoi4skill/error_log_{}.json", summary.changed_path.display(), package.id),
        format!(
            "hoi4skill large-mod-playtest-gate --mod-root {mod_root} --output .hoi4skill/playtest_gate.json"
        ),
    ]
}

fn large_mod_playtest_gate_stop_conditions() -> Vec<String> {
    vec![
        "Do not treat playtest as complete while any package gate_status is blocked.".to_string(),
        "Do not accept missing playtest reports for ready packages.".to_string(),
        "Regenerate release bundle and release brief after playtest gate changes.".to_string(),
    ]
}

fn large_mod_playtest_brief_markdown(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    reports: &[PathBuf],
) -> Result<String, String> {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let mut reports_by_package: BTreeMap<String, PathBuf> = BTreeMap::new();
    for report in reports {
        if let Some(package) = package_id_from_playtest_report_path(report) {
            reports_by_package.insert(package, report.to_path_buf());
        }
    }

    let mut passed_count = 0usize;
    let mut missing_report_count = 0usize;
    let mut needs_review_count = 0usize;
    let mut blocked_package_count = 0usize;
    let mut rows = Vec::new();
    let mut findings = Vec::new();

    for package in packages {
        let summary = work_package_readiness_summary(package, mod_root);
        let mut blockers = Vec::new();
        if !summary.ready {
            blocked_package_count += 1;
            blockers.push("package_not_ready".to_string());
        }
        if !summary.handoff_path.exists() {
            blockers.push("missing_handoff".to_string());
        }
        let report_path = reports_by_package
            .get(&package.id)
            .cloned()
            .unwrap_or_else(|| large_mod_default_playtest_report_path(package, mod_root));
        let mut report_summary = Vec::new();
        let report_status = if report_path.exists() {
            match read_work_package_report_status(&report_path) {
                Ok(report) => {
                    report_summary = report.summary;
                    if report.status == "needs_review" {
                        needs_review_count += 1;
                        blockers.push("playtest_needs_review".to_string());
                    }
                    report.status
                }
                Err(err) => {
                    needs_review_count += 1;
                    report_summary.push(err);
                    blockers.push("playtest_needs_review".to_string());
                    "needs_review".to_string()
                }
            }
        } else {
            missing_report_count += 1;
            blockers.push("missing_playtest_report".to_string());
            "missing".to_string()
        };
        let gate_status = if blockers.is_empty() {
            passed_count += 1;
            "passed"
        } else {
            for blocker in &blockers {
                findings.push(format!("{}: {}", package.id, blocker));
            }
            "blocked"
        };
        rows.push((
            package.id.clone(),
            package.kind.clone(),
            package.title.clone(),
            gate_status.to_string(),
            if summary.ready { "ready" } else { "blocked" }.to_string(),
            report_status,
            report_path,
            blockers,
            report_summary,
        ));
    }

    let playtest_complete = passed_count == packages.len();
    let mut out = String::new();
    out.push_str(&format!(
        "# Large Mod Playtest Brief: {}\n\n",
        blueprint.name
    ));
    out.push_str("- schema: `hoi4skill.large_mod_playtest_brief.v1`\n");
    out.push_str(&format!(
        "- decision: `{}`\n",
        if playtest_complete {
            "playtest_complete"
        } else {
            "blocked"
        }
    ));
    out.push_str(&format!("- acronym: `{}`\n", blueprint.acronym));
    out.push_str(&format!("- mod_root: `{}`\n", root));
    out.push_str(&format!("- blueprint: `{}`\n", blueprint_path.display()));
    out.push_str(&format!(
        "- packages: `{}` passed, `{}` blocked package readiness, `{}` missing playtest reports, `{}` needs review\n",
        passed_count,
        blocked_package_count,
        missing_report_count,
        needs_review_count
    ));

    out.push_str("\n## Playtest Findings\n\n");
    if findings.is_empty() {
        out.push_str("- No blocking playtest findings found in the current report set.\n");
    } else {
        for finding in &findings {
            out.push_str(&format!("- `{finding}`\n"));
        }
    }

    out.push_str("\n## Package Playtest Status\n\n");
    out.push_str(
        "| Package | Kind | Gate | Readiness | Report | Report Path | Blockers | Summary |\n",
    );
    out.push_str("| --- | --- | --- | --- | --- | --- | --- | --- |\n");
    for (id, kind, _title, gate_status, readiness, report_status, report_path, blockers, summary) in
        &rows
    {
        out.push_str(&format!(
            "| `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | {} | {} |\n",
            id,
            kind,
            gate_status,
            readiness,
            report_status,
            report_path.display(),
            markdown_table_cell(&blockers.join(", ")),
            markdown_table_cell(&summary.join("; "))
        ));
    }

    out.push_str("\n## Reviewer Commands\n\n");
    for command in large_mod_playtest_brief_next_commands(&root) {
        out.push_str(&format!("- `{command}`\n"));
    }

    out.push_str("\n## Stop Conditions\n\n");
    out.push_str("- Do not approve playtest while the decision is `blocked`.\n");
    out.push_str("- Do not accept missing package playtest reports as implicit passes.\n");
    out.push_str("- Regenerate release bundle and release brief after playtest changes.\n");
    Ok(out)
}

fn large_mod_playtest_brief_next_commands(mod_root: &str) -> Vec<String> {
    vec![
        format!("hoi4skill large-mod-playtest-plan --mod-root {mod_root} --output .hoi4skill/playtest_plan.json"),
        format!("hoi4skill large-mod-playtest-gate --mod-root {mod_root} --output .hoi4skill/playtest_gate.json"),
        format!("hoi4skill large-mod-playtest-brief --mod-root {mod_root} --output .hoi4skill/playtest_brief.md"),
        format!("hoi4skill large-mod-release-bundle --mod-root {mod_root} --output .hoi4skill/release_bundle.json"),
        format!("hoi4skill large-mod-release-brief --mod-root {mod_root} --output .hoi4skill/release_brief.md"),
    ]
}

fn large_mod_release_notes_markdown(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    playtest_reports: &[PathBuf],
) -> Result<String, String> {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let mut reports_by_package: BTreeMap<String, PathBuf> = BTreeMap::new();
    for report in playtest_reports {
        if let Some(package) = package_id_from_playtest_report_path(report) {
            reports_by_package.insert(package, report.to_path_buf());
        }
    }

    let mut ready_count = 0usize;
    let mut handoff_count = 0usize;
    let mut playtest_passed_count = 0usize;
    let mut needs_review_count = 0usize;
    let mut rows = Vec::new();
    for package in packages {
        let summary = work_package_readiness_summary(package, mod_root);
        if summary.ready {
            ready_count += 1;
        }
        if summary.handoff_path.exists() {
            handoff_count += 1;
        }
        let report_path = reports_by_package
            .get(&package.id)
            .cloned()
            .unwrap_or_else(|| large_mod_default_playtest_report_path(package, mod_root));
        let (playtest_status, report_summary) = if report_path.exists() {
            match read_work_package_report_status(&report_path) {
                Ok(report) => {
                    if report.status == "ok" {
                        playtest_passed_count += 1;
                    } else {
                        needs_review_count += 1;
                    }
                    (report.status, report.summary)
                }
                Err(err) => {
                    needs_review_count += 1;
                    ("needs_review".to_string(), vec![err])
                }
            }
        } else {
            ("missing".to_string(), Vec::new())
        };
        rows.push((
            package.kind.clone(),
            package.id.clone(),
            package.title.clone(),
            if summary.ready { "ready" } else { "blocked" }.to_string(),
            if summary.handoff_path.exists() {
                "present"
            } else {
                "missing"
            }
            .to_string(),
            summary.handoff_path,
            playtest_status,
            report_path,
            report_summary,
        ));
    }

    let mut out = String::new();
    out.push_str(&format!("# Release Notes Draft: {}\n\n", blueprint.name));
    out.push_str("- schema: `hoi4skill.large_mod_release_notes.v1`\n");
    out.push_str("- status: `draft_requires_human_review`\n");
    out.push_str(&format!("- acronym: `{}`\n", blueprint.acronym));
    out.push_str(&format!("- mod_root: `{}`\n", root));
    out.push_str(&format!("- blueprint: `{}`\n", blueprint_path.display()));
    out.push_str(&format!(
        "- package evidence: `{}` ready, `{}` handoffs, `{}` playtest passed, `{}` playtest needs review\n",
        ready_count, handoff_count, playtest_passed_count, needs_review_count
    ));
    out.push_str("\n## Release Summary\n\n");
    out.push_str(&format!("- Mod: `{}`\n", blueprint.name));
    out.push_str(&format!("- Packages in scope: `{}`\n", packages.len()));
    out.push_str("- This draft is generated from package metadata, handoff files, and playtest reports only.\n");

    for kind in ["country", "system", "region"] {
        let matching = rows
            .iter()
            .filter(|(row_kind, _, _, _, _, _, _, _, _)| row_kind == kind)
            .collect::<Vec<_>>();
        if matching.is_empty() {
            continue;
        }
        out.push_str(&format!("\n## {} Packages\n\n", title_case_ascii(kind)));
        for (
            _,
            id,
            title,
            readiness,
            handoff,
            handoff_path,
            playtest_status,
            report_path,
            summary,
        ) in matching
        {
            out.push_str(&format!("- `{id}`: {title}\n"));
            out.push_str(&format!("  - readiness: `{readiness}`\n"));
            out.push_str(&format!(
                "  - handoff: `{handoff}` at `{}`\n",
                handoff_path.display()
            ));
            out.push_str(&format!(
                "  - playtest: `{playtest_status}` at `{}`\n",
                report_path.display()
            ));
            if !summary.is_empty() {
                out.push_str(&format!(
                    "  - evidence summary: {}\n",
                    markdown_table_cell(&summary.join("; "))
                ));
            }
        }
    }

    out.push_str("\n## Release Review Checklist\n\n");
    out.push_str("- Confirm player-facing wording and patch-note tone manually.\n");
    out.push_str("- Confirm every listed package has accepted handoff and playtest evidence.\n");
    out.push_str(
        "- Do not describe unimplemented gameplay beyond package titles and evidence summaries.\n",
    );

    out.push_str("\n## Reviewer Commands\n\n");
    for command in large_mod_release_notes_next_commands(&root) {
        out.push_str(&format!("- `{command}`\n"));
    }
    Ok(out)
}

fn title_case_ascii(raw: &str) -> String {
    let mut chars = raw.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

fn large_mod_release_notes_next_commands(mod_root: &str) -> Vec<String> {
    vec![
        format!("hoi4skill large-mod-playtest-gate --mod-root {mod_root} --output .hoi4skill/playtest_gate.json"),
        format!("hoi4skill large-mod-playtest-brief --mod-root {mod_root} --output .hoi4skill/playtest_brief.md"),
        format!("hoi4skill large-mod-release-bundle --mod-root {mod_root} --output .hoi4skill/release_bundle.json"),
        format!("hoi4skill large-mod-release-brief --mod-root {mod_root} --output .hoi4skill/release_brief.md"),
        format!("hoi4skill large-mod-release-notes --mod-root {mod_root} --output .hoi4skill/release_notes.md"),
    ]
}

fn work_package_playtest_report_json(
    blueprint: &LargeModBlueprint,
    package: &WorkPackage,
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    map: &ArgMap,
    result: &str,
    findings: &[String],
) -> String {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let ok = result == "passed";
    let status = if ok { "ok" } else { "needs_review" };
    let mut summary = repeated_values(map, "summary")
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if summary.is_empty() {
        summary.push(if ok {
            "package playtest passed".to_string()
        } else {
            "package playtest needs review".to_string()
        });
    }
    let evidence = repeated_values(map, "evidence")
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let focus = large_mod_playtest_focus(package);
    let next_commands = vec![
        format!(
            "hoi4skill large-mod-playtest-gate --mod-root {root} --output .hoi4skill/playtest_gate.json"
        ),
        format!(
            "hoi4skill large-mod-release-bundle --mod-root {root} --output .hoi4skill/release_bundle.json"
        ),
        format!(
            "hoi4skill large-mod-release-brief --mod-root {root} --output .hoi4skill/release_brief.md"
        ),
    ];
    format!(
        "{{\n  \"schema\": \"hoi4skill.playtest_report.v1\",\n  \"ok\": {},\n  \"status\": {},\n  \"result\": {},\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"package\": {},\n  \"kind\": {},\n  \"title\": {},\n  \"tester\": {},\n  \"summary\": {},\n  \"finding_count\": {},\n  \"findings\": {},\n  \"evidence\": {},\n  \"playtest_focus\": {},\n  \"validation_report\": {},\n  \"error_log_report\": {},\n  \"save_file\": {},\n  \"next_commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_bool(ok),
        json_str(status),
        json_str(result),
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        json_str(&package.id),
        json_str(&package.kind),
        json_str(&package.title),
        json_optional_str(value(map, "tester")),
        json_array(&summary),
        findings.len(),
        json_array(findings),
        json_array(&evidence),
        json_array(&focus),
        json_optional_str(value(map, "validation-report")),
        json_optional_str(value(map, "error-log-report")),
        json_optional_str(value(map, "save")),
        json_array(&next_commands),
        json_array(&work_package_playtest_report_stop_conditions()),
    )
}

fn normalize_playtest_result(raw: &str) -> Result<String, String> {
    match raw.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "passed" | "pass" | "ok" => Ok("passed".to_string()),
        "needs_review" | "review" | "failed" | "fail" | "blocked" => Ok("needs_review".to_string()),
        other => Err(format!(
            "--result expects passed or needs_review, got {other}"
        )),
    }
}

fn work_package_playtest_report_stop_conditions() -> Vec<String> {
    vec![
        "Do not mark a playtest report passed while findings remain open.".to_string(),
        "Attach validation and error-log evidence for release-candidate playtests.".to_string(),
        "Regenerate the large-mod playtest gate after writing or editing this report.".to_string(),
    ]
}

fn large_mod_release_bundle_artifacts() -> Vec<(String, String, bool)> {
    vec![
        (
            "ci_plan.json".to_string(),
            "coordination_report".to_string(),
            true,
        ),
        (
            "dispatch_gate.json".to_string(),
            "coordination_gate".to_string(),
            true,
        ),
        (
            "merge_gate.json".to_string(),
            "coordination_gate".to_string(),
            true,
        ),
        (
            "merge_gates/manifest.json".to_string(),
            "package_merge_gates".to_string(),
            true,
        ),
        (
            "review_queue.json".to_string(),
            "review_report".to_string(),
            true,
        ),
        (
            "risk_register.json".to_string(),
            "review_report".to_string(),
            true,
        ),
        (
            "evidence_pack.json".to_string(),
            "review_report".to_string(),
            true,
        ),
        (
            "review_brief.md".to_string(),
            "review_brief".to_string(),
            false,
        ),
        (
            "release_gate.json".to_string(),
            "release_gate".to_string(),
            true,
        ),
        (
            "dashboard.md".to_string(),
            "review_dashboard".to_string(),
            false,
        ),
        (
            "next_actions.json".to_string(),
            "review_report".to_string(),
            true,
        ),
        (
            "production_snapshot.json".to_string(),
            "production_snapshot".to_string(),
            true,
        ),
        (
            "production_brief.md".to_string(),
            "production_brief".to_string(),
            false,
        ),
        (
            "playtest_plan.json".to_string(),
            "playtest_report".to_string(),
            true,
        ),
        (
            "playtest_gate.json".to_string(),
            "playtest_gate".to_string(),
            true,
        ),
        (
            "playtest_brief.md".to_string(),
            "playtest_brief".to_string(),
            false,
        ),
        (
            "regression_plan.json".to_string(),
            "regression_plan".to_string(),
            true,
        ),
        (
            "regression_gate.json".to_string(),
            "regression_gate".to_string(),
            true,
        ),
        (
            "regression_brief.md".to_string(),
            "regression_brief".to_string(),
            false,
        ),
        (
            "release_notes.md".to_string(),
            "release_notes".to_string(),
            false,
        ),
    ]
}

fn large_mod_release_bundle_stop_conditions() -> Vec<String> {
    vec![
        "Do not publish a release candidate while release_candidate=false.".to_string(),
        "Do not omit package handoff files from the release bundle.".to_string(),
        "Regenerate the release bundle after changing any package artifact, gate, or review report."
            .to_string(),
    ]
}

fn work_package_review_checklist_markdown(
    blueprint: &LargeModBlueprint,
    package: &WorkPackage,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    claims_dir: &Path,
) -> Result<String, String> {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let summary = work_package_readiness_summary(package, mod_root);
    let claim = work_package_claim_summary(package, packages, mod_root, claims_dir);
    let handoff_exists = summary.handoff_path.exists();
    let claim_blocks_review =
        claim.claim_status == "blocked_claim" || claim.claim_status == "needs_review";
    let decision = if !summary.ready || claim_blocks_review {
        "blocked"
    } else if handoff_exists {
        "approved"
    } else {
        "ready_for_handoff"
    };
    let artifacts = work_package_readiness_artifacts(package, mod_root);
    let changed_files = artifacts
        .iter()
        .find(|artifact| artifact.label == "changed")
        .and_then(|artifact| {
            if artifact.path.exists() {
                read_utf8_lossy(&artifact.path).ok()
            } else {
                None
            }
        })
        .map(|text| {
            text.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let mut out = String::new();
    out.push_str(&format!(
        "# Work Package Review Checklist: {}\n\n",
        package.title
    ));
    out.push_str("- schema: `hoi4skill.work_package_review_checklist.v1`\n");
    out.push_str(&format!("- decision: `{decision}`\n"));
    out.push_str(&format!("- mod: `{}`\n", blueprint.name));
    out.push_str(&format!("- acronym: `{}`\n", blueprint.acronym));
    out.push_str(&format!("- mod_root: `{}`\n", root));
    out.push_str(&format!("- blueprint: `{}`\n", blueprint_path.display()));
    out.push_str(&format!("- package_id: `{}`\n", package.id));
    out.push_str(&format!("- package_kind: `{}`\n", package.kind));
    out.push_str(&format!(
        "- claim: `{}` by `{}`\n",
        claim.claim_status,
        claim.assignee.as_deref().unwrap_or("unassigned")
    ));
    out.push_str(&format!(
        "- handoff: `{}` at `{}`\n",
        if handoff_exists { "present" } else { "missing" },
        summary.handoff_path.display()
    ));

    out.push_str("\n## Acceptance Checks\n\n");
    out.push_str("| Check | Result | Evidence | Summary |\n");
    out.push_str("| --- | --- | --- | --- |\n");
    for artifact in artifacts {
        let (result, details) = if !artifact.path.exists() {
            ("missing".to_string(), String::new())
        } else if artifact.report_like {
            match read_work_package_report_status(&artifact.path) {
                Ok(report) => (report.status, report.summary.join("; ")),
                Err(err) => ("needs_review".to_string(), err),
            }
        } else {
            ("present".to_string(), String::new())
        };
        out.push_str(&format!(
            "| `{}` | `{}` | `{}` | {} |\n",
            artifact.label,
            result,
            artifact.path.display(),
            markdown_table_cell(&details)
        ));
    }
    out.push_str(&format!(
        "| `handoff` | `{}` | `{}` | {} |\n",
        if handoff_exists { "present" } else { "missing" },
        summary.handoff_path.display(),
        markdown_table_cell(if handoff_exists {
            "handoff artifact is ready for reviewer"
        } else {
            "generate handoff before approving package"
        })
    ));
    out.push_str(&format!(
        "| `claim` | `{}` | `{}` | {} |\n",
        claim.claim_status,
        claim.claim_path.display(),
        markdown_table_cell(claim.assignee.as_deref().unwrap_or("unassigned"))
    ));

    out.push_str("\n## Changed Files\n\n");
    if changed_files.is_empty() {
        out.push_str("- No changed-file list has been written for this package yet.\n");
    } else {
        for path in changed_files {
            out.push_str(&format!("- `{path}`\n"));
        }
    }

    out.push_str("\n## Allowed Edit Surface\n\n");
    for path in work_package_boundary_allowed_prefixes(package) {
        out.push_str(&format!("- `{path}`\n"));
    }

    out.push_str("\n## Required Fixes\n\n");
    if summary.missing.is_empty()
        && summary.blocking.is_empty()
        && handoff_exists
        && !claim_blocks_review
    {
        out.push_str("- No required fixes found by the checklist.\n");
    } else {
        for label in &summary.missing {
            out.push_str(&format!("- Missing `{label}` artifact.\n"));
        }
        for label in &summary.blocking {
            out.push_str(&format!("- `{label}` artifact needs review.\n"));
        }
        if claim_blocks_review {
            out.push_str(&format!(
                "- Claim status `{}` must be resolved before approval.\n",
                claim.claim_status
            ));
        }
        if !handoff_exists {
            out.push_str("- Missing `handoff` artifact.\n");
        }
    }

    out.push_str("\n## Next Commands\n\n");
    for command in work_package_review_checklist_next_commands(package, &root) {
        out.push_str(&format!("- `{command}`\n"));
    }

    out.push_str("\n## Stop Conditions\n\n");
    out.push_str("- Do not approve while decision is `blocked` or `ready_for_handoff`.\n");
    out.push_str(
        "- Do not approve if any report row is `needs_review` or any artifact row is `missing`.\n",
    );
    out.push_str("- Do not approve changed files outside the allowed edit surface; rerun boundary with `--strict-names`.\n");
    out.push_str("- Treat this checklist as review evidence, not as permission to bypass final validation.\n");
    Ok(out)
}

fn work_package_review_checklist_next_commands(
    package: &WorkPackage,
    mod_root: &str,
) -> Vec<String> {
    vec![
        format!(
            "hoi4skill work-package-review-checklist --mod-root {mod_root} --package {} --output .hoi4skill/review_checklist_{}.md",
            package.id, package.id
        ),
        format!(
            "hoi4skill check-work-package-boundary --mod-root {mod_root} --package {} --changed-file .hoi4skill/changed_{}.txt --strict-names --output .hoi4skill/boundary_{}.json",
            package.id, package.id, package.id
        ),
        format!(
            "hoi4skill work-package-handoff --mod-root {mod_root} --package {} --output .hoi4skill/handoff_{}.md",
            package.id, package.id
        ),
        format!(
            "hoi4skill work-package-merge-gate --mod-root {mod_root} --package {} --output .hoi4skill/merge_gate_{}.json",
            package.id, package.id
        ),
        format!("hoi4skill large-mod-dispatch-gate --mod-root {mod_root} --output .hoi4skill/dispatch_gate.json"),
        format!("hoi4skill large-mod-release-gate --mod-root {mod_root} --output .hoi4skill/release_gate.json"),
    ]
}

fn work_package_merge_gate_json(
    blueprint: &LargeModBlueprint,
    package: &WorkPackage,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    claims_dir: &Path,
) -> Result<String, String> {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let summary = work_package_readiness_summary(package, mod_root);
    let claim = work_package_claim_summary(package, packages, mod_root, claims_dir);
    let mut checks = Vec::new();
    let mut blocking_count = 0usize;

    for artifact in work_package_readiness_artifacts(package, mod_root) {
        let (ok, status, details) = if !artifact.path.exists() {
            (false, "missing".to_string(), String::new())
        } else if artifact.report_like {
            match read_work_package_report_status(&artifact.path) {
                Ok(report) => (
                    report.status == "ok",
                    report.status,
                    report.summary.join("; "),
                ),
                Err(err) => (false, "needs_review".to_string(), err),
            }
        } else {
            (true, "present".to_string(), String::new())
        };
        if !ok {
            blocking_count += 1;
        }
        checks.push(work_package_merge_check_json(
            artifact.label,
            ok,
            &status,
            &artifact.path,
            &details,
            &work_package_artifact_command(package, artifact.label, &root),
        ));
    }

    let handoff_ok = summary.handoff_path.exists();
    if !handoff_ok {
        blocking_count += 1;
    }
    checks.push(work_package_merge_check_json(
        "handoff",
        handoff_ok,
        if handoff_ok { "present" } else { "missing" },
        &summary.handoff_path,
        if handoff_ok {
            "handoff artifact exists"
        } else {
            "handoff artifact is required before merge"
        },
        &format!(
            "hoi4skill work-package-handoff --mod-root {root} --package {} --output .hoi4skill/handoff_{}.md",
            package.id, package.id
        ),
    ));

    let claim_blocks = claim.claim_status == "needs_review"
        || claim.claim_status == "blocked_claim"
        || (claim.claim_status == "claimed" && claim.current_state == "already_handed_off");
    if claim_blocks {
        blocking_count += 1;
    }
    let claim_status =
        if claim.claim_status == "claimed" && claim.current_state == "already_handed_off" {
            "stale_claim_after_handoff"
        } else {
            &claim.claim_status
        };
    checks.push(work_package_merge_check_json(
        "claim",
        !claim_blocks,
        claim_status,
        &claim.claim_path,
        if claim_blocks {
            "claim must be released or refreshed before merge"
        } else {
            "claim state does not block merge"
        },
        &format!(
            "hoi4skill work-package-release-claim --mod-root {root} --package {} --released-by <assignee> --reason <reason>",
            package.id
        ),
    ));

    let mergeable = blocking_count == 0;
    let decision = if mergeable { "mergeable" } else { "blocked" };
    let next_commands = vec![
        format!(
            "hoi4skill work-package-merge-gate --mod-root {root} --package {} --output .hoi4skill/merge_gate_{}.json",
            package.id, package.id
        ),
        format!(
            "hoi4skill work-package-review-checklist --mod-root {root} --package {} --output .hoi4skill/review_checklist_{}.md",
            package.id, package.id
        ),
        format!("hoi4skill large-mod-risk-register --mod-root {root} --output .hoi4skill/risk_register.json"),
        format!("hoi4skill large-mod-release-gate --mod-root {root} --output .hoi4skill/release_gate.json"),
    ];
    Ok(format!(
        "{{\n  \"schema\": \"hoi4skill.work_package_merge_gate.v1\",\n  \"mergeable\": {},\n  \"decision\": {},\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"claims_dir\": {},\n  \"package\": {{\n    \"id\": {},\n    \"kind\": {},\n    \"title\": {}\n  }},\n  \"blocking_count\": {},\n  \"checks\": [\n{}\n  ],\n  \"next_commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_bool(mergeable),
        json_str(decision),
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        json_str(&claims_dir.display().to_string()),
        json_str(&package.id),
        json_str(&package.kind),
        json_str(&package.title),
        blocking_count,
        checks.join(",\n"),
        json_array(&next_commands),
        json_array(&work_package_merge_gate_stop_conditions()),
    ))
}

fn work_package_merge_check_json(
    name: &str,
    ok: bool,
    status: &str,
    evidence: &Path,
    summary: &str,
    command: &str,
) -> String {
    format!(
        "    {{\"name\": {}, \"ok\": {}, \"status\": {}, \"evidence\": {}, \"summary\": {}, \"command\": {}}}",
        json_str(name),
        json_bool(ok),
        json_str(status),
        json_str(&evidence.display().to_string()),
        json_str(summary),
        json_str(command),
    )
}

fn work_package_merge_gate_stop_conditions() -> Vec<String> {
    vec![
        "Do not merge while mergeable is false.".to_string(),
        "Do not merge while any artifact check is missing or needs_review.".to_string(),
        "Do not merge while an active claim remains after handoff.".to_string(),
        "Do not use package merge approval as a substitute for the final large-mod release gate."
            .to_string(),
    ]
}

fn write_work_package_merge_gates(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    claims_dir: &Path,
    output_dir: &Path,
) -> Result<String, String> {
    fs::create_dir_all(output_dir).map_err(|e| format!("create {}: {e}", output_dir.display()))?;
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let mut mergeable_count = 0usize;
    let mut blocked_count = 0usize;
    let mut entries = Vec::new();
    for package in packages {
        let path = output_dir.join(format!("merge_gate_{}.json", package.id));
        let json = work_package_merge_gate_json(
            blueprint,
            package,
            packages,
            blueprint_path,
            mod_root,
            claims_dir,
        )?;
        fs::write(&path, json).map_err(|e| format!("write {}: {e}", path.display()))?;
        let summary = work_package_merge_summary(package, packages, mod_root, claims_dir);
        if summary.mergeable {
            mergeable_count += 1;
        } else {
            blocked_count += 1;
        }
        entries.push(format!(
            "{{\n      \"id\": {},\n      \"kind\": {},\n      \"title\": {},\n      \"mergeable\": {},\n      \"blocking_count\": {},\n      \"blockers\": {},\n      \"path\": {}\n    }}",
            json_str(&summary.id),
            json_str(&summary.kind),
            json_str(&summary.title),
            json_bool(summary.mergeable),
            summary.blocking_count,
            json_array(&summary.blockers),
            json_str(&path.display().to_string()),
        ));
    }
    let manifest = format!(
        "{{\n  \"schema\": \"hoi4skill.work_package_merge_gates.v1\",\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"claims_dir\": {},\n  \"output_dir\": {},\n  \"package_count\": {},\n  \"mergeable_count\": {},\n  \"blocked_count\": {},\n  \"gates\": [\n{}\n  ],\n  \"next_commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        json_str(&claims_dir.display().to_string()),
        json_str(&output_dir.display().to_string()),
        packages.len(),
        mergeable_count,
        blocked_count,
        entries.join(",\n"),
        json_array(&[
            format!("hoi4skill work-package-merge-gates --mod-root {root} --output-dir .hoi4skill/merge_gates --output .hoi4skill/merge_gates/manifest.json"),
            format!("hoi4skill large-mod-merge-gate --mod-root {root} --output .hoi4skill/merge_gate.json"),
            format!("hoi4skill large-mod-review-queue --mod-root {root} --output .hoi4skill/review_queue.json"),
        ]),
        json_array(&work_package_merge_gates_stop_conditions()),
    );
    let manifest_path = output_dir.join("manifest.json");
    fs::write(&manifest_path, &manifest)
        .map_err(|e| format!("write {}: {e}", manifest_path.display()))?;
    Ok(manifest)
}

fn work_package_merge_gates_stop_conditions() -> Vec<String> {
    vec![
        "Do not merge packages whose generated gate has mergeable=false.".to_string(),
        "Regenerate merge gates after changing package artifacts, handoffs, or claims.".to_string(),
        "Use the large-mod merge gate to confirm the whole integration branch before release."
            .to_string(),
    ]
}

#[derive(Clone, Debug)]
struct WorkPackageMergeSummary {
    id: String,
    kind: String,
    title: String,
    mergeable: bool,
    blocking_count: usize,
    blockers: Vec<String>,
    handoff_path: PathBuf,
    claim_status: String,
    claim_path: PathBuf,
}

fn work_package_merge_summary(
    package: &WorkPackage,
    packages: &[WorkPackage],
    mod_root: Option<&Path>,
    claims_dir: &Path,
) -> WorkPackageMergeSummary {
    let mut blockers = Vec::new();
    for artifact in work_package_readiness_artifacts(package, mod_root) {
        if !artifact.path.exists() {
            blockers.push(format!("missing_{}", artifact.label));
            continue;
        }
        if artifact.report_like {
            match read_work_package_report_status(&artifact.path) {
                Ok(report) => {
                    if report.status == "needs_review" {
                        blockers.push(format!("{}_needs_review", artifact.label));
                    }
                }
                Err(_) => blockers.push(format!("{}_needs_review", artifact.label)),
            }
        }
    }
    let summary = work_package_readiness_summary(package, mod_root);
    if !summary.handoff_path.exists() {
        blockers.push("missing_handoff".to_string());
    }
    let claim = work_package_claim_summary(package, packages, mod_root, claims_dir);
    let claim_status =
        if claim.claim_status == "claimed" && claim.current_state == "already_handed_off" {
            "stale_claim_after_handoff".to_string()
        } else {
            claim.claim_status.clone()
        };
    if claim_status == "needs_review"
        || claim_status == "blocked_claim"
        || claim_status == "stale_claim_after_handoff"
    {
        blockers.push(format!("claim_{claim_status}"));
    }
    blockers.sort();
    blockers.dedup();
    WorkPackageMergeSummary {
        id: package.id.clone(),
        kind: package.kind.clone(),
        title: package.title.clone(),
        mergeable: blockers.is_empty(),
        blocking_count: blockers.len(),
        blockers,
        handoff_path: summary.handoff_path,
        claim_status,
        claim_path: claim.claim_path,
    }
}

fn large_mod_merge_gate_json(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    claims_dir: &Path,
) -> String {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let summaries = packages
        .iter()
        .map(|package| work_package_merge_summary(package, packages, mod_root, claims_dir))
        .collect::<Vec<_>>();
    let mergeable_count = summaries.iter().filter(|summary| summary.mergeable).count();
    let blocked_count = summaries.len().saturating_sub(mergeable_count);
    let blocking_count = summaries
        .iter()
        .map(|summary| summary.blocking_count)
        .sum::<usize>();
    let package_json = summaries
        .iter()
        .map(|summary| {
            format!(
                "{{\n      \"id\": {},\n      \"kind\": {},\n      \"title\": {},\n      \"mergeable\": {},\n      \"blocking_count\": {},\n      \"blockers\": {},\n      \"handoff\": {},\n      \"claim_status\": {},\n      \"claim_path\": {},\n      \"next_commands\": {}\n    }}",
                json_str(&summary.id),
                json_str(&summary.kind),
                json_str(&summary.title),
                json_bool(summary.mergeable),
                summary.blocking_count,
                json_array(&summary.blockers),
                json_str(&summary.handoff_path.display().to_string()),
                json_str(&summary.claim_status),
                json_str(&summary.claim_path.display().to_string()),
                json_array(&large_mod_merge_gate_package_commands(summary, &root)),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let mergeable = blocked_count == 0;
    let next_commands = vec![
        format!("hoi4skill work-package-merge-gates --mod-root {root} --output-dir .hoi4skill/merge_gates --output .hoi4skill/merge_gates/manifest.json"),
        format!("hoi4skill large-mod-merge-gate --mod-root {root} --output .hoi4skill/merge_gate.json"),
        format!("hoi4skill large-mod-review-queue --mod-root {root} --output .hoi4skill/review_queue.json"),
        format!("hoi4skill large-mod-risk-register --mod-root {root} --output .hoi4skill/risk_register.json"),
        format!("hoi4skill large-mod-release-gate --mod-root {root} --output .hoi4skill/release_gate.json"),
    ];
    format!(
        "{{\n  \"schema\": \"hoi4skill.large_mod_merge_gate.v1\",\n  \"mergeable\": {},\n  \"decision\": {},\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"claims_dir\": {},\n  \"package_count\": {},\n  \"mergeable_count\": {},\n  \"blocked_count\": {},\n  \"blocking_count\": {},\n  \"packages\": [\n{}\n  ],\n  \"next_commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_bool(mergeable),
        json_str(if mergeable { "mergeable" } else { "blocked" }),
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        json_str(&claims_dir.display().to_string()),
        packages.len(),
        mergeable_count,
        blocked_count,
        blocking_count,
        package_json,
        json_array(&next_commands),
        json_array(&large_mod_merge_gate_stop_conditions()),
    )
}

fn large_mod_merge_gate_package_commands(
    summary: &WorkPackageMergeSummary,
    mod_root: &str,
) -> Vec<String> {
    let mut commands = vec![format!(
        "hoi4skill work-package-merge-gate --mod-root {mod_root} --package {} --output .hoi4skill/merge_gate_{}.json",
        summary.id, summary.id
    )];
    if summary
        .blockers
        .iter()
        .any(|blocker| blocker == "missing_handoff")
    {
        commands.push(format!(
            "hoi4skill work-package-handoff --mod-root {mod_root} --package {} --output .hoi4skill/handoff_{}.md",
            summary.id, summary.id
        ));
    }
    if summary
        .blockers
        .iter()
        .any(|blocker| blocker.starts_with("claim_"))
    {
        commands.push(format!(
            "hoi4skill work-package-release-claim --mod-root {mod_root} --package {} --released-by <assignee> --reason <reason>",
            summary.id
        ));
    }
    commands
}

fn large_mod_merge_gate_stop_conditions() -> Vec<String> {
    vec![
        "Do not merge the large-mod integration branch while mergeable is false.".to_string(),
        "Do not skip blocked packages; resolve every package blocker or remove the package from scope explicitly.".to_string(),
        "Do not use the large-mod merge gate as a substitute for the final release gate.".to_string(),
    ]
}

fn large_mod_review_queue_json(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    claims_dir: &Path,
) -> String {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let mut queue = packages
        .iter()
        .map(|package| work_package_merge_summary(package, packages, mod_root, claims_dir))
        .collect::<Vec<_>>();
    queue.sort_by(|left, right| {
        review_state_rank(work_package_review_state(left))
            .cmp(&review_state_rank(work_package_review_state(right)))
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.id.cmp(&right.id))
    });

    let merge_ready_count = queue
        .iter()
        .filter(|summary| work_package_review_state(summary) == "merge_ready")
        .count();
    let handoff_ready_count = queue
        .iter()
        .filter(|summary| work_package_review_state(summary) == "handoff_ready")
        .count();
    let blocked_count = queue
        .len()
        .saturating_sub(merge_ready_count + handoff_ready_count);
    let package_json = queue
        .iter()
        .enumerate()
        .map(|(idx, summary)| {
            let state = work_package_review_state(summary);
            format!(
                "{{\n      \"queue_index\": {},\n      \"id\": {},\n      \"kind\": {},\n      \"title\": {},\n      \"review_state\": {},\n      \"mergeable\": {},\n      \"blocking_count\": {},\n      \"blockers\": {},\n      \"handoff\": {},\n      \"claim_status\": {},\n      \"next_commands\": {}\n    }}",
                idx + 1,
                json_str(&summary.id),
                json_str(&summary.kind),
                json_str(&summary.title),
                json_str(state),
                json_bool(summary.mergeable),
                summary.blocking_count,
                json_array(&summary.blockers),
                json_str(&summary.handoff_path.display().to_string()),
                json_str(&summary.claim_status),
                json_array(&large_mod_review_queue_package_commands(summary, &root)),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let next_commands = vec![
        format!("hoi4skill large-mod-review-queue --mod-root {root} --output .hoi4skill/review_queue.json"),
        format!("hoi4skill large-mod-merge-gate --mod-root {root} --output .hoi4skill/merge_gate.json"),
        format!("hoi4skill large-mod-risk-register --mod-root {root} --output .hoi4skill/risk_register.json"),
        format!("hoi4skill large-mod-release-gate --mod-root {root} --output .hoi4skill/release_gate.json"),
    ];
    format!(
        "{{\n  \"schema\": \"hoi4skill.large_mod_review_queue.v1\",\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"claims_dir\": {},\n  \"package_count\": {},\n  \"merge_ready_count\": {},\n  \"handoff_ready_count\": {},\n  \"blocked_count\": {},\n  \"queue\": [\n{}\n  ],\n  \"next_commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        json_str(&claims_dir.display().to_string()),
        packages.len(),
        merge_ready_count,
        handoff_ready_count,
        blocked_count,
        package_json,
        json_array(&next_commands),
        json_array(&large_mod_review_queue_stop_conditions()),
    )
}

fn work_package_review_state(summary: &WorkPackageMergeSummary) -> &'static str {
    if summary.mergeable {
        "merge_ready"
    } else if summary.blockers.len() == 1 && summary.blockers[0] == "missing_handoff" {
        "handoff_ready"
    } else if summary
        .blockers
        .iter()
        .any(|blocker| blocker.starts_with("claim_"))
    {
        "claim_blocked"
    } else if summary
        .blockers
        .iter()
        .any(|blocker| blocker.ends_with("_needs_review"))
    {
        "artifact_needs_review"
    } else if summary
        .blockers
        .iter()
        .any(|blocker| blocker.starts_with("missing_"))
    {
        "missing_artifacts"
    } else {
        "waiting"
    }
}

fn review_state_rank(state: &str) -> usize {
    match state {
        "merge_ready" => 0,
        "handoff_ready" => 1,
        "claim_blocked" => 2,
        "artifact_needs_review" => 3,
        "missing_artifacts" => 4,
        _ => 5,
    }
}

fn large_mod_review_queue_package_commands(
    summary: &WorkPackageMergeSummary,
    mod_root: &str,
) -> Vec<String> {
    let state = work_package_review_state(summary);
    let mut commands = vec![format!(
        "hoi4skill work-package-review-checklist --mod-root {mod_root} --package {} --output .hoi4skill/review_checklist_{}.md",
        summary.id, summary.id
    )];
    if state == "handoff_ready"
        || summary
            .blockers
            .iter()
            .any(|blocker| blocker == "missing_handoff")
    {
        commands.push(format!(
            "hoi4skill work-package-handoff --mod-root {mod_root} --package {} --output .hoi4skill/handoff_{}.md",
            summary.id, summary.id
        ));
    }
    if state == "merge_ready" {
        commands.push(format!(
            "hoi4skill work-package-merge-gate --mod-root {mod_root} --package {} --output .hoi4skill/merge_gate_{}.json",
            summary.id, summary.id
        ));
    }
    if state == "claim_blocked" {
        commands.push(format!(
            "hoi4skill work-package-release-claim --mod-root {mod_root} --package {} --released-by <assignee> --reason <reason>",
            summary.id
        ));
    }
    commands
}

fn large_mod_review_queue_stop_conditions() -> Vec<String> {
    vec![
        "Do not spend reviewer time on missing_artifacts packages before merge_ready or handoff_ready packages."
            .to_string(),
        "Do not merge a merge_ready package without running its package merge gate.".to_string(),
        "Resolve claim_blocked packages before assigning them to reviewers.".to_string(),
    ]
}

fn work_package_handoff_markdown(
    blueprint: &LargeModBlueprint,
    package: &WorkPackage,
    blueprint_path: &Path,
    mod_root: Option<&Path>,
) -> Result<String, String> {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let tag = package_tag(package);
    let namespace = package_namespace(package, blueprint);
    let package_token = package_token(package);
    let artifacts = work_package_readiness_artifacts(package, mod_root);
    let changed_files = artifacts
        .iter()
        .find(|artifact| artifact.label == "changed")
        .and_then(|artifact| {
            if artifact.path.exists() {
                read_utf8_lossy(&artifact.path).ok()
            } else {
                None
            }
        })
        .map(|text| {
            text.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let mut out = String::new();
    out.push_str(&format!("# Work Package Handoff: {}\n\n", package.title));
    out.push_str("- schema: `hoi4skill.work_package_handoff.v1`\n");
    out.push_str(&format!("- mod: `{}`\n", blueprint.name));
    out.push_str(&format!("- acronym: `{}`\n", blueprint.acronym));
    out.push_str(&format!("- mod_root: `{}`\n", root));
    out.push_str(&format!("- blueprint: `{}`\n", blueprint_path.display()));
    out.push_str(&format!("- package_id: `{}`\n", package.id));
    out.push_str(&format!("- package_kind: `{}`\n", package.kind));
    out.push_str(&format!("- package_token: `{}`\n", package_token));
    out.push_str(&format!("- namespace: `{}`\n", namespace));
    if let Some(tag) = tag.as_deref() {
        out.push_str(&format!("- tag_hint: `{tag}`\n"));
    }

    out.push_str("\n## Identity Terms\n\n");
    for term in work_package_identity_terms(package, blueprint) {
        out.push_str(&format!("- `{term}`\n"));
    }

    out.push_str("\n## Allowed Edit Surface\n\n");
    for path in work_package_boundary_allowed_prefixes(package) {
        out.push_str(&format!("- `{path}`\n"));
    }

    out.push_str("\n## Deliverables\n\n");
    for deliverable in &package.deliverables {
        out.push_str(&format!("- {deliverable}\n"));
    }

    out.push_str("\n## Changed Files\n\n");
    if changed_files.is_empty() {
        out.push_str("- No changed-file list has been written for this package yet.\n");
    } else {
        for path in &changed_files {
            out.push_str(&format!("- `{path}`\n"));
        }
    }

    out.push_str("\n## Package Artifacts\n\n");
    out.push_str("| Artifact | Status | Path | Summary |\n");
    out.push_str("| --- | --- | --- | --- |\n");
    for artifact in artifacts {
        let (status, summary) = if !artifact.path.exists() {
            ("missing".to_string(), String::new())
        } else if artifact.report_like {
            match read_work_package_report_status(&artifact.path) {
                Ok(report) => (report.status, report.summary.join("; ")),
                Err(err) => ("needs_review".to_string(), err),
            }
        } else {
            ("present".to_string(), String::new())
        };
        out.push_str(&format!(
            "| `{}` | `{}` | `{}` | {} |\n",
            artifact.label,
            status,
            artifact.path.display(),
            markdown_table_cell(&summary)
        ));
    }

    out.push_str("\n## Next Commands\n\n");
    for command in work_package_handoff_next_commands(package, &root) {
        out.push_str(&format!("- `{command}`\n"));
    }

    out.push_str("\n## Stop Conditions\n\n");
    out.push_str("- Do not continue package generation while any package artifact is missing or needs review.\n");
    out.push_str("- Do not edit files outside the allowed edit surface unless the blueprint and literal user request expand this package.\n");
    out.push_str("- Do not create country tags, history, map data, GUI, technologies, or characters unless explicitly authorized.\n");
    out.push_str("- Before final output, run strict validation and text-alignment checks when player-visible user text is involved.\n");
    Ok(out)
}

fn work_package_handoff_next_commands(package: &WorkPackage, mod_root: &str) -> Vec<String> {
    vec![
        format!(
            "hoi4skill generate-work-package --mod-root {mod_root} --package {} --dry-run --output .hoi4skill/plan_{}.json",
            package.id, package.id
        ),
        format!(
            "hoi4skill asset-pack-plan --mod-root {mod_root} --package {} --output .hoi4skill/assets_{}.md",
            package.id, package.id
        ),
        format!(
            "hoi4skill check-work-package-boundary --mod-root {mod_root} --package {} --changed-file .hoi4skill/changed_{}.txt --strict-names --output .hoi4skill/boundary_{}.json",
            package.id, package.id, package.id
        ),
        format!(
            "hoi4skill work-package-status --mod-root {mod_root} --package {} --output .hoi4skill/status_{}.json",
            package.id, package.id
        ),
        format!(
            "hoi4skill work-package-readiness --mod-root {mod_root} --package {} --output .hoi4skill/readiness_{}.json",
            package.id, package.id
        ),
    ]
}

fn markdown_table_cell(value: &str) -> String {
    if value.is_empty() {
        String::new()
    } else {
        value.replace('|', "\\|")
    }
}

fn changed_list_text(paths: &[String]) -> String {
    if paths.is_empty() {
        String::new()
    } else {
        format!("{}\n", paths.join("\n"))
    }
}

fn work_package_match_for_path(
    normalized_path: &str,
    package: &WorkPackage,
    blueprint: &LargeModBlueprint,
    strict_names: bool,
) -> Option<String> {
    let allowed_by = work_package_boundary_allowed_prefixes(package)
        .into_iter()
        .find(|prefix| boundary_path_matches_prefix(normalized_path, prefix))?;
    if strict_names && !boundary_path_matches_package_identity(normalized_path, package, blueprint)
    {
        return None;
    }
    Some(allowed_by)
}

fn work_package_boundary_json(
    blueprint: &LargeModBlueprint,
    package: &WorkPackage,
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    changed: &[String],
    strict_names: bool,
) -> String {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let allowed_prefixes = work_package_boundary_allowed_prefixes(package);
    let mut allowed_changes = Vec::new();
    let mut violations = Vec::new();
    for raw in changed {
        let normalized = normalize_boundary_path(raw, mod_root);
        let allowed_by = allowed_prefixes
            .iter()
            .find(|prefix| boundary_path_matches_prefix(&normalized, prefix))
            .cloned();
        if let Some(prefix) = allowed_by {
            if strict_names
                && !boundary_path_matches_package_identity(&normalized, package, blueprint)
            {
                violations.push(boundary_change_json(
                    raw,
                    &normalized,
                    Some(&prefix),
                    "strict_name_mismatch",
                ));
            } else {
                allowed_changes.push(boundary_change_json(
                    raw,
                    &normalized,
                    Some(&prefix),
                    "allowed",
                ));
            }
        } else {
            violations.push(boundary_change_json(
                raw,
                &normalized,
                None,
                "prefix_not_allowed",
            ));
        }
    }
    let ok = violations.is_empty();
    let next_commands = vec![
        format!(
            "hoi4skill impact {root} --changed <file> --output .hoi4skill/impact_{}.json",
            package.id
        ),
        format!(
            "hoi4skill validate {root} --changed-only --changed <file> --strict-code-index --output .hoi4skill/validation_{}.json",
            package.id
        ),
        format!(
            "hoi4skill work-package-status --mod-root {root} --package {} --output .hoi4skill/status_{}.json",
            package.id, package.id
        ),
    ];
    format!(
        "{{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": {},\n  \"strict_names\": {},\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"package\": {{\n    \"id\": {},\n    \"kind\": {},\n    \"title\": {}\n  }},\n  \"changed_count\": {},\n  \"allowed_count\": {},\n  \"violation_count\": {},\n  \"allowed_prefixes\": {},\n  \"identity_terms\": {},\n  \"allowed_changes\": [\n{}\n  ],\n  \"violations\": [\n{}\n  ],\n  \"next_commands\": {},\n  \"stop_condition\": {}\n}}\n",
        json_bool(ok),
        json_bool(strict_names),
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        json_str(&package.id),
        json_str(&package.kind),
        json_str(&package.title),
        changed.len(),
        allowed_changes.len(),
        violations.len(),
        json_array(&allowed_prefixes),
        json_array(&work_package_identity_terms(package, blueprint)),
        allowed_changes.join(",\n"),
        violations.join(",\n"),
        json_array(&next_commands),
        json_str("Do not continue package generation until every changed file is inside the package boundary or the blueprint/user request explicitly expands the edit surface."),
    )
}

fn boundary_change_json(
    raw: &str,
    normalized: &str,
    allowed_by: Option<&str>,
    reason: &str,
) -> String {
    format!(
        "    {{\"path\": {}, \"normalized\": {}, \"allowed_by\": {}, \"reason\": {}}}",
        json_str(raw),
        json_str(normalized),
        json_optional_str(allowed_by),
        json_str(reason)
    )
}

fn work_package_boundary_allowed_prefixes(package: &WorkPackage) -> Vec<String> {
    let mut prefixes = package
        .allowed_paths
        .iter()
        .map(|path| normalize_boundary_slashes(path))
        .collect::<Vec<_>>();
    prefixes.extend([
        format!(".hoi4skill/work_packages/{}.md", package.id),
        format!(".hoi4skill/plan_{}.json", package.id),
        format!(".hoi4skill/assets_{}.md", package.id),
        format!(".hoi4skill/context_{}.md", package.id),
        format!(".hoi4skill/validation_{}.json", package.id),
    ]);
    prefixes.sort();
    prefixes.dedup();
    prefixes
}

fn normalize_boundary_path(raw: &str, mod_root: Option<&Path>) -> String {
    let mut path = normalize_boundary_slashes(raw.trim());
    if let Some(root) = mod_root {
        let root = normalize_boundary_slashes(&root.display().to_string());
        let root = root.trim_end_matches('/');
        if path == root {
            path.clear();
        } else if let Some(rest) = path.strip_prefix(&format!("{root}/")) {
            path = rest.to_string();
        }
    }
    while let Some(rest) = path.strip_prefix("./") {
        path = rest.to_string();
    }
    path.trim_start_matches('/').to_string()
}

fn normalize_boundary_slashes(raw: &str) -> String {
    raw.replace('\\', "/")
}

fn boundary_path_matches_prefix(path: &str, prefix: &str) -> bool {
    path == prefix || path.starts_with(&format!("{}/", prefix.trim_end_matches('/')))
}

fn boundary_path_matches_package_identity(
    path: &str,
    package: &WorkPackage,
    blueprint: &LargeModBlueprint,
) -> bool {
    if path.starts_with(".hoi4skill/") {
        return path.contains(&package.id);
    }
    work_package_identity_terms(package, blueprint)
        .iter()
        .any(|term| boundary_path_contains_term(path, term))
}

fn work_package_identity_terms(
    package: &WorkPackage,
    blueprint: &LargeModBlueprint,
) -> Vec<String> {
    let mut terms = vec![
        package.id.clone(),
        package_token(package),
        package_namespace(package, blueprint),
    ];
    if let Some(tag) = package_tag(package) {
        if !tag.contains('<') {
            terms.push(tag);
        }
    }
    terms.retain(|term| !term.trim().is_empty());
    terms.sort();
    terms.dedup();
    terms
}

fn boundary_path_contains_term(path: &str, term: &str) -> bool {
    let path = path.to_ascii_lowercase();
    let term = term.to_ascii_lowercase();
    let normalized_path = normalize_boundary_identity_text(&path);
    let normalized_term = normalize_boundary_identity_text(&term);
    normalized_path
        .split('_')
        .any(|part| part == normalized_term)
        || normalized_path.contains(&format!("_{normalized_term}_"))
        || normalized_path.starts_with(&format!("{normalized_term}_"))
        || normalized_path.ends_with(&format!("_{normalized_term}"))
}

fn normalize_boundary_identity_text(text: &str) -> String {
    text.chars()
        .map(|ch| {
            if ch == '\\' || ch == '/' || ch == '.' || ch == '-' {
                '_'
            } else {
                ch
            }
        })
        .collect()
}

fn large_mod_ci_plan_json(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    game_root: Option<&str>,
    strict_names: bool,
) -> String {
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let game_root = game_root.unwrap_or("<game-root>");
    let strict_flag = if strict_names { " --strict-names" } else { "" };
    let global_commands = vec![
        format!("hoi4skill build-mod-index {root} --output .hoi4skill/mod_index.json"),
        format!(
            "hoi4skill large-mod-ownership-map --mod-root {root} --output .hoi4skill/ownership_map.json"
        ),
        format!(
            "hoi4skill large-mod-dependency-graph --mod-root {root} --output .hoi4skill/dependency_graph.json"
        ),
        format!(
            "hoi4skill large-mod-milestone-plan --mod-root {root} --output .hoi4skill/milestone_plan.json"
        ),
        format!(
            "hoi4skill large-mod-execution-queue --mod-root {root} --output .hoi4skill/execution_queue.json"
        ),
        format!("hoi4skill loc-audit {root} --output .hoi4skill/loc_audit.json"),
        format!("hoi4skill gfx-audit {root} --output .hoi4skill/gfx_audit.json"),
        format!("hoi4skill logic-audit {root} --output .hoi4skill/logic_audit.json"),
        format!(
            "hoi4skill analyze-error-log --input <error.log> --mod-root {root} --output .hoi4skill/error_log_report.json"
        ),
    ];
    let package_json = packages
        .iter()
        .map(|package| {
            let commands = vec![
                format!(
                    "hoi4skill work-package-authoring-pack --mod-root {root} --package {} --output-dir .hoi4skill/authoring/{} --output .hoi4skill/authoring/{}/manifest.json",
                    package.id, package.id, package.id
                ),
                format!(
                    "hoi4skill work-package-start-brief --mod-root {root} --package {} --output .hoi4skill/start_{}.md",
                    package.id, package.id
                ),
                format!(
                    "hoi4skill check-work-package-boundary --mod-root {root} --package {} --changed-file .hoi4skill/changed_{}.txt{strict_flag} --output .hoi4skill/boundary_{}.json",
                    package.id, package.id, package.id
                ),
                format!(
                    "hoi4skill generate-work-package --mod-root {root} --package {} --dry-run --output .hoi4skill/plan_{}.json",
                    package.id, package.id
                ),
                format!(
                    "hoi4skill asset-pack-plan --mod-root {root} --package {} --output .hoi4skill/assets_{}.md",
                    package.id, package.id
                ),
                format!(
                    "hoi4skill work-package-status --mod-root {root} --package {} --output .hoi4skill/status_{}.json",
                    package.id, package.id
                ),
                format!(
                    "hoi4skill work-package-handoff --mod-root {root} --package {} --output .hoi4skill/handoff_{}.md",
                    package.id, package.id
                ),
                format!(
                    "hoi4skill work-package-review-checklist --mod-root {root} --package {} --output .hoi4skill/review_checklist_{}.md",
                    package.id, package.id
                ),
                format!(
                    "hoi4skill work-package-merge-gate --mod-root {root} --package {} --output .hoi4skill/merge_gate_{}.json",
                    package.id, package.id
                ),
            ];
            format!(
                "{{\n      \"id\": {},\n      \"kind\": {},\n      \"title\": {},\n      \"identity_terms\": {},\n      \"commands\": {}\n    }}",
                json_str(&package.id),
                json_str(&package.kind),
                json_str(&package.title),
                json_array(&work_package_identity_terms(package, blueprint)),
                json_array(&commands),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let final_commands = vec![
        format!(
            "hoi4skill validate {root} --game-root {game_root} --strict-code-index --output .hoi4skill/validation.json"
        ),
        format!("hoi4skill work-package-status --mod-root {root} --output .hoi4skill/work_package_status.json"),
        format!("hoi4skill work-package-readiness --mod-root {root} --output .hoi4skill/readiness.json"),
        format!("hoi4skill large-mod-next-actions --mod-root {root} --output .hoi4skill/next_actions.json"),
        format!("hoi4skill large-mod-risk-register --mod-root {root} --output .hoi4skill/risk_register.json"),
        format!("hoi4skill large-mod-dispatch-gate --mod-root {root} --output .hoi4skill/dispatch_gate.json"),
        format!("hoi4skill work-package-merge-gates --mod-root {root} --output-dir .hoi4skill/merge_gates --output .hoi4skill/merge_gates/manifest.json"),
        format!("hoi4skill large-mod-merge-gate --mod-root {root} --output .hoi4skill/merge_gate.json"),
        format!("hoi4skill large-mod-dashboard --mod-root {root} --output .hoi4skill/dashboard.md"),
        format!("hoi4skill large-mod-production-snapshot --mod-root {root} --output .hoi4skill/production_snapshot.json"),
        format!("hoi4skill large-mod-production-brief --mod-root {root} --output .hoi4skill/production_brief.md"),
        format!("hoi4skill large-mod-evidence-pack --mod-root {root} --output .hoi4skill/evidence_pack.json"),
        format!("hoi4skill large-mod-review-brief --mod-root {root} --output .hoi4skill/review_brief.md"),
        format!("hoi4skill large-mod-release-bundle --mod-root {root} --output .hoi4skill/release_bundle.json"),
        format!("hoi4skill large-mod-release-brief --mod-root {root} --output .hoi4skill/release_brief.md"),
        format!("hoi4skill large-mod-release-gate --mod-root {root} --output .hoi4skill/release_gate.json"),
    ];
    format!(
        "{{\n  \"schema\": \"hoi4skill.large_mod_ci_plan.v1\",\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"game_root\": {},\n  \"blueprint\": {},\n  \"strict_names\": {},\n  \"package_count\": {},\n  \"global_commands\": {},\n  \"package_gates\": [\n{}\n  ],\n  \"final_commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(game_root),
        json_str(&blueprint_path.display().to_string()),
        json_bool(strict_names),
        packages.len(),
        json_array(&global_commands),
        package_json,
        json_array(&final_commands),
        json_array(&large_mod_ci_stop_conditions()),
    )
}

fn large_mod_ci_stop_conditions() -> Vec<String> {
    vec![
        "Do not merge a package with boundary violations.".to_string(),
        "Do not treat a status report with needs_review as releasable.".to_string(),
        "Do not skip strict-code-index validation for final generated content.".to_string(),
        "Do not create country tags, history, map data, GUI, technologies, or characters unless the literal user request authorizes them.".to_string(),
    ]
}

fn large_mod_release_gate_json(
    blueprint: &LargeModBlueprint,
    packages: &[WorkPackage],
    blueprint_path: &Path,
    mod_root: Option<&Path>,
    reports: &[PathBuf],
) -> Result<String, String> {
    let report_statuses = reports
        .iter()
        .map(|path| read_work_package_report_status(path))
        .collect::<Result<Vec<_>, _>>()?;
    let required_reports = large_mod_required_release_reports();
    let missing_required_reports = large_mod_missing_release_reports(mod_root, &required_reports);
    let blocking_reports = report_statuses
        .iter()
        .filter(|report| report.status == "needs_review")
        .collect::<Vec<_>>();
    let releasable = missing_required_reports.is_empty() && blocking_reports.is_empty();
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let report_json = report_statuses
        .iter()
        .map(|report| {
            format!(
                "{{\n      \"path\": {},\n      \"schema\": {},\n      \"status\": {},\n      \"summary\": {}\n    }}",
                json_str(&report.path.display().to_string()),
                json_optional_str(report.schema.as_deref()),
                json_str(&report.status),
                json_array(&report.summary),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let blocking_json = blocking_reports
        .iter()
        .map(|report| report.path.display().to_string())
        .collect::<Vec<_>>();
    let next_commands = vec![
        format!("hoi4skill large-mod-ci-plan --mod-root {root} --output .hoi4skill/ci_plan.json"),
        format!("hoi4skill large-mod-ownership-map --mod-root {root} --output .hoi4skill/ownership_map.json"),
        format!("hoi4skill large-mod-dependency-graph --mod-root {root} --output .hoi4skill/dependency_graph.json"),
        format!("hoi4skill large-mod-milestone-plan --mod-root {root} --output .hoi4skill/milestone_plan.json"),
        format!("hoi4skill large-mod-execution-queue --mod-root {root} --output .hoi4skill/execution_queue.json"),
        format!("hoi4skill work-package-status --mod-root {root} --output .hoi4skill/work_package_status.json"),
        format!("hoi4skill work-package-readiness --mod-root {root} --output .hoi4skill/readiness.json"),
        format!("hoi4skill validate {root} --strict-code-index --output .hoi4skill/validation.json"),
        format!("hoi4skill large-mod-dispatch-gate --mod-root {root} --output .hoi4skill/dispatch_gate.json"),
        format!("hoi4skill large-mod-merge-gate --mod-root {root} --output .hoi4skill/merge_gate.json"),
        format!("hoi4skill large-mod-regression-gate --mod-root {root} --output .hoi4skill/regression_gate.json"),
        format!("hoi4skill large-mod-regression-brief --mod-root {root} --output .hoi4skill/regression_brief.md"),
        format!("hoi4skill large-mod-release-gate --mod-root {root} --output .hoi4skill/release_gate.json"),
        format!("hoi4skill large-mod-next-actions --mod-root {root} --output .hoi4skill/next_actions.json"),
        format!("hoi4skill large-mod-risk-register --mod-root {root} --output .hoi4skill/risk_register.json"),
        format!("hoi4skill large-mod-dashboard --mod-root {root} --output .hoi4skill/dashboard.md"),
        format!("hoi4skill large-mod-production-snapshot --mod-root {root} --output .hoi4skill/production_snapshot.json"),
        format!("hoi4skill large-mod-production-brief --mod-root {root} --output .hoi4skill/production_brief.md"),
        format!("hoi4skill large-mod-evidence-pack --mod-root {root} --output .hoi4skill/evidence_pack.json"),
        format!("hoi4skill large-mod-review-brief --mod-root {root} --output .hoi4skill/review_brief.md"),
        format!("hoi4skill large-mod-release-bundle --mod-root {root} --output .hoi4skill/release_bundle.json"),
        format!("hoi4skill large-mod-release-brief --mod-root {root} --output .hoi4skill/release_brief.md"),
    ];
    Ok(format!(
        "{{\n  \"schema\": \"hoi4skill.large_mod_release_gate.v1\",\n  \"releasable\": {},\n  \"mod\": {},\n  \"acronym\": {},\n  \"mod_root\": {},\n  \"blueprint\": {},\n  \"package_count\": {},\n  \"report_count\": {},\n  \"blocking_count\": {},\n  \"missing_required_reports\": {},\n  \"blocking_reports\": {},\n  \"reports\": [\n{}\n  ],\n  \"next_commands\": {},\n  \"stop_conditions\": {}\n}}\n",
        json_bool(releasable),
        json_str(&blueprint.name),
        json_str(&blueprint.acronym),
        json_str(&root),
        json_str(&blueprint_path.display().to_string()),
        packages.len(),
        report_statuses.len(),
        blocking_reports.len(),
        json_array(&missing_required_reports),
        json_array(&blocking_json),
        report_json,
        json_array(&next_commands),
        json_array(&large_mod_release_stop_conditions()),
    ))
}

fn large_mod_required_release_reports() -> Vec<String> {
    vec![
        "mod_index.json".to_string(),
        "ownership_map.json".to_string(),
        "loc_audit.json".to_string(),
        "gfx_audit.json".to_string(),
        "logic_audit.json".to_string(),
        "validation.json".to_string(),
        "regression_gate.json".to_string(),
        "work_package_status.json".to_string(),
        "readiness.json".to_string(),
    ]
}

fn large_mod_missing_release_reports(
    mod_root: Option<&Path>,
    required_reports: &[String],
) -> Vec<String> {
    let Some(root) = mod_root else {
        return required_reports.to_vec();
    };
    required_reports
        .iter()
        .filter(|name| !root.join(".hoi4skill").join(name).exists())
        .cloned()
        .collect()
}

fn large_mod_release_stop_conditions() -> Vec<String> {
    vec![
        "Do not release while any required report is missing.".to_string(),
        "Do not release while any report has needs_review status.".to_string(),
        "Do not release while any work-package boundary report contains violations.".to_string(),
        "Do not release until regression_gate reports regression_passed=true.".to_string(),
        "Do not release without final strict-code-index validation against the local game/dependency codebase.".to_string(),
    ]
}

fn read_work_package_report_status(path: &Path) -> Result<WorkPackageReportStatus, String> {
    let text = read_utf8_lossy(path)?;
    let schema = status_json_string_field(&text, "schema");
    let mut summary = Vec::new();
    if let Some(schema) = &schema {
        summary.push(format!("schema={schema}"));
    }
    let status = if report_json_needs_review(&text, &mut summary) {
        "needs_review"
    } else {
        "ok"
    };
    if summary.is_empty() {
        summary.push("no known issue counters found".to_string());
    }
    Ok(WorkPackageReportStatus {
        path: path.to_path_buf(),
        schema,
        status: status.to_string(),
        summary,
    })
}

fn report_json_needs_review(text: &str, summary: &mut Vec<String>) -> bool {
    let mut needs_review = text.contains("\"ok\": false")
        || text.contains("\"status\": \"needs_review\"")
        || text.contains("\"status\":\"needs_review\"");
    if needs_review {
        if text.contains("\"ok\": false") {
            summary.push("ok=false".to_string());
        }
        if text.contains("\"status\": \"needs_review\"")
            || text.contains("\"status\":\"needs_review\"")
        {
            summary.push("status=needs_review".to_string());
        }
    }
    for key in [
        "effective_errors",
        "error_count",
        "errors",
        "finding_count",
        "diagnostics_effective",
        "issue_count",
        "warning_count",
        "missing_count",
        "missing_report_count",
        "missing_textures_count",
        "missing_sprites_count",
        "needs_review_count",
        "orphan_sprites_count",
        "unregistered_images_count",
        "missing_in_to_count",
        "duplicate_from_count",
        "duplicate_to_count",
        "violation_count",
        "blocked_count",
        "blocked_package_count",
        "missing_package_count",
        "blocking_count",
        "unassigned_count",
        "ambiguous_count",
    ] {
        if let Some(value) = status_json_i64_field(text, key) {
            summary.push(format!("{key}={value}"));
            if value > 0 {
                needs_review = true;
            }
        }
    }
    needs_review
}

fn status_json_string_field(text: &str, key: &str) -> Option<String> {
    let marker = format!("\"{key}\"");
    let idx = text.find(&marker)?;
    let after_key = &text[idx + marker.len()..];
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim_start();
    let body = after_colon.strip_prefix('"')?;
    let mut out = String::new();
    let mut escaped = false;
    for ch in body.chars() {
        if escaped {
            out.push(ch);
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

fn status_json_i64_field(text: &str, key: &str) -> Option<i64> {
    let marker = format!("\"{key}\"");
    let idx = text.find(&marker)?;
    let after_key = &text[idx + marker.len()..];
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim_start();
    let number = after_colon
        .chars()
        .take_while(|ch| ch.is_ascii_digit() || *ch == '-')
        .collect::<String>();
    if number.is_empty() || number == "-" {
        None
    } else {
        number.parse().ok()
    }
}

fn package_token(package: &WorkPackage) -> String {
    package
        .id
        .split_once('_')
        .map(|(_, rest)| rest)
        .unwrap_or(&package.id)
        .to_string()
}

fn package_tag(package: &WorkPackage) -> Option<String> {
    if package.kind != "country" {
        return None;
    }
    let token = package_token(package);
    let compact = token
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>();
    if compact.len() == 3 {
        Some(compact.to_ascii_uppercase())
    } else {
        Some("<TAG>".to_string())
    }
}

fn package_namespace(package: &WorkPackage, blueprint: &LargeModBlueprint) -> String {
    let token = package_token(package);
    let acronym = slugify(&blueprint.acronym, "mod");
    match package.kind.as_str() {
        "country" | "region" | "system" => format!("{acronym}_{token}"),
        _ => format!("{acronym}_{}", slugify(&package.id, "package")),
    }
}

fn work_package_preflight_commands(
    package: &WorkPackage,
    mod_root: &str,
    tag: Option<&str>,
    namespace: &str,
) -> Vec<String> {
    let mut commands = vec![
        format!("hoi4skill build-mod-index {mod_root} --output .hoi4skill/mod_index.json"),
        format!("hoi4skill loc-audit {mod_root} --output .hoi4skill/loc_audit.json"),
        format!("hoi4skill gfx-audit {mod_root} --output .hoi4skill/gfx_audit.json"),
    ];
    match package.kind.as_str() {
        "country" => {
            let tag = tag.unwrap_or("<TAG>");
            commands.push(format!(
                "hoi4skill feature-context {mod_root} --tag {tag} --output .hoi4skill/context_{}.md",
                package.id
            ));
            commands.push(format!(
                "hoi4skill reserve-id {mod_root} --kind focus --tag {tag} --count 40 --output .hoi4skill/ids_{}_focus.json",
                package.id
            ));
            commands.push(format!(
                "hoi4skill reserve-id {mod_root} --kind event --namespace {namespace} --count 20 --output .hoi4skill/ids_{}_events.json",
                package.id
            ));
        }
        "region" | "system" => {
            let system = package_token(package);
            commands.push(format!(
                "hoi4skill feature-context {mod_root} --system {system} --output .hoi4skill/context_{}.md",
                package.id
            ));
            commands.push(format!(
                "hoi4skill reserve-id {mod_root} --kind event --namespace {namespace} --count 20 --output .hoi4skill/ids_{}_events.json",
                package.id
            ));
        }
        _ => {}
    }
    commands.push(format!(
        "hoi4skill validate {mod_root} --changed-only --changed <planned-file> --strict-code-index --output .hoi4skill/validation_{}.json",
        package.id
    ));
    commands
}

fn work_package_recommended_generators(package: &WorkPackage, tag: Option<&str>) -> Vec<String> {
    let tag = tag.unwrap_or("<TAG>");
    match package.kind.as_str() {
        "country" => vec![
            format!(
                "hoi4skill apply-focus-layout --input <layout.txt> --mod-root <mod-root> --tag {tag} --prefix <prefix> --final-check"
            ),
            format!(
                "hoi4skill apply-feature-cards --input <feature_cards.txt> --mod-root <mod-root> --tag {tag} --prefix <prefix> --final-check"
            ),
            format!(
                "hoi4skill apply-event-cards --input <event_cards.txt> --mod-root <mod-root> --tag {tag} --prefix <prefix> --final-check"
            ),
        ],
        "region" => vec![
            "hoi4skill apply-event-cards --input <regional_events.txt> --mod-root <mod-root> --tag <verified-tag> --prefix <prefix> --final-check".to_string(),
            "hoi4skill apply-feature-cards --input <regional_feature_cards.txt> --mod-root <mod-root> --tag <verified-tag> --prefix <prefix> --final-check".to_string(),
        ],
        "system" => vec![
            "hoi4skill apply-feature-cards --input <system_cards.txt> --mod-root <mod-root> --tag <verified-tag> --prefix <prefix> --final-check".to_string(),
            "hoi4skill apply-event-cards --input <system_events.txt> --mod-root <mod-root> --tag <verified-tag> --prefix <prefix> --final-check".to_string(),
        ],
        _ => Vec::new(),
    }
}

fn work_package_planned_files(
    package: &WorkPackage,
    mod_root: &str,
    tag: Option<&str>,
    token: &str,
) -> Vec<String> {
    let tag = tag.unwrap_or("<TAG>");
    match package.kind.as_str() {
        "country" => vec![
            format!("{mod_root}/common/national_focus/{token}_{tag}_focus.txt"),
            format!("{mod_root}/common/ideas/{token}_{tag}_ideas.txt"),
            format!("{mod_root}/common/decisions/{token}_{tag}_decisions.txt"),
            format!("{mod_root}/events/{token}_{tag}_events.txt"),
            format!("{mod_root}/localisation/<language>/{token}_{tag}_l_<language>.yml"),
            format!("{mod_root}/interface/{token}_{tag}.gfx"),
            format!("{mod_root}/gfx/interface/goals/<asset>.dds"),
        ],
        "region" => vec![
            format!("{mod_root}/events/{token}_regional_events.txt"),
            format!("{mod_root}/common/decisions/{token}_regional_decisions.txt"),
            format!("{mod_root}/common/scripted_effects/{token}_regional_effects.txt"),
            format!("{mod_root}/common/scripted_triggers/{token}_regional_triggers.txt"),
            format!("{mod_root}/localisation/<language>/{token}_region_l_<language>.yml"),
        ],
        "system" => vec![
            format!("{mod_root}/common/scripted_effects/{token}_effects.txt"),
            format!("{mod_root}/common/scripted_triggers/{token}_triggers.txt"),
            format!("{mod_root}/common/on_actions/{token}_on_actions.txt"),
            format!("{mod_root}/common/decisions/{token}_decisions.txt"),
            format!("{mod_root}/localisation/<language>/{token}_system_l_<language>.yml"),
        ],
        _ => Vec::new(),
    }
}

fn asset_pack_plan_markdown(
    blueprint: &LargeModBlueprint,
    package: &WorkPackage,
    blueprint_path: &Path,
    mod_root: Option<&Path>,
) -> String {
    let token = package_token(package);
    let tag = package_tag(package).unwrap_or_else(|| "<TAG>".to_string());
    let root = mod_root
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<mod-root>".to_string());
    let prefix = format!(
        "{}_{}",
        slugify(&blueprint.acronym, "mod"),
        slugify(&token, "package")
    );
    let slots = asset_pack_slots(package);

    let mut out = String::new();
    out.push_str(&format!("# Asset Pack Plan: {}\n\n", package.title));
    out.push_str("- schema: `hoi4skill.asset_pack_plan.v1`\n");
    out.push_str(&format!("- mod: `{}`\n", blueprint.name));
    out.push_str(&format!("- package: `{}`\n", package.id));
    out.push_str(&format!("- kind: `{}`\n", package.kind));
    out.push_str(&format!("- blueprint: `{}`\n", blueprint_path.display()));
    out.push_str(&format!("- mod_root: `{root}`\n"));
    if package.kind == "country" {
        out.push_str(&format!("- tag_hint: `{tag}`\n"));
    }
    out.push_str(&format!("- prefix_hint: `{prefix}`\n\n"));

    out.push_str("## Asset Slots\n\n");
    for slot in slots {
        out.push_str(&format!(
            "- `{}`: {} asset(s); sprite prefix `{}`; texture path `{}`\n",
            slot.kind, slot.count, slot.sprite_prefix, slot.texture_path
        ));
    }

    out.push_str("\n## Naming Rules\n\n");
    out.push_str(&format!(
        "- Focus icon sprite: `GFX_goal_{prefix}_<english_slug>` with texture `gfx/interface/goals/{prefix}_<english_slug>.dds`\n"
    ));
    out.push_str(&format!(
        "- Idea icon sprite: `GFX_idea_{prefix}_<english_slug>` with texture `gfx/interface/ideas/{prefix}_<english_slug>.dds`\n"
    ));
    out.push_str(&format!(
        "- Event picture sprite: `GFX_report_event_{prefix}_<english_slug>` with texture `gfx/event_pictures/{prefix}_<english_slug>.dds`\n"
    ));
    out.push_str(&format!(
        "- Decision icon sprite: `GFX_decision_{prefix}_<english_slug>` with texture `gfx/interface/decisions/{prefix}_<english_slug>.dds`\n"
    ));
    out.push_str("- Filenames should be ASCII English slugs; keep Chinese descriptions in the asset request list, not in filenames.\n");

    out.push_str("\n## Expected Files\n\n");
    out.push_str(&format!("- `{root}/interface/{prefix}_goals.gfx`\n"));
    out.push_str(&format!(
        "- `{root}/interface/{prefix}_focus_idea_icons.gfx`\n"
    ));
    out.push_str(&format!(
        "- `{root}/interface/{prefix}_event_pictures.gfx`\n"
    ));
    out.push_str(&format!(
        "- `{root}/interface/{prefix}_decision_pictures.gfx`\n"
    ));
    out.push_str(&format!("- `{root}/gfx/interface/goals/*.dds`\n"));
    out.push_str(&format!("- `{root}/gfx/interface/ideas/*.dds`\n"));
    out.push_str(&format!("- `{root}/gfx/interface/decisions/*.dds`\n"));
    out.push_str(&format!("- `{root}/gfx/event_pictures/*.dds`\n"));

    out.push_str("\n## Blueprint Asset Needs\n\n");
    for need in &blueprint.asset_needs {
        out.push_str(&format!("- `{need}`\n"));
    }

    out.push_str("\n## Commands\n\n");
    out.push_str(&format!(
        "- `hoi4skill icon-preview --mod-root {root} --output .hoi4skill/icon_preview_{}`\n",
        package.id
    ));
    out.push_str(&format!(
        "- `hoi4skill register-gfx-icons --mod-root {root} --prefix {prefix} --category all --output .hoi4skill/gfx_register_{}.json`\n",
        package.id
    ));
    out.push_str(&format!(
        "- `hoi4skill gfx-audit {root} --output .hoi4skill/gfx_audit_{}.json`\n",
        package.id
    ));
    out.push_str(&format!(
        "- `hoi4skill validate {root} --changed-only --changed interface/{prefix}_goals.gfx --strict-code-index`\n"
    ));

    out.push_str("\n## Stop Conditions\n\n");
    out.push_str("- Do not create portraits, characters, GUI, technologies, or new country tags unless the literal user request authorizes them.\n");
    out.push_str("- Do not leave script references to missing `GFX_*` sprites; run `gfx-audit` before final validation.\n");
    out.push_str("- Do not use non-ASCII image filenames; register renamed assets with `register-gfx-icons`.\n");
    out.push_str("- Treat missing user-provided visual requirements as unfinished work.\n");
    out
}

struct AssetSlot {
    kind: &'static str,
    count: usize,
    sprite_prefix: &'static str,
    texture_path: &'static str,
}

fn asset_pack_slots(package: &WorkPackage) -> Vec<AssetSlot> {
    match package.kind.as_str() {
        "country" => vec![
            AssetSlot {
                kind: "focus_icons",
                count: 40,
                sprite_prefix: "GFX_goal",
                texture_path: "gfx/interface/goals",
            },
            AssetSlot {
                kind: "event_pictures",
                count: 12,
                sprite_prefix: "GFX_report_event",
                texture_path: "gfx/event_pictures",
            },
            AssetSlot {
                kind: "idea_icons",
                count: 8,
                sprite_prefix: "GFX_idea",
                texture_path: "gfx/interface/ideas",
            },
            AssetSlot {
                kind: "decision_icons",
                count: 8,
                sprite_prefix: "GFX_decision",
                texture_path: "gfx/interface/decisions",
            },
        ],
        "region" => vec![
            AssetSlot {
                kind: "event_pictures",
                count: 20,
                sprite_prefix: "GFX_report_event",
                texture_path: "gfx/event_pictures",
            },
            AssetSlot {
                kind: "decision_icons",
                count: 8,
                sprite_prefix: "GFX_decision",
                texture_path: "gfx/interface/decisions",
            },
        ],
        "system" => vec![
            AssetSlot {
                kind: "idea_icons",
                count: 8,
                sprite_prefix: "GFX_idea",
                texture_path: "gfx/interface/ideas",
            },
            AssetSlot {
                kind: "decision_icons",
                count: 8,
                sprite_prefix: "GFX_decision",
                texture_path: "gfx/interface/decisions",
            },
            AssetSlot {
                kind: "event_pictures",
                count: 6,
                sprite_prefix: "GFX_report_event",
                texture_path: "gfx/event_pictures",
            },
        ],
        _ => Vec::new(),
    }
}
