//! P14 parent-mod override, conflict, and stale-template planning.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_parent_mod_diff_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let target = required_root_value(&map, "mod-root")?;
    let parent = required_root_value(&map, "mod-path")?;
    let changed = require_value(&map, "changed")?;
    let target_file = target.join(changed.replace('/', "\\"));
    let parent_file = parent.join(changed.replace('/', "\\"));
    let target_exists = target_file.exists();
    let parent_exists = parent_file.exists();
    let target_hash = target_exists
        .then(|| file_hash_hex(&target_file))
        .transpose()?;
    let parent_hash = parent_exists
        .then(|| file_hash_hex(&parent_file))
        .transpose()?;
    let relation = if target_exists && parent_exists {
        if target_hash == parent_hash {
            "same_as_parent"
        } else {
            "overrides_parent"
        }
    } else if target_exists {
        "target_only"
    } else if parent_exists {
        "parent_only"
    } else {
        "missing_both"
    };
    let blockers = if target_exists {
        Vec::new()
    } else {
        vec![format!("target changed file `{changed}` does not exist")]
    };
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"changed\": {},\n  \"relation\": {},\n  \"target_file\": {},\n  \"parent_file\": {},\n  \"target_hash\": {},\n  \"parent_hash\": {},\n  \"recommended_refresh\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.parent_mod_diff_plan.v1"),
        json_bool(ok),
        json_str(if ok { "parent_diff_ready" } else { "blocked" }),
        json_str(&changed),
        json_str(relation),
        json_str(&target_file.display().to_string()),
        json_str(&parent_file.display().to_string()),
        json_optional_str(target_hash.as_deref()),
        json_optional_str(parent_hash.as_deref()),
        json_array(&[changed.to_string()]),
        json_array(&blockers),
        json_str("parent-mod edits compare same relative paths and refresh only affected templates/files")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_override_risk_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let target = required_root_value(&map, "mod-root")?;
    let parent = required_root_value(&map, "mod-path")?;
    let target_files = indexed_text_files(&target)?;
    let parent_files = indexed_text_files(&parent)?;
    let parent_set = parent_files.iter().cloned().collect::<BTreeSet<_>>();
    let mut overlaps = Vec::new();
    for file in &target_files {
        if parent_set.contains(file) {
            let target_hash = file_hash_hex(&target.join(file.replace('/', "\\")))?;
            let parent_hash = file_hash_hex(&parent.join(file.replace('/', "\\")))?;
            overlaps.push((file.clone(), target_hash, parent_hash));
        }
    }
    let rows = overlaps
        .iter()
        .take(parse_usize_option(&map, "max-items", 200)?)
        .map(|(file, target_hash, parent_hash)| {
            format!(
                "{{\"file\": {}, \"target_hash\": {}, \"parent_hash\": {}, \"same\": {}}}",
                json_str(file),
                json_str(target_hash),
                json_str(parent_hash),
                json_bool(target_hash == parent_hash)
            )
        })
        .collect::<Vec<_>>();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": true,\n  \"status\": {},\n  \"target_file_count\": {},\n  \"parent_file_count\": {},\n  \"overlap_count\": {},\n  \"overlaps\": [{}],\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.override_risk_audit.v1"),
        json_str(if overlaps.is_empty() {
            "no_overrides_found"
        } else {
            "override_risk_reported"
        }),
        target_files.len(),
        parent_files.len(),
        overlaps.len(),
        rows.join(", "),
        json_str("same relative-path files are override risks; inspect hash changes before letting AI rewrite inherited systems")
    );
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_dependency_freshness_check(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let target = required_root_value(&map, "mod-root")?;
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    let parents = repeated_values(&map, "mod-path")
        .into_iter()
        .map(normalize_path)
        .collect::<Result<Vec<_>, _>>()?;
    if parents.is_empty() {
        return Err("missing --mod-path".to_string());
    }
    let mut roots = Vec::new();
    if let Some(game_root) = &game_root {
        roots.push(("game".to_string(), game_root.clone()));
    }
    for parent in &parents {
        roots.push(("parent_mod".to_string(), parent.clone()));
    }
    roots.push(("target_mod".to_string(), target.clone()));
    let rows = roots
        .iter()
        .map(|(kind, root)| {
            let hash = root_fingerprint(root).unwrap_or_else(|_| "missing".to_string());
            format!(
                "{{\"kind\": {}, \"root\": {}, \"fingerprint\": {}}}",
                json_str(kind),
                json_str(&root.display().to_string()),
                json_str(&hash)
            )
        })
        .collect::<Vec<_>>();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": true,\n  \"status\": {},\n  \"root_count\": {},\n  \"roots\": [{}],\n  \"recommended_refresh\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.dependency_freshness_check.v1"),
        json_str("freshness_snapshot_ready"),
        rows.len(),
        rows.join(", "),
        json_array(&["knowledge-base-refresh --incremental --previous <old> with only changed roots/files".to_string()]),
        json_str("freshness checks produce fingerprints for incremental comparison; do not rebuild all templates when only a subset changed")
    );
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_stale_template_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let knowledge = normalize_path(&require_value(&map, "knowledge")?)?;
    let text = read_utf8_lossy(&knowledge)?;
    let stale_markers = [
        "\"change\": \"changed\"",
        "\"stale\": true",
        "parent_hash_changed",
    ];
    let stale = stale_markers.iter().any(|marker| text.contains(marker));
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"knowledge\": {},\n  \"stale_detected\": {},\n  \"refresh_scope\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.stale_template_audit.v1"),
        json_bool(!stale),
        json_str(if stale {
            "stale_templates_detected"
        } else {
            "templates_current"
        }),
        json_str(&knowledge.display().to_string()),
        json_bool(stale),
        json_array(&["refresh only changed file hashes and their related templates".to_string()]),
        json_str("AI must not reuse stale parent-mod templates after dependency fingerprints change")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && stale {
        return Err("stale templates detected".to_string());
    }
    Ok(())
}

fn required_root_value(map: &ArgMap, key: &str) -> Result<PathBuf, String> {
    normalize_path(&require_value(map, key)?)
}

fn indexed_text_files(root: &Path) -> Result<Vec<String>, String> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut files = collect_files(root)?
        .into_iter()
        .filter(|file| {
            matches!(
                file.extension().and_then(OsStr::to_str).unwrap_or(""),
                "txt" | "yml" | "gui" | "gfx" | "mod"
            )
        })
        .map(|file| relative_slash_path(root, &file))
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

fn root_fingerprint(root: &Path) -> Result<String, String> {
    let mut hash: u64 = 0xcbf29ce484222325;
    for rel in indexed_text_files(root)?.into_iter().take(5000) {
        for byte in rel.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        let file_hash = file_hash_hex(&root.join(rel.replace('/', "\\")))?;
        for byte in file_hash.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    Ok(format!("{hash:016x}"))
}

fn file_hash_hex(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    Ok(format!("{hash:016x}"))
}
