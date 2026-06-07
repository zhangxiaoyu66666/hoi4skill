//! Shared scanners for descriptors, focus trees, localisation, countries, and history files.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn scan_descriptor_metadata(text: &str) -> BTreeMap<String, String> {
    let mut metadata = BTreeMap::new();
    for key in [
        "name",
        "version",
        "supported_version",
        "remote_file_id",
        "path",
    ] {
        if let Some(value) = descriptor_scalar_value(text, key) {
            metadata.insert(key.to_string(), value);
        }
    }
    metadata
}

pub(crate) fn descriptor_scalar_value(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim_start_matches('\u{feff}').trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(value) = find_assignment_in_text(trimmed, key) {
            return Some(value.to_string());
        }
    }
    None
}

pub(crate) fn descriptor_list_values(text: &str, key: &str) -> Vec<String> {
    let mut values = Vec::new();
    for block in blocks_named(text, key) {
        values.extend(quoted_values(&block));
    }
    values.sort();
    values.dedup();
    values
}

pub(crate) fn quoted_values(text: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut rest = text;
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
        let Some(end) = end else {
            break;
        };
        values.push(after[..end].to_string());
        rest = &after[end + 1..];
    }
    values
}

pub(crate) fn find_launcher_mod_files(root: &Path) -> Result<Vec<String>, String> {
    let Some(parent) = root.parent() else {
        return Ok(Vec::new());
    };
    let root_key = slash_path(root).to_ascii_lowercase();
    let mut out = Vec::new();
    for entry in fs::read_dir(parent).map_err(|e| format!("read dir {}: {e}", parent.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(OsStr::to_str).unwrap_or("") != "mod" {
            continue;
        }
        let Ok(text) = read_utf8_lossy(&path) else {
            continue;
        };
        let Some(mod_path) = descriptor_scalar_value(&text, "path") else {
            continue;
        };
        let candidate = PathBuf::from(mod_path.replace('/', "\\"));
        if slash_path(&candidate).to_ascii_lowercase() == root_key {
            out.push(path.display().to_string());
        }
    }
    out.sort();
    Ok(out)
}

pub(crate) fn scan_top_level_entries(root: &Path) -> Result<BTreeMap<String, String>, String> {
    let mut entries = BTreeMap::new();
    for entry in fs::read_dir(root).map_err(|e| format!("read dir {}: {e}", root.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let kind = if path.is_dir() { "dir" } else { "file" };
        entries.insert(name, kind.to_string());
    }
    Ok(entries)
}

pub(crate) fn scan_common_modules(root: &Path) -> Result<BTreeMap<String, i64>, String> {
    let mut modules = BTreeMap::new();
    let common = root.join("common");
    if !common.exists() {
        return Ok(modules);
    }
    for entry in fs::read_dir(&common).map_err(|e| format!("read dir {}: {e}", common.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let count = if path.is_dir() {
            collect_files(&path)?.len()
        } else if path.is_file() {
            1
        } else {
            0
        };
        modules.insert(name, count as i64);
    }
    Ok(modules)
}

pub(crate) fn scan_extension_counts(root: &Path, files: &[PathBuf]) -> BTreeMap<String, i64> {
    let mut counts = BTreeMap::new();
    for file in files {
        let ext = file
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("<none>")
            .to_ascii_lowercase();
        *counts.entry(ext).or_default() += 1;
        let rel = rel_slash(root, file);
        if rel.contains('/') {
            continue;
        }
    }
    counts
}

pub(crate) fn scan_focus_tree_styles(root: &Path) -> Result<Vec<FocusTreeStyle>, String> {
    let focus_root = root.join("common").join("national_focus");
    let mut out = Vec::new();
    if !focus_root.exists() {
        return Ok(out);
    }
    for file in collect_files(&focus_root)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        let rel = rel_slash(root, &file);
        let trees = blocks_named(&text, "focus_tree");
        if trees.is_empty() {
            let focus_count = blocks_named(&text, "focus").len();
            if focus_count > 0 {
                out.push(FocusTreeStyle {
                    file: rel,
                    tree_id: String::new(),
                    country_tag: String::new(),
                    focus_count,
                });
            }
            continue;
        }
        for tree in trees {
            let tree_id = block_assignment(&tree, "id").unwrap_or_default();
            let country_tag = blocks_named(&tree, "country")
                .first()
                .and_then(|block| block_assignment(block, "tag"))
                .unwrap_or_default();
            out.push(FocusTreeStyle {
                file: rel.clone(),
                tree_id,
                country_tag,
                focus_count: blocks_named(&tree, "focus").len(),
            });
        }
    }
    out.sort_by(|a, b| a.file.cmp(&b.file).then(a.tree_id.cmp(&b.tree_id)));
    Ok(out)
}

pub(crate) fn scan_focus_id_prefixes(root: &Path) -> Result<BTreeMap<String, i64>, String> {
    let mut counts = BTreeMap::new();
    let focus_root = root.join("common").join("national_focus");
    if !focus_root.exists() {
        return Ok(counts);
    }
    for file in collect_files(&focus_root)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        for block in blocks_named(&text, "focus") {
            if let Some(id) = block_assignment(&block, "id") {
                *counts.entry(id_prefix(&id)).or_default() += 1;
            }
        }
    }
    Ok(counts)
}

pub(crate) fn id_prefix(id: &str) -> String {
    for sep in ['_', '.'] {
        if let Some(idx) = id.find(sep) {
            return id[..=idx].to_string();
        }
    }
    id.chars()
        .take_while(|ch| ch.is_ascii_uppercase())
        .collect::<String>()
}

pub(crate) fn scan_focus_icon_counts(root: &Path) -> Result<BTreeMap<String, i64>, String> {
    let mut counts = BTreeMap::new();
    let focus_root = root.join("common").join("national_focus");
    if !focus_root.exists() {
        return Ok(counts);
    }
    for file in collect_files(&focus_root)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        for block in blocks_named(&text, "focus") {
            if let Some(icon) = block_assignment(&block, "icon") {
                *counts.entry(icon).or_default() += 1;
            }
        }
    }
    Ok(counts)
}

