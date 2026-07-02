//! Incremental local knowledge-base and template summaries.
//!
//! P1 is read-only for final mod content: these commands scan existing
//! game/mod files, record changed evidence, and summarize reusable templates
//! for later safety gates.

#[allow(unused_imports)]
use crate::*;

struct KnowledgeRoot {
    kind: String,
    root: PathBuf,
}

#[derive(Clone)]
struct KnowledgeFile {
    cache_key: String,
    root_kind: String,
    path: String,
    extension: String,
    bytes: u64,
    modified_unix: u64,
    hash: String,
    change: String,
}

struct TemplateSummary {
    id: String,
    lane: String,
    count: usize,
    evidence_files: Vec<String>,
    reuse_rule: String,
}

pub(crate) fn cmd_knowledge_base_refresh(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let roots = knowledge_roots_from_args(&map)?;
    let max_files = parse_usize_option(&map, "max-files", 10000)?;
    let output_path = value(&map, "output").map(normalize_path).transpose()?;
    let previous_path = value(&map, "previous")
        .map(normalize_path)
        .transpose()?
        .or_else(|| output_path.clone().filter(|path| path.exists()));
    let previous = previous_path
        .as_deref()
        .map(read_previous_knowledge_signatures)
        .transpose()?
        .unwrap_or_default();
    let files = collect_knowledge_files(&roots, &previous, max_files)?;
    let json = render_knowledge_base_refresh_json(
        &roots,
        &files,
        previous_path.as_deref(),
        map.flags.contains("incremental"),
        max_files,
    );

    if let Some(path) = output_path {
        write_or_print(&json, Some(&path.display().to_string()))
    } else {
        write_or_print(&json, None)
    }
}

pub(crate) fn cmd_knowledge_template_summarize(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mut roots = knowledge_roots_from_args(&map)?;
    if let Some(source) = value(&map, "source") {
        let kind = knowledge_source_kind(source)?;
        roots.retain(|root| root.kind == kind);
        if roots.is_empty() {
            return Err(format!("--source {source} did not match any supplied root"));
        }
    }
    let max_files = parse_usize_option(&map, "max-files", 10000)?;
    let max_examples = parse_usize_option(&map, "max-examples", 8)?;
    let previous = value(&map, "previous")
        .map(normalize_path)
        .transpose()?
        .map(|path| read_previous_knowledge_signatures(&path))
        .transpose()?
        .unwrap_or_default();
    let mut files = collect_knowledge_files(&roots, &previous, max_files)?;
    if map.flags.contains("changed-only") {
        if let Some(knowledge) = value(&map, "knowledge").or_else(|| value(&map, "refresh-report"))
        {
            let changed = read_changed_knowledge_cache_keys(&normalize_path(knowledge)?)?;
            files.retain(|file| changed.contains(&file.cache_key));
        } else {
            files.retain(|file| file.change != "unchanged");
        }
    }
    let summaries = summarize_templates_from_roots(&roots, &files, max_examples)?;
    let evidence_count = summaries.iter().filter(|summary| summary.count > 0).count();
    let json = render_knowledge_template_summary_json(
        &roots,
        &summaries,
        evidence_count,
        value(&map, "source"),
        map.flags.contains("changed-only"),
        files.len(),
    );
    write_or_print(&json, value(&map, "output"))?;

    if map.flags.contains("require-evidence") && evidence_count == 0 {
        return Err(
            "no reusable HOI4 code or prose templates were found in supplied roots".to_string(),
        );
    }

    Ok(())
}

pub(crate) fn cmd_evidence_db_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let roots = knowledge_roots_from_args(&map)?;
    let max_files = parse_usize_option(&map, "max-files", 10000)?;
    let previous_path = value(&map, "previous").map(normalize_path).transpose()?;
    let previous = previous_path
        .as_deref()
        .map(read_previous_knowledge_signatures)
        .transpose()?
        .unwrap_or_default();
    let files = collect_knowledge_files(&roots, &previous, max_files)?;
    let summaries = summarize_templates_from_roots(&roots, &files, 6)?;
    let json = render_evidence_db_audit_json(&roots, &files, &summaries, previous_path.as_deref());
    write_or_print(&json, value(&map, "output"))?;
    Ok(())
}

