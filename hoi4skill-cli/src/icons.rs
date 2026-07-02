//! Icon preview, GFX registration, asset renaming, and sprite lookup.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_icon_preview(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let root = require_value(&map, "mod-root")?;
    let root = normalize_path(&root)?;
    let output = value(&map, "output")
        .map(normalize_path)
        .transpose()?
        .unwrap_or_else(|| {
            env::temp_dir().join("hoi4-icon-preview").join(slugify(
                root.file_name().and_then(OsStr::to_str).unwrap_or("mod"),
                "mod",
            ))
        });
    let max_icons = value(&map, "max-icons")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(800);
    fs::create_dir_all(output.join("assets"))
        .map_err(|e| format!("create {}: {e}", output.display()))?;

    let sprites = scan_sprites(&root)?;
    let limited: Vec<_> = sprites.into_iter().take(max_icons).collect();
    let mut rows = Vec::new();
    for (idx, sprite) in limited.iter().enumerate() {
        let status;
        let mut preview = String::new();
        if let Some(local) = resolve_texture(&root, &sprite.texturefile) {
            let ext = local
                .extension()
                .and_then(OsStr::to_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            if ext == "png" {
                let name = format!("{idx:04}_{}.png", slugify(&sprite.name, "icon"));
                let dest = output.join("assets").join(&name);
                fs::copy(&local, &dest).map_err(|e| format!("copy {}: {e}", local.display()))?;
                preview = format!("assets/{name}");
                status = "preview ok".to_string();
            } else if ext == "dds" {
                status = "dds listed; build PNG thumbnails with a DDS-capable image tool if needed"
                    .to_string();
            } else {
                status = "listed".to_string();
            }
            rows.push((sprite.clone(), local.display().to_string(), preview, status));
        } else {
            rows.push((
                sprite.clone(),
                String::new(),
                preview,
                "missing texture".to_string(),
            ));
        }
    }
    write_icon_preview(&output, &root, &rows)?;
    println!("Preview: {}", output.join("index.html").display());
    println!("Manifest: {}", output.join("icons.tsv").display());
    println!("Items: {}", rows.len());
    Ok(())
}

pub(crate) fn cmd_register_gfx_icons(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let root = normalize_path(&require_value(&map, "mod-root")?)?;
    let raw_prefix = require_value(&map, "prefix")?;
    let prefix = sanitize_identifier_part(&raw_prefix, "mod");
    let categories = parse_gfx_registration_categories(value(&map, "category"))?;
    let report = register_gfx_icons(&root, &prefix, &categories)?;
    write_or_print(
        &gfx_registration_report_json(&report),
        value(&map, "output"),
    )
}

pub(crate) fn cmd_register_gui_asset(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let root = normalize_path(&require_value(&map, "mod-root")?)?;
    if !root.is_dir() {
        return Err(format!("mod root does not exist: {}", root.display()));
    }
    let sprite = require_value(&map, "sprite")
        .or_else(|_| require_value(&map, "name"))
        .map(|value| value.trim().trim_matches('"').to_string())?;
    let texturefile = require_value(&map, "texturefile")
        .or_else(|_| require_value(&map, "texture"))
        .map(|value| value.replace('\\', "/").trim_matches('"').to_string())?;
    let gfx_file = value(&map, "gfx-file")
        .map(|value| value.replace('\\', "/"))
        .unwrap_or_else(|| "interface/generated_gui_assets.gfx".to_string());
    let execute = map.flags.contains("execute");
    let approved = map.flags.contains("approve-new-asset")
        || map.flags.contains("authorize-new-asset")
        || map.flags.contains("user-approved");
    let expected_dimensions = gui_asset_expected_dimensions(&map)?;
    let report = register_gui_asset(
        &root,
        &sprite,
        &texturefile,
        &gfx_file,
        execute,
        approved,
        expected_dimensions,
    )?;
    write_or_print(
        &gui_asset_registration_report_json(&report),
        value(&map, "output"),
    )?;
    if map.flags.contains("require-passed") && !report.ok {
        return Err("register-gui-asset did not pass".to_string());
    }
    Ok(())
}

pub(crate) fn cmd_asset_import_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let root = normalize_path(&require_value(&map, "mod-root")?)?;
    let file = value(&map, "file")
        .or_else(|| value(&map, "input"))
        .map(normalize_path)
        .transpose()?;
    let kind = value(&map, "kind").unwrap_or("flag").to_ascii_lowercase();
    let tag = value(&map, "tag").map(str::to_string);
    let sprite = value(&map, "sprite")
        .map(str::to_string)
        .or_else(|| asset_import_default_sprite(&kind, &map));
    let extension = file
        .as_ref()
        .and_then(|path| path.extension().and_then(OsStr::to_str))
        .unwrap_or("")
        .to_ascii_lowercase();
    let mut blockers = Vec::new();
    let mut questions = Vec::new();
    if !root.exists() {
        questions.push(format!(
            "mod root `{}` does not exist yet; asset writers will create target directories only after approval",
            root.display()
        ));
    }
    if let Some(file) = &file {
        if !file.is_file() {
            blockers.push(format!("asset source `{}` does not exist", file.display()));
        }
    } else {
        blockers.push("asset-import-plan requires --file or --input".to_string());
    }
    let supported = asset_import_supported(&kind, &extension);
    if !supported {
        blockers.push(format!(
            "unsupported asset source format `{extension}` for kind `{kind}`"
        ));
    }
    if kind == "flag" && tag.is_none() {
        blockers.push("flag asset import requires --tag or --flag-id".to_string());
    }
    if kind != "flag" && sprite.is_none() {
        blockers
            .push("sprite asset import requires --sprite or an inferrable kind/prefix".to_string());
    }
    let ok = blockers.is_empty();
    let json = asset_import_plan_json(
        ok,
        &root,
        file.as_deref(),
        &kind,
        &extension,
        tag.as_deref(),
        sprite.as_deref(),
        supported,
        &questions,
        &blockers,
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub(crate) struct GuiAssetRegistrationReport {
    pub(crate) ok: bool,
    pub(crate) status: String,
    pub(crate) executed: bool,
    pub(crate) approved: bool,
    pub(crate) mod_root: PathBuf,
    pub(crate) sprite: String,
    pub(crate) texturefile: String,
    pub(crate) local_texture_path: PathBuf,
    pub(crate) gfx_file: PathBuf,
    pub(crate) changed_files: Vec<PathBuf>,
    pub(crate) blockers: Vec<String>,
    pub(crate) questions: Vec<String>,
    pub(crate) image_probe: Option<GuiImageProbe>,
    pub(crate) expected_dimensions: Option<(u32, u32)>,
    pub(crate) code: String,
}

#[derive(Clone, Debug)]
pub(crate) struct GuiImageProbe {
    pub(crate) format: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) alpha_capable: bool,
}

