//! HOI4 installation and Steam library discovery.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_detect_hoi4_path(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mut explicit = Vec::new();
    if let Some(path) = value(&map, "hoi4-path").or_else(|| value(&map, "game-root")) {
        explicit.push(normalize_path(path)?);
    }
    for path in &map.positionals {
        explicit.push(normalize_path(path)?);
    }
    let candidates = detect_hoi4_path_candidates(&explicit, &default_steam_roots());
    let json = detect_hoi4_path_json(&candidates);
    write_or_print(&json, value(&map, "output"))
}

#[derive(Clone)]
pub(crate) struct Hoi4PathCandidate {
    pub(crate) path: PathBuf,
    pub(crate) sources: Vec<String>,
    pub(crate) exists: bool,
    pub(crate) valid: bool,
    pub(crate) has_exe: bool,
    pub(crate) has_common: bool,
    pub(crate) has_localisation: bool,
}

pub(crate) fn detect_hoi4_path_candidates(
    explicit_paths: &[PathBuf],
    steam_roots: &[PathBuf],
) -> Vec<Hoi4PathCandidate> {
    let mut raw = Vec::new();
    for path in explicit_paths {
        raw.push((path.clone(), "explicit".to_string()));
    }
    if let Ok(path) = env::var("HOI4_PATH") {
        if !path.trim().is_empty() {
            raw.push((PathBuf::from(path), "HOI4_PATH".to_string()));
        }
    }
    for steam_root in steam_roots {
        for library in steam_libraries_from_root(steam_root) {
            raw.push((
                library
                    .join("steamapps")
                    .join("common")
                    .join("Hearts of Iron IV"),
                format!("steam library {}", steam_root.display()),
            ));
        }
    }
    merge_hoi4_path_candidates(raw)
}

pub(crate) fn default_steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for var in ["STEAM_DIR", "ProgramFiles(x86)", "ProgramFiles"] {
        if let Ok(value) = env::var(var) {
            if !value.trim().is_empty() {
                let base = PathBuf::from(value);
                roots.push(if var.starts_with("ProgramFiles") {
                    base.join("Steam")
                } else {
                    base
                });
            }
        }
    }
    roots.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
    roots.push(PathBuf::from(r"C:\Program Files\Steam"));
    for drive in b'C'..=b'Z' {
        let root = format!("{}:\\", drive as char);
        roots.push(PathBuf::from(&root).join("Steam"));
        roots.push(PathBuf::from(&root).join("SteamLibrary"));
    }
    dedupe_paths(roots)
}

pub(crate) fn steam_libraries_from_root(steam_root: &Path) -> Vec<PathBuf> {
    let mut libraries = vec![steam_root.to_path_buf()];
    let vdf = steam_root.join("steamapps").join("libraryfolders.vdf");
    if let Ok(text) = read_utf8_lossy(&vdf) {
        for path in parse_steam_libraryfolders_vdf(&text) {
            libraries.push(path);
        }
    }
    dedupe_paths(libraries)
}

pub(crate) fn parse_steam_libraryfolders_vdf(text: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for line in text.lines() {
        let Some((key, value)) = parse_vdf_quoted_pair(line.trim()) else {
            continue;
        };
        if key == "path"
            || (key.chars().all(|ch| ch.is_ascii_digit()) && looks_like_vdf_library_path(&value))
        {
            paths.push(PathBuf::from(unescape_vdf_path(&value)));
        }
    }
    dedupe_paths(paths)
}

pub(crate) fn looks_like_vdf_library_path(value: &str) -> bool {
    value.contains(":\\")
        || value.contains(":/")
        || value.starts_with('/')
        || value.starts_with("\\\\")
}

pub(crate) fn parse_vdf_quoted_pair(line: &str) -> Option<(String, String)> {
    let mut values = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find('"') {
        let after = &rest[start + 1..];
        let mut escape = false;
        let mut end = None;
        for (idx, ch) in after.char_indices() {
            if ch == '"' && !escape {
                end = Some(idx);
                break;
            }
            escape = ch == '\\' && !escape;
            if ch != '\\' {
                escape = false;
            }
        }
        let end = end?;
        values.push(after[..end].to_string());
        rest = &after[end + 1..];
        if values.len() == 2 {
            break;
        }
    }
    if values.len() == 2 {
        Some((values.remove(0), values.remove(0)))
    } else {
        None
    }
}

pub(crate) fn unescape_vdf_path(path: &str) -> String {
    path.replace("\\\\", "\\").replace("\\/", "/")
}

pub(crate) fn merge_hoi4_path_candidates(raw: Vec<(PathBuf, String)>) -> Vec<Hoi4PathCandidate> {
    let mut grouped: BTreeMap<String, (PathBuf, BTreeSet<String>)> = BTreeMap::new();
    for (path, source) in raw {
        let key = slash_path(&path).to_ascii_lowercase();
        grouped
            .entry(key)
            .or_insert_with(|| (path, BTreeSet::new()))
            .1
            .insert(source);
    }
    grouped
        .into_values()
        .map(|(path, sources)| classify_hoi4_path_candidate(path, sources.into_iter().collect()))
        .collect()
}

pub(crate) fn classify_hoi4_path_candidate(
    path: PathBuf,
    sources: Vec<String>,
) -> Hoi4PathCandidate {
    let exists = path.is_dir();
    let has_exe = path.join("hoi4.exe").exists();
    let has_common = path.join("common").is_dir();
    let has_localisation = path.join("localisation").is_dir();
    let valid = exists && (has_exe || (has_common && has_localisation));
    Hoi4PathCandidate {
        path,
        sources,
        exists,
        valid,
        has_exe,
        has_common,
        has_localisation,
    }
}

pub(crate) fn detect_hoi4_path_json(candidates: &[Hoi4PathCandidate]) -> String {
    let selected = candidates.iter().find(|candidate| candidate.valid);
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!(
        "  \"selected\": {},\n",
        selected
            .map(|candidate| json_str(&candidate.path.display().to_string()))
            .unwrap_or_else(|| "null".to_string())
    ));
    out.push_str("  \"candidates\": [\n");
    for (idx, candidate) in candidates.iter().enumerate() {
        comma(&mut out, idx, "    ");
        out.push_str(&format!(
            "{{\"path\": {}, \"sources\": {}, \"exists\": {}, \"valid\": {}, \"has_exe\": {}, \"has_common\": {}, \"has_localisation\": {}}}",
            json_str(&candidate.path.display().to_string()),
            json_array(&candidate.sources),
            json_bool(candidate.exists),
            json_bool(candidate.valid),
            json_bool(candidate.has_exe),
            json_bool(candidate.has_common),
            json_bool(candidate.has_localisation)
        ));
    }
    out.push_str("\n  ]\n}\n");
    out
}

pub(crate) fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for path in paths {
        let key = slash_path(&path).to_ascii_lowercase();
        if seen.insert(key) {
            out.push(path);
        }
    }
    out
}