pub(crate) fn cmd_knowledge_compatibility_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let roots = knowledge_roots_from_args(&map)?;
    let max_files = parse_usize_option(&map, "max-files", 10000)?;
    let previous_path = value(&map, "previous")
        .or_else(|| value(&map, "knowledge"))
        .map(normalize_path)
        .transpose()?;
    let previous = previous_path
        .as_deref()
        .map(read_previous_knowledge_signatures)
        .transpose()?
        .unwrap_or_default();
    let files = collect_knowledge_files(&roots, &previous, max_files)?;
    let manifest_paths = knowledge_manifest_paths_from_args(&map)?;
    let manifest_files = knowledge_manifest_files(&manifest_paths)?;
    let steamdb_inputs = knowledge_steamdb_inputs(&map);
    let explicit_full_rebuild = map.flags.contains("full-rebuild");
    let schema_version = value(&map, "schema-version").unwrap_or("p102.v1");
    let previous_schema = value(&map, "previous-schema-version").unwrap_or(schema_version);
    let first_scan = previous_path.is_none() || previous.is_empty();
    let schema_changed = schema_version != previous_schema;
    let full_rebuild_required = explicit_full_rebuild || first_scan || schema_changed;
    let json = render_knowledge_compatibility_plan_json(
        &roots,
        &files,
        &manifest_files,
        &steamdb_inputs,
        previous_path.as_deref(),
        schema_version,
        previous_schema,
        full_rebuild_required,
        explicit_full_rebuild,
        first_scan,
        schema_changed,
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && full_rebuild_required {
        return Err("knowledge compatibility plan requires full rebuild before apply".to_string());
    }
    Ok(())
}

fn knowledge_source_kind(source: &str) -> Result<&'static str, String> {
    match source {
        "target" | "target_mod" | "mod" => Ok("target_mod"),
        "parent" | "dependency" | "dependency_mod" | "mod-path" => Ok("dependency_mod"),
        "game" | "game_root" => Ok("game"),
        _ => Err(format!(
            "unknown --source {source}; expected target, parent, dependency, or game"
        )),
    }
}

fn knowledge_manifest_paths_from_args(map: &ArgMap) -> Result<Vec<PathBuf>, String> {
    repeated_values(map, "manifest")
        .into_iter()
        .chain(repeated_values(map, "game-manifest"))
        .chain(repeated_values(map, "mod-manifest"))
        .map(normalize_path)
        .collect()
}

fn knowledge_manifest_files(paths: &[PathBuf]) -> Result<Vec<KnowledgeFile>, String> {
    let mut files = Vec::new();
    for path in paths {
        if !path.is_file() {
            return Err(format!("{}: manifest is not a file", path.display()));
        }
        let root = KnowledgeRoot {
            kind: "manifest".to_string(),
            root: path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from(".")),
        };
        files.push(knowledge_file_from_path(&root, path, &BTreeMap::new())?);
    }
    Ok(files)
}

fn knowledge_steamdb_inputs(map: &ArgMap) -> Vec<(String, String)> {
    let mut inputs = Vec::new();
    if let Some(value) = value(map, "steamdb-game-updated-at") {
        inputs.push(("game".to_string(), value.to_string()));
    }
    for value in repeated_values(map, "steamdb-parent-updated-at") {
        inputs.push(("dependency_mod".to_string(), value.to_string()));
    }
    inputs
}

fn knowledge_roots_from_args(map: &ArgMap) -> Result<Vec<KnowledgeRoot>, String> {
    let mut roots = Vec::new();
    if let Some(input) = map
        .positionals
        .first()
        .cloned()
        .or_else(|| value(map, "mod-root").map(str::to_string))
    {
        let resolved = resolve_mod_root(&normalize_path(&input)?)?;
        roots.push(KnowledgeRoot {
            kind: "target_mod".to_string(),
            root: resolved.root,
        });
    }
    if let Some(game_root) = value(map, "game-root") {
        let root = normalize_path(game_root)?;
        if !root.is_dir() {
            return Err(format!("{}: game root is not a directory", root.display()));
        }
        roots.push(KnowledgeRoot {
            kind: "game".to_string(),
            root,
        });
    }
    for dependency in repeated_values(map, "mod-path") {
        let resolved = resolve_mod_root(&normalize_path(dependency)?)?;
        roots.push(KnowledgeRoot {
            kind: "dependency_mod".to_string(),
            root: resolved.root,
        });
    }
    if roots.is_empty() {
        return Err(
            "missing --mod-root, positional mod root, --game-root, or --mod-path".to_string(),
        );
    }
    roots.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.root.cmp(&b.root)));
    roots.dedup_by(|a, b| a.kind == b.kind && a.root == b.root);
    Ok(roots)
}

