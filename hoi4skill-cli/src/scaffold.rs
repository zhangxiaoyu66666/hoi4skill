//! HOI4 mod scaffolding and descriptor writing.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_scaffold(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let name = require_value(&map, "name")?;
    let output = require_value(&map, "output")?;
    let version = value(&map, "version").unwrap_or("0.1.0");
    let supported_version = value(&map, "supported-version").unwrap_or("*");
    let tags = value(&map, "tags").unwrap_or("Alternative History");
    let mod_root = normalize_path(&output)?;
    let created = scaffold_mod(
        &mod_root,
        &name,
        version,
        supported_version,
        tags,
        map.flags.contains("launcher-file"),
    )?;

    println!("Mod root: {}", mod_root.display());
    if created.is_empty() {
        println!("No new files or directories were needed.");
    } else {
        println!("Created:");
        for path in created {
            println!("  {}", path.display());
        }
    }
    Ok(())
}

pub(crate) fn scaffold_mod(
    mod_root: &Path,
    name: &str,
    version: &str,
    supported_version: &str,
    tags: &str,
    launcher_file: bool,
) -> Result<Vec<PathBuf>, String> {
    fs::create_dir_all(mod_root).map_err(|e| format!("create {}: {e}", mod_root.display()))?;

    let mut created = Vec::new();
    for rel in HOI4_PROFILE.default_mod_dirs {
        let path = mod_root.join(rel);
        if !path.exists() {
            fs::create_dir_all(&path).map_err(|e| format!("create {}: {e}", path.display()))?;
            created.push(path);
        }
    }

    let tag_values: Vec<String> = tags
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let descriptor = descriptor_content(name, version, supported_version, &tag_values, None);
    let descriptor_path = mod_root.join("descriptor.mod");
    if write_if_missing(&descriptor_path, descriptor.as_bytes())? {
        created.push(descriptor_path);
    }

    let mod_id = slugify(
        mod_root.file_name().and_then(OsStr::to_str).unwrap_or(name),
        "hoi4_mod",
    );

    if launcher_file {
        let launcher = descriptor_content(
            name,
            version,
            supported_version,
            &tag_values,
            Some(&mod_root.to_string_lossy().replace('\\', "/")),
        );
        let launcher_path = mod_root
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!("{mod_id}.mod"));
        if write_if_missing(&launcher_path, launcher.as_bytes())? {
            created.push(launcher_path);
        }
    }

    Ok(created)
}

pub(crate) fn descriptor_content(
    name: &str,
    version: &str,
    supported_version: &str,
    tags: &[String],
    path: Option<&str>,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("name={}\n", hoi4_quote(name)));
    if let Some(path) = path {
        out.push_str(&format!("path={}\n", hoi4_quote(path)));
    }
    out.push_str(&format!("version={}\n", hoi4_quote(version)));
    out.push_str(&format!(
        "supported_version={}\n",
        hoi4_quote(supported_version)
    ));
    out.push_str("tags={\n");
    for tag in tags {
        out.push_str(&format!("\t{}\n", hoi4_quote(tag)));
    }
    out.push_str("}\n");
    out
}