pub(crate) fn register_gui_asset(
    root: &Path,
    sprite: &str,
    texturefile: &str,
    gfx_file: &str,
    execute: bool,
    approved: bool,
    expected_dimensions: Option<(u32, u32)>,
) -> Result<GuiAssetRegistrationReport, String> {
    let mut blockers = Vec::new();
    let mut questions = Vec::new();
    let texturefile = texturefile.replace('\\', "/");
    let gfx_file = gfx_file.replace('\\', "/");
    if !sprite.starts_with("GFX_") || !is_identifier_like(sprite) {
        blockers.push(format!("invalid_gui_sprite_name:{sprite}"));
        questions
            .push("Provide a valid indexed-style GUI sprite id beginning with `GFX_`.".to_string());
    }
    if texturefile.starts_with('/') || texturefile.contains("..") {
        blockers.push(format!("unsafe_gui_texturefile:{texturefile}"));
        questions
            .push("Texturefile must be a safe mod-relative path under gfx/interface/.".to_string());
    }
    if !texturefile
        .to_ascii_lowercase()
        .starts_with("gfx/interface/")
    {
        blockers.push(format!(
            "gui_texturefile_outside_gfx_interface:{texturefile}"
        ));
        questions.push("GUI assets must live under gfx/interface/ so HOI4 and strict-code-index can resolve them.".to_string());
    }
    let ext = Path::new(&texturefile)
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(ext.as_str(), "dds" | "png" | "tga") {
        blockers.push(format!("unsupported_gui_texture_extension:{texturefile}"));
        questions.push("Provide a .dds, .png, or .tga GUI texture asset.".to_string());
    }
    if gfx_file.starts_with('/') || gfx_file.contains("..") {
        blockers.push(format!("unsafe_gui_gfx_file:{gfx_file}"));
        questions.push(
            "GFX registration file must be a safe mod-relative interface/*.gfx path.".to_string(),
        );
    }
    if !gfx_file.to_ascii_lowercase().starts_with("interface/")
        || !gfx_file.to_ascii_lowercase().ends_with(".gfx")
    {
        blockers.push(format!("gui_gfx_file_not_interface_gfx:{gfx_file}"));
        questions.push("Write GUI sprite registrations to an interface/*.gfx file.".to_string());
    }
    if !approved {
        blockers.push("gui_asset_registration_requires_user_approval".to_string());
        questions.push("Rerun with --approve-new-asset only after the user confirms this new GUI asset should be registered.".to_string());
    }
    let local_texture_path = root.join(texturefile.split('/').collect::<PathBuf>());
    if !local_texture_path.is_file() {
        blockers.push(format!("gui_texturefile_missing:{texturefile}"));
        questions.push(format!(
            "Create or copy the approved transparent GUI asset to `{texturefile}` before registering `{sprite}`."
        ));
    }
    let image_probe = if local_texture_path.is_file() {
        match probe_gui_image_asset(&local_texture_path) {
            Ok(probe) => {
                if probe.width == 0 || probe.height == 0 {
                    blockers.push(format!("gui_texture_invalid_dimensions:{texturefile}"));
                    questions.push(
                        "Replace the GUI asset with an image whose width and height are non-zero."
                            .to_string(),
                    );
                }
                if !probe.alpha_capable {
                    blockers.push(format!("gui_texture_missing_alpha_channel:{texturefile}"));
                    questions.push("GUI assets should have a transparent-capable alpha channel; provide a PNG/TGA/DDS with alpha before registering.".to_string());
                }
                if let Some((expected_width, expected_height)) = expected_dimensions {
                    if probe.width != expected_width || probe.height != expected_height {
                        blockers.push(format!(
                            "gui_texture_dimension_mismatch:{texturefile}:expected_{}x{}:actual_{}x{}",
                            expected_width, expected_height, probe.width, probe.height
                        ));
                        questions.push(format!(
                            "Resize or replace `{texturefile}` to exactly {}x{} before registering `{sprite}`.",
                            expected_width, expected_height
                        ));
                    }
                }
                Some(probe)
            }
            Err(reason) => {
                blockers.push(format!("gui_texture_probe_failed:{texturefile}"));
                questions.push(format!(
                    "Replace `{texturefile}` with a readable GUI image; probe failed: {reason}"
                ));
                None
            }
        }
    } else {
        None
    };
    let lookup = sprite_lookup(root)?;
    let texture_key = normalize_texture_key(&texturefile);
    let existing_texture = lookup.name_to_texture.get(sprite).cloned();
    if let Some(existing) = existing_texture.as_deref() {
        if existing != texture_key {
            blockers.push(format!("gui_sprite_name_conflict:{sprite}:{existing}"));
            questions.push(format!(
                "Sprite `{sprite}` already points to `{existing}`; choose a new sprite id or reuse the existing texture."
            ));
        }
    }
    blockers.sort();
    blockers.dedup();
    questions.sort();
    questions.dedup();
    let gfx_path = root.join(gfx_file.split('/').collect::<PathBuf>());
    let code = render_sprite_type_block(
        sprite,
        &texturefile,
        "GUI asset registered by hoi4skill after explicit user approval.",
        GfxSpriteRenderKind::StandardLower,
    );
    let mut changed_files = Vec::new();
    let mut status = if blockers.is_empty() {
        if existing_texture.as_deref() == Some(texture_key.as_str()) {
            "existing".to_string()
        } else if execute {
            "registered".to_string()
        } else {
            "planned".to_string()
        }
    } else {
        "blocked".to_string()
    };
    if blockers.is_empty() && execute && existing_texture.as_deref() != Some(texture_key.as_str()) {
        let changed = append_blocks_to_named_wrapper(
            &gfx_path,
            "spriteTypes",
            "# Generated GUI asset registrations by hoi4skill\n",
            &[(sprite.to_string(), code.clone())],
        )?;
        if changed {
            changed_files.push(gfx_path.clone());
        } else {
            status = "existing".to_string();
        }
    }
    Ok(GuiAssetRegistrationReport {
        ok: blockers.is_empty(),
        status,
        executed: execute && blockers.is_empty(),
        approved,
        mod_root: root.to_path_buf(),
        sprite: sprite.to_string(),
        texturefile,
        local_texture_path,
        gfx_file: gfx_path,
        changed_files,
        blockers,
        questions,
        image_probe,
        expected_dimensions,
        code,
    })
}

fn asset_import_supported(kind: &str, extension: &str) -> bool {
    match kind {
        "flag" | "country_flag" => matches!(extension, "jpg" | "jpeg" | "png" | "webp" | "tga"),
        "focus_icon" | "idea_icon" | "decision_icon" | "event_picture" | "portrait"
        | "gui_asset" => {
            matches!(extension, "dds" | "png" | "tga" | "jpg" | "jpeg" | "webp")
        }
        _ => false,
    }
}

fn asset_import_default_sprite(kind: &str, map: &ArgMap) -> Option<String> {
    let id = value(map, "id")
        .or_else(|| value(map, "name"))
        .or_else(|| value(map, "tag"))?;
    let slug = slugify(id, "asset");
    let prefix = match kind {
        "focus_icon" => "GFX_goal",
        "idea_icon" => "GFX_idea",
        "decision_icon" => "GFX_decision",
        "event_picture" => "GFX_report_event",
        "portrait" => "GFX_portrait",
        "gui_asset" => "GFX_gui",
        _ => return None,
    };
    Some(format!("{prefix}_{slug}"))
}

fn asset_import_plan_json(
    ok: bool,
    root: &Path,
    file: Option<&Path>,
    kind: &str,
    extension: &str,
    tag: Option<&str>,
    sprite: Option<&str>,
    supported: bool,
    questions: &[String],
    blockers: &[String],
) -> String {
    let mut map = BTreeMap::new();
    map.insert(
        "schema".to_string(),
        json_str("hoi4skill.asset_import_plan.v1"),
    );
    map.insert("ok".to_string(), json_bool(ok).to_string());
    map.insert(
        "status".to_string(),
        json_str(if ok {
            "asset_import_plan_ready"
        } else {
            "blocked"
        }),
    );
    map.insert("direct_write".to_string(), json_bool(false).to_string());
    map.insert(
        "mod_root".to_string(),
        json_str(&root.display().to_string()),
    );
    map.insert(
        "source_file".to_string(),
        json_optional_str(file.map(|path| path.display().to_string()).as_deref()),
    );
    map.insert("kind".to_string(), json_str(kind));
    map.insert("input_extension".to_string(), json_str(extension));
    map.insert(
        "supported_input".to_string(),
        json_bool(supported).to_string(),
    );
    map.insert("tag".to_string(), json_optional_str(tag));
    map.insert("sprite".to_string(), json_optional_str(sprite));
    map.insert(
        "planned_outputs".to_string(),
        asset_import_planned_outputs_json(root, kind, tag, sprite),
    );
    map.insert(
        "next_commands".to_string(),
        json_array(&asset_import_next_commands(kind, tag, sprite)),
    );
    map.insert("questions".to_string(), json_array(questions));
    map.insert("blocker_count".to_string(), blockers.len().to_string());
    map.insert("blockers".to_string(), json_array(blockers));
    map.insert(
        "rules".to_string(),
        json_array(&[
            "asset-import-plan is plan-only and never writes image or GFX files".to_string(),
            "flag assets must become normal/medium/small TGA triplets before tag/cosmetic references use them".to_string(),
            "sprite assets must be registered before focus/idea/event/decision/GUI code references them".to_string(),
            "missing assets require user choice: provide file, generate placeholder, or reuse indexed parent asset".to_string(),
        ]),
    );
    json_raw_object(&map)
}

