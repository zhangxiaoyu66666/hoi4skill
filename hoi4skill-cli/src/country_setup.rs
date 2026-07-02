//! P19 country and start-date setup gates.
//!
//! Creating a country is a high-blast-radius operation. The command here is a
//! plan gate: it proves the user explicitly asked for a new TAG or a reuse-only
//! setup before later writers can touch country tags, country definitions,
//! history, states, OOB, or flags.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_country_setup_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let target_root = resolve_mod_root(&mod_root)?.root;
    let tag = require_value(&map, "tag")?.to_ascii_uppercase();
    let create_new = map.flags.contains("create-new-tag")
        || map.flags.contains("new-country")
        || map.flags.contains("new-start-country");
    let reuse_existing = map.flags.contains("reuse-existing");
    let parent_roots = repeated_values(&map, "mod-path")
        .into_iter()
        .map(|path| resolve_mod_root(&normalize_path(path)?).map(|resolved| resolved.root))
        .collect::<Result<Vec<_>, String>>()?;
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    let roots = country_setup_roots(&target_root, &parent_roots, game_root.as_deref());

    let tag_matches = find_tag_matches(&roots, &tag)?;
    let country_file_matches = find_country_file_matches(&roots, &tag)?;
    let history_matches = find_history_country_matches(&roots, &tag)?;
    let flag_status = flag_triplet_status(&target_root, &tag);
    let mut blockers = Vec::new();
    let mut operations = Vec::new();
    if !create_new && !reuse_existing {
        blockers.push(
            "country setup needs explicit --create-new-tag or --reuse-existing authorization"
                .to_string(),
        );
    }
    if create_new && reuse_existing {
        blockers.push("--create-new-tag and --reuse-existing are mutually exclusive".to_string());
    }
    if create_new {
        if !tag_matches.is_empty() {
            blockers.push(format!("tag `{tag}` already exists in indexed roots"));
        }
        if !country_file_matches.is_empty() {
            blockers.push(format!(
                "country definition file for `{tag}` already exists"
            ));
        }
        if !history_matches.is_empty() {
            blockers.push(format!("history country file for `{tag}` already exists"));
        }
        if !flag_status.iter().all(|(_, exists)| *exists) {
            blockers.push(format!("flag triplet for `{tag}` is incomplete"));
        }
        operations.extend([
            format!("reserve new country tag `{tag}`"),
            format!("create common/country_tags entry for `{tag}`"),
            format!("create common/countries definition for `{tag}`"),
            format!("create history/countries file for `{tag}`"),
            "produce state ownership/controller impact list before any state write".to_string(),
            "choose character system from parent-mod style before leader write".to_string(),
        ]);
    }
    if reuse_existing {
        if tag_matches.is_empty() {
            blockers.push(format!(
                "cannot reuse `{tag}` because no indexed tag definition exists"
            ));
        }
        operations.push(format!(
            "reuse existing `{tag}`; forbid writes to common/country_tags, common/countries, history/countries unless user separately authorizes new country setup"
        ));
    }
    let ok = blockers.is_empty();
    let json = country_setup_plan_json(CountrySetupReport {
        ok,
        target_root: &target_root,
        tag: &tag,
        create_new,
        reuse_existing,
        tag_matches: &tag_matches,
        country_file_matches: &country_file_matches,
        history_matches: &history_matches,
        flag_status: &flag_status,
        operations: &operations,
        blockers: &blockers,
    });
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_country_setup_apply(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let plan = read_utf8_lossy(&input)?;
    let mut blockers = Vec::new();
    if !map.flags.contains("execute") {
        blockers.push("country-setup-apply requires --execute".to_string());
    }
    if !map.flags.contains("final-check") {
        blockers.push("country-setup-apply requires --final-check".to_string());
    }
    if !plan.contains("\"schema\": \"hoi4skill.country_setup_plan.v1\"") {
        blockers.push("input is not a country-setup-plan report".to_string());
    }
    if !plan.contains("\"ok\": true") {
        blockers.push("input country setup plan is not ok".to_string());
    }
    if !plan.contains("\"create_new_tag\": true") {
        blockers.push(
            "country-setup-apply only writes explicitly authorized new TAG plans".to_string(),
        );
    }
    if plan.contains("\"reuse_existing\": true") {
        blockers.push("reuse-existing plans must not write common/country_tags, common/countries, or history/countries".to_string());
    }
    let target_root = json_string_field(&plan, "target_root")
        .map(|path| normalize_path(&path))
        .transpose()?;
    let tag = json_string_field(&plan, "tag")
        .unwrap_or_default()
        .to_ascii_uppercase();
    if target_root.is_none() {
        blockers.push("input plan is missing target_root".to_string());
    }
    if tag.len() != 3 || !tag.chars().all(|ch| ch.is_ascii_uppercase()) {
        blockers.push(format!(
            "invalid country tag `{tag}`; expected three uppercase ASCII letters"
        ));
    }
    let capital = value(&map, "capital").unwrap_or("1");
    if !capital.chars().all(|ch| ch.is_ascii_digit()) {
        blockers.push(format!(
            "invalid --capital `{capital}`; expected numeric state id"
        ));
    }
    let mut write_plan = Vec::new();
    if let Some(target_root) = target_root.as_ref() {
        let flag_status = flag_triplet_status(target_root, &tag);
        if !flag_status.iter().all(|(_, exists)| *exists) {
            blockers.push(format!("flag triplet for `{tag}` is incomplete"));
        }
        for (relative, content) in country_setup_skeleton_files(&tag, capital) {
            let path = target_root.join(Path::new(&relative));
            if path.exists() {
                blockers.push(format!(
                    "transaction target already exists and will not be overwritten: {}",
                    path.display()
                ));
            }
            write_plan.push((relative, path, content));
        }
    }

    let mut changed_files = Vec::new();
    let mut rollback_blockers = Vec::new();
    if blockers.is_empty() {
        match write_country_setup_transaction(&write_plan) {
            Ok(changed) => changed_files = changed,
            Err((err, changed)) => {
                rollback_blockers.push(err);
                rollback_blockers.extend(rollback_country_setup_files(&changed));
                blockers.push(
                    "country setup transaction failed and rollback was attempted".to_string(),
                );
                changed_files = changed
                    .into_iter()
                    .map(|path| path.display().to_string())
                    .collect();
            }
        }
    }

    let ok = blockers.is_empty();
    let report = country_setup_apply_json(
        &input,
        ok,
        &tag,
        capital,
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

fn country_setup_roots(
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

fn find_tag_matches(roots: &[(&'static str, PathBuf)], tag: &str) -> Result<Vec<String>, String> {
    let needle = format!("{tag} =");
    find_text_matches(roots, &["common/country_tags"], &needle)
}

fn find_country_file_matches(
    roots: &[(&'static str, PathBuf)],
    tag: &str,
) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for (role, root) in roots {
        let dir = root.join("common").join("countries");
        if !dir.is_dir() {
            continue;
        }
        for file in collect_files(&dir)? {
            if file
                .file_stem()
                .and_then(OsStr::to_str)
                .is_some_and(|stem| stem.eq_ignore_ascii_case(tag) || stem.starts_with(tag))
            {
                out.push(format!("{role}:{}", relative_slash_path(root, &file)));
            }
        }
    }
    Ok(out)
}

fn find_history_country_matches(
    roots: &[(&'static str, PathBuf)],
    tag: &str,
) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for (role, root) in roots {
        let dir = root.join("history").join("countries");
        if !dir.is_dir() {
            continue;
        }
        for file in collect_files(&dir)? {
            if file
                .file_name()
                .and_then(OsStr::to_str)
                .is_some_and(|name| name.to_ascii_uppercase().starts_with(tag))
            {
                out.push(format!("{role}:{}", relative_slash_path(root, &file)));
            }
        }
    }
    Ok(out)
}

fn find_text_matches(
    roots: &[(&'static str, PathBuf)],
    dirs: &[&str],
    needle: &str,
) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for (role, root) in roots {
        for dir in dirs {
            let dir = root.join(dir);
            if !dir.is_dir() {
                continue;
            }
            for file in collect_files(&dir)? {
                if file.extension().and_then(OsStr::to_str) != Some("txt") {
                    continue;
                }
                if read_utf8_lossy(&file)?.contains(needle) {
                    out.push(format!("{role}:{}", relative_slash_path(root, &file)));
                }
            }
        }
    }
    Ok(out)
}

fn flag_triplet_status(root: &Path, tag: &str) -> Vec<(String, bool)> {
    [
        format!("gfx/flags/{tag}.tga"),
        format!("gfx/flags/medium/{tag}.tga"),
        format!("gfx/flags/small/{tag}.tga"),
    ]
    .into_iter()
    .map(|relative| {
        let exists = root.join(&relative).is_file();
        (relative, exists)
    })
    .collect()
}

struct CountrySetupReport<'a> {
    ok: bool,
    target_root: &'a Path,
    tag: &'a str,
    create_new: bool,
    reuse_existing: bool,
    tag_matches: &'a [String],
    country_file_matches: &'a [String],
    history_matches: &'a [String],
    flag_status: &'a [(String, bool)],
    operations: &'a [String],
    blockers: &'a [String],
}

fn country_setup_plan_json(report: CountrySetupReport<'_>) -> String {
    let mut map = BTreeMap::new();
    map.insert(
        "schema".to_string(),
        json_str("hoi4skill.country_setup_plan.v1"),
    );
    map.insert("ok".to_string(), json_bool(report.ok).to_string());
    map.insert(
        "status".to_string(),
        json_str(if report.ok {
            "country_setup_plan_ready"
        } else {
            "country_setup_plan_blocked"
        }),
    );
    map.insert("tag".to_string(), json_str(report.tag));
    map.insert(
        "target_root".to_string(),
        json_str(&report.target_root.display().to_string()),
    );
    map.insert(
        "create_new_tag".to_string(),
        json_bool(report.create_new).to_string(),
    );
    map.insert(
        "reuse_existing".to_string(),
        json_bool(report.reuse_existing).to_string(),
    );
    map.insert("tag_matches".to_string(), json_array(report.tag_matches));
    map.insert(
        "country_file_matches".to_string(),
        json_array(report.country_file_matches),
    );
    map.insert(
        "history_country_matches".to_string(),
        json_array(report.history_matches),
    );
    map.insert(
        "flag_triplet".to_string(),
        flag_status_json(report.flag_status),
    );
    map.insert("operations".to_string(), json_array(report.operations));
    map.insert(
        "rules".to_string(),
        json_array(&[
            "new country setup requires explicit --create-new-tag".to_string(),
            "reuse existing tag forbids country tag/country/history writes".to_string(),
            "state ownership changes need original owner/controller impact list".to_string(),
            "leader system must follow parent-mod character style".to_string(),
        ]),
    );
    map.insert("blockers".to_string(), json_array(report.blockers));
    json_raw_object(&map) + "\n"
}

fn flag_status_json(values: &[(String, bool)]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|(path, exists)| format!(
                "{{\"path\": {}, \"exists\": {}}}",
                json_str(path),
                json_bool(*exists)
            ))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn country_setup_skeleton_files(tag: &str, capital: &str) -> Vec<(String, String)> {
    vec![
        (
            format!("common/country_tags/zzz_hoi4skill_{tag}.txt"),
            format!("{tag} = \"countries/{tag} - Generated.txt\"\n"),
        ),
        (
            format!("common/countries/{tag} - Generated.txt"),
            "# Generated by hoi4skill P19 country-setup-apply.\ncolor = { 120 120 120 }\ncolor_ui = { 120 120 120 }\ngraphical_culture = western_european_gfx\ngraphical_culture_2d = western_european_2d\n".to_string(),
        ),
        (
            format!("history/countries/{tag} - Generated.txt"),
            format!("# Generated by hoi4skill P19 country-setup-apply.\ncapital = {capital}\nset_politics = {{\n\truling_party = neutrality\n\telections_allowed = no\n}}\nset_popularities = {{\n\tneutrality = 100\n}}\n"),
        ),
        (
            format!("localisation/simp_chinese/hoi4skill_{tag}_countries_l_simp_chinese.yml"),
            format!("\u{feff}l_simp_chinese:\n {tag}:0 \"{tag}\"\n {tag}_DEF:0 \"{tag}\"\n {tag}_ADJ:0 \"{tag}\"\n"),
        ),
    ]
}

fn write_country_setup_transaction(
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

fn rollback_country_setup_files(changed: &[PathBuf]) -> Vec<String> {
    let mut blockers = Vec::new();
    for path in changed.iter().rev() {
        if let Err(err) = fs::remove_file(path) {
            blockers.push(format!("rollback remove {}: {err}", path.display()));
        }
    }
    blockers
}

fn country_setup_apply_json(
    input: &Path,
    ok: bool,
    tag: &str,
    capital: &str,
    changed_files: &[String],
    blockers: &[String],
    rollback_blockers: &[String],
) -> String {
    let mut map = BTreeMap::new();
    map.insert(
        "schema".to_string(),
        json_str("hoi4skill.country_setup_apply.v1"),
    );
    map.insert("ok".to_string(), json_bool(ok).to_string());
    map.insert(
        "status".to_string(),
        json_str(if ok {
            "country_setup_applied"
        } else {
            "country_setup_apply_blocked"
        }),
    );
    map.insert("input".to_string(), json_str(&input.display().to_string()));
    map.insert("tag".to_string(), json_str(tag));
    map.insert("capital".to_string(), json_str(capital));
    map.insert(
        "transaction".to_string(),
        json_str(if ok {
            "committed_country_skeleton_files"
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
        json_str("run hoi4skill validate <mod> --game-root <hoi4> --strict-code-index after country skeleton review"),
    );
    json_raw_object(&map) + "\n"
}
