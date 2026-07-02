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
    let mut report = audit_gfx(&resolved.root)?;
    if map.flags.contains("changed-only") {
        report.filter_changed(&changed_files);
    }
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
    id: String,
    files: Vec<String>,
    detail: Option<String>,
}

impl GfxAuditReport {
    fn filter_changed(&mut self, changed_files: &[String]) {
        self.missing_textures
            .retain(|issue| gfx_issue_touches_changed(issue, changed_files));
        self.missing_sprites
            .retain(|issue| gfx_issue_touches_changed(issue, changed_files));
        self.orphan_sprites
            .retain(|issue| gfx_issue_touches_changed(issue, changed_files));
        self.unregistered_images
            .retain(|issue| gfx_issue_touches_changed(issue, changed_files));
    }
}

fn audit_gfx(root: &Path) -> Result<GfxAuditReport, String> {
    if !root.exists() {
        return Err(format!("{}: mod root does not exist", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("{}: mod root is not a directory", root.display()));
    }
    let sprites = scan_interface_sprites(root)?;
    let mut sprite_defs: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut texture_defs: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut missing_textures = Vec::new();
    collect_sprite_definitions(
        root,
        &mut sprite_defs,
        &mut texture_defs,
        &mut missing_textures,
    )?;
    let refs = collect_project_gfx_refs(root)?;
    let image_files = collect_image_files(root)?;

    let missing_sprites = refs
        .iter()
        .filter(|(sprite, _)| !sprite_defs.contains_key(*sprite) && !is_known_vanilla_gfx(sprite))
        .map(|(sprite, files)| GfxIssue {
            id: sprite.clone(),
            files: files.iter().cloned().collect(),
            detail: Some("referenced sprite has no local spriteType".to_string()),
        })
        .collect::<Vec<_>>();
    let orphan_sprites = sprite_defs
        .iter()
        .filter(|(sprite, _)| !refs.contains_key(*sprite))
        .map(|(sprite, files)| GfxIssue {
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
            id: image.clone(),
            files: vec![image.clone()],
            detail: Some(
                "image exists but no local spriteType texturefile references it".to_string(),
            ),
        })
        .collect::<Vec<_>>();

    Ok(GfxAuditReport {
        sprites_total: sprites.len(),
        refs_total: refs.len(),
        image_files_total: image_files.len(),
        missing_textures,
        missing_sprites,
        orphan_sprites,
        unregistered_images,
    })
}

fn collect_sprite_definitions(
    root: &Path,
    sprite_defs: &mut BTreeMap<String, Vec<String>>,
    texture_defs: &mut BTreeMap<String, Vec<String>>,
    missing_textures: &mut Vec<GfxIssue>,
) -> Result<(), String> {
    let interface = root.join("interface");
    if !interface.exists() {
        return Ok(());
    }
    for file in collect_files(&interface)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "gfx" {
            continue;
        }
        let rel = rel_slash(root, &file);
        let text = read_utf8_lossy(&file)?;
        for block in sprite_type_blocks(&text) {
            let name = block_assignment(&block, "name").unwrap_or_default();
            let texturefile = block_assignment(&block, "texturefile").unwrap_or_default();
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
                        id: name,
                        files: vec![rel.clone()],
                        detail: Some(normalized_texture),
                    });
                }
            }
        }
    }
    Ok(())
}

fn collect_project_gfx_refs(root: &Path) -> Result<BTreeMap<String, BTreeSet<String>>, String> {
    let mut refs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for file in collect_files(root)? {
        let ext = file
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(ext.as_str(), "txt" | "gui" | "asset") {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        let mut file_refs = BTreeMap::<String, BTreeSet<PathBuf>>::new();
        collect_gfx_refs(&file, &text, &mut file_refs);
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

fn collect_image_files(root: &Path) -> Result<BTreeSet<String>, String> {
    let mut files = BTreeSet::new();
    let gfx_root = root.join("gfx");
    if !gfx_root.exists() {
        return Ok(files);
    }
    for file in collect_files(&gfx_root)? {
        let ext = file
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if matches!(ext.as_str(), "dds" | "png" | "tga") {
            files.insert(rel_slash(root, &file));
        }
    }
    Ok(files)
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
                    "{{\"id\": {}, \"files\": {}, \"detail\": {}}}",
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

fn gfx_issue_touches_changed(issue: &GfxIssue, changed_files: &[String]) -> bool {
    issue.files.iter().any(|file| {
        changed_files
            .iter()
            .any(|changed| file == changed || file.starts_with(&format!("{changed}:")))
    })
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
