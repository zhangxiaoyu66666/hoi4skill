//! P63 stale plan gate.
//!
//! Apply commands should not consume old plans after the local game, parent mod,
//! or target mod knowledge report shows changed evidence.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_stale_plan_gate(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let knowledge = normalize_path(&require_value(&map, "knowledge")?)?;
    let plan_text = read_utf8_lossy(&input)?;
    let knowledge_text = read_utf8_lossy(&knowledge)?;
    let changed_files = stale_plan_json_string_array(&plan_text, "changed_files");
    let changed_cache_keys = stale_knowledge_changed_cache_keys(&knowledge_text);
    let stale_markers = [
        "\"change\": \"changed\"",
        "\"stale\": true",
        "parent_hash_changed",
        "source_hash_mismatch",
    ];
    let stale_detected = stale_markers
        .iter()
        .any(|marker| knowledge_text.contains(marker));
    let mut blockers = Vec::new();
    if changed_files.is_empty() {
        blockers.push("plan has no changed_files evidence".to_string());
    }
    if stale_detected {
        blockers.push("knowledge report contains changed/stale source evidence; regenerate the plan after incremental refresh".to_string());
    }
    if map.flags.contains("require-freshness-record")
        && !plan_text.contains("\"knowledge\"")
        && !plan_text.contains("\"knowledge_version\"")
        && !plan_text.contains("\"source_hash\"")
    {
        blockers.push("plan has no knowledge/source hash record".to_string());
    }

    let ok = blockers.is_empty();
    let report = stale_plan_gate_json(
        ok,
        &input,
        &knowledge,
        &changed_files,
        &changed_cache_keys,
        &blockers,
    );
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

fn stale_knowledge_changed_cache_keys(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for object in text.split('{').skip(1) {
        let Some(end) = object.find('}') else {
            continue;
        };
        let item = &object[..end];
        if !item.contains("\"change\": \"changed\"") && !item.contains("\"stale\": true") {
            continue;
        }
        if let Some(cache_key) = stale_json_string_field(item, "cache_key") {
            out.push(cache_key);
        }
    }
    out.sort();
    out.dedup();
    out
}

fn stale_plan_json_string_array(text: &str, key: &str) -> Vec<String> {
    let marker = format!("\"{key}\": [");
    let Some(start) = text.find(&marker) else {
        return Vec::new();
    };
    let rest = &text[start + marker.len()..];
    let Some(end) = rest.find(']') else {
        return Vec::new();
    };
    rest[..end]
        .split(',')
        .filter_map(|raw| {
            let trimmed = raw.trim().trim_matches('"');
            (!trimmed.is_empty()).then(|| trimmed.replace("\\\"", "\"").replace("\\\\", "\\"))
        })
        .collect()
}

fn stale_json_string_field(text: &str, key: &str) -> Option<String> {
    let marker = format!("\"{key}\"");
    let start = text.find(&marker)? + marker.len();
    let rest = &text[start..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let mut out = String::new();
    let mut escaped = false;
    for ch in rest.chars() {
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

fn stale_plan_gate_json(
    ok: bool,
    input: &Path,
    knowledge: &Path,
    changed_files: &[String],
    changed_cache_keys: &[String],
    blockers: &[String],
) -> String {
    let mut map = BTreeMap::new();
    map.insert(
        "schema".to_string(),
        json_str("hoi4skill.stale_plan_gate.v1"),
    );
    map.insert("ok".to_string(), json_bool(ok).to_string());
    map.insert(
        "status".to_string(),
        json_str(if ok {
            "plan_fresh"
        } else {
            "stale_plan_blocked"
        }),
    );
    map.insert("input".to_string(), json_str(&input.display().to_string()));
    map.insert(
        "knowledge".to_string(),
        json_str(&knowledge.display().to_string()),
    );
    map.insert("changed_files".to_string(), json_array(changed_files));
    map.insert(
        "changed_cache_keys".to_string(),
        json_array(changed_cache_keys),
    );
    map.insert(
        "stale_detected".to_string(),
        json_bool(!changed_cache_keys.is_empty() || blockers.iter().any(|b| b.contains("stale")))
            .to_string(),
    );
    map.insert("blocker_count".to_string(), blockers.len().to_string());
    map.insert("blockers".to_string(), json_array(blockers));
    map.insert(
        "rules".to_string(),
        json_array(&[
            "apply must run after stale-plan-gate".to_string(),
            "knowledge changed/stale evidence requires regenerating the transaction plan"
                .to_string(),
            "new first-scan knowledge is not stale by itself; changed hashes are stale".to_string(),
        ]),
    );
    json_raw_object(&map)
}