fn asset_import_planned_outputs_json(
    root: &Path,
    kind: &str,
    tag: Option<&str>,
    sprite: Option<&str>,
) -> String {
    let outputs = if kind == "flag" || kind == "country_flag" {
        let id = tag.unwrap_or("<TAG>");
        vec![
            format!(
                "{}:82x52",
                root.join("gfx/flags").join(format!("{id}.tga")).display()
            ),
            format!(
                "{}:41x26",
                root.join("gfx/flags/medium")
                    .join(format!("{id}.tga"))
                    .display()
            ),
            format!(
                "{}:10x7",
                root.join("gfx/flags/small")
                    .join(format!("{id}.tga"))
                    .display()
            ),
        ]
    } else {
        let sprite = sprite.unwrap_or("<GFX_sprite>");
        let texture = match kind {
            "focus_icon" => "gfx/interface/goals/<asset>.dds",
            "idea_icon" => "gfx/interface/ideas/<asset>.dds",
            "decision_icon" => "gfx/interface/decisions/<asset>.dds",
            "event_picture" => "gfx/event_pictures/<asset>.dds",
            "portrait" => "gfx/leaders/<TAG>/<asset>.dds",
            "gui_asset" => "gfx/interface/gui/<asset>.dds",
            _ => "gfx/interface/<asset>.dds",
        };
        vec![
            format!("{sprite}:{texture}"),
            "interface/generated_assets.gfx".to_string(),
        ]
    };
    json_array(&outputs)
}

fn asset_import_next_commands(kind: &str, tag: Option<&str>, sprite: Option<&str>) -> Vec<String> {
    if kind == "flag" || kind == "country_flag" {
        vec![format!(
            "hoi4skill flag-image-import --mod-root <target> --file <image> --tag {} --execute --require-passed",
            tag.unwrap_or("<TAG>")
        )]
    } else {
        vec![
            format!(
                "hoi4skill register-gui-asset --mod-root <target> --sprite {} --texturefile <gfx/path.dds|png|tga> --approve-new-asset --execute --require-passed",
                sprite.unwrap_or("<GFX_sprite>")
            ),
            "hoi4skill gfx-audit --mod-root <target> --require-passed".to_string(),
        ]
    }
}

pub(crate) fn gui_asset_registration_report_json(report: &GuiAssetRegistrationReport) -> String {
    let changed_files = report
        .changed_files
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    format!(
        "{{\n  \"schema\": \"hoi4skill.gui_asset_registration.v1\",\n  \"status\": {},\n  \"ok\": {},\n  \"executed\": {},\n  \"approved\": {},\n  \"mod_root\": {},\n  \"sprite\": {},\n  \"texturefile\": {},\n  \"local_texture_path\": {},\n  \"gfx_file\": {},\n  \"changed_files\": {},\n  \"blocking_count\": {},\n  \"blockers\": {},\n  \"questions\": {},\n  \"image_probe\": {},\n  \"expected_dimensions\": {},\n  \"dimension_check\": {},\n  \"transparent_background_required\": true,\n  \"written_to_mod\": {},\n  \"code\": {},\n  \"next_commands\": {}\n}}\n",
        json_str(&report.status),
        json_bool(report.ok),
        json_bool(report.executed),
        json_bool(report.approved),
        json_str(&report.mod_root.display().to_string()),
        json_str(&report.sprite),
        json_str(&report.texturefile),
        json_str(&report.local_texture_path.display().to_string()),
        json_str(&report.gfx_file.display().to_string()),
        json_array(&changed_files),
        report.blockers.len(),
        json_array(&report.blockers),
        json_array(&report.questions),
        gui_image_probe_json(report.image_probe.as_ref()),
        gui_expected_dimensions_json(report.expected_dimensions),
        gui_asset_dimension_check_status(report),
        json_bool(report.executed && report.ok),
        json_str(&report.code),
        json_array(&vec![
            "hoi4skill gfx-audit --mod-root <mod-root> --changed interface/generated_gui_assets.gfx --require-passed".to_string(),
            "hoi4skill validate <mod-root> --game-root <HOI4 root> --strict-code-index".to_string(),
            "hoi4skill apply-gui-intent --mod-root <mod-root> --input <gui intent> --game-root <HOI4 root> --execute --final-check --require-passed".to_string(),
        ]),
    )
}

fn gui_asset_expected_dimensions(map: &ArgMap) -> Result<Option<(u32, u32)>, String> {
    if let Some(value) = value(map, "dimensions")
        .or_else(|| value(map, "expected-dimensions"))
        .or_else(|| value(map, "size"))
    {
        let normalized = value
            .to_ascii_lowercase()
            .replace(['×', '*'], "x")
            .replace(' ', "");
        let Some((width, height)) = normalized.split_once('x') else {
            return Err(format!("invalid --dimensions `{value}`; expected WxH"));
        };
        let width = width
            .parse::<u32>()
            .map_err(|_| format!("invalid --dimensions width `{value}`"))?;
        let height = height
            .parse::<u32>()
            .map_err(|_| format!("invalid --dimensions height `{value}`"))?;
        return Ok(Some((width, height)));
    }
    match (value(map, "expected-width"), value(map, "expected-height")) {
        (Some(width), Some(height)) => Ok(Some((
            width
                .parse::<u32>()
                .map_err(|_| format!("invalid --expected-width `{width}`"))?,
            height
                .parse::<u32>()
                .map_err(|_| format!("invalid --expected-height `{height}`"))?,
        ))),
        (None, None) => Ok(None),
        _ => Err("--expected-width and --expected-height must be supplied together".to_string()),
    }
}

fn gui_expected_dimensions_json(value: Option<(u32, u32)>) -> String {
    value
        .map(|(width, height)| format!("{{\"width\": {width}, \"height\": {height}}}"))
        .unwrap_or_else(|| "null".to_string())
}

fn gui_asset_dimension_check_status(report: &GuiAssetRegistrationReport) -> String {
    let status = match (report.image_probe.as_ref(), report.expected_dimensions) {
        (Some(probe), Some((width, height))) if probe.width == width && probe.height == height => {
            "passed_expected_dimensions"
        }
        (Some(_), Some(_)) => "failed_expected_dimensions",
        (Some(_), None) => {
            "passed_header_probe; still compare against parent-mod visual sample before release"
        }
        (None, _) => "failed_or_missing_header_probe",
    };
    json_str(status)
}

fn gui_image_probe_json(probe: Option<&GuiImageProbe>) -> String {
    probe
        .map(|probe| {
            format!(
                "{{\"format\": {}, \"width\": {}, \"height\": {}, \"alpha_capable\": {}}}",
                json_str(&probe.format),
                probe.width,
                probe.height,
                json_bool(probe.alpha_capable),
            )
        })
        .unwrap_or_else(|| "null".to_string())
}

pub(crate) fn probe_gui_image_asset(path: &Path) -> Result<GuiImageProbe, String> {
    let bytes = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        return probe_png_image_header(&bytes);
    }
    if bytes.starts_with(b"DDS ") {
        return probe_dds_image_header(&bytes);
    }
    if path
        .extension()
        .and_then(OsStr::to_str)
        .is_some_and(|ext| ext.eq_ignore_ascii_case("tga"))
    {
        return probe_tga_image_header(&bytes);
    }
    Err("unsupported or unreadable GUI image header".to_string())
}

fn probe_png_image_header(bytes: &[u8]) -> Result<GuiImageProbe, String> {
    if bytes.len() < 33 || &bytes[12..16] != b"IHDR" {
        return Err("PNG IHDR header is missing".to_string());
    }
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    let color_type = bytes[25];
    Ok(GuiImageProbe {
        format: "png".to_string(),
        width,
        height,
        alpha_capable: matches!(color_type, 4 | 6),
    })
}