fn collect_knowledge_files(
    roots: &[KnowledgeRoot],
    previous: &BTreeMap<String, String>,
    max_files: usize,
) -> Result<Vec<KnowledgeFile>, String> {
    let mut files = Vec::new();
    for root in roots {
        collect_knowledge_files_inner(root, &root.root, previous, max_files, &mut files)?;
        if files.len() >= max_files {
            break;
        }
    }
    files.sort_by(|a, b| a.cache_key.cmp(&b.cache_key));
    Ok(files)
}

fn collect_knowledge_files_inner(
    root: &KnowledgeRoot,
    dir: &Path,
    previous: &BTreeMap<String, String>,
    max_files: usize,
    files: &mut Vec<KnowledgeFile>,
) -> Result<(), String> {
    if files.len() >= max_files {
        return Ok(());
    }
    let mut entries = fs::read_dir(dir)
        .map_err(|e| format!("read dir {}: {e}", dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("read dir {}: {e}", dir.display()))?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        if files.len() >= max_files {
            break;
        }
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(OsStr::to_str).unwrap_or("");
            if matches!(name, ".git" | "target" | "node_modules") {
                continue;
            }
            collect_knowledge_files_inner(root, &path, previous, max_files, files)?;
        } else if path.is_file() && is_knowledge_file(&path) {
            files.push(knowledge_file_from_path(root, &path, previous)?);
        }
    }
    Ok(())
}

fn knowledge_file_from_path(
    root: &KnowledgeRoot,
    path: &Path,
    previous: &BTreeMap<String, String>,
) -> Result<KnowledgeFile, String> {
    let metadata = fs::metadata(path).map_err(|e| format!("metadata {}: {e}", path.display()))?;
    let bytes = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let rel = relative_slash_path(&root.root, path);
    let cache_key = format!("{}:{}", root.kind, rel);
    let hash = fnv1a_hex(&bytes);
    let change = match previous.get(&cache_key) {
        Some(old) if old == &hash => "unchanged",
        Some(_) => "changed",
        None => "new",
    }
    .to_string();
    let modified_unix = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    Ok(KnowledgeFile {
        cache_key,
        root_kind: root.kind.clone(),
        path: rel,
        extension: path
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase(),
        bytes: metadata.len(),
        modified_unix,
        hash,
        change,
    })
}

fn is_knowledge_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "txt" | "yml" | "yaml" | "gfx" | "gui" | "asset" | "mod" | "md" | "json" | "csv"
    )
}

fn fnv1a_hex(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn read_previous_knowledge_signatures(path: &Path) -> Result<BTreeMap<String, String>, String> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let text = read_utf8_lossy(path)?;
    let mut out = BTreeMap::new();
    let mut rest = text.as_str();
    while let Some(key_pos) = rest.find("\"cache_key\"") {
        rest = &rest[key_pos + "\"cache_key\"".len()..];
        let Some(cache_key) = parse_json_string_after_colon(rest) else {
            continue;
        };
        let Some(hash_pos) = rest.find("\"hash\"") else {
            break;
        };
        rest = &rest[hash_pos + "\"hash\"".len()..];
        let Some(hash) = parse_json_string_after_colon(rest) else {
            continue;
        };
        out.insert(cache_key, hash);
    }
    Ok(out)
}

fn read_changed_knowledge_cache_keys(path: &Path) -> Result<BTreeSet<String>, String> {
    if !path.exists() {
        return Err(format!(
            "{}: knowledge refresh report not found",
            path.display()
        ));
    }
    let text = read_utf8_lossy(path)?;
    let mut out = BTreeSet::new();
    let mut rest = text.as_str();
    while let Some(key_pos) = rest.find("\"cache_key\"") {
        rest = &rest[key_pos + "\"cache_key\"".len()..];
        let Some(cache_key) = parse_json_string_after_colon(rest) else {
            continue;
        };
        let change = rest
            .find("\"change\"")
            .and_then(|change_pos| {
                parse_json_string_after_colon(&rest[change_pos + "\"change\"".len()..])
            })
            .unwrap_or_default();
        if change == "new" || change == "changed" || change == "deleted" {
            out.insert(cache_key);
        }
    }
    Ok(out)
}

