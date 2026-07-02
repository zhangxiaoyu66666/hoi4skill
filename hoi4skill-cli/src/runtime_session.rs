//! P53 runtime session planning and evidence collection.
//!
//! This does not silently launch or click through HOI4. It records the exact
//! game/mod/log evidence needed for a runtime smoke and lets a later desktop
//! runner provide interactive automation.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_hoi4_runtime_session_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let game_root = normalize_path(&require_value(&map, "game-root")?)?;
    let launcher_dir = value(&map, "launcher-dir")
        .map(normalize_path)
        .transpose()?
        .unwrap_or_else(default_hoi4_launcher_dir);
    let logs_dir = value(&map, "logs-dir")
        .map(normalize_path)
        .transpose()?
        .unwrap_or_else(default_hoi4_logs_dir);
    let mut blockers = Vec::new();
    if !mod_root.exists() {
        blockers.push(format!("mod root `{}` does not exist", mod_root.display()));
    }
    if !game_root.exists() {
        blockers.push(format!(
            "game root `{}` does not exist",
            game_root.display()
        ));
    }
    if !game_root.join("hoi4.exe").exists() {
        blockers.push(format!(
            "hoi4.exe was not found under `{}`",
            game_root.display()
        ));
    }
    if !launcher_dir.exists() {
        blockers.push(format!(
            "launcher mod directory `{}` does not exist",
            launcher_dir.display()
        ));
    }
    let descriptor = mod_root.join("descriptor.mod");
    if !descriptor.exists() {
        blockers.push(format!(
            "target mod descriptor `{}` does not exist",
            descriptor.display()
        ));
    }
    let ok = blockers.is_empty();
    let report = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"mod_root\": {},\n  \"game_root\": {},\n  \"launcher_dir\": {},\n  \"logs_dir\": {},\n  \"hoi4_exe\": {},\n  \"expected_logs\": {},\n  \"blocking_count\": {},\n  \"blockers\": {},\n  \"manual_steps\": {},\n  \"next_commands\": {},\n  \"rules\": {}\n}}\n",
        json_str("hoi4skill.hoi4_runtime_session_plan.v1"),
        json_bool(ok),
        json_str(if ok { "runtime_session_plan_ready" } else { "blocked" }),
        json_str(&mod_root.display().to_string()),
        json_str(&game_root.display().to_string()),
        json_str(&launcher_dir.display().to_string()),
        json_str(&logs_dir.display().to_string()),
        json_str(&game_root.join("hoi4.exe").display().to_string()),
        json_array(&[
            logs_dir.join("error.log").display().to_string(),
            logs_dir.join("map.log").display().to_string(),
            logs_dir.join("setup.log").display().to_string(),
        ]),
        blockers.len(),
        json_array(&blockers),
        json_array(&[
            "enable the target mod and required parent mods in the launcher".to_string(),
            "launch HOI4 to the main menu and wait until logs stop changing".to_string(),
            "run hoi4-runtime-session-runner with --manual-confirmed and the plan report".to_string(),
        ]),
        json_array(&["hoi4skill hoi4-runtime-session-runner --input runtime_session_plan.json --manual-confirmed --require-passed".to_string()]),
        json_array(&[
            "runtime evidence must prove the game, mod, and logs were checked after this plan was created".to_string(),
            "runner is evidence collection first; direct process automation belongs to the desktop/runtime runner layer".to_string(),
        ])
    );
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_hoi4_runtime_session_runner(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let plan = read_utf8_lossy(&input)?;
    let mut blockers = Vec::new();
    if !plan.contains("\"schema\": \"hoi4skill.hoi4_runtime_session_plan.v1\"") {
        blockers.push("input is not a hoi4-runtime-session-plan report".to_string());
    }
    if !plan.contains("\"ok\": true") {
        blockers.push("runtime session plan is not ok".to_string());
    }
    if !map.flags.contains("manual-confirmed") {
        blockers.push("hoi4-runtime-session-runner requires --manual-confirmed after launching HOI4 with the mod enabled".to_string());
    }
    let logs_dir = json_string_field(&plan, "logs_dir")
        .map(PathBuf::from)
        .or_else(|| value(&map, "logs-dir").map(PathBuf::from))
        .unwrap_or_else(default_hoi4_logs_dir);
    let mut log_rows = Vec::new();
    for name in ["error.log", "map.log", "setup.log"] {
        let path = logs_dir.join(name);
        let exists = path.exists();
        let bytes = if exists {
            fs::metadata(&path)
                .map(|metadata| metadata.len())
                .unwrap_or(0)
        } else {
            0
        };
        if !exists {
            blockers.push(format!(
                "expected runtime log `{}` does not exist",
                path.display()
            ));
        }
        log_rows.push(format!(
            "{{\"path\": {}, \"exists\": {}, \"bytes\": {}}}",
            json_str(&path.display().to_string()),
            json_bool(exists),
            bytes
        ));
    }
    let ok = blockers.is_empty();
    let report = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"input\": {},\n  \"manual_confirmed\": {},\n  \"logs_dir\": {},\n  \"logs\": [{}],\n  \"blocking_count\": {},\n  \"blockers\": {},\n  \"next_commands\": {},\n  \"rules\": {}\n}}\n",
        json_str("hoi4skill.hoi4_runtime_session_runner.v1"),
        json_bool(ok),
        json_str(if ok { "runtime_session_evidence_ready" } else { "blocked" }),
        json_str(&input.display().to_string()),
        json_bool(map.flags.contains("manual-confirmed")),
        json_str(&logs_dir.display().to_string()),
        log_rows.join(", "),
        blockers.len(),
        json_array(&blockers),
        json_array(&["hoi4skill map-runtime-gate --error-log <error.log> --map-log <map.log> --setup-log <setup.log> --require-passed".to_string()]),
        json_array(&[
            "logs must exist after the user or desktop runner launched HOI4 with the target mod enabled".to_string(),
            "map-runtime-gate remains the blocker for new runtime errors".to_string(),
        ])
    );
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

fn default_hoi4_launcher_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Documents")
        .join("Paradox Interactive")
        .join("Hearts of Iron IV")
        .join("mod")
}

fn default_hoi4_logs_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Documents")
        .join("Paradox Interactive")
        .join("Hearts of Iron IV")
        .join("logs")
}