fn probe_dds_image_header(bytes: &[u8]) -> Result<GuiImageProbe, String> {
    if bytes.len() < 128 {
        return Err("DDS header is shorter than 128 bytes".to_string());
    }
    let height = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
    let width = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let pixel_flags = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]);
    let fourcc = &bytes[84..88];
    let alpha_bits = u32::from_le_bytes([bytes[108], bytes[109], bytes[110], bytes[111]]);
    let alpha_capable = pixel_flags & 0x1 != 0
        || alpha_bits != 0
        || matches!(fourcc, b"DXT3" | b"DXT5" | b"BC2 " | b"BC3 " | b"ATI2");
    Ok(GuiImageProbe {
        format: "dds".to_string(),
        width,
        height,
        alpha_capable,
    })
}

fn probe_tga_image_header(bytes: &[u8]) -> Result<GuiImageProbe, String> {
    if bytes.len() < 18 {
        return Err("TGA header is shorter than 18 bytes".to_string());
    }
    let width = u16::from_le_bytes([bytes[12], bytes[13]]) as u32;
    let height = u16::from_le_bytes([bytes[14], bytes[15]]) as u32;
    let pixel_depth = bytes[16];
    let descriptor = bytes[17];
    let alpha_bits = descriptor & 0x0f;
    Ok(GuiImageProbe {
        format: "tga".to_string(),
        width,
        height,
        alpha_capable: alpha_bits > 0 || pixel_depth == 32,
    })
}

#[derive(Clone)]
pub(crate) struct Sprite {
    pub(crate) name: String,
    pub(crate) texturefile: String,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum GfxRegistrationCategory {
    Dynamic,
    Focus,
    Idea,
    Event,
    Decision,
}

#[derive(Clone)]
pub(crate) struct GfxImageAsset {
    pub(crate) texturefile: String,
    pub(crate) original_texturefile: Option<String>,
    pub(crate) base: String,
    pub(crate) local_file_name: String,
    pub(crate) english_file_name: String,
    pub(crate) english_name_instruction: String,
    pub(crate) remark: String,
}

pub(crate) struct GfxImageScan {
    pub(crate) assets: Vec<GfxImageAsset>,
    pub(crate) changed_files: Vec<PathBuf>,
    pub(crate) skipped_assets: Vec<GfxSkippedAsset>,
}

#[derive(Clone)]
pub(crate) struct GfxSkippedAsset {
    pub(crate) texturefile: String,
    pub(crate) local_file_name: String,
    pub(crate) reason: String,
    pub(crate) required_action: String,
}

#[derive(Default)]
pub(crate) struct SpriteLookup {
    pub(crate) name_to_texture: BTreeMap<String, String>,
    pub(crate) texture_to_names: BTreeMap<String, Vec<String>>,
}

pub(crate) struct GfxRegistrationEntry {
    pub(crate) category: String,
    pub(crate) sprite_name: String,
    pub(crate) texturefile: String,
    pub(crate) original_texturefile: Option<String>,
    pub(crate) local_file_name: String,
    pub(crate) english_file_name: String,
    pub(crate) file: PathBuf,
    pub(crate) status: String,
    pub(crate) english_name_instruction: String,
    pub(crate) remark: String,
    pub(crate) existing_names: Vec<String>,
    pub(crate) conflict: Option<String>,
}

pub(crate) struct GfxRegistrationReport {
    pub(crate) mod_root: PathBuf,
    pub(crate) prefix: String,
    pub(crate) categories: Vec<String>,
    pub(crate) assets_scanned: usize,
    pub(crate) changed_files: Vec<PathBuf>,
    pub(crate) skipped_assets: Vec<GfxSkippedAsset>,
    pub(crate) entries: Vec<GfxRegistrationEntry>,
}

pub(crate) struct GfxSpriteTarget<'a> {
    pub(crate) category: &'a str,
    pub(crate) name_prefix: &'a str,
    pub(crate) file_suffix: &'a str,
    pub(crate) render_kind: GfxSpriteRenderKind,
}

#[derive(Clone, Copy)]
pub(crate) enum GfxSpriteRenderKind {
    DynamicGui,
    StandardLower,
    GoalUpper,
    GoalShine,
}

pub(crate) const GFX_SPRITE_TARGETS: &[GfxSpriteTarget<'static>] = &[
    GfxSpriteTarget {
        category: "dynamic_gui",
        name_prefix: "GFX",
        file_suffix: "dynamic_icons",
        render_kind: GfxSpriteRenderKind::DynamicGui,
    },
    GfxSpriteTarget {
        category: "focus",
        name_prefix: "GFX_goal",
        file_suffix: "goals",
        render_kind: GfxSpriteRenderKind::GoalUpper,
    },
    GfxSpriteTarget {
        category: "focus_shine",
        name_prefix: "GFX_goal",
        file_suffix: "goals_shine",
        render_kind: GfxSpriteRenderKind::GoalShine,
    },
    GfxSpriteTarget {
        category: "idea",
        name_prefix: "GFX_idea",
        file_suffix: "focus_idea_icons",
        render_kind: GfxSpriteRenderKind::StandardLower,
    },
    GfxSpriteTarget {
        category: "event",
        name_prefix: "GFX_report_event",
        file_suffix: "event_pictures",
        render_kind: GfxSpriteRenderKind::StandardLower,
    },
    GfxSpriteTarget {
        category: "decision",
        name_prefix: "GFX_decision",
        file_suffix: "decision_pictures",
        render_kind: GfxSpriteRenderKind::StandardLower,
    },
    GfxSpriteTarget {
        category: "decision_category",
        name_prefix: "GFX_decision_category",
        file_suffix: "decision_pictures",
        render_kind: GfxSpriteRenderKind::StandardLower,
    },
];

pub(crate) fn parse_gfx_registration_categories(
    raw: Option<&str>,
) -> Result<BTreeSet<GfxRegistrationCategory>, String> {
    let mut categories = BTreeSet::new();
    let raw = raw.unwrap_or("all");
    for item in raw.split([',', ';', '|']) {
        let item = item.trim().to_ascii_lowercase().replace('_', "-");
        match item.as_str() {
            "" => {}
            "all" => {
                categories.insert(GfxRegistrationCategory::Dynamic);
                categories.insert(GfxRegistrationCategory::Focus);
                categories.insert(GfxRegistrationCategory::Idea);
                categories.insert(GfxRegistrationCategory::Event);
                categories.insert(GfxRegistrationCategory::Decision);
            }
            "dynamic" | "gui" | "dynamic-gui" => {
                categories.insert(GfxRegistrationCategory::Dynamic);
            }
            "focus" | "national-focus" => {
                categories.insert(GfxRegistrationCategory::Focus);
            }
            "idea" | "ideas" | "national-spirit" | "national-spirits" => {
                categories.insert(GfxRegistrationCategory::Idea);
            }
            "focus-idea" | "focus-ideas" | "focus-and-idea" | "focus-and-ideas" => {
                categories.insert(GfxRegistrationCategory::Focus);
                categories.insert(GfxRegistrationCategory::Idea);
            }
            "event" | "events" => {
                categories.insert(GfxRegistrationCategory::Event);
            }
            "decision" | "decisions" | "decision-picture" | "decision-pictures" => {
                categories.insert(GfxRegistrationCategory::Decision);
            }
            other => {
                return Err(format!(
                    "unknown --category {other}; use all, dynamic, focus-idea, event, or decision"
                ));
            }
        }
    }
    if categories.is_empty() {
        return Err("--category did not select any GFX registration target".to_string());
    }
    Ok(categories)
}