fn parse_json_string_after_colon(text: &str) -> Option<String> {
    let colon = text.find(':')?;
    let mut chars = text[colon + 1..].chars().peekable();
    while matches!(chars.peek(), Some(ch) if ch.is_whitespace()) {
        chars.next();
    }
    if chars.next()? != '"' {
        return None;
    }
    let mut out = String::new();
    let mut escaped = false;
    for ch in chars {
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

fn render_knowledge_base_refresh_json(
    roots: &[KnowledgeRoot],
    files: &[KnowledgeFile],
    previous_path: Option<&Path>,
    incremental: bool,
    max_files: usize,
) -> String {
    let changed_count = files
        .iter()
        .filter(|file| file.change == "changed" || file.change == "new")
        .count();
    let unchanged_count = files
        .iter()
        .filter(|file| file.change == "unchanged")
        .count();
    let previous_text = previous_path.map(|path| path.to_string_lossy().to_string());
    let mut map = BTreeMap::new();
    map.insert(
        "schema".to_string(),
        json_str("hoi4skill.knowledge_base_refresh.v1"),
    );
    map.insert("ok".to_string(), "true".to_string());
    map.insert(
        "status".to_string(),
        json_str(if incremental {
            "incremental_refresh_ready"
        } else {
            "snapshot_ready"
        }),
    );
    map.insert(
        "incremental".to_string(),
        json_bool(incremental).to_string(),
    );
    map.insert("root_count".to_string(), roots.len().to_string());
    map.insert("file_count".to_string(), files.len().to_string());
    map.insert("changed_count".to_string(), changed_count.to_string());
    map.insert("unchanged_count".to_string(), unchanged_count.to_string());
    map.insert("max_files".to_string(), max_files.to_string());
    map.insert(
        "previous".to_string(),
        json_optional_str(previous_text.as_deref()),
    );
    map.insert("roots".to_string(), render_knowledge_roots_json(roots));
    map.insert("files".to_string(), render_knowledge_files_json(files));
    map.insert(
        "rules".to_string(),
        json_array(&[
            "do not full-rebuild unless first run, schema change, dependency change, corruption, or explicit --full-rebuild".to_string(),
            "only changed cache_key entries need downstream template/symbol refresh".to_string(),
            "AI receives summaries and evidence links, not permission to invent Clausewitz syntax".to_string(),
        ]),
    );
    json_raw_object(&map)
}

fn render_knowledge_roots_json(roots: &[KnowledgeRoot]) -> String {
    format!(
        "[{}]",
        roots
            .iter()
            .map(|root| {
                let mut map = BTreeMap::new();
                map.insert("kind".to_string(), json_str(&root.kind));
                map.insert(
                    "root".to_string(),
                    json_str(&root.root.display().to_string()),
                );
                json_raw_object(&map)
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn render_knowledge_files_json(files: &[KnowledgeFile]) -> String {
    format!(
        "[{}]",
        files
            .iter()
            .map(|file| {
                let mut map = BTreeMap::new();
                map.insert("cache_key".to_string(), json_str(&file.cache_key));
                map.insert("root_kind".to_string(), json_str(&file.root_kind));
                map.insert("path".to_string(), json_str(&file.path));
                map.insert("extension".to_string(), json_str(&file.extension));
                map.insert("bytes".to_string(), file.bytes.to_string());
                map.insert("modified_unix".to_string(), file.modified_unix.to_string());
                map.insert("hash".to_string(), json_str(&file.hash));
                map.insert("change".to_string(), json_str(&file.change));
                json_raw_object(&map)
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn render_evidence_db_audit_json(
    roots: &[KnowledgeRoot],
    files: &[KnowledgeFile],
    summaries: &[TemplateSummary],
    previous_path: Option<&Path>,
) -> String {
    let changed_count = files
        .iter()
        .filter(|file| matches!(file.change.as_str(), "new" | "changed"))
        .count();
    let unchanged_count = files
        .iter()
        .filter(|file| file.change == "unchanged")
        .count();
    let symbol_rows = evidence_symbol_index_rows(files);
    let template_evidence_count = summaries.iter().filter(|summary| summary.count > 0).count();
    let previous_text = previous_path.map(|path| path.to_string_lossy().to_string());
    let mut map = BTreeMap::new();
    map.insert(
        "schema".to_string(),
        json_str("hoi4skill.evidence_db_audit.v1"),
    );
    map.insert("ok".to_string(), "true".to_string());
    map.insert("status".to_string(), json_str("evidence_db_ready"));
    map.insert(
        "local_evidence_only".to_string(),
        json_bool(true).to_string(),
    );
    map.insert(
        "stores_source_code".to_string(),
        json_bool(false).to_string(),
    );
    map.insert(
        "previous".to_string(),
        json_optional_str(previous_text.as_deref()),
    );
    map.insert("root_count".to_string(), roots.len().to_string());
    map.insert("file_count".to_string(), files.len().to_string());
    map.insert("changed_count".to_string(), changed_count.to_string());
    map.insert("unchanged_count".to_string(), unchanged_count.to_string());
    map.insert(
        "source_layers".to_string(),
        evidence_source_layers_json(files),
    );
    map.insert(
        "symbol_indices".to_string(),
        evidence_symbol_rows_json(&symbol_rows),
    );
    map.insert("template_count".to_string(), summaries.len().to_string());
    map.insert(
        "template_evidence_count".to_string(),
        template_evidence_count.to_string(),
    );
    map.insert(
        "templates".to_string(),
        render_template_summaries_json(summaries),
    );
    map.insert(
        "changed_cache_keys".to_string(),
        json_array(
            &files
                .iter()
                .filter(|file| matches!(file.change.as_str(), "new" | "changed"))
                .map(|file| file.cache_key.clone())
                .collect::<Vec<_>>(),
        ),
    );
    map.insert(
        "rules".to_string(),
        json_array(&[
            "evidence DB is a derived cache from local game, target mod, and dependency mod files"
                .to_string(),
            "do not store or ship source code snippets from game or parent mods".to_string(),
            "unchanged cache_key entries do not need template or symbol refresh".to_string(),
            "changed cache_key entries must trigger stale-plan-gate before apply".to_string(),
        ]),
    );
    json_raw_object(&map)
}

fn render_knowledge_compatibility_plan_json(
    roots: &[KnowledgeRoot],
    files: &[KnowledgeFile],
    manifests: &[KnowledgeFile],
    steamdb_inputs: &[(String, String)],
    previous_path: Option<&Path>,
    schema_version: &str,
    previous_schema: &str,
    full_rebuild_required: bool,
    explicit_full_rebuild: bool,
    first_scan: bool,
    schema_changed: bool,
) -> String {
    let changed_files = files
        .iter()
        .filter(|file| matches!(file.change.as_str(), "new" | "changed"))
        .cloned()
        .collect::<Vec<_>>();
    let affected_lanes = knowledge_affected_lanes(&changed_files);
    let mut rebuild_reasons = Vec::new();
    if explicit_full_rebuild {
        rebuild_reasons.push("explicit --full-rebuild".to_string());
    }
    if first_scan {
        rebuild_reasons.push("first scan has no previous knowledge signatures".to_string());
    }
    if schema_changed {
        rebuild_reasons.push("knowledge schema version changed".to_string());
    }
    let previous_text = previous_path.map(|path| path.to_string_lossy().to_string());
    let mut map = BTreeMap::new();
    map.insert(
        "schema".to_string(),
        json_str("hoi4skill.knowledge_compatibility_plan.v1"),
    );
    map.insert(
        "ok".to_string(),
        json_bool(!full_rebuild_required).to_string(),
    );
    map.insert(
        "status".to_string(),
        json_str(if full_rebuild_required {
            "full_rebuild_required"
        } else if changed_files.is_empty() {
            "knowledge_current"
        } else {
            "incremental_refresh_required"
        }),
    );
    map.insert(
        "local_evidence_only".to_string(),
        json_bool(true).to_string(),
    );
    map.insert(
        "stores_source_code".to_string(),
        json_bool(false).to_string(),
    );
    map.insert(
        "refresh_mode".to_string(),
        json_str(if full_rebuild_required {
            "full"
        } else if changed_files.is_empty() {
            "none"
        } else {
            "incremental"
        }),
    );
    map.insert(
        "full_rebuild_required".to_string(),
        json_bool(full_rebuild_required).to_string(),
    );
    map.insert("rebuild_reasons".to_string(), json_array(&rebuild_reasons));
    map.insert("schema_version".to_string(), json_str(schema_version));
    map.insert(
        "previous_schema_version".to_string(),
        json_str(previous_schema),
    );
    map.insert(
        "previous".to_string(),
        json_optional_str(previous_text.as_deref()),
    );
    map.insert("root_count".to_string(), roots.len().to_string());
    map.insert("roots".to_string(), render_knowledge_roots_json(roots));
    map.insert("file_count".to_string(), files.len().to_string());
    map.insert("changed_count".to_string(), changed_files.len().to_string());
    map.insert(
        "changed_cache_keys".to_string(),
        json_array(
            &changed_files
                .iter()
                .map(|file| file.cache_key.clone())
                .collect::<Vec<_>>(),
        ),
    );
    map.insert(
        "affected_lanes".to_string(),
        json_array(&affected_lanes.into_iter().collect::<Vec<_>>()),
    );
    map.insert("manifest_count".to_string(), manifests.len().to_string());
    map.insert(
        "manifests".to_string(),
        render_knowledge_files_json(manifests),
    );
    map.insert(
        "external_update_hints".to_string(),
        knowledge_steamdb_inputs_json(steamdb_inputs),
    );
    map.insert(
        "next_commands".to_string(),
        json_array(&[
            "hoi4skill knowledge-delta-refresh --mod-root <target> --game-root <hoi4> --mod-path <parent> --previous .hoi4skill/kb.json --incremental --output .hoi4skill/kb.json".to_string(),
            "hoi4skill knowledge-template-summarize --mod-root <target> --game-root <hoi4> --mod-path <parent> --knowledge .hoi4skill/kb.json --changed-only --output .hoi4skill/template_delta.json".to_string(),
            "hoi4skill stale-plan-gate --input .hoi4skill/transaction.json --knowledge .hoi4skill/kb.json --require-passed".to_string(),
        ]),
    );
    map.insert(
        "rules".to_string(),
        json_array(&[
            "do not rebuild unchanged cache_key entries".to_string(),
            "full rebuild is reserved for first scan, schema change, explicit request, corruption, or dependency identity change".to_string(),
            "SteamDB or launcher manifests are update hints; local file hashes remain the authoritative code evidence".to_string(),
            "AI context receives changed lane summaries and evidence references, not source-code copies".to_string(),
        ]),
    );
    json_raw_object(&map)
}

fn knowledge_affected_lanes(files: &[KnowledgeFile]) -> BTreeSet<String> {
    let mut lanes = BTreeSet::new();
    for file in files {
        let path = file.path.replace('\\', "/").to_ascii_lowercase();
        let lane = if path.contains("common/national_focus") {
            "focus"
        } else if path.contains("events/") {
            "event"
        } else if path.contains("common/decisions") {
            "decision"
        } else if path.contains("common/ideas") {
            "idea"
        } else if path.contains("common/dynamic_modifiers") {
            "dynamic_modifier"
        } else if path.contains("common/scripted_effects") {
            "scripted_effect"
        } else if path.contains("common/scripted_triggers") {
            "scripted_trigger"
        } else if path.contains("common/scripted_guis") || path.contains("interface/") {
            "gui_gfx"
        } else if path.contains("localisation/") || path.contains("localization/") {
            "localisation"
        } else if path.contains("history/units") {
            "oob"
        } else if path.contains("history/states") || path.starts_with("map/") {
            "map"
        } else if path.contains("history/countries") || path.contains("history/diplomacy") {
            "history"
        } else if path.contains("common/units") || path.contains("common/technologies") {
            "unit_technology"
        } else {
            "common"
        };
        lanes.insert(lane.to_string());
    }
    lanes
}

fn knowledge_steamdb_inputs_json(inputs: &[(String, String)]) -> String {
    format!(
        "[{}]",
        inputs
            .iter()
            .map(|(kind, updated_at)| {
                let mut map = BTreeMap::new();
                map.insert("kind".to_string(), json_str(kind));
                map.insert("updated_at".to_string(), json_str(updated_at));
                map.insert("source".to_string(), json_str("external_update_hint"));
                json_raw_object(&map)
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn evidence_source_layers_json(files: &[KnowledgeFile]) -> String {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for file in files {
        *counts.entry(file.root_kind.clone()).or_insert(0) += 1;
    }
    format!(
        "[{}]",
        counts
            .into_iter()
            .map(|(layer, count)| {
                format!(
                    "{{\"layer\": {}, \"file_count\": {}}}",
                    json_str(&layer),
                    count
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn evidence_symbol_index_rows(files: &[KnowledgeFile]) -> Vec<(&'static str, usize)> {
    let categories: [(&str, fn(&str) -> bool); 16] = [
        ("focus", |path: &str| path.contains("common/national_focus")),
        ("event", |path: &str| path.contains("events/")),
        ("decision", |path: &str| path.contains("common/decisions")),
        ("idea", |path: &str| path.contains("common/ideas")),
        ("dynamic_modifier", |path: &str| {
            path.contains("common/dynamic_modifiers")
        }),
        ("scripted_effect", |path: &str| {
            path.contains("common/scripted_effects")
        }),
        ("scripted_trigger", |path: &str| {
            path.contains("common/scripted_triggers")
        }),
        ("scripted_localisation", |path: &str| {
            path.contains("common/scripted_localisation")
        }),
        ("sprite", |path: &str| path.ends_with(".gfx")),
        ("technology", |path: &str| {
            path.contains("common/technologies")
        }),
        ("equipment", |path: &str| {
            path.contains("common/units/equipment")
        }),
        ("unit", |path: &str| path.contains("common/units/")),
        ("state", |path: &str| path.contains("history/states")),
        ("province", |path: &str| path.contains("map/definition.csv")),
        ("gui", |path: &str| {
            path.contains("common/scripted_guis") || path.ends_with(".gui")
        }),
        ("map_topology", |path: &str| {
            path.contains("map/")
                && (path.ends_with(".csv")
                    || path.ends_with(".bmp")
                    || path.ends_with("default.map")
                    || path.contains("strategicregions"))
        }),
    ];
    categories
        .into_iter()
        .map(|(id, matcher)| {
            let count = files
                .iter()
                .filter(|file| matcher(&file.path.to_ascii_lowercase()))
                .count();
            (id, count)
        })
        .collect()
}

fn evidence_symbol_rows_json(rows: &[(&'static str, usize)]) -> String {
    format!(
        "[{}]",
        rows.iter()
            .map(|(id, count)| {
                format!(
                    "{{\"id\": {}, \"evidence_file_count\": {}, \"status\": {}}}",
                    json_str(id),
                    count,
                    json_str(if *count > 0 {
                        "evidence_ready"
                    } else {
                        "missing_evidence"
                    })
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn summarize_templates_from_roots(
    roots: &[KnowledgeRoot],
    files: &[KnowledgeFile],
    max_examples: usize,
) -> Result<Vec<TemplateSummary>, String> {
    let mut summaries = template_summary_seed();
    for root in roots {
        for file in files.iter().filter(|file| file.root_kind == root.kind) {
            let path = root.root.join(file.path.replace('/', "\\"));
            let Ok(text) = read_utf8_lossy(&path) else {
                continue;
            };
            let lower_path = file.path.to_ascii_lowercase();
            let lower_text = text.to_ascii_lowercase();
            for summary in &mut summaries {
                if template_matches(&summary.id, &lower_path, &lower_text) {
                    summary.count += 1;
                    if summary.evidence_files.len() < max_examples
                        && !summary
                            .evidence_files
                            .iter()
                            .any(|existing| existing == &file.cache_key)
                    {
                        summary.evidence_files.push(file.cache_key.clone());
                    }
                }
            }
        }
    }
    Ok(summaries)
}

fn template_summary_seed() -> Vec<TemplateSummary> {
    vec![
        template_summary("national_focus", "focus_tree", "reuse focus_tree/focus layout shape only; register new IDs and localisation before writing"),
        template_summary("event_chain", "events", "reuse country_event/news_event structure, trigger style, and option cadence only"),
        template_summary("decision", "decisions", "reuse decision category/decision shape only; verify visible/available/remove_effect scopes"),
        template_summary("national_spirit", "ideas", "reuse idea picture/modifier container shape only; modifiers must pass scope catalog later"),
        template_summary("dynamic_modifier", "dynamic_modifiers", "reuse scripted dynamic-modifier helper pattern; do not treat dynamic modifiers as national spirits"),
        template_summary("scripted_effect", "scripted_effects", "reuse effect wrapper names and variable plumbing only after symbol registration"),
        template_summary("scripted_trigger", "scripted_triggers", "reuse trigger wrappers only in trigger/limit/available contexts"),
        template_summary("gui", "gui", "reuse GUI mount/control/style evidence only; final GUI writes need later GUI gates"),
        template_summary("localisation_prose", "localisation", "learn punctuation, color-token, icon-token, and sentence cadence without copying prose"),
    ]
}

fn template_summary(id: &str, lane: &str, reuse_rule: &str) -> TemplateSummary {
    TemplateSummary {
        id: id.to_string(),
        lane: lane.to_string(),
        count: 0,
        evidence_files: Vec::new(),
        reuse_rule: reuse_rule.to_string(),
    }
}

fn template_matches(id: &str, lower_path: &str, lower_text: &str) -> bool {
    match id {
        "national_focus" => {
            lower_path.contains("common/national_focus") && lower_text.contains("focus")
        }
        "event_chain" => lower_path.contains("events") && lower_text.contains("_event"),
        "decision" => lower_path.contains("common/decisions") && lower_text.contains("decision"),
        "national_spirit" => lower_path.contains("common/ideas") && lower_text.contains("ideas"),
        "dynamic_modifier" => {
            lower_path.contains("common/dynamic_modifiers")
                || lower_text.contains("dynamic_modifier")
        }
        "scripted_effect" => lower_path.contains("common/scripted_effects"),
        "scripted_trigger" => lower_path.contains("common/scripted_triggers"),
        "gui" => lower_path.contains("common/scripted_guis") || lower_path.contains("interface/"),
        "localisation_prose" => {
            lower_path.contains("localisation") && lower_text.contains("_desc:")
        }
        _ => false,
    }
}

fn render_knowledge_template_summary_json(
    roots: &[KnowledgeRoot],
    summaries: &[TemplateSummary],
    evidence_count: usize,
    source_filter: Option<&str>,
    changed_only: bool,
    scanned_file_count: usize,
) -> String {
    let mut map = BTreeMap::new();
    map.insert(
        "schema".to_string(),
        json_str("hoi4skill.knowledge_template_summary.v1"),
    );
    map.insert("ok".to_string(), json_bool(evidence_count > 0).to_string());
    map.insert(
        "status".to_string(),
        json_str(if evidence_count > 0 {
            "template_evidence_ready"
        } else {
            "no_template_evidence"
        }),
    );
    map.insert("root_count".to_string(), roots.len().to_string());
    map.insert(
        "source_filter".to_string(),
        json_optional_str(source_filter),
    );
    map.insert(
        "changed_only".to_string(),
        json_bool(changed_only).to_string(),
    );
    map.insert(
        "scanned_file_count".to_string(),
        scanned_file_count.to_string(),
    );
    map.insert("template_count".to_string(), summaries.len().to_string());
    map.insert(
        "evidence_template_count".to_string(),
        evidence_count.to_string(),
    );
    map.insert("roots".to_string(), render_knowledge_roots_json(roots));
    map.insert(
        "templates".to_string(),
        render_template_summaries_json(summaries),
    );
    map.insert(
        "style_learning_rule".to_string(),
        json_str("summaries may learn structure and style signals from local evidence; do not copy source prose or invent missing syntax"),
    );
    json_raw_object(&map)
}

fn render_template_summaries_json(summaries: &[TemplateSummary]) -> String {
    format!(
        "[{}]",
        summaries
            .iter()
            .map(|summary| {
                let mut map = BTreeMap::new();
                map.insert("id".to_string(), json_str(&summary.id));
                map.insert("lane".to_string(), json_str(&summary.lane));
                map.insert("count".to_string(), summary.count.to_string());
                map.insert(
                    "status".to_string(),
                    json_str(if summary.count > 0 {
                        "evidence_ready"
                    } else {
                        "missing_evidence"
                    }),
                );
                map.insert(
                    "evidence_files".to_string(),
                    json_array(&summary.evidence_files),
                );
                map.insert("reuse_rule".to_string(), json_str(&summary.reuse_rule));
                json_raw_object(&map)
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}