pub(crate) fn scan_event_namespace_styles(
    root: &Path,
) -> Result<BTreeMap<String, EventNamespaceStats>, String> {
    let mut namespaces: BTreeMap<String, EventNamespaceStats> = BTreeMap::new();
    let events = root.join("events");
    if !events.exists() {
        return Ok(namespaces);
    }
    for file in collect_files(&events)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        let rel = rel_slash(root, &file);
        for line in text.lines() {
            if let Some(namespace) = assignment_value(line.trim(), "add_namespace") {
                namespaces
                    .entry(namespace.to_string())
                    .or_default()
                    .files
                    .insert(rel.clone());
            }
        }
        scan_event_kind_namespace_counts(&text, &rel, "country_event", &mut namespaces);
        scan_event_kind_namespace_counts(&text, &rel, "news_event", &mut namespaces);
        scan_event_kind_namespace_counts(&text, &rel, "state_event", &mut namespaces);
    }
    Ok(namespaces)
}

pub(crate) fn scan_event_kind_namespace_counts(
    text: &str,
    rel: &str,
    kind: &str,
    namespaces: &mut BTreeMap<String, EventNamespaceStats>,
) {
    for block in blocks_named(text, kind) {
        let Some(id) = block_assignment(&block, "id") else {
            continue;
        };
        let Some((namespace, number)) = event_id_namespace_number(&id) else {
            continue;
        };
        let stats = namespaces.entry(namespace).or_default();
        stats.files.insert(rel.to_string());
        match kind {
            "country_event" => stats.country_events += 1,
            "news_event" => stats.news_events += 1,
            "state_event" => stats.state_events += 1,
            _ => {}
        }
        stats.max_id = Some(stats.max_id.unwrap_or(number).max(number));
    }
}