pub(crate) fn register_gfx_icons(
    root: &Path,
    prefix: &str,
    categories: &BTreeSet<GfxRegistrationCategory>,
) -> Result<GfxRegistrationReport, String> {
    let scan = scan_gfx_interface_images(root, prefix)?;
    let assets_scanned = scan.assets.len() + scan.skipped_assets.len();
    let skipped_assets = scan.skipped_assets;
    let assets = scan.assets;
    let lookup = sprite_lookup(root)?;
    let mut used_names = lookup
        .name_to_texture
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut blocks_by_file: BTreeMap<PathBuf, Vec<(String, String)>> = BTreeMap::new();
    let mut entries = Vec::new();

    for asset in &assets {
        let texture_key = normalize_texture_key(&asset.texturefile);
        let existing_names = lookup
            .texture_to_names
            .get(&texture_key)
            .cloned()
            .unwrap_or_default();
        for target in GFX_SPRITE_TARGETS {
            if !target_enabled(target.category, categories) {
                continue;
            }
            let base_name = compose_sprite_name(target.name_prefix, prefix, &asset.base);
            let candidate = if matches!(target.render_kind, GfxSpriteRenderKind::GoalShine) {
                format!("{base_name}_shine")
            } else {
                base_name
            };
            let (sprite_name, status, conflict) =
                reserve_sprite_name(&candidate, &texture_key, &lookup, &mut used_names);
            let file = root
                .join("interface")
                .join(format!("{prefix}_{}.gfx", target.file_suffix));
            if status != "existing" {
                blocks_by_file.entry(file.clone()).or_default().push((
                    sprite_name.clone(),
                    render_sprite_type_block(
                        &sprite_name,
                        &asset.texturefile,
                        &asset.remark,
                        target.render_kind,
                    ),
                ));
            }
            entries.push(GfxRegistrationEntry {
                category: target.category.to_string(),
                sprite_name,
                texturefile: asset.texturefile.clone(),
                original_texturefile: asset.original_texturefile.clone(),
                local_file_name: asset.local_file_name.clone(),
                english_file_name: asset.english_file_name.clone(),
                file,
                status,
                english_name_instruction: asset.english_name_instruction.clone(),
                remark: asset.remark.clone(),
                existing_names: existing_names.clone(),
                conflict,
            });
        }
    }

    let mut changed_files = scan.changed_files;
    for (file, blocks) in blocks_by_file {
        let header = "# Generated GFX sprite registrations by hoi4skill\n";
        if append_blocks_to_named_wrapper(&file, "spriteTypes", header, &blocks)? {
            changed_files.push(file);
        }
    }

    Ok(GfxRegistrationReport {
        mod_root: root.to_path_buf(),
        prefix: prefix.to_string(),
        categories: categories.iter().map(gfx_category_label).collect(),
        assets_scanned,
        changed_files,
        skipped_assets,
        entries,
    })
}

pub(crate) fn target_enabled(
    category: &str,
    categories: &BTreeSet<GfxRegistrationCategory>,
) -> bool {
    match category {
        "dynamic_gui" => categories.contains(&GfxRegistrationCategory::Dynamic),
        "focus" | "focus_shine" => categories.contains(&GfxRegistrationCategory::Focus),
        "idea" => categories.contains(&GfxRegistrationCategory::Idea),
        "event" => categories.contains(&GfxRegistrationCategory::Event),
        "decision" | "decision_category" => categories.contains(&GfxRegistrationCategory::Decision),
        _ => false,
    }
}

pub(crate) fn gfx_category_label(category: &GfxRegistrationCategory) -> String {
    match category {
        GfxRegistrationCategory::Dynamic => "dynamic".to_string(),
        GfxRegistrationCategory::Focus => "focus".to_string(),
        GfxRegistrationCategory::Idea => "idea".to_string(),
        GfxRegistrationCategory::Event => "event".to_string(),
        GfxRegistrationCategory::Decision => "decision".to_string(),
    }
}

pub(crate) fn scan_gfx_interface_images(root: &Path, prefix: &str) -> Result<GfxImageScan, String> {
    let image_root = root.join("gfx").join("interface");
    if !image_root.exists() {
        return Ok(GfxImageScan {
            assets: Vec::new(),
            changed_files: Vec::new(),
            skipped_assets: Vec::new(),
        });
    }
    let mut files = collect_files(&image_root)?;
    files.sort();
    let mut assets = Vec::new();
    let mut changed_files = Vec::new();
    let mut skipped_assets = Vec::new();
    for file in files {
        let ext = file
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(ext.as_str(), "dds" | "png" | "tga") {
            continue;
        }
        if !file.exists() {
            continue;
        }
        let prepared = match prepare_gfx_image_asset(root, &image_root, &file)? {
            GfxAssetPreparation::Ready(prepared) => prepared,
            GfxAssetPreparation::Skipped(skipped) => {
                skipped_assets.push(skipped);
                continue;
            }
        };
        changed_files.extend(prepared.changed_files);
        let file = prepared.path;
        let rel = file.strip_prefix(&image_root).unwrap_or(&file);
        let raw_asset_name = slash_path(&rel.with_extension(""));
        if !raw_asset_name.is_ascii() {
            skipped_assets.push(GfxSkippedAsset {
                texturefile: slash_path(file.strip_prefix(root).unwrap_or(&file)),
                local_file_name: file
                    .file_name()
                    .and_then(OsStr::to_str)
                    .unwrap_or("icon")
                    .to_string(),
                reason: "image path still contains non-ASCII text after filename translation"
                    .to_string(),
                required_action: "把图片移动到英文目录后重跑；本次跳过该图片，未注册 sprite。"
                    .to_string(),
            });
            continue;
        }
        let texturefile = slash_path(file.strip_prefix(root).unwrap_or(&file));
        let base = sprite_base_from_gfx_asset(&image_root, &file, prefix);
        let local_file_name = file
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("icon")
            .to_string();
        assets.push(GfxImageAsset {
            texturefile,
            original_texturefile: prepared.original_texturefile,
            base,
            local_file_name,
            english_file_name: prepared.english_file_name,
            english_name_instruction: prepared.english_name_instruction,
            remark: prepared.remark,
        });
    }
    changed_files.sort();
    changed_files.dedup();
    Ok(GfxImageScan {
        assets,
        changed_files,
        skipped_assets,
    })
}

pub(crate) enum GfxAssetPreparation {
    Ready(PreparedGfxAsset),
    Skipped(GfxSkippedAsset),
}

pub(crate) struct PreparedGfxAsset {
    pub(crate) path: PathBuf,
    pub(crate) original_texturefile: Option<String>,
    pub(crate) english_file_name: String,
    pub(crate) english_name_instruction: String,
    pub(crate) remark: String,
    pub(crate) changed_files: Vec<PathBuf>,
}

pub(crate) fn prepare_gfx_image_asset(
    root: &Path,
    image_root: &Path,
    file: &Path,
) -> Result<GfxAssetPreparation, String> {
    let file_name = file.file_name().and_then(OsStr::to_str).unwrap_or("icon");
    let stem = file.file_stem().and_then(OsStr::to_str).unwrap_or("icon");
    if stem.is_ascii() {
        return Ok(GfxAssetPreparation::Ready(PreparedGfxAsset {
            path: file.to_path_buf(),
            original_texturefile: None,
            english_file_name: file_name.to_string(),
            english_name_instruction:
                "文件名已经是英文或 ASCII 语义名，不需要重命名；直接按该文件名注册 sprite。"
                    .to_string(),
            remark: "备注：本地文件名已经是英文/ASCII，未执行自动重命名。".to_string(),
            changed_files: Vec::new(),
        }));
    }

    let ext = file.extension().and_then(OsStr::to_str).unwrap_or("dds");
    let english_stem = match translate_asset_stem_to_english(stem) {
        Ok(english_stem) => english_stem,
        Err(reason) => {
            return Ok(GfxAssetPreparation::Skipped(GfxSkippedAsset {
                texturefile: slash_path(file.strip_prefix(root).unwrap_or(file)),
                local_file_name: file_name.to_string(),
                reason,
                required_action: "给该图片补一个语义英文文件名，或扩展内置 HOI4 术语表后重跑；本次跳过该图片，未注册 sprite。不要使用随机数字名。"
                    .to_string(),
            }));
        }
    };
    let target = unique_asset_rename_target(file, &english_stem, ext)?;
    let old_texturefile = slash_path(file.strip_prefix(root).unwrap_or(file));
    let new_texturefile = slash_path(target.strip_prefix(root).unwrap_or(&target));
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    fs::rename(file, &target)
        .map_err(|e| format!("rename {} -> {}: {e}", file.display(), target.display()))?;
    let mut changed_files =
        replace_interface_texturefile_refs(root, &old_texturefile, &new_texturefile)?;
    changed_files.push(target.clone());
    let english_file_name = target
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("icon.dds")
        .to_string();
    let rel_old = file.strip_prefix(image_root).unwrap_or(file);
    Ok(GfxAssetPreparation::Ready(PreparedGfxAsset {
        path: target,
        original_texturefile: Some(old_texturefile),
        english_file_name: english_file_name.clone(),
        english_name_instruction: format!(
            "已根据本地文件名 `{}` 翻译并自动重命名为 `{english_file_name}`；后续注册必须使用新的英文 texturefile 路径。",
            slash_path(rel_old)
        ),
        remark: format!(
            "备注：自动翻译中文文件名 `{stem}` -> `{english_stem}`，并重命名为 `{english_file_name}`。"
        ),
        changed_files,
    }))
}

