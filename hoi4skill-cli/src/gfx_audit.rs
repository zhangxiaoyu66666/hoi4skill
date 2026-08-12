//! Project-level GFX audit for large mods.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_gfx_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = map
        .positionals
        .first()
        .cloned()
        .or_else(|| value(&map, "mod-root").map(str::to_string))
        .ok_or_else(|| "missing mod root or launcher .mod file".to_string())?;
    let resolved = resolve_mod_root(&normalize_path(&input)?)?;
    let max_items = parse_usize_option(&map, "max-items", 200)?;
    let changed_files = gfx_audit_changed_files(&resolved.root, &map)?;
    if map.flags.contains("changed-only") && changed_files.is_empty() {
        return Err("--changed-only requires at least one --changed <path>".to_string());
    }
    let report = if map.flags.contains("changed-only") {
        audit_gfx_changed(&resolved.root, &changed_files)?
    } else {
        audit_gfx(&resolved.root)?
    };
    let json = gfx_audit_json(&resolved, &report, &changed_files, max_items);
    write_or_print(&json, value(&map, "output"))
}

#[derive(Default)]
struct GfxAuditReport {
    sprites_total: usize,
    refs_total: usize,
    image_files_total: usize,
    missing_textures: Vec<GfxIssue>,
    missing_sprites: Vec<GfxIssue>,
    orphan_sprites: Vec<GfxIssue>,
    unregistered_images: Vec<GfxIssue>,
}

#[derive(Clone)]
struct GfxIssue {
    classification: &'static str,
    id: String,
    files: Vec<String>,
    detail: Option<String>,
}

