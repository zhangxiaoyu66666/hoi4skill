//! Detect and remove duplicate installed copies of this Agent Skill.

#[allow(unused_imports)]
use crate::*;

const SKILL_NAME: &str = "hoi4-mod-maker";
const MAX_SCAN_DEPTH: usize = 6;

pub(crate) fn cmd_doctor_skill_install(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let roots = if map.value_lists.contains_key("root") {
        repeated_values(&map, "root")
            .into_iter()
            .map(normalize_path)
            .collect::<Result<Vec<_>, _>>()?
    } else {
        default_skill_roots()?
    };
    let candidates = find_installed_skill_copies(&roots)?;
    let keep = value(&map, "keep")
        .map(normalize_path)
        .transpose()?
        .or_else(infer_running_skill_root);
    let fix = map.flags.contains("fix");
    let report = repair_installed_skill_copies(&candidates, keep.as_deref(), fix)?;

    println!("HOI4 skill install doctor");
    println!("  found: {}", report.found.len());
    if let Some(keep) = &report.kept {
        println!("  keep: {}", keep.display());
    }
    for path in &report.removed {
        println!("  removed: {}", path.display());
    }
    for path in &report.duplicates {
        println!("  duplicate: {}", path.display());
    }

    if !report.duplicates.is_empty() {
        return Err(
            "duplicate hoi4-mod-maker skills found; rerun the bundled command with --fix"
                .to_string(),
        );
    }
    Ok(())
}

#[derive(Debug)]
pub(crate) struct SkillInstallRepair {
    pub(crate) found: Vec<PathBuf>,
    pub(crate) kept: Option<PathBuf>,
    pub(crate) removed: Vec<PathBuf>,
    pub(crate) duplicates: Vec<PathBuf>,
}

pub(crate) fn repair_installed_skill_copies(
    candidates: &[PathBuf],
    keep: Option<&Path>,
    fix: bool,
) -> Result<SkillInstallRepair, String> {
    let mut found = dedup_paths(candidates);
    found.sort();
    if found.len() <= 1 {
        return Ok(SkillInstallRepair {
            kept: found.first().cloned(),
            found,
            removed: Vec::new(),
            duplicates: Vec::new(),
        });
    }

    let keep = keep.ok_or_else(|| {
        "multiple hoi4-mod-maker copies found, but the current skill directory could not be inferred; run the bundled binary or pass --keep <current-skill-dir>"
            .to_string()
    })?;
    let keep = canonical_or_absolute(keep)?;
    let Some(keep) = found
        .iter()
        .find(|candidate| paths_equal(candidate, &keep))
        .cloned()
    else {
        return Err(format!(
            "keep path {} is not one of the discovered hoi4-mod-maker skill directories",
            keep.display()
        ));
    };
    let duplicates = found
        .iter()
        .filter(|candidate| !paths_equal(candidate, &keep))
        .cloned()
        .collect::<Vec<_>>();

    if !fix {
        return Ok(SkillInstallRepair {
            found,
            kept: Some(keep),
            removed: Vec::new(),
            duplicates,
        });
    }

    let mut removed = Vec::new();
    for duplicate in duplicates {
        verify_removable_skill_copy(&duplicate, &keep)?;
        fs::remove_dir_all(&duplicate)
            .map_err(|e| format!("remove duplicate skill {}: {e}", duplicate.display()))?;
        removed.push(duplicate);
    }
    Ok(SkillInstallRepair {
        found,
        kept: Some(keep),
        removed,
        duplicates: Vec::new(),
    })
}

pub(crate) fn find_installed_skill_copies(roots: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        scan_skill_root(root, root, 0, &mut out)?;
    }
    Ok(dedup_paths(&out))
}

fn scan_skill_root(
    scan_root: &Path,
    directory: &Path,
    depth: usize,
    out: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if depth > MAX_SCAN_DEPTH {
        return Ok(());
    }
    let skill_file = directory.join("SKILL.md");
    if skill_file.is_file() && skill_frontmatter_name(&skill_file)?.as_deref() == Some(SKILL_NAME) {
        let candidate = canonical_or_absolute(directory)?;
        let canonical_root = canonical_or_absolute(scan_root)?;
        if candidate.starts_with(&canonical_root) && candidate != canonical_root {
            out.push(candidate);
            return Ok(());
        }
    }
    for entry in fs::read_dir(directory)
        .map_err(|e| format!("read skill directory {}: {e}", directory.display()))?
    {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            scan_skill_root(scan_root, &path, depth + 1, out)?;
        }
    }
    Ok(())
}

fn skill_frontmatter_name(path: &Path) -> Result<Option<String>, String> {
    let text = read_utf8_lossy(path)?;
    if !text.starts_with("---") {
        return Ok(None);
    }
    for line in text.lines().skip(1) {
        let line = line.trim();
        if line == "---" {
            break;
        }
        if let Some(name) = line.strip_prefix("name:") {
            return Ok(Some(name.trim().trim_matches('"').to_string()));
        }
    }
    Ok(None)
}

fn default_skill_roots() -> Result<Vec<PathBuf>, String> {
    let mut roots = Vec::new();
    if let Some(home) = user_home_dir() {
        roots.extend([
            home.join(".codex/skills"),
            home.join(".claude/skills"),
            home.join(".config/opencode/skills"),
            home.join(".agents/skills"),
        ]);
    }
    let cwd = env::current_dir().map_err(|e| e.to_string())?;
    roots.extend([
        cwd.join(".codex/skills"),
        cwd.join(".claude/skills"),
        cwd.join(".opencode/skills"),
        cwd.join(".agents/skills"),
    ]);
    Ok(dedup_paths(&roots))
}

fn user_home_dir() -> Option<PathBuf> {
    env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
}

fn infer_running_skill_root() -> Option<PathBuf> {
    let executable = env::current_exe().ok()?;
    executable.ancestors().find_map(|ancestor| {
        let skill = ancestor.join("SKILL.md");
        if skill.is_file()
            && skill_frontmatter_name(&skill).ok().flatten().as_deref() == Some(SKILL_NAME)
        {
            canonical_or_absolute(ancestor).ok()
        } else {
            None
        }
    })
}

fn verify_removable_skill_copy(path: &Path, keep: &Path) -> Result<(), String> {
    let path = canonical_or_absolute(path)?;
    let keep = canonical_or_absolute(keep)?;
    if paths_equal(&path, &keep) || keep.starts_with(&path) || path.starts_with(&keep) {
        return Err(format!(
            "refusing to remove overlapping keep path {}",
            path.display()
        ));
    }
    let skill = path.join("SKILL.md");
    if !skill.is_file() || skill_frontmatter_name(&skill)?.as_deref() != Some(SKILL_NAME) {
        return Err(format!(
            "refusing to remove {} because it is not a verified {SKILL_NAME} skill directory",
            path.display()
        ));
    }
    Ok(())
}

fn canonical_or_absolute(path: &Path) -> Result<PathBuf, String> {
    if path.exists() {
        path.canonicalize()
            .map_err(|e| format!("canonicalize {}: {e}", path.display()))
    } else if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(env::current_dir().map_err(|e| e.to_string())?.join(path))
    }
}

fn dedup_paths(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for path in paths {
        if !out.iter().any(|existing| paths_equal(existing, path)) {
            out.push(path.clone());
        }
    }
    out
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}