pub(crate) fn unique_asset_rename_target(
    file: &Path,
    english_stem: &str,
    ext: &str,
) -> Result<PathBuf, String> {
    let parent = file
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", file.display()))?;
    let mut idx = 1usize;
    loop {
        let stem = if idx == 1 {
            english_stem.to_string()
        } else {
            format!("{english_stem}_{idx}")
        };
        let candidate = parent.join(format!("{}.{}", stem, ext.to_ascii_lowercase()));
        if !candidate.exists() {
            return Ok(candidate);
        }
        idx += 1;
    }
}

pub(crate) fn replace_interface_texturefile_refs(
    root: &Path,
    old_texturefile: &str,
    new_texturefile: &str,
) -> Result<Vec<PathBuf>, String> {
    let interface = root.join("interface");
    if !interface.exists() {
        return Ok(Vec::new());
    }
    let mut changed = Vec::new();
    for file in collect_files(&interface)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "gfx" {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        let updated = text
            .replace(old_texturefile, new_texturefile)
            .replace(&old_texturefile.replace('/', "\\"), new_texturefile);
        if updated != text {
            fs::write(&file, updated).map_err(|e| format!("write {}: {e}", file.display()))?;
            changed.push(file);
        }
    }
    Ok(changed)
}

pub(crate) fn translate_asset_stem_to_english(stem: &str) -> Result<String, String> {
    let mut words = Vec::new();
    let mut unknown = BTreeSet::new();
    let mut rest = stem;
    while !rest.is_empty() {
        let Some(ch) = rest.chars().next() else {
            break;
        };
        if ch.is_ascii_alphanumeric() {
            let len = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .map(char::len_utf8)
                .sum::<usize>();
            words.push(rest[..len].to_ascii_lowercase());
            rest = &rest[len..];
            continue;
        }
        if ch.is_ascii()
            || ch.is_whitespace()
            || matches!(
                ch,
                '_' | '-'
                    | '—'
                    | '·'
                    | ' '
                    | '　'
                    | '（'
                    | '）'
                    | '【'
                    | '】'
                    | '《'
                    | '》'
                    | '，'
                    | '。'
                    | '、'
                    | '：'
                    | '；'
                    | '！'
                    | '？'
            )
        {
            rest = &rest[ch.len_utf8()..];
            continue;
        }
        if let Some((cn, en)) = chinese_asset_terms()
            .iter()
            .find(|(cn, _)| rest.starts_with(*cn))
        {
            words.push((*en).to_string());
            rest = &rest[cn.len()..];
            continue;
        }
        if let Some(en) = chinese_asset_char_word(ch) {
            words.push(en.to_string());
        } else {
            unknown.insert(ch);
        }
        rest = &rest[ch.len_utf8()..];
    }
    if !unknown.is_empty() {
        let unknown = unknown.into_iter().collect::<String>();
        return Err(format!(
            "cannot translate image filename `{stem}` into an English semantic name; add dictionary terms for: {unknown}"
        ));
    }
    let joined = words.join("_");
    let slug = sanitize_identifier_part(&joined, "");
    if slug.is_empty() || slug == "asset" || slug == "icon" {
        return Err(format!(
            "cannot translate image filename `{stem}` into a meaningful English semantic name"
        ));
    }
    Ok(slug)
}

pub(crate) fn chinese_asset_terms() -> &'static [(&'static str, &'static str)] {
    &[
        ("新经济政策", "new_economic_policy"),
        ("五年计划", "five_year_plan"),
        ("快速工业化", "rapid_industrialization"),
        ("集体化", "collectivization"),
        ("农业集体化", "agricultural_collectivization"),
        ("土地改革", "land_reform"),
        ("经济改革", "economic_reform"),
        ("政治改革", "political_reform"),
        ("军事改革", "military_reform"),
        ("工业化", "industrialization"),
        ("现代化", "modernization"),
        ("国有化", "nationalization"),
        ("私有化", "privatization"),
        ("重建", "rebuild"),
        ("建设", "construction"),
        ("发展", "development"),
        ("复兴", "revival"),
        ("振兴", "revitalization"),
        ("稳定", "stability"),
        ("统一", "unification"),
        ("独立", "independence"),
        ("革命", "revolution"),
        ("反革命", "counter_revolution"),
        ("宪法", "constitution"),
        ("委员会", "committee"),
        ("最高苏维埃", "supreme_soviet"),
        ("苏维埃", "soviet"),
        ("人民委员会", "people_committee"),
        ("人民代表大会", "people_congress"),
        ("代表大会", "congress"),
        ("国民大会", "national_assembly"),
        ("中央委员会", "central_committee"),
        ("政治局", "politburo"),
        ("内务部", "interior_ministry"),
        ("外交部", "foreign_ministry"),
        ("财政部", "finance_ministry"),
        ("工业部", "industry_ministry"),
        ("农业部", "agriculture_ministry"),
        ("国防部", "defense_ministry"),
        ("总参谋部", "general_staff"),
        ("铁路", "railway"),
        ("西伯利亚", "siberia"),
        ("远东", "far_east"),
        ("东北", "northeast"),
        ("东南", "southeast"),
        ("西北", "northwest"),
        ("西南", "southwest"),
        ("华北", "north_china"),
        ("华南", "south_china"),
        ("华东", "east_china"),
        ("华中", "central_china"),
        ("中亚", "central_asia"),
        ("东亚", "east_asia"),
        ("南亚", "south_asia"),
        ("欧洲", "europe"),
        ("亚洲", "asia"),
        ("美洲", "america"),
        ("非洲", "africa"),
        ("首都", "capital"),
        ("边疆", "frontier"),
        ("边境", "border"),
        ("防线", "defense_line"),
        ("要塞", "fortress"),
        ("军队", "army"),
        ("陆军", "army"),
        ("海军", "navy"),
        ("空军", "air_force"),
        ("红军", "red_army"),
        ("国民军", "national_army"),
        ("游击队", "partisans"),
        ("民兵", "militia"),
        ("骑兵", "cavalry"),
        ("装甲", "armor"),
        ("坦克", "tank"),
        ("炮兵", "artillery"),
        ("步兵", "infantry"),
        ("将军", "general"),
        ("元帅", "marshal"),
        ("动员", "mobilization"),
        ("征兵", "conscription"),
        ("军工", "military_industry"),
        ("民工", "civilian_industry"),
        ("工厂", "factory"),
        ("兵工厂", "arsenal"),
        ("造船厂", "dockyard"),
        ("船坞", "dockyard"),
        ("炼油厂", "refinery"),
        ("钢铁", "steel"),
        ("石油", "oil"),
        ("橡胶", "rubber"),
        ("铝", "aluminum"),
        ("钨", "tungsten"),
        ("铬", "chromium"),
        ("煤", "coal"),
        ("资源", "resources"),
        ("农业", "agriculture"),
        ("农民", "peasants"),
        ("粮食", "grain"),
        ("工业", "industry"),
        ("商业", "commerce"),
        ("贸易", "trade"),
        ("金融", "finance"),
        ("银行", "bank"),
        ("市场", "market"),
        ("计划经济", "planned_economy"),
        ("自由市场", "free_market"),
        ("基础设施", "infrastructure"),
        ("科研", "research"),
        ("科技", "technology"),
        ("教育", "education"),
        ("大学", "university"),
        ("学院", "academy"),
        ("宣传", "propaganda"),
        ("报纸", "newspaper"),
        ("电台", "radio"),
        ("广播", "broadcast"),
        ("情报", "intelligence"),
        ("特务", "secret_police"),
        ("警察", "police"),
        ("安全", "security"),
        ("叛乱", "rebellion"),
        ("内战", "civil_war"),
        ("战争", "war"),
        ("和平", "peace"),
        ("停战", "ceasefire"),
        ("同盟", "alliance"),
        ("外交", "diplomacy"),
        ("协议", "agreement"),
        ("条约", "treaty"),
        ("会议", "conference"),
        ("选举", "election"),
        ("政党", "party"),
        ("工会", "union"),
        ("青年", "youth"),
        ("妇女", "women"),
        ("民族", "nation"),
        ("国家", "state"),
        ("人民", "people"),
        ("共和国", "republic"),
        ("帝国", "empire"),
        ("王国", "kingdom"),
        ("政府", "government"),
        ("临时政府", "provisional_government"),
        ("共和国政府", "republican_government"),
        ("危机", "crisis"),
        ("胜利", "victory"),
        ("失败", "defeat"),
        ("事件", "event"),
        ("决议", "decision"),
        ("国策", "focus"),
        ("民族精神", "national_spirit"),
        ("精神", "spirit"),
        ("图标", "icon"),
        ("默认", "default"),
        ("通用", "generic"),
    ]
}