pub(crate) fn event_id_namespace_number(id: &str) -> Option<(String, i64)> {
    let (namespace, number) = id.rsplit_once('.')?;
    let number = number.parse::<i64>().ok()?;
    Some((namespace.to_string(), number))
}

pub(crate) fn scan_localisation_file_styles(
    root: &Path,
) -> Result<Vec<LocalisationFileStyle>, String> {
    let loc = root.join("localisation");
    let mut out = Vec::new();
    if !loc.exists() {
        return Ok(out);
    }
    for file in collect_files(&loc)? {
        let ext = file
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext != "yml" && ext != "yaml" {
            continue;
        }
        let bytes = fs::read(&file).map_err(|e| format!("read {}: {e}", file.display()))?;
        let bom = bytes.starts_with(&[0xef, 0xbb, 0xbf]);
        let text = String::from_utf8_lossy(&bytes);
        let header = text
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("")
            .trim_start_matches('\u{feff}')
            .trim()
            .to_string();
        let mut key_count = 0usize;
        let mut colon_zero_count = 0usize;
        let mut loose_count = 0usize;
        for line in text.lines() {
            let trimmed = line.trim_start_matches('\u{feff}').trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if trimmed.starts_with("l_") && trimmed.ends_with(':') {
                continue;
            }
            let Some((_, rest)) = trimmed.split_once(':') else {
                continue;
            };
            key_count += 1;
            if rest.trim_start().starts_with('0') {
                colon_zero_count += 1;
            } else {
                loose_count += 1;
            }
        }
        out.push(LocalisationFileStyle {
            file: rel_slash(root, &file),
            header,
            bom,
            key_count,
            colon_zero_count,
            loose_count,
        });
    }
    out.sort_by(|a, b| a.file.cmp(&b.file));
    Ok(out)
}

pub(crate) fn localisation_language_counts(
    files: &[LocalisationFileStyle],
) -> BTreeMap<String, i64> {
    let mut counts = BTreeMap::new();
    for file in files {
        if !file.header.is_empty() {
            *counts.entry(file.header.clone()).or_default() += 1;
        }
    }
    counts
}

pub(crate) fn scan_sprite_index(
    root: &Path,
    max_sprites: usize,
) -> Result<BTreeMap<String, String>, String> {
    let mut sprites = BTreeMap::new();
    for sprite in scan_sprites(root)?.into_iter().take(max_sprites) {
        if !sprite.name.is_empty() {
            sprites.insert(sprite.name, sprite.texturefile);
        }
    }
    Ok(sprites)
}

pub(crate) fn scan_assignment_counts_in_dir(
    root: &Path,
    rel_dir: &str,
    key: &str,
) -> Result<BTreeMap<String, i64>, String> {
    let mut counts = BTreeMap::new();
    let dir = root.join(rel_dir.replace('/', "\\"));
    if !dir.exists() {
        return Ok(counts);
    }
    for file in collect_files(&dir)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        for value in assignment_values_in_text(&strip_comments(&text), key) {
            *counts.entry(value).or_default() += 1;
        }
    }
    Ok(counts)
}

pub(crate) fn scan_decision_categories(root: &Path) -> Result<Vec<String>, String> {
    let mut categories = BTreeSet::new();
    let dir = root.join("common").join("decisions").join("categories");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    for file in collect_files(&dir)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        for key in direct_block_keys(&strip_comments(&text)) {
            if is_identifier_like(&key) && !is_common_definition_field(&key) {
                categories.insert(key);
            }
        }
    }
    Ok(categories.into_iter().collect())
}

pub(crate) fn scan_country_tag_styles(root: &Path) -> Result<Vec<String>, String> {
    let mut tags = BTreeSet::new();
    let dir = root.join("common").join("country_tags");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    for file in collect_files(&dir)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        for line in strip_comments(&text).lines() {
            if let Some(key) = assignment_key(line) {
                if looks_like_tag(key) {
                    tags.insert(key.to_string());
                }
            }
        }
    }
    Ok(tags.into_iter().collect())
}