fn audit_gfx(root: &Path) -> Result<GfxAuditReport, String> {
    if !root.exists() {
        return Err(format!("{}: mod root does not exist", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("{}: mod root is not a directory", root.display()));
    }
    let files = collect_files(root)?;
    let mut sprite_defs: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut texture_defs: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut raw_sprite_defs: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut missing_textures = Vec::new();
    let sprites_total = collect_sprite_definitions(
        root,
        &files,
        &mut sprite_defs,
        &mut texture_defs,
        &mut raw_sprite_defs,
        &mut missing_textures,
    )?;
    let refs = collect_project_gfx_refs(root, &files)?;
    let image_files = collect_image_files(root, &files);

    let missing_sprites = refs
        .iter()
        .filter(|(sprite, _)| !sprite_defs.contains_key(*sprite) && !is_known_vanilla_gfx(sprite))
        .map(|(sprite, files)| GfxIssue {
            classification: if raw_sprite_defs.contains_key(sprite) {
                "parser_gap"
            } else {
                "confirmed_missing"
            },
            id: sprite.clone(),
            files: files.iter().cloned().collect(),
            detail: Some("referenced sprite has no local spriteType".to_string()),
        })
        .collect::<Vec<_>>();
    let orphan_sprites = sprite_defs
        .iter()
        .filter(|(sprite, _)| !refs.contains_key(*sprite))
        .map(|(sprite, files)| GfxIssue {
            classification: "confirmed_missing",
            id: sprite.clone(),
            files: files.clone(),
            detail: Some(
                "spriteType is registered but no local script reference was found".to_string(),
            ),
        })
        .collect::<Vec<_>>();
    let registered_textures = texture_defs.keys().cloned().collect::<BTreeSet<_>>();
    let unregistered_images = image_files
        .iter()
        .filter(|image| !registered_textures.contains(*image))
        .map(|image| GfxIssue {
            classification: "confirmed_missing",
            id: image.clone(),
            files: vec![image.clone()],
            detail: Some(
                "image exists but no local spriteType texturefile references it".to_string(),
            ),
        })
        .collect::<Vec<_>>();

    Ok(GfxAuditReport {
        sprites_total,
        refs_total: refs.len(),
        image_files_total: image_files.len(),
        missing_textures,
        missing_sprites,
        orphan_sprites,
        unregistered_images,
    })
}

fn audit_gfx_changed(root: &Path, changed_files: &[String]) -> Result<GfxAuditReport, String> {
    let fragments = load_changed_gfx_fragments(root, changed_files)?;
    let changed = changed_files.iter().cloned().collect::<BTreeSet<_>>();
    let mut sprite_defs = BTreeMap::<String, Vec<String>>::new();
    let mut texture_defs = BTreeMap::<String, Vec<String>>::new();
    let mut raw_sprite_defs = BTreeMap::<String, Vec<String>>::new();
    let mut missing_textures = Vec::new();
    let mut sprites_total = 0usize;
    for (relative, fragment) in &fragments.files {
        for name in &fragment.raw_names {
            raw_sprite_defs
                .entry(name.clone())
                .or_default()
                .push(relative.clone());
        }
        for (name, texturefile) in &fragment.sprites {
            sprites_total += 1;
            if !name.is_empty() {
                sprite_defs
                    .entry(name.clone())
                    .or_default()
                    .push(relative.clone());
            }
            if !texturefile.is_empty() {
                let normalized_texture = normalize_texture_path(texturefile);
                texture_defs
                    .entry(normalized_texture.clone())
                    .or_default()
                    .push(relative.clone());
                if changed.contains(relative) && resolve_texture(root, texturefile).is_none() {
                    missing_textures.push(GfxIssue {
                        classification: "confirmed_missing",
                        id: name.clone(),
                        files: vec![relative.clone()],
                        detail: Some(normalized_texture),
                    });
                }
            }
        }
    }
    let refs = collect_project_gfx_refs_from_fragments(&fragments.files);
    let missing_sprites = refs
        .iter()
        .filter(|(sprite, files)| {
            files.iter().any(|file| changed.contains(file))
                && !sprite_defs.contains_key(*sprite)
                && !is_known_vanilla_gfx(sprite)
        })
        .map(|(sprite, files)| GfxIssue {
            classification: if raw_sprite_defs.contains_key(sprite) {
                "parser_gap"
            } else {
                "confirmed_missing"
            },
            id: sprite.clone(),
            files: files.iter().cloned().collect(),
            detail: Some("referenced sprite has no local spriteType".to_string()),
        })
        .collect::<Vec<_>>();
    let orphan_sprites = sprite_defs
        .iter()
        .filter(|(sprite, files)| {
            files.iter().any(|file| changed.contains(file)) && !refs.contains_key(*sprite)
        })
        .map(|(sprite, files)| GfxIssue {
            classification: "confirmed_missing",
            id: sprite.clone(),
            files: files.clone(),
            detail: Some(
                "spriteType is registered but no local script reference was found".to_string(),
            ),
        })
        .collect::<Vec<_>>();
    let registered_textures = texture_defs.keys().cloned().collect::<BTreeSet<_>>();
    let unregistered_images = fragments
        .changed_images
        .iter()
        .filter(|image| !registered_textures.contains(*image))
        .map(|image| GfxIssue {
            classification: "confirmed_missing",
            id: image.clone(),
            files: vec![image.clone()],
            detail: Some(
                "image exists but no local spriteType texturefile references it".to_string(),
            ),
        })
        .collect();
    for image in &fragments.changed_images {
        if !root.join(image.replace('/', "\\")).is_file() && registered_textures.contains(image) {
            for (sprite, files) in &sprite_defs {
                if texture_defs
                    .get(image)
                    .is_some_and(|definitions| definitions.iter().any(|file| files.contains(file)))
                {
                    missing_textures.push(GfxIssue {
                        classification: "confirmed_missing",
                        id: sprite.clone(),
                        files: files.clone(),
                        detail: Some(image.clone()),
                    });
                }
            }
        }
    }
    Ok(GfxAuditReport {
        sprites_total,
        refs_total: refs.len(),
        image_files_total: fragments.image_files_total,
        missing_textures,
        missing_sprites,
        orphan_sprites,
        unregistered_images,
    })
}

fn collect_project_gfx_refs_from_fragments(
    fragments: &BTreeMap<String, GfxFileFragment>,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut refs = BTreeMap::<String, BTreeSet<String>>::new();
    for (relative, fragment) in fragments {
        for sprite in &fragment.refs {
            refs.entry(sprite.clone())
                .or_default()
                .insert(relative.clone());
        }
    }
    refs
}

fn collect_sprite_definitions(
    root: &Path,
    files: &[PathBuf],
    sprite_defs: &mut BTreeMap<String, Vec<String>>,
    texture_defs: &mut BTreeMap<String, Vec<String>>,
    raw_sprite_defs: &mut BTreeMap<String, Vec<String>>,
    missing_textures: &mut Vec<GfxIssue>,
) -> Result<usize, String> {
    let mut sprites_total = 0usize;
    for file in files {
        let rel = rel_slash(root, file);
        if !rel.starts_with("interface/")
            || !file
                .extension()
                .and_then(OsStr::to_str)
                .is_some_and(|ext| ext.eq_ignore_ascii_case("gfx"))
        {
            continue;
        }
        let text = read_utf8_lossy(file)?;
        for name in raw_gfx_name_assignments(&text) {
            raw_sprite_defs.entry(name).or_default().push(rel.clone());
        }
        for block in named_gfx_type_blocks(&text) {
            let name = block_assignment(&block, "name").unwrap_or_default();
            let texturefile = gfx_texturefile_assignment(&block).unwrap_or_default();
            if !name.is_empty() || !texturefile.is_empty() {
                sprites_total += 1;
            }
            if !name.is_empty() {
                sprite_defs
                    .entry(name.clone())
                    .or_default()
                    .push(rel.clone());
            }
            if !texturefile.is_empty() {
                let normalized_texture = normalize_texture_path(&texturefile);
                texture_defs
                    .entry(normalized_texture.clone())
                    .or_default()
                    .push(rel.clone());
                if resolve_texture(root, &texturefile).is_none() {
                    missing_textures.push(GfxIssue {
                        classification: "confirmed_missing",
                        id: name,
                        files: vec![rel.clone()],
                        detail: Some(normalized_texture),
                    });
                }
            }
        }
    }
    Ok(sprites_total)
}

fn collect_project_gfx_refs(
    root: &Path,
    files: &[PathBuf],
) -> Result<BTreeMap<String, BTreeSet<String>>, String> {
    let mut refs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for file in files {
        let ext = file
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(ext.as_str(), "txt" | "gui" | "asset") {
            continue;
        }
        let text = read_utf8_lossy(file)?;
        let mut file_refs = BTreeMap::<String, BTreeSet<PathBuf>>::new();
        collect_gfx_refs(file, &text, &mut file_refs);
        for (sprite, paths) in file_refs {
            for path in paths {
                refs.entry(sprite.clone())
                    .or_default()
                    .insert(rel_slash(root, &path));
            }
        }
    }
    Ok(refs)
}

fn collect_image_files(root: &Path, paths: &[PathBuf]) -> BTreeSet<String> {
    let mut files = BTreeSet::new();
    for file in paths {
        let ext = file
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        let rel = rel_slash(root, file);
        if rel.starts_with("gfx/") && matches!(ext.as_str(), "dds" | "png" | "tga") {
            files.insert(rel);
        }
    }
    files
}

fn gfx_audit_json(
    resolved: &ModRootResolution,
    report: &GfxAuditReport,
    changed_files: &[String],
    max_items: usize,
) -> String {
    format!(
        "{{\n  \"schema\": \"hoi4skill.gfx_audit.v1\",\n  \"mod_root\": {},\n  \"input\": {},\n  \"input_kind\": {},\n  \"sprites_total\": {},\n  \"refs_total\": {},\n  \"image_files_total\": {},\n  \"missing_textures_count\": {},\n  \"missing_sprites_count\": {},\n  \"orphan_sprites_count\": {},\n  \"unregistered_images_count\": {},\n  \"changed_files\": {},\n  \"missing_textures\": {},\n  \"missing_sprites\": {},\n  \"orphan_sprites\": {},\n  \"unregistered_images\": {}\n}}\n",
        json_str(&resolved.root.display().to_string()),
        json_str(&resolved.input.display().to_string()),
        json_str(&resolved.input_kind),
        report.sprites_total,
        report.refs_total,
        report.image_files_total,
        report.missing_textures.len(),
        report.missing_sprites.len(),
        report.orphan_sprites.len(),
        report.unregistered_images.len(),
        json_array(changed_files),
        gfx_issues_json(&report.missing_textures, max_items),
        gfx_issues_json(&report.missing_sprites, max_items),
        gfx_issues_json(&report.orphan_sprites, max_items),
        gfx_issues_json(&report.unregistered_images, max_items)
    )
}

fn gfx_issues_json(issues: &[GfxIssue], max_items: usize) -> String {
    format!(
        "[{}]",
        issues
            .iter()
            .take(max_items)
            .map(|issue| {
                format!(
                    "{{\"classification\": {}, \"id\": {}, \"files\": {}, \"detail\": {}}}",
                    json_str(issue.classification),
                    json_str(&issue.id),
                    json_array(&issue.files),
                    json_optional_str(issue.detail.as_deref())
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn gfx_audit_changed_files(root: &Path, map: &ArgMap) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    for key in ["changed", "changed-file"] {
        for raw in repeated_values(map, key) {
            let path = PathBuf::from(raw);
            let rel = if path.is_absolute() {
                relative_slash_path(root, &path)
            } else {
                slash_path(&path)
            };
            if !files.iter().any(|item| item == &rel) {
                files.push(rel);
            }
        }
    }
    Ok(files)
}

fn normalize_texture_path(texturefile: &str) -> String {
    texturefile
        .trim_matches('"')
        .replace('\\', "/")
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}