pub(crate) fn chinese_asset_char_word(ch: char) -> Option<&'static str> {
    match ch {
        '东' => Some("east"),
        '南' => Some("south"),
        '西' => Some("west"),
        '北' => Some("north"),
        '中' => Some("central"),
        '华' => Some("china"),
        '国' => Some("country"),
        '民' => Some("people"),
        '人' => Some("people"),
        '党' => Some("party"),
        '军' => Some("army"),
        '兵' => Some("soldier"),
        '工' => Some("industry"),
        '农' => Some("farm"),
        '铁' => Some("rail"),
        '路' => Some("road"),
        '油' => Some("oil"),
        '钢' => Some("steel"),
        '船' => Some("ship"),
        '海' => Some("sea"),
        '空' => Some("air"),
        '陆' => Some("land"),
        '政' => Some("politics"),
        '经' => Some("economy"),
        '革' => Some("reform"),
        '改' => Some("reform"),
        '命' => Some("revolution"),
        '建' => Some("build"),
        '重' => Some("rebuild"),
        '新' => Some("new"),
        '旧' => Some("old"),
        '红' => Some("red"),
        '白' => Some("white"),
        '黑' => Some("black"),
        '蓝' => Some("blue"),
        '黄' => Some("yellow"),
        '绿' => Some("green"),
        '会' => Some("council"),
        '法' => Some("law"),
        '权' => Some("power"),
        '土' => Some("land"),
        '地' => Some("land"),
        '资' => Some("capital"),
        '本' => Some("capital"),
        '社' => Some("society"),
        '安' => Some("security"),
        '全' => Some("security"),
        '战' => Some("war"),
        '和' => Some("peace"),
        '平' => Some("peace"),
        '胜' => Some("victory"),
        '利' => Some("benefit"),
        '危' => Some("crisis"),
        '机' => Some("machine"),
        _ => None,
    }
}

pub(crate) fn sprite_base_from_gfx_asset(image_root: &Path, file: &Path, prefix: &str) -> String {
    let rel = file.strip_prefix(image_root).unwrap_or(file);
    let raw = slash_path(&rel.with_extension(""));
    let mut base = sanitize_identifier_part(&raw, "");
    if base.is_empty() {
        base = "asset".to_string();
    }
    let prefix_with_sep = format!("{prefix}_");
    while let Some(rest) = base.strip_prefix(&prefix_with_sep) {
        if rest.is_empty() {
            break;
        }
        base = rest.to_string();
    }
    base
}

pub(crate) fn sprite_lookup(root: &Path) -> Result<SpriteLookup, String> {
    let mut lookup = SpriteLookup::default();
    for sprite in scan_interface_sprites(root)? {
        if sprite.name.is_empty() || sprite.texturefile.is_empty() {
            continue;
        }
        let texture_key = normalize_texture_key(&sprite.texturefile);
        lookup
            .name_to_texture
            .insert(sprite.name.clone(), texture_key.clone());
        lookup
            .texture_to_names
            .entry(texture_key)
            .or_default()
            .push(sprite.name);
    }
    for names in lookup.texture_to_names.values_mut() {
        names.sort();
        names.dedup();
    }
    Ok(lookup)
}

pub(crate) fn normalize_texture_key(texturefile: &str) -> String {
    texturefile
        .trim()
        .trim_matches('"')
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_ascii_lowercase()
}

pub(crate) fn compose_sprite_name(name_prefix: &str, prefix: &str, base: &str) -> String {
    if name_prefix == "GFX" {
        format!("GFX_{prefix}_{base}")
    } else {
        format!("{name_prefix}_{prefix}_{base}")
    }
}

pub(crate) fn reserve_sprite_name(
    candidate: &str,
    texture_key: &str,
    lookup: &SpriteLookup,
    used_names: &mut BTreeSet<String>,
) -> (String, String, Option<String>) {
    let mut suffix = 2usize;
    let mut name = candidate.to_string();
    let mut conflict = None;
    loop {
        if let Some(existing_texture) = lookup.name_to_texture.get(&name) {
            if existing_texture == texture_key {
                return (name, "existing".to_string(), conflict);
            }
            if conflict.is_none() {
                conflict = Some(format!("{name} already points to {existing_texture}"));
            }
        } else if !used_names.contains(&name) {
            used_names.insert(name.clone());
            let status = if conflict.is_some() {
                "renamed"
            } else {
                "added"
            };
            return (name, status.to_string(), conflict);
        }
        name = format!("{candidate}_{suffix}");
        suffix += 1;
    }
}