pub(crate) fn scan_history_country_styles(root: &Path) -> Result<Vec<String>, String> {
    let mut countries = BTreeSet::new();
    let dir = root.join("history").join("countries");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    for file in collect_files(&dir)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let stem = file.file_stem().and_then(OsStr::to_str).unwrap_or("");
        let tag = stem.split([' ', '-']).next().unwrap_or("").trim();
        if looks_like_tag(tag) {
            countries.insert(tag.to_string());
        }
    }
    Ok(countries.into_iter().collect())
}

pub(crate) fn scan_country_tag_mappings(root: &Path) -> Result<Vec<CountryTagMapping>, String> {
    let mut mappings = Vec::new();
    for file in txt_files(root, "common/country_tags")? {
        let rel = rel_slash(root, &file);
        let text = strip_comments(&read_utf8_lossy(&file)?);
        for line in text.lines() {
            let Some(tag) = assignment_key(line) else {
                continue;
            };
            if !looks_like_tag(tag) {
                continue;
            }
            if let Some(country_file) = assignment_value(line.trim(), tag) {
                mappings.push(CountryTagMapping {
                    file: rel.clone(),
                    tag: tag.to_string(),
                    country_file: country_file.to_string(),
                });
            }
        }
    }
    mappings.sort_by(|a, b| a.tag.cmp(&b.tag).then(a.file.cmp(&b.file)));
    Ok(mappings)
}

