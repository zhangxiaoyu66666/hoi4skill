//! Pre-edit context packaging for model-assisted HOI4 edits.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_prepare_edit_context(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = value(&map, "input").map(normalize_path).transpose()?;
    let direct_text = value(&map, "text");
    if input.is_none() && direct_text.is_none() {
        return Err("prepare-edit-context requires --input <file> or --text <request>".to_string());
    }
    let mod_input = normalize_path(&require_value(&map, "mod-root")?)?;
    let tag = value(&map, "tag").unwrap_or("TAG");
    let prefix = value(&map, "prefix").unwrap_or("mod");
    let sheet = value(&map, "sheet");
    let tree_id = value(&map, "tree-id");
    let max_items = parse_usize_option(&map, "max-items", 80)?;
    let max_sprites = parse_usize_option(&map, "max-sprites", 400)?;
    let max_context_files = parse_usize_option(&map, "max-context-files", 24)?;
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    let dependency_roots =
        dependency_mod_roots_for_edited_mod(&map, &mod_input, game_root.is_some())?;
    if game_root.is_none() && !dependency_roots.is_empty() {
        return Err("--mod-path requires --game-root during edit-context preparation".to_string());
    }
    let game_index = game_root
        .as_ref()
        .map(|path| build_game_index_with_mod_paths(path, &dependency_roots))
        .transpose()?;
    let requested_library = value(&map, "code-library")
        .map(normalize_path)
        .transpose()?;
    let code_mod_roots = code_mod_roots(&map)?;
    if !code_mod_roots.is_empty() {
        let request = value(&map, "request").ok_or_else(|| {
            "--code-mod-path requires --request with the user's literal authorization".to_string()
        })?;
        enforce_mod_code_request(request, &code_mod_roots)?;
    }
    let code_libraries = game_root
        .as_ref()
        .map(|root| {
            ensure_clausewitz_libraries(root, &code_mod_roots, requested_library.as_deref())
        })
        .transpose()?
        .or_else(|| requested_library.map(|path| vec![path]));
    enforce_tag_request_contract(&map, tag, game_index.as_ref())?;

    let (input_label, workflow_input) = if let Some(text) = direct_text {
        (
            "<inline --text>".to_string(),
            workflow_input_from_text(text),
        )
    } else {
        let input = input.as_deref().expect("input checked above");
        (
            input.display().to_string(),
            workflow_input_from_path(input, sheet, tag, prefix)?,
        )
    };
    let context = prepare_edit_context_markdown_from_workflow_input(
        &input_label,
        workflow_input,
        &mod_input,
        tag,
        prefix,
        sheet,
        tree_id,
        value(&map, "request"),
        &dependency_roots,
        game_root.as_deref(),
        game_index.as_ref(),
        max_items,
        max_sprites,
        max_context_files,
        code_libraries.as_deref(),
        true,
    )?;
    let output = value(&map, "output");
    write_or_print(&context, output)?;
    write_ai_context_contract_sidecar(
        &context,
        &mod_input,
        value(&map, "context-contract-output"),
        output.is_some() && !map.flags.contains("no-context-contract-sidecar"),
    )?;
    Ok(())
}

pub(crate) fn cmd_ai_repair_prompt(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let edit_context_path = value(&map, "edit-context")
        .or_else(|| value(&map, "context"))
        .map(|path| path.to_string())
        .or_else(|| map.positionals.first().cloned())
        .ok_or_else(|| "ai-repair-prompt requires --edit-context <edit_context.md>".to_string())?;
    let edit_context_path = normalize_path(&edit_context_path)?;
    let edit_context = read_utf8_lossy(&edit_context_path)?;
    let repair_context =
        optional_repair_prompt_input(&map, &["repair-context", "validation-context"])?;
    let failed_patch =
        optional_repair_prompt_input(&map, &["failed-patch", "failed-output", "patch", "diff"])?;
    let max_context_chars = parse_usize_option(&map, "max-context-chars", 80_000)?;
    let max_repair_chars = parse_usize_option(&map, "max-repair-chars", 40_000)?;
    let max_patch_chars = parse_usize_option(&map, "max-patch-chars", 40_000)?;
    let prompt = render_ai_repair_prompt(
        value(&map, "request"),
        &edit_context_path,
        &edit_context,
        repair_context.as_ref(),
        failed_patch.as_ref(),
        max_context_chars,
        max_repair_chars,
        max_patch_chars,
    );
    write_or_print(&prompt, value(&map, "output"))
}

pub(crate) fn cmd_repair_failed_output(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let text = read_utf8_lossy(&input)?;
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    let kind = value(&map, "kind").unwrap_or("auto");
    let source_kind = classify_failed_output_source(&text, kind)?;
    let dependency_roots = repair_failed_output_dependency_roots(&map, &text, source_kind)?;
    let changed_files = repair_failed_output_changed_files(&map);
    let max_items = parse_usize_option(&map, "max-items", 80)?.max(1);
    let context = FailedOutputContext {
        mod_root,
        game_root,
        dependency_roots,
        changed_files,
    };
    let pack = render_repair_failed_output_pack(&input, &text, kind, &context, max_items)?;
    write_or_print(&pack, value(&map, "output"))
}

fn repair_failed_output_changed_files(map: &ArgMap) -> Vec<String> {
    let mut out = Vec::new();
    for key in ["changed", "changed-file"] {
        for value in repeated_values(map, key) {
            let value = value.trim();
            if !value.is_empty() && !out.iter().any(|item| item == value) {
                out.push(value.to_string());
            }
        }
    }
    for key in ["changed-list", "changed-files"] {
        for raw in repeated_values(map, key) {
            let path = if Path::new(raw).is_absolute() {
                PathBuf::from(raw)
            } else if let Some(root) =
                value(map, "mod-root").and_then(|item| normalize_path(item).ok())
            {
                root.join(raw)
            } else {
                PathBuf::from(raw)
            };
            if let Ok(items) = read_changed_paths_file(&path) {
                for item in items {
                    if !item.trim().is_empty() && !out.iter().any(|existing| existing == &item) {
                        out.push(item);
                    }
                }
            }
        }
    }
    out
}

fn repair_failed_output_dependency_roots(
    map: &ArgMap,
    text: &str,
    source_kind: &str,
) -> Result<Vec<PathBuf>, String> {
    match dependency_mod_roots(map) {
        Ok(roots) => Ok(roots),
        Err(err)
            if source_kind == "validation_report"
                && err.starts_with("--auto-mod-paths")
                && validation_report_has_embedded_context(text) =>
        {
            Ok(Vec::new())
        }
        Err(err) => Err(err),
    }
}

fn validation_report_has_embedded_context(text: &str) -> bool {
    json_string_field(text, "mod_root").is_some()
        || json_string_field(text, "game_root").is_some()
        || !json_string_array_field(text, "dependency_mods").is_empty()
}