pub(crate) fn render_sprite_type_block(
    name: &str,
    texturefile: &str,
    remark: &str,
    render_kind: GfxSpriteRenderKind,
) -> String {
    match render_kind {
        GfxSpriteRenderKind::DynamicGui => {
            let no_of_frames = dynamic_gui_frame_count(name, texturefile)
                .map(|count| format!("\n\t\tnoOfFrames = {count}"))
                .unwrap_or_default();
            format!(
                "\t# source_texturefile = {}\n\t# {}\n\tspriteType = {{\n\t\tname = {}\n\t\ttexturefile = {}\n\t\tlegacy_lazy_load = no{}\n\t}}\n",
                texturefile,
                remark.replace('\n', " "),
                hoi4_quote(name),
                hoi4_quote(texturefile),
                no_of_frames
            )
        }
        GfxSpriteRenderKind::StandardLower => format!(
            "\t# source_texturefile = {}\n\t# {}\n\tspriteType = {{\n\t\tname = {}\n\t\ttexturefile = {}\n\t}}\n",
            texturefile,
            remark.replace('\n', " "),
            hoi4_quote(name),
            hoi4_quote(texturefile)
        ),
        GfxSpriteRenderKind::GoalUpper => format!(
            "\t# source_texturefile = {}\n\t# {}\n\tSpriteType = {{\n\t\tname = {}\n\t\ttexturefile = {}\n\t}}\n",
            texturefile,
            remark.replace('\n', " "),
            hoi4_quote(name),
            hoi4_quote(texturefile)
        ),
        GfxSpriteRenderKind::GoalShine => format!(
            "\t# source_texturefile = {}\n\t# {}\n\tspriteType = {{\n\t\tname = {}\n\t\ttexturefile = {}\n\t\teffectFile = \"gfx/FX/buttonstate.lua\"\n\t\tanimation = {{\n\t\t\tanimationmaskfile = {}\n\t\t\tanimationtexturefile = \"gfx/interface/goals/shine_overlay.dds\"\n\t\t\tanimationrotation = -90.0\n\t\t\tanimationlooping = no\n\t\t\tanimationtime = 0.75\n\t\t\tanimationdelay = 0\n\t\t\tanimationblendmode = \"add\"\n\t\t\tanimationtype = \"scrolling\"\n\t\t\tanimationrotationoffset = {{ x = 0.0 y = 0.0 }}\n\t\t\tanimationtexturescale = {{ x = 1.0 y = 1.0 }}\n\t\t}}\n\t\tanimation = {{\n\t\t\tanimationmaskfile = {}\n\t\t\tanimationtexturefile = \"gfx/interface/goals/shine_overlay.dds\"\n\t\t\tanimationrotation = 90.0\n\t\t\tanimationlooping = no\n\t\t\tanimationtime = 0.75\n\t\t\tanimationdelay = 0\n\t\t\tanimationblendmode = \"add\"\n\t\t\tanimationtype = \"scrolling\"\n\t\t\tanimationrotationoffset = {{ x = 0.0 y = 0.0 }}\n\t\t\tanimationtexturescale = {{ x = 1.0 y = 1.0 }}\n\t\t}}\n\t\tlegacy_lazy_load = no\n\t}}\n",
            texturefile,
            remark.replace('\n', " "),
            hoi4_quote(name),
            hoi4_quote(texturefile),
            hoi4_quote(texturefile),
            hoi4_quote(texturefile)
        ),
    }
}

pub(crate) fn dynamic_gui_frame_count(name: &str, texturefile: &str) -> Option<usize> {
    let probe = format!(
        "{} {}",
        name.to_ascii_lowercase(),
        texturefile.to_ascii_lowercase()
    );
    (probe.contains("meter") || probe.contains("paranoia")).then_some(21)
}

pub(crate) fn gfx_registration_report_json(report: &GfxRegistrationReport) -> String {
    let added = report
        .entries
        .iter()
        .filter(|entry| entry.status == "added")
        .count();
    let existing = report
        .entries
        .iter()
        .filter(|entry| entry.status == "existing")
        .count();
    let renamed = report
        .entries
        .iter()
        .filter(|entry| entry.status == "renamed")
        .count();
    let assets_renamed = report
        .entries
        .iter()
        .filter(|entry| entry.original_texturefile.is_some())
        .map(|entry| entry.texturefile.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"schema\": \"hoi4skill.gfx_registration.v1\",\n");
    out.push_str(&format!(
        "  \"mod_root\": {},\n",
        json_str(&report.mod_root.display().to_string())
    ));
    out.push_str(&format!("  \"prefix\": {},\n", json_str(&report.prefix)));
    out.push_str(&format!(
        "  \"categories\": {},\n",
        json_array(&report.categories)
    ));
    out.push_str(&format!(
        "  \"assets_scanned\": {},\n",
        report.assets_scanned
    ));
    out.push_str(&format!("  \"sprites_added\": {},\n", added + renamed));
    out.push_str(&format!("  \"sprites_existing\": {},\n", existing));
    out.push_str(&format!("  \"name_conflicts_avoided\": {},\n", renamed));
    out.push_str(&format!(
        "  \"assets_renamed_to_english\": {},\n",
        assets_renamed
    ));
    out.push_str(&format!(
        "  \"assets_skipped\": {},\n",
        report.skipped_assets.len()
    ));
    out.push_str("  \"changed_files\": [");
    for (idx, file) in report.changed_files.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        out.push_str(&json_str(&file.display().to_string()));
    }
    out.push_str("],\n");
    out.push_str("  \"skipped_assets\": [\n");
    for (idx, skipped) in report.skipped_assets.iter().enumerate() {
        out.push_str("    {\n");
        out.push_str(&format!(
            "      \"texturefile\": {},\n",
            json_str(&skipped.texturefile)
        ));
        out.push_str(&format!(
            "      \"local_file_name\": {},\n",
            json_str(&skipped.local_file_name)
        ));
        out.push_str(&format!(
            "      \"reason\": {},\n",
            json_str(&skipped.reason)
        ));
        out.push_str(&format!(
            "      \"required_action\": {}\n",
            json_str(&skipped.required_action)
        ));
        out.push_str("    }");
        if idx + 1 < report.skipped_assets.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ],\n");
    out.push_str("  \"entries\": [\n");
    for (idx, entry) in report.entries.iter().enumerate() {
        out.push_str("    {\n");
        out.push_str(&format!(
            "      \"category\": {},\n",
            json_str(&entry.category)
        ));
        out.push_str(&format!(
            "      \"sprite_name\": {},\n",
            json_str(&entry.sprite_name)
        ));
        out.push_str(&format!(
            "      \"texturefile\": {},\n",
            json_str(&entry.texturefile)
        ));
        out.push_str(&format!(
            "      \"original_texturefile\": {},\n",
            json_optional_str(entry.original_texturefile.as_deref())
        ));
        out.push_str(&format!(
            "      \"local_file_name\": {},\n",
            json_str(&entry.local_file_name)
        ));
        out.push_str(&format!(
            "      \"english_file_name\": {},\n",
            json_str(&entry.english_file_name)
        ));
        out.push_str(&format!(
            "      \"file\": {},\n",
            json_str(&entry.file.display().to_string())
        ));
        out.push_str(&format!("      \"status\": {},\n", json_str(&entry.status)));
        out.push_str(&format!(
            "      \"english_name_instruction\": {},\n",
            json_str(&entry.english_name_instruction)
        ));
        out.push_str(&format!("      \"remark\": {},\n", json_str(&entry.remark)));
        out.push_str(&format!(
            "      \"existing_names_for_texture\": {},\n",
            json_array(&entry.existing_names)
        ));
        out.push_str(&format!(
            "      \"conflict\": {}\n",
            json_optional_str(entry.conflict.as_deref())
        ));
        out.push_str("    }");
        if idx + 1 < report.entries.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n");
    out.push_str("}\n");
    out
}

pub(crate) fn scan_sprites(root: &Path) -> Result<Vec<Sprite>, String> {
    let mut sprites = scan_interface_sprites(root)?;
    if sprites.is_empty() {
        for file in collect_files(root)? {
            let ext = file
                .extension()
                .and_then(OsStr::to_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            if ext == "png" || ext == "dds" {
                let rel = slash_path(file.strip_prefix(root).unwrap_or(&file));
                sprites.push(Sprite {
                    name: file
                        .file_stem()
                        .and_then(OsStr::to_str)
                        .unwrap_or("icon")
                        .to_string(),
                    texturefile: rel,
                });
            }
        }
    }
    Ok(sprites)
}

pub(crate) fn scan_interface_sprites(root: &Path) -> Result<Vec<Sprite>, String> {
    let mut sprites = Vec::new();
    let interface = root.join("interface");
    if interface.exists() {
        for file in collect_files(&interface)? {
            if file.extension().and_then(OsStr::to_str).unwrap_or("") == "gfx" {
                let text = read_utf8_lossy(&file)?;
                for block in sprite_type_blocks(&text) {
                    let name = block_assignment(&block, "name").unwrap_or_default();
                    let texturefile = block_assignment(&block, "texturefile").unwrap_or_default();
                    if !name.is_empty() || !texturefile.is_empty() {
                        sprites.push(Sprite { name, texturefile });
                    }
                }
            }
        }
    }
    Ok(sprites)
}

pub(crate) fn sprite_type_blocks(text: &str) -> Vec<String> {
    let mut blocks = blocks_named(text, "spriteType");
    blocks.extend(blocks_named(text, "SpriteType"));
    blocks
}