pub(crate) fn scan_country_definition_files(root: &Path) -> Result<Vec<String>, String> {
    let mut files = txt_files(root, "common/countries")?
        .iter()
        .map(|file| rel_slash(root, file))
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

pub(crate) fn scan_country_leader_traits(root: &Path) -> Result<Vec<String>, String> {
    let mut traits = BTreeSet::new();
    for file in txt_files(root, "common/country_leader")? {
        let text = read_utf8_lossy(&file)?;
        collect_direct_entries_in_wrappers(
            &text,
            &mut traits,
            &["leader_traits", "country_leader_traits"],
        );
    }
    Ok(traits.into_iter().collect())
}

pub(crate) fn scan_character_styles(
    root: &Path,
    limit: usize,
) -> Result<Vec<CharacterStyle>, String> {
    let mut out = Vec::new();
    for file in txt_files(root, "common/characters")? {
        let rel = rel_slash(root, &file);
        let text = strip_comments(&read_utf8_lossy(&file)?);
        let wrappers = direct_blocks_named(&text, "characters");
        let roots = if wrappers.is_empty() {
            vec![text]
        } else {
            wrappers
        };
        for wrapper in roots {
            for (id, block) in direct_child_blocks(&wrapper) {
                if !is_identifier_like(&id) || is_common_definition_field(&id) {
                    continue;
                }
                let roles = character_roles(&block);
                if roles.is_empty() {
                    continue;
                }
                out.push(CharacterStyle {
                    file: rel.clone(),
                    id,
                    roles,
                    traits: traits_in_block(&block),
                });
            }
        }
    }
    out.sort_by(|a, b| a.file.cmp(&b.file).then(a.id.cmp(&b.id)));
    out.truncate(limit);
    Ok(out)
}

pub(crate) fn character_roles(block: &str) -> Vec<String> {
    let mut roles = Vec::new();
    for role in [
        "country_leader",
        "advisor",
        "corps_commander",
        "field_marshal",
        "navy_leader",
        "scientist",
        "unit_leader",
    ] {
        if !blocks_named(block, role).is_empty() {
            roles.push(role.to_string());
        }
    }
    roles
}

pub(crate) fn traits_in_block(block: &str) -> Vec<String> {
    let mut traits = BTreeSet::new();
    for traits_block in blocks_named(block, "traits") {
        for token in token_candidates(&traits_block) {
            if is_reference_identifier(token) {
                traits.insert(token.to_string());
            }
        }
    }
    traits.into_iter().collect()
}

pub(crate) fn scan_history_character_uses(
    root: &Path,
    limit: usize,
) -> Result<Vec<HistoryCharacterUse>, String> {
    let mut out = Vec::new();
    for file in txt_files(root, "history/countries")? {
        let rel = rel_slash(root, &file);
        let text = strip_comments(&read_utf8_lossy(&file)?);
        let mut recruited_characters = assignment_values_in_text(&text, "recruit_character");
        recruited_characters.sort();
        recruited_characters.dedup();
        let legacy_country_leaders = blocks_named(&text, "create_country_leader").len();
        if recruited_characters.is_empty() && legacy_country_leaders == 0 {
            continue;
        }
        out.push(HistoryCharacterUse {
            tag: history_country_tag_from_path(&file),
            file: rel,
            recruited_characters,
            legacy_country_leaders,
        });
    }
    out.sort_by(|a, b| a.file.cmp(&b.file));
    out.truncate(limit);
    Ok(out)
}

pub(crate) fn scan_legacy_country_leaders(
    root: &Path,
    limit: usize,
) -> Result<Vec<LegacyCountryLeaderStyle>, String> {
    let mut out = Vec::new();
    for file in txt_files(root, "history/countries")? {
        let rel = rel_slash(root, &file);
        let text = strip_comments(&read_utf8_lossy(&file)?);
        for block in blocks_named(&text, "create_country_leader") {
            out.push(LegacyCountryLeaderStyle {
                file: rel.clone(),
                name: block_assignment(&block, "name"),
                ideology: block_assignment(&block, "ideology"),
                picture: block_assignment(&block, "picture"),
                traits: traits_in_block(&block),
            });
        }
    }
    out.sort_by(|a, b| {
        a.file.cmp(&b.file).then(
            a.name
                .clone()
                .unwrap_or_default()
                .cmp(&b.name.clone().unwrap_or_default()),
        )
    });
    out.truncate(limit);
    Ok(out)
}

pub(crate) fn history_country_tag_from_path(path: &Path) -> String {
    let stem = path.file_stem().and_then(OsStr::to_str).unwrap_or("");
    let tag = stem.split([' ', '-']).next().unwrap_or("").trim();
    if looks_like_tag(tag) {
        tag.to_string()
    } else {
        String::new()
    }
}

pub(crate) fn make_country_creation_syntax_summary(
    root: &Path,
    country_tag_mappings: usize,
    country_definition_files: usize,
    country_leader_traits: usize,
    characters: usize,
    history_character_files: usize,
    legacy_country_leaders: usize,
) -> CountryCreationSyntaxSummary {
    CountryCreationSyntaxSummary {
        root: root.display().to_string(),
        leader_style: infer_leader_style(characters, legacy_country_leaders),
        country_tag_mappings,
        country_definition_files,
        country_leader_traits,
        characters,
        history_character_files,
        legacy_country_leaders,
    }
}

pub(crate) fn scan_country_creation_syntax_summary(
    root: &Path,
) -> Result<CountryCreationSyntaxSummary, String> {
    let country_tag_mappings = scan_country_tag_mappings(root)?;
    let country_definition_files = scan_country_definition_files(root)?;
    let country_leader_traits = scan_country_leader_traits(root)?;
    let characters = scan_character_styles(root, usize::MAX)?;
    let history_character_uses = scan_history_character_uses(root, usize::MAX)?;
    let legacy_country_leaders = scan_legacy_country_leaders(root, usize::MAX)?;
    Ok(make_country_creation_syntax_summary(
        root,
        country_tag_mappings.len(),
        country_definition_files.len(),
        country_leader_traits.len(),
        characters.len(),
        history_character_uses.len(),
        legacy_country_leaders.len(),
    ))
}

pub(crate) fn scan_dependency_country_creation_styles(
    dependency_roots: &[PathBuf],
) -> Result<Vec<CountryCreationSyntaxSummary>, String> {
    let mut out = Vec::new();
    for root in dependency_roots {
        if root.exists() && root.is_dir() {
            out.push(scan_country_creation_syntax_summary(root)?);
        }
    }
    Ok(out)
}

pub(crate) fn infer_leader_style(characters: usize, legacy_country_leaders: usize) -> String {
    match (characters > 0, legacy_country_leaders > 0) {
        (true, true) => "mixed_modern_characters_and_legacy_create_country_leader".to_string(),
        (true, false) => "modern_common_characters_plus_recruit_character".to_string(),
        (false, true) => "legacy_history_create_country_leader".to_string(),
        (false, false) => "unknown_no_country_leader_style_observed".to_string(),
    }
}

pub(crate) fn scan_history_state_files(root: &Path) -> Result<Vec<String>, String> {
    let mut files = txt_files(root, "history/states")?
        .iter()
        .map(|file| rel_slash(root, file))
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

pub(crate) fn scan_history_state_styles(root: &Path) -> Result<Vec<HistoryStateStyle>, String> {
    let mut out = Vec::new();
    for file in txt_files(root, "history/states")? {
        let rel = rel_slash(root, &file);
        let text = strip_comments(&read_utf8_lossy(&file)?);
        for block in direct_blocks_named(&text, "state") {
            let province_ids = state_province_ids(&block);
            let history_blocks = direct_blocks_named(&block, "history");
            let mut cores = BTreeSet::new();
            let mut victory_points = BTreeSet::new();
            let mut buildings = BTreeSet::new();

            for history in &history_blocks {
                for value in assignment_values_in_text(history, "add_core_of") {
                    if looks_like_tag(&value) || is_reference_identifier(&value) {
                        cores.insert(value);
                    }
                }
                for vp_block in direct_blocks_named(history, "victory_points") {
                    for province in victory_point_province_ids(&vp_block) {
                        victory_points.insert(province);
                    }
                }
                for buildings_block in direct_blocks_named(history, "buildings") {
                    collect_history_buildings(&buildings_block, &mut buildings);
                }
            }

            out.push(HistoryStateStyle {
                file: rel.clone(),
                id: block_assignment(&block, "id").and_then(|value| value.parse::<i64>().ok()),
                name: block_assignment(&block, "name"),
                manpower: block_assignment(&block, "manpower")
                    .and_then(|value| value.parse::<i64>().ok()),
                state_category: block_assignment(&block, "state_category"),
                owner: first_direct_assignment(&history_blocks, "owner"),
                controller: first_direct_assignment(&history_blocks, "controller"),
                cores: cores.into_iter().collect(),
                province_count: province_ids.len(),
                province_sample: province_ids.into_iter().take(30).collect(),
                victory_point_provinces: victory_points.into_iter().take(30).collect(),
                buildings: buildings.into_iter().take(30).collect(),
                resources: collect_state_resources(&block)
                    .into_iter()
                    .take(30)
                    .collect(),
            });
        }
    }
    out.sort_by(|a, b| {
        a.id.unwrap_or(i64::MAX)
            .cmp(&b.id.unwrap_or(i64::MAX))
            .then(a.file.cmp(&b.file))
    });
    Ok(out)
}

pub(crate) fn scan_province_definitions(
    root: &Path,
) -> Result<Vec<ProvinceDefinitionSummary>, String> {
    let path = root.join("map").join("definition.csv");
    if !path.exists() {
        return Ok(Vec::new());
    }

    let mut summary = ProvinceDefinitionSummary {
        file: rel_slash(root, &path),
        province_count: 0,
        land_count: 0,
        sea_count: 0,
        lake_count: 0,
        unknown_type_count: 0,
        sample_ids: Vec::new(),
    };

    for line in read_utf8_lossy(&path)?.lines() {
        let trimmed = line.trim_start_matches('\u{feff}').trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let parts = trimmed.split(';').map(str::trim).collect::<Vec<_>>();
        let Some(raw_id) = parts.first() else {
            continue;
        };
        let Ok(id) = raw_id.parse::<i64>() else {
            continue;
        };
        if id <= 0 {
            continue;
        }

        summary.province_count += 1;
        if summary.sample_ids.len() < 30 {
            summary.sample_ids.push(id);
        }
        match parts
            .get(4)
            .copied()
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "land" => summary.land_count += 1,
            "sea" => summary.sea_count += 1,
            "lake" => summary.lake_count += 1,
            _ => summary.unknown_type_count += 1,
        }
    }

    Ok(vec![summary])
}

pub(crate) fn state_province_ids(block: &str) -> Vec<i64> {
    let mut ids = BTreeSet::new();
    for provinces in direct_blocks_named(block, "provinces") {
        collect_i64_tokens(&provinces, &mut ids);
    }
    ids.into_iter().collect()
}

pub(crate) fn first_direct_assignment(blocks: &[String], key: &str) -> Option<String> {
    for block in blocks {
        if let Some(value) = block_assignment(block, key) {
            return Some(value);
        }
    }
    None
}

pub(crate) fn victory_point_province_ids(block: &str) -> Vec<i64> {
    let values = token_candidates(block)
        .into_iter()
        .filter_map(|token| token.parse::<i64>().ok())
        .collect::<Vec<_>>();
    let mut provinces = BTreeSet::new();
    for pair in values.chunks(2) {
        if let Some(province) = pair.first() {
            provinces.insert(*province);
        }
    }
    provinces.into_iter().collect()
}

pub(crate) fn collect_history_buildings(block: &str, out: &mut BTreeSet<String>) {
    for key in direct_block_keys(block) {
        collect_history_building_key(&key, out);
    }
    for (key, child) in direct_child_blocks(block) {
        if key.parse::<i64>().is_err() {
            continue;
        }
        for nested_key in direct_block_keys(&child) {
            collect_history_building_key(&nested_key, out);
        }
    }
}

pub(crate) fn collect_history_building_key(key: &str, out: &mut BTreeSet<String>) {
    if key.parse::<i64>().is_ok() || is_common_definition_field(key) {
        return;
    }
    if is_reference_identifier(key) {
        out.insert(key.to_string());
    }
}

pub(crate) fn collect_state_resources(block: &str) -> Vec<String> {
    let mut resources = BTreeSet::new();
    for resources_block in direct_blocks_named(block, "resources") {
        for key in direct_block_keys(&resources_block) {
            if key.parse::<i64>().is_ok() || is_common_definition_field(&key) {
                continue;
            }
            if is_reference_identifier(&key) {
                resources.insert(key);
            }
        }
    }
    resources.into_iter().collect()
}

pub(crate) fn scan_non_ascii_paths(root: &Path, files: &[PathBuf], limit: usize) -> Vec<String> {
    let mut paths = files
        .iter()
        .map(|file| rel_slash(root, file))
        .filter(|path| !path.is_ascii())
        .collect::<Vec<_>>();
    paths.sort();
    paths.truncate(limit);
    paths
}

pub(crate) fn focus_trees_json(values: &[FocusTreeStyle]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|tree| {
                format!(
                    "{{\"file\": {}, \"tree_id\": {}, \"country_tag\": {}, \"focus_count\": {}}}",
                    json_str(&tree.file),
                    json_str(&tree.tree_id),
                    json_str(&tree.country_tag),
                    tree.focus_count
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn country_tag_mappings_json(values: &[CountryTagMapping]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|mapping| {
                format!(
                    "{{\"file\": {}, \"tag\": {}, \"country_file\": {}}}",
                    json_str(&mapping.file),
                    json_str(&mapping.tag),
                    json_str(&mapping.country_file)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn character_styles_json(values: &[CharacterStyle]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|character| {
                format!(
                    "{{\"file\": {}, \"id\": {}, \"roles\": {}, \"has_country_leader\": {}, \"traits\": {}}}",
                    json_str(&character.file),
                    json_str(&character.id),
                    json_array(&character.roles),
                    json_bool(character.roles.iter().any(|role| role == "country_leader")),
                    json_array(&character.traits)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn history_character_uses_json(values: &[HistoryCharacterUse]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|history| {
                format!(
                    "{{\"file\": {}, \"tag\": {}, \"recruited_characters\": {}, \"legacy_country_leaders\": {}}}",
                    json_str(&history.file),
                    json_str(&history.tag),
                    json_array(&history.recruited_characters),
                    history.legacy_country_leaders
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn legacy_country_leaders_json(values: &[LegacyCountryLeaderStyle]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|leader| {
                format!(
                    "{{\"file\": {}, \"name\": {}, \"ideology\": {}, \"picture\": {}, \"traits\": {}}}",
                    json_str(&leader.file),
                    json_optional_str(leader.name.as_deref()),
                    json_optional_str(leader.ideology.as_deref()),
                    json_optional_str(leader.picture.as_deref()),
                    json_array(&leader.traits)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn history_states_json(values: &[HistoryStateStyle]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|state| {
                format!(
                    "{{\"file\": {}, \"id\": {}, \"name\": {}, \"manpower\": {}, \"state_category\": {}, \"owner\": {}, \"controller\": {}, \"cores\": {}, \"province_count\": {}, \"province_sample\": {}, \"victory_point_provinces\": {}, \"buildings\": {}, \"resources\": {}}}",
                    json_str(&state.file),
                    json_optional_i64(state.id),
                    json_optional_str(state.name.as_deref()),
                    json_optional_i64(state.manpower),
                    json_optional_str(state.state_category.as_deref()),
                    json_optional_str(state.owner.as_deref()),
                    json_optional_str(state.controller.as_deref()),
                    json_array(&state.cores),
                    state.province_count,
                    json_i64_array(&state.province_sample),
                    json_i64_array(&state.victory_point_provinces),
                    json_array(&state.buildings),
                    json_array(&state.resources)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn province_definitions_json(values: &[ProvinceDefinitionSummary]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|definition| {
                format!(
                    "{{\"file\": {}, \"province_count\": {}, \"land_count\": {}, \"sea_count\": {}, \"lake_count\": {}, \"unknown_type_count\": {}, \"sample_ids\": {}}}",
                    json_str(&definition.file),
                    definition.province_count,
                    definition.land_count,
                    definition.sea_count,
                    definition.lake_count,
                    definition.unknown_type_count,
                    json_i64_array(&definition.sample_ids)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn country_creation_syntax_json(summary: &CountryCreationSyntaxSummary) -> String {
    format!(
        "{{\"root\": {}, \"leader_style\": {}, \"country_tag_mappings\": {}, \"country_definition_files\": {}, \"country_leader_traits\": {}, \"characters\": {}, \"history_character_files\": {}, \"legacy_country_leaders\": {}}}",
        json_str(&summary.root),
        json_str(&summary.leader_style),
        summary.country_tag_mappings,
        summary.country_definition_files,
        summary.country_leader_traits,
        summary.characters,
        summary.history_character_files,
        summary.legacy_country_leaders
    )
}

pub(crate) fn country_creation_syntax_array_json(
    values: &[CountryCreationSyntaxSummary],
) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(country_creation_syntax_json)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn event_namespace_stats_json(values: &BTreeMap<String, EventNamespaceStats>) -> String {
    format!(
        "{{{}}}",
        values
            .iter()
            .map(|(namespace, stats)| {
                let files = stats.files.iter().cloned().collect::<Vec<_>>();
                format!(
                    "{}: {{\"max_id\": {}, \"files\": {}, \"country_event\": {}, \"news_event\": {}, \"state_event\": {}}}",
                    json_str(namespace),
                    stats
                        .max_id
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "null".to_string()),
                    json_array(&files),
                    stats.country_events,
                    stats.news_events,
                    stats.state_events
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn localisation_files_json(values: &[LocalisationFileStyle]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|file| {
                format!(
                    "{{\"file\": {}, \"header\": {}, \"bom\": {}, \"key_count\": {}, \"colon_zero_count\": {}, \"loose_count\": {}}}",
                    json_str(&file.file),
                    json_str(&file.header),
                    json_bool(file.bom),
                    file.key_count,
                    file.colon_zero_count,
                    file.loose_count
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn rel_slash(root: &Path, path: &Path) -> String {
    slash_path(path.strip_prefix(root).unwrap_or(path))
}