pub(crate) fn cmd_ai_repair_bundle(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = map
        .positionals
        .first()
        .cloned()
        .or_else(|| map.values.get("mod-root").cloned())
        .ok_or_else(|| "ai-repair-bundle requires <mod-root> or --mod-root".to_string())?;
    let mod_root = normalize_path(&mod_root)?;
    let game_root = repair_bundle_game_root(&map)?;
    let output_dir = value(&map, "output-dir")
        .map(normalize_path)
        .transpose()?
        .unwrap_or_else(|| mod_root.join(".hoi4skill").join("repair_bundle"));
    fs::create_dir_all(&output_dir).map_err(|e| format!("create {}: {e}", output_dir.display()))?;
    let edit_context = ensure_repair_bundle_edit_context(&map, &mod_root, &game_root, &output_dir)?;
    let validation_path = output_dir.join("validation.json");
    let repair_context_path = output_dir.join("ai_repair_context.json");
    let failed_output_path = output_dir.join("failed_output.md");
    let error_log_report_path = output_dir.join("error_log_report.json");
    let logic_audit_path = output_dir.join("logic_audit.json");
    let loc_audit_path = output_dir.join("loc_audit.json");
    let gfx_audit_path = output_dir.join("gfx_audit.json");
    let boundary_path = output_dir.join("boundary.json");
    let repair_prompt_path = output_dir.join("repair_prompt.md");
    let manifest_path = value(&map, "output")
        .map(normalize_path)
        .transpose()?
        .unwrap_or_else(|| output_dir.join("repair_bundle.json"));
    let max_items = parse_usize_option(&map, "max-items", 80)?.max(1);

    let mut validate_args = vec![
        mod_root.display().to_string(),
        "--game-root".to_string(),
        game_root.display().to_string(),
        "--strict-code-index".to_string(),
        "--output".to_string(),
        validation_path.display().to_string(),
    ];
    push_common_repair_bundle_validation_args(&mut validate_args, &map);
    let validation_result = cmd_validate(&validate_args);
    let validation_ok = match validation_result {
        Ok(()) => true,
        Err(err) if err == "validation failed" => false,
        Err(err) => return Err(err),
    };
    let mut repair_findings = !validation_ok;

    let mut repair_context_args = vec![
        mod_root.display().to_string(),
        "--game-root".to_string(),
        game_root.display().to_string(),
        "--max-items".to_string(),
        max_items.to_string(),
        "--output".to_string(),
        repair_context_path.display().to_string(),
    ];
    push_common_repair_bundle_validation_args(&mut repair_context_args, &map);
    cmd_validate_repair_context(&repair_context_args)?;

    let has_error_log = value(&map, "error-log")
        .or_else(|| value(&map, "error-log-report"))
        .is_some();
    let has_boundary = value(&map, "package")
        .or_else(|| value(&map, "boundary-package"))
        .is_some();
    let has_logic_audit = map.flags.contains("logic-audit") || map.flags.contains("logic");
    let has_loc_audit = map.flags.contains("loc-audit") || map.flags.contains("localisation-audit");
    let has_gfx_audit = map.flags.contains("gfx-audit") || map.flags.contains("asset-audit");
    let validation_failed_output_path =
        if has_error_log || has_boundary || has_logic_audit || has_loc_audit || has_gfx_audit {
            output_dir.join("validation_failed_output.md")
        } else {
            failed_output_path.clone()
        };
    let mut validation_failed_args = vec![
        "--input".to_string(),
        validation_path.display().to_string(),
        "--kind".to_string(),
        "validation-report".to_string(),
        "--max-items".to_string(),
        max_items.to_string(),
        "--output".to_string(),
        validation_failed_output_path.display().to_string(),
        "--mod-root".to_string(),
        mod_root.display().to_string(),
        "--game-root".to_string(),
        game_root.display().to_string(),
    ];
    push_common_repair_bundle_validation_args(&mut validation_failed_args, &map);
    cmd_repair_failed_output(&validation_failed_args)?;
    let mut failed_output_packs = vec![validation_failed_output_path.clone()];
    let mut optional_artifacts = Vec::new();
    if let Some(package) = value(&map, "package").or_else(|| value(&map, "boundary-package")) {
        let mut boundary_args = vec![
            "--mod-root".to_string(),
            mod_root.display().to_string(),
            "--package".to_string(),
            package.to_string(),
            "--strict-names".to_string(),
            "--fail-on-violation".to_string(),
            "--output".to_string(),
            boundary_path.display().to_string(),
        ];
        push_repair_bundle_boundary_args(&mut boundary_args, &map);
        let boundary_result = cmd_check_work_package_boundary(&boundary_args);
        match boundary_result {
            Ok(()) => {}
            Err(err) if err.contains("work-package boundary gate failed") => {
                repair_findings = true;
            }
            Err(err) => return Err(err),
        }
        let boundary_failed_output_path = output_dir.join("boundary_failed_output.md");
        let mut boundary_failed_args = vec![
            "--input".to_string(),
            boundary_path.display().to_string(),
            "--kind".to_string(),
            "boundary-report".to_string(),
            "--max-items".to_string(),
            max_items.to_string(),
            "--output".to_string(),
            boundary_failed_output_path.display().to_string(),
            "--mod-root".to_string(),
            mod_root.display().to_string(),
        ];
        push_common_repair_bundle_validation_args(&mut boundary_failed_args, &map);
        cmd_repair_failed_output(&boundary_failed_args)?;
        optional_artifacts.push(("boundary_report", boundary_path.display().to_string()));
        optional_artifacts.push((
            "boundary_failed_output_pack",
            boundary_failed_output_path.display().to_string(),
        ));
        failed_output_packs.push(boundary_failed_output_path);
    }
    if let Some(error_log_report) = value(&map, "error-log-report") {
        let error_log_report = normalize_path(error_log_report)?;
        let error_log_failed_output_path = output_dir.join("error_log_failed_output.md");
        let mut error_log_failed_args = vec![
            "--input".to_string(),
            error_log_report.display().to_string(),
            "--kind".to_string(),
            "error-log-report".to_string(),
            "--max-items".to_string(),
            max_items.to_string(),
            "--output".to_string(),
            error_log_failed_output_path.display().to_string(),
            "--mod-root".to_string(),
            mod_root.display().to_string(),
        ];
        push_common_repair_bundle_validation_args(&mut error_log_failed_args, &map);
        cmd_repair_failed_output(&error_log_failed_args)?;
        let error_log_text = read_utf8_lossy(&error_log_report)?;
        if !json_i64_field_is_zero(&error_log_text, "diagnostics_effective") {
            repair_findings = true;
        }
        optional_artifacts.push(("error_log_report", error_log_report.display().to_string()));
        optional_artifacts.push((
            "error_log_failed_output_pack",
            error_log_failed_output_path.display().to_string(),
        ));
        failed_output_packs.push(error_log_failed_output_path);
    } else if let Some(error_log) = value(&map, "error-log") {
        let error_log = normalize_path(error_log)?;
        let mut analyze_args = vec![
            "--input".to_string(),
            error_log.display().to_string(),
            "--mod-root".to_string(),
            mod_root.display().to_string(),
            "--output".to_string(),
            error_log_report_path.display().to_string(),
        ];
        push_repair_bundle_error_log_args(&mut analyze_args, &map);
        cmd_analyze_error_log(&analyze_args)?;
        let error_log_text = read_utf8_lossy(&error_log_report_path)?;
        if !json_i64_field_is_zero(&error_log_text, "diagnostics_effective") {
            repair_findings = true;
        }
        let error_log_failed_output_path = output_dir.join("error_log_failed_output.md");
        let mut error_log_failed_args = vec![
            "--input".to_string(),
            error_log_report_path.display().to_string(),
            "--kind".to_string(),
            "error-log-report".to_string(),
            "--max-items".to_string(),
            max_items.to_string(),
            "--output".to_string(),
            error_log_failed_output_path.display().to_string(),
            "--mod-root".to_string(),
            mod_root.display().to_string(),
        ];
        push_common_repair_bundle_validation_args(&mut error_log_failed_args, &map);
        cmd_repair_failed_output(&error_log_failed_args)?;
        optional_artifacts.push((
            "error_log_report",
            error_log_report_path.display().to_string(),
        ));
        optional_artifacts.push((
            "error_log_failed_output_pack",
            error_log_failed_output_path.display().to_string(),
        ));
        failed_output_packs.push(error_log_failed_output_path);
    }
    if has_logic_audit {
        let mut logic_audit_args = vec![
            mod_root.display().to_string(),
            "--max-items".to_string(),
            max_items.to_string(),
            "--output".to_string(),
            logic_audit_path.display().to_string(),
        ];
        push_repair_bundle_logic_audit_args(&mut logic_audit_args, &map);
        cmd_logic_audit(&logic_audit_args)?;
        let logic_audit_text = read_utf8_lossy(&logic_audit_path)?;
        if !json_i64_field_is_zero(&logic_audit_text, "issue_count") {
            repair_findings = true;
        }
        let logic_failed_output_path = output_dir.join("logic_audit_failed_output.md");
        let mut logic_failed_args = vec![
            "--input".to_string(),
            logic_audit_path.display().to_string(),
            "--kind".to_string(),
            "logic-audit-report".to_string(),
            "--max-items".to_string(),
            max_items.to_string(),
            "--output".to_string(),
            logic_failed_output_path.display().to_string(),
            "--mod-root".to_string(),
            mod_root.display().to_string(),
        ];
        push_common_repair_bundle_validation_args(&mut logic_failed_args, &map);
        cmd_repair_failed_output(&logic_failed_args)?;
        optional_artifacts.push(("logic_audit_report", logic_audit_path.display().to_string()));
        optional_artifacts.push((
            "logic_audit_failed_output_pack",
            logic_failed_output_path.display().to_string(),
        ));
        failed_output_packs.push(logic_failed_output_path);
    }
    if has_loc_audit {
        let mut loc_audit_args = vec![
            mod_root.display().to_string(),
            "--max-items".to_string(),
            max_items.to_string(),
            "--output".to_string(),
            loc_audit_path.display().to_string(),
        ];
        push_repair_bundle_changed_args(&mut loc_audit_args, &map);
        cmd_loc_audit(&loc_audit_args)?;
        let loc_audit_text = read_utf8_lossy(&loc_audit_path)?;
        if !loc_audit_report_is_clean(&loc_audit_text) {
            repair_findings = true;
        }
        let loc_failed_output_path = output_dir.join("loc_audit_failed_output.md");
        let mut loc_failed_args = vec![
            "--input".to_string(),
            loc_audit_path.display().to_string(),
            "--kind".to_string(),
            "loc-audit-report".to_string(),
            "--max-items".to_string(),
            max_items.to_string(),
            "--output".to_string(),
            loc_failed_output_path.display().to_string(),
            "--mod-root".to_string(),
            mod_root.display().to_string(),
        ];
        push_common_repair_bundle_validation_args(&mut loc_failed_args, &map);
        cmd_repair_failed_output(&loc_failed_args)?;
        optional_artifacts.push(("loc_audit_report", loc_audit_path.display().to_string()));
        optional_artifacts.push((
            "loc_audit_failed_output_pack",
            loc_failed_output_path.display().to_string(),
        ));
        failed_output_packs.push(loc_failed_output_path);
    }
    if has_gfx_audit {
        let mut gfx_audit_args = vec![
            mod_root.display().to_string(),
            "--max-items".to_string(),
            max_items.to_string(),
            "--output".to_string(),
            gfx_audit_path.display().to_string(),
        ];
        push_repair_bundle_changed_args(&mut gfx_audit_args, &map);
        cmd_gfx_audit(&gfx_audit_args)?;
        let gfx_audit_text = read_utf8_lossy(&gfx_audit_path)?;
        if !gfx_audit_report_is_clean(&gfx_audit_text) {
            repair_findings = true;
        }
        let gfx_failed_output_path = output_dir.join("gfx_audit_failed_output.md");
        let mut gfx_failed_args = vec![
            "--input".to_string(),
            gfx_audit_path.display().to_string(),
            "--kind".to_string(),
            "gfx-audit-report".to_string(),
            "--max-items".to_string(),
            max_items.to_string(),
            "--output".to_string(),
            gfx_failed_output_path.display().to_string(),
            "--mod-root".to_string(),
            mod_root.display().to_string(),
        ];
        push_common_repair_bundle_validation_args(&mut gfx_failed_args, &map);
        cmd_repair_failed_output(&gfx_failed_args)?;
        optional_artifacts.push(("gfx_audit_report", gfx_audit_path.display().to_string()));
        optional_artifacts.push((
            "gfx_audit_failed_output_pack",
            gfx_failed_output_path.display().to_string(),
        ));
        failed_output_packs.push(gfx_failed_output_path);
    }
    if failed_output_packs.len() > 1 {
        combine_failed_output_packs(&failed_output_packs, &failed_output_path)?;
    }

    let mut repair_prompt_args = vec![
        "--edit-context".to_string(),
        edit_context.display().to_string(),
        "--repair-context".to_string(),
        repair_context_path.display().to_string(),
        "--failed-patch".to_string(),
        failed_output_path.display().to_string(),
        "--output".to_string(),
        repair_prompt_path.display().to_string(),
    ];
    if let Some(request) = value(&map, "request") {
        repair_prompt_args.push("--request".to_string());
        repair_prompt_args.push(request.to_string());
    }
    cmd_ai_repair_prompt(&repair_prompt_args)?;

    let mut artifacts = vec![
        ("validation_report", validation_path.display().to_string()),
        (
            "validation_repair_context",
            repair_context_path.display().to_string(),
        ),
        (
            "failed_output_pack",
            failed_output_path.display().to_string(),
        ),
        ("ai_repair_prompt", repair_prompt_path.display().to_string()),
    ];
    artifacts.extend(optional_artifacts);
    let artifacts_json = artifacts
        .iter()
        .map(|(name, path)| {
            format!(
                "{{\"name\": {}, \"path\": {}}}",
                json_str(name),
                json_str(path)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let dependency_args = repair_bundle_dependency_command_args(&map)?;
    let changed_files_for_commands = repair_failed_output_changed_files(&map);
    let changed_args = failed_output_changed_command_args(&changed_files_for_commands);
    let mut next_commands = vec![
        format!("open {}", command_path_arg(&repair_prompt_path)),
        format!(
            "hoi4skill validate {} --game-root {}{}{} --strict-code-index",
            command_path_arg(&mod_root),
            command_path_arg(&game_root),
            dependency_args,
            changed_args
        ),
    ];
    if has_logic_audit {
        next_commands.push(format!(
            "hoi4skill logic-audit {}{} --output {}",
            command_path_arg(&mod_root),
            changed_args,
            command_path_arg(&logic_audit_path)
        ));
    }
    if has_loc_audit {
        next_commands.push(format!(
            "hoi4skill loc-audit {}{} --output {}",
            command_path_arg(&mod_root),
            changed_args,
            command_path_arg(&loc_audit_path)
        ));
    }
    if has_gfx_audit {
        next_commands.push(format!(
            "hoi4skill gfx-audit {}{} --output {}",
            command_path_arg(&mod_root),
            changed_args,
            command_path_arg(&gfx_audit_path)
        ));
    }
    let manifest = format!(
        "{{\n  \"schema\": \"hoi4skill.ai_repair_bundle.v1\",\n  \"status\": {},\n  \"validation_ok\": {},\n  \"mod_root\": {},\n  \"game_root\": {},\n  \"edit_context\": {},\n  \"output_dir\": {},\n  \"artifacts\": [{}],\n  \"next_commands\": {}\n}}\n",
        if repair_findings {
            json_str("repair_prompt_ready")
        } else {
            json_str("validation_clean")
        },
        json_bool(validation_ok),
        json_str(&mod_root.display().to_string()),
        json_str(&game_root.display().to_string()),
        json_str(&edit_context.display().to_string()),
        json_str(&output_dir.display().to_string()),
        artifacts_json,
        json_array(&next_commands)
    );
    write_or_print(&manifest, Some(&manifest_path.display().to_string()))
}

fn repair_bundle_game_root(map: &ArgMap) -> Result<PathBuf, String> {
    if let Some(path) = value(map, "game-root").or_else(|| value(map, "hoi4-path")) {
        return normalize_path(path);
    }
    detect_selected_hoi4_path(&[]).ok_or_else(|| {
        "ai-repair-bundle requires --game-root <Hearts of Iron IV> because detect-hoi4-path found no valid selected path".to_string()
    })
}

fn ensure_repair_bundle_edit_context(
    map: &ArgMap,
    mod_root: &Path,
    game_root: &Path,
    output_dir: &Path,
) -> Result<PathBuf, String> {
    if let Some(existing) = value(map, "edit-context").or_else(|| value(map, "context")) {
        return normalize_path(existing);
    }
    let input = value(map, "input")
        .or_else(|| value(map, "copy"))
        .ok_or_else(|| {
            "ai-repair-bundle requires --edit-context <edit_context.md> or --input <request file> --tag <TAG> --prefix <prefix>".to_string()
        })?;
    let input = normalize_path(input)?;
    let tag = value(map, "tag")
        .ok_or_else(|| "ai-repair-bundle auto edit-context requires --tag <TAG>".to_string())?;
    let prefix = value(map, "prefix").ok_or_else(|| {
        "ai-repair-bundle auto edit-context requires --prefix <prefix>".to_string()
    })?;
    let dependency_roots = dependency_mod_roots(map)?;
    let game_index = build_game_index_with_mod_paths(game_root, &dependency_roots)?;
    let max_items = parse_usize_option(map, "max-items", 80)?;
    let max_sprites = parse_usize_option(map, "max-sprites", 400)?;
    let max_context_files = parse_usize_option(map, "max-context-files", 24)?;
    let requested_library = value(map, "code-library").map(normalize_path).transpose()?;
    let code_mod_roots = code_mod_roots(map)?;
    if !code_mod_roots.is_empty() {
        let request = value(map, "request").ok_or_else(|| {
            "--code-mod-path requires --request with the user's literal authorization".to_string()
        })?;
        enforce_mod_code_request(request, &code_mod_roots)?;
    }
    let code_libraries =
        ensure_clausewitz_libraries(game_root, &code_mod_roots, requested_library.as_deref())?;
    enforce_tag_request_contract(map, tag, Some(&game_index))?;
    let mut workflow_input = workflow_input_from_path(&input, value(map, "sheet"), tag, prefix)?;
    append_explicit_request(&mut workflow_input, value(map, "request"));
    let context = prepare_edit_context_markdown_from_workflow_input(
        &input.display().to_string(),
        workflow_input,
        mod_root,
        tag,
        prefix,
        value(map, "sheet"),
        value(map, "tree-id"),
        None,
        &dependency_roots,
        Some(game_root),
        Some(&game_index),
        max_items,
        max_sprites,
        max_context_files,
        Some(&code_libraries),
        false,
    )?;
    let edit_context = output_dir.join("edit_context.md");
    write_or_print(&context, Some(&edit_context.display().to_string()))?;
    write_ai_context_contract_sidecar(&context, mod_root, None, true)?;
    Ok(edit_context)
}

fn write_ai_context_contract_sidecar(
    context: &str,
    mod_input: &Path,
    explicit_output: Option<&str>,
    default_to_mod_root: bool,
) -> Result<Option<PathBuf>, String> {
    if explicit_output.is_none() && !default_to_mod_root {
        return Ok(None);
    }
    let contract = extract_ai_context_contract_json(context)?;
    let output = if let Some(path) = explicit_output {
        normalize_path(path)?
    } else {
        resolve_mod_root(mod_input)?
            .root
            .join(".hoi4skill")
            .join("ai_context_contract.json")
    };
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    fs::write(&output, contract).map_err(|e| format!("write {}: {e}", output.display()))?;
    Ok(Some(output))
}

fn extract_ai_context_contract_json(context: &str) -> Result<String, String> {
    let schema_marker = "\"schema\": \"hoi4skill.ai_context_contract.v1\"";
    let schema_idx = context
        .find(schema_marker)
        .ok_or_else(|| "edit context is missing hoi4skill.ai_context_contract.v1".to_string())?;
    let fence_start = context[..schema_idx]
        .rfind("````json")
        .ok_or_else(|| "edit context AI contract is missing a json fence".to_string())?;
    let after_fence = &context[fence_start..];
    let line_end = after_fence
        .find('\n')
        .ok_or_else(|| "edit context AI contract json fence is malformed".to_string())?;
    let json_start = fence_start + line_end + 1;
    let json_end = context[json_start..]
        .find("\n````")
        .map(|idx| json_start + idx)
        .ok_or_else(|| "edit context AI contract json fence is not closed".to_string())?;
    let contract = context[json_start..json_end].trim();
    if !contract.starts_with('{') || !contract.ends_with('}') {
        return Err("edit context AI contract is not a JSON object".to_string());
    }
    Ok(format!("{contract}\n"))
}

fn push_common_repair_bundle_validation_args(args: &mut Vec<String>, map: &ArgMap) {
    for key in [
        "mod-path",
        "dependency-mod",
        "dependency-mod-path",
        "launcher-dir",
        "changed",
        "changed-file",
        "changed-list",
        "changed-files",
        "baseline",
        "text-source",
        "expect-title",
    ] {
        for item in repeated_values(map, key) {
            args.push(format!("--{key}"));
            args.push(item.to_string());
        }
    }
    for flag in [
        "changed-only",
        "check-output",
        "check-output-text",
        "auto-mod-paths",
        "resolve-dependencies",
        "no-code-examples",
    ] {
        if map.flags.contains(flag) {
            args.push(format!("--{flag}"));
        }
    }
    if let Some(request) = value(map, "request") {
        args.push("--request".to_string());
        args.push(request.to_string());
    }
}

fn repair_bundle_dependency_command_args(map: &ArgMap) -> Result<String, String> {
    Ok(dependency_command_args_from_roots(&dependency_mod_roots(
        map,
    )?))
}

fn dependency_command_args_from_roots(dependency_roots: &[PathBuf]) -> String {
    let mut out = String::new();
    for root in dependency_roots {
        out.push_str(" --mod-path ");
        out.push_str(&command_path_arg(&root));
    }
    out
}

fn command_path_arg(path: &Path) -> String {
    format!("\"{}\"", path.display().to_string().replace('"', "\\\""))
}

fn push_repair_bundle_error_log_args(args: &mut Vec<String>, map: &ArgMap) {
    for key in ["baseline", "changed", "changed-file"] {
        for item in repeated_values(map, key) {
            args.push(format!("--{key}"));
            args.push(item.to_string());
        }
    }
    if map.flags.contains("changed-only") {
        args.push("--changed-only".to_string());
    }
}

fn push_repair_bundle_boundary_args(args: &mut Vec<String>, map: &ArgMap) {
    let mut has_changed = false;
    for key in ["changed", "changed-file"] {
        for item in repeated_values(map, key) {
            args.push(format!("--{key}"));
            args.push(item.to_string());
            has_changed = true;
        }
    }
    if map.flags.contains("from-git") || map.flags.contains("changed-from-git") {
        args.push("--from-git".to_string());
        has_changed = true;
    }
    if !has_changed {
        args.push("--from-git".to_string());
    }
}

fn push_repair_bundle_logic_audit_args(args: &mut Vec<String>, map: &ArgMap) {
    push_repair_bundle_changed_args(args, map);
}

fn push_repair_bundle_changed_args(args: &mut Vec<String>, map: &ArgMap) {
    for key in ["changed", "changed-file"] {
        for item in repeated_values(map, key) {
            args.push(format!("--{key}"));
            args.push(item.to_string());
        }
    }
    if map.flags.contains("changed-only") {
        args.push("--changed-only".to_string());
    }
}

fn combine_failed_output_packs(paths: &[PathBuf], output: &Path) -> Result<(), String> {
    let mut out = String::new();
    out.push_str("# HOI4 Combined Failed Output Pack\n\n");
    out.push_str("- schema: `hoi4skill.combined_failed_output_pack.v1`\n");
    out.push_str(&format!("- source_count: `{}`\n", paths.len()));
    out.push_str("\n## Sources\n\n");
    for path in paths {
        out.push_str(&format!("- `{}`\n", path.display()));
    }
    out.push_str("\n## Packs\n\n");
    for (idx, path) in paths.iter().enumerate() {
        out.push_str(&format!("### {}. `{}`\n\n", idx + 1, path.display()));
        let text = read_utf8_lossy(path)?;
        out.push_str(&markdown_fence("markdown", &text));
    }
    write_or_print(&out, Some(&output.display().to_string()))
}

pub(crate) fn render_repair_failed_output_pack(
    input: &Path,
    text: &str,
    kind: &str,
    context: &FailedOutputContext,
    max_items: usize,
) -> Result<String, String> {
    let source_kind = classify_failed_output_source(text, kind)?;
    let context = failed_output_context_with_source(text, source_kind, context);
    let issues = failed_output_issues(text, source_kind, &context, max_items);
    let mut out = String::new();
    out.push_str("# HOI4 Failed Output Pack\n\n");
    out.push_str("- schema: `hoi4skill.failed_output_pack.v1`\n");
    out.push_str(&format!("- source: `{}`\n", input.display()));
    out.push_str(&format!("- source_kind: `{source_kind}`\n"));
    out.push_str(&format!("- issue_count: `{}`\n", issues.len()));
    if let Some(root) = &context.mod_root {
        out.push_str(&format!("- mod_root: `{}`\n", root.display()));
    }
    if let Some(root) = &context.game_root {
        out.push_str(&format!("- game_root: `{}`\n", root.display()));
    }
    if !context.dependency_roots.is_empty() {
        out.push_str("- dependency_mod_roots:\n");
        for root in &context.dependency_roots {
            out.push_str(&format!("  - `{}`\n", root.display()));
        }
    }
    if !context.changed_files.is_empty() {
        out.push_str("- changed_files:\n");
        for path in &context.changed_files {
            out.push_str(&format!("  - `{path}`\n"));
        }
    }
    out.push_str("\n## Use With\n\n");
    out.push_str("- `hoi4skill ai-repair-prompt --edit-context edit_context.md --failed-patch failed_output.md --output repair_prompt.md`\n");
    out.push_str(
        "- If strict validation has already run, also pass `--repair-context ai_repair_context.json`.\n",
    );
    out.push_str("\n## Repair Rules\n\n");
    out.push_str("- Treat each issue below as failed generated output, not as permission to redesign the feature.\n");
    out.push_str("- Preserve user-provided text; do not hide failures by deleting localisation, event options, or follow-up links.\n");
    out.push_str("- Unknown syntax, symbols, pictures, tags, and localisation tokens must go through `check-code-symbol`, `compile-intent`, or `validate-repair-context`.\n");
    if !context.changed_files.is_empty() {
        out.push_str("- Repair only the listed `changed_files` unless the user explicitly expands the work-package boundary.\n");
    }
    out.push_str("- If an issue does not identify a file or symbol clearly, ask the user or rerun the relevant analyzer with narrower changed files.\n");
    out.push_str("\n## Issues\n\n");
    if issues.is_empty() {
        out.push_str("- No effective issues were found in the supplied source.\n");
    } else {
        for (idx, issue) in issues.iter().enumerate() {
            out.push_str(&format!("### {}. {}\n\n", idx + 1, issue.kind));
            if let Some(file) = &issue.file {
                out.push_str(&format!("- file: `{file}`\n"));
            }
            if let Some(line) = issue.line {
                out.push_str(&format!("- line: `{line}`\n"));
            }
            if let Some(category) = &issue.category {
                out.push_str(&format!("- category: `{category}`\n"));
            }
            if let Some(suggestion) = &issue.suggestion {
                out.push_str(&format!("- suggestion: {suggestion}\n"));
            }
            out.push('\n');
            out.push_str(&markdown_fence("text", &issue.message));
        }
    }
    Ok(out)
}

fn failed_output_context_with_source(
    text: &str,
    source_kind: &str,
    context: &FailedOutputContext,
) -> FailedOutputContext {
    let mut out = context.clone();
    if source_kind == "validation_report" {
        if out.mod_root.is_none() {
            out.mod_root = json_string_field(text, "mod_root").map(PathBuf::from);
        }
        if out.game_root.is_none() {
            out.game_root = json_string_field(text, "game_root").map(PathBuf::from);
        }
        if out.dependency_roots.is_empty() {
            out.dependency_roots = json_string_array_field(text, "dependency_mods")
                .into_iter()
                .map(PathBuf::from)
                .collect();
        }
        if out.changed_files.is_empty() {
            out.changed_files = json_string_array_field(text, "changed_files");
        }
    } else if source_kind == "error_log_report" {
        if out.mod_root.is_none() {
            out.mod_root = json_string_field(text, "mod_root").map(PathBuf::from);
        }
        if out.changed_files.is_empty() {
            out.changed_files = json_string_array_field(text, "changed_files");
        }
    } else if source_kind == "logic_audit_report" {
        if out.mod_root.is_none() {
            out.mod_root = json_string_field(text, "mod_root").map(PathBuf::from);
        }
        if out.changed_files.is_empty() {
            out.changed_files = json_string_array_field(text, "changed_files");
        }
    } else if source_kind == "loc_audit_report" {
        if out.mod_root.is_none() {
            out.mod_root = json_string_field(text, "mod_root").map(PathBuf::from);
        }
        if out.changed_files.is_empty() {
            out.changed_files = json_string_array_field(text, "changed_files");
        }
    } else if source_kind == "gfx_audit_report" {
        if out.mod_root.is_none() {
            out.mod_root = json_string_field(text, "mod_root").map(PathBuf::from);
        }
        if out.changed_files.is_empty() {
            out.changed_files = json_string_array_field(text, "changed_files");
        }
    }
    out
}

#[derive(Clone)]
pub(crate) struct FailedOutputIssue {
    pub(crate) kind: String,
    pub(crate) category: Option<String>,
    pub(crate) file: Option<String>,
    pub(crate) line: Option<i64>,
    pub(crate) suggestion: Option<String>,
    pub(crate) message: String,
}

#[derive(Clone, Default)]
pub(crate) struct FailedOutputContext {
    pub(crate) mod_root: Option<PathBuf>,
    pub(crate) game_root: Option<PathBuf>,
    pub(crate) dependency_roots: Vec<PathBuf>,
    pub(crate) changed_files: Vec<String>,
}

pub(crate) fn classify_failed_output_source(
    text: &str,
    kind: &str,
) -> Result<&'static str, String> {
    match kind {
        "auto" => {
            if text.contains("\"schema\": \"hoi4skill.validation_report.v1\"")
                || text.contains("\"schema\":\"hoi4skill.validation_report.v1\"")
            {
                Ok("validation_report")
            } else if text.contains("\"schema\": \"hoi4skill.error_log_report.v1\"")
                || text.contains("\"schema\":\"hoi4skill.error_log_report.v1\"")
            {
                Ok("error_log_report")
            } else if text.contains("\"schema\": \"hoi4skill.logic_audit.v1\"")
                || text.contains("\"schema\":\"hoi4skill.logic_audit.v1\"")
            {
                Ok("logic_audit_report")
            } else if text.contains("\"schema\": \"hoi4skill.loc_audit.v1\"")
                || text.contains("\"schema\":\"hoi4skill.loc_audit.v1\"")
            {
                Ok("loc_audit_report")
            } else if text.contains("\"schema\": \"hoi4skill.gfx_audit.v1\"")
                || text.contains("\"schema\":\"hoi4skill.gfx_audit.v1\"")
            {
                Ok("gfx_audit_report")
            } else if text.contains("\"schema\": \"hoi4skill.work_package_boundary.v1\"")
                || text.contains("\"schema\":\"hoi4skill.work_package_boundary.v1\"")
            {
                Ok("boundary_report")
            } else {
                Ok("error_log")
            }
        }
        "validation-report" | "validation_report" | "validate" => Ok("validation_report"),
        "error-log-report" | "error_log_report" => Ok("error_log_report"),
        "logic-audit-report" | "logic_audit_report" | "logic-audit" | "logic_audit" => {
            Ok("logic_audit_report")
        }
        "loc-audit-report" | "loc_audit_report" | "loc-audit" | "loc_audit"
        | "localisation-audit" => Ok("loc_audit_report"),
        "gfx-audit-report" | "gfx_audit_report" | "gfx-audit" | "gfx_audit"
        | "asset-audit" => Ok("gfx_audit_report"),
        "error-log" | "error_log" | "log" => Ok("error_log"),
        "boundary-report" | "boundary_report" | "work-package-boundary" => Ok("boundary_report"),
        other => Err(format!(
            "unsupported failed-output kind `{other}`; expected auto, validation-report, error-log-report, logic-audit-report, loc-audit-report, gfx-audit-report, error-log, or boundary-report"
        )),
    }
}

pub(crate) fn failed_output_issues(
    text: &str,
    source_kind: &str,
    context: &FailedOutputContext,
    max_items: usize,
) -> Vec<FailedOutputIssue> {
    match source_kind {
        "validation_report" => validation_report_failed_output_issues(text, context, max_items),
        "error_log_report" => error_log_report_failed_output_issues(text, max_items),
        "logic_audit_report" => logic_audit_report_failed_output_issues(text, max_items),
        "loc_audit_report" => loc_audit_report_failed_output_issues(text, max_items),
        "gfx_audit_report" => gfx_audit_report_failed_output_issues(text, max_items),
        "boundary_report" => boundary_report_failed_output_issues(text, max_items),
        "error_log" => analyze_error_log(text, context.mod_root.as_deref())
            .into_iter()
            .take(max_items)
            .map(|diagnostic| FailedOutputIssue {
                kind: format!("error_log_{}", diagnostic.severity),
                category: Some(diagnostic.category),
                file: diagnostic.resolved_file.or(diagnostic.file),
                line: diagnostic.line,
                suggestion: Some(diagnostic.suggestion),
                message: diagnostic.raw,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn boundary_report_failed_output_issues(text: &str, _max_items: usize) -> Vec<FailedOutputIssue> {
    if !text.contains("\"schema\": \"hoi4skill.work_package_boundary.v1\"")
        && !text.contains("\"schema\":\"hoi4skill.work_package_boundary.v1\"")
    {
        return Vec::new();
    }
    if text.contains("\"violation_count\": 0") {
        return Vec::new();
    }
    Vec::from([FailedOutputIssue {
        kind: "work_package_boundary_violation".to_string(),
        category: Some("boundary_violation".to_string()),
        file: None,
        line: None,
        suggestion: Some(
            "Move or revert files outside the package boundary; rerun check-work-package-boundary before repair is accepted."
                .to_string(),
        ),
        message: truncate_chars(text, 12_000),
    }])
}

fn validation_report_failed_output_issues(
    text: &str,
    context: &FailedOutputContext,
    max_items: usize,
) -> Vec<FailedOutputIssue> {
    let mut out = Vec::new();
    let repair_context_command = failed_output_validate_repair_context_command(context);
    for message in json_string_array_field(text, "errors") {
        out.push(FailedOutputIssue {
            kind: "validation_error".to_string(),
            category: None,
            file: failed_output_file_from_message(&message),
            line: None,
            suggestion: Some(format!(
                "Run `{repair_context_command}` to get related indexed code and required questions."
            )),
            message,
        });
        if out.len() >= max_items {
            return out;
        }
    }
    for message in json_string_array_field(text, "warnings") {
        out.push(FailedOutputIssue {
            kind: "validation_warning".to_string(),
            category: None,
            file: failed_output_file_from_message(&message),
            line: None,
            suggestion: Some(
                "Review the warning; do not suppress it unless it is an accepted baseline issue."
                    .to_string(),
            ),
            message,
        });
        if out.len() >= max_items {
            return out;
        }
    }
    out
}

fn failed_output_validate_repair_context_command(context: &FailedOutputContext) -> String {
    let mod_arg = context
        .mod_root
        .as_deref()
        .map(command_path_arg)
        .unwrap_or_else(|| "<mod-root>".to_string());
    let game_arg = context
        .game_root
        .as_deref()
        .map(command_path_arg)
        .unwrap_or_else(|| "<HOI4 root>".to_string());
    let dependency_args = dependency_command_args_from_roots(&context.dependency_roots);
    let changed_args = failed_output_changed_command_args(&context.changed_files);
    format!(
        "hoi4skill validate-repair-context {mod_arg} --game-root {game_arg}{dependency_args}{changed_args} --strict-code-index --output .hoi4skill/ai_repair_context.json"
    )
}

fn failed_output_changed_command_args(changed_files: &[String]) -> String {
    if changed_files.is_empty() {
        return String::new();
    }
    let mut out = " --changed-only".to_string();
    for path in changed_files {
        out.push_str(" --changed ");
        out.push_str(&command_path_arg(Path::new(path)));
    }
    out
}

fn error_log_report_failed_output_issues(text: &str, max_items: usize) -> Vec<FailedOutputIssue> {
    let raws = json_string_values_for_field(text, "raw");
    let messages = json_string_values_for_field(text, "message");
    let categories = json_string_values_for_field(text, "category");
    let resolved_files = json_string_values_for_field(text, "resolved_file");
    let files = json_string_values_for_field(text, "file");
    let lines = json_i64_values_for_field(text, "line");
    let suggestions = json_string_values_for_field(text, "suggestion");
    raws.into_iter()
        .enumerate()
        .take(max_items)
        .map(|(idx, raw)| FailedOutputIssue {
            kind: "error_log_diagnostic".to_string(),
            category: categories.get(idx).cloned(),
            file: resolved_files
                .get(idx)
                .filter(|value| !value.is_empty())
                .or_else(|| files.get(idx).filter(|value| !value.is_empty()))
                .cloned(),
            line: lines.get(idx).copied().flatten(),
            suggestion: suggestions.get(idx).cloned(),
            message: messages.get(idx).cloned().unwrap_or(raw),
        })
        .collect()
}

fn logic_audit_report_failed_output_issues(text: &str, max_items: usize) -> Vec<FailedOutputIssue> {
    let details = json_string_values_for_field(text, "detail");
    let kinds = json_string_values_for_field(text, "kind");
    let ids = json_string_values_for_field(text, "id");
    if details.is_empty()
        && !text.contains("\"issue_count\": 0")
        && text.contains("\"schema\": \"hoi4skill.logic_audit.v1\"")
    {
        return vec![FailedOutputIssue {
            kind: "logic_audit_issue".to_string(),
            category: Some("logic_audit".to_string()),
            file: None,
            line: None,
            suggestion: Some(
                "Inspect the logic audit report, repair only changed files, then rerun logic-audit and strict validation."
                    .to_string(),
            ),
            message: truncate_chars(text, 12_000),
        }];
    }
    details
        .into_iter()
        .enumerate()
        .take(max_items)
        .map(|(idx, detail)| {
            let kind = kinds
                .get(idx)
                .filter(|value| !value.is_empty())
                .cloned()
                .unwrap_or_else(|| "logic_audit_issue".to_string());
            let id = ids.get(idx).filter(|value| !value.is_empty());
            let suggestion = id.map_or_else(
                || {
                    "Repair the gameplay logic issue, rerun logic-audit, then rerun strict validation."
                        .to_string()
                },
                |id| {
                    format!(
                        "Repair logic around `{id}`, rerun logic-audit, then rerun strict validation."
                    )
                },
            );
            FailedOutputIssue {
                kind,
                category: Some("logic_audit".to_string()),
                file: None,
                line: None,
                suggestion: Some(suggestion),
                message: detail,
            }
        })
        .collect()
}

fn loc_audit_report_failed_output_issues(text: &str, max_items: usize) -> Vec<FailedOutputIssue> {
    let mut out = Vec::new();
    push_loc_audit_report_issue(
        &mut out,
        text,
        "missing",
        "missing_count",
        "missing_localisation",
        "Add the missing localisation keys or remove only the invalid references after confirming they are not user-provided text.",
        max_items,
    );
    push_loc_audit_report_issue(
        &mut out,
        text,
        "orphan",
        "orphan_count",
        "orphan_localisation",
        "Review orphan localisation keys; keep user-provided prose unless the key is genuinely unused.",
        max_items,
    );
    push_loc_audit_report_issue(
        &mut out,
        text,
        "duplicate",
        "duplicate_count",
        "duplicate_localisation",
        "Merge or rename duplicate localisation keys without changing player-visible text.",
        max_items,
    );
    push_loc_audit_report_issue(
        &mut out,
        text,
        "token_issues",
        "token_issue_count",
        "localisation_token_issue",
        "Fix HOI4 localisation colour/control tokens; ask for missing icon or scripted-localisation mappings instead of inventing them.",
        max_items,
    );
    if out.is_empty() && !loc_audit_report_is_clean(text) {
        out.push(FailedOutputIssue {
            kind: "loc_audit_issue".to_string(),
            category: Some("loc_audit".to_string()),
            file: None,
            line: None,
            suggestion: Some(
                "Inspect the localisation audit report, repair only changed files, then rerun loc-audit and strict validation."
                    .to_string(),
            ),
            message: truncate_chars(text, 12_000),
        });
    }
    out.truncate(max_items);
    out
}

fn push_loc_audit_report_issue(
    out: &mut Vec<FailedOutputIssue>,
    text: &str,
    field: &str,
    count_field: &str,
    kind: &str,
    suggestion: &str,
    max_items: usize,
) {
    if out.len() >= max_items || json_i64_field_is_zero(text, count_field) {
        return;
    }
    let message = json_array_field_raw(text, field)
        .map(|raw| format!("`{field}` entries:\n{raw}"))
        .unwrap_or_else(|| format!("loc-audit reported nonzero `{count_field}`"));
    out.push(FailedOutputIssue {
        kind: kind.to_string(),
        category: Some("loc_audit".to_string()),
        file: None,
        line: None,
        suggestion: Some(suggestion.to_string()),
        message,
    });
}

fn gfx_audit_report_failed_output_issues(text: &str, max_items: usize) -> Vec<FailedOutputIssue> {
    let mut out = Vec::new();
    push_gfx_audit_report_issue(
        &mut out,
        text,
        "missing_textures",
        "missing_textures_count",
        "missing_gfx_texture",
        "Fix the spriteType texturefile path or add/register the referenced texture asset.",
        max_items,
    );
    push_gfx_audit_report_issue(
        &mut out,
        text,
        "missing_sprites",
        "missing_sprites_count",
        "missing_gfx_sprite",
        "Register a verified spriteType for the referenced GFX key or change the reference to an existing indexed sprite.",
        max_items,
    );
    push_gfx_audit_report_issue(
        &mut out,
        text,
        "orphan_sprites",
        "orphan_sprites_count",
        "orphan_gfx_sprite",
        "Review orphan spriteType registrations; keep them if intentionally reserved, otherwise remove or wire the intended reference.",
        max_items,
    );
    push_gfx_audit_report_issue(
        &mut out,
        text,
        "unregistered_images",
        "unregistered_images_count",
        "unregistered_gfx_image",
        "Register image files through spriteType entries before referencing them from focuses, ideas, events, or decisions.",
        max_items,
    );
    if out.is_empty() && !gfx_audit_report_is_clean(text) {
        out.push(FailedOutputIssue {
            kind: "gfx_audit_issue".to_string(),
            category: Some("gfx_audit".to_string()),
            file: None,
            line: None,
            suggestion: Some(
                "Inspect the GFX audit report, repair only changed files, then rerun gfx-audit and strict validation."
                    .to_string(),
            ),
            message: truncate_chars(text, 12_000),
        });
    }
    out.truncate(max_items);
    out
}

fn push_gfx_audit_report_issue(
    out: &mut Vec<FailedOutputIssue>,
    text: &str,
    field: &str,
    count_field: &str,
    kind: &str,
    suggestion: &str,
    max_items: usize,
) {
    if out.len() >= max_items || json_i64_field_is_zero(text, count_field) {
        return;
    }
    let message = json_array_field_raw(text, field)
        .map(|raw| format!("`{field}` entries:\n{raw}"))
        .unwrap_or_else(|| format!("gfx-audit reported nonzero `{count_field}`"));
    out.push(FailedOutputIssue {
        kind: kind.to_string(),
        category: Some("gfx_audit".to_string()),
        file: None,
        line: None,
        suggestion: Some(suggestion.to_string()),
        message,
    });
}

fn failed_output_file_from_message(message: &str) -> Option<String> {
    let (head, _) = message.split_once(": ")?;
    if looks_like_windows_path_prefix(message) || message.starts_with('/') {
        Some(head.to_string())
    } else {
        None
    }
}

fn optional_repair_prompt_input(
    map: &ArgMap,
    keys: &[&str],
) -> Result<Option<(PathBuf, String)>, String> {
    for key in keys {
        if let Some(path) = value(map, key) {
            let path = normalize_path(path)?;
            let text = read_utf8_lossy(&path)?;
            return Ok(Some((path, text)));
        }
    }
    Ok(None)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_ai_repair_prompt(
    request: Option<&str>,
    edit_context_path: &Path,
    edit_context: &str,
    repair_context: Option<&(PathBuf, String)>,
    failed_patch: Option<&(PathBuf, String)>,
    max_context_chars: usize,
    max_repair_chars: usize,
    max_patch_chars: usize,
) -> String {
    let mut out = String::new();
    out.push_str("# HOI4 AI Repair Prompt\n\n");
    out.push_str("- schema: `hoi4skill.ai_repair_prompt.v1`\n");
    out.push_str("- purpose: repair failed generated HOI4 mod code without leaving the verified edit boundary\n");
    out.push_str(&format!(
        "- edit_context: `{}`\n",
        edit_context_path.display()
    ));
    if let Some((path, _)) = repair_context {
        out.push_str(&format!(
            "- validation_repair_context: `{}`\n",
            path.display()
        ));
    } else {
        out.push_str("- validation_repair_context: not supplied; use the embedded `AI Repair Insurance` block first\n");
    }
    if let Some((path, _)) = failed_patch {
        out.push_str(&format!("- failed_patch_or_output: `{}`\n", path.display()));
    } else {
        out.push_str("- failed_patch_or_output: not supplied\n");
    }

    out.push_str("\n## Task\n\n");
    out.push_str("Repair the failed generated output using only verified facts in this prompt. Do not redesign the feature, widen scope, or create unrelated HOI4 systems.\n\n");
    if let Some(request) = request {
        out.push_str("Literal user request:\n\n");
        out.push_str(&markdown_fence("text", request));
    }

    out.push_str("\n## Mandatory Repair Loop\n\n");
    out.push_str("1. Read `Write Gate`, `Requirement Scope Contract`, and `AI Repair Insurance` before proposing changes.\n");
    out.push_str("2. Fix only listed safety blockers, validation errors, validation warnings, or failed-patch lines that are inside the allowed edit surface.\n");
    out.push_str("3. For natural-language effects, first produce `hoi4skill_intent` or run `compile-intent`; do not handwrite fallback Clausewitz.\n");
    out.push_str("4. For unknown effects, triggers, modifiers, sprites, tags, pictures, technologies, or localisation tokens, use `related_indexed_code`, `check-code-symbol`, `code-catalog`, or ask the user.\n");
    out.push_str("5. Preserve user-provided text and event-chain links; never hide an error by deleting prose, dropping options, removing follow-up events, or replacing dynamic modifiers with national spirits.\n");
    out.push_str("6. If `Failed Patch Or Output` lists `changed_files`, your `changed_files` and `patch_plan` must stay within that list unless the user explicitly expands scope.\n");
    out.push_str("7. End with the exact validation commands that must pass, including `validate --strict-code-index` and any text-alignment command required by the edit context.\n");

    out.push_str("\n## Required Model Output Format\n\n");
    out.push_str("- `repair_summary`: one short paragraph describing the smallest fix.\n");
    out.push_str(
        "- `changed_files`: exact files to edit; every file must be allowed by the Write Gate.\n",
    );
    out.push_str("- `patch_plan`: structured steps or corrected cards/intents for the Rust writer; no unverified raw Clausewitz syntax.\n");
    out.push_str("- `questions`: required user questions for any missing mapping; use an empty list only when none are needed.\n");
    out.push_str("- `validation_commands`: commands to run after applying the repair.\n");

    if let Some((path, text)) = failed_patch {
        out.push_str("\n## Failed Patch Or Output\n\n");
        out.push_str(&format!("- source: `{}`\n\n", path.display()));
        out.push_str(&markdown_fence(
            "text",
            truncate_chars(text, max_patch_chars).as_str(),
        ));
    }

    if let Some((path, text)) = repair_context {
        out.push_str("\n## Validation Repair Context\n\n");
        out.push_str(&format!("- source: `{}`\n\n", path.display()));
        out.push_str(&markdown_fence(
            "json",
            truncate_chars(text, max_repair_chars).as_str(),
        ));
    }

    out.push_str("\n## Edit Context Pack\n\n");
    out.push_str(&markdown_fence(
        "markdown",
        truncate_chars(edit_context, max_context_chars).as_str(),
    ));
    out
}

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
pub(crate) fn prepare_edit_context_markdown(
    input: &Path,
    mod_input: &Path,
    tag: &str,
    prefix: &str,
    sheet: Option<&str>,
    tree_id: Option<&str>,
    explicit_request: Option<&str>,
    dependency_roots: &[PathBuf],
    game_root: Option<&Path>,
    game_index: Option<&GameIndex>,
    max_items: usize,
    max_sprites: usize,
    max_context_files: usize,
    code_libraries: Option<&[PathBuf]>,
) -> Result<String, String> {
    let workflow_input = workflow_input_from_path(input, sheet, tag, prefix)?;
    prepare_edit_context_markdown_from_workflow_input(
        &input.display().to_string(),
        workflow_input,
        mod_input,
        tag,
        prefix,
        sheet,
        tree_id,
        explicit_request,
        dependency_roots,
        game_root,
        game_index,
        max_items,
        max_sprites,
        max_context_files,
        code_libraries,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_edit_context_markdown_from_workflow_input(
    input_label: &str,
    mut workflow_input: WorkflowInput,
    mod_input: &Path,
    tag: &str,
    prefix: &str,
    sheet: Option<&str>,
    tree_id: Option<&str>,
    explicit_request: Option<&str>,
    dependency_roots: &[PathBuf],
    game_root: Option<&Path>,
    game_index: Option<&GameIndex>,
    max_items: usize,
    max_sprites: usize,
    max_context_files: usize,
    code_libraries: Option<&[PathBuf]>,
    allow_existing_target_validation_suppression: bool,
) -> Result<String, String> {
    let resolved = resolve_mod_root(mod_input)?;
    append_explicit_request(&mut workflow_input, explicit_request);
    let request_text = &workflow_input.text;
    let knowledge_json = mod_knowledge_json(&resolved, max_items, max_sprites, dependency_roots)?;
    let context_validation_options = ValidationOptions {
        strict_code_index: game_index.is_some(),
    };
    let workflow_json = run_workflow_json_with_focus_layout_options(
        request_text,
        workflow_input.focus_layout.as_ref(),
        Some(&resolved.root),
        tag,
        prefix,
        tree_id,
        true,
        game_index,
        context_validation_options,
    )?;
    let markdown_summary = json_string_field(&knowledge_json, "markdown_summary")
        .unwrap_or_else(|| "mod_knowledge markdown_summary was not found".to_string());
    let anti_hallucination_rules =
        json_string_array_field(&knowledge_json, "anti_hallucination_rules");
    let unknown_facts =
        edit_context_unknown_facts(request_text, &knowledge_json, dependency_roots, game_index);
    let blocked = edit_context_blocked_until_verified(
        &unknown_facts,
        &workflow_json,
        allow_existing_target_validation_suppression,
        game_index.is_some(),
    );
    let repair_insurance_json = edit_context_repair_insurance_json(
        &workflow_json,
        &resolved.root,
        game_root,
        dependency_roots,
        game_index,
        16,
    );
    let write_gate = edit_context_write_gate(
        request_text,
        workflow_input.focus_layout.is_some(),
        &knowledge_json,
        dependency_roots,
        game_index,
        &unknown_facts,
        &workflow_json,
        allow_existing_target_validation_suppression,
    );
    let scope_contract = requirement_scope_contract(
        request_text,
        workflow_input.focus_layout.is_some(),
        tag,
        prefix,
    );
    let excerpts = edit_context_file_excerpts(&resolved.root, max_context_files)?;

    let mut out = String::new();
    out.push_str("# HOI4 Edit Context Pack\n\n");
    out.push_str("Use this as the first context block before generating or editing files. ");
    out.push_str("Do not write code from memory when a fact is missing here.\n\n");
    out.push_str("## Request\n\n");
    out.push_str(&format!("- input: `{input_label}`\n"));
    out.push_str(&format!("- mod_root: `{}`\n", resolved.root.display()));
    out.push_str(&format!("- tag: `{tag}`\n- prefix: `{prefix}`\n"));
    if let Some(sheet) = sheet {
        out.push_str(&format!("- sheet: `{sheet}`\n"));
    }
    if let Some(tree_id) = tree_id {
        out.push_str(&format!("- tree_id: `{tree_id}`\n"));
    }
    out.push_str(&format!(
        "- dry_run_validation: `{}`\n",
        if context_validation_options.strict_code_index {
            "strict-code-index"
        } else {
            "local-static-only"
        }
    ));
    if dependency_roots.is_empty() {
        out.push_str("- dependency_mod_roots: none supplied\n");
    } else {
        out.push_str("- dependency_mod_roots:\n");
        for root in dependency_roots {
            out.push_str(&format!("  - `{}`\n", root.display()));
        }
    }
    if let Some(libraries) = code_libraries {
        out.push_str("- clausewitz_code_layers (highest priority first):\n");
        for (index, library) in libraries.iter().enumerate() {
            let kind = if index + 1 == libraries.len() {
                "vanilla_base"
            } else {
                "user_authorized_mod"
            };
            out.push_str(&format!("  - {kind}: `{}`\n", library.display()));
        }
    }
    out.push('\n');
    out.push_str(&markdown_fence(
        "text",
        truncate_chars(request_text, 18_000).as_str(),
    ));

    out.push_str("\n## Requirement Scope Contract\n\n");
    out.push_str("- rule: this section is the complete file-creation boundary; a new mod does not authorize unrelated systems.\n");
    out.push_str(&format!(
        "- authorized_systems: {}\n",
        list_or_none(&scope_contract.authorized_systems, 50)
    ));
    out.push_str(&format!(
        "- minimum_events: {}\n",
        scope_contract
            .minimum_events
            .map(|value| value.to_string())
            .unwrap_or_else(|| "not requested".to_string())
    ));
    out.push_str(&format!(
        "- minimum_national_spirits: {}\n",
        scope_contract
            .minimum_ideas
            .map(|value| value.to_string())
            .unwrap_or_else(|| "not requested".to_string())
    ));
    out.push_str("\n### Planned Files\n\n");
    push_markdown_list(&mut out, &scope_contract.planned_files);
    out.push_str("\n### Forbidden Without Explicit Request\n\n");
    push_markdown_list(&mut out, &scope_contract.forbidden_without_explicit_request);
    out.push_str("\n### Scope Rules\n\n");
    push_markdown_list(&mut out, &scope_contract.rules);

    out.push_str("\n## AI Authoring Contract\n\n");
    out.push_str("- Treat player-facing prose as intent, not as Clausewitz code.\n");
    out.push_str("- Convert shorthand such as `llm：战争正当化 = -10%` with `hoi4skill compile-intent --kind auto --game-root <HOI4 root> --strict-code-index` before any final script output.\n");
    out.push_str("- For synonym-heavy Chinese or English user text, first normalize with the model into `hoi4skill_intent` fields (`type: add_national_spirit|replace_national_spirit`, `idea_name`, `old_idea`, `new_idea`, `effects`), then pass that structure to `compile-intent`; do not expand the Rust synonym list for every wording variant.\n");
    out.push_str("- Use only effects, triggers, modifiers, buildings, resources, sprites, technologies, tags, and IDs that appear in this context pack, local excerpts, or `check-code-symbol`/`code-catalog` results.\n");
    out.push_str("- For variable-driven dynamic modifiers, prefer the reusable scripted-effect protocol: `custom_effect_tooltip = <dynamic_modifier>_tt`, `set_temp_variable = { temp_<dynamic_modifier> = <number> }`, then call verified `change_<dynamic_modifier><slot> = yes` scripted effects. Do not invent `change_*` helpers or dynamic modifier variables; verify them through `code-catalog`, `check-code-symbol --kind effect`, and `check-code-symbol --kind dynamic_modifier_variable`.\n");
    out.push_str("- If `compile-intent`, `check-code-symbol`, dry-run safety, or validation says `ok: false` / `final_code_allowed: false`, stop and fix the structured input; do not handwrite fallback Clausewitz.\n");
    out.push_str("- Final generated files must pass `hoi4skill validate <mod-root> --game-root <HOI4 root> --strict-code-index` before being treated as usable.\n");

    out.push_str("\n## AI Context Contract\n\n");
    out.push_str("Machine-readable write contract for weak models and automation. If this block says final code is not allowed, the next model response must ask/verify/repair instead of writing Clausewitz.\n\n");
    out.push_str(&markdown_fence(
        "json",
        &edit_context_ai_context_contract_json(
            &resolved.root,
            tag,
            prefix,
            context_validation_options.strict_code_index,
            &write_gate,
            &unknown_facts,
            &blocked,
        ),
    ));

    out.push_str("\n## Write Gate\n\n");
    out.push_str(&format!("- status: `{}`\n", write_gate.status));
    out.push_str("- rule: if the status is not `READY_FOR_NARROW_WRITE`, resolve the missing evidence before writing final game script.\n");
    out.push_str("- rule: write only inside the allowed edit surface and only for IDs/paths shown in the dry-run plan or verified local files.\n\n");
    out.push_str("### Verified Evidence\n\n");
    push_markdown_list(&mut out, &write_gate.verified_evidence);
    out.push_str("\n### Allowed Edit Surface\n\n");
    push_markdown_list(&mut out, &write_gate.allowed_edit_surface);
    out.push_str("\n### Missing Evidence To Resolve\n\n");
    push_markdown_list(&mut out, &write_gate.missing_evidence);
    out.push_str("\n### Verification Steps\n\n");
    push_markdown_list(&mut out, &write_gate.verification_steps);
    out.push_str("\n### Stop Conditions\n\n");
    push_markdown_list(&mut out, &write_gate.stop_conditions);

    out.push_str("\n## AI Repair Insurance\n\n");
    out.push_str("This block is for weak-model retries: if generated code fails, feed this JSON back with the failed patch and ask the model to repair only the listed items. Related code candidates come from the local strict code index; empty candidates mean the model must ask or run a lookup command instead of inventing syntax.\n\n");
    out.push_str(&markdown_fence(
        "json",
        truncate_chars(&repair_insurance_json, 40_000).as_str(),
    ));

    out.push_str("\n## Knowledge Summary\n\n");
    out.push_str(&markdown_summary);
    if !markdown_summary.ends_with('\n') {
        out.push('\n');
    }

    if let Some(index) = game_index {
        out.push_str("\n## Indexed Game/Dependency Resources\n\n");
        out.push_str(&render_indexed_resource_summary(index, 40));
        out.push_str("\n## Clausewitz Syntax Reference Table\n\n");
        out.push_str(&render_clausewitz_reference_table(Some(index)));
    }

    if let Some(libraries) = code_libraries {
        out.push_str("\n## Retrieved Clausewitz Code Library\n\n");
        out.push_str("- rule: read these verified local examples before producing structured inputs or changing a generator.\n");
        out.push_str("- rule: copy syntax and block ownership only; never copy IDs, country-specific narrative, or unrelated effects.\n");
        out.push_str(&render_retrieved_clausewitz_context(
            libraries,
            request_text,
            &scope_contract.authorized_systems,
        )?);
    }

    out.push_str("\n## Dry Run Plan\n\n");
    out.push_str("This is a non-writing `run-workflow` plan with validation against the target mod root. When a game/dependency index is available, this dry run uses strict code-index validation so the model sees final-gate failures before writing.\n\n");
    let workflow_json_for_context = edit_context_workflow_json_for_markdown(
        &workflow_json,
        allow_existing_target_validation_suppression,
        game_index.is_some(),
    );
    out.push_str(&markdown_fence(
        "json",
        truncate_chars(&workflow_json_for_context, 60_000).as_str(),
    ));

    out.push_str("\n## Anti-Hallucination Rules\n\n");
    if anti_hallucination_rules.is_empty() {
        out.push_str("- Use only facts from the knowledge summary, local excerpts, explicit user input, or an indexed game/dependency root.\n");
        out.push_str("- Missing facts are unknown; verify them before editing.\n");
    } else {
        for rule in anti_hallucination_rules {
            out.push_str(&format!("- {rule}\n"));
        }
    }

    out.push_str("\n## Unknown Facts\n\n");
    for fact in &unknown_facts {
        out.push_str(&format!("- {fact}\n"));
    }

    out.push_str("\n## Blocked Until Verified\n\n");
    for item in &blocked {
        out.push_str(&format!("- {item}\n"));
    }

    out.push_str("\n## Local File Excerpts\n\n");
    if excerpts.is_empty() {
        out.push_str("- No local content excerpts were selected.\n");
    } else {
        for excerpt in excerpts {
            out.push_str(&format!("### `{}`\n\n", excerpt.path));
            out.push_str(&markdown_fence(
                "text",
                truncate_chars(&excerpt.text, 12_000).as_str(),
            ));
            out.push('\n');
        }
    }

    out.push_str("\n## Safe Next Step\n\n");
    out.push_str("- If every `Blocked Until Verified` item is resolved, rerun `run-workflow` without `--dry-run` or use the narrow `apply-*` command.\n");
    out.push_str("- If any blocked item remains, read/index the missing files first or ask the user for explicit IDs/roots.\n");
    out.push_str("- After writes, run `hoi4skill validate <mod-root> --game-root <HOI4 root> --strict-code-index --request \"<literal user request>\"` and then check HOI4 `error.log` from an in-game launch.\n");
    Ok(out)
}

pub(crate) fn edit_context_ai_context_contract_json(
    mod_root: &Path,
    tag: &str,
    prefix: &str,
    strict_code_index: bool,
    write_gate: &EditContextWriteGate,
    unknown_facts: &[String],
    blocked_until_verified: &[String],
) -> String {
    let final_code_allowed = write_gate.status == "READY_FOR_NARROW_WRITE";
    let required_next_action = if final_code_allowed {
        "write_only_inside_allowed_edit_surface_then_validate"
    } else if write_gate.status == "VERIFY_FIRST" {
        "verify_missing_evidence_before_writing"
    } else {
        "ask_or_repair_before_writing"
    };
    let ai_rules = vec![
        "Use this JSON block as the authority for whether final code may be written.".to_string(),
        "If final_code_allowed_by_context is false, ask the user or run the listed verification steps instead of writing Clausewitz.".to_string(),
        "Write only inside allowed_edit_surface and preserve all unrelated files.".to_string(),
        "Treat every unknown_fact and blocked_until_verified item as unresolved until current local evidence proves it.".to_string(),
    ];
    format!(
        "{{\n  \"schema\": \"hoi4skill.ai_context_contract.v1\",\n  \"mod_root\": {},\n  \"tag\": {},\n  \"prefix\": {},\n  \"write_gate_status\": {},\n  \"strict_code_index\": {},\n  \"final_code_allowed_by_context\": {},\n  \"required_next_action\": {},\n  \"allowed_edit_surface\": {},\n  \"missing_evidence\": {},\n  \"verification_steps\": {},\n  \"unknown_facts\": {},\n  \"blocked_until_verified\": {},\n  \"ai_rules\": {}\n}}\n",
        json_str(&mod_root.display().to_string()),
        json_str(tag),
        json_str(prefix),
        json_str(write_gate.status),
        json_bool(strict_code_index),
        json_bool(final_code_allowed),
        json_str(required_next_action),
        json_array(&write_gate.allowed_edit_surface),
        json_array(&write_gate.missing_evidence),
        json_array(&write_gate.verification_steps),
        json_array(unknown_facts),
        json_array(blocked_until_verified),
        json_array(&ai_rules),
    )
}

pub(crate) fn render_indexed_resource_summary(index: &GameIndex, limit: usize) -> String {
    let mut out = String::new();
    out.push_str("- rule: only use these indexed resources or local `interface/*.gfx` evidence; missing resources are unknown.\n");
    out.push_str(&format!(
        "- country_tags: {} total; sample: {}\n",
        index.country_tags.len(),
        sample_btree_strings(&index.country_tags, limit)
    ));
    out.push_str(&format!(
        "- ideologies: {} total; sample: {}\n",
        index.ideologies.len(),
        sample_btree_strings(&index.ideologies, limit)
    ));
    out.push_str(&format!(
        "- focus_goal_sprites: {} total; sample: {}\n",
        index.focus_goal_sprites.len(),
        sample_btree_strings(&index.focus_goal_sprites, limit)
    ));
    out.push_str(&format!(
        "- idea_pictures: {} total; sample: {}\n",
        index.idea_pictures.len(),
        sample_btree_strings(&index.idea_pictures, limit)
    ));
    out.push_str(&format!(
        "- event_pictures: {} total; sample: {}\n",
        index.event_pictures.len(),
        sample_btree_strings(&index.event_pictures, limit)
    ));
    out.push_str(&format!(
        "- decision_icons: {} total; sample: {}\n",
        index.decision_icons.len(),
        sample_btree_strings(&index.decision_icons, limit)
    ));
    out.push_str(&format!(
        "- decision_category_pictures: {} total; sample: {}\n",
        index.decision_category_pictures.len(),
        sample_btree_strings(&index.decision_category_pictures, limit)
    ));
    out.push_str(&format!(
        "- leader_portraits: {} total; sample: {}\n",
        index.leader_portraits.len(),
        sample_btree_strings(&index.leader_portraits, limit)
    ));
    out.push_str(&format!(
        "- effects: {} total; sample: {}\n",
        index.effects.len(),
        sample_btree_strings(&index.effects, limit)
    ));
    out.push_str(&format!(
        "- triggers: {} total; sample: {}\n",
        index.triggers.len(),
        sample_btree_strings(&index.triggers, limit)
    ));
    out.push_str(&format!(
        "- modifiers: {} total; sample: {}\n",
        index.modifiers.len(),
        sample_btree_strings(&index.modifiers, limit)
    ));
    out
}

fn sample_btree_strings(values: &BTreeSet<String>, limit: usize) -> String {
    if values.is_empty() {
        return "none".to_string();
    }
    values
        .iter()
        .take(limit)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Clone)]
pub(crate) struct EditContextExcerpt {
    pub(crate) path: String,
    pub(crate) text: String,
}

pub(crate) struct EditContextWriteGate {
    pub(crate) status: &'static str,
    pub(crate) verified_evidence: Vec<String>,
    pub(crate) allowed_edit_surface: Vec<String>,
    pub(crate) missing_evidence: Vec<String>,
    pub(crate) verification_steps: Vec<String>,
    pub(crate) stop_conditions: Vec<String>,
}

pub(crate) fn edit_context_file_excerpts(
    root: &Path,
    max_context_files: usize,
) -> Result<Vec<EditContextExcerpt>, String> {
    let files = collect_files(root)?;
    let sample = sample_content_files(root, &files, max_context_files);
    let mut out = Vec::new();
    for rel in sample {
        let path = root.join(rel.replace('/', "\\"));
        if !path.exists() || !path.is_file() {
            continue;
        }
        let Ok(text) = read_utf8_lossy(&path) else {
            continue;
        };
        out.push(EditContextExcerpt { path: rel, text });
    }
    Ok(out)
}

pub(crate) fn edit_context_write_gate(
    request_text: &str,
    supplied_focus_layout: bool,
    knowledge_json: &str,
    dependency_roots: &[PathBuf],
    game_index: Option<&GameIndex>,
    unknown_facts: &[String],
    workflow_json: &str,
    allow_existing_target_validation_suppression: bool,
) -> EditContextWriteGate {
    let focus_text = extract_focus_layout_text(request_text);
    let feature_text = extract_card_text(request_text, FEATURE_CARD_HEADERS);
    let event_text = extract_card_text(request_text, &["事件"]);
    let feature_cards = parse_cards(&feature_text, FEATURE_CARD_HEADERS);
    let event_cards = parse_cards(&event_text, &["事件"]);
    let dynamic_modifier_intents = workflow_dynamic_modifier_intents(request_text);
    let localisation_entries = workflow_localisation_entries(request_text);
    let localisation_token_issues = workflow_localisation_token_issues(request_text);
    let has_focus_layout = supplied_focus_layout || !focus_text.trim().is_empty();
    let scope_contract =
        requirement_scope_contract(request_text, has_focus_layout, "TAG", "feature");
    let scope_wants_ideas = scope_contract
        .authorized_systems
        .iter()
        .any(|system| system == "national_spirits");
    let scope_wants_events = scope_contract
        .authorized_systems
        .iter()
        .any(|system| system == "events");
    let detected_sections = usize::from(has_focus_layout)
        + usize::from(scope_wants_ideas || !feature_cards.is_empty())
        + usize::from(scope_wants_events || !event_cards.is_empty())
        + usize::from(!dynamic_modifier_intents.is_empty())
        + usize::from(!localisation_entries.is_empty());
    let is_submod = knowledge_json.contains("\"kind\": \"submod\"");
    let unknown_descriptor = knowledge_json.contains("\"kind\": \"unknown_no_descriptor\"");

    let mut verified_evidence = Vec::new();
    if unknown_descriptor {
        verified_evidence
            .push("mod root is not confirmed; descriptor.mod was not observed".to_string());
    } else {
        verified_evidence.push(
            "mod root was resolved and mod-knowledge generated descriptor/local-file evidence"
                .to_string(),
        );
    }
    if is_submod {
        if dependency_roots.is_empty() {
            verified_evidence
                .push("target is a submod, but no dependency roots were indexed".to_string());
        } else {
            verified_evidence.push(format!(
                "target is a submod and {} dependency root(s) were supplied",
                dependency_roots.len()
            ));
        }
    } else {
        verified_evidence
            .push("target is not classified as a dependency-backed submod".to_string());
    }
    if game_index.is_some() {
        verified_evidence.push(
            "game/dependency index is available for tags, sprites, leader portraits, states, provinces, technologies, and symbols"
                .to_string(),
        );
    } else {
        verified_evidence.push(
            "no game/dependency index is available; only local mod facts are verified".to_string(),
        );
    }
    verified_evidence.push(format!(
        "request parsed as focus_layout={}, feature_cards={}, event_cards={}",
        has_focus_layout,
        feature_cards.len(),
        event_cards.len()
    ));
    verified_evidence.push(format!(
        "request auxiliary inputs parsed as dynamic_modifier_intents={}, localisation_entries={}, localisation_token_issues={}",
        dynamic_modifier_intents.len(),
        localisation_entries.len(),
        localisation_token_issues.len()
    ));
    verified_evidence.push(format!(
        "dry-run validation status is {}",
        workflow_validation_status(workflow_json)
    ));
    verified_evidence.push(format!(
        "dry-run safety status is {}",
        workflow_safety_status(workflow_json)
    ));
    let pre_existing_validation_issues = has_suppressible_existing_target_validation_issues(
        workflow_json,
        allow_existing_target_validation_suppression,
        game_index.is_some(),
    );
    if pre_existing_validation_issues {
        verified_evidence.push(
            "target mod has pre-existing validation issues, but planned edit safety allows final code; require changed-only strict validation after writing"
                .to_string(),
        );
    }

    let mut allowed_edit_surface = Vec::new();
    if has_focus_layout {
        allowed_edit_surface.push(
            "common/national_focus and localisation/simp_chinese for the target focus tree only"
                .to_string(),
        );
    }
    for card in &feature_cards {
        match feature_card_type(&card.kind).unwrap_or("") {
            "decision" => allowed_edit_surface.push(
                "common/decisions, common/decisions/categories, and localisation for parsed decision cards"
                    .to_string(),
            ),
            "idea" => allowed_edit_surface
                .push("common/ideas and localisation for parsed national-spirit cards".to_string()),
            "technology" => allowed_edit_surface.push(
                "common/technologies and localisation for parsed technology cards, after indexed reference checks"
                    .to_string(),
            ),
            "gui" => allowed_edit_surface.push(
                "common/scripted_guis, interface/*.gui, and localisation for conservative GUI skeletons"
                    .to_string(),
            ),
            "scripted_effect" => allowed_edit_surface
                .push("common/scripted_effects for parsed scripted-effect helper cards".to_string()),
            "scripted_trigger" => allowed_edit_surface.push(
                "common/scripted_triggers for parsed scripted-trigger helper cards".to_string(),
            ),
            "state_effect" => allowed_edit_surface.push(
                "common/scripted_effects state-scope helpers only; no direct history/states writes without plan-history-edit"
                    .to_string(),
            ),
            _ => {}
        }
    }
    if scope_wants_ideas
        && !allowed_edit_surface
            .iter()
            .any(|surface| surface.starts_with("common/ideas"))
    {
        allowed_edit_surface.push(
            "common/ideas and Simplified Chinese localisation for explicitly requested national spirits only"
                .to_string(),
        );
    }
    if scope_wants_events || !event_cards.is_empty() {
        allowed_edit_surface.push(
            "events and Simplified Chinese localisation for explicitly requested events and verified namespaces only"
                .to_string(),
        );
    }
    if !dynamic_modifier_intents.is_empty() {
        allowed_edit_surface.push(
            "dynamic modifier protocol planning only until plan-dynamic-modifier-change or compile-intent verifies indexed helpers"
                .to_string(),
        );
    }
    if !localisation_entries.is_empty() {
        allowed_edit_surface.push(
            "Simplified Chinese localisation entries explicitly supplied by the request"
                .to_string(),
        );
    }
    if allowed_edit_surface.is_empty() {
        allowed_edit_surface.push(
            "no file writes; convert the request into a parseable focus/card/event plan first"
                .to_string(),
        );
    }
    allowed_edit_surface.push(
        "preserve every file and setting outside the dry-run plan and verified local evidence"
            .to_string(),
    );
    allowed_edit_surface.sort();
    allowed_edit_surface.dedup();

    let mut missing_evidence = unknown_facts
        .iter()
        .filter(|fact| !fact.starts_with("no obvious missing high-risk facts"))
        .cloned()
        .collect::<Vec<_>>();
    if workflow_json.contains("\"ok\": false") && !pre_existing_validation_issues {
        missing_evidence
            .push("dry-run validation is not clean; review validation errors/warnings".to_string());
    }
    if workflow_json.contains("\"final_code_allowed\": false") {
        missing_evidence.push(
            "dry-run safety blocks final code; map every raw effect/trigger or placeholder through verified CLI output"
                .to_string(),
        );
    }
    if !dynamic_modifier_intents.is_empty() {
        missing_evidence.push(
            "dynamic modifier intents require `plan-dynamic-modifier-change` or `compile-intent` before final Clausewitz writes"
                .to_string(),
        );
    }
    if !localisation_token_issues.is_empty() {
        missing_evidence.push(
            "localisation token issues require author-placeholder-plan, indexed icon/tag evidence, or corrected HOI4 colour/scripted-localisation syntax before final .yml writes"
                .to_string(),
        );
    }
    if detected_sections == 0 {
        missing_evidence.push(
            "request was not parsed into focus layout, feature cards, or event cards".to_string(),
        );
    }
    if missing_evidence.is_empty() {
        missing_evidence
            .push("none detected by preflight; still treat absent facts as unknown".to_string());
    }
    missing_evidence.sort();
    missing_evidence.dedup();

    let mut verification_steps = edit_context_verification_steps(&missing_evidence);
    if pre_existing_validation_issues {
        verification_steps.push("after writing, run `hoi4skill validate <mod-root> --changed-only --changed <planned-file> --game-root <HOI4 root> --strict-code-index` so old target-mod errors do not mask this edit".to_string());
    }
    if verification_steps.is_empty() {
        verification_steps.push(
            "rerun `run-workflow --dry-run` after any context change and compare the plan before writing"
                .to_string(),
        );
    }

    let mut stop_conditions = vec![
        "stop if a needed tag, state/province ID, technology, modifier, sprite, namespace, file path, or leader syntax is absent from the context pack".to_string(),
        "stop if the dry-run plan does not mention the system you intend to edit".to_string(),
        "stop if dry-run safety blocks the planned edit; pre-existing target-mod validation errors must be handled by changed-only validation after writing".to_string(),
    ];
    if unknown_descriptor {
        stop_conditions
            .push("stop until the real mod root or launcher .mod file is confirmed".to_string());
    }
    if is_submod && dependency_roots.is_empty() {
        stop_conditions.push(
            "stop before using inherited dependency content until --mod-path roots are indexed"
                .to_string(),
        );
    }

    let hard_blocked = unknown_descriptor
        || detected_sections == 0
        || workflow_json.contains("\"final_code_allowed\": false");
    let status = if hard_blocked {
        "BLOCKED"
    } else if missing_evidence
        .iter()
        .any(|fact| !fact.starts_with("none detected by preflight"))
    {
        "VERIFY_FIRST"
    } else {
        "READY_FOR_NARROW_WRITE"
    };

    EditContextWriteGate {
        status,
        verified_evidence,
        allowed_edit_surface,
        missing_evidence,
        verification_steps,
        stop_conditions,
    }
}

pub(crate) fn edit_context_unknown_facts(
    request_text: &str,
    knowledge_json: &str,
    dependency_roots: &[PathBuf],
    game_index: Option<&GameIndex>,
) -> Vec<String> {
    let mut facts = Vec::new();
    let lower = request_text.to_ascii_lowercase();
    let mentions_history = request_mentions_direct_history_edit(request_text);
    let mentions_icons = contains_any(request_text, &["图标", "icon", "gfx", "dds", "png"]);
    let mentions_country_or_leader = contains_any(
        request_text,
        &[
            "创建国家",
            "国家tag",
            "国家TAG",
            "领袖",
            "将领",
            "顾问",
            "country_leader",
            "character",
        ],
    );
    let is_submod = knowledge_json.contains("\"kind\": \"submod\"");
    let unknown_descriptor = knowledge_json.contains("\"kind\": \"unknown_no_descriptor\"");
    let no_dependency_roots = is_submod
        && dependency_roots.is_empty()
        && knowledge_json.contains("\"dependency_mod_roots\": []");

    if unknown_descriptor {
        facts.push(
            "target mod root is not confirmed because descriptor.mod was not found".to_string(),
        );
    }
    if no_dependency_roots {
        facts.push("submod dependencies are named but no dependency --mod-path roots were supplied, so inherited tags/sprites/scripts/technologies/state facts remain unknown".to_string());
    }
    if mentions_history {
        facts.push("history/state/province/capital facts require `plan-history-edit`, indexed game/dependency roots, or explicit user-provided IDs before direct history writes".to_string());
    }
    if mentions_icons && game_index.is_none() {
        facts.push("game/dependency icon index was not built; focus icons, idea pictures, decision icons, decision category pictures, event pictures, and leader portraits may use only locally observed registrations, with `GFX_goal_unknown` as the focus fallback".to_string());
    }
    if mentions_country_or_leader && no_dependency_roots {
        facts.push("country/leader syntax for dependency-provided content is unknown until dependency roots are indexed".to_string());
    }
    if (lower.contains("technology") || request_text.contains("科技")) && game_index.is_none() {
        facts.push("technology, category, equipment, and modifier references are not checked against a game index".to_string());
    }
    if facts.is_empty() {
        facts.push("no obvious missing high-risk facts were detected; still treat facts absent from the knowledge summary as unknown".to_string());
    }
    facts.sort();
    facts.dedup();
    facts
}

fn request_mentions_direct_history_edit(request_text: &str) -> bool {
    let lower = request_text.to_ascii_lowercase();
    if contains_any(
        &lower,
        &[
            "history/states",
            "history/countries",
            "state id",
            "province id",
            "state_id",
            "province_id",
            "capital =",
            "set_capital",
        ],
    ) {
        return true;
    }
    request_text.lines().any(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return false;
        }
        contains_any(
            trimmed,
            &[
                "州ID",
                "省份ID",
                "STATE_",
                "首都ID",
                "迁都",
                "设置首都",
                "州效果",
                "省份效果",
                "胜利点",
                "建筑：",
                "建筑:",
                "资源：",
                "资源:",
            ],
        ) || trimmed.starts_with("首都：")
            || trimmed.starts_with("首都:")
            || trimmed.starts_with("capital:")
    })
}

fn edit_context_verification_steps(missing_evidence: &[String]) -> Vec<String> {
    let mut steps = Vec::new();
    for fact in missing_evidence {
        if fact.contains("history/state/province/capital") {
            steps.push("run `hoi4skill plan-history-edit <mod-root> --text <request> --game-root <hoi4-root> [--mod-path <dependency>]` before direct history writes".to_string());
        } else if fact.contains("submod dependencies") || fact.contains("country/leader syntax") {
            steps.push("rerun `prepare-edit-context` with each dependency launcher/root supplied through `--mod-path`".to_string());
        } else if fact.contains("icon index") {
            steps.push("supply `--game-root` or verify exact focus/idea/decision/event/leader portrait sprite registrations in local/dependency `interface/*.gfx`; ideas register `GFX_idea_*` but idea blocks must omit the `GFX_idea_` prefix".to_string());
        } else if fact.contains("technology") {
            steps.push("supply `--game-root` so technologies, categories, equipment, and modifiers are checked against an index".to_string());
        } else if fact.contains("descriptor.mod") {
            steps.push("rerun against the real mod directory, descriptor.mod, or launcher-side `.mod` file".to_string());
        } else if fact.contains("dry-run validation") {
            steps.push("read the `Dry Run Plan` validation errors/warnings and fix or explicitly accept each warning before writing".to_string());
        } else if fact.contains("pre-existing validation issues") {
            steps.push("after writing, run `hoi4skill validate <mod-root> --changed-only --changed <planned-file> --game-root <HOI4 root> --strict-code-index` so old target-mod errors do not mask this edit".to_string());
        } else if fact.contains("dry-run safety blocks") {
            steps.push("use `hoi4skill compile-intent --kind auto --game-root <HOI4 root> --strict-code-index` or `check-code-symbol` to replace every raw effect/trigger or placeholder with verified structured input".to_string());
        } else if fact.contains("dynamic modifier intents") {
            steps.push("run `hoi4skill plan-dynamic-modifier-change <mod-root> --text <dynamic-modifier-request> --game-root <HOI4 root> --strict-code-index` or `hoi4skill compile-intent --kind auto` before writing dynamic modifier Clausewitz".to_string());
        } else if fact.contains("not parsed") {
            steps.push("rewrite the input as a focus layout, feature card, or event card, then regenerate the context pack".to_string());
        }
    }
    steps.sort();
    steps.dedup();
    steps
}

pub(crate) fn edit_context_blocked_until_verified(
    unknown_facts: &[String],
    workflow_json: &str,
    allow_existing_target_validation_suppression: bool,
    has_game_index: bool,
) -> Vec<String> {
    let mut blocked = Vec::new();
    for fact in unknown_facts {
        if fact.contains("history/state/province/capital") {
            blocked.push("Do not edit `history/states` or `history/countries` directly until `plan-history-edit` says the IDs are known.".to_string());
        } else if fact.contains("submod dependencies") {
            blocked.push("Do not reference inherited dependency tags, sprites, scripted values, technologies, or state/province IDs until dependency roots are indexed.".to_string());
        } else if fact.contains("icon index") {
            blocked.push("Do not invent focus, idea, decision-category, event, or leader portrait sprite names; use verified local/indexed registrations, reference ideas without the `GFX_idea_` prefix, or use `GFX_goal_unknown` for an unresolved focus icon.".to_string());
        } else if fact.contains("technology") {
            blocked.push("Do not use unindexed technology/equipment/category/modifier IDs as confirmed facts.".to_string());
        } else if fact.contains("descriptor.mod") {
            blocked.push(
                "Do not edit until the real mod root or launcher `.mod` file is confirmed."
                    .to_string(),
            );
        }
    }
    if workflow_json.contains("\"validation\": {\"ran\": true, \"ok\": false")
        && !has_suppressible_existing_target_validation_issues(
            workflow_json,
            allow_existing_target_validation_suppression,
            has_game_index,
        )
    {
        blocked.push("Do not report success until dry-run validation errors/warnings are reviewed and resolved.".to_string());
    }
    if workflow_json.contains("\"final_code_allowed\": false") {
        blocked.push("Do not write final Clausewitz until dry-run safety allows final code; unresolved raw effects/triggers and placeholders must be mapped first.".to_string());
    }
    if workflow_json.contains(
        "\"detected\": {\"focus_layout\": false, \"feature_cards\": 0, \"event_cards\": 0}",
    ) {
        blocked.push("Do not write files yet; the request was not parsed into a focus layout, feature card, or event card plan.".to_string());
    }
    if blocked.is_empty() {
        blocked.push("No hard blocker detected; write only the files shown by the plan and preserve unrelated content.".to_string());
    }
    blocked.sort();
    blocked.dedup();
    blocked
}

fn edit_context_workflow_json_for_markdown(
    workflow_json: &str,
    allow_existing_target_validation_suppression: bool,
    has_game_index: bool,
) -> String {
    if has_suppressible_existing_target_validation_issues(
        workflow_json,
        allow_existing_target_validation_suppression,
        has_game_index,
    ) {
        let replacement = format!(
            "{{\"ran\": true, \"ok\": false, \"status\": {}, \"suppressed_existing_target_errors\": true, \"detail\": {}}}",
            json_str(workflow_validation_status(workflow_json)),
            json_str("pre-existing target-mod validation errors omitted from edit context; prove this edit with changed-only strict validation for planned files")
        );
        return replace_json_object_field(workflow_json, "validation", &replacement)
            .unwrap_or_else(|| workflow_json.to_string());
    }
    workflow_json.to_string()
}

fn has_suppressible_existing_target_validation_issues(
    workflow_json: &str,
    allow_existing_target_validation_suppression: bool,
    has_game_index: bool,
) -> bool {
    allow_existing_target_validation_suppression
        && has_game_index
        && workflow_json.contains("\"validation\": {\"ran\": true, \"ok\": false")
        && workflow_json.contains("\"final_code_allowed\": true")
}

fn replace_json_object_field(text: &str, field: &str, replacement: &str) -> Option<String> {
    let marker = format!("\"{field}\"");
    let marker_idx = text.find(&marker)?;
    let after_marker = &text[marker_idx + marker.len()..];
    let colon = after_marker.find(':')?;
    let after_colon_idx = marker_idx + marker.len() + colon + 1;
    let object_offset = text[after_colon_idx..].find('{')?;
    let object_start = after_colon_idx + object_offset;
    let object_end = matching_json_object_end(text, object_start)?;
    let mut out = String::new();
    out.push_str(&text[..object_start]);
    out.push_str(replacement);
    out.push_str(&text[object_end..]);
    Some(out)
}

fn matching_json_object_end(text: &str, object_start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    if bytes.get(object_start) != Some(&b'{') {
        return None;
    }
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (offset, ch) in text[object_start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(object_start + offset + ch.len_utf8());
                }
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn edit_context_repair_insurance_json(
    workflow_json: &str,
    mod_root: &Path,
    game_root: Option<&Path>,
    dependency_roots: &[PathBuf],
    game_index: Option<&GameIndex>,
    max_items: usize,
) -> String {
    let issues = edit_context_repair_insurance_messages(workflow_json, max_items.max(1));
    let rules = vec![
        "Repair only files inside the Write Gate allowed edit surface.".to_string(),
        "Treat every listed blocker, validation error, and token mismatch as blocking until strict validation passes.".to_string(),
        "Use related_indexed_code, compile-intent, check-code-symbol, or validate-repair-context before writing replacement Clausewitz.".to_string(),
        "If related_indexed_code is empty or ambiguous, ask the user for the missing mapping instead of inventing code.".to_string(),
        "Never hide an error by deleting user-provided player-facing text, replacing dynamic modifiers with national spirits, or removing event-chain links.".to_string(),
    ];
    let mod_arg = command_path_arg(mod_root);
    let game_arg = game_root
        .map(command_path_arg)
        .unwrap_or_else(|| "<HOI4 root>".to_string());
    let dependency_args = dependency_command_args_from_roots(dependency_roots);
    let next_commands = vec![
        format!("hoi4skill compile-intent --kind auto --game-root {game_arg}{dependency_args} --strict-code-index --text <natural-language-effect>"),
        format!("hoi4skill check-code-symbol --game-root {game_arg}{dependency_args} --kind <effect|trigger|modifier|event_picture|resource_id|country_tag> --symbol <symbol>"),
        format!("hoi4skill validate-repair-context {mod_arg} --game-root {game_arg}{dependency_args} --strict-code-index --output .hoi4skill/ai_repair_context.json"),
        format!("hoi4skill validate {mod_arg} --game-root {game_arg}{dependency_args} --strict-code-index"),
    ];
    let status = if issues.is_empty() {
        "no_dry_run_issues"
    } else if game_index.is_some() {
        "repair_items_ready"
    } else {
        "needs_code_index"
    };
    let repair_items = if let Some(index) = game_index {
        issues
            .iter()
            .enumerate()
            .map(|(idx, issue)| validation_repair_item_json(idx + 1, &issue.message, index))
            .collect::<Vec<_>>()
            .join(",\n")
    } else {
        String::new()
    };
    let issue_json = issues
        .iter()
        .map(|issue| {
            format!(
                "{{\"source\": {}, \"message\": {}}}",
                json_str(&issue.source),
                json_str(&issue.message)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{{\n  \"schema\": \"hoi4skill.edit_context_repair_insurance.v1\",\n  \"status\": {},\n  \"strict_code_index_available\": {},\n  \"dry_run_issue_count\": {},\n  \"dry_run_issues\": [{}],\n  \"repair_items\": [\n{}\n  ],\n  \"ai_rules\": {},\n  \"next_commands\": {},\n  \"anti_hallucination_rule\": {}\n}}\n",
        json_str(status),
        json_bool(game_index.is_some()),
        issues.len(),
        issue_json,
        repair_items,
        json_array(&rules),
        json_array(&next_commands),
        json_str("A weak model may draft intent, but the CLI owns syntax, symbol lookup, final validation, and user questions for unknown mappings.")
    )
}

#[derive(Clone)]
pub(crate) struct EditContextRepairIssue {
    pub(crate) source: String,
    pub(crate) message: String,
}

pub(crate) fn edit_context_repair_insurance_messages(
    workflow_json: &str,
    max_items: usize,
) -> Vec<EditContextRepairIssue> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    let mut fields = vec![("blockers", "dry_run_safety_blocker")];
    if !workflow_json.contains("\"final_code_allowed\": true") {
        fields.push(("errors", "dry_run_validation_error"));
        fields.push(("warnings", "dry_run_validation_warning"));
    }
    for (field, source) in fields {
        for message in json_string_array_field(workflow_json, field) {
            if message.trim().is_empty() {
                continue;
            }
            let key = format!("{source}\n{message}");
            if seen.insert(key) {
                out.push(EditContextRepairIssue {
                    source: source.to_string(),
                    message,
                });
            }
            if out.len() >= max_items {
                return out;
            }
        }
    }
    out
}

fn workflow_validation_status(workflow_json: &str) -> &'static str {
    if workflow_json.contains("\"status\": \"errors\"") {
        "errors"
    } else if workflow_json.contains("\"status\": \"warnings\"") {
        "warnings"
    } else if workflow_json.contains("\"status\": \"ok\"") {
        "ok"
    } else if workflow_json.contains("\"ran\": false") {
        "not_run"
    } else {
        "unknown"
    }
}

fn workflow_safety_status(workflow_json: &str) -> &'static str {
    if workflow_json.contains("\"final_code_allowed\": false") {
        "blocked"
    } else if workflow_json.contains("\"final_code_allowed\": true") {
        "allows_final_code"
    } else {
        "unknown"
    }
}

fn push_markdown_list(out: &mut String, items: &[String]) {
    if items.is_empty() {
        out.push_str("- none\n");
    } else {
        for item in items {
            out.push_str(&format!("- {item}\n"));
        }
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    let lower = text.to_ascii_lowercase();
    needles.iter().any(|needle| {
        let needle_lower = needle.to_ascii_lowercase();
        lower.contains(&needle_lower) || text.contains(needle)
    })
}

pub(crate) fn json_string_field(json: &str, field: &str) -> Option<String> {
    let key = format!("\"{field}\"");
    let start = json.find(&key)? + key.len();
    let after_colon = json[start..].find(':')? + start + 1;
    let mut offset = after_colon;
    offset = skip_json_whitespace(json, offset)?;
    parse_json_string_at(json, offset).map(|(value, _)| value)
}

pub(crate) fn json_string_array_field(json: &str, field: &str) -> Vec<String> {
    let key = format!("\"{field}\"");
    let Some((_, after_colon)) = find_json_field(json, &key, 0) else {
        return Vec::new();
    };
    let mut offset = after_colon;
    let Some(next_offset) = skip_json_whitespace(json, offset) else {
        return Vec::new();
    };
    offset = next_offset;
    if !json[offset..].starts_with('[') {
        return Vec::new();
    }
    offset += 1;
    let mut out = Vec::new();
    loop {
        let Some(next_offset) = skip_json_whitespace(json, offset) else {
            break;
        };
        offset = next_offset;
        if json[offset..].starts_with(']') {
            break;
        }
        let Some((value, next)) = parse_json_string_at(json, offset) else {
            break;
        };
        out.push(value);
        offset = next;
        while json[offset..]
            .chars()
            .next()
            .is_some_and(|ch| ch.is_whitespace() || ch == ',')
        {
            offset += json[offset..].chars().next().unwrap().len_utf8();
        }
    }
    out
}

pub(crate) fn json_string_values_for_field(json: &str, field: &str) -> Vec<String> {
    let key = format!("\"{field}\"");
    let mut search_from = 0;
    let mut out = Vec::new();
    while let Some((field_start, after_colon)) = find_json_field(json, &key, search_from) {
        let Some(offset) = skip_json_whitespace(json, after_colon) else {
            break;
        };
        if let Some((value, next)) = parse_json_string_at(json, offset) {
            out.push(value);
            search_from = next;
        } else if json[offset..].starts_with("null") {
            out.push(String::new());
            search_from = offset + 4;
        } else {
            search_from = field_start + key.len();
        }
    }
    out
}

pub(crate) fn json_i64_values_for_field(json: &str, field: &str) -> Vec<Option<i64>> {
    let key = format!("\"{field}\"");
    let mut search_from = 0;
    let mut out = Vec::new();
    while let Some((field_start, after_colon)) = find_json_field(json, &key, search_from) {
        let Some(mut offset) = skip_json_whitespace(json, after_colon) else {
            break;
        };
        if json[offset..].starts_with("null") {
            out.push(None);
            search_from = offset + 4;
            continue;
        }
        let number_start = offset;
        if json[offset..].starts_with('-') {
            offset += 1;
        }
        while offset < json.len()
            && json[offset..]
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_digit())
        {
            offset += json[offset..].chars().next().unwrap().len_utf8();
        }
        if offset > number_start {
            out.push(json[number_start..offset].parse::<i64>().ok());
            search_from = offset;
        } else {
            search_from = field_start + key.len();
        }
    }
    out
}

fn json_i64_field_is_zero(json: &str, field: &str) -> bool {
    json_i64_values_for_field(json, field)
        .first()
        .is_some_and(|value| *value == Some(0))
}

fn loc_audit_report_is_clean(json: &str) -> bool {
    [
        "missing_count",
        "orphan_count",
        "duplicate_count",
        "token_issue_count",
    ]
    .iter()
    .all(|field| json_i64_field_is_zero(json, field))
}

fn gfx_audit_report_is_clean(json: &str) -> bool {
    [
        "missing_textures_count",
        "missing_sprites_count",
        "orphan_sprites_count",
        "unregistered_images_count",
    ]
    .iter()
    .all(|field| json_i64_field_is_zero(json, field))
}

fn json_array_field_raw(json: &str, field: &str) -> Option<String> {
    let key = format!("\"{field}\"");
    let (_, after_colon) = find_json_field(json, &key, 0)?;
    let mut offset = skip_json_whitespace(json, after_colon)?;
    if !json[offset..].starts_with('[') {
        return None;
    }
    let start = offset;
    let mut depth = 0i32;
    let mut in_quote = false;
    let mut escape = false;
    while offset < json.len() {
        let ch = json[offset..].chars().next()?;
        offset += ch.len_utf8();
        if in_quote {
            if ch == '"' && !escape {
                in_quote = false;
            }
            if escape {
                escape = false;
            } else {
                escape = ch == '\\';
            }
            continue;
        }
        if ch == '"' {
            in_quote = true;
            escape = false;
            continue;
        }
        if ch == '[' {
            depth += 1;
        } else if ch == ']' {
            depth -= 1;
            if depth == 0 {
                return Some(json[start..offset].to_string());
            }
        }
    }
    None
}

fn find_json_field(json: &str, key: &str, mut search_from: usize) -> Option<(usize, usize)> {
    while let Some(relative) = json[search_from..].find(key) {
        let field_start = search_from + relative;
        let after_key = field_start + key.len();
        let Some(after_ws) = skip_json_whitespace(json, after_key) else {
            return None;
        };
        if json[after_ws..].starts_with(':') {
            return Some((field_start, after_ws + 1));
        }
        search_from = after_key;
    }
    None
}

fn parse_json_string_at(json: &str, start: usize) -> Option<(String, usize)> {
    if !json[start..].starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let mut offset = start + 1;
    while offset < json.len() {
        let ch = json[offset..].chars().next()?;
        offset += ch.len_utf8();
        match ch {
            '"' => return Some((out, offset)),
            '\\' => {
                let esc = json[offset..].chars().next()?;
                offset += esc.len_utf8();
                match esc {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    'b' => out.push('\u{0008}'),
                    'f' => out.push('\u{000c}'),
                    'u' => {
                        let hex = json.get(offset..offset + 4)?;
                        let code = u16::from_str_radix(hex, 16).ok()?;
                        let decoded = char::from_u32(code as u32)?;
                        out.push(decoded);
                        offset += 4;
                    }
                    other => out.push(other),
                }
            }
            other => out.push(other),
        }
    }
    None
}

fn skip_json_whitespace(json: &str, mut offset: usize) -> Option<usize> {
    while offset < json.len() {
        let ch = json[offset..].chars().next()?;
        if !ch.is_whitespace() {
            break;
        }
        offset += ch.len_utf8();
    }
    Some(offset)
}

pub(crate) fn markdown_fence(info: &str, text: &str) -> String {
    format!("````{info}\n{text}\n````\n")
}

pub(crate) fn truncate_chars(text: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (idx, ch) in text.chars().enumerate() {
        if idx >= max_chars {
            out.push_str("\n... <truncated> ...");
            return out;
        }
        out.push(ch);
    }
    out
}
