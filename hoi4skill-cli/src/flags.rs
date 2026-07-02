//! Flag image import helpers.
//!
//! HOI4 country flags must be shipped as a normal/medium/small TGA triplet. This
//! command keeps that repetitive asset work inside the CLI instead of asking an
//! LLM to invent paths or image dimensions.

#[allow(unused_imports)]
use crate::*;

const FLAG_NORMAL_WIDTH: u32 = 82;
const FLAG_NORMAL_HEIGHT: u32 = 52;
const FLAG_MEDIUM_WIDTH: u32 = 41;
const FLAG_MEDIUM_HEIGHT: u32 = 26;
const FLAG_SMALL_WIDTH: u32 = 10;
const FLAG_SMALL_HEIGHT: u32 = 7;

#[derive(Clone)]
struct FlagImage {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

struct FlagOutput {
    kind: &'static str,
    path: PathBuf,
    width: u32,
    height: u32,
    written: bool,
}

pub(crate) fn cmd_flag_image_import(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let input_raw = value(&map, "input")
        .or_else(|| value(&map, "file"))
        .ok_or_else(|| "flag-image-import requires --input or --file".to_string())?;
    let input = normalize_path(input_raw)?;
    let flag_id = resolve_flag_import_id(&map)?;
    let execute = map.flags.contains("execute");
    let overwrite = map.flags.contains("overwrite");
    let mut blockers = Vec::new();
    let mut questions = Vec::new();

    if !is_identifier_like(&flag_id) {
        blockers.push(format!(
            "flag id `{flag_id}` is not a safe HOI4 asset identifier"
        ));
    }
    if !input.is_file() {
        blockers.push(format!("source image `{}` is missing", input.display()));
    }
    if !mod_root.exists() {
        questions.push(format!(
            "mod root `{}` does not exist yet; --execute will create flag directories under it",
            mod_root.display()
        ));
    }

    let outputs = flag_triplet_outputs(&mod_root, &flag_id, false);
    if !overwrite {
        for output in &outputs {
            if output.path.exists() {
                blockers.push(format!(
                    "target flag asset `{}` already exists; pass --overwrite to replace it",
                    output.path.display()
                ));
            }
        }
    }

    let decoded = if blockers.iter().any(|item| item.contains("source image")) {
        None
    } else {
        match decode_flag_source(&input) {
            Ok(image) => Some(image),
            Err(err) => {
                blockers.push(err);
                None
            }
        }
    };

    let mut outputs = flag_triplet_outputs(&mod_root, &flag_id, false);
    let ok = blockers.is_empty();
    if execute && ok {
        let image = decoded
            .as_ref()
            .ok_or_else(|| "source image was not decoded".to_string())?;
        for output in &mut outputs {
            let resized = resize_nearest_rgba(image, output.width, output.height)?;
            write_tga_rgba(&output.path, output.width, output.height, &resized)?;
            output.written = true;
        }
    }

    let status = if !ok {
        "blocked"
    } else if execute {
        "flag_triplet_written"
    } else {
        "flag_triplet_plan_ready"
    };
    let json = flag_image_import_json(
        ok,
        status,
        execute,
        overwrite,
        &input,
        decoded.as_ref(),
        &mod_root,
        &flag_id,
        &outputs,
        &blockers,
        &questions,
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

fn resolve_flag_import_id(map: &ArgMap) -> Result<String, String> {
    if let Some(flag_id) = value(map, "flag-id") {
        return Ok(flag_id.to_string());
    }
    if let Some(cosmetic) = value(map, "cosmetic") {
        return Ok(cosmetic.to_string());
    }
    let tag = require_value(map, "tag")?;
    if let Some(ideology) = value(map, "ideology") {
        return Ok(format!("{tag}_{ideology}"));
    }
    Ok(tag)
}

fn flag_triplet_outputs(mod_root: &Path, flag_id: &str, written: bool) -> Vec<FlagOutput> {
    vec![
        FlagOutput {
            kind: "normal",
            path: mod_root
                .join("gfx")
                .join("flags")
                .join(format!("{flag_id}.tga")),
            width: FLAG_NORMAL_WIDTH,
            height: FLAG_NORMAL_HEIGHT,
            written,
        },
        FlagOutput {
            kind: "medium",
            path: mod_root
                .join("gfx")
                .join("flags")
                .join("medium")
                .join(format!("{flag_id}.tga")),
            width: FLAG_MEDIUM_WIDTH,
            height: FLAG_MEDIUM_HEIGHT,
            written,
        },
        FlagOutput {
            kind: "small",
            path: mod_root
                .join("gfx")
                .join("flags")
                .join("small")
                .join(format!("{flag_id}.tga")),
            width: FLAG_SMALL_WIDTH,
            height: FLAG_SMALL_HEIGHT,
            written,
        },
    ]
}

fn decode_flag_source(path: &Path) -> Result<FlagImage, String> {
    let extension = path
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    match extension.as_str() {
        "png" => decode_png_rgba(path),
        "jpg" | "jpeg" | "webp" => decode_common_image_rgba(path),
        "tga" => decode_tga_rgba(path),
        other => Err(format!(
            "unsupported flag source format `{other}` for `{}`; provide PNG, JPG, JPEG, WEBP, or uncompressed truecolor TGA",
            path.display()
        )),
    }
}

fn decode_common_image_rgba(path: &Path) -> Result<FlagImage, String> {
    let reader = image::ImageReader::open(path)
        .map_err(|e| format!("open image {}: {e}", path.display()))?
        .with_guessed_format()
        .map_err(|e| format!("detect image format {}: {e}", path.display()))?;
    let image = reader
        .decode()
        .map_err(|e| format!("decode image {}: {e}", path.display()))?;
    let rgba = image.to_rgba8();
    let width = rgba.width();
    let height = rgba.height();
    let rgba = rgba.into_raw();
    ensure_rgba_len(path, width, height, &rgba)?;
    Ok(FlagImage {
        width,
        height,
        rgba,
    })
}

fn decode_png_rgba(path: &Path) -> Result<FlagImage, String> {
    let file = fs::File::open(path).map_err(|e| format!("open PNG {}: {e}", path.display()))?;
    let mut decoder = png::Decoder::new(file);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("decode PNG {}: {e}", path.display()))?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|e| format!("read PNG frame {}: {e}", path.display()))?;
    let bytes = &buffer[..info.buffer_size()];
    let mut rgba = Vec::with_capacity((info.width as usize) * (info.height as usize) * 4);
    match info.color_type {
        png::ColorType::Rgba => rgba.extend_from_slice(bytes),
        png::ColorType::Rgb => {
            for chunk in bytes.chunks_exact(3) {
                rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
        }
        png::ColorType::Grayscale => {
            for gray in bytes {
                rgba.extend_from_slice(&[*gray, *gray, *gray, 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for chunk in bytes.chunks_exact(2) {
                rgba.extend_from_slice(&[chunk[0], chunk[0], chunk[0], chunk[1]]);
            }
        }
        png::ColorType::Indexed => {
            return Err(format!(
                "indexed PNG `{}` was not expanded by the decoder",
                path.display()
            ));
        }
    }
    ensure_rgba_len(path, info.width, info.height, &rgba)?;
    Ok(FlagImage {
        width: info.width,
        height: info.height,
        rgba,
    })
}

fn decode_tga_rgba(path: &Path) -> Result<FlagImage, String> {
    let bytes = fs::read(path).map_err(|e| format!("read TGA {}: {e}", path.display()))?;
    if bytes.len() < 18 {
        return Err(format!(
            "TGA `{}` header is shorter than 18 bytes",
            path.display()
        ));
    }
    let id_len = bytes[0] as usize;
    let color_map_type = bytes[1];
    let image_type = bytes[2];
    if color_map_type != 0 || image_type != 2 {
        return Err(format!(
            "TGA `{}` must be uncompressed truecolor image type 2",
            path.display()
        ));
    }
    let width = u16::from_le_bytes([bytes[12], bytes[13]]) as u32;
    let height = u16::from_le_bytes([bytes[14], bytes[15]]) as u32;
    let pixel_depth = bytes[16];
    if !matches!(pixel_depth, 24 | 32) {
        return Err(format!(
            "TGA `{}` must be 24-bit or 32-bit, got {pixel_depth}",
            path.display()
        ));
    }
    if width == 0 || height == 0 {
        return Err(format!("TGA `{}` has zero dimensions", path.display()));
    }
    let bytes_per_pixel = (pixel_depth / 8) as usize;
    let pixel_start = 18usize
        .checked_add(id_len)
        .ok_or_else(|| format!("TGA `{}` has invalid ID length", path.display()))?;
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(bytes_per_pixel))
        .and_then(|len| len.checked_add(pixel_start))
        .ok_or_else(|| format!("TGA `{}` dimensions overflow", path.display()))?;
    if bytes.len() < expected {
        return Err(format!(
            "TGA `{}` pixel data is shorter than expected",
            path.display()
        ));
    }
    let top_origin = bytes[17] & 0x20 != 0;
    let mut rgba = vec![0u8; (width as usize) * (height as usize) * 4];
    for y in 0..height as usize {
        let source_y = if top_origin {
            y
        } else {
            height as usize - 1 - y
        };
        for x in 0..width as usize {
            let source = pixel_start + ((source_y * width as usize + x) * bytes_per_pixel);
            let target = (y * width as usize + x) * 4;
            rgba[target] = bytes[source + 2];
            rgba[target + 1] = bytes[source + 1];
            rgba[target + 2] = bytes[source];
            rgba[target + 3] = if bytes_per_pixel == 4 {
                bytes[source + 3]
            } else {
                255
            };
        }
    }
    Ok(FlagImage {
        width,
        height,
        rgba,
    })
}

fn ensure_rgba_len(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| format!("image `{}` dimensions overflow", path.display()))?;
    if rgba.len() != expected {
        return Err(format!(
            "image `{}` decoded to {} RGBA bytes, expected {expected}",
            path.display(),
            rgba.len()
        ));
    }
    if width == 0 || height == 0 {
        return Err(format!("image `{}` has zero dimensions", path.display()));
    }
    Ok(())
}

fn resize_nearest_rgba(
    image: &FlagImage,
    target_width: u32,
    target_height: u32,
) -> Result<Vec<u8>, String> {
    ensure_rgba_len(
        Path::new("<decoded flag source>"),
        image.width,
        image.height,
        &image.rgba,
    )?;
    let mut out = vec![0u8; (target_width as usize) * (target_height as usize) * 4];
    for y in 0..target_height as usize {
        let source_y = y * image.height as usize / target_height as usize;
        for x in 0..target_width as usize {
            let source_x = x * image.width as usize / target_width as usize;
            let source = (source_y * image.width as usize + source_x) * 4;
            let target = (y * target_width as usize + x) * 4;
            out[target..target + 4].copy_from_slice(&image.rgba[source..source + 4]);
        }
    }
    Ok(out)
}

fn write_tga_rgba(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
    if width > u16::MAX as u32 || height > u16::MAX as u32 {
        return Err(format!(
            "TGA `{}` dimensions exceed u16 header limits",
            path.display()
        ));
    }
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| format!("TGA `{}` dimensions overflow", path.display()))?;
    if rgba.len() != expected {
        return Err(format!(
            "TGA `{}` got {} RGBA bytes, expected {expected}",
            path.display(),
            rgba.len()
        ));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    let mut bytes = vec![0u8; 18 + rgba.len()];
    bytes[2] = 2;
    bytes[12..14].copy_from_slice(&(width as u16).to_le_bytes());
    bytes[14..16].copy_from_slice(&(height as u16).to_le_bytes());
    bytes[16] = 32;
    bytes[17] = 0x28;
    let mut offset = 18usize;
    for chunk in rgba.chunks_exact(4) {
        bytes[offset] = chunk[2];
        bytes[offset + 1] = chunk[1];
        bytes[offset + 2] = chunk[0];
        bytes[offset + 3] = chunk[3];
        offset += 4;
    }
    fs::write(path, bytes).map_err(|e| format!("write {}: {e}", path.display()))
}

fn flag_image_import_json(
    ok: bool,
    status: &str,
    executed: bool,
    overwrite: bool,
    input: &Path,
    image: Option<&FlagImage>,
    mod_root: &Path,
    flag_id: &str,
    outputs: &[FlagOutput],
    blockers: &[String],
    questions: &[String],
) -> String {
    let source_width = image
        .map(|image| image.width.to_string())
        .unwrap_or_else(|| "null".to_string());
    let source_height = image
        .map(|image| image.height.to_string())
        .unwrap_or_else(|| "null".to_string());
    format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"executed\": {},\n  \"overwrite\": {},\n  \"source\": {},\n  \"source_width\": {},\n  \"source_height\": {},\n  \"mod_root\": {},\n  \"flag_id\": {},\n  \"outputs\": [{}],\n  \"blockers\": {},\n  \"questions\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.flag_image_import.v1"),
        json_bool(ok),
        json_str(status),
        json_bool(executed),
        json_bool(overwrite),
        json_str(&input.display().to_string()),
        source_width,
        source_height,
        json_str(&mod_root.display().to_string()),
        json_str(flag_id),
        outputs
            .iter()
            .map(flag_output_json)
            .collect::<Vec<_>>()
            .join(", "),
        json_array(blockers),
        json_array(questions),
        json_str("country flags must be emitted as a complete normal 82x52, medium 41x26, and small 10x7 TGA triplet; existing assets require --overwrite")
    )
}

fn flag_output_json(output: &FlagOutput) -> String {
    format!(
        "{{\"kind\": {}, \"path\": {}, \"width\": {}, \"height\": {}, \"written\": {}, \"exists_after\": {}}}",
        json_str(output.kind),
        json_str(&output.path.display().to_string()),
        output.width,
        output.height,
        json_bool(output.written),
        json_bool(output.path.exists()),
    )
}
