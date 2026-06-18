use crate::*;

fn unique_temp_dir(name: &str) -> PathBuf {
    env::temp_dir().join(format!(
        "hoi4skill-{name}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn write_test_skill(path: &Path, name: &str) {
    fs::create_dir_all(path).unwrap();
    fs::write(
        path.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: test\n---\n"),
    )
    .unwrap();
}

#[test]
fn large_mod_blueprint_round_trips_generated_yaml() {
    let source = "name: New Order Rising\ncountries: RUS, GER\nregions: east asia, europe\nsystems: black monday, faction congress";
    let blueprint = plan_large_mod_blueprint(source, "New Order Rising", "NOR", "simp_chinese");
    let yaml = blueprint.to_yaml();
    let parsed = parse_large_mod_blueprint(&yaml).unwrap();

    assert_eq!(parsed.name, "New Order Rising");
    assert_eq!(parsed.acronym, "NOR");
    assert_eq!(parsed.default_language, "simp_chinese");
    assert_eq!(parsed.countries.len(), 2);
    assert_eq!(parsed.countries[0].id, "rus");
    assert_eq!(parsed.regions[0].id, "east_asia");
    assert_eq!(parsed.systems[0].id, "black_monday");
    assert!(yaml.contains("hoi4skill.large_mod_blueprint.v1"));
}

#[test]
fn large_mod_commands_create_project_and_work_packages() {
    let root = unique_temp_dir("large-mod-project");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let packages = root.join("packages");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS, GER\nregions: europe\nsystems: political crisis".to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_split_work_packages(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        packages.to_string_lossy().to_string(),
    ])
    .unwrap();

    assert!(mod_root.join("descriptor.mod").exists());
    assert!(mod_root.join(".hoi4skill/large_mod_blueprint.yml").exists());
    assert!(mod_root.join(".hoi4skill/project.json").exists());
    assert!(mod_root.join("common/national_focus").is_dir());
    assert!(mod_root.join("localisation/simp_chinese").is_dir());
    assert!(packages.join("country_rus.md").exists());
    assert!(packages.join("region_europe.md").exists());
    assert!(packages.join("system_political_crisis.md").exists());

    let manifest = read_utf8_lossy(&packages.join("manifest.json")).unwrap();
    assert!(manifest.contains("\"schema\": \"hoi4skill.large_mod_work_packages.v1\""));
    assert!(manifest.contains("\"package_count\": 4"));
}

#[test]
fn large_mod_ownership_map_marks_shared_edit_surfaces() {
    let root = unique_temp_dir("large-mod-ownership-map");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("ownership_map.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS, GER\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_large_mod_ownership_map(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.large_mod_ownership_map.v1\""));
    assert!(json.contains("\"package_count\": 4"));
    assert!(json.contains("\"shared_path_count\""));
    assert!(json.contains("\"id\": \"country_rus\""));
    assert!(json.contains("\"identity_terms\": [\"RUS\", \"country_rus\", \"rus\", \"tgc_rus\"]"));
    assert!(json.contains("\"path\": \"events\""));
    assert!(json.contains("\"owner_count\": 3"));
    assert!(json.contains("\"path\": \"localisation/simp_chinese\""));
    assert!(json.contains("\"owner_count\": 4"));
    assert!(json.contains("\"requires_identity_terms\": true"));
    assert!(json.contains("split-changed-work-packages"));
    assert!(json.contains("Do not assign shared paths by directory prefix alone"));
}

#[test]
fn large_mod_dependency_graph_orders_system_country_region_packages() {
    let root = unique_temp_dir("large-mod-dependency-graph");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("dependency_graph.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS, GER\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_large_mod_dependency_graph(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.large_mod_dependency_graph.v1\""));
    assert!(json.contains("\"package_count\": 4"));
    assert!(json.contains("\"cycle_count\": 0"));
    assert!(json.contains("\"id\": \"system_black_monday\""));
    assert!(json.contains("\"layer\": 1"));
    assert!(json.contains("\"name\": \"system_contracts\""));
    assert!(json.contains("\"package\": \"country_rus\""));
    assert!(json.contains("\"depends_on\": \"system_black_monday\""));
    assert!(json.contains("\"package\": \"region_europe\""));
    assert!(json.contains("\"depends_on\": \"country_rus\""));
    assert!(json.contains("\"name\": \"country_content\""));
    assert!(json.contains("\"name\": \"regional_integration\""));
    assert!(json.contains("large-mod-ci-plan"));
    assert!(json.contains("Do not schedule a package before its dependency layer"));
}

#[test]
fn large_mod_milestone_plan_groups_packages_by_production_phase() {
    let root = unique_temp_dir("large-mod-milestone-plan");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("milestone_plan.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS, GER\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_large_mod_milestone_plan(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.large_mod_milestone_plan.v1\""));
    assert!(json.contains("\"package_count\": 4"));
    assert!(json.contains("\"milestone_count\": 5"));
    assert!(json.contains("\"phase\": \"system_contracts\""));
    assert!(json.contains("\"packages\": [\"system_black_monday\"]"));
    assert!(json.contains("\"phase\": \"country_content\""));
    assert!(json.contains("\"packages\": [\"country_rus\", \"country_ger\"]"));
    assert!(json.contains("\"phase\": \"regional_integration\""));
    assert!(json.contains("\"packages\": [\"region_europe\"]"));
    assert!(json.contains("\"required_reports\": [\"ownership_map.json\", \"dependency_graph.json\", \"ci_plan.json\"]"));
    assert!(json.contains("large-mod-dependency-graph"));
    assert!(json.contains("large-mod-release-gate"));
    assert!(json.contains("Do not advance a milestone while any listed required report is missing"));
}

#[test]
fn large_mod_execution_queue_respects_dependency_handoffs() {
    let root = unique_temp_dir("large-mod-execution-queue");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("execution_queue.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    for (name, text) in [
        ("changed_system_black_monday.txt", "common/scripted_effects/tgc_black_monday.txt\n"),
        (
            "plan_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        ),
        ("assets_system_black_monday.md", "# Asset Pack Plan\n"),
        (
            "boundary_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        ),
        (
            "status_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "validation_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        ("handoff_system_black_monday.md", "# Work Package Handoff\n"),
    ] {
        fs::write(mod_root.join(".hoi4skill").join(name), text).unwrap();
    }

    cmd_large_mod_execution_queue(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.large_mod_execution_queue.v1\""));
    assert!(json.contains("\"package_count\": 3"));
    assert!(json.contains("\"completed_count\": 1"));
    assert!(json.contains("\"ready_to_start_count\": 1"));
    assert!(json.contains("\"blocked_count\": 1"));
    assert!(json.contains("\"id\": \"system_black_monday\""));
    assert!(json.contains("\"status\": \"completed\""));
    assert!(json.contains("\"id\": \"country_rus\""));
    assert!(json.contains("\"status\": \"ready_to_start\""));
    assert!(json.contains("\"depends_on\": [\"system_black_monday\"]"));
    assert!(json.contains("\"id\": \"region_europe\""));
    assert!(json.contains("\"status\": \"blocked_by_dependencies\""));
    assert!(json.contains("\"blocked_by\": [\"country_rus\"]"));
    assert!(json.contains("work-package-claim --mod-root"));
    assert!(json.contains("work-package-start-brief --mod-root"));
    assert!(json.contains("work-package-start-briefs --mod-root"));
    assert!(json.contains("work-package-claims --mod-root"));
    assert!(json.contains("work-package-dispatch-board --mod-root"));
    assert!(json.contains("--ready-only"));
    assert!(json.contains("generate-work-package --mod-root"));
    assert!(json.contains("Do not start a package while status is blocked_by_dependencies"));
}

#[test]
fn work_package_start_brief_summarizes_ready_package_boundaries() {
    let root = unique_temp_dir("work-package-start-brief");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("start_country_rus.md");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    for (name, text) in [
        ("changed_system_black_monday.txt", "common/scripted_effects/tgc_black_monday.txt\n"),
        (
            "plan_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        ),
        ("assets_system_black_monday.md", "# Asset Pack Plan\n"),
        (
            "boundary_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        ),
        (
            "status_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "validation_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        ("handoff_system_black_monday.md", "# Work Package Handoff\n"),
    ] {
        fs::write(mod_root.join(".hoi4skill").join(name), text).unwrap();
    }

    cmd_work_package_start_brief(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let markdown = read_utf8_lossy(&output).unwrap();
    assert!(markdown.contains("# Work Package Start Brief: RUS Country Content"));
    assert!(markdown.contains("`hoi4skill.work_package_start_brief.v1`"));
    assert!(markdown.contains("- state: `ready_to_start`"));
    assert!(markdown.contains("- package: `country_rus`"));
    assert!(markdown.contains("- tag: `RUS`"));
    assert!(markdown.contains("- namespace: `tgc_rus`"));
    assert!(markdown.contains("- depends_on: `system_black_monday`"));
    assert!(markdown.contains("- dependency_state: `clear`"));
    assert!(markdown.contains("- `common/national_focus`"));
    assert!(markdown.contains("- `tgc_rus`"));
    assert!(markdown.contains("feature-context"));
    assert!(markdown.contains("apply-focus-layout"));
    assert!(markdown.contains("generate-work-package --mod-root"));
    assert!(markdown.contains("work-package-handoff --mod-root"));
    assert!(markdown.contains("## Code Authoring Contract"));
    assert!(markdown.contains("AI outputs intent"));
    assert!(markdown.contains("hoi4skill code-catalog"));
    assert!(markdown.contains("hoi4skill compile-intent"));
    assert!(markdown.contains("safety.final_code_allowed"));
    assert!(markdown.contains("Do not start while `state` is `blocked_by_dependencies`"));
}

#[test]
fn work_package_start_briefs_ready_only_writes_dispatch_manifest() {
    let root = unique_temp_dir("work-package-start-briefs");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output_dir = root.join("start_briefs");
    let output = root.join("start_briefs_manifest.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    for (name, text) in [
        (
            "changed_system_black_monday.txt",
            "common/scripted_effects/tgc_black_monday.txt\n",
        ),
        (
            "plan_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        ),
        ("assets_system_black_monday.md", "# Asset Pack Plan\n"),
        (
            "boundary_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        ),
        (
            "status_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "validation_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        ("handoff_system_black_monday.md", "# Work Package Handoff\n"),
    ] {
        fs::write(mod_root.join(".hoi4skill").join(name), text).unwrap();
    }

    cmd_work_package_start_briefs(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--ready-only".to_string(),
        "--output-dir".to_string(),
        output_dir.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let manifest = read_utf8_lossy(&output).unwrap();
    assert!(manifest.contains("\"schema\": \"hoi4skill.work_package_start_briefs.v1\""));
    assert!(manifest.contains("\"ready_only\": true"));
    assert!(manifest.contains("\"package_count\": 3"));
    assert!(manifest.contains("\"generated_count\": 1"));
    assert!(manifest.contains("\"skipped_count\": 2"));
    assert!(manifest.contains("\"id\": \"country_rus\""));
    assert!(manifest.contains("\"state\": \"ready_to_start\""));
    assert!(manifest.contains("\"generated\": true"));
    assert!(manifest.contains("\"id\": \"system_black_monday\""));
    assert!(manifest.contains("\"state\": \"already_handed_off\""));
    assert!(manifest.contains("\"generated\": false"));
    assert!(manifest.contains("\"id\": \"region_europe\""));
    assert!(manifest.contains("\"state\": \"blocked_by_dependencies\""));
    assert!(manifest.contains("\"blocked_by\": [\"country_rus\"]"));
    assert!(manifest.contains("Do not dispatch skipped packages"));
    assert!(output_dir.join("start_country_rus.md").exists());
    assert!(!output_dir.join("start_system_black_monday.md").exists());
    assert!(!output_dir.join("start_region_europe.md").exists());
    assert!(output_dir.join("manifest.json").exists());
}

#[test]
fn work_package_authoring_pack_writes_start_plan_assets_context() {
    let root = unique_temp_dir("work-package-authoring-pack");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output_dir = root.join("authoring_country_rus");
    let manifest_output = root.join("authoring_manifest.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS".to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_work_package_authoring_pack(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--output-dir".to_string(),
        output_dir.to_string_lossy().to_string(),
        "--output".to_string(),
        manifest_output.to_string_lossy().to_string(),
    ])
    .unwrap();

    assert!(output_dir.join("start.md").exists());
    assert!(output_dir.join("plan.json").exists());
    assert!(output_dir.join("assets.md").exists());
    assert!(output_dir.join("context.md").exists());
    assert!(output_dir.join("manifest.json").exists());

    let manifest = read_utf8_lossy(&manifest_output).unwrap();
    assert!(manifest.contains("\"schema\": \"hoi4skill.work_package_authoring_pack.v1\""));
    assert!(manifest.contains("\"state\": \"blocked_by_dependencies\""));
    assert!(manifest.contains("\"blocked_by\": [\"system_political_paths\""));
    assert!(manifest.contains("\"kind\": \"authoring_context\""));
    assert!(manifest.contains("check-work-package-boundary --mod-root"));
    assert!(manifest.contains("Do not write outside the allowed edit surface"));

    let context = read_utf8_lossy(&output_dir.join("context.md")).unwrap();
    assert!(context.contains("`hoi4skill.work_package_authoring_context.v1`"));
    assert!(context.contains("## Allowed Edit Surface"));
    assert!(context.contains("common/national_focus"));
    assert!(context.contains("## Identity Terms"));
    assert!(context.contains("`RUS`"));
    assert!(context.contains("work-package-handoff --mod-root"));
}

#[test]
fn work_package_claim_records_assignee_and_blocks_dependency_violations() {
    let root = unique_temp_dir("work-package-claim");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let claim_output = root.join("claim_country_rus.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    for (name, text) in [
        (
            "changed_system_black_monday.txt",
            "common/scripted_effects/tgc_black_monday.txt\n",
        ),
        (
            "plan_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        ),
        ("assets_system_black_monday.md", "# Asset Pack Plan\n"),
        (
            "boundary_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        ),
        (
            "status_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "validation_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        ("handoff_system_black_monday.md", "# Work Package Handoff\n"),
    ] {
        fs::write(mod_root.join(".hoi4skill").join(name), text).unwrap();
    }

    cmd_work_package_claim(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--assignee".to_string(),
        "codex-a".to_string(),
        "--output".to_string(),
        claim_output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let claim = read_utf8_lossy(&claim_output).unwrap();
    assert!(claim.contains("\"schema\": \"hoi4skill.work_package_claim.v1\""));
    assert!(claim.contains("\"assignee\": \"codex-a\""));
    assert!(claim.contains("\"can_start\": true"));
    assert!(claim.contains("\"state\": \"ready_to_start\""));
    assert!(claim.contains("\"id\": \"country_rus\""));
    assert!(claim.contains("\"depends_on\": [\"system_black_monday\"]"));
    assert!(claim.contains("work-package-start-brief --mod-root"));
    assert!(claim.contains("Do not overwrite another active claim"));

    let duplicate = cmd_work_package_claim(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--assignee".to_string(),
        "codex-b".to_string(),
        "--output".to_string(),
        claim_output.to_string_lossy().to_string(),
    ]);
    assert!(duplicate.is_err());
    assert!(duplicate.unwrap_err().contains("claim already exists"));

    let blocked = cmd_work_package_claim(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "region_europe".to_string(),
        "--assignee".to_string(),
        "codex-region".to_string(),
        "--output".to_string(),
        root.join("claim_region_europe.json")
            .to_string_lossy()
            .to_string(),
    ]);
    assert!(blocked.is_err());
    assert!(blocked
        .unwrap_err()
        .contains("blocked by dependencies: country_rus"));
}

#[test]
fn work_package_release_claim_archives_active_claim_and_reopens_package() {
    let root = unique_temp_dir("work-package-release-claim");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let release_output = root.join("release_country_rus.json");
    let claims_output = root.join("claims_after_release.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nsystems: black monday".to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    for (name, text) in [
        (
            "changed_system_black_monday.txt",
            "common/scripted_effects/tgc_black_monday.txt\n",
        ),
        (
            "plan_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        ),
        ("assets_system_black_monday.md", "# Asset Pack Plan\n"),
        (
            "boundary_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        ),
        (
            "status_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "validation_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        ("handoff_system_black_monday.md", "# Work Package Handoff\n"),
    ] {
        fs::write(mod_root.join(".hoi4skill").join(name), text).unwrap();
    }

    cmd_work_package_claim(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--assignee".to_string(),
        "codex-a".to_string(),
    ])
    .unwrap();
    let active_claim = mod_root
        .join(".hoi4skill")
        .join("claims")
        .join("claim_country_rus.json");
    assert!(active_claim.exists());

    let missing_reason = cmd_work_package_release_claim(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--released-by".to_string(),
        "codex-a".to_string(),
    ]);
    assert!(missing_reason.is_err());
    assert!(missing_reason.unwrap_err().contains("missing --reason"));

    cmd_work_package_release_claim(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--released-by".to_string(),
        "codex-a".to_string(),
        "--reason".to_string(),
        "handoff reassigned".to_string(),
        "--output".to_string(),
        release_output.to_string_lossy().to_string(),
    ])
    .unwrap();

    assert!(!active_claim.exists());
    let release = read_utf8_lossy(&release_output).unwrap();
    assert!(release.contains("\"schema\": \"hoi4skill.work_package_claim_release.v1\""));
    assert!(release.contains("\"previous_assignee\": \"codex-a\""));
    assert!(release.contains("\"previous_state\": \"ready_to_start\""));
    assert!(release.contains("\"released_by\": \"codex-a\""));
    assert!(release.contains("\"reason\": \"handoff reassigned\""));
    assert!(release.contains("work-package-dispatch-board --mod-root"));
    assert!(release.contains("Do not treat release as package completion"));

    cmd_work_package_claims(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        claims_output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let claims = read_utf8_lossy(&claims_output).unwrap();
    assert!(claims.contains("\"claimed_count\": 0"));
    assert!(claims.contains("\"unclaimed_count\": 3"));
    assert!(claims.contains("\"id\": \"country_rus\""));
    assert!(claims.contains("\"claim_status\": \"unclaimed\""));
    assert!(claims.contains("\"current_state\": \"ready_to_start\""));
}

#[test]
fn work_package_claims_summarizes_claimed_and_unclaimed_packages() {
    let root = unique_temp_dir("work-package-claims");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let claims_output = root.join("claims.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    for (name, text) in [
        (
            "changed_system_black_monday.txt",
            "common/scripted_effects/tgc_black_monday.txt\n",
        ),
        (
            "plan_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        ),
        ("assets_system_black_monday.md", "# Asset Pack Plan\n"),
        (
            "boundary_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        ),
        (
            "status_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "validation_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        ("handoff_system_black_monday.md", "# Work Package Handoff\n"),
    ] {
        fs::write(mod_root.join(".hoi4skill").join(name), text).unwrap();
    }

    cmd_work_package_claim(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--assignee".to_string(),
        "codex-a".to_string(),
    ])
    .unwrap();

    cmd_work_package_claims(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        claims_output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&claims_output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.work_package_claims.v1\""));
    assert!(json.contains("\"package_count\": 3"));
    assert!(json.contains("\"claimed_count\": 1"));
    assert!(json.contains("\"unclaimed_count\": 2"));
    assert!(json.contains("\"stale_or_blocked_count\": 0"));
    assert!(json.contains("\"id\": \"country_rus\""));
    assert!(json.contains("\"claim_status\": \"claimed\""));
    assert!(json.contains("\"assignee\": \"codex-a\""));
    assert!(json.contains("\"current_state\": \"ready_to_start\""));
    assert!(json.contains("\"id\": \"region_europe\""));
    assert!(json.contains("\"claim_status\": \"unclaimed\""));
    assert!(json.contains("\"current_state\": \"blocked_by_dependencies\""));
    assert!(json.contains("work-package-start-briefs --mod-root"));
    assert!(json.contains("Do not dispatch unclaimed packages"));
}

#[test]
fn work_package_dispatch_board_renders_claims_for_human_coordination() {
    let root = unique_temp_dir("work-package-dispatch-board");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let board_output = root.join("dispatch_board.md");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS, GER\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    for (name, text) in [
        (
            "changed_system_black_monday.txt",
            "common/scripted_effects/tgc_black_monday.txt\n",
        ),
        (
            "plan_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        ),
        ("assets_system_black_monday.md", "# Asset Pack Plan\n"),
        (
            "boundary_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        ),
        (
            "status_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "validation_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        ("handoff_system_black_monday.md", "# Work Package Handoff\n"),
    ] {
        fs::write(mod_root.join(".hoi4skill").join(name), text).unwrap();
    }

    cmd_work_package_claim(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--assignee".to_string(),
        "codex-a".to_string(),
    ])
    .unwrap();

    cmd_work_package_dispatch_board(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        board_output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let markdown = read_utf8_lossy(&board_output).unwrap();
    assert!(markdown.contains("# Work Package Dispatch Board: Test Grand Campaign"));
    assert!(markdown.contains("`hoi4skill.work_package_dispatch_board.v1`"));
    assert!(markdown.contains(
        "- packages: `1` claimed, `3` unclaimed, `0` blocked/stale, `1` ready-unclaimed"
    ));
    assert!(
        markdown.contains("| `country_rus` | `country` | codex-a | `claimed` | `ready_to_start` |")
    );
    assert!(markdown
        .contains("| `country_ger` | `country` | unassigned | `unclaimed` | `ready_to_start` |"));
    assert!(markdown.contains("| `region_europe` | `region` | unassigned | `unclaimed` | `blocked_by_dependencies` | country_rus, country_ger |"));
    assert!(markdown.contains("## Ready To Claim"));
    assert!(markdown.contains("work-package-claim --mod-root"));
    assert!(markdown.contains("work-package-dispatch-board --mod-root"));
    assert!(markdown
        .contains("Do not assign a package whose current state is `blocked_by_dependencies`"));
}

#[test]
fn large_mod_dispatch_gate_blocks_unclaimed_blocked_and_stale_claims() {
    let root = unique_temp_dir("large-mod-dispatch-gate");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("dispatch_gate.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS, GER\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_work_package_claim(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "system_black_monday".to_string(),
        "--assignee".to_string(),
        "codex-system".to_string(),
    ])
    .unwrap();

    for (name, text) in [
        (
            "changed_system_black_monday.txt",
            "common/scripted_effects/tgc_black_monday.txt\n",
        ),
        (
            "plan_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        ),
        ("assets_system_black_monday.md", "# Asset Pack Plan\n"),
        (
            "boundary_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        ),
        (
            "status_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "validation_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        ("handoff_system_black_monday.md", "# Work Package Handoff\n"),
    ] {
        fs::write(mod_root.join(".hoi4skill").join(name), text).unwrap();
    }

    cmd_work_package_claim(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--assignee".to_string(),
        "codex-a".to_string(),
    ])
    .unwrap();

    cmd_work_package_claim(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "region_europe".to_string(),
        "--assignee".to_string(),
        "codex-region".to_string(),
        "--allow-blocked".to_string(),
    ])
    .unwrap();

    cmd_large_mod_dispatch_gate(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.large_mod_dispatch_gate.v1\""));
    assert!(json.contains("\"dispatchable\": false"));
    assert!(json.contains("\"package_count\": 4"));
    assert!(json.contains("\"claim_count\": 3"));
    assert!(json.contains("\"ready_unclaimed_count\": 1"));
    assert!(json.contains("\"blocked_claim_count\": 1"));
    assert!(json.contains("\"stale_claim_count\": 1"));
    assert!(json.contains("\"blocking_count\": 3"));
    assert!(json.contains("\"id\": \"country_ger\""));
    assert!(json.contains("\"dispatch_status\": \"ready_unclaimed\""));
    assert!(json.contains("\"ready_package_unclaimed\""));
    assert!(json.contains("\"id\": \"region_europe\""));
    assert!(json.contains("\"dispatch_status\": \"blocked_claim\""));
    assert!(json.contains("\"claimed_package_is_not_ready\""));
    assert!(json.contains("\"id\": \"system_black_monday\""));
    assert!(json.contains("\"dispatch_status\": \"stale_claim\""));
    assert!(json.contains("\"claim_exists_after_handoff\""));
    assert!(json.contains("work-package-dispatch-board --mod-root"));
    assert!(json.contains("work-package-release-claim --mod-root"));
    assert!(json.contains("Do not dispatch ready packages"));
}

#[test]
fn large_mod_evidence_pack_lists_release_and_package_artifacts() {
    let root = unique_temp_dir("large-mod-evidence-pack");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("evidence_pack.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS".to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    for (name, text) in [
        (
            "mod_index.json",
            "{\n  \"schema\": \"hoi4skill.mod_index.v1\"\n}\n",
        ),
        (
            "ownership_map.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_ownership_map.v1\"\n}\n",
        ),
        (
            "loc_audit.json",
            "{\n  \"schema\": \"hoi4skill.loc_audit.v1\",\n  \"missing_count\": 0\n}\n",
        ),
        (
            "gfx_audit.json",
            "{\n  \"schema\": \"hoi4skill.gfx_audit.v1\",\n  \"missing_sprites_count\": 0\n}\n",
        ),
        (
            "logic_audit.json",
            "{\n  \"schema\": \"hoi4skill.logic_audit.v1\",\n  \"issue_count\": 0\n}\n",
        ),
        (
            "validation.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        (
            "regression_gate.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_regression_gate.v1\",\n  \"regression_passed\": true,\n  \"blocking_count\": 0\n}\n",
        ),
        (
            "work_package_status.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "readiness.json",
            "{\n  \"schema\": \"hoi4skill.work_package_readiness.v1\",\n  \"blocked_count\": 0,\n  \"missing_package_count\": 0\n}\n",
        ),
        (
            "changed_country_rus.txt",
            "events/rus_events.txt\ncommon/national_focus/RUS.txt\n",
        ),
        (
            "plan_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        ),
        ("assets_country_rus.md", "# Asset Pack Plan\n"),
        (
            "boundary_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        ),
        (
            "status_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "validation_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        ("handoff_country_rus.md", "# Work Package Handoff\n"),
    ] {
        fs::write(mod_root.join(".hoi4skill").join(name), text).unwrap();
    }

    cmd_large_mod_evidence_pack(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.large_mod_evidence_pack.v1\""));
    assert!(json.contains("\"complete\": false"));
    assert!(json.contains("\"missing_count\": 28"));
    assert!(json.contains("\"needs_review_count\": 0"));
    assert!(json.contains("\"kind\": \"required_report\""));
    assert!(json.contains("\"path\": "));
    assert!(json.contains("ownership_map.json"));
    assert!(json.contains("\"kind\": \"package_changed\""));
    assert!(json.contains("\"kind\": \"package_handoff\""));
    assert!(json.contains("\"package\": \"country_rus\""));
    assert!(json.contains("\"package\": \"region_core_region\""));
    assert!(json.contains("\"status\": \"missing\""));
    assert!(json.contains("large-mod-evidence-pack --mod-root"));
    assert!(json.contains("large-mod-review-brief --mod-root"));
}

#[test]
fn large_mod_review_brief_summarizes_blockers_for_human_review() {
    let root = unique_temp_dir("large-mod-review-brief");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("review_brief.md");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    for (name, text) in [
        (
            "mod_index.json",
            "{\n  \"schema\": \"hoi4skill.mod_index.v1\"\n}\n",
        ),
        (
            "ownership_map.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_ownership_map.v1\"\n}\n",
        ),
        (
            "loc_audit.json",
            "{\n  \"schema\": \"hoi4skill.loc_audit.v1\",\n  \"missing_count\": 0\n}\n",
        ),
        (
            "gfx_audit.json",
            "{\n  \"schema\": \"hoi4skill.gfx_audit.v1\",\n  \"missing_sprites_count\": 0\n}\n",
        ),
        (
            "logic_audit.json",
            "{\n  \"schema\": \"hoi4skill.logic_audit.v1\",\n  \"issue_count\": 0\n}\n",
        ),
        (
            "validation.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        (
            "regression_gate.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_regression_gate.v1\",\n  \"regression_passed\": true,\n  \"blocking_count\": 0\n}\n",
        ),
        (
            "work_package_status.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "readiness.json",
            "{\n  \"schema\": \"hoi4skill.work_package_readiness.v1\",\n  \"blocked_count\": 0,\n  \"missing_package_count\": 0\n}\n",
        ),
        ("changed_country_rus.txt", "events/rus_events.txt\n"),
        (
            "plan_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        ),
        ("assets_country_rus.md", "# Asset Pack Plan\n"),
        (
            "boundary_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        ),
        (
            "status_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "validation_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        ("handoff_country_rus.md", "# Work Package Handoff\n"),
    ] {
        fs::write(mod_root.join(".hoi4skill").join(name), text).unwrap();
    }

    cmd_large_mod_review_brief(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let markdown = read_utf8_lossy(&output).unwrap();
    assert!(markdown.contains("# Large Mod Review Brief: Test Grand Campaign"));
    assert!(markdown.contains("`hoi4skill.large_mod_review_brief.v1`"));
    assert!(markdown.contains("- decision: `blocked`"));
    assert!(markdown.contains("- reports: `0` missing required, `0` needs review"));
    assert!(markdown.contains("Package `region_europe` blocked"));
    assert!(markdown.contains("| `mod_index.json` | `ok` |"));
    assert!(markdown.contains("| `country_rus` | RUS Country Content | `ready` |"));
    assert!(markdown.contains("| `region_europe` | europe Regional Integration | `blocked` |"));
    assert!(markdown.contains("large-mod-evidence-pack --mod-root"));
    assert!(markdown.contains("Do not approve while the decision is `blocked`"));
}

#[test]
fn large_mod_release_bundle_and_brief_collect_candidate_artifacts() {
    let root = unique_temp_dir("large-mod-release-bundle");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("release_bundle.json");
    let brief_output = root.join("release_brief.md");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS".to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    let packages = [
        "country_rus",
        "region_core_region",
        "system_political_paths",
        "system_economic_system",
        "system_regional_crisis",
    ];

    fs::create_dir_all(mod_root.join(".hoi4skill").join("merge_gates")).unwrap();
    for (name, text) in [
        (
            "mod_index.json",
            "{\n  \"schema\": \"hoi4skill.mod_index.v1\"\n}\n",
        ),
        (
            "ownership_map.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_ownership_map.v1\"\n}\n",
        ),
        (
            "loc_audit.json",
            "{\n  \"schema\": \"hoi4skill.loc_audit.v1\",\n  \"missing_count\": 0\n}\n",
        ),
        (
            "gfx_audit.json",
            "{\n  \"schema\": \"hoi4skill.gfx_audit.v1\",\n  \"missing_sprites_count\": 0\n}\n",
        ),
        (
            "logic_audit.json",
            "{\n  \"schema\": \"hoi4skill.logic_audit.v1\",\n  \"issue_count\": 0\n}\n",
        ),
        (
            "validation.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        (
            "regression_gate.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_regression_gate.v1\",\n  \"regression_passed\": true,\n  \"blocking_count\": 0\n}\n",
        ),
        (
            "work_package_status.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "readiness.json",
            "{\n  \"schema\": \"hoi4skill.work_package_readiness.v1\",\n  \"blocked_count\": 0,\n  \"missing_package_count\": 0\n}\n",
        ),
        (
            "ci_plan.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_ci_plan.v1\"\n}\n",
        ),
        (
            "dispatch_gate.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_dispatch_gate.v1\",\n  \"blocking_count\": 0\n}\n",
        ),
        (
            "merge_gate.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_merge_gate.v1\",\n  \"blocking_count\": 0\n}\n",
        ),
        (
            "review_queue.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_review_queue.v1\",\n  \"blocked_count\": 0\n}\n",
        ),
        (
            "risk_register.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_risk_register.v1\"\n}\n",
        ),
        (
            "evidence_pack.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_evidence_pack.v1\",\n  \"missing_count\": 0,\n  \"needs_review_count\": 0\n}\n",
        ),
        (
            "release_gate.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_release_gate.v1\",\n  \"blocking_count\": 0\n}\n",
        ),
        (
            "regression_plan.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_regression_plan.v1\",\n  \"affected_packages\": [],\n  \"unassigned_count\": 0\n}\n",
        ),
        (
            "regression_gate.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_regression_gate.v1\",\n  \"regression_passed\": true,\n  \"blocking_count\": 0\n}\n",
        ),
        (
            "next_actions.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_next_actions.v1\"\n}\n",
        ),
        (
            "production_snapshot.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_production_snapshot.v1\",\n  \"blocking_count\": 0\n}\n",
        ),
        (
            "playtest_plan.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_playtest_plan.v1\"\n}\n",
        ),
        (
            "playtest_gate.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_playtest_gate.v1\",\n  \"playtest_complete\": true,\n  \"missing_report_count\": 0,\n  \"needs_review_count\": 0\n}\n",
        ),
        ("review_brief.md", "# Large Mod Review Brief\n"),
        ("production_brief.md", "# Large Mod Production Brief\n"),
        ("playtest_brief.md", "# Large Mod Playtest Brief\n"),
        ("regression_brief.md", "# Large Mod Regression Brief\n"),
        ("release_notes.md", "# Release Notes Draft\n"),
        ("dashboard.md", "# Large Mod Dashboard\n"),
    ] {
        fs::write(mod_root.join(".hoi4skill").join(name), text).unwrap();
    }
    fs::write(
        mod_root.join(".hoi4skill/merge_gates/manifest.json"),
        "{\n  \"schema\": \"hoi4skill.work_package_merge_gates.v1\",\n  \"blocked_count\": 0\n}\n",
    )
    .unwrap();

    for package in packages {
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("changed_{package}.txt")),
            "events/rus_events.txt\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("plan_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("assets_{package}.md")),
            "# Asset Pack Plan\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("boundary_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("status_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("validation_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("handoff_{package}.md")),
            "# Work Package Handoff\n",
        )
        .unwrap();
    }

    cmd_large_mod_release_bundle(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.large_mod_release_bundle.v1\""));
    assert!(json.contains("\"release_candidate\": true"));
    assert!(json.contains(&format!("\"package_count\": {}", packages.len())));
    assert!(json.contains("\"missing_required_count\": 0"));
    assert!(json.contains("\"needs_review_count\": 0"));
    assert!(json.contains("\"kind\": \"release_report\""));
    assert!(json.contains("\"kind\": \"package_merge_gates\""));
    assert!(json.contains("\"kind\": \"playtest_gate\""));
    assert!(json.contains("\"kind\": \"regression_gate\""));
    assert!(json.contains("\"kind\": \"regression_brief\""));
    assert!(json.contains("\"kind\": \"production_snapshot\""));
    assert!(json.contains("\"kind\": \"production_brief\""));
    assert!(json.contains("\"kind\": \"release_notes\""));
    assert!(json.contains("\"kind\": \"package_handoff\""));
    assert!(json.contains("\"package\": \"country_rus\""));
    assert!(json.contains("\"package\": \"region_core_region\""));
    assert!(json.contains("large-mod-release-bundle --mod-root"));
    assert!(json.contains("large-mod-release-brief --mod-root"));
    assert!(json.contains("Regenerate the release bundle after changing any package artifact"));

    cmd_large_mod_release_brief(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        brief_output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let markdown = read_utf8_lossy(&brief_output).unwrap();
    assert!(markdown.contains("# Large Mod Release Brief: Test Grand Campaign"));
    assert!(markdown.contains("`hoi4skill.large_mod_release_brief.v1`"));
    assert!(markdown.contains("- decision: `release_candidate`"));
    assert!(markdown.contains("| `package_handoff` |"));
    assert!(markdown.contains("| `playtest_gate` |"));
    assert!(markdown.contains("| `regression_gate` |"));
    assert!(markdown.contains("| `production_snapshot` |"));
    assert!(markdown.contains("| `release_report` |"));
    assert!(markdown.contains("| `package_handoff` | `country_rus` | `present` |"));
    assert!(markdown.contains("large-mod-release-bundle --mod-root"));
    assert!(markdown.contains("Do not publish while the decision is `blocked`"));
}

#[test]
fn large_mod_playtest_plan_prioritizes_ready_handoff_packages() {
    let root = unique_temp_dir("large-mod-playtest-plan");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("playtest_plan.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    for (name, text) in [
        ("changed_country_rus.txt", "events/rus_events.txt\n"),
        (
            "plan_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        ),
        ("assets_country_rus.md", "# Asset Pack Plan\n"),
        (
            "boundary_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        ),
        (
            "status_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "validation_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        ("handoff_country_rus.md", "# Work Package Handoff\n"),
    ] {
        fs::write(mod_root.join(".hoi4skill").join(name), text).unwrap();
    }

    cmd_large_mod_playtest_plan(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.large_mod_playtest_plan.v1\""));
    assert!(json.contains("\"package_count\": 3"));
    assert!(json.contains("\"ready_for_playtest_count\": 1"));
    assert!(json.contains("\"blocked_count\": 2"));
    assert!(json.contains("\"id\": \"country_rus\""));
    assert!(json.contains("\"status\": \"ready_for_playtest\""));
    assert!(json.contains("\"country_selection_smoke\""));
    assert!(json.contains("\"id\": \"region_europe\""));
    assert!(json.contains("\"regional_integration_smoke\""));
    assert!(json.contains("\"id\": \"system_black_monday\""));
    assert!(json.contains("\"system_regression_smoke\""));
    assert!(json.contains("analyze-error-log --input <error.log>"));
    assert!(json.contains("large-mod-release-brief --mod-root"));
    assert!(json.contains("Do not schedule blocked packages for playtest"));
}

#[test]
fn large_mod_playtest_gate_blocks_missing_and_failing_reports() {
    let root = unique_temp_dir("large-mod-playtest-gate");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("playtest_gate.json");
    let brief_output = root.join("playtest_brief.md");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    for package in ["country_rus", "region_europe", "system_black_monday"] {
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("changed_{package}.txt")),
            "events/test_events.txt\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("plan_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("assets_{package}.md")),
            "# Asset Pack Plan\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("boundary_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("status_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("validation_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("handoff_{package}.md")),
            "# Work Package Handoff\n",
        )
        .unwrap();
    }
    fs::write(
        mod_root.join(".hoi4skill/playtest_country_rus.json"),
        "{\n  \"schema\": \"hoi4skill.playtest_report.v1\",\n  \"ok\": true\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/playtest_system_black_monday.json"),
        "{\n  \"schema\": \"hoi4skill.playtest_report.v1\",\n  \"ok\": false,\n  \"error_count\": 1\n}\n",
    )
    .unwrap();

    cmd_large_mod_playtest_gate(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.large_mod_playtest_gate.v1\""));
    assert!(json.contains("\"playtest_complete\": false"));
    assert!(json.contains("\"passed_count\": 1"));
    assert!(json.contains("\"missing_report_count\": 1"));
    assert!(json.contains("\"needs_review_count\": 1"));
    assert!(json.contains("\"id\": \"country_rus\""));
    assert!(json.contains("\"gate_status\": \"passed\""));
    assert!(json.contains("\"id\": \"region_europe\""));
    assert!(json.contains("\"missing_playtest_report\""));
    assert!(json.contains("\"id\": \"system_black_monday\""));
    assert!(json.contains("\"playtest_needs_review\""));
    assert!(json.contains("large-mod-playtest-plan --mod-root"));
    assert!(json.contains("Do not accept missing playtest reports"));

    cmd_large_mod_playtest_brief(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        brief_output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let markdown = read_utf8_lossy(&brief_output).unwrap();
    assert!(markdown.contains("# Large Mod Playtest Brief: Test Grand Campaign"));
    assert!(markdown.contains("`hoi4skill.large_mod_playtest_brief.v1`"));
    assert!(markdown.contains("- decision: `blocked`"));
    assert!(markdown.contains("region_europe: missing_playtest_report"));
    assert!(markdown.contains("system_black_monday: playtest_needs_review"));
    assert!(markdown.contains("| `country_rus` | `country` | `passed` |"));
    assert!(markdown.contains("large-mod-playtest-gate --mod-root"));
    assert!(markdown.contains("Do not approve playtest while the decision is `blocked`"));
}

#[test]
fn work_package_playtest_report_writes_gate_compatible_json() {
    let root = unique_temp_dir("work-package-playtest-report");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let gate_output = root.join("playtest_gate.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    for package in ["country_rus", "region_europe", "system_black_monday"] {
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("changed_{package}.txt")),
            "events/test_events.txt\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("plan_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("assets_{package}.md")),
            "# Asset Pack Plan\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("boundary_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("status_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("validation_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("handoff_{package}.md")),
            "# Work Package Handoff\n",
        )
        .unwrap();
        cmd_work_package_playtest_report(&[
            "--mod-root".to_string(),
            mod_root.to_string_lossy().to_string(),
            "--package".to_string(),
            package.to_string(),
            "--result".to_string(),
            "passed".to_string(),
            "--summary".to_string(),
            format!("{package} smoke test passed"),
            "--tester".to_string(),
            "qa-a".to_string(),
            "--evidence".to_string(),
            format!("saves/{package}.hoi4"),
        ])
        .unwrap();
    }

    let report = read_utf8_lossy(&mod_root.join(".hoi4skill/playtest_country_rus.json")).unwrap();
    assert!(report.contains("\"schema\": \"hoi4skill.playtest_report.v1\""));
    assert!(report.contains("\"ok\": true"));
    assert!(report.contains("\"status\": \"ok\""));
    assert!(report.contains("\"finding_count\": 0"));
    assert!(report.contains("\"tester\": \"qa-a\""));
    assert!(report.contains("country_rus smoke test passed"));

    let contradictory = cmd_work_package_playtest_report(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--result".to_string(),
        "passed".to_string(),
        "--finding".to_string(),
        "unresolved issue".to_string(),
    ]);
    assert!(contradictory.is_err());
    assert!(contradictory
        .unwrap_err()
        .contains("--finding requires --result needs_review"));

    cmd_large_mod_playtest_gate(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        gate_output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let gate = read_utf8_lossy(&gate_output).unwrap();
    assert!(gate.contains("\"playtest_complete\": true"));
    assert!(gate.contains("\"passed_count\": 3"));
    assert!(gate.contains("\"missing_report_count\": 0"));
    assert!(gate.contains("\"needs_review_count\": 0"));
}

#[test]
fn large_mod_release_notes_draft_uses_package_evidence_only() {
    let root = unique_temp_dir("large-mod-release-notes");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("release_notes.md");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    for (name, text) in [
        ("changed_country_rus.txt", "events/rus_events.txt\n"),
        (
            "plan_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        ),
        ("assets_country_rus.md", "# Asset Pack Plan\n"),
        (
            "boundary_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        ),
        (
            "status_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "validation_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        ("handoff_country_rus.md", "# Work Package Handoff\n"),
        (
            "playtest_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.playtest_report.v1\",\n  \"ok\": true,\n  \"summary\": [\"country smoke passed\"]\n}\n",
        ),
    ] {
        fs::write(mod_root.join(".hoi4skill").join(name), text).unwrap();
    }

    cmd_large_mod_release_notes(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let markdown = read_utf8_lossy(&output).unwrap();
    assert!(markdown.contains("# Release Notes Draft: Test Grand Campaign"));
    assert!(markdown.contains("`hoi4skill.large_mod_release_notes.v1`"));
    assert!(markdown.contains("- status: `draft_requires_human_review`"));
    assert!(markdown.contains("This draft is generated from package metadata"));
    assert!(markdown.contains("## Country Packages"));
    assert!(markdown.contains("`country_rus`: RUS Country Content"));
    assert!(markdown.contains("playtest: `ok`"));
    assert!(markdown.contains("schema=hoi4skill.playtest_report.v1"));
    assert!(markdown.contains("Do not describe unimplemented gameplay"));
    assert!(markdown.contains("large-mod-release-notes --mod-root"));
}

#[test]
fn generate_work_package_dry_run_outputs_safe_execution_plan() {
    let root = unique_temp_dir("generate-work-package");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("package_plan.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS, German Empire\nregions: europe\nsystems: black monday".to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    let blocked = cmd_generate_work_package(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
    ]);
    assert!(blocked.is_err());
    assert!(blocked
        .unwrap_err()
        .contains("generate-work-package is dry-run only"));

    cmd_generate_work_package(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--dry-run".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.generate_work_package_plan.v1\""));
    assert!(json.contains("\"dry_run\": true"));
    assert!(json.contains("\"id\": \"country_rus\""));
    assert!(json.contains("\"tag\": \"RUS\""));
    assert!(json.contains("\"namespace\": \"tgc_rus\""));
    assert!(json.contains("hoi4skill build-mod-index"));
    assert!(json.contains("hoi4skill feature-context"));
    assert!(json.contains("hoi4skill reserve-id"));
    assert!(json.contains("hoi4skill loc-audit"));
    assert!(json.contains("hoi4skill gfx-audit"));
    assert!(json.contains("\"code_authoring_contract\""));
    assert!(json.contains("\"schema\": \"hoi4skill.code_authoring_contract.v1\""));
    assert!(json.contains("\"final_code_allowed\": false"));
    assert!(json.contains("hoi4skill code-catalog"));
    assert!(json.contains("hoi4skill compile-intent"));
    assert!(json.contains("raw Clausewitz blocks not produced by Rust writers"));
    assert!(json.contains("code index category"));
    assert!(json.contains("hoi4skill apply-focus-layout"));
    assert!(json.contains("hoi4skill apply-feature-cards"));
    assert!(json.contains("hoi4skill apply-event-cards"));
    assert!(json.contains("country tags, country history, state history"));
    assert!(json.contains("strict-code-index"));
}

#[test]
fn asset_pack_plan_outputs_work_package_asset_requirements() {
    let root = unique_temp_dir("asset-pack-plan");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("assets.md");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_asset_pack_plan(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let markdown = read_utf8_lossy(&output).unwrap();
    assert!(markdown.contains("# Asset Pack Plan: RUS Country Content"));
    assert!(markdown.contains("`hoi4skill.asset_pack_plan.v1`"));
    assert!(markdown.contains("- tag_hint: `RUS`"));
    assert!(markdown.contains("- prefix_hint: `tgc_rus`"));
    assert!(markdown.contains("`focus_icons`: 40 asset(s)"));
    assert!(markdown.contains("`event_pictures`: 12 asset(s)"));
    assert!(markdown.contains("GFX_goal_tgc_rus_<english_slug>"));
    assert!(markdown.contains("gfx/event_pictures/tgc_rus_<english_slug>.dds"));
    assert!(markdown.contains("register-gfx-icons"));
    assert!(markdown.contains("gfx-audit"));
    assert!(markdown.contains("Do not create portraits, characters, GUI, technologies"));
}

#[test]
fn work_package_status_aggregates_packages_and_audit_reports() {
    let root = unique_temp_dir("work-package-status");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("status.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS, GER\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_split_work_packages(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root
            .join(".hoi4skill/work_packages")
            .to_string_lossy()
            .to_string(),
    ])
    .unwrap();

    fs::write(
        mod_root.join(".hoi4skill/loc_audit.json"),
        "{\n  \"schema\": \"hoi4skill.loc_audit.v1\",\n  \"missing_count\": 2\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/logic_audit.json"),
        "{\n  \"schema\": \"hoi4skill.logic_audit.v1\",\n  \"issue_count\": 1\n}\n",
    )
    .unwrap();

    cmd_work_package_status(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.work_package_status.v1\""));
    assert!(json.contains("\"status\": \"needs_review\""));
    assert!(json.contains("\"package_count\": 4"));
    assert!(json.contains("\"id\": \"country_rus\""));
    assert!(json.contains("\"id\": \"system_black_monday\""));
    assert!(json.contains("\"schema=hoi4skill.loc_audit.v1\""));
    assert!(json.contains("\"missing_count=2\""));
    assert!(json.contains("\"schema=hoi4skill.logic_audit.v1\""));
    assert!(json.contains("\"issue_count=1\""));
    assert!(json.contains("generate-work-package --mod-root"));
    assert!(json.contains("asset-pack-plan --mod-root"));
    assert!(json.contains("work-package-handoff --mod-root"));
    assert!(json.contains("logic-audit"));

    let filtered = root.join("status_filtered.json");
    cmd_work_package_status(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--output".to_string(),
        filtered.to_string_lossy().to_string(),
    ])
    .unwrap();
    let filtered_json = read_utf8_lossy(&filtered).unwrap();
    assert!(filtered_json.contains("\"package_count\": 1"));
    assert!(filtered_json.contains("\"id\": \"country_rus\""));
    assert!(!filtered_json.contains("\"id\": \"country_ger\""));
}

#[test]
fn check_work_package_boundary_reports_out_of_scope_changed_files() {
    let root = unique_temp_dir("work-package-boundary");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("boundary.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    let changed_file = root.join("changed.txt");
    fs::write(
        &changed_file,
        "events/rus_events.txt\n# ignored comment\nhistory/states/64-Test.txt\n",
    )
    .unwrap();

    cmd_check_work_package_boundary(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--changed".to_string(),
        "common/national_focus/RUS.txt".to_string(),
        "--changed".to_string(),
        "events/ger_events.txt".to_string(),
        "--changed".to_string(),
        mod_root
            .join(".hoi4skill/plan_country_rus.json")
            .to_string_lossy()
            .to_string(),
        "--changed-file".to_string(),
        changed_file.to_string_lossy().to_string(),
        "--strict-names".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.work_package_boundary.v1\""));
    assert!(json.contains("\"ok\": false"));
    assert!(json.contains("\"strict_names\": true"));
    assert!(json.contains("\"changed_count\": 5"));
    assert!(json.contains("\"allowed_count\": 3"));
    assert!(json.contains("\"violation_count\": 2"));
    assert!(json.contains("\"id\": \"country_rus\""));
    assert!(json.contains("\"identity_terms\": [\"RUS\", \"country_rus\", \"rus\", \"tgc_rus\"]"));
    assert!(json.contains("\"path\": \"common/national_focus/RUS.txt\""));
    assert!(json.contains("\"allowed_by\": \"common/national_focus\""));
    assert!(json.contains("\"normalized\": \".hoi4skill/plan_country_rus.json\""));
    assert!(json.contains("\"allowed_by\": \".hoi4skill/plan_country_rus.json\""));
    assert!(json.contains("\"path\": \"events/ger_events.txt\""));
    assert!(json.contains("\"allowed_by\": \"events\""));
    assert!(json.contains("\"reason\": \"strict_name_mismatch\""));
    assert!(json.contains("\"path\": \"history/states/64-Test.txt\""));
    assert!(json.contains("\"allowed_by\": null"));
    assert!(json.contains("\"reason\": \"prefix_not_allowed\""));
    assert!(json.contains("Do not continue package generation"));
}

#[test]
fn large_mod_ci_plan_outputs_global_package_and_final_gates() {
    let root = unique_temp_dir("large-mod-ci-plan");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("ci_plan.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS, GER\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_large_mod_ci_plan(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--game-root".to_string(),
        "C:/Games/Hearts of Iron IV".to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--strict-names".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.large_mod_ci_plan.v1\""));
    assert!(json.contains("\"strict_names\": true"));
    assert!(json.contains("\"package_count\": 1"));
    assert!(json.contains("\"id\": \"country_rus\""));
    assert!(!json.contains("\"id\": \"country_ger\""));
    assert!(json.contains("build-mod-index"));
    assert!(json.contains("large-mod-ownership-map"));
    assert!(json.contains("large-mod-dependency-graph"));
    assert!(json.contains(".hoi4skill/dependency_graph.json"));
    assert!(json.contains("large-mod-milestone-plan"));
    assert!(json.contains(".hoi4skill/milestone_plan.json"));
    assert!(json.contains("large-mod-execution-queue"));
    assert!(json.contains(".hoi4skill/execution_queue.json"));
    assert!(json.contains("loc-audit"));
    assert!(json.contains("gfx-audit"));
    assert!(json.contains("logic-audit"));
    assert!(json.contains("analyze-error-log"));
    assert!(json.contains("check-work-package-boundary"));
    assert!(json.contains("work-package-start-brief"));
    assert!(json.contains(".hoi4skill/start_country_rus.md"));
    assert!(json.contains("--strict-names"));
    assert!(json.contains(".hoi4skill/changed_country_rus.txt"));
    assert!(json.contains("generate-work-package"));
    assert!(json.contains("asset-pack-plan"));
    assert!(json.contains("validate"));
    assert!(json.contains("work-package-readiness"));
    assert!(json.contains(".hoi4skill/readiness.json"));
    assert!(json.contains("work-package-handoff"));
    assert!(json.contains(".hoi4skill/handoff_country_rus.md"));
    assert!(json.contains("large-mod-dashboard"));
    assert!(json.contains(".hoi4skill/dashboard.md"));
    assert!(json.contains("large-mod-next-actions"));
    assert!(json.contains(".hoi4skill/next_actions.json"));
    assert!(json.contains(".hoi4skill/ownership_map.json"));
    assert!(json.contains("large-mod-evidence-pack"));
    assert!(json.contains(".hoi4skill/evidence_pack.json"));
    assert!(json.contains("large-mod-review-brief"));
    assert!(json.contains(".hoi4skill/review_brief.md"));
    assert!(json.contains("--game-root C:/Games/Hearts of Iron IV"));
    assert!(json.contains("strict-code-index"));
    assert!(json.contains("Do not merge a package with boundary violations"));
}

#[test]
fn large_mod_release_gate_blocks_missing_or_failing_reports() {
    let root = unique_temp_dir("large-mod-release-gate");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("release_gate.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    fs::write(
        mod_root.join(".hoi4skill/mod_index.json"),
        "{\n  \"schema\": \"hoi4skill.mod_index.v1\"\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/ownership_map.json"),
        "{\n  \"schema\": \"hoi4skill.large_mod_ownership_map.v1\"\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/loc_audit.json"),
        "{\n  \"schema\": \"hoi4skill.loc_audit.v1\",\n  \"missing_count\": 0\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/gfx_audit.json"),
        "{\n  \"schema\": \"hoi4skill.gfx_audit.v1\",\n  \"missing_sprites_count\": 0\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/logic_audit.json"),
        "{\n  \"schema\": \"hoi4skill.logic_audit.v1\",\n  \"issue_count\": 0\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/validation.json"),
        "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/work_package_status.json"),
        "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/readiness.json"),
        "{\n  \"schema\": \"hoi4skill.work_package_readiness.v1\",\n  \"blocked_count\": 0,\n  \"missing_package_count\": 0\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/regression_gate.json"),
        "{\n  \"schema\": \"hoi4skill.large_mod_regression_gate.v1\",\n  \"regression_passed\": true,\n  \"blocking_count\": 0\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/boundary_country_rus.json"),
        "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": false,\n  \"violation_count\": 1\n}\n",
    )
    .unwrap();

    cmd_large_mod_release_gate(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.large_mod_release_gate.v1\""));
    assert!(json.contains("\"releasable\": false"));
    assert!(json.contains("\"package_count\": 3"));
    assert!(json.contains("\"missing_required_reports\": []"));
    assert!(json.contains("\"blocking_count\": 1"));
    assert!(json.contains("boundary_country_rus.json"));
    assert!(json.contains("work-package-readiness"));
    assert!(json.contains("large-mod-dashboard"));
    assert!(json.contains("large-mod-next-actions"));
    assert!(json.contains("ownership_map.json"));
    assert!(json.contains("large-mod-evidence-pack"));
    assert!(json.contains("large-mod-review-brief"));
    assert!(json.contains("\"ok=false\""));
    assert!(json.contains("\"violation_count=1\""));
    assert!(json.contains("Do not release while any report has needs_review status"));
}

#[test]
fn large_mod_release_gate_requires_clean_regression_gate() {
    let root = unique_temp_dir("large-mod-release-regression-gate");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let missing_output = root.join("release_gate_missing_regression.json");
    let failing_output = root.join("release_gate_failing_regression.json");
    let passed_output = root.join("release_gate_passed_regression.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    for (name, text) in [
        (
            "mod_index.json",
            "{\n  \"schema\": \"hoi4skill.mod_index.v1\"\n}\n",
        ),
        (
            "ownership_map.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_ownership_map.v1\"\n}\n",
        ),
        (
            "loc_audit.json",
            "{\n  \"schema\": \"hoi4skill.loc_audit.v1\",\n  \"missing_count\": 0\n}\n",
        ),
        (
            "gfx_audit.json",
            "{\n  \"schema\": \"hoi4skill.gfx_audit.v1\",\n  \"missing_sprites_count\": 0\n}\n",
        ),
        (
            "logic_audit.json",
            "{\n  \"schema\": \"hoi4skill.logic_audit.v1\",\n  \"issue_count\": 0\n}\n",
        ),
        (
            "validation.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        (
            "work_package_status.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "readiness.json",
            "{\n  \"schema\": \"hoi4skill.work_package_readiness.v1\",\n  \"blocked_count\": 0,\n  \"missing_package_count\": 0\n}\n",
        ),
    ] {
        fs::write(mod_root.join(".hoi4skill").join(name), text).unwrap();
    }

    cmd_large_mod_release_gate(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        missing_output.to_string_lossy().to_string(),
    ])
    .unwrap();
    let missing = read_utf8_lossy(&missing_output).unwrap();
    assert!(missing.contains("\"releasable\": false"));
    assert!(missing.contains("\"regression_gate.json\""));

    fs::write(
        mod_root.join(".hoi4skill/regression_gate.json"),
        "{\n  \"schema\": \"hoi4skill.large_mod_regression_gate.v1\",\n  \"regression_passed\": false,\n  \"blocking_count\": 1\n}\n",
    )
    .unwrap();

    cmd_large_mod_release_gate(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        failing_output.to_string_lossy().to_string(),
    ])
    .unwrap();
    let failing = read_utf8_lossy(&failing_output).unwrap();
    assert!(failing.contains("\"releasable\": false"));
    assert!(failing.contains("regression_gate.json"));
    assert!(failing.contains("\"blocking_count=1\""));

    fs::write(
        mod_root.join(".hoi4skill/regression_gate.json"),
        "{\n  \"schema\": \"hoi4skill.large_mod_regression_gate.v1\",\n  \"regression_passed\": true,\n  \"blocking_count\": 0\n}\n",
    )
    .unwrap();

    cmd_large_mod_release_gate(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        passed_output.to_string_lossy().to_string(),
    ])
    .unwrap();
    let passed = read_utf8_lossy(&passed_output).unwrap();
    assert!(passed.contains("\"releasable\": true"));
    assert!(passed.contains("\"missing_required_reports\": []"));
    assert!(passed.contains("\"blocking_count\": 0"));
}

#[test]
fn identify_work_packages_maps_changed_files_to_strict_package_matches() {
    let root = unique_temp_dir("identify-work-packages");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("changed_work_packages.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS, GER\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    let changed_file = root.join("changed.txt");
    fs::write(
        &changed_file,
        "events/rus_events.txt\ncommon/scripted_effects/black_monday_effects.txt\nhistory/states/64-Test.txt\n",
    )
    .unwrap();

    cmd_identify_work_packages(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--changed".to_string(),
        "events/ger_events.txt".to_string(),
        "--changed-file".to_string(),
        changed_file.to_string_lossy().to_string(),
        "--strict-names".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.changed_work_packages.v1\""));
    assert!(json.contains("\"strict_names\": true"));
    assert!(json.contains("\"changed_count\": 4"));
    assert!(json.contains("\"assigned_count\": 3"));
    assert!(json.contains("\"unassigned_count\": 1"));
    assert!(json.contains("\"ambiguous_count\": 0"));
    assert!(json.contains(
        "\"affected_packages\": [\"country_ger\", \"country_rus\", \"system_black_monday\"]"
    ));
    assert!(json.contains("\"path\": \"events/rus_events.txt\""));
    assert!(json.contains("\"id\": \"country_rus\""));
    assert!(json.contains("\"path\": \"events/ger_events.txt\""));
    assert!(json.contains("\"id\": \"country_ger\""));
    assert!(json.contains("\"path\": \"common/scripted_effects/black_monday_effects.txt\""));
    assert!(json.contains("\"id\": \"system_black_monday\""));
    assert!(json.contains("\"path\": \"history/states/64-Test.txt\""));
    assert!(json.contains("\"status\": \"unassigned\""));
    assert!(json.contains("check-work-package-boundary"));
}

#[test]
fn split_changed_work_packages_writes_per_package_changed_files() {
    let root = unique_temp_dir("split-changed-work-packages");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("split_changed.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS, GER\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    fs::write(
        mod_root.join(".hoi4skill/changed_region_europe.txt"),
        "events/stale.txt\n",
    )
    .unwrap();

    let changed_file = root.join("changed.txt");
    fs::write(
        &changed_file,
        "events/rus_events.txt\ncommon/scripted_effects/black_monday_effects.txt\nhistory/states/64-Test.txt\n",
    )
    .unwrap();

    cmd_split_changed_work_packages(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--changed".to_string(),
        "events/ger_events.txt".to_string(),
        "--changed-file".to_string(),
        changed_file.to_string_lossy().to_string(),
        "--strict-names".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.split_changed_work_packages.v1\""));
    assert!(json.contains("\"affected_package_count\": 3"));
    assert!(json.contains("\"unassigned_count\": 1"));
    assert!(json.contains("\"ambiguous_count\": 0"));
    assert!(json.contains("changed_country_rus.txt"));
    assert!(json.contains("changed_country_ger.txt"));
    assert!(json.contains("changed_system_black_monday.txt"));
    assert!(json.contains("check-work-package-boundary"));

    let rus_changed =
        read_utf8_lossy(&mod_root.join(".hoi4skill/changed_country_rus.txt")).unwrap();
    let ger_changed =
        read_utf8_lossy(&mod_root.join(".hoi4skill/changed_country_ger.txt")).unwrap();
    let system_changed =
        read_utf8_lossy(&mod_root.join(".hoi4skill/changed_system_black_monday.txt")).unwrap();
    let region_changed =
        read_utf8_lossy(&mod_root.join(".hoi4skill/changed_region_europe.txt")).unwrap();
    let unassigned = read_utf8_lossy(&mod_root.join(".hoi4skill/changed_unassigned.txt")).unwrap();
    let ambiguous = read_utf8_lossy(&mod_root.join(".hoi4skill/changed_ambiguous.txt")).unwrap();

    assert_eq!(rus_changed, "events/rus_events.txt\n");
    assert_eq!(ger_changed, "events/ger_events.txt\n");
    assert_eq!(
        system_changed,
        "common/scripted_effects/black_monday_effects.txt\n"
    );
    assert_eq!(region_changed, "");
    assert_eq!(unassigned, "history/states/64-Test.txt\n");
    assert_eq!(ambiguous, "");
}

#[test]
fn work_package_readiness_reports_ready_and_missing_packages() {
    let root = unique_temp_dir("work-package-readiness");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("readiness.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS, GER\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    fs::write(
        mod_root.join(".hoi4skill/changed_country_rus.txt"),
        "events/rus_events.txt\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/plan_country_rus.json"),
        "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/assets_country_rus.md"),
        "# Asset Pack Plan\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/boundary_country_rus.json"),
        "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/status_country_rus.json"),
        "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/validation_country_rus.json"),
        "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
    )
    .unwrap();

    cmd_work_package_readiness(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.work_package_readiness.v1\""));
    assert!(json.contains("\"package_count\": 4"));
    assert!(json.contains("\"ready_count\": 1"));
    assert!(json.contains("\"blocked_count\": 3"));
    assert!(json.contains("\"missing_package_count\": 3"));
    assert!(json.contains("\"id\": \"country_rus\""));
    assert!(json.contains("\"ready\": true"));
    assert!(json.contains("\"id\": \"country_ger\""));
    assert!(json.contains("\"missing_artifacts\": [\"changed\", \"plan\", \"assets\", \"boundary\", \"status\", \"validation\"]"));
    assert!(json.contains("large-mod-release-gate"));
}

#[test]
fn work_package_handoff_writes_author_ready_markdown() {
    let root = unique_temp_dir("work-package-handoff");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("handoff.md");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    fs::write(
        mod_root.join(".hoi4skill/changed_country_rus.txt"),
        "events/rus_events.txt\ncommon/national_focus/RUS.txt\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/plan_country_rus.json"),
        "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/assets_country_rus.md"),
        "# Asset Pack Plan\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/boundary_country_rus.json"),
        "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/status_country_rus.json"),
        "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/validation_country_rus.json"),
        "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
    )
    .unwrap();

    cmd_work_package_handoff(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let markdown = read_utf8_lossy(&output).unwrap();
    assert!(markdown.contains("# Work Package Handoff: RUS Country Content"));
    assert!(markdown.contains("`hoi4skill.work_package_handoff.v1`"));
    assert!(markdown.contains("- package_id: `country_rus`"));
    assert!(markdown.contains("- namespace: `tgc_rus`"));
    assert!(markdown.contains("- tag_hint: `RUS`"));
    assert!(markdown.contains("- `country_rus`"));
    assert!(markdown.contains("- `common/national_focus`"));
    assert!(markdown.contains("- `events/rus_events.txt`"));
    assert!(markdown.contains("| `boundary` | `ok` |"));
    assert!(markdown.contains("hoi4skill check-work-package-boundary --mod-root"));
    assert!(markdown.contains("hoi4skill work-package-readiness --mod-root"));
    assert!(markdown.contains("Do not edit files outside the allowed edit surface"));
}

#[test]
fn work_package_review_checklist_tracks_handoff_acceptance() {
    let root = unique_temp_dir("work-package-review-checklist");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let pre_output = root.join("review_checklist_pre.md");
    let post_output = root.join("review_checklist_post.md");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nsystems: black monday".to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    for (name, text) in [
        (
            "changed_system_black_monday.txt",
            "common/scripted_effects/tgc_black_monday.txt\n",
        ),
        (
            "plan_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        ),
        ("assets_system_black_monday.md", "# Asset Pack Plan\n"),
        (
            "boundary_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        ),
        (
            "status_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "validation_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        ("handoff_system_black_monday.md", "# Work Package Handoff\n"),
    ] {
        fs::write(mod_root.join(".hoi4skill").join(name), text).unwrap();
    }

    cmd_work_package_claim(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--assignee".to_string(),
        "codex-a".to_string(),
    ])
    .unwrap();

    for (name, text) in [
        (
            "changed_country_rus.txt",
            "events/rus_events.txt\ncommon/national_focus/RUS.txt\n",
        ),
        (
            "plan_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        ),
        ("assets_country_rus.md", "# Asset Pack Plan\n"),
        (
            "boundary_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        ),
        (
            "status_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "validation_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
    ] {
        fs::write(mod_root.join(".hoi4skill").join(name), text).unwrap();
    }

    cmd_work_package_review_checklist(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--output".to_string(),
        pre_output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let pre = read_utf8_lossy(&pre_output).unwrap();
    assert!(pre.contains("# Work Package Review Checklist: RUS Country Content"));
    assert!(pre.contains("`hoi4skill.work_package_review_checklist.v1`"));
    assert!(pre.contains("- decision: `ready_for_handoff`"));
    assert!(pre.contains("- claim: `claimed` by `codex-a`"));
    assert!(pre.contains("| `boundary` | `ok` |"));
    assert!(pre.contains("| `handoff` | `missing` |"));
    assert!(pre.contains("- Missing `handoff` artifact."));
    assert!(pre.contains("- `events/rus_events.txt`"));
    assert!(pre.contains("work-package-review-checklist --mod-root"));
    assert!(pre.contains("Do not approve while decision is `blocked` or `ready_for_handoff`"));

    cmd_work_package_handoff(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--output".to_string(),
        mod_root
            .join(".hoi4skill")
            .join("handoff_country_rus.md")
            .to_string_lossy()
            .to_string(),
    ])
    .unwrap();

    cmd_work_package_review_checklist(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--output".to_string(),
        post_output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let post = read_utf8_lossy(&post_output).unwrap();
    assert!(post.contains("- decision: `approved`"));
    assert!(post.contains("| `handoff` | `present` |"));
    assert!(post.contains("- No required fixes found by the checklist."));
    assert!(post.contains("large-mod-release-gate --mod-root"));
}

#[test]
fn work_package_merge_gate_blocks_stale_claim_then_allows_release() {
    let root = unique_temp_dir("work-package-merge-gate");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let blocked_output = root.join("merge_gate_blocked.json");
    let released_output = root.join("claim_release.json");
    let mergeable_output = root.join("merge_gate_mergeable.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nsystems: black monday".to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    for (package, changed) in [
        (
            "system_black_monday",
            "common/scripted_effects/tgc_black_monday.txt\n",
        ),
        (
            "country_rus",
            "events/rus_events.txt\ncommon/national_focus/RUS.txt\n",
        ),
    ] {
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("changed_{package}.txt")),
            changed,
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("plan_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("assets_{package}.md")),
            "# Asset Pack Plan\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("boundary_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("status_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("validation_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("handoff_{package}.md")),
            "# Work Package Handoff\n",
        )
        .unwrap();
    }

    cmd_work_package_claim(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--assignee".to_string(),
        "codex-a".to_string(),
    ])
    .unwrap();

    cmd_work_package_merge_gate(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--output".to_string(),
        blocked_output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let blocked = read_utf8_lossy(&blocked_output).unwrap();
    assert!(blocked.contains("\"schema\": \"hoi4skill.work_package_merge_gate.v1\""));
    assert!(blocked.contains("\"mergeable\": false"));
    assert!(blocked.contains("\"decision\": \"blocked\""));
    assert!(blocked.contains("\"blocking_count\": 1"));
    assert!(blocked.contains("\"name\": \"claim\""));
    assert!(blocked.contains("\"status\": \"stale_claim_after_handoff\""));
    assert!(blocked.contains("work-package-release-claim --mod-root"));
    assert!(blocked.contains("Do not merge while an active claim remains after handoff"));

    cmd_work_package_release_claim(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--released-by".to_string(),
        "codex-a".to_string(),
        "--reason".to_string(),
        "handoff accepted".to_string(),
        "--output".to_string(),
        released_output.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_work_package_merge_gate(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--output".to_string(),
        mergeable_output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let mergeable = read_utf8_lossy(&mergeable_output).unwrap();
    assert!(mergeable.contains("\"mergeable\": true"));
    assert!(mergeable.contains("\"decision\": \"mergeable\""));
    assert!(mergeable.contains("\"blocking_count\": 0"));
    assert!(mergeable.contains("\"name\": \"handoff\""));
    assert!(mergeable.contains("\"status\": \"present\""));
    assert!(mergeable.contains("large-mod-release-gate --mod-root"));
}

#[test]
fn work_package_merge_gates_writes_all_package_gate_artifacts() {
    let root = unique_temp_dir("work-package-merge-gates");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output_dir = root.join("merge_gates");
    let manifest_output = root.join("merge_gates_manifest.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nsystems: black monday".to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    for (package, changed) in [
        (
            "system_black_monday",
            "common/scripted_effects/tgc_black_monday.txt\n",
        ),
        (
            "country_rus",
            "events/rus_events.txt\ncommon/national_focus/RUS.txt\n",
        ),
    ] {
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("changed_{package}.txt")),
            changed,
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("plan_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("assets_{package}.md")),
            "# Asset Pack Plan\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("boundary_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("status_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("validation_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("handoff_{package}.md")),
            "# Work Package Handoff\n",
        )
        .unwrap();
    }

    cmd_work_package_merge_gates(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output-dir".to_string(),
        output_dir.to_string_lossy().to_string(),
        "--output".to_string(),
        manifest_output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let manifest = read_utf8_lossy(&manifest_output).unwrap();
    assert!(manifest.contains("\"schema\": \"hoi4skill.work_package_merge_gates.v1\""));
    assert!(manifest.contains("\"package_count\": 3"));
    assert!(manifest.contains("\"mergeable_count\": 2"));
    assert!(manifest.contains("\"blocked_count\": 1"));
    assert!(manifest.contains("\"id\": \"country_rus\""));
    assert!(manifest.contains("\"id\": \"system_black_monday\""));
    assert!(manifest.contains("\"id\": \"region_core_region\""));
    assert!(manifest.contains("large-mod-merge-gate --mod-root"));
    assert!(manifest.contains("Regenerate merge gates after changing package artifacts"));
    assert!(output_dir.join("manifest.json").exists());
    assert!(output_dir.join("merge_gate_country_rus.json").exists());
    assert!(output_dir
        .join("merge_gate_system_black_monday.json")
        .exists());
    assert!(output_dir
        .join("merge_gate_region_core_region.json")
        .exists());

    let rus_gate = read_utf8_lossy(&output_dir.join("merge_gate_country_rus.json")).unwrap();
    assert!(rus_gate.contains("\"schema\": \"hoi4skill.work_package_merge_gate.v1\""));
    assert!(rus_gate.contains("\"mergeable\": true"));
    let region_gate =
        read_utf8_lossy(&output_dir.join("merge_gate_region_core_region.json")).unwrap();
    assert!(region_gate.contains("\"mergeable\": false"));
    assert!(region_gate.contains("\"name\": \"handoff\""));
    assert!(region_gate.contains("\"status\": \"missing\""));
}

#[test]
fn large_mod_merge_gate_summarizes_package_mergeability() {
    let root = unique_temp_dir("large-mod-merge-gate");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("large_merge_gate.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS, GER\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    for (package, changed) in [
        (
            "system_black_monday",
            "common/scripted_effects/tgc_black_monday.txt\n",
        ),
        (
            "country_rus",
            "events/rus_events.txt\ncommon/national_focus/RUS.txt\n",
        ),
    ] {
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("changed_{package}.txt")),
            changed,
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("plan_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("assets_{package}.md")),
            "# Asset Pack Plan\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("boundary_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("status_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("validation_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("handoff_{package}.md")),
            "# Work Package Handoff\n",
        )
        .unwrap();
    }

    cmd_large_mod_merge_gate(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.large_mod_merge_gate.v1\""));
    assert!(json.contains("\"mergeable\": false"));
    assert!(json.contains("\"decision\": \"blocked\""));
    assert!(json.contains("\"package_count\": 4"));
    assert!(json.contains("\"mergeable_count\": 2"));
    assert!(json.contains("\"blocked_count\": 2"));
    assert!(json.contains("\"id\": \"system_black_monday\""));
    assert!(json.contains("\"id\": \"country_rus\""));
    assert!(json.contains("\"id\": \"country_ger\""));
    assert!(json.contains("\"blockers\": [\"missing_assets\", \"missing_boundary\", \"missing_changed\", \"missing_handoff\", \"missing_plan\", \"missing_status\", \"missing_validation\"]"));
    assert!(json.contains("\"id\": \"region_europe\""));
    assert!(json.contains("work-package-merge-gate --mod-root"));
    assert!(json.contains("large-mod-release-gate --mod-root"));
    assert!(json.contains("Do not merge the large-mod integration branch"));
}

#[test]
fn large_mod_review_queue_prioritizes_merge_and_handoff_ready_packages() {
    let root = unique_temp_dir("large-mod-review-queue");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("review_queue.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS, GER\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    for (package, changed, handoff) in [
        (
            "system_black_monday",
            "common/scripted_effects/tgc_black_monday.txt\n",
            true,
        ),
        (
            "country_rus",
            "events/rus_events.txt\ncommon/national_focus/RUS.txt\n",
            true,
        ),
        (
            "country_ger",
            "events/ger_events.txt\ncommon/national_focus/GER.txt\n",
            false,
        ),
    ] {
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("changed_{package}.txt")),
            changed,
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("plan_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("assets_{package}.md")),
            "# Asset Pack Plan\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("boundary_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("status_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("validation_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        )
        .unwrap();
        if handoff {
            fs::write(
                mod_root
                    .join(".hoi4skill")
                    .join(format!("handoff_{package}.md")),
                "# Work Package Handoff\n",
            )
            .unwrap();
        }
    }

    cmd_work_package_claim(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "region_europe".to_string(),
        "--assignee".to_string(),
        "codex-region".to_string(),
        "--allow-blocked".to_string(),
    ])
    .unwrap();

    cmd_large_mod_review_queue(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.large_mod_review_queue.v1\""));
    assert!(json.contains("\"package_count\": 4"));
    assert!(json.contains("\"merge_ready_count\": 2"));
    assert!(json.contains("\"handoff_ready_count\": 1"));
    assert!(json.contains("\"blocked_count\": 1"));
    assert!(json.contains("\"id\": \"country_rus\""));
    assert!(json.contains("\"review_state\": \"merge_ready\""));
    assert!(json.contains("\"id\": \"country_ger\""));
    assert!(json.contains("\"review_state\": \"handoff_ready\""));
    assert!(json.contains("\"id\": \"region_europe\""));
    assert!(json.contains("\"review_state\": \"claim_blocked\""));
    assert!(json.contains("work-package-review-checklist --mod-root"));
    assert!(json.contains("work-package-handoff --mod-root"));
    assert!(json.contains("work-package-release-claim --mod-root"));
    assert!(json.contains("large-mod-merge-gate --mod-root"));
    assert!(json.contains("Do not spend reviewer time"));
}

#[test]
fn large_mod_dashboard_summarizes_reports_and_package_readiness() {
    let root = unique_temp_dir("large-mod-dashboard");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("dashboard.md");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    for (name, text) in [
        (
            "mod_index.json",
            "{\n  \"schema\": \"hoi4skill.mod_index.v1\"\n}\n",
        ),
        (
            "ownership_map.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_ownership_map.v1\"\n}\n",
        ),
        (
            "loc_audit.json",
            "{\n  \"schema\": \"hoi4skill.loc_audit.v1\",\n  \"missing_count\": 0\n}\n",
        ),
        (
            "gfx_audit.json",
            "{\n  \"schema\": \"hoi4skill.gfx_audit.v1\",\n  \"missing_sprites_count\": 0\n}\n",
        ),
        (
            "logic_audit.json",
            "{\n  \"schema\": \"hoi4skill.logic_audit.v1\",\n  \"issue_count\": 0\n}\n",
        ),
        (
            "validation.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        (
            "regression_gate.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_regression_gate.v1\",\n  \"regression_passed\": true,\n  \"blocking_count\": 0\n}\n",
        ),
        (
            "work_package_status.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "readiness.json",
            "{\n  \"schema\": \"hoi4skill.work_package_readiness.v1\",\n  \"blocked_count\": 0,\n  \"missing_package_count\": 0\n}\n",
        ),
    ] {
        fs::write(mod_root.join(".hoi4skill").join(name), text).unwrap();
    }
    fs::write(
        mod_root.join(".hoi4skill/changed_country_rus.txt"),
        "events/rus_events.txt\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/plan_country_rus.json"),
        "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/assets_country_rus.md"),
        "# Asset Pack Plan\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/boundary_country_rus.json"),
        "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/status_country_rus.json"),
        "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/validation_country_rus.json"),
        "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
    )
    .unwrap();

    cmd_large_mod_dashboard(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let markdown = read_utf8_lossy(&output).unwrap();
    assert!(markdown.contains("# Large Mod Dashboard: Test Grand Campaign"));
    assert!(markdown.contains("`hoi4skill.large_mod_dashboard.v1`"));
    assert!(markdown.contains("- release_ready: `no`"));
    assert!(markdown.contains("- packages: `1` ready, `2` blocked, `3` total"));
    assert!(markdown.contains("- reports: `0` missing required, `0` blocking"));
    assert!(markdown.contains("| `ownership_map.json` | `ok` |"));
    assert!(markdown.contains("| `country_rus` | RUS Country Content | `country` | `ready` |"));
    assert!(markdown
        .contains("| `region_europe` | europe Regional Integration | `region` | `blocked` |"));
    assert!(markdown.contains("changed_region_europe.txt"));
    assert!(markdown.contains("| `readiness.json` | `ok` |"));
    assert!(markdown.contains("large-mod-release-gate --mod-root"));
    assert!(markdown.contains("large-mod-evidence-pack --mod-root"));
    assert!(markdown.contains("large-mod-review-brief --mod-root"));
    assert!(markdown.contains("Use package handoff files"));
}

#[test]
fn large_mod_next_actions_lists_blocking_and_handoff_work() {
    let root = unique_temp_dir("large-mod-next-actions");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("next_actions.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    for (name, text) in [
        (
            "mod_index.json",
            "{\n  \"schema\": \"hoi4skill.mod_index.v1\"\n}\n",
        ),
        (
            "ownership_map.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_ownership_map.v1\"\n}\n",
        ),
        (
            "loc_audit.json",
            "{\n  \"schema\": \"hoi4skill.loc_audit.v1\",\n  \"missing_count\": 0\n}\n",
        ),
        (
            "gfx_audit.json",
            "{\n  \"schema\": \"hoi4skill.gfx_audit.v1\",\n  \"missing_sprites_count\": 0\n}\n",
        ),
        (
            "logic_audit.json",
            "{\n  \"schema\": \"hoi4skill.logic_audit.v1\",\n  \"issue_count\": 0\n}\n",
        ),
        (
            "validation.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        (
            "regression_gate.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_regression_gate.v1\",\n  \"regression_passed\": true,\n  \"blocking_count\": 0\n}\n",
        ),
        (
            "work_package_status.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "readiness.json",
            "{\n  \"schema\": \"hoi4skill.work_package_readiness.v1\",\n  \"blocked_count\": 0,\n  \"missing_package_count\": 0\n}\n",
        ),
    ] {
        fs::write(mod_root.join(".hoi4skill").join(name), text).unwrap();
    }
    fs::write(
        mod_root.join(".hoi4skill/changed_country_rus.txt"),
        "events/rus_events.txt\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/plan_country_rus.json"),
        "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/assets_country_rus.md"),
        "# Asset Pack Plan\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/boundary_country_rus.json"),
        "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/status_country_rus.json"),
        "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/validation_country_rus.json"),
        "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
    )
    .unwrap();

    cmd_large_mod_next_actions(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.large_mod_next_actions.v1\""));
    assert!(json.contains("\"blocking_count\": 12"));
    assert!(json.contains("\"kind\": \"handoff_missing\""));
    assert!(json.contains("\"blocking\": false"));
    assert!(json.contains("\"package\": \"country_rus\""));
    assert!(json.contains("\"package\": \"region_europe\""));
    assert!(json.contains("\"kind\": \"missing_package_artifact\""));
    assert!(json.contains("changed_region_europe.txt"));
    assert!(json.contains("hoi4skill split-changed-work-packages --mod-root"));
    assert!(json.contains("hoi4skill asset-pack-plan --mod-root"));
    assert!(json.contains("Do not skip boundary or validation artifacts"));
}

#[test]
fn large_mod_production_snapshot_and_brief_summarize_handoff_state() {
    let root = unique_temp_dir("large-mod-production-snapshot");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let snapshot_output = root.join("production_snapshot.json");
    let brief_output = root.join("production_brief.md");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    for (name, text) in [
        (
            "mod_index.json",
            "{\n  \"schema\": \"hoi4skill.mod_index.v1\"\n}\n",
        ),
        (
            "ownership_map.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_ownership_map.v1\"\n}\n",
        ),
        (
            "loc_audit.json",
            "{\n  \"schema\": \"hoi4skill.loc_audit.v1\",\n  \"missing_count\": 0\n}\n",
        ),
        (
            "gfx_audit.json",
            "{\n  \"schema\": \"hoi4skill.gfx_audit.v1\",\n  \"missing_sprites_count\": 0\n}\n",
        ),
        (
            "logic_audit.json",
            "{\n  \"schema\": \"hoi4skill.logic_audit.v1\",\n  \"issue_count\": 0\n}\n",
        ),
        (
            "validation.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        (
            "regression_gate.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_regression_gate.v1\",\n  \"regression_passed\": true,\n  \"blocking_count\": 0\n}\n",
        ),
        (
            "work_package_status.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "readiness.json",
            "{\n  \"schema\": \"hoi4skill.work_package_readiness.v1\",\n  \"blocked_count\": 0,\n  \"missing_package_count\": 0\n}\n",
        ),
        (
            "risk_register.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_risk_register.v1\",\n  \"risk_count\": 0,\n  \"blocking_count\": 0\n}\n",
        ),
        (
            "next_actions.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_next_actions.v1\",\n  \"blocking_count\": 0\n}\n",
        ),
        (
            "changed_country_rus.txt",
            "events/rus_events.txt\n",
        ),
        (
            "changed_system_black_monday.txt",
            "common/scripted_effects/tgc_black_monday.txt\n",
        ),
        (
            "plan_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        ),
        (
            "plan_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        ),
        ("assets_country_rus.md", "# Asset Pack Plan\n"),
        ("assets_system_black_monday.md", "# Asset Pack Plan\n"),
        (
            "boundary_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        ),
        (
            "boundary_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        ),
        (
            "status_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "status_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "validation_country_rus.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        (
            "validation_system_black_monday.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        ("handoff_country_rus.md", "# Work Package Handoff\n"),
        (
            "handoff_system_black_monday.md",
            "# Work Package Handoff\n",
        ),
    ] {
        fs::write(mod_root.join(".hoi4skill").join(name), text).unwrap();
    }

    cmd_work_package_claim(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--assignee".to_string(),
        "codex-a".to_string(),
        "--output".to_string(),
        mod_root
            .join(".hoi4skill/claims/claim_country_rus.json")
            .to_string_lossy()
            .to_string(),
    ])
    .unwrap();

    cmd_large_mod_production_snapshot(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        snapshot_output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let snapshot = read_utf8_lossy(&snapshot_output).unwrap();
    assert!(snapshot.contains("\"schema\": \"hoi4skill.large_mod_production_snapshot.v1\""));
    assert!(snapshot.contains("\"decision\": \"blocked\""));
    assert!(snapshot.contains("\"ready_package_count\": 2"));
    assert!(snapshot.contains("\"handoff_count\": 2"));
    assert!(snapshot.contains("\"claimed_count\": 1"));
    assert!(snapshot.contains("\"blocked_package_count\": 1"));
    assert!(snapshot.contains("\"missing_required_report_count\": 0"));
    assert!(snapshot.contains("\"kind\": \"risk_register\""));
    assert!(snapshot.contains("\"kind\": \"next_actions\""));
    assert!(snapshot.contains("\"stage\": \"handoff_ready\""));
    assert!(snapshot.contains("\"stage\": \"blocked\""));
    assert!(snapshot.contains("large-mod-production-brief --mod-root"));
    assert!(
        snapshot.contains("Do not hand off production while blocking_count is greater than zero")
    );

    cmd_large_mod_production_brief(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        brief_output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let brief = read_utf8_lossy(&brief_output).unwrap();
    assert!(brief.contains("# Large Mod Production Brief: Test Grand Campaign"));
    assert!(brief.contains("`hoi4skill.large_mod_production_brief.v1`"));
    assert!(brief.contains("- decision: `blocked`"));
    assert!(brief.contains("| `country_rus` | `country` | `handoff_ready` |"));
    assert!(brief.contains("| `region_europe` | `region` | `blocked` |"));
    assert!(brief.contains("| `risk_register.json` | `risk_register` | `false` | `true` | `ok` |"));
    assert!(brief.contains("large-mod-production-snapshot --mod-root"));
    assert!(brief.contains("Do not use this snapshot as a substitute"));
}

#[test]
fn large_mod_fix_queue_routes_reports_to_packages() {
    let root = unique_temp_dir("large-mod-fix-queue");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("fix_queue.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    fs::write(
        mod_root.join(".hoi4skill/error_log_country_rus.json"),
        r#"{
  "schema": "hoi4skill.error_log_report.v1",
  "diagnostics_effective": 1,
  "diagnostics": [
    {"severity": "error", "category": "syntax", "file": "events/rus_events.txt", "line": 12, "message": "Unexpected token", "suggestion": "Check braces near the event."}
  ]
}
"#,
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/validation_system_black_monday.json"),
        r#"{
  "schema": "hoi4skill.validation.v1",
  "ok": false,
  "error_count": 2
}
"#,
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/logic_audit.json"),
        r#"{
  "schema": "hoi4skill.logic_audit.v1",
  "issue_count": 1
}
"#,
    )
    .unwrap();

    cmd_large_mod_fix_queue(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.large_mod_fix_queue.v1\""));
    assert!(json.contains("\"healthy\": false"));
    assert!(json.contains("\"item_count\": 3"));
    assert!(json.contains("\"blocking_count\": 3"));
    assert!(json.contains("\"high_count\": 2"));
    assert!(json.contains("\"unassigned_count\": 1"));
    assert!(json.contains("\"affected_packages\": [\"country_rus\", \"system_black_monday\"]"));
    assert!(json.contains("\"package\": \"country_rus\""));
    assert!(json.contains("\"kind\": \"error_log_syntax\""));
    assert!(json.contains("events/rus_events.txt:12"));
    assert!(json.contains("\"package\": \"system_black_monday\""));
    assert!(json.contains("\"kind\": \"validation_failure\""));
    assert!(json.contains("\"kind\": \"logic_audit\""));
    assert!(json.contains("work-package-start-brief --mod-root"));
    assert!(json.contains("large-mod-risk-register --mod-root"));
    assert!(json.contains("Unassigned error-log items must be routed"));
}

#[test]
fn large_mod_regression_plan_groups_fix_items_by_package() {
    let root = unique_temp_dir("large-mod-regression-plan");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("regression_plan.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    fs::write(
        mod_root.join(".hoi4skill/error_log_country_rus.json"),
        r#"{
  "schema": "hoi4skill.error_log_report.v1",
  "diagnostics_effective": 1,
  "diagnostics": [
    {"severity": "error", "category": "syntax", "file": "events/rus_events.txt", "line": 12, "message": "Unexpected token", "suggestion": "Check braces near the event."}
  ]
}
"#,
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/validation_system_black_monday.json"),
        r#"{
  "schema": "hoi4skill.validation.v1",
  "ok": false,
  "error_count": 2
}
"#,
    )
    .unwrap();
    fs::write(
        mod_root.join(".hoi4skill/logic_audit.json"),
        r#"{
  "schema": "hoi4skill.logic_audit.v1",
  "issue_count": 1
}
"#,
    )
    .unwrap();

    cmd_large_mod_regression_plan(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.large_mod_regression_plan.v1\""));
    assert!(json.contains("\"healthy\": false"));
    assert!(json.contains("\"fix_item_count\": 3"));
    assert!(json.contains("\"scenario_count\": 3"));
    assert!(json.contains("\"package_scenario_count\": 2"));
    assert!(json.contains("\"unassigned_count\": 1"));
    assert!(json.contains("\"affected_packages\": [\"country_rus\", \"system_black_monday\"]"));
    assert!(json.contains("\"package\": \"country_rus\""));
    assert!(json.contains("\"contexts\": [\"events/rus_events.txt:12\"]"));
    assert!(json.contains("--changed events/rus_events.txt --strict-code-index"));
    assert!(json.contains("error_log_country_rus.json"));
    assert!(json.contains("\"package\": \"system_black_monday\""));
    assert!(json.contains("validation_system_black_monday.json"));
    assert!(json.contains("\"status\": \"routing_required\""));
    assert!(json.contains("identify-work-packages --mod-root"));
    assert!(json.contains("large-mod-playtest-gate --mod-root"));
    assert!(json.contains("Do not close a fix queue item"));
}

#[test]
fn large_mod_regression_gate_requires_clean_rerun_evidence() {
    let root = unique_temp_dir("large-mod-regression-gate");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let blocked_output = root.join("regression_gate_blocked.json");
    let passed_output = root.join("regression_gate_passed.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    fs::write(
        mod_root.join(".hoi4skill/regression_plan.json"),
        r#"{
  "schema": "hoi4skill.large_mod_regression_plan.v1",
  "affected_packages": ["country_rus", "system_black_monday"],
  "unassigned_count": 0
}
"#,
    )
    .unwrap();

    for package in ["country_rus", "system_black_monday"] {
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("boundary_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("validation_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true,\n  \"error_count\": 0\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("error_log_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.error_log_report.v1\",\n  \"diagnostics_effective\": 0,\n  \"diagnostics\": []\n}\n",
        )
        .unwrap();
    }
    fs::write(
        mod_root.join(".hoi4skill/playtest_country_rus.json"),
        "{\n  \"schema\": \"hoi4skill.playtest_report.v1\",\n  \"ok\": true,\n  \"status\": \"passed\",\n  \"finding_count\": 0\n}\n",
    )
    .unwrap();

    cmd_large_mod_regression_gate(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        blocked_output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let blocked = read_utf8_lossy(&blocked_output).unwrap();
    assert!(blocked.contains("\"schema\": \"hoi4skill.large_mod_regression_gate.v1\""));
    assert!(blocked.contains("\"regression_passed\": false"));
    assert!(blocked.contains("\"blocking_count\": 1"));
    assert!(blocked.contains("\"package\": \"system_black_monday\""));
    assert!(blocked.contains("\"kind\": \"playtest\""));
    assert!(blocked.contains("playtest_system_black_monday.json"));

    fs::write(
        mod_root.join(".hoi4skill/playtest_system_black_monday.json"),
        "{\n  \"schema\": \"hoi4skill.playtest_report.v1\",\n  \"ok\": true,\n  \"status\": \"passed\",\n  \"finding_count\": 0\n}\n",
    )
    .unwrap();

    cmd_large_mod_regression_gate(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        passed_output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let passed = read_utf8_lossy(&passed_output).unwrap();
    assert!(passed.contains("\"regression_passed\": true"));
    assert!(passed.contains("\"blocking_count\": 0"));
    assert!(passed.contains("\"affected_package_count\": 2"));
    assert!(passed.contains("\"status\": \"passed\""));
    assert!(passed.contains("large-mod-release-gate --mod-root"));
    assert!(passed.contains("regression_passed=true"));
}

#[test]
fn large_mod_regression_brief_summarizes_gate_blockers() {
    let root = unique_temp_dir("large-mod-regression-brief");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("regression_brief.md");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    fs::write(
        mod_root.join(".hoi4skill/regression_plan.json"),
        r#"{
  "schema": "hoi4skill.large_mod_regression_plan.v1",
  "affected_packages": ["country_rus", "system_black_monday"],
  "unassigned_count": 0
}
"#,
    )
    .unwrap();

    for package in ["country_rus", "system_black_monday"] {
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("boundary_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("validation_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true,\n  \"error_count\": 0\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("error_log_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.error_log_report.v1\",\n  \"diagnostics_effective\": 0,\n  \"diagnostics\": []\n}\n",
        )
        .unwrap();
    }
    fs::write(
        mod_root.join(".hoi4skill/playtest_country_rus.json"),
        "{\n  \"schema\": \"hoi4skill.playtest_report.v1\",\n  \"ok\": true,\n  \"status\": \"passed\",\n  \"finding_count\": 0\n}\n",
    )
    .unwrap();

    cmd_large_mod_regression_brief(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let markdown = read_utf8_lossy(&output).unwrap();
    assert!(markdown.contains("`hoi4skill.large_mod_regression_brief.v1`"));
    assert!(markdown.contains("- decision: `blocked`"));
    assert!(markdown.contains("system_black_monday: playtest is missing"));
    assert!(markdown.contains("| `country_rus` | `country` | `passed` |"));
    assert!(markdown.contains("| `system_black_monday` | `system` | `blocked` |"));
    assert!(markdown.contains("playtest:missing"));
    assert!(markdown.contains("large-mod-regression-gate --mod-root"));
    assert!(markdown.contains("large-mod-release-gate --mod-root"));
    assert!(markdown.contains("Do not close regression while decision is `blocked`"));
}

#[test]
fn large_mod_risk_register_prioritizes_release_and_dispatch_risks() {
    let root = unique_temp_dir("large-mod-risk-register");
    let blueprint_path = root.join("blueprint.yml");
    let mod_root = root.join("mod");
    let output = root.join("risk_register.json");

    cmd_plan_large_mod(&[
        "--text".to_string(),
        "name: Test Grand Campaign\ncountries: RUS, GER\nregions: europe\nsystems: black monday"
            .to_string(),
        "--name".to_string(),
        "Test Grand Campaign".to_string(),
        "--acronym".to_string(),
        "TGC".to_string(),
        "--output".to_string(),
        blueprint_path.to_string_lossy().to_string(),
    ])
    .unwrap();

    cmd_init_large_mod(&[
        "--blueprint".to_string(),
        blueprint_path.to_string_lossy().to_string(),
        "--output".to_string(),
        mod_root.to_string_lossy().to_string(),
    ])
    .unwrap();

    for (name, text) in [
        (
            "mod_index.json",
            "{\n  \"schema\": \"hoi4skill.mod_index.v1\"\n}\n",
        ),
        (
            "ownership_map.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_ownership_map.v1\"\n}\n",
        ),
        (
            "loc_audit.json",
            "{\n  \"schema\": \"hoi4skill.loc_audit.v1\",\n  \"missing_count\": 0\n}\n",
        ),
        (
            "logic_audit.json",
            "{\n  \"schema\": \"hoi4skill.logic_audit.v1\",\n  \"issue_count\": 0\n}\n",
        ),
        (
            "validation.json",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        (
            "regression_gate.json",
            "{\n  \"schema\": \"hoi4skill.large_mod_regression_gate.v1\",\n  \"regression_passed\": true,\n  \"blocking_count\": 0\n}\n",
        ),
        (
            "work_package_status.json",
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        ),
        (
            "readiness.json",
            "{\n  \"schema\": \"hoi4skill.work_package_readiness.v1\",\n  \"blocked_count\": 0,\n  \"missing_package_count\": 0\n}\n",
        ),
    ] {
        fs::write(mod_root.join(".hoi4skill").join(name), text).unwrap();
    }

    for (package, changed, validation) in [
        (
            "system_black_monday",
            "common/scripted_effects/tgc_black_monday.txt\n",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        (
            "country_ger",
            "events/ger_events.txt\n",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": true\n}\n",
        ),
        (
            "country_rus",
            "events/rus_events.txt\n",
            "{\n  \"schema\": \"hoi4skill.validation.v1\",\n  \"ok\": false,\n  \"error_count\": 1\n}\n",
        ),
    ] {
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("changed_{package}.txt")),
            changed,
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("plan_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.generate_work_package_plan.v1\",\n  \"dry_run\": true\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("assets_{package}.md")),
            "# Asset Pack Plan\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("boundary_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.work_package_boundary.v1\",\n  \"ok\": true,\n  \"violation_count\": 0\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("status_{package}.json")),
            "{\n  \"schema\": \"hoi4skill.work_package_status.v1\",\n  \"status\": \"ok\"\n}\n",
        )
        .unwrap();
        fs::write(
            mod_root
                .join(".hoi4skill")
                .join(format!("validation_{package}.json")),
            validation,
        )
        .unwrap();
    }
    fs::write(
        mod_root.join(".hoi4skill/handoff_system_black_monday.md"),
        "# Work Package Handoff\n",
    )
    .unwrap();

    cmd_work_package_claim(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--package".to_string(),
        "country_rus".to_string(),
        "--assignee".to_string(),
        "codex-a".to_string(),
        "--allow-blocked".to_string(),
    ])
    .unwrap();

    cmd_large_mod_risk_register(&[
        "--mod-root".to_string(),
        mod_root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.large_mod_risk_register.v1\""));
    assert!(json.contains("\"healthy\": false"));
    assert!(json.contains("\"package_count\": 4"));
    assert!(json.contains("\"risk_count\": 11"));
    assert!(json.contains("\"blocking_count\": 9"));
    assert!(json.contains("\"high_count\": 6"));
    assert!(json.contains("\"medium_count\": 5"));
    assert!(json.contains("\"kind\": \"missing_required_report\""));
    assert!(json.contains("gfx_audit.json"));
    assert!(json.contains("\"kind\": \"package_artifact_needs_review\""));
    assert!(json.contains("\"kind\": \"blocked_claim\""));
    assert!(json.contains("\"kind\": \"ready_package_unclaimed\""));
    assert!(json.contains("\"kind\": \"handoff_missing\""));
    assert!(json.contains("\"package\": \"region_europe\""));
    assert!(json.contains("work-package-release-claim --mod-root"));
    assert!(json.contains("work-package-claim --mod-root"));
    assert!(json.contains("large-mod-risk-register --mod-root"));
    assert!(json.contains("Do not release while high severity or blocking risks remain"));
}

fn write_mod_index_fixture(name: &str) -> PathBuf {
    let root = unique_temp_dir(name);
    fs::create_dir_all(root.join("common/national_focus")).unwrap();
    fs::create_dir_all(root.join("common/ideas")).unwrap();
    fs::create_dir_all(root.join("common/decisions/categories")).unwrap();
    fs::create_dir_all(root.join("common/scripted_effects")).unwrap();
    fs::create_dir_all(root.join("common/country_tags")).unwrap();
    fs::create_dir_all(root.join("events")).unwrap();
    fs::create_dir_all(root.join("interface")).unwrap();
    fs::create_dir_all(root.join("localisation/simp_chinese")).unwrap();

    fs::write(
        root.join("common/national_focus/tst.txt"),
        r#"
focus_tree = {
  id = tst_focus
  country = { factor = 0 modifier = { add = 10 tag = TST } }
  focus = {
    id = TST_rebuild_state
    icon = GFX_goal_tst_rebuild
    x = 0
    y = 0
    cost = 10
  }
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("events/tst_events.txt"),
        r#"
add_namespace = tst
country_event = {
  id = tst.1
  title = tst.1.t
  desc = tst.1.d
  picture = GFX_report_event_tst_rebuild
  is_triggered_only = yes
  option = { name = tst.1.a }
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("common/ideas/tst_ideas.txt"),
        r#"
ideas = {
  country = {
    TST_rebuilding_spirit = {
      picture = generic_production_bonus
    }
  }
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("common/decisions/categories/tst_categories.txt"),
        r#"
TST_rebuild_category = {
  icon = GFX_decision_tst_rebuild
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("common/decisions/tst_decisions.txt"),
        r#"
TST_rebuild_category = {
  TST_start_rebuild = {
    icon = GFX_decision_tst_rebuild
    cost = 25
  }
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("common/scripted_effects/tst_effects.txt"),
        r#"
TST_rebuild_effect = {
  add_political_power = 25
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("common/country_tags/tst_tags.txt"),
        "TST = \"countries/TST.txt\"\n",
    )
    .unwrap();
    fs::write(
        root.join("interface/tst.gfx"),
        r#"
spriteTypes = {
  spriteType = {
    name = "GFX_goal_tst_rebuild"
    texturefile = "gfx/interface/goals/tst_rebuild.dds"
  }
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("localisation/simp_chinese/tst_l_simp_chinese.yml"),
        "\u{feff}l_simp_chinese:\n TST_rebuild_state:0 \"重建国家\"\n tst.1.t:0 \"重建开始\"\n TST_rebuilding_spirit:0 \"重建精神\"\n",
    )
    .unwrap();
    root
}

#[test]
fn build_mod_index_collects_core_large_mod_symbols() {
    let root = write_mod_index_fixture("mod-index");
    let output = root.join("mod_index.json");
    cmd_build_mod_index(&[
        root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();
    let json = read_utf8_lossy(&output).unwrap();

    assert!(json.contains("\"schema\": \"hoi4skill.mod_index.v1\""));
    assert!(json.contains("\"kind\": \"focus\", \"id\": \"TST_rebuild_state\""));
    assert!(json.contains("\"kind\": \"event\", \"id\": \"tst.1\""));
    assert!(json.contains("\"kind\": \"idea\", \"id\": \"TST_rebuilding_spirit\""));
    assert!(json.contains("\"kind\": \"decision_category\", \"id\": \"TST_rebuild_category\""));
    assert!(json.contains("\"kind\": \"decision\", \"id\": \"TST_start_rebuild\""));
    assert!(json.contains("\"kind\": \"scripted_effect\", \"id\": \"TST_rebuild_effect\""));
    assert!(json.contains("\"kind\": \"country_tag\", \"id\": \"TST\""));
    assert!(json.contains("\"kind\": \"sprite\", \"id\": \"GFX_goal_tst_rebuild\""));
    assert!(json.contains("\"kind\": \"localisation\", \"id\": \"TST_rebuild_state\""));
    assert!(json.contains("\"by_kind\""));
}

#[test]
fn query_symbol_finds_exact_and_contains_matches() {
    let root = write_mod_index_fixture("query-symbol");
    let exact = root.join("exact.json");
    cmd_query_symbol(&[
        root.to_string_lossy().to_string(),
        "--symbol".to_string(),
        "TST_rebuild_state".to_string(),
        "--kind".to_string(),
        "focus".to_string(),
        "--output".to_string(),
        exact.to_string_lossy().to_string(),
    ])
    .unwrap();
    let exact_json = read_utf8_lossy(&exact).unwrap();
    assert!(exact_json.contains("\"schema\": \"hoi4skill.query_symbol.v1\""));
    assert!(exact_json.contains("\"matches\": 1"));
    assert!(exact_json.contains("\"kind\": \"focus\", \"id\": \"TST_rebuild_state\""));
    assert!(!exact_json.contains("\"kind\": \"localisation\", \"id\": \"TST_rebuild_state\""));

    let contains = root.join("contains.json");
    cmd_query_symbol(&[
        root.to_string_lossy().to_string(),
        "--symbol".to_string(),
        "rebuild".to_string(),
        "--contains".to_string(),
        "--max-results".to_string(),
        "20".to_string(),
        "--output".to_string(),
        contains.to_string_lossy().to_string(),
    ])
    .unwrap();
    let contains_json = read_utf8_lossy(&contains).unwrap();
    assert!(contains_json.contains("\"contains\": true"));
    assert!(contains_json.contains("\"kind\": \"focus\", \"id\": \"TST_rebuild_state\""));
    assert!(contains_json.contains("\"kind\": \"sprite\", \"id\": \"GFX_goal_tst_rebuild\""));
    assert!(contains_json.contains("\"kind\": \"scripted_effect\", \"id\": \"TST_rebuild_effect\""));
}

#[test]
fn impact_reports_seed_symbols_related_files_and_references() {
    let root = write_mod_index_fixture("impact");
    fs::write(
        root.join("events/tst_followup.txt"),
        r#"
add_namespace = tst
country_event = {
  id = tst.2
  title = tst.2.t
  desc = tst.2.d
  is_triggered_only = yes
  immediate = { country_event = { id = tst.1 } }
  option = { name = tst.2.a add_ideas = TST_rebuilding_spirit }
}
"#,
    )
    .unwrap();

    let symbol_report = root.join("impact_symbol.json");
    cmd_impact(&[
        root.to_string_lossy().to_string(),
        "--symbol".to_string(),
        "tst.1".to_string(),
        "--output".to_string(),
        symbol_report.to_string_lossy().to_string(),
    ])
    .unwrap();
    let symbol_json = read_utf8_lossy(&symbol_report).unwrap();
    assert!(symbol_json.contains("\"schema\": \"hoi4skill.impact.v1\""));
    assert!(symbol_json.contains("\"query_symbol\": \"tst.1\""));
    assert!(symbol_json.contains("\"kind\": \"event\", \"id\": \"tst.1\""));
    assert!(symbol_json.contains("\"file\": \"events/tst_followup.txt\""));
    assert!(symbol_json.contains("\"relation\": \"text_reference\""));
    assert!(symbol_json.contains("analyze-error-log"));

    let changed_report = root.join("impact_changed.json");
    cmd_impact(&[
        root.to_string_lossy().to_string(),
        "--changed".to_string(),
        "common/national_focus/tst.txt".to_string(),
        "--output".to_string(),
        changed_report.to_string_lossy().to_string(),
    ])
    .unwrap();
    let changed_json = read_utf8_lossy(&changed_report).unwrap();
    assert!(changed_json.contains("\"changed_file\": \"common/national_focus/tst.txt\""));
    assert!(changed_json.contains("\"kind\": \"focus\", \"id\": \"TST_rebuild_state\""));
    assert!(changed_json.contains("\"affected_files\""));
}

#[test]
fn reserve_id_suggests_non_colliding_event_and_focus_ids() {
    let root = write_mod_index_fixture("reserve-id");

    let event_ids = root.join("event_ids.json");
    cmd_reserve_id(&[
        root.to_string_lossy().to_string(),
        "--kind".to_string(),
        "event".to_string(),
        "--namespace".to_string(),
        "tst".to_string(),
        "--count".to_string(),
        "2".to_string(),
        "--output".to_string(),
        event_ids.to_string_lossy().to_string(),
    ])
    .unwrap();
    let event_json = read_utf8_lossy(&event_ids).unwrap();
    assert!(event_json.contains("\"schema\": \"hoi4skill.reserve_id.v1\""));
    assert!(event_json.contains("\"kind\": \"event\""));
    assert!(event_json.contains("\"namespace\": \"tst\""));
    assert!(event_json.contains("\"existing_event_max\": 1"));
    assert!(event_json.contains("\"ids\": [\"tst.2\", \"tst.3\"]"));

    let focus_ids = root.join("focus_ids.json");
    cmd_reserve_id(&[
        root.to_string_lossy().to_string(),
        "--kind".to_string(),
        "focus".to_string(),
        "--tag".to_string(),
        "TST".to_string(),
        "--count".to_string(),
        "2".to_string(),
        "--output".to_string(),
        focus_ids.to_string_lossy().to_string(),
    ])
    .unwrap();
    let focus_json = read_utf8_lossy(&focus_ids).unwrap();
    assert!(focus_json.contains("\"kind\": \"focus\""));
    assert!(focus_json.contains("\"prefix\": \"TST\""));
    assert!(focus_json.contains("\"ids\": [\"TST_focus_001\", \"TST_focus_002\"]"));
    assert!(focus_json.contains("Run hoi4skill build-mod-index again"));
}

#[test]
fn check_namespace_reports_event_namespace_collisions_and_next_ids() {
    let root = write_mod_index_fixture("check-namespace");
    fs::write(
        root.join("events/tst_duplicate.txt"),
        r#"
add_namespace = tst
country_event = {
  id = tst.1
  title = tst.1.t
  desc = tst.1.d
  is_triggered_only = yes
  option = { name = tst.1.a }
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("events/other_no_namespace.txt"),
        r#"
country_event = {
  id = other.1
  title = other.1.t
  desc = other.1.d
  is_triggered_only = yes
  option = { name = other.1.a }
}
"#,
    )
    .unwrap();

    let output = root.join("namespace.json");
    cmd_check_namespace(&[
        root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();
    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.namespace_check.v1\""));
    assert!(json.contains("\"ok\": false"));
    assert!(json.contains("\"namespace\": \"tst\""));
    assert!(json.contains("\"max_id\": 1"));
    assert!(json.contains("\"next_id\": 2"));
    assert!(json.contains("\"duplicate_event_id_count\": 1"));
    assert!(json.contains("\"id\": \"tst.1\""));
    assert!(json.contains("namespace tst is declared in multiple files"));
    assert!(json.contains(
        "event id other.1 appears in events/other_no_namespace.txt without add_namespace = other"
    ));

    let filtered = root.join("namespace_tst.json");
    cmd_check_namespace(&[
        root.to_string_lossy().to_string(),
        "--namespace".to_string(),
        "tst".to_string(),
        "--output".to_string(),
        filtered.to_string_lossy().to_string(),
    ])
    .unwrap();
    let filtered_json = read_utf8_lossy(&filtered).unwrap();
    assert!(filtered_json.contains("\"namespace_filter\": \"tst\""));
    assert!(filtered_json.contains("reserve-id <mod-root> --kind event --namespace tst"));
    assert!(!filtered_json.contains("\"namespace\": \"other\""));
}

#[test]
fn feature_context_writes_tag_and_system_markdown() {
    let root = write_mod_index_fixture("feature-context");

    let tag_context = root.join("tag_context.md");
    cmd_feature_context(&[
        root.to_string_lossy().to_string(),
        "--tag".to_string(),
        "TST".to_string(),
        "--output".to_string(),
        tag_context.to_string_lossy().to_string(),
    ])
    .unwrap();
    let tag_md = read_utf8_lossy(&tag_context).unwrap();
    assert!(tag_md.contains("# HOI4 Feature Context"));
    assert!(tag_md.contains("- tag: `TST`"));
    assert!(tag_md.contains("`focus` `TST_rebuild_state`"));
    assert!(tag_md.contains("`country_tag` `TST`"));
    assert!(tag_md.contains("common/national_focus"));
    assert!(tag_md.contains("Do not create country tags"));

    let system_context = root.join("system_context.md");
    cmd_feature_context(&[
        root.to_string_lossy().to_string(),
        "--system".to_string(),
        "rebuild".to_string(),
        "--output".to_string(),
        system_context.to_string_lossy().to_string(),
    ])
    .unwrap();
    let system_md = read_utf8_lossy(&system_context).unwrap();
    assert!(system_md.contains("- system: `rebuild`"));
    assert!(system_md.contains("`TST_rebuild_state`"));
    assert!(system_md.contains("`TST_rebuild_effect`"));
    assert!(system_md.contains("query-symbol"));
    assert!(system_md.contains("## Text References"));
}

#[test]
fn validate_baseline_filters_existing_errors() {
    let root = unique_temp_dir("validate-baseline");
    fs::create_dir_all(root.join("localisation/simp_chinese")).unwrap();
    fs::write(
        root.join("descriptor.mod"),
        "name=\"Baseline Test\"\nsupported_version=\"*\"\n",
    )
    .unwrap();
    fs::write(
        root.join("localisation/simp_chinese/bad_l_simp_chinese.yml"),
        "l_simp_chinese:\n TST_bad:0 \"坏文本\"\n",
    )
    .unwrap();

    let baseline = root.join("baseline.json");
    let first = cmd_validate(&[
        root.to_string_lossy().to_string(),
        "--output".to_string(),
        baseline.to_string_lossy().to_string(),
    ]);
    assert!(first.is_err());
    let baseline_json = read_utf8_lossy(&baseline).unwrap();
    assert!(baseline_json.contains("\"effective_errors\": 1"));
    assert!(baseline_json.contains("localisation file has no UTF-8 BOM"));

    let filtered = root.join("filtered.json");
    let second = cmd_validate(&[
        root.to_string_lossy().to_string(),
        "--baseline".to_string(),
        baseline.to_string_lossy().to_string(),
        "--output".to_string(),
        filtered.to_string_lossy().to_string(),
    ]);
    assert!(second.is_ok());
    let filtered_json = read_utf8_lossy(&filtered).unwrap();
    assert!(filtered_json.contains("\"total_errors\": 1"));
    assert!(filtered_json.contains("\"effective_errors\": 0"));
    assert!(filtered_json.contains("\"baseline_errors_filtered\": 1"));
}

#[test]
fn validate_changed_only_filters_to_changed_files() {
    let root = unique_temp_dir("validate-changed-only");
    fs::create_dir_all(root.join("localisation/simp_chinese")).unwrap();
    fs::write(
        root.join("descriptor.mod"),
        "name=\"Changed Test\"\nsupported_version=\"*\"\n",
    )
    .unwrap();
    fs::write(
        root.join("localisation/simp_chinese/one_l_simp_chinese.yml"),
        "l_simp_chinese:\n TST_one:0 \"一\"\n",
    )
    .unwrap();
    fs::write(
        root.join("localisation/simp_chinese/two_l_simp_chinese.yml"),
        "l_simp_chinese:\n TST_two:0 \"二\"\n",
    )
    .unwrap();

    let output = root.join("changed.json");
    let result = cmd_validate(&[
        root.to_string_lossy().to_string(),
        "--changed-only".to_string(),
        "--changed".to_string(),
        "localisation/simp_chinese/one_l_simp_chinese.yml".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ]);
    assert!(result.is_err());
    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"total_errors\": 2"));
    assert!(json.contains("\"effective_errors\": 1"));
    assert!(json.contains("one_l_simp_chinese.yml"));
    assert!(!json.contains("two_l_simp_chinese.yml"));
}

#[test]
fn loc_audit_reports_missing_orphan_duplicate_and_changed_only() {
    let root = unique_temp_dir("loc-audit");
    fs::create_dir_all(root.join("common/national_focus")).unwrap();
    fs::create_dir_all(root.join("localisation/simp_chinese")).unwrap();
    fs::write(
        root.join("common/national_focus/tst.txt"),
        r#"
focus_tree = {
  id = tst_focus
  focus = {
    id = TST_focus_one
    x = 0
    y = 0
  }
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("localisation/simp_chinese/tst_l_simp_chinese.yml"),
        "\u{feff}l_simp_chinese:\n TST_focus_one:0 \"第一个国策\"\n TST_orphan:0 \"孤儿文本\"\n TST_duplicate:0 \"重复一\"\n TST_duplicate:0 \"重复二\"\n",
    )
    .unwrap();

    let output = root.join("loc_audit.json");
    cmd_loc_audit(&[
        root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();
    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.loc_audit.v1\""));
    assert!(json.contains("\"missing_count\": 1"));
    assert!(json.contains("\"key\": \"TST_focus_one_desc\""));
    assert!(json.contains("\"key\": \"TST_orphan\""));
    assert!(json.contains("\"duplicate_count\": 1"));
    assert!(json.contains("\"simp_chinese\": 1"));

    let changed_output = root.join("loc_audit_changed.json");
    cmd_loc_audit(&[
        root.to_string_lossy().to_string(),
        "--changed-only".to_string(),
        "--changed".to_string(),
        "common/national_focus/tst.txt".to_string(),
        "--output".to_string(),
        changed_output.to_string_lossy().to_string(),
    ])
    .unwrap();
    let changed_json = read_utf8_lossy(&changed_output).unwrap();
    assert!(changed_json.contains("\"missing_count\": 1"));
    assert!(changed_json.contains("\"key\": \"TST_focus_one_desc\""));
    assert!(changed_json.contains("\"orphan_count\": 0"));
    assert!(changed_json.contains("\"duplicate_count\": 0"));
}

#[test]
fn loc_sync_report_compares_two_language_key_sets() {
    let root = unique_temp_dir("loc-sync-report");
    fs::create_dir_all(root.join("localisation/english")).unwrap();
    fs::create_dir_all(root.join("localisation/simp_chinese")).unwrap();
    fs::write(
        root.join("localisation/english/tst_l_english.yml"),
        "\u{feff}l_english:\n TST_focus:0 \"Focus\"\n TST_focus_desc:0 \"Desc\"\n TST_duplicate:0 \"One\"\n TST_duplicate:0 \"Two\"\n",
    )
    .unwrap();
    fs::write(
        root.join("localisation/simp_chinese/tst_l_simp_chinese.yml"),
        "\u{feff}l_simp_chinese:\n TST_focus:0 \"国策\"\n TST_extra:0 \"额外\"\n",
    )
    .unwrap();

    let output = root.join("loc_sync.json");
    cmd_loc_sync_report(&[
        root.to_string_lossy().to_string(),
        "--from".to_string(),
        "english".to_string(),
        "--to".to_string(),
        "simp_chinese".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();
    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.loc_sync_report.v1\""));
    assert!(json.contains("\"from\": \"english\""));
    assert!(json.contains("\"to\": \"simp_chinese\""));
    assert!(json.contains("\"from_keys_total\": 3"));
    assert!(json.contains("\"to_keys_total\": 2"));
    assert!(json.contains("\"common_count\": 1"));
    assert!(json.contains("\"missing_in_to_count\": 2"));
    assert!(json.contains("\"key\": \"TST_focus_desc\""));
    assert!(json.contains("\"key\": \"TST_duplicate\""));
    assert!(json.contains("\"extra_in_to_count\": 1"));
    assert!(json.contains("\"key\": \"TST_extra\""));
    assert!(json.contains("\"duplicate_from_count\": 1"));
    assert!(json
        .contains("translate-localisation --mod-root <mod-root> --from english --to simp_chinese"));

    let same = cmd_loc_sync_report(&[
        root.to_string_lossy().to_string(),
        "--from".to_string(),
        "english".to_string(),
        "--to".to_string(),
        "english".to_string(),
    ]);
    assert!(same.is_err());
    assert!(same.unwrap_err().contains("must be different languages"));
}

#[test]
fn gfx_audit_reports_missing_textures_refs_orphans_and_unregistered_images() {
    let root = unique_temp_dir("gfx-audit");
    fs::create_dir_all(root.join("common/national_focus")).unwrap();
    fs::create_dir_all(root.join("interface")).unwrap();
    fs::create_dir_all(root.join("gfx/interface/goals")).unwrap();
    fs::write(root.join("gfx/interface/goals/used.dds"), "fake").unwrap();
    fs::write(root.join("gfx/interface/goals/unregistered.dds"), "fake").unwrap();
    fs::write(
        root.join("interface/tst.gfx"),
        r#"
spriteTypes = {
  spriteType = {
    name = "GFX_goal_used"
    texturefile = "gfx/interface/goals/used.dds"
  }
  spriteType = {
    name = "GFX_goal_missing_texture"
    texturefile = "gfx/interface/goals/missing.dds"
  }
  spriteType = {
    name = "GFX_goal_orphan"
    texturefile = "gfx/interface/goals/used.dds"
  }
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("common/national_focus/tst.txt"),
        r#"
focus_tree = {
  id = tst_focus
  focus = {
    id = TST_used
    icon = GFX_goal_used
  }
  focus = {
    id = TST_missing
    icon = GFX_goal_missing_ref
  }
}
"#,
    )
    .unwrap();

    let output = root.join("gfx_audit.json");
    cmd_gfx_audit(&[
        root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();
    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.gfx_audit.v1\""));
    assert!(json.contains("\"missing_textures_count\": 1"));
    assert!(json.contains("\"id\": \"GFX_goal_missing_texture\""));
    assert!(json.contains("\"missing_sprites_count\": 1"));
    assert!(json.contains("\"id\": \"GFX_goal_missing_ref\""));
    assert!(json.contains("\"orphan_sprites_count\": 2"));
    assert!(json.contains("\"id\": \"GFX_goal_orphan\""));
    assert!(json.contains("\"unregistered_images_count\": 1"));
    assert!(json.contains("gfx/interface/goals/unregistered.dds"));

    let changed_output = root.join("gfx_changed.json");
    cmd_gfx_audit(&[
        root.to_string_lossy().to_string(),
        "--changed-only".to_string(),
        "--changed".to_string(),
        "common/national_focus/tst.txt".to_string(),
        "--output".to_string(),
        changed_output.to_string_lossy().to_string(),
    ])
    .unwrap();
    let changed_json = read_utf8_lossy(&changed_output).unwrap();
    assert!(changed_json.contains("\"missing_sprites_count\": 1"));
    assert!(changed_json.contains("\"missing_textures_count\": 0"));
    assert!(changed_json.contains("\"orphan_sprites_count\": 0"));
}

#[test]
fn logic_audit_reports_focus_graph_reference_issues() {
    let root = unique_temp_dir("logic-audit");
    fs::create_dir_all(root.join("common/national_focus")).unwrap();
    fs::create_dir_all(root.join("events")).unwrap();
    fs::write(
        root.join("common/national_focus/tst.txt"),
        r#"
focus_tree = {
  id = tst_focus
  country = { factor = 0 modifier = { add = 10 tag = TST } }
  focus = {
    id = TST_root
    x = 0
    y = 0
  }
  focus = {
    id = TST_child
    prerequisite = { focus = TST_missing }
    relative_position_id = TST_missing_position
    completion_reward = { country_event = { id = tst.missing } }
    x = 0
    y = 1
  }
  focus = {
    id = TST_left
    mutually_exclusive = { focus = TST_right }
    x = -1
    y = 0
  }
  focus = {
    id = TST_right
    x = 1
    y = 0
  }
}

focus_tree = {
  id = other_focus
  country = { factor = 0 modifier = { add = 10 tag = OTH } }
  focus = {
    id = OTH_root
    x = 0
    y = 0
  }
  focus = {
    id = OTH_cross
    prerequisite = { focus = TST_root }
    x = 0
    y = 1
  }
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("events/tst_events.txt"),
        r#"
add_namespace = tst
country_event = {
  id = tst.1
  title = tst.1.t
  desc = tst.1.d
  is_triggered_only = yes
  option = { name = tst.1.a }
}
news_event = {
  id = tst.2
  title = tst.2.t
  desc = tst.2.d
  is_triggered_only = yes
  option = { name = tst.2.a }
}
"#,
    )
    .unwrap();

    let output = root.join("logic.json");
    cmd_logic_audit(&[
        root.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();
    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.logic_audit.v1\""));
    assert!(json.contains("\"ok\": false"));
    assert!(json.contains("\"focus_total\": 6"));
    assert!(json.contains("\"focus_trees_total\": 2"));
    assert!(json.contains("\"broken_focus_refs_count\": 2"));
    assert!(json.contains("\"target\": \"TST_missing\""));
    assert!(json.contains("\"target\": \"TST_missing_position\""));
    assert!(json.contains("\"cross_tree_focus_refs_count\": 1"));
    assert!(json.contains("\"id\": \"OTH_cross\""));
    assert!(json.contains("\"asymmetric_mutual_exclusions_count\": 1"));
    assert!(json.contains("\"id\": \"TST_left\""));
    assert!(json.contains("\"unreachable_focuses_count\": 1"));
    assert!(json.contains("\"id\": \"TST_child\""));
    assert!(json.contains("\"event_total\": 2"));
    assert!(json.contains("\"event_refs_total\": 1"));
    assert!(json.contains("\"broken_event_refs_count\": 1"));
    assert!(json.contains("\"target\": \"tst.missing\""));
    assert!(json.contains("\"potential_orphan_events_count\": 2"));
    assert!(json.contains("\"id\": \"tst.1\""));

    let changed_output = root.join("logic_changed.json");
    cmd_logic_audit(&[
        root.to_string_lossy().to_string(),
        "--changed-only".to_string(),
        "--changed".to_string(),
        "common/national_focus/tst.txt".to_string(),
        "--output".to_string(),
        changed_output.to_string_lossy().to_string(),
    ])
    .unwrap();
    let changed_json = read_utf8_lossy(&changed_output).unwrap();
    assert!(changed_json.contains("\"changed_files\": [\"common/national_focus/tst.txt\"]"));
    assert!(changed_json.contains("\"broken_focus_refs_count\": 2"));

    let missing_changed = cmd_logic_audit(&[
        root.to_string_lossy().to_string(),
        "--changed-only".to_string(),
    ]);
    assert!(missing_changed.is_err());
    assert!(missing_changed
        .unwrap_err()
        .contains("--changed-only requires"));
}

fn write_minimal_focus_xlsx(path: &Path) {
    let file = fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();
    write_zip_xml(
        &mut zip,
        options,
        "[Content_Types].xml",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
</Types>"#,
    );
    write_zip_xml(
        &mut zip,
        options,
        "_rels/.rels",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#,
    );
    write_zip_xml(
        &mut zip,
        options,
        "xl/workbook.xml",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
    <sheet name="FocusTree" sheetId="1" r:id="rId1"/>
  </sheets>
</workbook>"#,
    );
    write_zip_xml(
        &mut zip,
        options,
        "xl/_rels/workbook.xml.rels",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
</Relationships>"#,
    );
    write_zip_xml(
        &mut zip,
        options,
        "xl/worksheets/sheet1.xml",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1">
      <c r="A1" t="inlineStr"><is><t>国策树</t></is></c>
      <c r="C1" t="inlineStr"><is><t>重建中央委员会|rebuild_committee</t></is></c>
    </row>
    <row r="2">
      <c r="C2" t="inlineStr"><is><t>│</t></is></c>
    </row>
    <row r="3">
      <c r="B3" t="inlineStr"><is><t xml:space="preserve">工业复兴&#10;ID: industrial_revival&#10;icon: GFX_goal_generic_construct_civ_factory&#10;completion_reward: 1个军工厂</t></is></c>
      <c r="D3" t="inlineStr"><is><t>整顿军队</t></is></c>
    </row>
  </sheetData>
</worksheet>"#,
    );
    zip.finish().unwrap();
}

fn write_drawing_title_focus_xlsx(path: &Path) {
    let file = fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();
    write_zip_xml(
        &mut zip,
        options,
        "[Content_Types].xml",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
  <Override PartName="/xl/drawings/drawing1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawing+xml"/>
</Types>"#,
    );
    write_zip_xml(
        &mut zip,
        options,
        "_rels/.rels",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#,
    );
    write_zip_xml(
        &mut zip,
        options,
        "xl/workbook.xml",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
    <sheet name="FocusTree" sheetId="1" r:id="rId1"/>
  </sheets>
</workbook>"#,
    );
    write_zip_xml(
        &mut zip,
        options,
        "xl/_rels/workbook.xml.rels",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
</Relationships>"#,
    );
    write_zip_xml(
        &mut zip,
        options,
        "xl/worksheets/sheet1.xml",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheetData>
    <row r="3">
      <c r="B3" t="inlineStr"><is><t>KOR_industry_revival</t></is></c>
    </row>
  </sheetData>
  <drawing r:id="rId1"/>
</worksheet>"#,
    );
    write_zip_xml(
        &mut zip,
        options,
        "xl/worksheets/_rels/sheet1.xml.rels",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/>
</Relationships>"#,
    );
    write_zip_xml(
        &mut zip,
        options,
        "xl/drawings/drawing1.xml",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <xdr:twoCellAnchor>
    <xdr:from><xdr:col>1</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>2</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>
    <xdr:to><xdr:col>2</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>3</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:to>
    <xdr:sp>
      <xdr:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>工业复兴</a:t></a:r></a:p></xdr:txBody>
    </xdr:sp>
    <xdr:clientData/>
  </xdr:twoCellAnchor>
</xdr:wsDr>"#,
    );
    zip.finish().unwrap();
}

fn write_zip_xml(
    zip: &mut zip::ZipWriter<fs::File>,
    options: zip::write::SimpleFileOptions,
    path: &str,
    xml: &str,
) {
    use std::io::Write;

    zip.start_file(path, options).unwrap();
    zip.write_all(xml.as_bytes()).unwrap();
}

fn write_fer_country_source(root: &Path) {
    fs::create_dir_all(root.join("common").join("country_tags")).unwrap();
    fs::create_dir_all(root.join("common").join("countries")).unwrap();
    fs::create_dir_all(root.join("localisation").join("simp_chinese")).unwrap();
    fs::write(
        root.join("common").join("country_tags").join("00_tags.txt"),
        "FER = \"countries/FER.txt\"\n",
    )
    .unwrap();
    fs::write(
        root.join("common").join("countries").join("FER.txt"),
        "graphical_culture = western_european_gfx\n",
    )
    .unwrap();
    let mut loc = vec![0xef, 0xbb, 0xbf];
    loc.extend_from_slice(
        "l_simp_chinese:\n FER:0 \"远东铁路共和国\"\n FER_DEF:0 \"远东铁路共和国\"\n FER_ADJ:0 \"远东\"\n".as_bytes(),
    );
    fs::write(
        root.join("localisation")
            .join("simp_chinese")
            .join("countries_l_simp_chinese.yml"),
        loc,
    )
    .unwrap();
}

fn write_country_knowledge_source(root: &Path, countries: &[(&str, &str, &str)]) {
    fs::create_dir_all(root.join("common").join("country_tags")).unwrap();
    fs::create_dir_all(root.join("common").join("countries")).unwrap();
    fs::create_dir_all(root.join("localisation").join("simp_chinese")).unwrap();
    fs::write(root.join("hoi4.exe"), "").unwrap();
    let mut tags = String::new();
    let mut loc = vec![0xef, 0xbb, 0xbf];
    loc.extend_from_slice(b"l_simp_chinese:\n");
    for (tag, file_name, name) in countries {
        tags.push_str(&format!("{tag} = \"countries/{file_name}\"\n"));
        fs::write(
            root.join("common").join("countries").join(file_name),
            "graphical_culture = western_european_gfx\n",
        )
        .unwrap();
        loc.extend_from_slice(format!(" {tag}:0 \"{name}\"\n").as_bytes());
    }
    fs::write(
        root.join("common")
            .join("country_tags")
            .join("00_countries.txt"),
        tags,
    )
    .unwrap();
    fs::write(
        root.join("localisation")
            .join("simp_chinese")
            .join("countries_l_simp_chinese.yml"),
        loc,
    )
    .unwrap();
}

#[test]
fn split_field_handles_chinese_colon() {
    let (key, value) = split_field("决议：鼓励奈普曼投资").unwrap();
    assert_eq!(key, "决议");
    assert_eq!(value, "鼓励奈普曼投资");
}

#[test]
fn feature_card_parser_accepts_chinese_colons() {
    let json = parse_decision_idea_cards_json(
        "决议：鼓励奈普曼投资\n目标：SOV\n效果：稳定度+2%",
        "SOV",
        "sov_nep",
    );

    assert!(json.contains("\"title\": \"鼓励奈普曼投资\""));
    assert!(json.contains("\"code\": \"add_stability = 0.02\""));
}

#[test]
fn detect_hoi4_path_accepts_explicit_game_root() {
    let root = unique_temp_dir("detect-hoi4-explicit");
    fs::create_dir_all(root.join("common")).unwrap();
    fs::create_dir_all(root.join("localisation")).unwrap();
    fs::write(root.join("hoi4.exe"), "").unwrap();

    let candidates = detect_hoi4_path_candidates(std::slice::from_ref(&root), &[]);
    let json = detect_hoi4_path_json(&candidates);
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(candidates.len(), 1);
    assert!(candidates[0].valid);
    assert!(json.contains("\"selected\""));
    assert!(json.contains("\"valid\": true"));
}

#[test]
fn detect_hoi4_path_reads_steam_libraryfolders() {
    let root = unique_temp_dir("detect-hoi4-steam");
    let steam = root.join("Steam");
    let library = root.join("SteamLibrary");
    let game = library
        .join("steamapps")
        .join("common")
        .join("Hearts of Iron IV");
    fs::create_dir_all(steam.join("steamapps")).unwrap();
    fs::create_dir_all(game.join("common")).unwrap();
    fs::create_dir_all(game.join("localisation")).unwrap();
    let escaped_library = library.display().to_string().replace('\\', "\\\\");
    fs::write(
        steam.join("steamapps").join("libraryfolders.vdf"),
        format!(
            "\"libraryfolders\"\n{{\n  \"1\"\n  {{\n    \"path\" \"{}\"\n  }}\n}}\n",
            escaped_library
        ),
    )
    .unwrap();

    let candidates = detect_hoi4_path_candidates(&[], std::slice::from_ref(&steam));
    let json = detect_hoi4_path_json(&candidates);
    fs::remove_dir_all(&root).unwrap();

    assert!(candidates
        .iter()
        .any(|candidate| candidate.path == game && candidate.valid));
    assert!(json.contains("Hearts of Iron IV"));
    assert!(json.contains("\"selected\""));
}

#[test]
fn arg_parser_keeps_repeated_mod_paths() {
    let args = vec![
        "--mod-path".to_string(),
        "A".to_string(),
        "--mod-path".to_string(),
        "B".to_string(),
    ];
    let map = parse_args(&args);

    assert_eq!(repeated_values(&map, "mod-path"), vec!["A", "B"]);
    assert_eq!(value(&map, "mod-path"), Some("B"));
}

#[test]
fn error_log_analyzer_extracts_file_line_and_categories() {
    let root = unique_temp_dir("error-log-analysis");
    fs::create_dir_all(root.join("common").join("national_focus")).unwrap();
    fs::create_dir_all(root.join("interface")).unwrap();
    fs::create_dir_all(root.join("events")).unwrap();
    fs::write(
        root.join("common")
            .join("national_focus")
            .join("bad_focus.txt"),
        "focus_tree = {}\n",
    )
    .unwrap();
    fs::write(root.join("interface").join("bad.gfx"), "").unwrap();
    fs::write(root.join("events").join("bad.txt"), "").unwrap();
    let text = r#"
[23:00:01][persistent.cpp:52]: Error: "Malformed token: =, near line: 12" in file: "common/national_focus/bad_focus.txt" near line: 12
[23:00:02][gfx_dx11.cpp:211]: Could not find spriteType "GFX_missing_icon" in file: "interface/bad.gfx" near line: 5
[23:00:03][eventmanager.cpp:99]: Unknown event namespace in events/bad.txt:42: sov_nep.1
[23:00:04][localisation.cpp:77]: Missing localisation key: sov_nep.1.t
"#;
    let diagnostics = analyze_error_log(text, Some(&root));
    let report = build_error_log_report(diagnostics.clone(), None, Vec::new(), false);
    let json = error_log_report_json(Path::new("M:\\logs\\error.log"), Some(&root), &report);
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(diagnostics.len(), 4);
    assert_eq!(diagnostics[0].category, "syntax");
    assert_eq!(
        diagnostics[0].file.as_deref(),
        Some("common/national_focus/bad_focus.txt")
    );
    assert_eq!(diagnostics[0].line, Some(12));
    assert!(diagnostics[0].resolved_file.is_some());
    assert_eq!(diagnostics[1].category, "gfx");
    assert_eq!(diagnostics[1].line, Some(5));
    assert_eq!(diagnostics[2].category, "event_namespace");
    assert_eq!(diagnostics[2].file.as_deref(), Some("events/bad.txt"));
    assert_eq!(diagnostics[2].line, Some(42));
    assert_eq!(diagnostics[3].category, "localisation");
    assert!(json.contains("\"by_category\": {"));
    assert!(json.contains("\"syntax\": 1"));
    assert!(json.contains("\"gfx\": 1"));
    assert!(json.contains("\"event_namespace\": 1"));
    assert!(json.contains("\"localisation\": 1"));
}

#[test]
fn error_log_analyzer_filters_baseline_and_changed_files() {
    let root = unique_temp_dir("error-log-baseline");
    fs::create_dir_all(root.join("events")).unwrap();
    fs::create_dir_all(root.join("interface")).unwrap();
    fs::write(root.join("events").join("new.txt"), "").unwrap();
    fs::write(root.join("interface").join("old.gfx"), "").unwrap();
    let baseline_log = root.join("baseline.log");
    fs::write(
        &baseline_log,
        r#"[23:00:01][gfx_dx11.cpp:211]: Could not find spriteType "GFX_old" in file: "interface/old.gfx" near line: 5
"#,
    )
    .unwrap();
    let current_log = root.join("error.log");
    fs::write(
        &current_log,
        r#"[23:00:01][gfx_dx11.cpp:211]: Could not find spriteType "GFX_old" in file: "interface/old.gfx" near line: 5
[23:00:02][eventmanager.cpp:99]: Unknown event namespace in events/new.txt:42: tst.1
"#,
    )
    .unwrap();
    let output = root.join("report.json");

    cmd_analyze_error_log(&[
        "--input".to_string(),
        current_log.to_string_lossy().to_string(),
        "--mod-root".to_string(),
        root.to_string_lossy().to_string(),
        "--baseline".to_string(),
        baseline_log.to_string_lossy().to_string(),
        "--changed-only".to_string(),
        "--changed".to_string(),
        "events/new.txt".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    assert!(json.contains("\"schema\": \"hoi4skill.error_log_report.v1\""));
    assert!(json.contains("\"diagnostics_total\": 2"));
    assert!(json.contains("\"diagnostics_effective\": 1"));
    assert!(json.contains("\"baseline_filtered\": 1"));
    assert!(json.contains("\"changed_files\": [\"events/new.txt\"]"));
    assert!(json.contains("\"category\": \"event_namespace\""));
    assert!(json.contains("events/new.txt"));
    assert!(!json.contains("\"GFX_old\""));

    let changed_without_file = cmd_analyze_error_log(&[
        "--input".to_string(),
        current_log.to_string_lossy().to_string(),
        "--changed-only".to_string(),
    ]);
    assert!(changed_without_file.is_err());
    assert!(changed_without_file
        .unwrap_err()
        .contains("--changed-only requires"));
}

#[test]
fn game_index_merges_dependency_mod_roots() {
    let root = unique_temp_dir("game-index-dependencies");
    let game = root.join("game");
    let dep = root.join("dep_mod");
    fs::create_dir_all(game.join("common").join("country_tags")).unwrap();
    fs::create_dir_all(game.join("interface")).unwrap();
    fs::create_dir_all(dep.join("common").join("country_tags")).unwrap();
    fs::create_dir_all(dep.join("interface")).unwrap();
    fs::write(
        game.join("common").join("country_tags").join("00_tags.txt"),
        "AAA = \"countries/AAA.txt\"\n",
    )
    .unwrap();
    fs::write(
        game.join("interface").join("game.gfx"),
        r#"spriteType = { name = "GFX_game_icon" texturefile = "gfx/interface/game.png" }"#,
    )
    .unwrap();
    fs::write(
        dep.join("common")
            .join("country_tags")
            .join("00_dep_tags.txt"),
        "DEP = \"countries/DEP.txt\"\n",
    )
    .unwrap();
    fs::write(
        dep.join("interface").join("dep.gfx"),
        r#"spriteType = { name = "GFX_dep_icon" texturefile = "gfx/interface/dep.png" }"#,
    )
    .unwrap();
    fs::write(
        root.join("dep.mod"),
        format!(
            "path=\"{}\"\n",
            dep.display().to_string().replace('\\', "/")
        ),
    )
    .unwrap();
    let map = parse_args(&[
        "--mod-path".to_string(),
        root.join("dep.mod").display().to_string(),
    ]);
    let deps = dependency_mod_roots(&map).unwrap();
    let index = build_game_index_with_mod_paths(&game, &deps).unwrap();
    let json = game_index_json(&index);
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(deps, vec![dep]);
    assert_eq!(index.indexed_roots.len(), 2);
    assert!(index.country_tags.contains("AAA"));
    assert!(index.country_tags.contains("DEP"));
    assert!(index.sprites.contains("GFX_game_icon"));
    assert!(index.sprites.contains("GFX_dep_icon"));
    assert!(json.contains("\"indexed_roots\""));
    assert!(json.contains("\"GFX_dep_icon\""));
}

#[test]
fn steam_vdf_parser_accepts_old_numeric_library_path() {
    let paths = parse_steam_libraryfolders_vdf("\"1\" \"D:\\\\SteamLibrary\"\n");

    assert_eq!(paths, vec![PathBuf::from(r"D:\SteamLibrary")]);
}

#[test]
fn steam_vdf_parser_ignores_numeric_app_entries() {
    let paths = parse_steam_libraryfolders_vdf("\"1030331990\" \"88826926247\"\n");

    assert!(paths.is_empty());
}

#[test]
fn scan_mod_style_collects_core_style_signals() {
    let workspace = unique_temp_dir("scan-mod-style");
    let root = workspace.join("sample_mod");
    fs::create_dir_all(root.join("common").join("national_focus")).unwrap();
    fs::create_dir_all(root.join("common").join("ideas")).unwrap();
    fs::create_dir_all(root.join("common").join("decisions").join("categories")).unwrap();
    fs::create_dir_all(root.join("common").join("country_tags")).unwrap();
    fs::create_dir_all(root.join("events")).unwrap();
    fs::create_dir_all(root.join("interface")).unwrap();
    fs::create_dir_all(root.join("gfx").join("interface")).unwrap();
    fs::create_dir_all(root.join("history").join("countries")).unwrap();
    fs::create_dir_all(root.join("localisation").join("simp_chinese")).unwrap();
    fs::write(
        root.join("descriptor.mod"),
        "name=\"Sample Mod\"\nversion=\"0.1\"\nsupported_version=\"*\"\nremote_file_id=\"123\"\ndependencies={ \"Kaiserredux\" }\ntags={ \"Alternative History\" }\n",
    )
    .unwrap();
    fs::write(
        workspace.join("sample.mod"),
        format!(
            "path=\"{}\"\n",
            root.display().to_string().replace('\\', "/")
        ),
    )
    .unwrap();
    fs::write(
        root.join("common")
            .join("national_focus")
            .join("cpc_focus.txt"),
        "focus_tree = { id = cpc_focus country = { factor = 0 modifier = { tag = CPC } } focus = { id = CPC_new_order icon = GFX_goal_test x = 0 y = 0 } }\n",
    )
    .unwrap();
    fs::write(
        root.join("events").join("cpc.txt"),
        "add_namespace = cpc\ncountry_event = { id = cpc.17 title = cpc.17.t desc = cpc.17.d is_triggered_only = yes option = { name = cpc.17.a } }\nnews_event = { id = cpc.18 title = cpc.18.t desc = cpc.18.d is_triggered_only = yes option = { name = cpc.18.a } }\n",
    )
    .unwrap();
    fs::write(
        root.join("interface").join("sample.gfx"),
        "spriteType = { name = \"GFX_goal_test\" texturefile = \"gfx/interface/中文图标.png\" }\n",
    )
    .unwrap();
    fs::write(root.join("gfx").join("interface").join("中文图标.png"), "").unwrap();
    fs::write(
        root.join("common").join("ideas").join("cpc_ideas.txt"),
        "ideas = { country = { CPC_sample_idea = { picture = GFX_idea_sample modifier = { stability_factor = 0.05 } } } }\n",
    )
    .unwrap();
    fs::write(
        root.join("common")
            .join("decisions")
            .join("categories")
            .join("cpc.txt"),
        "cpc_category = { icon = GFX_decision_test }\n",
    )
    .unwrap();
    fs::write(
        root.join("common").join("country_tags").join("00_tags.txt"),
        "CPC = \"countries/CPC.txt\"\n",
    )
    .unwrap();
    fs::write(
        root.join("history")
            .join("countries")
            .join("CPC - Sample.txt"),
        "capital = 123\n",
    )
    .unwrap();
    let mut loc = vec![0xef, 0xbb, 0xbf];
    loc.extend_from_slice(
        "l_simp_chinese:\n CPC_new_order:0 \"新秩序\"\n loose_key: \"宽松写法\"\n".as_bytes(),
    );
    fs::write(
        root.join("localisation")
            .join("simp_chinese")
            .join("sample_l_simp_chinese.yml"),
        loc,
    )
    .unwrap();

    let resolved = resolve_mod_root(&workspace.join("sample.mod")).unwrap();
    let json = scan_mod_style_json(
        &resolved,
        &ModStyleScanOptions {
            max_sprites: 10,
            max_non_ascii_paths: 10,
        },
    )
    .unwrap();
    fs::remove_dir_all(&workspace).unwrap();

    assert!(json.contains("\"input_kind\": \"launcher\""));
    assert!(json.contains("\"name\": \"Sample Mod\""));
    assert!(json.contains("\"dependencies\": [\"Kaiserredux\"]"));
    assert!(json.contains("\"tree_id\": \"cpc_focus\""));
    assert!(json.contains("\"country_tag\": \"CPC\""));
    assert!(json.contains("\"CPC_\": 1"));
    assert!(json.contains("\"cpc\": {\"max_id\": 18"));
    assert!(json.contains("\"l_simp_chinese:\""));
    assert!(json.contains("\"bom\": true"));
    assert!(json.contains("\"GFX_goal_test\": \"gfx/interface/中文图标.png\""));
    assert!(json.contains("\"cpc_category\""));
    assert!(json.contains("\"country_tags\": [\"CPC\"]"));
    assert!(json.contains("中文图标.png"));
}

#[test]
fn mod_knowledge_classifies_submod_and_summarizes_local_facts() {
    let workspace = unique_temp_dir("mod-knowledge-submod");
    let root = workspace.join("sample_mod");
    let dep = workspace.join("dependency_mod");
    fs::create_dir_all(root.join("common").join("national_focus")).unwrap();
    fs::create_dir_all(root.join("common").join("country_tags")).unwrap();
    fs::create_dir_all(root.join("common").join("countries")).unwrap();
    fs::create_dir_all(root.join("common").join("country_leader")).unwrap();
    fs::create_dir_all(root.join("common").join("characters")).unwrap();
    fs::create_dir_all(root.join("events")).unwrap();
    fs::create_dir_all(root.join("history").join("countries")).unwrap();
    fs::create_dir_all(root.join("history").join("states")).unwrap();
    fs::create_dir_all(root.join("interface")).unwrap();
    fs::create_dir_all(root.join("localisation").join("simp_chinese")).unwrap();
    fs::create_dir_all(root.join("map")).unwrap();
    fs::create_dir_all(dep.join("common").join("country_leader")).unwrap();
    fs::create_dir_all(dep.join("history").join("countries")).unwrap();
    fs::write(
        root.join("descriptor.mod"),
        "name=\"Sample Submod\"\nsupported_version=\"1.16.*\"\ndependencies={ \"Kaiserredux\" }\n",
    )
    .unwrap();
    fs::write(
        workspace.join("sample.mod"),
        format!(
            "name=\"Sample Submod\"\npath=\"{}\"\n",
            root.display().to_string().replace('\\', "/")
        ),
    )
    .unwrap();
    fs::write(
        root.join("common")
            .join("national_focus")
            .join("cpc_focus.txt"),
        "focus_tree = { id = cpc_focus country = { factor = 0 modifier = { tag = CPC } } focus = { id = CPC_new_order icon = GFX_goal_test x = 0 y = 0 } }\n",
    )
    .unwrap();
    fs::write(
        root.join("events").join("cpc.txt"),
        "add_namespace = cpc\ncountry_event = { id = cpc.17 title = cpc.17.t desc = cpc.17.d option = { name = cpc.17.a } }\n",
    )
    .unwrap();
    fs::write(
        root.join("interface").join("sample.gfx"),
        "spriteType = { name = \"GFX_goal_test\" texturefile = \"gfx/interface/test.png\" }\n",
    )
    .unwrap();
    fs::write(
        root.join("common").join("country_tags").join("00_tags.txt"),
        "CPC = \"countries/CPC.txt\"\n",
    )
    .unwrap();
    fs::write(
        root.join("common").join("countries").join("CPC.txt"),
        "graphical_culture = asian_gfx\ngraphical_culture_2d = asian_2d\ncolor = { 120 20 20 }\n",
    )
    .unwrap();
    fs::write(
        root.join("common")
            .join("country_leader")
            .join("cpc_traits.txt"),
        "leader_traits = { CPC_old_guard_trait = { random = no stability_factor = 0.05 } }\n",
    )
    .unwrap();
    fs::write(
        root.join("common").join("characters").join("CPC.txt"),
        "characters = { CPC_test_leader = { name = CPC_test_leader country_leader = { ideology = marxism traits = { CPC_old_guard_trait } id = -1 } } }\n",
    )
    .unwrap();
    fs::write(
        root.join("history")
            .join("countries")
            .join("CPC - Sample.txt"),
        "capital = 123\nrecruit_character = CPC_test_leader\ncreate_country_leader = { name = \"Legacy Leader\" picture = \"legacy.dds\" ideology = marxism traits = { CPC_old_guard_trait } }\n",
    )
    .unwrap();
    fs::write(
        root.join("history").join("states").join("64-Test.txt"),
        "state = { id = 64 name = \"STATE_64\" manpower = 1000 state_category = town resources = { steel = 8 aluminium = 2 } history = { owner = CPC controller = CPC add_core_of = CPC victory_points = { 123 5 } buildings = { infrastructure = 3 arms_factory = 1 123 = { naval_base = 1 } } } provinces = { 123 456 } }\n",
    )
    .unwrap();
    fs::write(
        root.join("map").join("definition.csv"),
        "123;1;2;3;land;false;plains;1\n456;4;5;6;sea;false;ocean;0\n789;7;8;9;lake;false;lakes;0\n",
    )
    .unwrap();
    fs::write(
        dep.join("common")
            .join("country_leader")
            .join("dep_traits.txt"),
        "leader_traits = { dep_legacy_trait = { random = no political_power_factor = 0.1 } }\n",
    )
    .unwrap();
    fs::write(
        dep.join("history")
            .join("countries")
            .join("DEP - Dependency.txt"),
        "create_country_leader = { name = \"Dependency Leader\" ideology = conservatism traits = { dep_legacy_trait } }\n",
    )
    .unwrap();
    fs::write(
        root.join("localisation")
            .join("simp_chinese")
            .join("sample_l_simp_chinese.yml"),
        "l_simp_chinese:\n CPC_new_order:0 \"新秩序\"\n cpc.17.t:0 \"测试事件\"\n cpc.17.d:0 \"事件描述。\"\n cpc.17.a:0 \"继续\"\n",
    )
    .unwrap();

    let resolved = resolve_mod_root(&workspace.join("sample.mod")).unwrap();
    let json = mod_knowledge_json(&resolved, 10, 10, std::slice::from_ref(&dep)).unwrap();
    fs::remove_dir_all(&workspace).unwrap();

    assert!(json.contains("\"schema\": \"hoi4skill.mod_knowledge.v1\""));
    assert!(json.contains("\"kind\": \"submod\""));
    assert!(json.contains("\"dependency_names\": [\"Kaiserredux\"]"));
    assert!(json.contains("\"dependency_mod_roots\""));
    assert!(json.contains("\"tree_id\": \"cpc_focus\""));
    assert!(json.contains("\"country_tags\": [\"CPC\"]"));
    assert!(json.contains("\"cpc\": {\"max_id\": 17"));
    assert!(json.contains("\"GFX_goal_test\""));
    assert!(json.contains("\"title\": \"新秩序\""));
    assert!(json.contains("Determine mod_kind before editing"));
    assert!(json.contains("unknown instead of inventing it"));
    assert!(json.contains("\"country_tag_mappings\""));
    assert!(json.contains("\"country_file\": \"countries/CPC.txt\""));
    assert!(json.contains("\"country_definition_files\": [\"common/countries/CPC.txt\"]"));
    assert!(json.contains("\"country_leader_traits\": [\"CPC_old_guard_trait\"]"));
    assert!(json.contains("\"id\": \"CPC_test_leader\""));
    assert!(json.contains("\"has_country_leader\": true"));
    assert!(json.contains("\"recruited_characters\": [\"CPC_test_leader\"]"));
    assert!(json.contains("\"name\": \"Legacy Leader\""));
    assert!(json.contains("legacy_history_create_country_leader"));
    assert!(json.contains("follow the indexed dependency mod's observed syntax"));
    assert!(json.contains("\"history_state_files\": [\"history/states/64-Test.txt\"]"));
    assert!(json.contains("\"history_states\""));
    assert!(json.contains("\"id\": 64"));
    assert!(json.contains("\"name\": \"STATE_64\""));
    assert!(json.contains("\"owner\": \"CPC\""));
    assert!(json.contains("\"controller\": \"CPC\""));
    assert!(json.contains("\"cores\": [\"CPC\"]"));
    assert!(json.contains("\"province_count\": 2"));
    assert!(json.contains("\"province_sample\": [123, 456]"));
    assert!(json.contains("\"victory_point_provinces\": [123]"));
    assert!(json.contains("\"buildings\": [\"arms_factory\", \"infrastructure\", \"naval_base\"]"));
    assert!(json.contains("\"resources\": [\"aluminium\", \"steel\"]"));
    assert!(json.contains("\"province_definitions\""));
    assert!(json.contains("\"sample_ids\": [123, 456, 789]"));
    assert!(json.contains("\"land_count\": 1"));
    assert!(json.contains("\"sea_count\": 1"));
    assert!(json.contains("\"lake_count\": 1"));
    assert!(json.contains("capital uses province ID"));
    assert!(json.contains("index the dependency/game root before using state or province IDs"));
}

#[test]
fn mod_knowledge_classifies_standalone_when_no_dependencies_exist() {
    let workspace = unique_temp_dir("mod-knowledge-standalone");
    let root = workspace.join("standalone_mod");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("descriptor.mod"),
        "name=\"Standalone Mod\"\nsupported_version=\"1.16.*\"\n",
    )
    .unwrap();
    fs::write(
        workspace.join("standalone.mod"),
        format!(
            "name=\"Standalone Mod\"\npath=\"{}\"\n",
            root.display().to_string().replace('\\', "/")
        ),
    )
    .unwrap();

    let resolved = resolve_mod_root(&workspace.join("standalone.mod")).unwrap();
    let json = mod_knowledge_json(&resolved, 10, 10, &[]).unwrap();
    fs::remove_dir_all(&workspace).unwrap();

    assert!(json.contains("\"kind\": \"standalone_mod\""));
    assert!(json.contains("\"confidence\": \"medium\""));
    assert!(json.contains("\"dependency_names\": []"));
    assert!(json.contains("descriptor.mod exists and no dependencies were declared"));
}

#[test]
fn import_mod_ir_extracts_core_content() {
    let workspace = unique_temp_dir("import-mod-ir");
    let root = workspace.join("sample_mod");
    fs::create_dir_all(root.join("common").join("national_focus")).unwrap();
    fs::create_dir_all(root.join("common").join("ideas")).unwrap();
    fs::create_dir_all(root.join("common").join("decisions").join("categories")).unwrap();
    fs::create_dir_all(root.join("events")).unwrap();
    fs::create_dir_all(root.join("localisation").join("simp_chinese")).unwrap();
    fs::write(
        workspace.join("sample.mod"),
        format!(
            "path=\"{}\"\n",
            root.display().to_string().replace('\\', "/")
        ),
    )
    .unwrap();
    fs::write(
        root.join("common")
            .join("national_focus")
            .join("test_focus.txt"),
        "focus_tree = { id = test_tree country = { factor = 0 modifier = { tag = TST } } focus = { id = TST_parent icon = GFX_goal_generic_construct_civ_factory x = 0 y = 0 } focus = { id = TST_child icon = GFX_goal_generic_political_pressure x = 1 y = 1 cost = 10 prerequisite = { focus = TST_parent } mutually_exclusive = { focus = TST_rival } } }\n",
    )
    .unwrap();
    fs::write(
        root.join("events").join("test_events.txt"),
        "add_namespace = tst\ncountry_event = { id = tst.1 title = tst.1.t desc = tst.1.d picture = GFX_report_event_generic option = { name = tst.1.a } }\n",
    )
    .unwrap();
    fs::write(
        root.join("common").join("ideas").join("test_ideas.txt"),
        "ideas = { country = { TST_sample_idea = { picture = GFX_idea_test modifier = { stability_factor = 0.05 } } } }\n",
    )
    .unwrap();
    fs::write(
        root.join("common")
            .join("decisions")
            .join("categories")
            .join("test_categories.txt"),
        "TST_category = { icon = GFX_decision_generic_political_reform }\n",
    )
    .unwrap();
    fs::write(
        root.join("common")
            .join("decisions")
            .join("test_decisions.txt"),
        "TST_category = { TST_decision = { icon = generic_political_discourse cost = 25 complete_effect = { add_political_power = 10 } } }\n",
    )
    .unwrap();
    fs::write(
        root.join("localisation")
            .join("simp_chinese")
            .join("test_l_simp_chinese.yml"),
        "l_simp_chinese:\n TST_parent:0 \"父国策\"\n TST_child:0 \"子国策\"\n TST_child_desc:0 \"沿着父国策继续推进。\"\n tst.1.t:0 \"测试事件\"\n tst.1.d:0 \"事件描述。\"\n tst.1.a:0 \"很好\"\n TST_sample_idea:0 \"测试民族精神\"\n TST_sample_idea_desc:0 \"民族精神描述。\"\n TST_category:0 \"测试决议组\"\n TST_decision:0 \"测试决议\"\n TST_decision_desc:0 \"决议描述。\"\n",
    )
    .unwrap();

    let resolved = resolve_mod_root(&workspace.join("sample.mod")).unwrap();
    let json = import_mod_ir_json(&resolved, 50).unwrap();
    fs::remove_dir_all(&workspace).unwrap();

    assert!(json.contains("\"schema\": \"hoi4skill.imported_mod_ir.v1\""));
    assert!(json.contains("\"type\": \"focus\""));
    assert!(json.contains("\"tree_id\": \"test_tree\""));
    assert!(json.contains("\"country_tag\": \"TST\""));
    assert!(json.contains("\"id\": \"TST_child\""));
    assert!(json.contains("\"title\": \"子国策\""));
    assert!(json.contains("\"desc\": \"沿着父国策继续推进。\""));
    assert!(json.contains("\"prerequisites\": [\"TST_parent\"]"));
    assert!(json.contains("\"mutually_exclusive\": [\"TST_rival\"]"));
    assert!(json.contains("\"type\": \"event\""));
    assert!(json.contains("\"namespace\": \"tst\""));
    assert!(json.contains("\"title\": \"测试事件\""));
    assert!(json.contains("\"name\": \"很好\""));
    assert!(json.contains("\"type\": \"idea\""));
    assert!(json.contains("\"id\": \"TST_sample_idea\""));
    assert!(json.contains("\"type\": \"decision_category\""));
    assert!(json.contains("\"id\": \"TST_category\""));
    assert!(json.contains("\"type\": \"decision\""));
    assert!(json.contains("\"id\": \"TST_decision\""));
    assert!(json.contains("\"cost\": 25"));
    assert!(json.contains("\"focuses_total\": 2"));
    assert!(json.contains("\"events_total\": 1"));
    assert!(json.contains("\"ideas_total\": 1"));
    assert!(json.contains("\"decision_categories_total\": 1"));
    assert!(json.contains("\"decisions_total\": 1"));
}

#[test]
fn workflow_dry_run_detects_mixed_copy() {
    let text = "国策树：\n斯大林宪法\n第一个五年计划   互斥   继续新经济政策\n\n决议：鼓励奈普曼投资\n目标：SOV\n效果：政治点+25\n\n民族精神：新经济政策复兴\n效果：稳定度+5%\n\n事件：新经济政策的未来\n类型：国家事件\n命名空间：sov_nep\n标题：新经济政策的未来\n描述：党内围绕新经济政策展开了激烈争论。\n选项A：继续试验\n效果A：政治点+50\n";
    let json = run_workflow_json(text, None, "SOV", "sov_nep", None, true, None).unwrap();

    assert!(json.contains("\"schema\": \"hoi4skill.copy_to_code_workflow.v1\""));
    assert!(json.contains("\"dry_run\": true"));
    assert!(json.contains("\"focus_layout\": true"));
    assert!(json.contains("\"feature_cards\": 2"));
    assert!(json.contains("\"event_cards\": 1"));
    assert!(json.contains("\"tree_id\": \"sov_nep_SOV_focus_tree\""));
    assert!(json.contains("\"type\": \"decision\""));
    assert!(json.contains("\"type\": \"idea\""));
    assert!(json.contains("\"type\": \"event\""));
    assert!(json.contains("--mod-root"));
}

#[test]
fn workflow_dry_run_reports_blocked_plan_safety() {
    let text = "国策树：\n整训舰队\n# completion_reward: 获得民族精神 舰队整训\n\n决议：未知动员\n目标：SOV\n效果：外星能量+50\n\n事件：未知局势\n命名空间：sov_ai\n触发：神秘局势\n选项A：继续\n效果A：政治点+50\n";
    let json = run_workflow_json(text, None, "SOV", "sov_ai", None, true, None).unwrap();

    assert!(json.contains("\"safety\": {\"status\": \"blocked\""));
    assert!(json.contains("\"final_code_allowed\": false"));
    assert!(json.contains("focus_layout: focus `整训舰队` completion_reward"));
    assert!(json.contains("feature_cards: raw_effect `外星能量+50` must be mapped"));
    assert!(json.contains("event_cards: raw_trigger `神秘局势` must be mapped"));
    assert!(json.contains("先解决 safety.blockers"));
}

#[test]
fn focus_layout_extraction_ignores_ai_markdown_preface_and_fences() {
    let text = "国策树：\n下面是按你的要求输出的国策树代码：\n```txt\n陈独秀回到中国共产党 | chen_duxiu_returns_to_the_ccp\n重读新青年 | reread_new_youth    党内复议 | party_reconsideration\n# completion_reward: 政治点+50\n```\n说明：以上只是布局，不要把说明写进游戏。\n";
    let json = run_workflow_json(text, None, "PRC", "prc_chen_duxiu", None, true, None).unwrap();

    assert!(json.contains("\"id\": \"PRC_chen_duxiu_returns_to_the_ccp\""));
    assert!(json.contains("\"id\": \"PRC_reread_new_youth\""));
    assert!(json.contains("\"id\": \"PRC_party_reconsideration\""));
    assert!(json.contains("\"completion_reward\": [\"add_political_power = 50\"]"));
    assert!(!json.contains("PRC_ai"));
    assert!(!json.contains("PRC_txt"));
    assert!(!json.contains("PRC_note"));
    assert!(!json.contains("PRC_above"));
}

#[test]
fn render_focus_code_uses_fixed_tree_and_focus_templates() {
    let root = unique_temp_dir("render-focus-code");
    fs::create_dir_all(&root).unwrap();
    let input = root.join("layout.txt");
    fs::write(
        &input,
        "朝鲜民族起义 | people_uprising\n联络游击队 | contact_guerrillas   动员市民 | mobilize_citizens\n",
    )
    .unwrap();
    let args = vec![
        "--input".to_string(),
        input.display().to_string(),
        "--tag".to_string(),
        "KOR".to_string(),
        "--prefix".to_string(),
        "kor_spring".to_string(),
    ];

    let map = parse_args(&args);
    let text =
        read_utf8_lossy(&normalize_path(&require_value(&map, "input").unwrap()).unwrap()).unwrap();
    let layout = parse_focus_layout_with_rewards(&text, "KOR", "kor_spring");
    let code = render_focus_tree(&layout, "KOR");
    fs::remove_dir_all(&root).unwrap();

    assert!(code.contains("country = {\n\t\tfactor = 0"));
    assert!(code.contains("modifier = {\n\t\t\tadd = 10\n\t\t\ttag = KOR"));
    assert!(!code.contains("default_focus"));
    assert!(!code.contains("country = KOR"));
    assert!(code.contains("id = KOR_people_uprising"));
    assert!(code.contains("cancel_if_invalid = yes"));
    assert!(code.contains("completion_reward = {"));
}

#[test]
fn requirement_scope_keeps_korean_revolution_prompt_narrow() {
    let text = "依据钢铁雄心4技能去按照韩国之春.xlsx生成一个韩国革命的mod，事件不少于4个，民族精神不少于5个（不准用python），游戏时间是1936，是反抗日本的起义以后的国策";
    let scope = requirement_scope_contract(text, true, "KOR", "kor_spring");
    let json = requirement_scope_contract_json(&scope);

    assert_eq!(scope.minimum_events, Some(4));
    assert_eq!(scope.minimum_ideas, Some(5));
    assert!(scope
        .authorized_systems
        .contains(&"national_focus".to_string()));
    assert!(scope.authorized_systems.contains(&"events".to_string()));
    assert!(scope
        .authorized_systems
        .contains(&"national_spirits".to_string()));
    assert!(scope
        .planned_files
        .contains(&"common/national_focus/kor_spring_focus.txt".to_string()));
    assert!(scope
        .planned_files
        .contains(&"common/ideas/kor_spring_ideas.txt".to_string()));
    assert!(scope
        .planned_files
        .contains(&"events/kor_spring_events.txt".to_string()));
    assert!(scope
        .forbidden_without_explicit_request
        .iter()
        .any(|path| path.contains("common/country_tags")));
    assert!(scope
        .forbidden_without_explicit_request
        .contains(&"history/countries".to_string()));
    assert!(scope
        .forbidden_without_explicit_request
        .contains(&"history/units".to_string()));
    assert!(scope
        .forbidden_without_explicit_request
        .contains(&"common/characters".to_string()));
    assert!(scope
        .forbidden_without_explicit_request
        .contains(&"localisation/english".to_string()));
    assert!(json.contains("\"events\": 4"));
    assert!(json.contains("\"national_spirits\": 5"));
    assert!(scope
        .rules
        .iter()
        .any(|rule| rule.contains("must submit structured focus, decision, event")));
    assert!(scope.rules.iter().any(|rule| {
        rule.contains("General requests such as create a mod")
            && rule.contains("explicit request to handwrite")
    }));
}

#[test]
fn explicit_request_is_combined_with_structured_workbook_input() {
    let layout = parse_focus_layout(
        "朝鲜民族起义\n联络游击队   动员市民   工人起义\n",
        "KOR",
        "kor_spring",
    );
    let mut input = WorkflowInput {
        text: "# Worksheet: Sheet1\n".to_string(),
        focus_layout: Some(layout),
    };
    append_explicit_request(
        &mut input,
        Some("事件不少于4个，民族精神不少于5个，是反抗日本的起义以后的国策"),
    );

    let json = run_workflow_json_with_focus_layout(
        &input.text,
        input.focus_layout.as_ref(),
        None,
        "KOR",
        "kor_spring",
        None,
        true,
        None,
    )
    .unwrap();

    assert!(input.text.contains("# Explicit User Requirement Contract"));
    assert!(json.contains("\"focus_layout\": true"));
    assert!(json.contains("\"events\": 4"));
    assert!(json.contains("\"national_spirits\": 5"));
    assert!(json.contains("history/countries"));
    assert!(json.contains("localisation/english"));
}

#[test]
fn explicit_request_does_not_become_focus_layout_content() {
    let mut input = WorkflowInput {
        text: "陈独秀回到中国共产党 | chen_duxiu_returns_to_the_ccp\n重读新青年 | reread_new_youth    党内复议 | party_reconsideration\n".to_string(),
        focus_layout: None,
    };
    append_explicit_request(&mut input, Some("陈独秀回到中国共产党国策树"));

    let json = run_workflow_json_with_focus_layout(
        &input.text,
        input.focus_layout.as_ref(),
        None,
        "PRC",
        "prc_chen_duxiu",
        None,
        true,
        None,
    )
    .unwrap();

    assert!(json.contains("\"id\": \"PRC_chen_duxiu_returns_to_the_ccp\""));
    assert!(!json.contains("PRC_country_party_country"));
}

#[test]
fn prepare_edit_context_packages_model_preflight_context() {
    let root = unique_temp_dir("edit-context");
    let library = unique_temp_dir("edit-context-library");
    fs::create_dir_all(root.join("common").join("national_focus")).unwrap();
    fs::create_dir_all(root.join("localisation").join("simp_chinese")).unwrap();
    fs::write(
        root.join("descriptor.mod"),
        "name=\"Context Test\"\nsupported_version=\"*\"\n",
    )
    .unwrap();
    fs::write(
        root.join("common")
            .join("national_focus")
            .join("sov_focus.txt"),
        "focus_tree = {\n\tid = sov_focus\n\tcountry = { factor = 0 modifier = { add = 10 tag = SOV } }\n\tfocus = {\n\t\tid = SOV_existing\n\t\ticon = GFX_goal_unknown\n\t\tx = 0\n\t\ty = 0\n\t\tcost = 10\n\t\tai_will_do = { factor = 100 }\n\t\tavailable = { }\n\t\tbypass = { }\n\t\tcancel_if_invalid = yes\n\t\tcontinue_if_invalid = no\n\t\tavailable_if_capitulated = no\n\t\tcompletion_reward = { }\n\t}\n}\n",
    )
    .unwrap();
    let mut loc = vec![0xef, 0xbb, 0xbf];
    loc.extend_from_slice(
        "l_simp_chinese:\n SOV_existing:0 \"既有国策\"\n SOV_existing_desc:0 \"既有描述。\"\n"
            .as_bytes(),
    );
    fs::write(
        root.join("localisation")
            .join("simp_chinese")
            .join("SOV_l_simp_chinese.yml"),
        loc,
    )
    .unwrap();
    let input = root.join("copy.txt");
    fs::write(
        &input,
        "国策树：\n工业复兴\n\n州效果：整顿首都工业\n州ID：64\n效果：1个军工厂\n图标：工业图标\n科技：新式步兵\n",
    )
    .unwrap();
    build_clausewitz_library(std::slice::from_ref(&root), &library).unwrap();

    let context = prepare_edit_context_markdown(
        &input,
        &root,
        "SOV",
        "sov_ctx",
        None,
        None,
        None,
        &[],
        None,
        20,
        20,
        10,
        Some(std::slice::from_ref(&library)),
    )
    .unwrap();
    fs::remove_dir_all(&root).unwrap();
    fs::remove_dir_all(&library).unwrap();

    assert!(context.contains("# HOI4 Edit Context Pack"));
    assert!(context.contains("## Write Gate"));
    assert!(context.contains("- status: `VERIFY_FIRST`"));
    assert!(context.contains("### Verified Evidence"));
    assert!(context.contains("request parsed as focus_layout=true, feature_cards=2, event_cards=0"));
    assert!(context.contains("### Allowed Edit Surface"));
    assert!(context.contains("common/national_focus and localisation/simp_chinese"));
    assert!(context.contains("common/technologies and localisation"));
    assert!(context.contains("common/scripted_effects state-scope helpers only"));
    assert!(context.contains("### Missing Evidence To Resolve"));
    assert!(context.contains("### Verification Steps"));
    assert!(context.contains("plan-history-edit"));
    assert!(context.contains("### Stop Conditions"));
    assert!(context.contains("## Knowledge Summary"));
    assert!(context.contains("## Retrieved Clausewitz Code Library"));
    assert!(context.contains("clausewitz_code_layers"));
    assert!(context.contains("vanilla_base"));
    assert!(context.contains("focus `SOV_existing`"));
    assert!(context.contains("## Dry Run Plan"));
    assert!(context.contains("## Unknown Facts"));
    assert!(context.contains("## Blocked Until Verified"));
    assert!(context.contains("history/state/province/capital facts require"));
    assert!(context.contains("Do not edit `history/states`"));
    assert!(context.contains("game/dependency icon index was not built"));
    assert!(context.contains("technology, category, equipment"));
    assert!(context.contains("common/national_focus/sov_focus.txt"));
    assert!(context.contains("\"focus_layout\": true"));
}

#[test]
fn prepare_edit_context_embeds_strict_authoring_contract_and_safety_blockers() {
    let root = unique_temp_dir("edit-context-strict-ai-contract");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("descriptor.mod"),
        "name=\"Strict Context Test\"\nsupported_version=\"*\"\n",
    )
    .unwrap();
    let input = root.join("copy.txt");
    fs::write(&input, "决议：政治动员\n目标：SOV\n效果：政治点+50\n").unwrap();
    let mut index = GameIndex::default();
    index.country_tags.insert("SOV".to_string());
    index.effects.insert("add_stability".to_string());
    index
        .effects
        .insert("add_scaled_political_power".to_string());

    let context = prepare_edit_context_markdown(
        &input,
        &root,
        "SOV",
        "sov_ctx",
        None,
        None,
        None,
        &[],
        Some(&index),
        20,
        20,
        10,
        None,
    )
    .unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(context.contains("- dry_run_validation: `strict-code-index`"));
    assert!(context.contains("## AI Authoring Contract"));
    assert!(context.contains("compile-intent --kind auto"));
    assert!(context.contains("- status: `BLOCKED`"));
    assert!(context.contains("dry-run safety status is blocked"));
    assert!(context.contains("dry-run safety blocks final code"));
    assert!(context.contains("Do not write final Clausewitz"));
    assert!(context.contains("\"final_code_allowed\": false"));
    assert!(context.contains("unindexed effect `add_political_power`"));
    assert!(context.contains("related indexed code: effects/effect `add_scaled_political_power`"));
}

#[test]
fn prepare_edit_context_requires_game_root_for_dependency_mod_paths() {
    let root = unique_temp_dir("edit-context-dependency-needs-game-root");
    let target_mod = root.join("target");
    let dependency_mod = root.join("dependency");
    fs::create_dir_all(&target_mod).unwrap();
    fs::create_dir_all(&dependency_mod).unwrap();
    fs::write(
        target_mod.join("descriptor.mod"),
        "name=\"Target Context Test\"\nsupported_version=\"*\"\n",
    )
    .unwrap();
    fs::write(
        dependency_mod.join("descriptor.mod"),
        "name=\"Dependency Context Test\"\nsupported_version=\"*\"\n",
    )
    .unwrap();
    let input = root.join("request.txt");
    let output = root.join("context.md");
    fs::write(&input, "给 SOV 加一个国策。").unwrap();

    let err = cmd_prepare_edit_context(&[
        "--input".to_string(),
        input.to_string_lossy().to_string(),
        "--mod-root".to_string(),
        target_mod.to_string_lossy().to_string(),
        "--tag".to_string(),
        "SOV".to_string(),
        "--prefix".to_string(),
        "sov_ctx".to_string(),
        "--mod-path".to_string(),
        dependency_mod.to_string_lossy().to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap_err();
    let output_exists = output.exists();
    fs::remove_dir_all(&root).unwrap();

    assert!(err.contains("--mod-path requires --game-root during edit-context preparation"));
    assert!(!output_exists);
}

#[test]
fn indexed_resource_summary_lists_verified_leader_portraits() {
    let mut index = GameIndex::default();
    index.country_tags.insert("CHI".to_string());
    index.ideologies.insert("democratic".to_string());
    index
        .focus_goal_sprites
        .insert("GFX_focus_CHI_democratic_reform".to_string());
    index
        .idea_pictures
        .insert("democratic_planned_economy".to_string());
    index
        .leader_portraits
        .insert("GFX_portrait_CHI_chairman_mao".to_string());

    let summary = render_indexed_resource_summary(&index, 10);

    assert!(summary.contains("leader_portraits: 1 total"));
    assert!(summary.contains("GFX_portrait_CHI_chairman_mao"));
    assert!(summary.contains("focus_goal_sprites: 1 total"));
    assert!(summary.contains("idea_pictures: 1 total"));
}

#[test]
fn workflow_applies_files_and_embeds_validation() {
    let root = unique_temp_dir("workflow-apply");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("descriptor.mod"),
        "name=\"Workflow Test\"\nsupported_version=\"*\"\n",
    )
    .unwrap();
    let text = "国策树：\n斯大林宪法\n继续新经济政策\n\n决议：鼓励奈普曼投资\n目标：SOV\n效果：政治点+25\n\n事件：新经济政策的未来\n类型：国家事件\n命名空间：sov_nep\n标题：新经济政策的未来\n描述：党内围绕新经济政策展开了激烈争论。\n选项A：继续试验\n效果A：政治点+50\n";

    let json = run_workflow_json(text, Some(&root), "SOV", "sov_nep", None, false, None).unwrap();
    let focus = fs::read_to_string(
        root.join("common")
            .join("national_focus")
            .join("sov_nep_SOV_focus.txt"),
    )
    .unwrap();
    let decisions = fs::read_to_string(
        root.join("common")
            .join("decisions")
            .join("sov_nep_decisions.txt"),
    )
    .unwrap();
    let events = fs::read_to_string(root.join("events").join("sov_nep_events.txt")).unwrap();
    let loc = String::from_utf8_lossy(&fs::read(target_localisation_path(&root, "SOV")).unwrap())
        .to_string();
    fs::remove_dir_all(&root).unwrap();

    assert!(json.contains("\"dry_run\": false"));
    assert!(json.contains("\"validation\": {\"ran\": true, \"ok\": true"));
    assert!(json.contains("sov_nep_SOV_focus.txt"));
    assert!(json.contains("sov_nep_decisions.txt"));
    assert!(json.contains("sov_nep_events.txt"));
    assert!(focus.contains("focus_tree = {"));
    assert!(focus.contains("SOV_"));
    assert!(decisions.contains("complete_effect = {"));
    assert!(decisions.contains("add_political_power = 25"));
    assert!(events.contains("add_namespace = sov_nep"));
    assert!(events.contains("country_event = {"));
    assert!(loc.contains("l_simp_chinese:"));
    assert!(loc.contains("新经济政策的未来"));
}

#[test]
fn strict_workflow_blocks_unresolved_cards_before_write() {
    let root = unique_temp_dir("strict-workflow-prewrite");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("descriptor.mod"),
        "name=\"Strict Workflow Test\"\nsupported_version=\"*\"\n",
    )
    .unwrap();
    let mut index = GameIndex::default();
    index.country_tags.insert("SOV".to_string());
    index.effects.insert("add_political_power".to_string());
    let text = "决议：未知动员\n目标：SOV\n效果：外星能量+50\n";

    let err = run_workflow_json_with_focus_layout_options(
        text,
        None,
        Some(&root),
        "SOV",
        "sov_ai",
        None,
        false,
        Some(&index),
        ValidationOptions {
            strict_code_index: true,
        },
    )
    .unwrap_err();
    let common_exists = root.join("common").exists();
    fs::remove_dir_all(&root).unwrap();

    assert!(err.contains("strict feature-card generation blocked unresolved AI mappings"));
    assert!(err.contains("外星能量+50"));
    assert!(!common_exists);
}

#[test]
fn strict_workflow_dry_run_reports_prewrite_code_index_blockers() {
    let root = unique_temp_dir("strict-workflow-dry-run-blockers");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("descriptor.mod"),
        "name=\"Strict Dry Run\"\nsupported_version=\"*\"\n",
    )
    .unwrap();
    let mut index = GameIndex::default();
    index.country_tags.insert("SOV".to_string());
    index.effects.insert("add_stability".to_string());
    index
        .effects
        .insert("add_scaled_political_power".to_string());
    let text = "决议：政治动员\n目标：SOV\n效果：政治点+50\n";

    let json = run_workflow_json_with_focus_layout_options(
        text,
        None,
        Some(&root),
        "SOV",
        "sov_ai",
        None,
        true,
        Some(&index),
        ValidationOptions {
            strict_code_index: true,
        },
    )
    .unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(json.contains("\"final_code_allowed\": false"));
    assert!(json.contains("feature_cards strict gate"));
    assert!(json.contains("unindexed effect `add_political_power`"));
    assert!(json.contains("related indexed code: effects/effect `add_scaled_political_power`"));
}

#[test]
fn one_sentence_workflow_synthesizes_focus_event_and_effects() {
    let text = "给德国加一个国策，完成后获得3个军工厂，并触发一个新闻事件。";
    let synthesized = synthesize_one_sentence_workflow(text, "GER", "ger_demo");

    assert!(synthesized.contains("国策树："));
    assert!(synthesized.contains("扩建军工体系"));
    assert!(synthesized.contains("# completion_reward: 3个军工厂"));
    assert!(synthesized.contains("事件：扩建军工体系的消息"));
    assert!(synthesized.contains("类型：新闻事件"));
    assert!(synthesized.contains("命名空间：ger_demo"));
    assert!(synthesized.contains("效果A：3个军工厂"));
}

#[test]
fn one_sentence_workflow_routes_long_term_focus_effects_through_idea() {
    let text = "给德国加一个国策，长期提高建造速度5%并降低消费品工厂3%。";
    let synthesized = synthesize_one_sentence_workflow(text, "GER", "ger_long_term");

    assert!(synthesized.contains("国策树："));
    assert!(synthesized.contains("# completion_reward: 添加民族精神"));
    assert!(synthesized.contains("民族精神："));
    assert!(synthesized.contains("建造速度"));
    assert!(synthesized.contains("消费品"));
    assert!(!synthesized.contains("# completion_reward: 建造速度"));
}

#[test]
fn one_sentence_workflow_synthesizes_technology_and_gui_without_default_decision() {
    let text = "给德国加一个独有科技和特殊GUI，显示铁路运力。";
    let synthesized = synthesize_one_sentence_workflow(text, "GER", "ger_tech_gui");

    assert!(!synthesized.contains("决议："));
    assert!(!synthesized.contains("国策树："));
    assert!(synthesized.contains("独有科技：重整铁路网络"));
    assert!(synthesized.contains("特殊GUI：重整铁路网络"));
    assert!(synthesized.contains("目标：GER"));
}

#[test]
fn country_inference_reads_localisation_and_verifies_country_file() {
    let root = unique_temp_dir("country-inference-source");
    write_fer_country_source(&root);

    let guess = infer_country_from_sources(
        "给远东铁路共和国加一个国策，完成后获得3个军工厂。",
        std::slice::from_ref(&root),
    )
    .unwrap()
    .unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(guess.tag, "FER");
    assert_eq!(guess.name, "远东铁路共和国");
    assert!(guess.source.contains("localisation/simp_chinese"));
    assert!(guess.source.contains("common/country_tags/00_tags.txt"));
    assert!(guess.source.contains("common/countries/FER.txt"));
}

#[test]
fn country_knowledge_resolves_target_instead_of_background_enemy() {
    let root = unique_temp_dir("country-target-context");
    write_country_knowledge_source(
        &root,
        &[("KOR", "Korea.txt", "韩国"), ("JAP", "Japan.txt", "日本")],
    );
    let guess = infer_country_from_sources(
        "依据韩国之春.xlsx生成一个韩国革命的mod，游戏时间是1936，是反抗日本的起义以后的国策",
        std::slice::from_ref(&root),
    )
    .unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(guess.unwrap().tag, "KOR");
}

#[test]
fn country_knowledge_uses_ideology_and_cosmetic_localisation_aliases() {
    let root = unique_temp_dir("country-target-alias");
    write_country_knowledge_source(&root, &[("KOR", "Korea.txt", "朝鲜")]);
    let loc_path = root
        .join("localisation")
        .join("simp_chinese")
        .join("countries_l_simp_chinese.yml");
    let mut loc = fs::read(&loc_path).unwrap();
    loc.extend_from_slice(b" KOR_democratic:0 \"\xe9\x9f\xa9\xe5\x9b\xbd\"\n");
    fs::write(&loc_path, loc).unwrap();

    let guess = infer_country_from_sources("为韩国制作国策", std::slice::from_ref(&root))
        .unwrap()
        .unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(guess.tag, "KOR");
    assert_eq!(guess.name, "韩国");
}

#[test]
fn country_knowledge_prefers_prc_for_chinese_communist_party_alias() {
    let root = unique_temp_dir("country-target-prc-alias");
    write_country_knowledge_source(
        &root,
        &[
            ("CHI", "China.txt", "中国"),
            ("PRC", "PRC.txt", "共产党中国"),
        ],
    );

    let guess =
        infer_country_from_sources("陈独秀回到中国共产党国策树", std::slice::from_ref(&root))
            .unwrap()
            .unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(guess.tag, "PRC");
    assert_eq!(guess.name, "中国共产党");
}

#[test]
fn country_knowledge_resolves_countries_absent_from_builtin_fallback() {
    let root = unique_temp_dir("country-target-vietnam");
    write_country_knowledge_source(
        &root,
        &[
            ("VIN", "Vietnam.txt", "越南"),
            ("FRA", "France.txt", "法国"),
        ],
    );
    let guess = infer_country_from_sources(
        "为越南制作摆脱法国统治后的国策、事件与民族精神",
        std::slice::from_ref(&root),
    )
    .unwrap()
    .unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(guess.tag, "VIN");
    assert_eq!(guess.name, "越南");
}

#[test]
fn tag_resolution_rejects_invented_revolutionary_committee_tag() {
    let request =
        "依据韩国之春.xlsx生成一个韩国革命的mod，游戏时间是1936，是反抗日本的起义以后的国策";
    let mut index = GameIndex::default();
    index.country_tags.insert("KOR".to_string());
    let inferred = CountryGuess {
        tag: "KOR".to_string(),
        name: "韩国".to_string(),
        source: "local country knowledge base".to_string(),
    };

    let error = resolve_country_tag(request, Some("KRC"), Some(inferred), Some(&index), false)
        .err()
        .unwrap();

    assert!(error.contains("request resolves to existing KOR"));
    assert!(error.contains("--tag KRC"));
    assert!(error.contains("not authorization to create a country TAG"));
}

#[test]
fn tag_resolution_reuses_indexed_kor_and_forbids_country_files() {
    let request = "给韩国制作国策、事件和民族精神";
    let mut index = GameIndex::default();
    index.country_tags.insert("KOR".to_string());
    let inferred = CountryGuess {
        tag: "KOR".to_string(),
        name: "韩国".to_string(),
        source: "local country knowledge base".to_string(),
    };
    let resolution =
        resolve_country_tag(request, Some("KOR"), Some(inferred), Some(&index), false).unwrap();
    let json = country_tag_resolution_json(request, &resolution);

    assert_eq!(resolution.decision, "reuse_existing_tag");
    assert_eq!(resolution.exists_in_index, Some(true));
    assert!(json.contains("\"resolved_tag\": \"KOR\""));
    assert!(json.contains("\"common/country_tags/*\""));
    assert!(!request_explicitly_creates_country_tag(request));
}

#[test]
fn new_tag_requires_literal_request_and_allow_flag() {
    let request = "创建新国家并创建新TAG KRC";
    let index = GameIndex::default();

    let blocked = resolve_country_tag(request, Some("KRC"), None, Some(&index), false)
        .err()
        .unwrap();
    assert!(blocked.contains("not present in the indexed"));

    let allowed = resolve_country_tag(request, Some("KRC"), None, Some(&index), true).unwrap();
    assert_eq!(allowed.tag, "KRC");
    assert_eq!(allowed.decision, "create_new_tag");
    assert!(allowed.new_tag_authorized);
}

#[test]
fn bare_tag_without_local_evidence_is_rejected() {
    let error = resolve_country_tag("制作这个国家的国策", Some("XYZ"), None, None, false)
        .err()
        .unwrap();

    assert!(error.contains("without local game/dependency/source-mod evidence"));
    assert!(error.contains("bare --tag is not proof"));
}

#[test]
fn generate_mod_infers_country_from_source_root() {
    let workspace = unique_temp_dir("generate-mod-source-inference");
    let source = workspace.join("source_mod");
    let output = workspace.join("generated_mod");
    let report = workspace.join("report.json");
    write_fer_country_source(&source);

    cmd_generate_mod(&[
        "--text".to_string(),
        "给远东铁路共和国加一个国策，完成后获得3个军工厂。".to_string(),
        "--source-root".to_string(),
        source.display().to_string(),
        "--output".to_string(),
        output.display().to_string(),
        "--report".to_string(),
        report.display().to_string(),
    ])
    .unwrap();

    let report_text = fs::read_to_string(&report).unwrap();
    let focus = fs::read_to_string(
        output
            .join("common")
            .join("national_focus")
            .join("fer_build_army_industry_FER_focus.txt"),
    )
    .unwrap();
    fs::remove_dir_all(&workspace).unwrap();

    assert!(report_text.contains("\"tag\": \"FER\""));
    assert!(report_text.contains("\"country_source\": \"localisation/simp_chinese"));
    assert!(focus.contains("id = FER_"));
    assert!(focus.contains("tag = FER"));
    assert!(focus.contains("type = arms_factory level = 3"));
}

#[test]
fn one_sentence_focus_tree_uses_default_stage_template() {
    let synthesized =
        synthesize_one_sentence_workflow("给德国做一套工业国策树。", "GER", "ger_industry");
    let layout_text = extract_focus_layout_text(&synthesized);
    let layout = parse_focus_layout(&layout_text, "GER", "ger_industry");

    assert_eq!(layout.rows.len(), 5);
    assert_eq!(
        layout
            .rows
            .iter()
            .map(|row| row.focus_ids.len())
            .collect::<Vec<_>>(),
        vec![1, 3, 1, 3, 1]
    );
    assert_eq!(
        layout
            .focuses
            .iter()
            .map(|focus| (focus.y, focus.x))
            .collect::<Vec<_>>(),
        vec![
            (0, 0),
            (1, -2),
            (1, 0),
            (1, 2),
            (2, 0),
            (3, -2),
            (3, 0),
            (3, 2),
            (4, 0),
        ]
    );
}

#[test]
fn apply_focus_layout_uses_indexed_goal_icons() {
    let root = unique_temp_dir("apply-focus-layout-indexed-icons");
    let game = unique_temp_dir("apply-focus-layout-game-icons");
    fs::create_dir_all(root.join("interface")).unwrap();
    fs::create_dir_all(game.join("interface")).unwrap();
    fs::write(
        root.join("interface").join("local_goals.gfx"),
        r#"spriteType = { name = "GFX_goal_local_political_reform" texturefile = "gfx/interface/goals/local.dds" }"#,
    )
    .unwrap();
    fs::write(
        root.join("interface").join("local_random.gfx"),
        r#"spriteType = { name = "GFX_goal_aaa_factory" texturefile = "gfx/interface/goals/decoy.dds" }"#,
    )
    .unwrap();
    fs::write(
        game.join("interface").join("game_goals.gfx"),
        r#"spriteType = { name = "GFX_goal_game_factory" texturefile = "gfx/interface/goals/factory.dds" }
spriteType = { name = "GFX_focus_generic_workers" texturefile = "gfx/interface/goals/workers.dds" }
spriteType = { name = "GFX_focus_SOV_socialism_in_one_country" texturefile = "gfx/interface/goals/socialism.dds" }"#,
    )
    .unwrap();
    fs::write(
        game.join("interface").join("game_goals_shine.gfx"),
        r#"spriteType = { name = "GFX_goal_game_factory_shine" texturefile = "gfx/interface/goals/factory.dds" }"#,
    )
    .unwrap();
    fs::write(
        game.join("interface").join("game_random.gfx"),
        r#"spriteType = { name = "GFX_goal_aaa_factory" texturefile = "gfx/interface/goals/decoy.dds" }"#,
    )
    .unwrap();
    let index = build_game_index(&game).unwrap();
    let layout = parse_focus_layout(
        "工业复兴\n政治改革\n工人起义   社会主义建设\n",
        "SOV",
        "sov_alt",
    );

    apply_focus_layout_to_mod_with_index(&root, &layout, "SOV", "sov_alt", Some(&index)).unwrap();

    let focus_file = fs::read_to_string(
        root.join("common")
            .join("national_focus")
            .join("sov_alt_SOV_focus.txt"),
    )
    .unwrap();
    fs::remove_dir_all(&root).unwrap();
    fs::remove_dir_all(&game).unwrap();

    assert!(index.focus_goal_sprites.contains("GFX_goal_game_factory"));
    assert!(index
        .focus_goal_sprites
        .contains("GFX_focus_generic_workers"));
    assert!(index
        .focus_goal_sprites
        .contains("GFX_focus_SOV_socialism_in_one_country"));
    assert!(!index.focus_goal_sprites.contains("GFX_goal_aaa_factory"));
    assert!(focus_file.contains("id = SOV_industry_revival"));
    assert!(focus_file.contains("icon = GFX_goal_game_factory"));
    assert!(!focus_file.contains("icon = GFX_goal_aaa_factory"));
    assert!(focus_file.contains("id = SOV_political_reform"));
    assert!(focus_file.contains("icon = GFX_goal_local_political_reform"));
    assert!(focus_file.contains("id = SOV_industry"));
    assert!(focus_file.contains("icon = GFX_focus_generic_workers"));
    assert!(focus_file.contains("id = SOV_society_build"));
    assert!(focus_file.contains("icon = GFX_focus_SOV_socialism_in_one_country"));
}

#[test]
fn workflow_dry_run_embeds_semantic_indexed_focus_icons() {
    let game = unique_temp_dir("workflow-dry-run-icons-game");
    fs::create_dir_all(game.join("interface")).unwrap();
    fs::write(
        game.join("interface").join("goals.gfx"),
        r#"spriteType = { name = "GFX_focus_generic_workers" texturefile = "gfx/interface/goals/workers.dds" }
spriteType = { name = "GFX_focus_SOV_socialism_in_one_country" texturefile = "gfx/interface/goals/socialism.dds" }"#,
    )
    .unwrap();
    let index = build_game_index(&game).unwrap();

    let json = run_workflow_json(
        "国策树：\n工人起义   社会主义建设\n",
        None,
        "SOV",
        "sov_alt",
        None,
        true,
        Some(&index),
    )
    .unwrap();
    fs::remove_dir_all(&game).unwrap();

    assert!(json.contains("\"title\": \"工人起义\""));
    assert!(json.contains("\"icon\": \"GFX_focus_generic_workers\""));
    assert!(json.contains("\"title\": \"社会主义建设\""));
    assert!(json.contains("\"icon\": \"GFX_focus_SOV_socialism_in_one_country\""));
}

#[test]
fn semantic_icon_keywords_cover_major_ideologies_countries_and_leaders() {
    let democratic = focus_icon_keywords("美国民主选举与宪政改革");
    assert!(democratic.contains(&"usa"));
    assert!(democratic.contains(&"democratic"));
    assert!(democratic.contains(&"election"));

    let fascist = focus_icon_keywords("德国法西斯黑衫运动");
    assert!(fascist.contains(&"ger"));
    assert!(fascist.contains(&"fascist"));
    assert!(fascist.contains(&"blackshirt"));

    let monarchist = focus_icon_keywords("日本皇帝与君主制复辟");
    assert!(monarchist.contains(&"jap"));
    assert!(monarchist.contains(&"monarchist"));
    assert!(monarchist.contains(&"emperor"));

    let anarchist = focus_icon_keywords("西班牙无政府工团公社");
    assert!(anarchist.contains(&"spr"));
    assert!(anarchist.contains(&"anarchist"));
    assert!(anarchist.contains(&"syndicalist"));

    let leader = focus_icon_keywords("中国主席与革命领袖");
    assert!(leader.contains(&"chi"));
    assert!(leader.contains(&"chairman"));
    assert!(leader.contains(&"leader"));
}

#[test]
fn semantic_icon_matching_handles_ideologies_countries_and_leaders() {
    let catalog = BTreeSet::from([
        "GFX_focus_generic_anarchist_commune".to_string(),
        "GFX_focus_generic_befriend_republican_spain_focus".to_string(),
        "GFX_focus_generic_democratic_reform".to_string(),
        "GFX_focus_generic_fascist_movement".to_string(),
        "GFX_focus_GER_revive_the_kaiserreich".to_string(),
        "GFX_focus_JAP_draft_the_showa_constitution".to_string(),
        "GFX_focus_JAP_democratic_reform".to_string(),
        "GFX_focus_JAP_promote_japanese_settlement".to_string(),
        "GFX_focus_SWI_closer_ties_with_germany".to_string(),
        "GFX_focus_spr_anarchism_knows_no_borders".to_string(),
        "GFX_focus_SOV_workers_council".to_string(),
    ]);

    assert_eq!(
        choose_focus_icon_from_catalog("民主选举", &catalog).as_deref(),
        Some("GFX_focus_generic_democratic_reform")
    );
    assert_eq!(
        choose_focus_icon_from_catalog("法西斯运动", &catalog).as_deref(),
        Some("GFX_focus_generic_fascist_movement")
    );
    assert_eq!(
        choose_focus_icon_from_catalog("德国君主制复辟", &catalog).as_deref(),
        Some("GFX_focus_GER_revive_the_kaiserreich")
    );
    assert_eq!(
        choose_focus_icon_from_catalog("西班牙无政府公社", &catalog).as_deref(),
        Some("GFX_focus_spr_anarchism_knows_no_borders")
    );
    assert_eq!(
        choose_focus_icon_from_catalog("日本民主改革", &catalog).as_deref(),
        Some("GFX_focus_JAP_democratic_reform")
    );
    assert_ne!(
        choose_focus_icon_from_catalog("德国君主制复辟", &catalog).as_deref(),
        Some("GFX_focus_SWI_closer_ties_with_germany")
    );

    let portraits = BTreeSet::from([
        "GFX_portrait_GER_wilhelm_ii".to_string(),
        "GFX_portrait_CHI_chairman_mao".to_string(),
        "GFX_portrait_USA_democratic_president".to_string(),
    ]);
    assert_eq!(
        choose_semantic_reference_from_catalog("中国主席", &portraits).as_deref(),
        Some("GFX_portrait_CHI_chairman_mao")
    );
    assert_eq!(
        choose_semantic_reference_from_catalog("美国总统民主派", &portraits).as_deref(),
        Some("GFX_portrait_USA_democratic_president")
    );
}

#[test]
fn apply_feature_cards_uses_registered_idea_picture_without_gfx_prefix() {
    let root = unique_temp_dir("apply-feature-cards-idea-picture");
    let game = unique_temp_dir("game-index-idea-picture");
    fs::create_dir_all(root.join("common").join("ideas")).unwrap();
    fs::create_dir_all(root.join("interface")).unwrap();
    fs::create_dir_all(game.join("interface")).unwrap();
    fs::write(
        root.join("interface").join("local_ideas.gfx"),
        r#"spriteType = { name = "GFX_idea_local_naval_reform" texturefile = "gfx/interface/ideas/naval.dds" }"#,
    )
    .unwrap();
    fs::write(
        game.join("interface").join("ideas.gfx"),
        r#"spriteType = { name = "GFX_idea_democratic_planned_economy" texturefile = "gfx//interface//ideas//democratic_planned_economy.dds" }"#,
    )
    .unwrap();
    fs::write(
        game.join("interface").join("decisions.gfx"),
        r#"spriteType = { name = "GFX_decision_SOV_the_workers_dictatorship" texturefile = "gfx/interface/decisions/workers.dds" }
spriteType = { name = "GFX_decision_category_generic_communism" texturefile = "gfx/interface/decisions/category_communism.dds" }"#,
    )
    .unwrap();
    let index = build_game_index(&game).unwrap();
    let cards = parse_cards(
        "民族精神：民主计划经济\n目标：SOV\n效果：稳定度+5%\n\n决议：工人委员会动员\n目标：SOV\n分类：共产主义动员\n效果：政治点+25",
        FEATURE_CARD_HEADERS,
    );

    apply_feature_cards_to_mod_with_index(&root, &cards, "SOV", "sov_reform", Some(&index))
        .unwrap();

    let ideas = fs::read_to_string(
        root.join("common")
            .join("ideas")
            .join("sov_reform_ideas.txt"),
    )
    .unwrap();
    let decisions = fs::read_to_string(
        root.join("common")
            .join("decisions")
            .join("sov_reform_decisions.txt"),
    )
    .unwrap();
    let categories = fs::read_to_string(
        root.join("common")
            .join("decisions")
            .join("categories")
            .join("sov_reform_categories.txt"),
    )
    .unwrap();
    fs::remove_dir_all(&root).unwrap();
    fs::remove_dir_all(&game).unwrap();

    assert!(index.idea_pictures.contains("democratic_planned_economy"));
    assert!(index
        .decision_icons
        .contains("SOV_the_workers_dictatorship"));
    assert!(index
        .decision_category_pictures
        .contains("GFX_decision_category_generic_communism"));
    assert!(ideas.contains("picture = democratic_planned_economy"));
    assert!(!ideas.contains("picture = GFX_idea_democratic_planned_economy"));
    assert!(decisions.contains("icon = SOV_the_workers_dictatorship"));
    assert!(categories.contains("icon = GFX_decision_SOV_the_workers_dictatorship"));
    assert!(categories.contains("picture = GFX_decision_category_generic_communism"));
}

#[test]
fn explicit_idea_gfx_sprite_is_normalized_to_picture_name() {
    let cards = parse_cards(
        "民族精神：民主计划经济\n图标：GFX_idea_democratic_planned_economy\n效果：稳定度+5%",
        FEATURE_CARD_HEADERS,
    );
    let rendered = render_idea_block(&cards[0], "sov_planned_economy_idea");

    assert!(rendered.contains("picture = democratic_planned_economy"));
    assert!(!rendered.contains("picture = GFX_idea_democratic_planned_economy"));
}

#[test]
fn scaffold_writes_mod_names_only_to_descriptors() {
    let root = unique_temp_dir("scaffold-no-mod-name-loc");
    let created = scaffold_mod(
        &root,
        "共和国一九七九：委员会民主",
        "0.1.0",
        "1.16.*",
        "Alternative History",
        true,
    )
    .unwrap();
    let mod_id = slugify(
        root.file_name().and_then(OsStr::to_str).unwrap_or("mod"),
        "hoi4_mod",
    );
    let launcher_path = root
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("{mod_id}.mod"));
    let descriptor = fs::read_to_string(root.join("descriptor.mod")).unwrap();
    let launcher = fs::read_to_string(&launcher_path).unwrap();
    fs::remove_file(&launcher_path).unwrap();

    assert!(descriptor.contains("name=\"共和国一九七九：委员会民主\""));
    assert!(launcher.contains("name=\"共和国一九七九：委员会民主\""));
    assert!(launcher.contains("path="));
    assert_eq!(
        collect_files(&root)
            .unwrap()
            .iter()
            .filter(|path| path.is_file())
            .count(),
        1
    );
    assert!(!root.join("common").exists());
    assert!(!root.join("history").exists());
    assert!(!root.join("interface").exists());
    assert!(!created.iter().any(|path| path
        .file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| name.ends_with("_l_simp_chinese.yml"))));
    assert!(!root.join("localisation").exists());
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn generate_mod_from_one_sentence_creates_playable_mod_files() {
    let root = unique_temp_dir("one-sentence-mod");
    let request = GenerateModRequest {
        text: "给德国加一个国策，完成后获得3个军工厂，并触发一个新闻事件。",
        mod_root: &root,
        name: "德国：扩建军工体系",
        tag: "GER",
        prefix: "ger_demo",
        tags: "Alternative History",
        version: "0.1.0",
        supported_version: "*",
        launcher_file: false,
        dry_run: false,
        country_source: None,
        game_index: None,
        validation_options: ValidationOptions::default(),
    };
    let json = generate_mod_json(&request).unwrap();

    let descriptor = fs::read_to_string(root.join("descriptor.mod")).unwrap();
    let focus = fs::read_to_string(
        root.join("common")
            .join("national_focus")
            .join("ger_demo_GER_focus.txt"),
    )
    .unwrap();
    let events = fs::read_to_string(root.join("events").join("ger_demo_events.txt")).unwrap();
    let loc = String::from_utf8_lossy(&fs::read(target_localisation_path(&root, "GER")).unwrap())
        .to_string();
    let localisation_files = collect_files(&root.join("localisation")).unwrap();
    let localisation_contains_mod_name = localisation_files.iter().any(|path| {
        read_utf8_lossy(path)
            .map(|text| text.contains("_mod_name"))
            .unwrap_or(false)
    });
    fs::remove_dir_all(&root).unwrap();

    assert!(json.contains("\"schema\": \"hoi4skill.one_sentence_mod.v1\""));
    assert!(json.contains("\"schema\": \"hoi4skill.copy_to_code_workflow.v1\""));
    assert!(descriptor.contains("德国：扩建军工体系"));
    assert!(!json.contains("_mod_name"));
    assert!(focus.contains("id = GER_"));
    assert!(focus.contains("random_owned_controlled_state = {"));
    assert!(focus.contains("type = arms_factory level = 3"));
    assert!(events.contains("add_namespace = ger_demo"));
    assert!(events.contains("news_event = {"));
    assert!(events.contains("type = arms_factory level = 3"));
    assert!(loc.contains("扩建军工体系的消息"));
    assert!(!localisation_contains_mod_name);
}

#[test]
fn strict_generate_mod_blocks_missing_effect_index_before_content_write() {
    let root = unique_temp_dir("strict-one-sentence-mod");
    let game = root.join("game");
    let output = root.join("mod");
    fs::create_dir_all(game.join("common").join("country_tags")).unwrap();
    fs::write(
        game.join("common")
            .join("country_tags")
            .join("00_countries.txt"),
        "GER = \"countries/Germany.txt\"\n",
    )
    .unwrap();

    let err = cmd_generate_mod(&[
        "--text".to_string(),
        "给德国加一个国策，完成后获得3个军工厂。".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
        "--game-root".to_string(),
        game.to_string_lossy().to_string(),
        "--tag".to_string(),
        "GER".to_string(),
        "--prefix".to_string(),
        "ger_demo".to_string(),
        "--final-check".to_string(),
    ])
    .unwrap_err();
    let focus_exists = output
        .join("common")
        .join("national_focus")
        .join("ger_demo_GER_focus.txt")
        .exists();
    let events_exists = output.join("events").join("ger_demo_events.txt").exists();
    fs::remove_dir_all(&root).unwrap();

    assert!(err.contains("strict focus layout generation blocked unresolved AI mappings"));
    assert!(err.contains("strict code index has no indexed effects"));
    assert!(!focus_exists);
    assert!(!events_exists);
}

#[test]
fn focus_layout_generates_ascii_english_ids_from_chinese_titles() {
    let layout = parse_focus_layout(
        "远东铁路委员会\n重启西伯利亚干线   互斥   开放太平洋贸易\n整编护路军   工业移民计划   港口自由区",
        "FER",
        "fer_rail",
    );

    let ids = layout
        .focuses
        .iter()
        .map(|focus| focus.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(layout.tree_id, "fer_rail_FER_focus_tree");
    assert!(ids.contains(&"FER_far_east_railway_committee"));
    assert!(ids.contains(&"FER_reopen_siberian_mainline"));
    assert!(ids.contains(&"FER_open_pacific_trade"));
    assert!(ids.contains(&"FER_reorganize_railway_guard"));
    assert!(ids.contains(&"FER_industrial_migration_plan"));
    assert!(ids.contains(&"FER_port_free_zone"));
    for id in ids {
        assert!(id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_'));
    }
}

#[test]
fn focus_layout_accepts_explicit_ai_id_hints() {
    let layout = parse_focus_layout(
        "远东铁路委员会 | far_east_railway_committee\n重启西伯利亚干线 [reopen_siberian_line]\n开放太平洋贸易 (id: pacific_trade)",
        "FER",
        "远东铁路",
    );

    assert_eq!(layout.tree_id, "focus_FER_focus_tree");
    assert_eq!(layout.focuses[0].title, "远东铁路委员会");
    assert_eq!(layout.focuses[0].id, "FER_far_east_railway_committee");
    assert_eq!(layout.focuses[1].id, "FER_reopen_siberian_line");
    assert_eq!(layout.focuses[2].id, "FER_pacific_trade");
}

#[test]
fn feature_cards_cover_documented_effect_shorthands() {
    let json = parse_decision_idea_cards_json(
        "民族精神：新经济政策复兴\n目标：SOV\n效果：稳定度+5%，建造速度+5%，消费品工厂-3%，战争正当化 = -10%\n移除：不可手动移除\n\n决议：整训舰队\n目标：ITA\n可用：战争中\n效果：海军经验+25，陆军经验+5，空军经验+5，获得民族精神 舰队整训，触发新闻 海军改革，军工+3",
        "SOV",
        "sov_nep",
    );

    assert!(json.contains("\"code\": \"production_speed_buildings_factor = 0.05\""));
    assert!(json.contains("\"code\": \"consumer_goods_factor = -0.03\""));
    assert!(json.contains("\"code\": \"justify_war_goal_time = -0.1\""));
    assert!(json.contains("\"code\": \"has_war = yes\""));
    assert!(json.contains("\"code\": \"navy_experience = 25\""));
    assert!(json.contains("\"code\": \"army_experience = 5\""));
    assert!(json.contains("\"code\": \"air_experience = 5\""));
    assert!(json.contains("\"code\": \"add_ideas = <idea id for 舰队整训>\""));
    assert!(json.contains("\"code\": \"news_event = { id = <event id for 海军改革> }\""));
    assert!(json.contains(
        "\"code\": \"add_building_construction = { type = arms_factory level = <number> instant_build = yes }\""
    ));
}

#[test]
fn parse_feature_cards_json_marks_unresolved_suggestions_as_blocked() {
    let json = parse_decision_idea_cards_json(
        "决议：神秘工业动员\n目标：KOR\n效果：外星工厂+5",
        "KOR",
        "kor_ai",
    );

    assert!(json.contains("\"safety\": {\"status\": \"blocked\""));
    assert!(json.contains("\"final_code_allowed\": false"));
    assert!(json.contains("\"requires_mapping\": true"));
    assert!(json.contains("raw_effect `外星工厂+5` must be mapped"));
}

#[test]
fn parse_cards_ignores_ai_markdown_noise_inside_cards() {
    let cards = parse_cards(
        "民族精神：工人自治委员会\n下面是按你的要求写的民族精神：\n```txt\n目标：CPC\n效果：稳定度＋5％\n```\n说明：以上不是本地化文案。\n描述：工人委员会正在重建基层组织。\n",
        &["决议", "民族精神"],
    );

    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].title, "工人自治委员会");
    assert_eq!(cards[0].fields.get("目标").map(String::as_str), Some("CPC"));
    assert_eq!(
        cards[0].fields.get("效果").map(String::as_str),
        Some("稳定度＋5％")
    );
    assert_eq!(
        cards[0].fields.get("描述").map(String::as_str),
        Some("工人委员会正在重建基层组织。")
    );
    assert!(!cards[0].fields.contains_key("说明"));
}

#[test]
fn feature_card_idea_ids_end_with_idea_suffix() {
    let cards = parse_cards(
        "民族精神：铁路主权破碎\n目标：FER\n效果：稳定度-5%\n\n决议：重整铁路局\n目标：FER\n效果：政治点+25",
        &["决议", "民族精神"],
    );
    let idea_id = feature_card_id(&cards[0], "fer_rail", "idea", 0);
    let decision_id = feature_card_id(&cards[1], "fer_rail", "decision", 1);
    let json = parse_decision_idea_cards_json(
        "民族精神：铁路主权破碎\n目标：FER\n效果：稳定度-5%",
        "FER",
        "fer_rail",
    );

    assert_eq!(idea_id, "fer_rail_spirit_0_idea");
    assert_eq!(decision_id, "fer_rail_decision_1");
    assert!(json.contains("\"id\": \"fer_rail_spirit_0_idea\""));
}

#[test]
fn country_localisation_template_groups_country_sections() {
    let map = parse_args(&[
        "--tag".to_string(),
        "FER".to_string(),
        "--name".to_string(),
        "远东铁路共和国".to_string(),
        "--prefix".to_string(),
        "fer_rail".to_string(),
        "--cosmetic-name".to_string(),
        "远东铁路委员会".to_string(),
        "--focus".to_string(),
        "FER_reopen_siberian_mainline=重启西伯利亚干线".to_string(),
        "--idea".to_string(),
        "FER_fragmented_railway_authority=分裂的铁路主权".to_string(),
        "--decision".to_string(),
        "fer_rail_reorganize_bureau=重整铁路局".to_string(),
        "--event".to_string(),
        "fer_rail.1=铁路委员会会议".to_string(),
        "--tech".to_string(),
        "FER_railway_logistics=铁路后勤学".to_string(),
        "--gui".to_string(),
        "FER_RAILWAY_STATUS=铁路状态".to_string(),
    ]);
    let template = country_localisation_template(&map, "FER", "远东铁路共和国", "fer_rail");

    assert!(template.starts_with('\u{feff}'));
    assert!(template.contains("# ===== 国家 tag / 国家名 ====="));
    assert!(template.contains("FER:0 \"远东铁路共和国\""));
    assert!(template.contains("# ===== 国家 cosmetic 名 ====="));
    assert!(template.contains("FER_fer_rail_cosmetic:0 \"远东铁路委员会\""));
    assert!(template.contains("# ===== 国策树 ====="));
    assert!(template.contains("FER_reopen_siberian_mainline:0 \"重启西伯利亚干线\""));
    assert!(template.contains("# ===== 民族精神 ====="));
    assert!(template.contains("FER_fragmented_railway_authority_idea:0 \"分裂的铁路主权\""));
    assert!(template.contains("# ===== 决议 ====="));
    assert!(template.contains("# ===== 事件 ====="));
    assert!(template.contains("fer_rail.1.t:0 \"铁路委员会会议\""));
    assert!(template.contains("# ===== 独有特殊科技 ====="));
    assert!(template.contains("FER_railway_logistics:0 \"铁路后勤学\""));
    assert!(template.contains("# ===== 特殊 GUI ====="));
    assert!(template.contains("FER_RAILWAY_STATUS:0 \"铁路状态\""));
}

#[test]
fn single_line_sprite_type_keeps_texturefile() {
    let text = r#"spriteType = { name = "GFX_single_line_icon" texturefile = "gfx/interface/test_icon.png" }"#;
    let blocks = blocks_named(text, "spriteType");

    assert_eq!(blocks.len(), 1);
    assert_eq!(
        block_assignment(&blocks[0], "name").as_deref(),
        Some("GFX_single_line_icon")
    );
    assert_eq!(
        block_assignment(&blocks[0], "texturefile").as_deref(),
        Some("gfx/interface/test_icon.png")
    );
}

#[test]
fn register_gfx_icons_writes_all_ui_sprite_categories() {
    let root = unique_temp_dir("register-gfx-all");
    let image_root = root.join("gfx").join("interface");
    fs::create_dir_all(image_root.join("goals")).unwrap();
    fs::write(image_root.join("goals").join("sov_factory.png"), b"").unwrap();
    fs::write(image_root.join("goals").join("重建东南.dds"), b"").unwrap();

    let categories = parse_gfx_registration_categories(Some("all")).unwrap();
    let report = register_gfx_icons(&root, "sov_nep", &categories).unwrap();
    let dynamic = root.join("interface").join("sov_nep_dynamic_icons.gfx");
    let goals = root.join("interface").join("sov_nep_goals.gfx");
    let goals_shine = root.join("interface").join("sov_nep_goals_shine.gfx");
    let focus_idea = root.join("interface").join("sov_nep_focus_idea_icons.gfx");
    let event = root.join("interface").join("sov_nep_event_pictures.gfx");
    let decision = root.join("interface").join("sov_nep_decision_pictures.gfx");

    assert_eq!(report.assets_scanned, 2);
    assert_eq!(report.entries.len(), 14);
    assert_eq!(report.changed_files.len(), 7);
    assert!(read_utf8_lossy(&dynamic)
        .unwrap()
        .contains(r#"name = "GFX_sov_nep_goals_sov_factory""#));
    assert!(read_utf8_lossy(&dynamic)
        .unwrap()
        .contains(r#"name = "GFX_sov_nep_goals_rebuild_southeast""#));
    assert!(read_utf8_lossy(&goals).unwrap().contains("SpriteType = {"));
    assert!(read_utf8_lossy(&goals)
        .unwrap()
        .contains(r#"name = "GFX_goal_sov_nep_goals_sov_factory""#));
    assert!(read_utf8_lossy(&goals_shine)
        .unwrap()
        .contains(r#"name = "GFX_goal_sov_nep_goals_sov_factory_shine""#));
    assert!(read_utf8_lossy(&goals_shine)
        .unwrap()
        .contains(r#"effectFile = "gfx/FX/buttonstate.lua""#));
    assert!(read_utf8_lossy(&goals_shine)
        .unwrap()
        .contains("legacy_lazy_load = no"));
    assert!(read_utf8_lossy(&goals_shine)
        .unwrap()
        .contains("animationrotation = -90.0"));
    assert!(read_utf8_lossy(&goals_shine)
        .unwrap()
        .contains("animationrotation = 90.0"));
    assert!(read_utf8_lossy(&focus_idea)
        .unwrap()
        .contains(r#"name = "GFX_idea_sov_nep_goals_sov_factory""#));
    assert!(read_utf8_lossy(&focus_idea)
        .unwrap()
        .contains("spriteType = {"));
    assert!(read_utf8_lossy(&event)
        .unwrap()
        .contains(r#"name = "GFX_report_event_sov_nep_goals_sov_factory""#));
    let decision_text = read_utf8_lossy(&decision).unwrap();
    assert!(decision_text.contains(r#"name = "GFX_decision_sov_nep_goals_sov_factory""#));
    assert!(decision_text.contains(r#"name = "GFX_decision_category_sov_nep_goals_sov_factory""#));

    let renamed = image_root.join("goals").join("rebuild_southeast.dds");
    assert!(renamed.exists());
    assert!(!image_root.join("goals").join("重建东南.dds").exists());
    let renamed_entry = report
        .entries
        .iter()
        .find(|entry| entry.texturefile == "gfx/interface/goals/rebuild_southeast.dds")
        .unwrap();
    assert_eq!(
        renamed_entry.original_texturefile.as_deref(),
        Some("gfx/interface/goals/重建东南.dds")
    );
    assert_eq!(renamed_entry.english_file_name, "rebuild_southeast.dds");
    assert!(renamed_entry.remark.contains("自动翻译中文文件名"));
    let english_entry = report
        .entries
        .iter()
        .find(|entry| entry.texturefile == "gfx/interface/goals/sov_factory.png")
        .unwrap();
    assert!(english_entry.original_texturefile.is_none());
    assert_eq!(english_entry.english_file_name, "sov_factory.png");

    let rerun = register_gfx_icons(&root, "sov_nep", &categories).unwrap();
    assert!(rerun.changed_files.is_empty());
    assert!(rerun.entries.iter().all(|entry| entry.status == "existing"));

    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn register_gfx_icons_skips_untranslated_chinese_filename() {
    let root = unique_temp_dir("register-gfx-untranslated");
    let image_root = root.join("gfx").join("interface").join("goals");
    fs::create_dir_all(&image_root).unwrap();
    let unknown = image_root.join("龘.dds");
    fs::write(&unknown, b"").unwrap();

    let categories = parse_gfx_registration_categories(Some("focus")).unwrap();
    let report = register_gfx_icons(&root, "sov_nep", &categories).unwrap();
    let json = gfx_registration_report_json(&report);

    assert_eq!(report.assets_scanned, 1);
    assert!(report.entries.is_empty());
    assert_eq!(report.skipped_assets.len(), 1);
    assert_eq!(
        report.skipped_assets[0].texturefile,
        "gfx/interface/goals/龘.dds"
    );
    assert!(report.skipped_assets[0]
        .reason
        .contains("cannot translate image filename"));
    assert!(report.skipped_assets[0]
        .required_action
        .contains("未注册 sprite"));
    assert!(json.contains("\"assets_skipped\": 1"));
    assert!(json.contains("\"skipped_assets\""));
    assert!(unknown.exists());
    assert!(!root.join("interface").join("sov_nep_goals.gfx").exists());

    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn register_gfx_icons_reverse_lookups_and_renames_name_conflicts() {
    let root = unique_temp_dir("register-gfx-conflict");
    fs::create_dir_all(root.join("gfx").join("interface").join("goals")).unwrap();
    fs::create_dir_all(root.join("interface")).unwrap();
    fs::write(
        root.join("gfx")
            .join("interface")
            .join("goals")
            .join("factory.png"),
        b"",
    )
    .unwrap();
    fs::write(
        root.join("interface").join("legacy.gfx"),
        r#"spriteTypes = {
	spriteType = { name = "GFX_legacy_factory" texturefile = "gfx/interface/goals/factory.png" }
	spriteType = { name = "GFX_goal_sov_nep_goals_factory" texturefile = "gfx/interface/goals/other.png" }
}
"#,
    )
    .unwrap();

    let categories = parse_gfx_registration_categories(Some("focus")).unwrap();
    let report = register_gfx_icons(&root, "sov_nep", &categories).unwrap();
    let entry = report
        .entries
        .iter()
        .find(|entry| entry.category == "focus")
        .unwrap();

    assert_eq!(entry.status, "renamed");
    assert_eq!(entry.sprite_name, "GFX_goal_sov_nep_goals_factory_2");
    assert!(entry
        .existing_names
        .contains(&"GFX_legacy_factory".to_string()));
    assert!(entry
        .conflict
        .as_deref()
        .unwrap_or("")
        .contains("other.png"));
    assert!(
        read_utf8_lossy(&root.join("interface").join("sov_nep_goals.gfx"))
            .unwrap()
            .contains(r#"name = "GFX_goal_sov_nep_goals_factory_2""#)
    );

    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn block_scanner_ignores_plain_words_before_real_blocks() {
    let text = "# Generated national focus tree by hoi4skill\nfocus_tree = {\n\tid = tree_id\n\tfocus = { id = real_focus }\n}\n";
    let blocks = blocks_named(text, "focus");

    assert_eq!(blocks.len(), 1);
    assert_eq!(
        block_assignment(&blocks[0], "id").as_deref(),
        Some("real_focus")
    );
}

#[test]
fn block_scanner_handles_multibyte_text_before_closing_brace() {
    let text = "focus = {\n\tid = CPC_real\n\t# 解放两广\n\tdesc = \"群众路线\"\n}\nfocus = { id = CPC_next }\n";
    let blocks = blocks_named(text, "focus");

    assert_eq!(blocks.len(), 2);
    assert_eq!(
        block_assignment(&blocks[0], "id").as_deref(),
        Some("CPC_real")
    );
    assert_eq!(
        block_assignment(&blocks[1], "id").as_deref(),
        Some("CPC_next")
    );
}

#[test]
fn sprite_type_blocks_accept_uppercase_and_lowercase_forms() {
    let text = "spriteType = { name = \"GFX_lower\" texturefile = \"gfx/interface/lower.png\" }\nSpriteType = { name = \"GFX_upper\" texturefile = \"gfx/interface/upper.png\" }\n";
    let blocks = sprite_type_blocks(text);
    let mut names = blocks
        .iter()
        .filter_map(|block| block_assignment(block, "name"))
        .collect::<Vec<_>>();
    names.sort();

    assert_eq!(
        names,
        vec!["GFX_lower".to_string(), "GFX_upper".to_string()]
    );
}

#[test]
fn render_sprite_type_block_uses_dynamic_gui_meter_template() {
    let block = render_sprite_type_block(
        "GFX_CPC_KMT_paranoia_meter",
        "gfx/interface/paranoia/CPC_KMT_paranoia_meter.dds",
        "paranoia meter",
        GfxSpriteRenderKind::DynamicGui,
    );

    assert!(block.contains("spriteType = {"));
    assert!(block.contains("legacy_lazy_load = no"));
    assert!(block.contains("noOfFrames = 21"));
    assert!(block.contains(r#"name = "GFX_CPC_KMT_paranoia_meter""#));
}

#[test]
fn apply_feature_cards_writes_decisions_ideas_and_localisation() {
    let root = unique_temp_dir("apply-feature-cards");
    fs::create_dir_all(&root).unwrap();
    let cards = parse_cards(
        "决议：整训舰队\n目标：ITA\n分类：海军改革\n花费：50政治点\n冷却：30天\n可用：战争中\n效果：海军经验+25，军工+3\n描述：集中资源整训舰队。\n\n民族精神：舰队整训\n目标：ITA\n效果：稳定度+5%，战争支持+2%\nllm：战争正当化 = -10%\n移除：不可手动移除\n描述：舰队整训正在提升国家动员能力。",
        &["决议", "民族精神"],
    );

    let changed = apply_feature_cards_to_mod(&root, &cards, "ITA", "ita_reform").unwrap();
    let changed_again = apply_feature_cards_to_mod(&root, &cards, "ITA", "ita_reform").unwrap();

    let decisions = fs::read_to_string(
        root.join("common")
            .join("decisions")
            .join("ita_reform_decisions.txt"),
    )
    .unwrap();
    let categories = fs::read_to_string(
        root.join("common")
            .join("decisions")
            .join("categories")
            .join("ita_reform_categories.txt"),
    )
    .unwrap();
    let ideas = fs::read_to_string(
        root.join("common")
            .join("ideas")
            .join("ita_reform_ideas.txt"),
    )
    .unwrap();
    let loc_path = target_localisation_path(&root, "ITA");
    let loc_bytes = fs::read(&loc_path).unwrap();
    let loc = String::from_utf8_lossy(&loc_bytes);
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(changed.len(), 4);
    assert!(changed_again.is_empty());
    assert!(categories.contains("ita_reform_category_0"));
    assert!(decisions.contains("ita_reform_decision_0"));
    assert!(decisions.contains("cost = 50"));
    assert!(decisions.contains("days_remove = 30"));
    assert!(decisions.contains("has_war = yes"));
    assert!(decisions.contains("navy_experience = 25"));
    assert!(decisions.contains("type = arms_factory level = 3"));
    assert!(ideas.contains("ita_reform_spirit_1_idea"));
    assert!(ideas.contains("stability_factor = 0.05"));
    assert!(ideas.contains("war_support_factor = 0.02"));
    assert!(ideas.contains("justify_war_goal_time = -0.1"));
    assert!(ideas.contains("removal_cost = -1"));
    assert!(loc_bytes.starts_with(&[0xef, 0xbb, 0xbf]));
    assert!(loc.contains("ita_reform_decision_0:0 \"整训舰队\""));
    assert!(loc.contains("ita_reform_spirit_1_idea:0 \"舰队整训\""));
}

#[test]
fn apply_feature_cards_reuses_existing_decision_category() {
    let root = unique_temp_dir("apply-feature-cards-existing-category");
    let category_dir = root.join("common").join("decisions").join("categories");
    let decision_dir = root.join("common").join("decisions");
    let loc_dir = root.join("localisation").join("simp_chinese");
    fs::create_dir_all(&category_dir).unwrap();
    fs::create_dir_all(&decision_dir).unwrap();
    fs::create_dir_all(&loc_dir).unwrap();
    fs::write(
        category_dir.join("ita_categories.txt"),
        "ita_navy_category = {\n\tallowed = { original_tag = ITA }\n\tvisible = { tag = ITA }\n\tvisible_when_empty = yes\n}\n",
    )
    .unwrap();
    let existing_decisions_path = decision_dir.join("ita_existing_decisions.txt");
    fs::write(
        &existing_decisions_path,
        "ita_navy_category = {\n\tITA_old_decision = {\n\t\ticon = generic_political_discourse\n\t}\n}\n",
    )
    .unwrap();
    fs::write(
        loc_dir.join("ita_l_simp_chinese.yml"),
        "\u{feff}l_simp_chinese:\n  ita_navy_category:0 \"海军改革\"\n",
    )
    .unwrap();
    let cards = parse_cards(
        "决议：整训舰队\n目标：ITA\n分类：海军改革\n花费：50政治点\n效果：海军经验+25",
        &["决议"],
    );

    let changed = apply_feature_cards_to_mod(&root, &cards, "ITA", "ita_reform").unwrap();
    let changed_again = apply_feature_cards_to_mod(&root, &cards, "ITA", "ita_reform").unwrap();

    let existing_decisions = fs::read_to_string(&existing_decisions_path).unwrap();
    let generated_categories = category_dir.join("ita_reform_categories.txt").exists();
    let generated_decisions = decision_dir.join("ita_reform_decisions.txt").exists();
    let loc = String::from_utf8_lossy(&fs::read(target_localisation_path(&root, "ITA")).unwrap())
        .to_string();
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(changed.len(), 2);
    assert!(changed.contains(&existing_decisions_path));
    assert!(changed_again.is_empty());
    assert!(!generated_categories);
    assert!(!generated_decisions);
    assert_eq!(
        existing_decisions.matches("ita_navy_category = {").count(),
        1
    );
    assert!(existing_decisions.contains("ITA_old_decision"));
    assert!(existing_decisions.contains("ita_reform_decision_0"));
    assert!(existing_decisions.contains("navy_experience = 25"));
    assert!(loc.contains("ita_reform_decision_0:0 \"整训舰队\""));
}

#[test]
fn apply_feature_cards_reuses_target_idea_file() {
    let root = unique_temp_dir("apply-feature-cards-existing-idea-file");
    let ideas_dir = root.join("common").join("ideas");
    fs::create_dir_all(&ideas_dir).unwrap();
    let cpc_ideas = ideas_dir.join("cpc.txt");
    fs::write(
        &cpc_ideas,
        "ideas = {\n\tcountry = {\n\t\tCPC_old_spirit = {\n\t\t\tpicture = GFX_idea_old\n\t\t}\n\t}\n}\n",
    )
    .unwrap();
    fs::write(
        ideas_dir.join("_Ministers_ideas.txt"),
        "ideas = { country = { CPC_minister_idea = { picture = GFX_idea_bad } } }\n",
    )
    .unwrap();
    let cards = parse_cards(
        "民族精神：工人自治委员会\n目标：CPC\n效果：稳定度+5%\n移除：不可手动移除",
        &["决议", "民族精神"],
    );

    let changed = apply_feature_cards_to_mod(&root, &cards, "CPC", "cpc_reform").unwrap();
    let changed_again = apply_feature_cards_to_mod(&root, &cards, "CPC", "cpc_reform").unwrap();

    let cpc_text = fs::read_to_string(&cpc_ideas).unwrap();
    let ministers_text = fs::read_to_string(ideas_dir.join("_Ministers_ideas.txt")).unwrap();
    let generated_ideas = ideas_dir.join("cpc_reform_ideas.txt").exists();
    let loc = String::from_utf8_lossy(&fs::read(target_localisation_path(&root, "CPC")).unwrap())
        .to_string();
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(changed.len(), 2);
    assert!(changed.contains(&cpc_ideas));
    assert!(changed_again.is_empty());
    assert!(!generated_ideas);
    assert!(cpc_text.contains("CPC_old_spirit"));
    assert!(cpc_text.contains("cpc_reform_spirit_0_idea"));
    assert!(cpc_text.contains("stability_factor = 0.05"));
    assert!(cpc_text.contains("removal_cost = -1"));
    assert!(!ministers_text.contains("cpc_reform_spirit_0_idea"));
    assert!(loc.contains("cpc_reform_spirit_0_idea:0 \"工人自治委员会\""));
}

#[test]
fn feature_cards_parse_technology_and_gui_cards() {
    let json = parse_decision_idea_cards_json(
        "独有科技：铁路调度算法\n目标：FER\n分类：engineering\n描述：铁路调度进入新的自动化阶段。\n\n特殊GUI：铁路运力面板\n目标：FER\n用途：显示铁路运力、军列占用和瓶颈州。",
        "FER",
        "fer_rail",
    );

    assert!(json.contains("\"type\": \"technology\""));
    assert!(json.contains("\"id\": \"fer_rail_technology_0_tech\""));
    assert!(json.contains("common/technologies/fer_rail_technologies.txt"));
    assert!(json.contains("\"type\": \"gui\""));
    assert!(json.contains("\"id\": \"fer_rail_gui_1_gui\""));
    assert!(json.contains("common/scripted_guis/fer_rail_scripted_guis.txt"));
    assert!(json.contains("interface/fer_rail.gui"));
}

#[test]
fn apply_feature_cards_writes_technology_and_gui_skeletons() {
    let root = unique_temp_dir("apply-feature-cards-tech-gui");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("descriptor.mod"),
        "name=\"Tech GUI Test\"\nsupported_version=\"*\"\n",
    )
    .unwrap();
    let cards = parse_cards(
        "独有科技：铁路调度算法\n目标：FER\n分类：engineering\n年份：1938\n研究花费：2\n效果：补给效率+5%\n描述：铁路调度进入新的自动化阶段。\n\n特殊GUI：铁路运力面板\n目标：FER\n用途：显示铁路运力、军列占用和瓶颈州。",
        FEATURE_CARD_HEADERS,
    );

    let changed = apply_feature_cards_to_mod(&root, &cards, "FER", "fer_rail").unwrap();
    let changed_again = apply_feature_cards_to_mod(&root, &cards, "FER", "fer_rail").unwrap();
    let validation = validate_mod(&root, None).unwrap();

    let tech = fs::read_to_string(
        root.join("common")
            .join("technologies")
            .join("fer_rail_technologies.txt"),
    )
    .unwrap();
    let scripted_gui = fs::read_to_string(
        root.join("common")
            .join("scripted_guis")
            .join("fer_rail_scripted_guis.txt"),
    )
    .unwrap();
    let gui = fs::read_to_string(root.join("interface").join("fer_rail.gui")).unwrap();
    let loc = String::from_utf8_lossy(&fs::read(target_localisation_path(&root, "FER")).unwrap())
        .to_string();
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(changed.len(), 4);
    assert!(changed_again.is_empty());
    assert!(validation.errors.is_empty());
    assert!(tech.contains("technologies = {"));
    assert!(tech.contains("fer_rail_technology_0_tech = {"));
    assert!(tech.contains("research_cost = 2"));
    assert!(tech.contains("start_year = 1938"));
    assert!(tech.contains("engineering"));
    assert!(scripted_gui.contains("scripted_gui = {"));
    assert!(scripted_gui.contains("fer_rail_gui_1_gui = {"));
    assert!(scripted_gui.contains("window_name = \"fer_rail_gui_1_gui_window\""));
    assert!(scripted_gui.contains("tag = FER"));
    assert!(gui.contains("guiTypes = {"));
    assert!(gui.contains("containerWindowType = {"));
    assert!(gui.contains("name = \"fer_rail_gui_1_gui_window\""));
    assert!(loc.contains("fer_rail_technology_0_tech:0 \"铁路调度算法\""));
    assert!(loc.contains("fer_rail_gui_1_gui:0 \"铁路运力面板\""));
}

#[test]
fn feature_cards_parse_scripted_effect_and_trigger_cards() {
    let json = parse_decision_idea_cards_json(
        "脚本效果：铁路瓶颈修复\n范围：州\n效果：军工+2\n\n脚本触发：战时铁路管制可用\n条件：战争中",
        "FER",
        "fer_rail",
    );

    assert!(json.contains("\"type\": \"scripted_effect\""));
    assert!(json.contains("\"id\": \"fer_rail_scripted_effect_0_effect\""));
    assert!(json.contains("common/scripted_effects/fer_rail_scripted_effects.txt"));
    assert!(json.contains("\"kind\": \"scripted_effect\""));
    assert!(json.contains(
        "add_building_construction = { type = arms_factory level = 2 instant_build = yes }"
    ));
    assert!(json.contains("\"type\": \"scripted_trigger\""));
    assert!(json.contains("\"id\": \"fer_rail_scripted_trigger_1_trigger\""));
    assert!(json.contains("common/scripted_triggers/fer_rail_scripted_triggers.txt"));
    assert!(json.contains("\"kind\": \"scripted_trigger\""));
    assert!(json.contains("has_war = yes"));
}

#[test]
fn apply_feature_cards_writes_scripted_helpers() {
    let root = unique_temp_dir("apply-feature-cards-scripted-helpers");
    fs::create_dir_all(&root).unwrap();
    let cards = parse_cards(
        "脚本效果：铁路瓶颈修复\n范围：州\n效果：军工+2\n\n脚本触发：战时铁路管制可用\n条件：战争中",
        FEATURE_CARD_HEADERS,
    );

    let changed = apply_feature_cards_to_mod(&root, &cards, "FER", "fer_rail").unwrap();
    let changed_again = apply_feature_cards_to_mod(&root, &cards, "FER", "fer_rail").unwrap();

    let scripted_effects = fs::read_to_string(
        root.join("common")
            .join("scripted_effects")
            .join("fer_rail_scripted_effects.txt"),
    )
    .unwrap();
    let scripted_triggers = fs::read_to_string(
        root.join("common")
            .join("scripted_triggers")
            .join("fer_rail_scripted_triggers.txt"),
    )
    .unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(changed.len(), 2);
    assert!(changed_again.is_empty());
    assert!(scripted_effects.contains("fer_rail_scripted_effect_0_effect = {"));
    assert!(scripted_effects.contains("# scope = state"));
    assert!(scripted_effects.contains("type = arms_factory level = 2"));
    assert!(!scripted_effects.contains("random_owned_controlled_state"));
    assert!(scripted_triggers.contains("fer_rail_scripted_trigger_1_trigger = {"));
    assert!(scripted_triggers.contains("has_war = yes"));
}

#[test]
fn feature_cards_parse_state_effect_cards() {
    let json = parse_decision_idea_cards_json(
        "州效果：莫斯科工业修复\n州ID：64\n目标：FER\n建筑：军工+2，基础设施+1\n资源：钢+8，铝+2\n核心：FER",
        "FER",
        "fer_rail",
    );

    assert!(json.contains("\"type\": \"state_effect\""));
    assert!(json.contains("\"id\": \"fer_rail_state_effect_0_state_effect\""));
    assert!(json.contains("common/scripted_effects/fer_rail_state_effects.txt"));
    assert!(json.contains("64 = {"));
    assert!(json.contains("type = arms_factory level = 2"));
    assert!(json.contains("type = infrastructure level = 1"));
    assert!(json.contains("add_resource = { type = steel amount = 8 }"));
    assert!(json.contains("add_resource = { type = aluminium amount = 2 }"));
    assert!(json.contains("add_core_of = FER"));
}

#[test]
fn apply_feature_cards_writes_state_effect_helpers() {
    let root = unique_temp_dir("apply-feature-cards-state-effects");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("descriptor.mod"),
        "name=\"State Effect Test\"\nsupported_version=\"*\"\n",
    )
    .unwrap();
    let cards = parse_cards(
        "州效果：莫斯科工业修复\n州ID：64\n目标：FER\n建筑：军工+2，基础设施+1\n资源：钢+8，铝+2\n核心：FER",
        FEATURE_CARD_HEADERS,
    );

    let changed = apply_feature_cards_to_mod(&root, &cards, "FER", "fer_rail").unwrap();
    let changed_again = apply_feature_cards_to_mod(&root, &cards, "FER", "fer_rail").unwrap();
    let validation = validate_mod(&root, None).unwrap();
    let state_effects = fs::read_to_string(
        root.join("common")
            .join("scripted_effects")
            .join("fer_rail_state_effects.txt"),
    )
    .unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(changed.len(), 1);
    assert!(changed_again.is_empty());
    assert!(validation.errors.is_empty());
    assert!(state_effects.contains("fer_rail_state_effect_0_state_effect = {"));
    assert!(state_effects.contains("# state_id = 64"));
    assert!(state_effects.contains("\t64 = {"));
    assert!(state_effects.contains("type = arms_factory level = 2"));
    assert!(state_effects.contains("type = infrastructure level = 1"));
    assert!(state_effects.contains("add_resource = { type = steel amount = 8 }"));
    assert!(state_effects.contains("add_resource = { type = aluminium amount = 2 }"));
    assert!(state_effects.contains("add_core_of = FER"));
}

#[test]
fn event_cards_include_hidden_effects_and_ai_chance() {
    let json = parse_event_cards_json(
        "事件：新经济政策的未来\n类型：新闻事件\n选项A：继续试验\n效果A：政治点+50\n隐藏效果A：设置旗标 nep_hidden\nAI权重A：75",
        "SOV",
        "sov_nep",
    );

    assert!(json.contains("\"hidden_effects\": \"设置旗标 nep_hidden\""));
    assert!(json.contains("\"ai_chance\": \"75\""));
    assert!(json.contains("\"kind\": \"event_script\""));
    assert!(json.contains("add_namespace = sov_nep\\n\\nnews_event ="));
}

#[test]
fn parse_event_cards_json_marks_unresolved_suggestions_as_blocked() {
    let json = parse_event_cards_json(
        "事件：未知动员\n目标：SOV\n命名空间：sov_ai\n触发：神秘局势\n选项A：继续\n效果A：外星能量+50",
        "SOV",
        "sov_ai",
    );

    assert!(json.contains("\"safety\": {\"status\": \"blocked\""));
    assert!(json.contains("\"final_code_allowed\": false"));
    assert!(json.contains("\"requires_mapping\": true"));
    assert!(json.contains("raw_trigger `神秘局势` must be mapped"));
    assert!(json.contains("raw_effect `外星能量+50` must be mapped"));
}

#[test]
fn event_cards_number_ids_inside_each_namespace() {
    let json = parse_event_cards_json(
        "事件：铁路会议\n命名空间：fer_rail\n选项A：通过\n\n事件：边境新闻\n类型：新闻事件\n命名空间：fer_news\n选项A：知道了\n\n事件：铁路复会\n命名空间：fer_rail\n选项A：继续",
        "FER",
        "fer_rail",
    );

    assert!(json.contains("\"event_id\": \"fer_rail.1\""));
    assert!(json.contains("\"event_id\": \"fer_news.1\""));
    assert!(json.contains("\"event_id\": \"fer_rail.2\""));
    assert!(!json.contains("\"event_id\": \"fer_news.2\""));
}

#[test]
fn apply_event_cards_writes_events_and_localisation() {
    let root = unique_temp_dir("apply-event-cards");
    fs::create_dir_all(&root).unwrap();
    let cards = parse_cards(
        "事件：新经济政策的未来\n类型：新闻事件\n目标：SOV\n命名空间：sov_nep\n标题：新经济政策的未来\n描述：党内围绕新经济政策展开了激烈争论。\n图片：GFX_report_event_generic\n触发：战争中\n选项A：继续试验\n效果A：政治点+50，稳定度+2%\n隐藏效果A：设置旗标 nep_hidden\nAI权重A：75\n选项B：回到计划经济\n效果B：稳定度-5%，设置旗标 end_nep",
        &["事件"],
    );

    let changed = apply_event_cards_to_mod(&root, &cards, "SOV", "sov_nep").unwrap();
    let changed_again = apply_event_cards_to_mod(&root, &cards, "SOV", "sov_nep").unwrap();

    let events = fs::read_to_string(root.join("events").join("sov_nep_events.txt")).unwrap();
    let loc_path = target_localisation_path(&root, "SOV");
    let loc_bytes = fs::read(&loc_path).unwrap();
    let loc = String::from_utf8_lossy(&loc_bytes);
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(changed.len(), 2);
    assert!(changed_again.is_empty());
    assert!(events.contains("add_namespace = sov_nep"));
    assert!(events.contains("news_event = {"));
    assert!(events.contains("id = sov_nep.1"));
    assert!(events.contains("title = sov_nep.1.t"));
    assert!(events.contains("desc = sov_nep.1.d"));
    assert!(events.contains("is_triggered_only = yes"));
    assert!(events.contains("trigger = {"));
    assert!(events.contains("has_war = yes"));
    assert!(events.contains("name = sov_nep.1.a"));
    assert!(events.contains("add_political_power = 50"));
    assert!(events.contains("add_stability = 0.02"));
    assert!(events.contains("hidden_effect = {"));
    assert!(events.contains("set_country_flag = nep_hidden"));
    assert!(events.contains("ai_chance = {"));
    assert!(events.contains("factor = 75"));
    assert!(events.contains("name = sov_nep.1.b"));
    assert!(events.contains("set_country_flag = end_nep"));
    assert!(loc_bytes.starts_with(&[0xef, 0xbb, 0xbf]));
    assert!(loc.contains("sov_nep.1.t:0 \"新经济政策的未来\""));
    assert!(loc.contains("sov_nep.1.d:0 \"党内围绕新经济政策展开了激烈争论。\""));
    assert!(loc.contains("sov_nep.1.a:0 \"继续试验\""));
    assert!(loc.contains("sov_nep.1.b:0 \"回到计划经济\""));
}

#[test]
fn apply_event_cards_uses_indexed_semantic_event_picture() {
    let root = unique_temp_dir("apply-event-cards-indexed-picture");
    let game = unique_temp_dir("apply-event-cards-game-picture");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(game.join("interface")).unwrap();
    fs::write(
        game.join("interface").join("eventpictures.gfx"),
        r#"spriteType = { name = "GFX_report_event_generic" texturefile = "gfx/event_pictures/generic.dds" }
spriteType = { name = "GFX_report_event_soviet_workers_revolution" texturefile = "gfx/event_pictures/workers.dds" }"#,
    )
    .unwrap();
    let index = build_game_index(&game).unwrap();
    let cards = parse_cards(
        "事件：工人革命胜利\n类型：新闻事件\n命名空间：sov_news\n选项A：万岁",
        &["事件"],
    );

    apply_event_cards_to_mod_with_index(&root, &cards, "SOV", "sov_news", Some(&index)).unwrap();

    let events = fs::read_to_string(root.join("events").join("sov_news_events.txt")).unwrap();
    fs::remove_dir_all(&root).unwrap();
    fs::remove_dir_all(&game).unwrap();

    assert!(index
        .event_pictures
        .contains("GFX_report_event_soviet_workers_revolution"));
    assert!(events.contains("picture = GFX_report_event_soviet_workers_revolution"));
}

#[test]
fn apply_event_cards_continues_existing_namespace_numbers() {
    let root = unique_temp_dir("apply-event-cards-existing-namespace");
    let events_dir = root.join("events");
    fs::create_dir_all(&events_dir).unwrap();
    let existing_path = events_dir.join("sov_existing_events.txt");
    fs::write(
        &existing_path,
        "add_namespace = sov_nep\n\ncountry_event = { id = sov_nep.17 title = sov_nep.17.t desc = sov_nep.17.d option = { name = sov_nep.17.a } }\n",
    )
    .unwrap();
    let cards = parse_cards(
        "事件：新经济政策的未来\n类型：国家事件\n命名空间：sov_nep\n标题：新经济政策的未来\n描述：党内围绕新经济政策展开了争论。\n选项A：继续试验\n效果A：政治点+50",
        &["事件"],
    );

    let changed = apply_event_cards_to_mod(&root, &cards, "SOV", "sov_nep").unwrap();
    let changed_again = apply_event_cards_to_mod(&root, &cards, "SOV", "sov_nep").unwrap();

    let events = fs::read_to_string(&existing_path).unwrap();
    let new_file = root.join("events").join("sov_nep_events.txt");
    let new_file_exists = new_file.exists();
    let loc = String::from_utf8_lossy(&fs::read(target_localisation_path(&root, "SOV")).unwrap())
        .to_string();
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(changed.len(), 2);
    assert!(changed.contains(&existing_path));
    assert!(changed_again.is_empty());
    assert!(!new_file_exists);
    assert_eq!(events.matches("add_namespace = sov_nep").count(), 1);
    assert!(events.contains("id = sov_nep.17"));
    assert!(events.contains("id = sov_nep.18"));
    assert!(events.contains("# hoi4skill_card = ev_"));
    assert!(loc.contains("sov_nep.18.t:0 \"新经济政策的未来\""));
    assert!(loc.contains("sov_nep.18.a:0 \"继续试验\""));
}

#[test]
fn apply_focus_layout_writes_focus_tree_and_localisation() {
    let root = unique_temp_dir("apply-focus-layout");
    fs::create_dir_all(&root).unwrap();
    let layout = parse_focus_layout(
        "斯大林宪法\n第一个五年计划   互斥       继续新经济政策\n快速工业化  强化国家       发财吧农民   奈普曼入党\n",
        "SOV",
        "sov_alt",
    );

    let changed = apply_focus_layout_to_mod(&root, &layout, "SOV", "sov_alt").unwrap();
    let changed_again = apply_focus_layout_to_mod(&root, &layout, "SOV", "sov_alt").unwrap();

    let focus_file = fs::read_to_string(
        root.join("common")
            .join("national_focus")
            .join("sov_alt_SOV_focus.txt"),
    )
    .unwrap();
    let loc_path = target_localisation_path(&root, "SOV");
    let loc_bytes = fs::read(&loc_path).unwrap();
    let loc = String::from_utf8_lossy(&loc_bytes);
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(changed.len(), 2);
    assert!(changed_again.is_empty());
    assert!(focus_file.contains("focus_tree = {"));
    assert!(focus_file.contains("id = sov_alt_SOV_focus_tree"));
    assert!(focus_file.contains("tag = SOV"));
    assert!(focus_file.contains("focus = {"));
    assert!(focus_file.contains("id = SOV_first_five_year_plan"));
    assert!(focus_file.contains("prerequisite = { focus = SOV_stalin_constitution }"));
    assert!(focus_file.contains("cost = 10"));
    assert!(focus_file.contains("ai_will_do = {\n\t\t\tfactor = 100\n\t\t}"));
    assert!(focus_file.contains("available = {\n\t\t}"));
    assert!(focus_file.contains("bypass = {\n\t\t}"));
    assert!(focus_file.contains("cancel_if_invalid = yes"));
    assert!(focus_file.contains("continue_if_invalid = no"));
    assert!(focus_file.contains("available_if_capitulated = no"));
    assert!(
        focus_file.contains("mutually_exclusive = { focus = SOV_continue_new_economic_policy }")
    );
    assert!(focus_file.contains("completion_reward = {"));
    assert!(loc_bytes.starts_with(&[0xef, 0xbb, 0xbf]));
    assert!(loc.contains("SOV_first_five_year_plan:0 \"第一个五年计划\""));
    assert!(loc.contains(
        "SOV_first_five_year_plan_desc:0 \"分散的工厂、铁路与计划机关无法独自承担时代的重压。"
    ));
    assert!(!loc.contains("具体效果待补充"));
}

#[test]
fn strict_focus_layout_requires_game_root_before_write() {
    let root = unique_temp_dir("strict-focus-layout-requires-game-root");
    fs::create_dir_all(&root).unwrap();
    let input = root.join("layout.txt");
    fs::write(&input, "工业复兴\n").unwrap();

    let err = cmd_apply_focus_layout(&[
        "--input".to_string(),
        input.display().to_string(),
        "--mod-root".to_string(),
        root.display().to_string(),
        "--tag".to_string(),
        "SOV".to_string(),
        "--prefix".to_string(),
        "sov_test".to_string(),
        "--final-check".to_string(),
    ])
    .unwrap_err();
    let common_exists = root.join("common").exists();
    fs::remove_dir_all(&root).unwrap();

    assert!(err.contains("strict focus layout generation requires --game-root"));
    assert!(!common_exists);
}

#[test]
fn strict_focus_layout_gate_blocks_unresolved_reward_before_write() {
    let root = unique_temp_dir("strict-focus-layout-prewrite");
    let game = unique_temp_dir("strict-focus-layout-game");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(game.join("common").join("country_tags")).unwrap();
    fs::create_dir_all(game.join("interface")).unwrap();
    fs::write(
        game.join("common")
            .join("country_tags")
            .join("00_countries.txt"),
        "SOV = \"countries/Soviet.txt\"\n",
    )
    .unwrap();
    fs::write(
        game.join("interface").join("goals.gfx"),
        r#"spriteType = { name = "GFX_goal_industrial_revival" texturefile = "gfx/interface/goals/industry.dds" }"#,
    )
    .unwrap();
    let input = root.join("layout.txt");
    fs::write(&input, "工业复兴\n# completion_reward: 外星能量+50\n").unwrap();

    let err = cmd_apply_focus_layout(&[
        "--input".to_string(),
        input.display().to_string(),
        "--mod-root".to_string(),
        root.display().to_string(),
        "--tag".to_string(),
        "SOV".to_string(),
        "--prefix".to_string(),
        "sov_test".to_string(),
        "--game-root".to_string(),
        game.display().to_string(),
        "--final-check".to_string(),
    ])
    .unwrap_err();
    let common_exists = root.join("common").exists();
    fs::remove_dir_all(&root).unwrap();
    fs::remove_dir_all(&game).unwrap();

    assert!(err.contains("strict focus layout generation blocked unresolved AI mappings"));
    assert!(err.contains("外星能量+50"));
    assert!(!common_exists);
}

#[test]
fn strict_focus_layout_gate_blocks_unindexed_reward_effect_before_write() {
    let root = unique_temp_dir("strict-focus-layout-unindexed-reward");
    fs::create_dir_all(&root).unwrap();
    let mut index = GameIndex::default();
    index
        .focus_goal_sprites
        .insert("GFX_goal_industry".to_string());
    index.effects.insert("add_stability".to_string());
    index
        .effects
        .insert("add_scaled_political_power".to_string());
    let layout = FocusLayout {
        tree_id: "kor_ai_focus_tree".to_string(),
        rows: Vec::new(),
        mutuals: Vec::new(),
        focuses: vec![FocusNode {
            title: "工业复兴".to_string(),
            id: "KOR_industrial_revival".to_string(),
            icon: Some("GFX_goal_industry".to_string()),
            x: 0,
            y: 0,
            relative_position_id: None,
            relative_x: None,
            relative_y: None,
            row: 0,
            column: 0,
            prerequisite: Vec::new(),
            mutually_exclusive: Vec::new(),
            completion_reward: vec!["add_political_power = 50".to_string()],
        }],
    };

    let err = enforce_strict_focus_layout_gate_with_options(
        ValidationOptions {
            strict_code_index: true,
        },
        &root,
        &layout,
        "KOR",
        Some(&index),
    )
    .unwrap_err();
    fs::remove_dir_all(&root).unwrap();

    assert!(err.contains("strict focus layout generation blocked unresolved AI mappings"));
    assert!(err.contains("unindexed effect `add_political_power`"));
    assert!(err.contains("check-code-symbol --kind effect"));
    assert!(err.contains("related indexed code: effects/effect `add_scaled_political_power`"));
}

#[test]
fn strict_focus_layout_gate_blocks_missing_effect_index_category_before_write() {
    let root = unique_temp_dir("strict-focus-layout-missing-effect-category");
    fs::create_dir_all(&root).unwrap();
    let mut index = GameIndex::default();
    index
        .focus_goal_sprites
        .insert("GFX_goal_industry".to_string());
    let layout = FocusLayout {
        tree_id: "kor_ai_focus_tree".to_string(),
        rows: Vec::new(),
        mutuals: Vec::new(),
        focuses: vec![FocusNode {
            title: "工业复兴".to_string(),
            id: "KOR_industrial_revival".to_string(),
            icon: Some("GFX_goal_industry".to_string()),
            x: 0,
            y: 0,
            relative_position_id: None,
            relative_x: None,
            relative_y: None,
            row: 0,
            column: 0,
            prerequisite: Vec::new(),
            mutually_exclusive: Vec::new(),
            completion_reward: vec!["add_political_power = 50".to_string()],
        }],
    };

    let err = enforce_strict_focus_layout_gate_with_options(
        ValidationOptions {
            strict_code_index: true,
        },
        &root,
        &layout,
        "KOR",
        Some(&index),
    )
    .unwrap_err();
    fs::remove_dir_all(&root).unwrap();

    assert!(err.contains("strict focus layout generation blocked unresolved AI mappings"));
    assert!(err.contains("strict code index has no indexed effects"));
    assert!(err.contains("documentation/effects_documentation.md"));
}

#[test]
fn strict_parse_focus_layout_requires_game_root_before_output() {
    let root = unique_temp_dir("strict-parse-focus-requires-game-root");
    fs::create_dir_all(&root).unwrap();
    let input = root.join("layout.txt");
    let output = root.join("layout.json");
    fs::write(&input, "工业复兴\n").unwrap();

    let err = cmd_parse_focus_layout(&[
        "--input".to_string(),
        input.display().to_string(),
        "--tag".to_string(),
        "KOR".to_string(),
        "--prefix".to_string(),
        "kor_ai".to_string(),
        "--strict-code-index".to_string(),
        "--output".to_string(),
        output.display().to_string(),
    ])
    .unwrap_err();
    let output_exists = output.exists();
    fs::remove_dir_all(&root).unwrap();

    assert!(err.contains("strict focus layout generation requires --game-root"));
    assert!(!output_exists);
}

#[test]
fn strict_render_focus_code_blocks_unindexed_reward_effect_before_output() {
    let root = unique_temp_dir("strict-render-focus-code");
    let game = root.join("game");
    fs::create_dir_all(game.join("common").join("country_tags")).unwrap();
    fs::create_dir_all(game.join("documentation")).unwrap();
    fs::create_dir_all(game.join("interface")).unwrap();
    fs::write(
        game.join("common")
            .join("country_tags")
            .join("00_countries.txt"),
        "KOR = \"countries/Korea.txt\"\n",
    )
    .unwrap();
    fs::write(
        game.join("documentation").join("effects_documentation.md"),
        "## Effects\n\n## add_stability\n",
    )
    .unwrap();
    fs::write(
        game.join("interface").join("goals.gfx"),
        r#"spriteType = { name = "GFX_goal_industrial_revival" texturefile = "gfx/interface/goals/industry.dds" }"#,
    )
    .unwrap();
    let input = root.join("layout.txt");
    let output = root.join("focus.txt");
    fs::write(&input, "工业复兴\n# completion_reward: 政治点+50\n").unwrap();

    let err = cmd_render_focus_code(&[
        "--input".to_string(),
        input.display().to_string(),
        "--tag".to_string(),
        "KOR".to_string(),
        "--prefix".to_string(),
        "kor_ai".to_string(),
        "--game-root".to_string(),
        game.display().to_string(),
        "--strict-code-index".to_string(),
        "--output".to_string(),
        output.display().to_string(),
    ])
    .unwrap_err();
    let output_exists = output.exists();
    fs::remove_dir_all(&root).unwrap();

    assert!(err.contains("strict focus layout generation blocked unresolved AI mappings"));
    assert!(err.contains("unindexed effect `add_political_power`"));
    assert!(!output_exists);
}

#[test]
fn focus_excel_reads_drawn_tree_and_renders_standard_skeleton() {
    let root = unique_temp_dir("focus-excel");
    fs::create_dir_all(&root).unwrap();
    let xlsx = root.join("tree.xlsx");
    write_minimal_focus_xlsx(&xlsx);

    let layout = read_focus_excel_layout(&xlsx, Some("FocusTree"), "SOV", "sov_excel").unwrap();
    let tree = render_focus_tree(&layout, "SOV");
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(layout.focuses.len(), 3);
    assert!(layout
        .focuses
        .iter()
        .any(|focus| focus.id == "SOV_rebuild_committee" && focus.x == 2));
    let industry = layout
        .focuses
        .iter()
        .find(|focus| focus.title == "工业复兴")
        .unwrap();
    assert_eq!(industry.id, "SOV_industrial_revival");
    assert_eq!(industry.x, 0);
    assert_eq!(industry.relative_x, Some(-2));
    assert_eq!(industry.relative_y, Some(2));
    assert_eq!(
        industry.relative_position_id.as_deref(),
        Some("SOV_rebuild_committee")
    );
    let army = layout
        .focuses
        .iter()
        .find(|focus| focus.title == "整顿军队")
        .unwrap();
    assert_eq!(army.x, 4);
    assert_eq!(army.relative_x, Some(2));

    assert!(tree.contains("id = SOV_industrial_revival"));
    assert!(tree.contains("icon = GFX_goal_generic_construct_civ_factory"));
    assert!(tree.contains("x = -2"));
    assert!(tree.contains("relative_position_id = SOV_rebuild_committee"));
    assert!(tree.contains("cost = 10"));
    assert!(tree.contains("factor = 100"));
    assert!(tree.contains("available = {\n\t\t}"));
    assert!(tree.contains("bypass = {\n\t\t}"));
    assert!(tree.contains("cancel_if_invalid = yes"));
    assert!(tree.contains("continue_if_invalid = no"));
    assert!(tree.contains("available_if_capitulated = no"));
    assert!(tree.contains("arms_factory"));
}

#[test]
fn strict_parse_focus_excel_requires_game_root_before_output() {
    let root = unique_temp_dir("strict-parse-focus-excel-root");
    fs::create_dir_all(&root).unwrap();
    let xlsx = root.join("tree.xlsx");
    let output = root.join("focus_review.md");
    write_minimal_focus_xlsx(&xlsx);

    let err = cmd_parse_focus_excel(&[
        "--input".to_string(),
        xlsx.display().to_string(),
        "--tag".to_string(),
        "SOV".to_string(),
        "--prefix".to_string(),
        "sov_excel".to_string(),
        "--strict-code-index".to_string(),
        "--output".to_string(),
        output.display().to_string(),
    ])
    .unwrap_err();
    let output_exists = output.exists();
    fs::remove_dir_all(&root).unwrap();

    assert!(err.contains("strict focus layout generation requires --game-root"));
    assert!(!output_exists);
}

#[test]
fn strict_parse_focus_excel_blocks_unindexed_reward_effect_before_output() {
    let root = unique_temp_dir("strict-parse-focus-excel-reward");
    let game = root.join("game");
    fs::create_dir_all(game.join("common").join("country_tags")).unwrap();
    fs::create_dir_all(game.join("documentation")).unwrap();
    fs::create_dir_all(game.join("interface")).unwrap();
    fs::write(
        game.join("common")
            .join("country_tags")
            .join("00_countries.txt"),
        "SOV = \"countries/Soviet.txt\"\n",
    )
    .unwrap();
    fs::write(
        game.join("documentation").join("effects_documentation.md"),
        "## Effects\n\n## add_stability\n",
    )
    .unwrap();
    fs::write(
        game.join("interface").join("goals.gfx"),
        r#"spriteType = { name = "GFX_goal_generic_construct_civ_factory" texturefile = "gfx/interface/goals/factory.dds" }
spriteType = { name = "GFX_goal_army_effort" texturefile = "gfx/interface/goals/army.dds" }
spriteType = { name = "GFX_goal_generic_political_pressure" texturefile = "gfx/interface/goals/politics.dds" }"#,
    )
    .unwrap();
    let xlsx = root.join("tree.xlsx");
    let output = root.join("focus_tree.txt");
    write_minimal_focus_xlsx(&xlsx);

    let err = cmd_parse_focus_excel(&[
        "--input".to_string(),
        xlsx.display().to_string(),
        "--tag".to_string(),
        "SOV".to_string(),
        "--prefix".to_string(),
        "sov_excel".to_string(),
        "--format".to_string(),
        "focus-tree".to_string(),
        "--game-root".to_string(),
        game.display().to_string(),
        "--strict-code-index".to_string(),
        "--output".to_string(),
        output.display().to_string(),
    ])
    .unwrap_err();
    let output_exists = output.exists();
    fs::remove_dir_all(&root).unwrap();

    assert!(err.contains("strict focus layout generation blocked unresolved AI mappings"));
    assert!(err.contains("unindexed effect"));
    assert!(!output_exists);
}

#[test]
fn focus_excel_merges_drawing_chinese_title_with_cell_english_id() {
    let root = unique_temp_dir("focus-excel-drawing-title");
    fs::create_dir_all(&root).unwrap();
    let xlsx = root.join("tree.xlsx");
    write_drawing_title_focus_xlsx(&xlsx);

    let layout = read_focus_excel_layout(&xlsx, Some("FocusTree"), "KOR", "kor_spring").unwrap();
    let json = focus_excel_layout_json(&layout, &xlsx, Some("FocusTree"), "KOR", "kor_spring");
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(layout.focuses.len(), 1);
    assert_eq!(layout.focuses[0].title, "工业复兴");
    assert_eq!(layout.focuses[0].id, "KOR_industry_revival");
    assert!(json.contains("\"title\": \"工业复兴\""));
    assert!(json.contains("\"id\": \"KOR_industry_revival\""));
}

#[test]
fn parse_focus_excel_can_render_markdown_table() {
    let root = unique_temp_dir("focus-excel-markdown");
    fs::create_dir_all(&root).unwrap();
    let xlsx = root.join("tree.xlsx");
    write_minimal_focus_xlsx(&xlsx);

    let markdown =
        render_focus_excel_markdown(&xlsx, Some("FocusTree"), "SOV", "sov_excel").unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(markdown.contains("# Worksheet: FocusTree"));
    assert!(markdown.contains("## Immutable Import Contract"));
    assert!(markdown.contains("- focus_count: 3"));
    assert!(markdown
        .contains("A generated or generic ID never means the worksheet title/content is missing"));
    assert!(markdown.contains("## Original Worksheet Grid"));
    assert!(markdown.contains("## Simulated HOI4 x/y Grid"));
    assert!(markdown.contains("| Row | A | B | C | D |"));
    assert!(markdown.contains("| 1 | 国策树 |  | 重建中央委员会\\|rebuild_committee |  |"));
    assert!(markdown.contains(
        "工业复兴<br>ID: industrial_revival<br>icon: GFX_goal_generic_construct_civ_factory"
    ));
    assert!(markdown.contains("| y\\\\x | 0 | 2 | 4 |"));
    assert!(markdown.contains("重建中央委员会<br><sub>id: SOV_rebuild_committee</sub>"));
}

#[test]
fn parse_focus_excel_default_format_is_model_readable_markdown() {
    assert_eq!(DEFAULT_FOCUS_EXCEL_FORMAT, "markdown");
    assert_eq!(normalise_focus_excel_format("table"), "markdown");
}

#[test]
fn validator_requires_game_index_for_history_files() {
    let root = unique_temp_dir("validate-history-needs-index");
    fs::create_dir_all(root.join("history/states")).unwrap();
    fs::write(
        root.join("descriptor.mod"),
        "name=\"History Index Gate\"\nsupported_version=\"*\"\n",
    )
    .unwrap();
    fs::write(
        root.join("history/states/1-Test.txt"),
        "state = { id = 1 history = { owner = GER } provinces = { 1 } }\n",
    )
    .unwrap();

    let reporter = validate_mod(&root, None).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(reporter.errors.iter().any(|error| {
        error.contains("history files require indexed validation with --game-root")
    }));
}

#[test]
fn new_mod_request_scope_rejects_unrequested_country_and_state_files() {
    let root = unique_temp_dir("validate-new-mod-request-scope");
    fs::create_dir_all(root.join("common/countries")).unwrap();
    fs::create_dir_all(root.join("history/states")).unwrap();
    fs::write(
        root.join("common/countries/KOR.txt"),
        "graphical_culture = asian_gfx\n",
    )
    .unwrap();
    fs::write(
        root.join("history/states/525.txt"),
        "state = { id = 525 history = { owner = KOR } provinces = { 7125 } }\n",
    )
    .unwrap();
    let request = "按照表格生成一个韩国革命mod，事件不少于4个，民族精神不少于5个";
    let mut reporter = Reporter::default();

    check_request_scope_for_new_mod(&root, request, &mut reporter);
    fs::remove_dir_all(&root).unwrap();

    assert!(reporter.errors.iter().any(|error| {
        error.contains("common/countries") && error.contains("country_definition")
    }));
    assert!(reporter
        .errors
        .iter()
        .any(|error| error.contains("history/states") && error.contains("state_history")));
}

#[test]
fn request_scope_audit_is_generic_for_any_existing_country() {
    let root = unique_temp_dir("validate-generic-country-request-scope");
    fs::create_dir_all(root.join("history/countries")).unwrap();
    fs::write(
        root.join("history/countries/GER - Germany.txt"),
        "capital = 6521\n",
    )
    .unwrap();
    let request = "create a Germany mod with focuses and events";
    let mut reporter = Reporter::default();

    check_request_scope_for_new_mod(&root, request, &mut reporter);
    fs::remove_dir_all(&root).unwrap();

    assert!(reporter
        .errors
        .iter()
        .any(|error| error.contains("history/countries") && error.contains("country_history")));
}

#[test]
fn skill_install_doctor_finds_nested_duplicate_copies() {
    let root = unique_temp_dir("skill-install-doctor-scan");
    let keep = root.join(".opencode/skills/hoi4-mod-maker");
    let backup = root.join(".agents/skills/hoi4-mod-maker.backup-v0.2.0");
    let unrelated = root.join(".codex/skills/unrelated");
    write_test_skill(&keep, "hoi4-mod-maker");
    write_test_skill(&backup, "hoi4-mod-maker");
    write_test_skill(&unrelated, "other-skill");

    let found = find_installed_skill_copies(&[
        root.join(".opencode/skills"),
        root.join(".agents/skills"),
        root.join(".codex/skills"),
    ])
    .unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(found.len(), 2);
    assert!(found.iter().any(|path| path.ends_with("hoi4-mod-maker")));
    assert!(found
        .iter()
        .any(|path| path.ends_with("hoi4-mod-maker.backup-v0.2.0")));
}

#[test]
fn skill_install_doctor_fix_keeps_current_and_removes_old_copies() {
    let root = unique_temp_dir("skill-install-doctor-fix");
    let keep = root.join(".opencode/skills/hoi4-mod-maker");
    let old = root.join(".agents/skills/hoi4skill-repo/hoi4-mod-maker");
    write_test_skill(&keep, "hoi4-mod-maker");
    write_test_skill(&old, "hoi4-mod-maker");
    let found =
        find_installed_skill_copies(&[root.join(".opencode/skills"), root.join(".agents/skills")])
            .unwrap();

    let report = repair_installed_skill_copies(&found, Some(&keep), true).unwrap();

    assert_eq!(report.removed.len(), 1);
    assert!(keep.exists());
    assert!(!old.exists());
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn skill_install_doctor_refuses_ambiguous_automatic_deletion() {
    let root = unique_temp_dir("skill-install-doctor-ambiguous");
    let first = root.join(".opencode/skills/hoi4-mod-maker");
    let second = root.join(".agents/skills/hoi4-mod-maker");
    write_test_skill(&first, "hoi4-mod-maker");
    write_test_skill(&second, "hoi4-mod-maker");
    let found =
        find_installed_skill_copies(&[root.join(".opencode/skills"), root.join(".agents/skills")])
            .unwrap();

    let error = repair_installed_skill_copies(&found, None, true).unwrap_err();
    fs::remove_dir_all(&root).unwrap();

    assert!(error.contains("current skill directory could not be inferred"));
}

#[test]
fn clausewitz_library_builds_and_retrieves_real_block_shapes() {
    let root = unique_temp_dir("clausewitz-library-source");
    let library = unique_temp_dir("clausewitz-library-output");
    fs::create_dir_all(root.join("common/national_focus")).unwrap();
    fs::create_dir_all(root.join("events")).unwrap();
    fs::create_dir_all(root.join("common/ideas")).unwrap();
    fs::write(
        root.join("common/national_focus/test.txt"),
        r#"focus_tree = {
	id = test_tree
	country = { factor = 0 modifier = { add = 10 tag = TST } }
	focus = {
		id = TST_workers_revolution
		icon = GFX_goal_generic_communism
		x = 0
		y = 0
		cost = 10
		completion_reward = { add_stability = 0.05 }
	}
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("events/test.txt"),
        r#"add_namespace = test
country_event = {
	id = test.1
	title = test.1.t
	desc = test.1.d
	is_triggered_only = yes
	option = { name = test.1.a add_stability = 0.05 }
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("common/ideas/test.txt"),
        r#"ideas = {
	country = {
		TST_planned_economy = {
			picture = planned_economy
			modifier = { production_speed_industrial_complex_factor = 0.05 }
		}
	}
}
"#,
    )
    .unwrap();

    let count = build_clausewitz_library(std::slice::from_ref(&root), &library).unwrap();
    let focuses =
        query_clausewitz_library(&library, "工人 共产主义 革命", Some("focus"), 3).unwrap();
    let events =
        query_clausewitz_library(&library, "revolution country event", Some("event"), 3).unwrap();
    let ideas =
        query_clausewitz_library(&library, "planned economy national spirit", Some("idea"), 3)
            .unwrap();
    fs::remove_dir_all(&root).unwrap();
    fs::remove_dir_all(&library).unwrap();

    assert!(count >= 4);
    assert!(focuses
        .iter()
        .any(|example| example.code.contains("id = TST_workers_revolution")));
    assert!(events
        .iter()
        .any(|example| example.code.starts_with("country_event = {")));
    assert!(ideas
        .iter()
        .any(|example| example.code.contains("picture = planned_economy")));
}

#[test]
fn clausewitz_library_refuses_to_replace_unrelated_directory() {
    let root = unique_temp_dir("clausewitz-library-safe-source");
    let output = unique_temp_dir("clausewitz-library-unsafe-output");
    fs::create_dir_all(root.join("events")).unwrap();
    fs::create_dir_all(&output).unwrap();
    fs::write(output.join("user-file.txt"), "keep me").unwrap();

    let error = build_clausewitz_library(std::slice::from_ref(&root), &output).unwrap_err();
    assert!(output.join("user-file.txt").is_file());
    fs::remove_dir_all(&root).unwrap();
    fs::remove_dir_all(&output).unwrap();

    assert!(error.contains("refusing to replace non-library directory"));
}

#[test]
fn mod_code_layer_requires_literal_user_authorization() {
    let roots = vec![PathBuf::from("M:\\mods\\example")];
    let denied = enforce_mod_code_request("给德国制作国策", &roots).unwrap_err();

    assert!(denied.contains("mod code loading is forbidden"));
    assert!(enforce_mod_code_request("加载 example 模组代码作为参考", &roots).is_ok());
    assert!(enforce_mod_code_request("reference mod code from example", &roots).is_ok());
}

#[test]
fn dependency_mod_path_does_not_authorize_code_library_loading() {
    let args = vec!["--mod-path".to_string(), "M:\\mods\\dependency".to_string()];
    let map = parse_args(&args);

    assert!(code_mod_roots(&map).unwrap().is_empty());
}

#[test]
fn mod_code_layer_is_separate_and_precedes_vanilla_results() {
    let vanilla_root = unique_temp_dir("clausewitz-vanilla-source");
    let mod_root = unique_temp_dir("clausewitz-mod-source");
    let vanilla_library = unique_temp_dir("clausewitz-vanilla-library");
    let mod_library = unique_temp_dir("clausewitz-mod-library");
    fs::create_dir_all(vanilla_root.join("events")).unwrap();
    fs::create_dir_all(mod_root.join("events")).unwrap();
    fs::write(
        vanilla_root.join("events/base.txt"),
        "country_event = {\n id = base.1\n is_triggered_only = yes\n option = { name = base.1.a }\n}\n",
    )
    .unwrap();
    fs::write(
        mod_root.join("events/mod.txt"),
        "country_event = {\n id = custom_mod.1\n is_triggered_only = yes\n option = { name = custom_mod.1.a }\n}\n",
    )
    .unwrap();
    build_clausewitz_library(std::slice::from_ref(&vanilla_root), &vanilla_library).unwrap();
    build_clausewitz_library(std::slice::from_ref(&mod_root), &mod_library).unwrap();

    let results = query_clausewitz_libraries(
        &[mod_library.clone(), vanilla_library.clone()],
        "country event",
        Some("event"),
        2,
    )
    .unwrap();
    fs::remove_dir_all(&vanilla_root).unwrap();
    fs::remove_dir_all(&mod_root).unwrap();
    fs::remove_dir_all(&vanilla_library).unwrap();
    fs::remove_dir_all(&mod_library).unwrap();

    assert_eq!(results[0].symbol, "custom_mod.1");
    assert_eq!(results[1].symbol, "base.1");
}

#[test]
fn workflow_input_from_xlsx_preserves_structured_layout_without_text_round_trip() {
    let root = unique_temp_dir("workflow-xlsx-input");
    fs::create_dir_all(&root).unwrap();
    let xlsx = root.join("tree.xlsx");
    write_minimal_focus_xlsx(&xlsx);

    let input = workflow_input_from_path(&xlsx, Some("FocusTree"), "SOV", "sov_excel").unwrap();
    let layout = input.focus_layout.as_ref().unwrap();
    let workflow = run_workflow_json_with_focus_layout(
        &input.text,
        Some(layout),
        None,
        "SOV",
        "sov_excel",
        None,
        true,
        None,
    )
    .unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(input.text.contains("# Worksheet: FocusTree"));
    assert!(input.text.contains("## Simulated HOI4 x/y Grid"));
    assert!(input.text.contains("## Immutable Excel Import Contract"));
    assert!(!input.text.contains("\n国策树："));
    assert_eq!(layout.focuses.len(), 3);
    assert_eq!(layout.focuses[0].title, "重建中央委员会");
    assert_eq!(layout.focuses[1].title, "工业复兴");
    assert_eq!(layout.focuses[2].title, "整顿军队");
    assert_eq!(
        layout.focuses[1].relative_position_id.as_deref(),
        Some("SOV_rebuild_committee")
    );
    assert_eq!(layout.focuses[1].relative_x, Some(-2));
    assert_eq!(layout.focuses[2].relative_x, Some(2));
    assert!(workflow.contains("\"focus_layout\": true"));
    assert!(workflow.contains("SOV_industrial_revival"));
    assert!(workflow.contains(
        "\"id\": \"SOV_industrial_revival\", \"icon\": \"GFX_goal_generic_construct_civ_factory\", \"x\": -2, \"y\": 2, \"worksheet_x\": 0, \"worksheet_y\": 2, \"relative_position_id\": \"SOV_rebuild_committee\""
    ));
}

#[test]
fn parse_focus_layout_anchors_positions_to_start_focus() {
    let layout = parse_focus_layout("开端\n左线   中线   右线\n", "CPC", "cpc_demo");
    let root = layout.focuses.iter().find(|focus| focus.row == 0).unwrap();
    let middle = layout
        .focuses
        .iter()
        .find(|focus| focus.title == "中线")
        .unwrap();

    assert_eq!(root.relative_position_id, None);
    assert_eq!(
        middle.relative_position_id.as_deref(),
        Some(root.id.as_str())
    );
    assert_eq!(middle.relative_x, Some(0));
    assert_eq!(middle.relative_y, Some(1));
}

#[test]
fn parse_focus_layout_replaces_position_fallback_ids() {
    let layout = parse_focus_layout(
        "???\n奇怪|focus_3_0\n怪名|abc_focus_3_0\n",
        "SOV",
        "sov_demo",
    );
    let ids = layout
        .focuses
        .iter()
        .map(|focus| focus.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        ids,
        vec![
            "SOV_generated_focus_1",
            "SOV_generated_focus_2",
            "SOV_generated_focus_3"
        ]
    );
    assert!(!ids.iter().any(|id| is_position_fallback_focus_id(id)));
}

#[test]
fn focus_excel_replaces_position_fallback_ids() {
    let imported = ExcelFocusImport {
        cells: vec![
            ExcelFocusCell {
                row: 3,
                column: 0,
                title: "???".to_string(),
                id_hint: None,
                icon: None,
                completion_reward: Vec::new(),
            },
            ExcelFocusCell {
                row: 3,
                column: 2,
                title: "奇怪".to_string(),
                id_hint: Some("focus_3_0".to_string()),
                icon: None,
                completion_reward: Vec::new(),
            },
        ],
        mutual_markers: Vec::new(),
    };

    let layout = focus_layout_from_excel_cells(imported, "SOV", "sov_demo").unwrap();
    let ids = layout
        .focuses
        .iter()
        .map(|focus| focus.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(ids, vec!["SOV_generated_focus_1", "SOV_generated_focus_2"]);
    assert!(!ids.iter().any(|id| is_position_fallback_focus_id(id)));
}

#[test]
fn focus_excel_preserves_only_explicit_mutual_exclusion_pair() {
    let imported = ExcelFocusImport {
        cells: vec![
            ExcelFocusCell {
                row: 0,
                column: 0,
                title: "邀请盟军调停".to_string(),
                id_hint: Some("invite_allied_mediation".to_string()),
                icon: None,
                completion_reward: Vec::new(),
            },
            ExcelFocusCell {
                row: 0,
                column: 2,
                title: "接触中苏".to_string(),
                id_hint: Some("contact_ccp_soviet_union".to_string()),
                icon: None,
                completion_reward: Vec::new(),
            },
            ExcelFocusCell {
                row: 1,
                column: 0,
                title: "临时政府归来".to_string(),
                id_hint: Some("provisional_government_returns".to_string()),
                icon: None,
                completion_reward: Vec::new(),
            },
            ExcelFocusCell {
                row: 1,
                column: 2,
                title: "苏维埃政权".to_string(),
                id_hint: Some("soviet_power".to_string()),
                icon: None,
                completion_reward: Vec::new(),
            },
            ExcelFocusCell {
                row: 2,
                column: 0,
                title: "夺取平壤".to_string(),
                id_hint: Some("capture_pyongyang".to_string()),
                icon: None,
                completion_reward: Vec::new(),
            },
            ExcelFocusCell {
                row: 2,
                column: 2,
                title: "夺取汉城".to_string(),
                id_hint: Some("capture_seoul".to_string()),
                icon: None,
                completion_reward: Vec::new(),
            },
        ],
        mutual_markers: vec![(2, 1)],
    };

    let layout = focus_layout_from_excel_cells(imported, "KOR", "kor_spring").unwrap();
    let json = focus_excel_layout_json(
        &layout,
        Path::new("tree.xlsx"),
        Some("FocusTree"),
        "KOR",
        "kor_spring",
    );

    assert_eq!(
        layout.mutuals,
        vec![(
            "KOR_capture_pyongyang".to_string(),
            "KOR_capture_seoul".to_string(),
            2
        )]
    );
    assert!(json.contains("\"left\": \"KOR_capture_pyongyang\", \"right\": \"KOR_capture_seoul\""));
    assert!(!json.contains(
        "\"left\": \"KOR_invite_allied_mediation\", \"right\": \"KOR_contact_ccp_soviet_union\""
    ));
    assert!(!json.contains(
        "\"left\": \"KOR_provisional_government_returns\", \"right\": \"KOR_soviet_power\""
    ));
}

#[test]
fn excel_mutual_marker_accepts_descriptive_text() {
    assert!(is_excel_mutual_marker(
        "互斥（大韩民国和朝鲜苏维埃政权互斥）"
    ));
    assert!(is_excel_mutual_marker("相互排斥：左线和右线"));
    assert!(!is_excel_mutual_marker(
        "路线说明：大韩民国和朝鲜苏维埃政权互斥"
    ));
}

#[test]
fn focus_excel_json_marks_unresolved_rewards_as_blocked() {
    let layout = FocusLayout {
        tree_id: "sov_excel_SOV_focus_tree".to_string(),
        rows: Vec::new(),
        mutuals: Vec::new(),
        focuses: vec![FocusNode {
            title: "整训舰队".to_string(),
            id: "SOV_fleet_drill".to_string(),
            icon: None,
            x: 0,
            y: 0,
            relative_position_id: None,
            relative_x: None,
            relative_y: None,
            row: 0,
            column: 0,
            prerequisite: Vec::new(),
            mutually_exclusive: Vec::new(),
            completion_reward: vec![
                "# 获得民族精神 舰队整训 -> add_ideas = <idea id for 舰队整训> ()".to_string(),
            ],
        }],
    };
    let json = focus_excel_layout_json(
        &layout,
        Path::new("tree.xlsx"),
        Some("FocusTree"),
        "SOV",
        "sov_excel",
    );

    assert!(json.contains("\"safety\": {\"status\": \"blocked\""));
    assert!(json.contains("\"final_code_allowed\": false"));
    assert!(json.contains("<idea id for 舰队整训>"));
}

#[test]
fn apply_focus_layout_extends_existing_country_focus_tree() {
    let root = unique_temp_dir("apply-focus-layout-existing-tree");
    let focus_dir = root.join("common").join("national_focus");
    fs::create_dir_all(&focus_dir).unwrap();
    let existing_path = focus_dir.join("sov_existing.txt");
    fs::write(
        &existing_path,
        "focus_tree = {\n\tid = sov_existing_focus\n\tcountry = { factor = 0 modifier = { add = 10 tag = SOV } }\n\tfocus = {\n\t\tid = SOV_old_industry\n\t\tx = 0\n\t\ty = 3\n\t}\n}\n",
    )
    .unwrap();
    let layout = parse_focus_layout_with_rewards(
        "重启西伯利亚干线\n# completion_reward: 3个军工厂\n",
        "SOV",
        "sov_alt",
    );

    let changed = apply_focus_layout_to_mod(&root, &layout, "SOV", "sov_alt").unwrap();
    let changed_again = apply_focus_layout_to_mod(&root, &layout, "SOV", "sov_alt").unwrap();

    let existing = fs::read_to_string(&existing_path).unwrap();
    let new_file = root
        .join("common")
        .join("national_focus")
        .join("sov_alt_SOV_focus.txt");
    let loc = String::from_utf8_lossy(&fs::read(target_localisation_path(&root, "SOV")).unwrap())
        .to_string();
    let new_file_exists = new_file.exists();
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(changed.len(), 2);
    assert!(changed.contains(&existing_path));
    assert!(changed_again.is_empty());
    assert!(!new_file_exists);
    assert_eq!(existing.matches("focus_tree = {").count(), 1);
    assert!(existing.contains("id = SOV_old_industry"));
    assert!(existing.contains("id = SOV_reopen_siberian_mainline"));
    assert!(existing.contains("\t\ty = 4"));
    assert!(existing.contains("type = arms_factory level = 3"));
    assert!(loc.contains("SOV_reopen_siberian_mainline:0 \"重启西伯利亚干线\""));
}

#[test]
fn apply_focus_layout_avoids_existing_focus_id_collision() {
    let root = unique_temp_dir("apply-focus-layout-id-collision");
    let focus_dir = root.join("common").join("national_focus");
    fs::create_dir_all(&focus_dir).unwrap();
    let target_path = focus_dir.join("sov_existing.txt");
    fs::write(
        &target_path,
        "focus_tree = {\n\tid = sov_existing_focus\n\tcountry = { factor = 0 modifier = { add = 10 tag = SOV } }\n\tfocus = {\n\t\tid = SOV_old_industry\n\t\tx = 0\n\t\ty = 3\n\t}\n}\n",
    )
    .unwrap();
    fs::write(
        focus_dir.join("sov_other.txt"),
        "focus_tree = {\n\tid = sov_other_focus\n\tcountry = { factor = 0 modifier = { add = 10 tag = GER } }\n\tfocus = {\n\t\tid = SOV_reopen_siberian_mainline\n\t\tx = 0\n\t\ty = 0\n\t}\n}\n",
    )
    .unwrap();
    let layout = parse_focus_layout("重启西伯利亚干线\n", "SOV", "sov_alt");

    let changed = apply_focus_layout_to_mod(&root, &layout, "SOV", "sov_alt").unwrap();
    let changed_again = apply_focus_layout_to_mod(&root, &layout, "SOV", "sov_alt").unwrap();

    let target = fs::read_to_string(&target_path).unwrap();
    let loc = String::from_utf8_lossy(&fs::read(target_localisation_path(&root, "SOV")).unwrap())
        .to_string();
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(changed.len(), 2);
    assert!(changed.contains(&target_path));
    assert!(changed_again.is_empty());
    assert!(!target.contains("\t\tid = SOV_reopen_siberian_mainline\n\t\tx = 0\n\t\ty = 4"));
    assert!(target.contains("id = SOV_reopen_siberian_mainline_2"));
    assert!(!target.contains("id = SOV_reopen_siberian_mainline_3"));
    assert!(loc.contains("SOV_reopen_siberian_mainline_2:0 \"重启西伯利亚干线\""));
    assert!(!loc.contains("SOV_reopen_siberian_mainline_3"));
}

#[test]
fn validator_collects_event_localisation_refs_and_option_warnings() {
    let path = Path::new("M:\\mod\\events\\sov_nep_events.txt");
    let text = r#"
add_namespace = sov_nep
news_event = { id = sov_nep.1 title = sov_nep.1.t desc = sov_nep.1.d
  option = { name = sov_nep.1.a add_political_power = 50 }
  option = { add_stability = 0.02 }
}
"#;
    let mut refs = BTreeMap::new();
    let mut reporter = Reporter::default();

    collect_localisation_refs(path, text, &mut refs, &mut reporter);

    assert!(refs.contains_key("sov_nep.1.t"));
    assert!(refs.contains_key("sov_nep.1.d"));
    assert!(refs.contains_key("sov_nep.1.a"));
    assert!(reporter.warnings.iter().any(|warning| warning
        .contains("event block should include is_triggered_only = yes or mean_time_to_happen")));
    assert!(reporter
        .warnings
        .iter()
        .any(|warning| warning.contains("event option block should include name")));
}

#[test]
fn validator_warns_when_event_id_uses_undeclared_namespace() {
    let path = Path::new("M:\\mod\\events\\bad_events.txt");
    let text = r#"
add_namespace = sov_nep
country_event = {
	id = other_ns.1
	title = other_ns.1.t
	desc = other_ns.1.d
	is_triggered_only = yes
	option = { name = other_ns.1.a }
}
"#;
    let mut ids = BTreeMap::new();
    let mut namespaces = BTreeMap::new();
    let mut reporter = Reporter::default();

    collect_ids_and_namespaces(path, text, &mut ids, &mut namespaces, &mut reporter);

    assert!(ids.contains_key("other_ns.1"));
    assert!(namespaces.contains_key("sov_nep"));
    assert!(reporter.warnings.iter().any(|warning| warning
        .contains("event id other_ns.1 uses namespace other_ns, but this file declares sov_nep")));
}

#[test]
fn validator_accepts_multiple_top_level_event_namespaces_and_max_id() {
    let path = Path::new("M:\\mod\\events\\multi_events.txt");
    let text = r#"
add_namespace = sov_nep
add_namespace = fer_news

country_event = {
	id = sov_nep.200000
	title = sov_nep.200000.t
	desc = sov_nep.200000.d
	is_triggered_only = yes
	option = { name = sov_nep.200000.a }
}

news_event = {
	id = fer_news.1
	title = fer_news.1.t
	desc = fer_news.1.d
	is_triggered_only = yes
	option = { name = fer_news.1.a }
}
"#;
    let mut ids = BTreeMap::new();
    let mut namespaces = BTreeMap::new();
    let mut reporter = Reporter::default();

    collect_ids_and_namespaces(path, text, &mut ids, &mut namespaces, &mut reporter);

    assert!(namespaces.contains_key("sov_nep"));
    assert!(namespaces.contains_key("fer_news"));
    assert!(ids.contains_key("sov_nep.200000"));
    assert!(ids.contains_key("fer_news.1"));
    assert!(reporter.warnings.is_empty());
}

#[test]
fn validator_warns_for_late_namespace_and_event_id_overflow() {
    let path = Path::new("M:\\mod\\events\\late_namespace.txt");
    let text = r#"
country_event = {
	id = sov_nep.200001
	title = sov_nep.200001.t
	desc = sov_nep.200001.d
	is_triggered_only = yes
	option = { name = sov_nep.200001.a }
}

add_namespace = sov_nep
"#;
    let mut ids = BTreeMap::new();
    let mut namespaces = BTreeMap::new();
    let mut reporter = Reporter::default();

    collect_ids_and_namespaces(path, text, &mut ids, &mut namespaces, &mut reporter);

    assert!(reporter
        .warnings
        .iter()
        .any(|warning| warning.contains("uses number 200001")));
    assert!(reporter
        .warnings
        .iter()
        .any(|warning| warning.contains("should be declared at the top level")));
}

#[test]
fn validator_collects_focus_localisation_refs() {
    let path = Path::new("M:\\mod\\common\\national_focus\\sov_focus.txt");
    let mut refs = BTreeMap::new();
    let mut reporter = Reporter::default();

    collect_localisation_refs(
        path,
        "focus = { id = SOV_focus_0_0 icon = GFX_goal_generic_construct_civ_factory }",
        &mut refs,
        &mut reporter,
    );

    assert!(refs.contains_key("SOV_focus_0_0"));
    assert!(refs.contains_key("SOV_focus_0_0_desc"));
    assert!(reporter.warnings.is_empty());
}

#[test]
fn validator_warns_for_missing_sprite_texture() {
    let root = unique_temp_dir("missing-sprite");
    fs::create_dir_all(&root).unwrap();
    let path = root.join("interface").join("icons.gfx");
    let mut reporter = Reporter::default();

    check_sprite_textures(
        &root,
        &path,
        r#"spriteType = { name = "GFX_missing_icon" texturefile = "gfx/interface/missing.png" }"#,
        &mut reporter,
    );
    fs::remove_dir_all(&root).unwrap();

    assert!(reporter
        .warnings
        .iter()
        .any(|warning| warning.contains("sprite texturefile not found")));
}

#[test]
fn validator_detects_unknown_custom_gfx_refs() {
    let script_path = Path::new("M:\\mod\\common\\national_focus\\focus.txt");
    let mut sprites = BTreeSet::new();
    let mut refs = BTreeMap::new();

    collect_sprite_names(
        r#"spriteType = { name = "GFX_local_icon" texturefile = "gfx/interface/local.png" }"#,
        &mut sprites,
    );
    collect_gfx_refs(
        script_path,
        "focus = { icon = GFX_local_icon }\nfocus = { icon = GFX_goal_generic_political_reform }\nfocus = { icon = GFX_my_missing_icon }",
        &mut refs,
    );

    let unknown = refs
        .keys()
        .filter(|sprite| !sprites.contains(*sprite) && !is_known_vanilla_gfx(sprite))
        .cloned()
        .collect::<Vec<_>>();

    assert_eq!(unknown, vec!["GFX_my_missing_icon".to_string()]);
}

#[test]
fn game_index_collects_tags_states_and_sprites() {
    let root = unique_temp_dir("game-index");
    fs::create_dir_all(root.join("common").join("country_tags")).unwrap();
    fs::create_dir_all(root.join("common").join("buildings")).unwrap();
    fs::create_dir_all(root.join("common").join("resources")).unwrap();
    fs::create_dir_all(root.join("common").join("ideologies")).unwrap();
    fs::create_dir_all(root.join("common").join("country_leader")).unwrap();
    fs::create_dir_all(root.join("common").join("units").join("equipment")).unwrap();
    fs::create_dir_all(root.join("common").join("technologies")).unwrap();
    fs::create_dir_all(root.join("common").join("units")).unwrap();
    fs::create_dir_all(root.join("common").join("wargoals")).unwrap();
    fs::create_dir_all(root.join("documentation")).unwrap();
    fs::create_dir_all(root.join("history").join("states")).unwrap();
    fs::create_dir_all(root.join("interface")).unwrap();
    fs::create_dir_all(root.join("map")).unwrap();
    fs::write(
        root.join("common")
            .join("country_tags")
            .join("00_countries.txt"),
        "SOV = \"countries/Soviet.txt\"\nITA = \"countries/Italy.txt\"\n",
    )
    .unwrap();
    fs::write(
        root.join("history").join("states").join("64-Moscow.txt"),
        "state = { id = 64 name = \"STATE_64\" provinces = { 123 456 } }",
    )
    .unwrap();
    fs::write(
        root.join("map").join("definition.csv"),
        "province;red;green;blue;x\n789;1;2;3;land\n",
    )
    .unwrap();
    fs::write(
        root.join("common").join("buildings").join("00_buildings.txt"),
        "buildings = {\n  arms_factory = { cost = 7200 max_level = 5 }\n  industrial_complex = {\n    cost = 7200\n    max_level = 10\n  }\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("common")
            .join("resources")
            .join("00_resources.txt"),
        "resources = {\n  oil = { icon = 1 }\n  steel = { icon = 2 }\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("common").join("ideologies").join("00_ideologies.txt"),
        "ideologies = {\n  democratic = { types = { conservatism = {} } }\n  communism = { types = { marxism = {} } }\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("common")
            .join("country_leader")
            .join("00_traits.txt"),
        "leader_traits = {\n  popular_figurehead = { random = no }\n  war_industrialist = { random = no }\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("common")
            .join("units")
            .join("equipment")
            .join("00_equipment.txt"),
        "equipments = {\n  infantry_equipment = { year = 1936 }\n  artillery_equipment = { year = 1936 }\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("common")
            .join("technologies")
            .join("infantry.txt"),
        "technologies = {\n  infantry_weapons = { categories = { infantry tech_infantry } }\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("common").join("units").join("00_units.txt"),
        "sub_units = {\n  infantry = { sprite = infantry }\n  artillery_brigade = { sprite = artillery }\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("common").join("wargoals").join("00_wargoals.txt"),
        "wargoal_types = {\n  annex_everything = { sprite_index = 1 }\n  puppet_wargoal_focus = { sprite_index = 2 }\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("documentation")
            .join("modifiers_documentation.md"),
        "## Modifiers\n\n## Table\n\n## stability_factor\n\n## political_power_factor\n",
    )
    .unwrap();
    fs::write(
        root.join("documentation").join("effects_documentation.md"),
        "## Effects\n\n## Table\n\n## add_political_power\n\n## add_opinion_modifier\n\n## create_unit\n",
    )
    .unwrap();
    fs::write(
        root.join("documentation").join("triggers_documentation.md"),
        "## Triggers\n\n## Table\n\n## has_war\n\n## has_completed_focus\n\n## has_idea\n",
    )
    .unwrap();
    fs::write(
        root.join("interface").join("goals.gfx"),
        r#"spriteType = { name = "GFX_goal_game_focus_icon" texturefile = "gfx/interface/goals/game.dds" }
spriteType = { name = "GFX_focus_generic_workers" texturefile = "gfx/interface/goals/workers.dds" }"#,
    )
    .unwrap();
    fs::write(
        root.join("interface").join("ideas.gfx"),
        r#"spriteType = { name = "GFX_idea_workers_council" texturefile = "gfx/interface/ideas/workers.dds" }"#,
    )
    .unwrap();
    fs::write(
        root.join("interface").join("decisions.gfx"),
        r#"spriteType = { name = "GFX_decision_SOV_the_workers_dictatorship" texturefile = "gfx/interface/decisions/workers.dds" }
spriteType = { name = "GFX_decision_category_generic_communism" texturefile = "gfx/interface/decisions/category_communism.dds" }"#,
    )
    .unwrap();
    fs::write(
        root.join("interface").join("eventpictures.gfx"),
        r#"spriteType = { name = "GFX_report_event_soviet_workers_revolution" texturefile = "gfx/event_pictures/workers.dds" }"#,
    )
    .unwrap();
    fs::write(
        root.join("interface").join("leader_portraits.gfx"),
        r#"spriteType = { name = "GFX_portrait_SOV_lenin" texturefile = "gfx/leaders/SOV/lenin.dds" }
spriteType = { name = "GFX_portrait_GER_wilhelm_ii" texturefile = "gfx/leaders/GER/wilhelm_ii.dds" }"#,
    )
    .unwrap();

    let index = build_game_index(&root).unwrap();
    let json = game_index_json(&index);
    fs::remove_dir_all(&root).unwrap();

    assert!(index.country_tags.contains("SOV"));
    assert!(index.country_tags.contains("ITA"));
    assert!(index.state_ids.contains(&64));
    assert_eq!(index.state_names.get("STATE_64"), Some(&64));
    assert!(index.province_ids.contains(&123));
    assert!(index.province_ids.contains(&456));
    assert!(index.province_ids.contains(&789));
    assert!(index.sprites.contains("GFX_goal_game_focus_icon"));
    assert!(index
        .focus_goal_sprites
        .contains("GFX_goal_game_focus_icon"));
    assert!(index
        .focus_goal_sprites
        .contains("GFX_focus_generic_workers"));
    assert!(index.idea_pictures.contains("workers_council"));
    assert!(index
        .event_pictures
        .contains("GFX_report_event_soviet_workers_revolution"));
    assert!(index
        .decision_icons
        .contains("SOV_the_workers_dictatorship"));
    assert!(index
        .decision_category_pictures
        .contains("GFX_decision_category_generic_communism"));
    assert!(index.leader_portraits.contains("GFX_portrait_SOV_lenin"));
    assert!(index
        .leader_portraits
        .contains("GFX_portrait_GER_wilhelm_ii"));
    assert!(index.buildings.contains("arms_factory"));
    assert_eq!(index.building_max_levels.get("arms_factory"), Some(&5));
    assert_eq!(
        index.building_max_levels.get("industrial_complex"),
        Some(&10)
    );
    assert!(index.resources.contains("oil"));
    assert!(index.ideologies.contains("democratic"));
    assert!(index.traits.contains("popular_figurehead"));
    assert!(index.equipment_types.contains("infantry_equipment"));
    assert!(index.technologies.contains("infantry_weapons"));
    assert!(index.technology_categories.contains("tech_infantry"));
    assert!(index.sub_units.contains("infantry"));
    assert!(index.wargoal_types.contains("annex_everything"));
    assert!(index.effects.contains("add_political_power"));
    assert!(index.effects.contains("add_opinion_modifier"));
    assert!(!index.effects.contains("Effects"));
    assert!(index.triggers.contains("has_war"));
    assert!(index.triggers.contains("has_completed_focus"));
    assert!(!index.triggers.contains("Triggers"));
    assert!(index.modifiers.contains("stability_factor"));
    assert!(!index.modifiers.contains("Modifiers"));
    assert!(json.contains("\"country_tags\": [\"ITA\", \"SOV\"]"));
    assert!(json.contains("\"state_ids\": [64]"));
    assert!(json.contains("\"state_names\": {\"STATE_64\": 64}"));
    assert!(json.contains("\"province_ids\": [123, 456, 789]"));
    assert!(json.contains("\"GFX_goal_game_focus_icon\""));
    assert!(json.contains(
        "\"focus_goal_sprites\": [\"GFX_focus_generic_workers\", \"GFX_goal_game_focus_icon\"]"
    ));
    assert!(json.contains("\"idea_pictures\": [\"workers_council\"]"));
    assert!(json.contains("\"event_pictures\": [\"GFX_report_event_soviet_workers_revolution\"]"));
    assert!(json.contains("\"decision_icons\": [\"SOV_the_workers_dictatorship\"]"));
    assert!(json
        .contains("\"decision_category_pictures\": [\"GFX_decision_category_generic_communism\"]"));
    assert!(json.contains(
        "\"leader_portraits\": [\"GFX_portrait_GER_wilhelm_ii\", \"GFX_portrait_SOV_lenin\"]"
    ));
    assert!(json.contains("\"buildings\": [\"arms_factory\", \"industrial_complex\"]"));
    assert!(
        json.contains("\"building_max_levels\": {\"arms_factory\": 5, \"industrial_complex\": 10}")
    );
    assert!(json.contains("\"resources\": [\"oil\", \"steel\"]"));
    assert!(json.contains("\"ideologies\": [\"communism\", \"democratic\"]"));
    assert!(json.contains("\"traits\": [\"popular_figurehead\", \"war_industrialist\"]"));
    assert!(json.contains("\"equipment_types\": [\"artillery_equipment\", \"infantry_equipment\"]"));
    assert!(json.contains("\"technologies\": [\"infantry_weapons\"]"));
    assert!(json.contains("\"technology_categories\": [\"infantry\", \"tech_infantry\"]"));
    assert!(json.contains("\"sub_units\": [\"artillery_brigade\", \"infantry\"]"));
    assert!(json.contains("\"wargoal_types\": [\"annex_everything\", \"puppet_wargoal_focus\"]"));
    assert!(json.contains(
        "\"effects\": [\"add_opinion_modifier\", \"add_political_power\", \"create_unit\"]"
    ));
    assert!(json.contains("\"triggers\": [\"has_completed_focus\", \"has_idea\", \"has_war\"]"));
    assert!(json.contains("\"modifiers\": [\"political_power_factor\", \"stability_factor\"]"));
}

#[test]
fn code_catalog_classifies_indexed_hoi4_code_for_model_use() {
    let root = unique_temp_dir("code-catalog");
    fs::create_dir_all(root.join("common").join("country_tags")).unwrap();
    fs::create_dir_all(root.join("common").join("buildings")).unwrap();
    fs::create_dir_all(root.join("common").join("resources")).unwrap();
    fs::create_dir_all(root.join("common").join("modifiers")).unwrap();
    fs::create_dir_all(root.join("documentation")).unwrap();
    fs::create_dir_all(root.join("history").join("states")).unwrap();
    fs::write(
        root.join("common")
            .join("country_tags")
            .join("00_countries.txt"),
        "SOV = \"countries/Soviet.txt\"\n",
    )
    .unwrap();
    fs::write(
        root.join("common")
            .join("buildings")
            .join("00_buildings.txt"),
        "buildings = { arms_factory = { max_level = 5 } industrial_complex = { max_level = 10 } }",
    )
    .unwrap();
    fs::write(
        root.join("common")
            .join("resources")
            .join("00_resources.txt"),
        "resources = { oil = {} steel = {} }",
    )
    .unwrap();
    fs::write(
        root.join("common").join("modifiers").join("00_static.txt"),
        "static_modifiers = { test_static = { stability_factor = 0.05 } }",
    )
    .unwrap();
    fs::write(
        root.join("history").join("states").join("64-Moscow.txt"),
        "state = { id = 64 name = \"STATE_64\" provinces = { 123 } }",
    )
    .unwrap();
    fs::write(
        root.join("documentation")
            .join("modifiers_documentation.md"),
        "## Modifiers\n\n## stability_factor\n\n## political_power_factor\n",
    )
    .unwrap();
    fs::write(
        root.join("documentation").join("effects_documentation.md"),
        "## Effects\n\n## add_political_power\n\n## add_stability\n",
    )
    .unwrap();
    fs::write(
        root.join("documentation").join("triggers_documentation.md"),
        "## Triggers\n\n## has_war\n\n## has_idea\n",
    )
    .unwrap();
    let output = root.join("catalog.json");

    cmd_code_catalog(&[
        "--game-root".to_string(),
        root.to_string_lossy().to_string(),
        "--max-items".to_string(),
        "1".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(json.contains("\"schema\": \"hoi4skill.code_catalog.v1\""));
    assert!(json.contains("\"id\": \"effects\""));
    assert!(json.contains("\"kind\": \"script_command\""));
    assert!(json.contains("\"id\": \"triggers\""));
    assert!(json.contains("\"kind\": \"script_condition\""));
    assert!(json.contains("\"id\": \"modifiers\""));
    assert!(json.contains("\"kind\": \"modifier\""));
    assert!(json.contains("\"id\": \"buildings\""));
    assert!(json.contains("\"id\": \"country_tags\""));
    assert!(json.contains("\"id\": \"state_ids\""));
    assert!(json.contains("\"items\": [\"add_political_power\"]"));
    assert!(json.contains("\"truncated\": true"));
    assert!(json.contains("Treat missing category entries as unknown"));
    assert!(json.contains("\"effects\": 2"));
    assert!(json.contains("\"triggers\": 2"));
    assert!(json.contains("\"modifiers\": 3"));
}

#[test]
fn check_code_symbol_classifies_known_hoi4_code_and_rejects_wrong_kind() {
    let root = unique_temp_dir("check-code-symbol");
    fs::create_dir_all(root.join("documentation")).unwrap();
    fs::create_dir_all(root.join("interface")).unwrap();
    fs::write(
        root.join("documentation").join("effects_documentation.md"),
        "## Effects\n\n## add_political_power\n",
    )
    .unwrap();
    fs::write(
        root.join("documentation").join("triggers_documentation.md"),
        "## Triggers\n\n## has_war\n",
    )
    .unwrap();
    fs::write(
        root.join("documentation")
            .join("modifiers_documentation.md"),
        "## Modifiers\n\n## stability_factor\n",
    )
    .unwrap();
    fs::write(
        root.join("interface").join("ideas.gfx"),
        r#"spriteType = { name = "GFX_idea_workers_council" texturefile = "gfx/interface/ideas/workers.dds" }"#,
    )
    .unwrap();
    let effect_output = root.join("effect.json");
    let wrong_kind_output = root.join("wrong_kind.json");
    let idea_output = root.join("idea.json");

    cmd_check_code_symbol(&[
        "--game-root".to_string(),
        root.to_string_lossy().to_string(),
        "--symbol".to_string(),
        "add_political_power".to_string(),
        "--kind".to_string(),
        "effect".to_string(),
        "--output".to_string(),
        effect_output.to_string_lossy().to_string(),
    ])
    .unwrap();
    let err = cmd_check_code_symbol(&[
        "--game-root".to_string(),
        root.to_string_lossy().to_string(),
        "--symbol".to_string(),
        "add_political_power".to_string(),
        "--kind".to_string(),
        "modifier".to_string(),
        "--output".to_string(),
        wrong_kind_output.to_string_lossy().to_string(),
    ])
    .unwrap_err();
    cmd_check_code_symbol(&[
        "--game-root".to_string(),
        root.to_string_lossy().to_string(),
        "--symbol".to_string(),
        "GFX_idea_workers_council".to_string(),
        "--kind".to_string(),
        "resource_id".to_string(),
        "--output".to_string(),
        idea_output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let effect_json = read_utf8_lossy(&effect_output).unwrap();
    let wrong_kind_json = read_utf8_lossy(&wrong_kind_output).unwrap();
    let idea_json = read_utf8_lossy(&idea_output).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(effect_json.contains("\"schema\": \"hoi4skill.code_symbol_check.v1\""));
    assert!(effect_json.contains("\"ok\": true"));
    assert!(effect_json.contains("\"category\": \"effects\""));
    assert!(effect_json.contains("\"kind\": \"effect\""));
    assert!(err.contains("was not found in the indexed HOI4 code catalog for kind `modifier`"));
    assert!(wrong_kind_json.contains("\"ok\": false"));
    assert!(wrong_kind_json.contains("do not emit it as Clausewitz code"));
    assert!(idea_json.contains("\"ok\": true"));
    assert!(idea_json.contains("\"category\": \"idea_pictures\""));
    assert!(idea_json.contains("\"symbol\": \"workers_council\""));
    assert!(idea_json.contains(
        "\"normalized_candidates\": [\"GFX_idea_workers_council\", \"workers_council\"]"
    ));
}

#[test]
fn check_code_symbol_returns_semantic_candidates_for_bad_ai_symbol() {
    let root = unique_temp_dir("check-code-symbol-semantic-candidates");
    fs::create_dir_all(root.join("documentation")).unwrap();
    fs::write(
        root.join("documentation").join("effects_documentation.md"),
        "## Effects\n\n## add_political_power\n\n## add_stability\n",
    )
    .unwrap();
    let output = root.join("symbol.json");

    let err = cmd_check_code_symbol(&[
        "--game-root".to_string(),
        root.to_string_lossy().to_string(),
        "--symbol".to_string(),
        "add_politicalpower".to_string(),
        "--kind".to_string(),
        "effect".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap_err();

    let json = read_utf8_lossy(&output).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(err.contains("was not found"));
    assert!(json.contains("\"ok\": false"));
    assert!(json.contains("\"semantic_candidates\""));
    assert!(json.contains("\"symbol\": \"add_political_power\""));
}

#[test]
fn related_code_symbol_matches_can_scope_buildings_and_resources() {
    let mut index = GameIndex::default();
    index.buildings.insert("arms_factory".to_string());
    index.resources.insert("steel".to_string());
    index
        .focus_goal_sprites
        .insert("GFX_goal_steel".to_string());
    index.technologies.insert("steel_production".to_string());

    let building_matches = related_code_symbol_matches(&index, "arm_factory", Some("building"), 5)
        .into_iter()
        .map(|item| item.category.to_string())
        .collect::<Vec<_>>();
    let resource_matches = related_code_symbol_matches(&index, "steel", Some("resource"), 5)
        .into_iter()
        .map(|item| item.category.to_string())
        .collect::<Vec<_>>();

    assert_eq!(building_matches, vec!["buildings".to_string()]);
    assert_eq!(resource_matches, vec!["resources".to_string()]);
}

#[test]
fn compile_intent_turns_llm_shorthand_into_modifier_code() {
    let root = unique_temp_dir("compile-intent");
    fs::create_dir_all(&root).unwrap();
    let output = root.join("intent.json");

    cmd_compile_intent(&[
        "--text".to_string(),
        "llm：战争正当化 = -10%".to_string(),
        "--kind".to_string(),
        "idea".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(json.contains("\"schema\": \"hoi4skill.intent_compile.v1\""));
    assert!(json.contains("\"intent\": \"战争正当化 = -10%\""));
    assert!(json.contains("\"kind\": \"idea_modifier_candidate\""));
    assert!(json.contains("\"code\": \"justify_war_goal_time = -0.1\""));
    assert!(json.contains("\"status\": \"verified_shape\""));
    assert!(json.contains("\"anti_hallucination_rule\""));
}

#[test]
fn compile_intent_error_reports_related_indexed_code_for_bad_ai_effect() {
    let root = unique_temp_dir("compile-intent-related-code");
    fs::create_dir_all(root.join("documentation")).unwrap();
    fs::write(
        root.join("documentation").join("effects_documentation.md"),
        "## Effects\n\n## add_political_power\n",
    )
    .unwrap();
    let output = root.join("intent.json");

    let err = cmd_compile_intent(&[
        "--text".to_string(),
        "llm：political_power = 50".to_string(),
        "--kind".to_string(),
        "effect".to_string(),
        "--game-root".to_string(),
        root.to_string_lossy().to_string(),
        "--strict-code-index".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap_err();

    let json = read_utf8_lossy(&output).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(err.contains("intent compilation blocked unresolved or unindexed HOI4 code"));
    assert!(json.contains("\"ok\": false"));
    assert!(json.contains("related indexed code: effects/effect `add_political_power`"));
    assert!(json.contains("political_power = 50"));
}

#[test]
fn compile_intent_blocks_context_mismatched_code_kind() {
    let root = unique_temp_dir("compile-intent-context-mismatch");
    fs::create_dir_all(&root).unwrap();
    let output = root.join("intent.json");

    let err = cmd_compile_intent(&[
        "--text".to_string(),
        "llm：政治点+50".to_string(),
        "--kind".to_string(),
        "idea".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap_err();

    let json = read_utf8_lossy(&output).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(err.contains("intent compilation blocked unresolved or unindexed HOI4 code"));
    assert!(json.contains("\"ok\": false"));
    assert!(json.contains("\"final_code_allowed\": false"));
    assert!(json.contains("\"status\": \"blocked\""));
    assert!(json.contains("\"allowed_kinds\": [\"idea_modifier\""));
    assert!(json.contains("\"kind\": \"country_effect\""));
    assert!(json.contains("compiled to `country_effect` but --kind `idea`"));
}

#[test]
fn compile_intent_auto_infers_effect_context_without_raw_trigger_noise() {
    let root = unique_temp_dir("compile-intent-auto-effect");
    fs::create_dir_all(&root).unwrap();
    let output = root.join("intent.json");

    cmd_compile_intent(&[
        "--text".to_string(),
        "llm：政治点+50".to_string(),
        "--kind".to_string(),
        "auto".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(json.contains("\"context\": \"effect\""));
    assert!(json.contains("\"kind\": \"country_effect\""));
    assert!(json.contains("\"code\": \"add_political_power = 50\""));
    assert!(json.contains("\"ok\": true"));
    assert!(!json.contains("raw_trigger"));
}

#[test]
fn compile_intent_handles_fullwidth_numbers_and_negative_direction() {
    let root = unique_temp_dir("compile-intent-fullwidth");
    fs::create_dir_all(&root).unwrap();
    let game = root.join("game");
    fs::create_dir_all(game.join("documentation")).unwrap();
    fs::write(
        game.join("documentation")
            .join("modifiers_documentation.md"),
        "## Modifiers\n\n## stability_factor\n\n## justify_war_goal_time\n",
    )
    .unwrap();
    let stable = root.join("stable.json");
    let justify = root.join("justify.json");
    let fullwidth = root.join("fullwidth.json");

    cmd_compile_intent(&[
        "--text".to_string(),
        "llm：稳定度＋5％".to_string(),
        "--kind".to_string(),
        "idea".to_string(),
        "--game-root".to_string(),
        game.to_string_lossy().to_string(),
        "--strict-code-index".to_string(),
        "--output".to_string(),
        stable.to_string_lossy().to_string(),
    ])
    .unwrap();
    cmd_compile_intent(&[
        "--text".to_string(),
        "llm：战争正当化降低10%".to_string(),
        "--kind".to_string(),
        "idea".to_string(),
        "--game-root".to_string(),
        game.to_string_lossy().to_string(),
        "--strict-code-index".to_string(),
        "--output".to_string(),
        justify.to_string_lossy().to_string(),
    ])
    .unwrap();
    cmd_compile_intent(&[
        "--text".to_string(),
        "llm：战争正当化 ＝ −10％".to_string(),
        "--kind".to_string(),
        "idea".to_string(),
        "--game-root".to_string(),
        game.to_string_lossy().to_string(),
        "--strict-code-index".to_string(),
        "--output".to_string(),
        fullwidth.to_string_lossy().to_string(),
    ])
    .unwrap();

    let stable_json = read_utf8_lossy(&stable).unwrap();
    let justify_json = read_utf8_lossy(&justify).unwrap();
    let fullwidth_json = read_utf8_lossy(&fullwidth).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(stable_json.contains("\"code\": \"stability_factor = 0.05\""));
    assert!(stable_json.contains("\"ok\": true"));
    assert!(justify_json.contains("\"code\": \"justify_war_goal_time = -0.1\""));
    assert!(fullwidth_json.contains("\"code\": \"justify_war_goal_time = -0.1\""));
}

#[test]
fn compile_intent_blocks_keyword_effect_when_number_is_missing() {
    let root = unique_temp_dir("compile-intent-missing-number");
    fs::create_dir_all(&root).unwrap();
    let output = root.join("intent.json");

    let err = cmd_compile_intent(&[
        "--text".to_string(),
        "llm：稳定度提高很多".to_string(),
        "--kind".to_string(),
        "idea".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap_err();

    let json = read_utf8_lossy(&output).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(err.contains("intent compilation blocked unresolved or unindexed HOI4 code"));
    assert!(json.contains("\"ok\": false"));
    assert!(json.contains("\"final_code_allowed\": false"));
    assert!(json.contains("Could not parse a required numeric value"));
}

#[test]
fn compile_intent_applies_negative_direction_to_integer_effects() {
    let root = unique_temp_dir("compile-intent-negative-int");
    fs::create_dir_all(&root).unwrap();
    let game = root.join("game");
    fs::create_dir_all(game.join("documentation")).unwrap();
    fs::write(
        game.join("documentation").join("effects_documentation.md"),
        "## Effects\n\n## add_political_power\n\n## army_experience\n",
    )
    .unwrap();
    let pp = root.join("pp.json");
    let xp = root.join("xp.json");

    cmd_compile_intent(&[
        "--text".to_string(),
        "llm：政治点减少50".to_string(),
        "--kind".to_string(),
        "effect".to_string(),
        "--game-root".to_string(),
        game.to_string_lossy().to_string(),
        "--strict-code-index".to_string(),
        "--output".to_string(),
        pp.to_string_lossy().to_string(),
    ])
    .unwrap();
    cmd_compile_intent(&[
        "--text".to_string(),
        "llm：陆军经验降低5".to_string(),
        "--kind".to_string(),
        "effect".to_string(),
        "--game-root".to_string(),
        game.to_string_lossy().to_string(),
        "--strict-code-index".to_string(),
        "--output".to_string(),
        xp.to_string_lossy().to_string(),
    ])
    .unwrap();

    let pp_json = read_utf8_lossy(&pp).unwrap();
    let xp_json = read_utf8_lossy(&xp).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(pp_json.contains("\"code\": \"add_political_power = -50\""));
    assert!(xp_json.contains("\"code\": \"army_experience = -5\""));
}

#[test]
fn compile_intent_blocks_unseparated_multiple_effects() {
    let root = unique_temp_dir("compile-intent-unseparated-effects");
    fs::create_dir_all(&root).unwrap();
    let output = root.join("intent.json");

    let err = cmd_compile_intent(&[
        "--text".to_string(),
        "llm：政治点+50 稳定度+5%".to_string(),
        "--kind".to_string(),
        "auto".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap_err();

    let json = read_utf8_lossy(&output).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(err.contains("intent compilation blocked unresolved or unindexed HOI4 code"));
    assert!(json.contains("\"ok\": false"));
    assert!(json.contains("\"final_code_allowed\": false"));
    assert!(json.contains("Multiple effect intents appear in one segment"));
    assert!(!json.contains("\"code\": \"add_political_power = 5\""));
}

#[test]
fn compile_intent_accepts_newline_separated_effects() {
    let root = unique_temp_dir("compile-intent-newline-effects");
    fs::create_dir_all(&root).unwrap();
    let game = root.join("game");
    fs::create_dir_all(game.join("documentation")).unwrap();
    fs::write(
        game.join("documentation").join("effects_documentation.md"),
        "## Effects\n\n## add_political_power\n\n## add_stability\n",
    )
    .unwrap();
    let output = root.join("intent.json");

    cmd_compile_intent(&[
        "--text".to_string(),
        "llm：政治点+50\n稳定度+5%".to_string(),
        "--kind".to_string(),
        "auto".to_string(),
        "--game-root".to_string(),
        game.to_string_lossy().to_string(),
        "--strict-code-index".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(json.contains("\"code\": \"add_political_power = 50\""));
    assert!(json.contains("\"code\": \"add_stability = 0.05\""));
    assert!(json.contains("\"ok\": true"));
}

#[test]
fn strict_compile_intent_blocks_unindexed_modifier_code() {
    let root = unique_temp_dir("strict-compile-intent");
    fs::create_dir_all(root.join("documentation")).unwrap();
    fs::write(
        root.join("documentation")
            .join("modifiers_documentation.md"),
        "## Modifiers\n\n## stability_factor\n",
    )
    .unwrap();
    let output = root.join("intent.json");

    let err = cmd_compile_intent(&[
        "--text".to_string(),
        "llm：战争正当化 = -10%".to_string(),
        "--kind".to_string(),
        "idea".to_string(),
        "--game-root".to_string(),
        root.to_string_lossy().to_string(),
        "--strict-code-index".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap_err();

    let json = read_utf8_lossy(&output).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(err.contains("intent compilation blocked unresolved or unindexed HOI4 code"));
    assert!(json.contains("\"ok\": false"));
    assert!(json.contains("unindexed modifier `justify_war_goal_time`"));
    assert!(json.contains("\"symbol\": \"justify_war_goal_time\""));
    assert!(json.contains("\"kind\": \"modifier\""));
    assert!(json.contains("\"ok\": false"));
}

#[test]
fn strict_compile_intent_blocks_missing_modifier_index_category() {
    let root = unique_temp_dir("strict-compile-intent-missing-category");
    fs::create_dir_all(root.join("documentation")).unwrap();
    let output = root.join("intent.json");

    let err = cmd_compile_intent(&[
        "--text".to_string(),
        "llm：战争正当化 = -10%".to_string(),
        "--kind".to_string(),
        "idea".to_string(),
        "--game-root".to_string(),
        root.to_string_lossy().to_string(),
        "--strict-code-index".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap_err();

    let json = read_utf8_lossy(&output).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(err.contains("intent compilation blocked unresolved or unindexed HOI4 code"));
    assert!(json.contains("\"ok\": false"));
    assert!(json.contains("strict code index has no indexed modifiers"));
    assert!(json.contains("documentation/modifiers_documentation.md"));
}

#[test]
fn strict_compile_intent_accepts_indexed_modifier_code() {
    let root = unique_temp_dir("strict-compile-intent-indexed");
    fs::create_dir_all(root.join("documentation")).unwrap();
    fs::write(
        root.join("documentation")
            .join("modifiers_documentation.md"),
        "## Modifiers\n\n## justify_war_goal_time\n",
    )
    .unwrap();
    let output = root.join("intent.json");

    cmd_compile_intent(&[
        "--text".to_string(),
        "llm：战争正当化 = -10%".to_string(),
        "--kind".to_string(),
        "idea".to_string(),
        "--game-root".to_string(),
        root.to_string_lossy().to_string(),
        "--strict-code-index".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let json = read_utf8_lossy(&output).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(json.contains("\"ok\": true"));
    assert!(json.contains("\"code_index_checked\": true"));
    assert!(json.contains("\"symbol\": \"justify_war_goal_time\""));
    assert!(json.contains("\"kind\": \"modifier\""));
}

#[test]
fn strict_compile_intent_blocks_missing_effect_index_category() {
    let root = unique_temp_dir("strict-compile-intent-missing-effect-category");
    fs::create_dir_all(root.join("documentation")).unwrap();
    let output = root.join("intent.json");

    let err = cmd_compile_intent(&[
        "--text".to_string(),
        "llm：政治点+50".to_string(),
        "--kind".to_string(),
        "effect".to_string(),
        "--game-root".to_string(),
        root.to_string_lossy().to_string(),
        "--strict-code-index".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap_err();

    let json = read_utf8_lossy(&output).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(err.contains("intent compilation blocked unresolved or unindexed HOI4 code"));
    assert!(json.contains("strict code index has no indexed effects"));
    assert!(json.contains("documentation/effects_documentation.md"));
}

#[test]
fn strict_compile_intent_blocks_missing_trigger_index_category() {
    let root = unique_temp_dir("strict-compile-intent-missing-trigger-category");
    fs::create_dir_all(root.join("documentation")).unwrap();
    let output = root.join("intent.json");

    let err = cmd_compile_intent(&[
        "--text".to_string(),
        "llm：战争中".to_string(),
        "--kind".to_string(),
        "trigger".to_string(),
        "--game-root".to_string(),
        root.to_string_lossy().to_string(),
        "--strict-code-index".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap_err();

    let json = read_utf8_lossy(&output).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(err.contains("intent compilation blocked unresolved or unindexed HOI4 code"));
    assert!(json.contains("strict code index has no indexed triggers"));
    assert!(json.contains("documentation/triggers_documentation.md"));
}

#[test]
fn game_index_helpers_distinguish_known_and_unknown_refs() {
    let mut index = GameIndex::default();
    index.country_tags.insert("SOV".to_string());
    index.focus_ids.insert("SOV_real_focus".to_string());
    index.sprites.insert("GFX_game_focus_icon".to_string());
    index.buildings.insert("arms_factory".to_string());
    index
        .building_max_levels
        .insert("arms_factory".to_string(), 5);
    index.resources.insert("oil".to_string());
    index.ideologies.insert("democratic".to_string());
    index.traits.insert("popular_figurehead".to_string());
    index
        .equipment_types
        .insert("infantry_equipment".to_string());
    index.technologies.insert("infantry_weapons".to_string());
    index
        .technology_categories
        .insert("tech_infantry".to_string());
    index.sub_units.insert("infantry".to_string());
    index.wargoal_types.insert("annex_everything".to_string());
    index.modifiers.insert("stability_factor".to_string());
    let local_sprites = BTreeSet::new();
    let path = Path::new("M:\\mod\\common\\national_focus\\focus.txt");
    let mut gfx_refs = BTreeMap::new();
    let mut tag_refs = BTreeMap::new();
    let mut game_data_refs = GameDataRefs::default();
    let technology_path = Path::new("M:\\mod\\common\\technologies\\sample.txt");

    collect_gfx_refs(
        path,
        "focus = { icon = GFX_game_focus_icon }\nfocus = { icon = GFX_missing_custom }",
        &mut gfx_refs,
    );
    collect_country_tag_refs(
        path,
        "tag = SOV\navailable = { tag = BAD }\ntag = ROOT",
        &mut tag_refs,
    );
    collect_game_data_refs(
        technology_path,
        "completion_reward = {\n add_building_construction = { type = arms_factory level = 1 }\n add_building_construction = { type = arms_factory level = 6 }\n add_building_construction = { type = mystery_factory level = 1 }\n add_resource = { type = oil amount = 4 }\n add_resource = { type = unobtainium amount = 4 }\n add_equipment_to_stockpile = { type = infantry_equipment amount = 100 }\n add_equipment_to_stockpile = { type = mystery_equipment amount = 100 }\n set_technology = { infantry_weapons = 1 mystery_tech = 1 }\n create_wargoal = { type = annex_everything target = BAD }\n create_wargoal = { type = mystery_wargoal target = BAD }\n}\nset_politics = { ruling_party = democratic }\nset_politics = { ruling_party = monarchism }\ntraits = { popular_figurehead mystery_trait }\ntechnologies = { sample_tech = { categories = { tech_infantry mystery_category } } }\ndivision_template = { regiments = { infantry = { x = 0 y = 0 } mystery_battalion = { x = 0 y = 1 } } }\nmodifier = { stability_factor = 0.05 mystery_modifier = 0.1 add = 10 tag = SOV }",
        &mut game_data_refs,
    );

    let unknown_gfx = gfx_refs
        .keys()
        .filter(|sprite| !is_known_sprite(sprite, &local_sprites, Some(&index)))
        .cloned()
        .collect::<Vec<_>>();
    let unknown_tags = tag_refs
        .keys()
        .filter(|tag| !index.country_tags.contains(*tag) && !is_dynamic_tag_ref(tag))
        .cloned()
        .collect::<Vec<_>>();
    let unknown_buildings = game_data_refs
        .buildings
        .keys()
        .filter(|building| !index.buildings.contains(*building))
        .cloned()
        .collect::<Vec<_>>();
    let unknown_resources = game_data_refs
        .resources
        .keys()
        .filter(|resource| !index.resources.contains(*resource))
        .cloned()
        .collect::<Vec<_>>();
    let unknown_ideologies = game_data_refs
        .ideologies
        .keys()
        .filter(|ideology| !index.ideologies.contains(*ideology))
        .cloned()
        .collect::<Vec<_>>();
    let unknown_traits = game_data_refs
        .traits
        .keys()
        .filter(|name| !index.traits.contains(*name))
        .cloned()
        .collect::<Vec<_>>();
    let unknown_equipment = game_data_refs
        .equipment
        .keys()
        .filter(|name| !index.equipment_types.contains(*name))
        .cloned()
        .collect::<Vec<_>>();
    let unknown_modifiers = game_data_refs
        .modifiers
        .keys()
        .filter(|name| !index.modifiers.contains(*name))
        .cloned()
        .collect::<Vec<_>>();
    let unknown_technologies = game_data_refs
        .technologies
        .keys()
        .filter(|name| !index.technologies.contains(*name))
        .cloned()
        .collect::<Vec<_>>();
    let unknown_technology_categories = game_data_refs
        .technology_categories
        .keys()
        .filter(|name| !index.technology_categories.contains(*name))
        .cloned()
        .collect::<Vec<_>>();
    let unknown_sub_units = game_data_refs
        .sub_units
        .keys()
        .filter(|name| !index.sub_units.contains(*name))
        .cloned()
        .collect::<Vec<_>>();
    let unknown_wargoals = game_data_refs
        .wargoal_types
        .keys()
        .filter(|name| !index.wargoal_types.contains(*name))
        .cloned()
        .collect::<Vec<_>>();

    assert_eq!(unknown_gfx, vec!["GFX_missing_custom".to_string()]);
    assert_eq!(unknown_tags, vec!["BAD".to_string()]);
    assert_eq!(unknown_buildings, vec!["mystery_factory".to_string()]);
    assert_eq!(unknown_resources, vec!["unobtainium".to_string()]);
    assert_eq!(unknown_ideologies, vec!["monarchism".to_string()]);
    assert_eq!(unknown_traits, vec!["mystery_trait".to_string()]);
    assert_eq!(unknown_equipment, vec!["mystery_equipment".to_string()]);
    assert_eq!(unknown_modifiers, vec!["mystery_modifier".to_string()]);
    assert_eq!(unknown_technologies, vec!["mystery_tech".to_string()]);
    assert_eq!(
        unknown_technology_categories,
        vec!["mystery_category".to_string()]
    );
    assert_eq!(unknown_sub_units, vec!["mystery_battalion".to_string()]);
    assert_eq!(unknown_wargoals, vec!["mystery_wargoal".to_string()]);
    assert_eq!(game_data_refs.building_levels.len(), 3);

    let mut reporter = Reporter::default();
    warn_building_levels(&game_data_refs.building_levels, &index, &mut reporter);
    assert!(reporter
        .warnings
        .iter()
        .any(|warning| warning.contains("arms_factory level 6 exceeds game max_level 5")));
}

#[test]
fn validator_rejects_unknown_effect_keys_against_indexed_docs() {
    let root = unique_temp_dir("validate-unknown-effects");
    fs::create_dir_all(root.join("events")).unwrap();
    fs::create_dir_all(root.join("common").join("national_focus")).unwrap();
    fs::write(
        root.join("events").join("bad_events.txt"),
        "add_namespace = tst\ncountry_event = {\n id = tst.1\n is_triggered_only = yes\n option = {\n  name = tst.1.a\n  add_stability = 0.05\n  add_army_org = 0.1\n  spawn_units = { division = { division = \"infantry\" } }\n  USA = { add_opinion = KOR = 20 }\n }\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("common")
            .join("national_focus")
            .join("bad_focus.txt"),
        "focus_tree = {\n id = tst_focus\n country = { factor = 0 modifier = { add = 10 tag = KOR } }\n focus = {\n  id = KOR_bad_reward\n  icon = GFX_goal_unknown\n  x = 0\n  y = 0\n  cost = 10\n  ai_will_do = { factor = 100 }\n  available = { }\n  bypass = { }\n  cancel_if_invalid = yes\n  continue_if_invalid = no\n  available_if_capitulated = no\n  completion_reward = { add_modifier = { mystery = yes } }\n }\n}\n",
    )
    .unwrap();

    let mut index = GameIndex::default();
    index.country_tags.insert("KOR".to_string());
    index.country_tags.insert("USA".to_string());
    for effect in [
        "add_stability",
        "country_event",
        "news_event",
        "add_opinion_modifier",
        "create_unit",
    ] {
        index.effects.insert(effect.to_string());
    }

    let report = validate_mod(&root, Some(&index)).unwrap();
    fs::remove_dir_all(&root).unwrap();

    let errors = report.errors.join("\n");
    assert!(errors.contains("unknown effect `add_army_org`"));
    assert!(errors.contains("unknown effect `spawn_units`"));
    assert!(errors.contains("unknown effect `add_opinion`"));
    assert!(errors.contains("related indexed code: effects/effect `add_opinion_modifier`"));
    assert!(errors.contains("unknown effect `add_modifier`"));
    assert!(!errors.contains("unknown effect `add_stability`"));
    assert!(!errors.contains("unknown effect `USA`"));
}

#[test]
fn validator_rejects_unknown_trigger_keys_against_indexed_docs() {
    let root = unique_temp_dir("validate-unknown-triggers");
    fs::create_dir_all(root.join("common").join("national_focus")).unwrap();
    fs::write(
        root.join("common")
            .join("national_focus")
            .join("bad_trigger.txt"),
        "focus_tree = {\n id = tst_focus\n country = { factor = 0 modifier = { add = 10 tag = KOR } }\n focus = {\n  id = KOR_bad_trigger\n  icon = GFX_goal_unknown\n  x = 0\n  y = 0\n  cost = 10\n  ai_will_do = { factor = 100 }\n  available = { has_war = yes haswar = yes has_fake_condition = yes OR = { has_idea = test_idea has_fake_nested = yes } }\n  bypass = { }\n  cancel_if_invalid = yes\n  continue_if_invalid = no\n  available_if_capitulated = no\n  completion_reward = { }\n }\n}\n",
    )
    .unwrap();

    let mut index = GameIndex::default();
    index.triggers.insert("has_war".to_string());
    index.triggers.insert("has_idea".to_string());

    let report = validate_mod(&root, Some(&index)).unwrap();
    fs::remove_dir_all(&root).unwrap();

    let errors = report.errors.join("\n");
    assert!(errors.contains("unknown trigger `haswar`"));
    assert!(errors.contains("related indexed code: triggers/trigger `has_war`"));
    assert!(errors.contains("unknown trigger `has_fake_condition`"));
    assert!(errors.contains("unknown trigger `has_fake_nested`"));
    assert!(!errors.contains("unknown trigger `has_war`"));
    assert!(!errors.contains("unknown trigger `has_idea`"));
}

#[test]
fn validate_strict_rejects_unknown_effect_inside_scripted_effect_file_with_related_code() {
    let root = unique_temp_dir("validate-scripted-effect-unknown");
    fs::create_dir_all(root.join("common").join("scripted_effects")).unwrap();
    fs::write(
        root.join("common")
            .join("scripted_effects")
            .join("bad_effects.txt"),
        "bad_ai_effect = {\n add_politicalpower = 50\n}\n",
    )
    .unwrap();

    let mut index = GameIndex::default();
    index.effects.insert("add_political_power".to_string());

    let report = validate_mod_with_options(
        &root,
        Some(&index),
        ValidationOptions {
            strict_code_index: true,
        },
    )
    .unwrap();
    fs::remove_dir_all(&root).unwrap();

    let errors = report.errors.join("\n");
    assert!(errors.contains("scripted_effect `bad_ai_effect`"));
    assert!(errors.contains("unknown effect `add_politicalpower`"));
    assert!(errors.contains("related indexed code: effects/effect `add_political_power`"));
}

#[test]
fn validate_strict_rejects_unknown_trigger_inside_scripted_trigger_file_with_related_code() {
    let root = unique_temp_dir("validate-scripted-trigger-unknown");
    fs::create_dir_all(root.join("common").join("scripted_triggers")).unwrap();
    fs::write(
        root.join("common")
            .join("scripted_triggers")
            .join("bad_triggers.txt"),
        "bad_ai_trigger = {\n haswar = yes\n}\n",
    )
    .unwrap();

    let mut index = GameIndex::default();
    index.triggers.insert("has_war".to_string());

    let report = validate_mod_with_options(
        &root,
        Some(&index),
        ValidationOptions {
            strict_code_index: true,
        },
    )
    .unwrap();
    fs::remove_dir_all(&root).unwrap();

    let errors = report.errors.join("\n");
    assert!(errors.contains("scripted_trigger `bad_ai_trigger`"));
    assert!(errors.contains("unknown trigger `haswar`"));
    assert!(errors.contains("related indexed code: triggers/trigger `has_war`"));
}

#[test]
fn validate_strict_rejects_unknown_effect_inside_state_effect_helper_with_related_code() {
    let root = unique_temp_dir("validate-state-effect-helper-unknown");
    fs::create_dir_all(root.join("common").join("scripted_effects")).unwrap();
    fs::write(
        root.join("common")
            .join("scripted_effects")
            .join("bad_state_effects.txt"),
        "bad_state_effect = {\n 64 = {\n  add_politicalpower = 50\n }\n}\n",
    )
    .unwrap();

    let mut index = GameIndex::default();
    index.effects.insert("add_political_power".to_string());

    let report = validate_mod_with_options(
        &root,
        Some(&index),
        ValidationOptions {
            strict_code_index: true,
        },
    )
    .unwrap();
    fs::remove_dir_all(&root).unwrap();

    let errors = report.errors.join("\n");
    assert!(errors.contains("scripted_effect `bad_state_effect`"));
    assert!(errors.contains("unknown effect `add_politicalpower`"));
    assert!(errors.contains("related indexed code: effects/effect `add_political_power`"));
}

#[test]
fn validate_strict_rejects_unknown_code_inside_scripted_gui_effects_and_triggers() {
    let root = unique_temp_dir("validate-scripted-gui-unknown");
    fs::create_dir_all(root.join("common").join("scripted_guis")).unwrap();
    fs::write(
        root.join("common")
            .join("scripted_guis")
            .join("bad_guis.txt"),
        "scripted_gui = {\n bad_ai_gui = {\n  triggers = { haswar = yes }\n  effects = { add_politicalpower = 50 }\n }\n}\n",
    )
    .unwrap();

    let mut index = GameIndex::default();
    index.effects.insert("add_political_power".to_string());
    index.triggers.insert("has_war".to_string());

    let report = validate_mod_with_options(
        &root,
        Some(&index),
        ValidationOptions {
            strict_code_index: true,
        },
    )
    .unwrap();
    fs::remove_dir_all(&root).unwrap();

    let errors = report.errors.join("\n");
    assert!(errors.contains("effect context `effects` uses unknown effect `add_politicalpower`"));
    assert!(errors.contains("related indexed code: effects/effect `add_political_power`"));
    assert!(errors.contains("trigger context `triggers` uses unknown trigger `haswar`"));
    assert!(errors.contains("related indexed code: triggers/trigger `has_war`"));
}

#[test]
fn strict_code_index_rejects_unresolved_ai_mapping_markers() {
    let root = unique_temp_dir("strict-unresolved-ai-markers");
    fs::create_dir_all(&root).unwrap();
    let cards = parse_cards(
        "决议：神秘工业动员\n目标：KOR\n效果：外星工厂+5\n描述：这条效果无法映射。",
        &["决议", "民族精神"],
    );
    apply_feature_cards_to_mod(&root, &cards, "KOR", "kor_ai").unwrap();

    let mut index = GameIndex::default();
    index.country_tags.insert("KOR".to_string());
    index.triggers.insert("always".to_string());
    index.effects.insert("add_political_power".to_string());
    index.modifiers.insert("stability_factor".to_string());
    let report = validate_mod_with_options(
        &root,
        Some(&index),
        ValidationOptions {
            strict_code_index: true,
        },
    )
    .unwrap();
    fs::remove_dir_all(&root).unwrap();

    let errors = report.errors.join("\n");
    assert!(errors.contains("unresolved generated code marker"));
    assert!(errors.contains("Needs Codex mapping before final code"));
}

#[test]
fn strict_code_index_requires_game_root_before_final_check() {
    let root = unique_temp_dir("strict-code-index-requires-root");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("descriptor.mod"),
        "name=\"Strict Code Index\"\nsupported_version=\"*\"\n",
    )
    .unwrap();

    let report = validate_mod_with_options(
        &root,
        None,
        ValidationOptions {
            strict_code_index: true,
        },
    )
    .unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(report
        .errors
        .iter()
        .any(|error| { error.contains("strict code index validation requires --game-root") }));
}

#[test]
fn strict_code_index_rejects_unverified_generated_code_refs() {
    let root = unique_temp_dir("strict-code-index-unverified-refs");
    fs::create_dir_all(root.join("common").join("national_focus")).unwrap();
    fs::create_dir_all(root.join("common").join("ideas")).unwrap();
    fs::write(
        root.join("descriptor.mod"),
        "name=\"Strict Code Refs\"\nsupported_version=\"*\"\n",
    )
    .unwrap();
    fs::write(
        root.join("common")
            .join("national_focus")
            .join("bad_focus.txt"),
        "focus_tree = {\n id = tst_focus\n country = { factor = 0 modifier = { add = 10 tag = KOR } }\n focus = {\n  id = KOR_strict_test\n  icon = GFX_goal_generic_fake_socialism\n  x = 0\n  y = 0\n  cost = 10\n  ai_will_do = { factor = 100 }\n  available = { }\n  bypass = { }\n  cancel_if_invalid = yes\n  continue_if_invalid = no\n  available_if_capitulated = no\n  completion_reward = { add_political_power = 50 }\n }\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("common").join("ideas").join("bad_ideas.txt"),
        "ideas = { country = { tst_strict_idea = { picture = generic_production_bonus modifier = { stability_factor = 0.05 } } } }\n",
    )
    .unwrap();
    let mut index = GameIndex::default();
    index.country_tags.insert("KOR".to_string());

    let report = validate_mod_with_options(
        &root,
        Some(&index),
        ValidationOptions {
            strict_code_index: true,
        },
    )
    .unwrap();
    fs::remove_dir_all(&root).unwrap();

    let errors = report.errors.join("\n");
    assert!(errors.contains("GFX key GFX_goal_generic_fake_socialism"));
    assert!(errors.contains("idea picture generic_production_bonus"));
    assert!(errors.contains("effect-like key `add_political_power`"));
    assert!(errors.contains("modifier stability_factor cannot be verified"));
}

#[test]
fn apply_commands_final_check_runs_post_apply_validation() {
    let root = unique_temp_dir("apply-final-check");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("descriptor.mod"),
        "name=\"Apply Final Check\"\nsupported_version=\"*\"\n",
    )
    .unwrap();
    let input = root.join("cards.txt");
    fs::write(&input, "民族精神：测试精神\n效果：稳定度+5%\n").unwrap();

    let err = cmd_apply_feature_cards(&[
        "--input".to_string(),
        input.display().to_string(),
        "--mod-root".to_string(),
        root.display().to_string(),
        "--tag".to_string(),
        "KOR".to_string(),
        "--prefix".to_string(),
        "kor_test".to_string(),
        "--final-check".to_string(),
    ])
    .unwrap_err();
    fs::remove_dir_all(&root).unwrap();

    assert!(err.contains("strict feature-card generation requires --game-root"));
}

#[test]
fn strict_feature_card_gate_blocks_unresolved_mapping_before_write() {
    let root = unique_temp_dir("strict-feature-card-prewrite");
    let game = unique_temp_dir("strict-feature-card-game");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(game.join("common").join("country_tags")).unwrap();
    fs::write(
        game.join("common")
            .join("country_tags")
            .join("00_countries.txt"),
        "KOR = \"countries/Korea.txt\"\n",
    )
    .unwrap();
    let input = root.join("cards.txt");
    fs::write(
        &input,
        "决议：神秘工业动员\n目标：KOR\n效果：外星工厂+5\n描述：无法映射的效果。",
    )
    .unwrap();

    let err = cmd_apply_feature_cards(&[
        "--input".to_string(),
        input.display().to_string(),
        "--mod-root".to_string(),
        root.display().to_string(),
        "--tag".to_string(),
        "KOR".to_string(),
        "--prefix".to_string(),
        "kor_ai".to_string(),
        "--game-root".to_string(),
        game.display().to_string(),
        "--final-check".to_string(),
    ])
    .unwrap_err();
    let common_exists = root.join("common").exists();
    fs::remove_dir_all(&root).unwrap();
    fs::remove_dir_all(&game).unwrap();

    assert!(err.contains("strict feature-card generation blocked unresolved AI mappings"));
    assert!(err.contains("外星工厂+5"));
    assert!(!common_exists);
}

#[test]
fn strict_feature_card_gate_blocks_unindexed_semantic_modifier_before_write() {
    let cards = parse_cards(
        "民族精神：短期战争宣传\n目标：KOR\n效果：战争正当化 = -10%",
        FEATURE_CARD_HEADERS,
    );
    let mut index = GameIndex::default();
    index.country_tags.insert("KOR".to_string());
    index.modifiers.insert("stability_factor".to_string());

    let err = enforce_strict_feature_card_gate_with_options(
        ValidationOptions {
            strict_code_index: true,
        },
        &cards,
        "KOR",
        "kor_ai",
        Some(&index),
    )
    .unwrap_err();

    assert!(err.contains("strict feature-card generation blocked unresolved AI mappings"));
    assert!(err.contains("unindexed modifier `justify_war_goal_time`"));
    assert!(err.contains("check-code-symbol --kind modifier"));
}

#[test]
fn strict_feature_card_gate_blocks_missing_effect_index_category_before_write() {
    let cards = parse_cards(
        "决议：政治动员\n目标：KOR\n效果：政治点+50",
        FEATURE_CARD_HEADERS,
    );
    let mut index = GameIndex::default();
    index.country_tags.insert("KOR".to_string());

    let err = enforce_strict_feature_card_gate_with_options(
        ValidationOptions {
            strict_code_index: true,
        },
        &cards,
        "KOR",
        "kor_ai",
        Some(&index),
    )
    .unwrap_err();

    assert!(err.contains("strict feature-card generation blocked unresolved AI mappings"));
    assert!(err.contains("strict code index has no indexed effects"));
    assert!(err.contains("documentation/effects_documentation.md"));
}

#[test]
fn strict_feature_card_gate_blocks_unindexed_resource_inside_effect() {
    let cards = parse_cards(
        "决议：开发钢矿\n目标：KOR\n效果：钢+8",
        FEATURE_CARD_HEADERS,
    );
    let mut index = GameIndex::default();
    index.country_tags.insert("KOR".to_string());
    index.effects.insert("add_resource".to_string());
    index.resources.insert("oil".to_string());

    let err = enforce_strict_feature_card_gate_with_options(
        ValidationOptions {
            strict_code_index: true,
        },
        &cards,
        "KOR",
        "kor_ai",
        Some(&index),
    )
    .unwrap_err();

    assert!(err.contains("strict feature-card generation blocked unresolved AI mappings"));
    assert!(err.contains("unindexed resource `steel`"));
    assert!(err.contains("check-code-symbol --kind resource"));
}

#[test]
fn strict_feature_card_gate_blocks_unindexed_scripted_effect_inner_effect_before_write() {
    let cards = parse_cards(
        "脚本效果：政治动员脚本\n目标：KOR\n效果：政治点+50",
        FEATURE_CARD_HEADERS,
    );
    let mut index = GameIndex::default();
    index.country_tags.insert("KOR".to_string());
    index.effects.insert("add_stability".to_string());
    index
        .effects
        .insert("add_scaled_political_power".to_string());

    let err = enforce_strict_feature_card_gate_with_options(
        ValidationOptions {
            strict_code_index: true,
        },
        &cards,
        "KOR",
        "kor_ai",
        Some(&index),
    )
    .unwrap_err();

    assert!(err.contains("strict feature-card generation blocked unresolved AI mappings"));
    assert!(err.contains("unindexed effect `add_political_power`"));
    assert!(err.contains("related indexed code: effects/effect `add_scaled_political_power`"));
}

#[test]
fn strict_feature_card_gate_blocks_unresolved_scripted_trigger_inner_trigger_before_write() {
    let cards = parse_cards(
        "脚本触发：坏战争条件\n目标：KOR\n条件：haswar = yes",
        FEATURE_CARD_HEADERS,
    );
    let mut index = GameIndex::default();
    index.country_tags.insert("KOR".to_string());
    index.triggers.insert("has_war".to_string());

    let err = enforce_strict_feature_card_gate_with_options(
        ValidationOptions {
            strict_code_index: true,
        },
        &cards,
        "KOR",
        "kor_ai",
        Some(&index),
    )
    .unwrap_err();

    assert!(err.contains("strict feature-card generation blocked unresolved AI mappings"));
    assert!(err.contains("haswar = yes"));
    assert!(err.contains("related indexed code: triggers/trigger `has_war`"));
}

#[test]
fn strict_feature_card_gate_blocks_unindexed_state_effect_inner_building_before_write() {
    let cards = parse_cards(
        "州效果：建设军工\n目标：KOR\n州ID：64\n建筑：军工+2",
        FEATURE_CARD_HEADERS,
    );
    let mut index = GameIndex::default();
    index.country_tags.insert("KOR".to_string());
    index
        .effects
        .insert("add_building_construction".to_string());
    index.buildings.insert("industrial_complex".to_string());

    let err = enforce_strict_feature_card_gate_with_options(
        ValidationOptions {
            strict_code_index: true,
        },
        &cards,
        "KOR",
        "kor_ai",
        Some(&index),
    )
    .unwrap_err();

    assert!(err.contains("strict feature-card generation blocked unresolved AI mappings"));
    assert!(err.contains("unindexed building `arms_factory`"));
    assert!(err.contains("check-code-symbol --kind building"));
}

#[test]
fn strict_feature_card_gate_blocks_unmapped_scripted_gui_effect_before_write() {
    let cards = parse_cards(
        "特殊GUI：铁路运力面板\n目标：KOR\n用途：显示铁路运力。\n效果：政治点+50",
        FEATURE_CARD_HEADERS,
    );
    let mut index = GameIndex::default();
    index.country_tags.insert("KOR".to_string());
    index.effects.insert("add_political_power".to_string());

    let err = enforce_strict_feature_card_gate_with_options(
        ValidationOptions {
            strict_code_index: true,
        },
        &cards,
        "KOR",
        "kor_ai",
        Some(&index),
    )
    .unwrap_err();

    assert!(err.contains("strict feature-card generation blocked unresolved AI mappings"));
    assert!(err.contains("raw_effect"));
    assert!(err.contains("related indexed code: effects/effect `add_political_power`"));
}

#[test]
fn strict_parse_feature_cards_blocks_unindexed_semantic_modifier() {
    let root = unique_temp_dir("strict-parse-feature-cards");
    let game = root.join("game");
    fs::create_dir_all(game.join("common").join("country_tags")).unwrap();
    fs::create_dir_all(game.join("documentation")).unwrap();
    fs::write(
        game.join("common")
            .join("country_tags")
            .join("00_countries.txt"),
        "KOR = \"countries/Korea.txt\"\n",
    )
    .unwrap();
    fs::write(
        game.join("documentation")
            .join("modifiers_documentation.md"),
        "## Modifiers\n\n## stability_factor\n",
    )
    .unwrap();
    let input = root.join("cards.txt");
    let output = root.join("plan.json");
    fs::write(
        &input,
        "民族精神：短期战争宣传\n目标：KOR\n效果：战争正当化 = -10%",
    )
    .unwrap();

    let err = cmd_parse_feature_cards(&[
        "--input".to_string(),
        input.display().to_string(),
        "--tag".to_string(),
        "KOR".to_string(),
        "--prefix".to_string(),
        "kor_ai".to_string(),
        "--game-root".to_string(),
        game.display().to_string(),
        "--strict-code-index".to_string(),
        "--output".to_string(),
        output.display().to_string(),
    ])
    .unwrap_err();
    let output_exists = output.exists();
    fs::remove_dir_all(&root).unwrap();

    assert!(err.contains("strict feature-card generation blocked unresolved AI mappings"));
    assert!(err.contains("unindexed modifier `justify_war_goal_time`"));
    assert!(!output_exists);
}

#[test]
fn strict_event_card_gate_blocks_unresolved_mapping_before_write() {
    let root = unique_temp_dir("strict-event-card-prewrite");
    let game = unique_temp_dir("strict-event-card-game");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(game.join("common").join("country_tags")).unwrap();
    fs::write(
        game.join("common")
            .join("country_tags")
            .join("00_countries.txt"),
        "SOV = \"countries/Soviet.txt\"\n",
    )
    .unwrap();
    let input = root.join("events.txt");
    fs::write(
        &input,
        "事件：未知动员\n目标：SOV\n命名空间：sov_ai\n触发：神秘局势\n选项A：继续\n效果A：外星能量+50",
    )
    .unwrap();

    let err = cmd_apply_event_cards(&[
        "--input".to_string(),
        input.display().to_string(),
        "--mod-root".to_string(),
        root.display().to_string(),
        "--tag".to_string(),
        "SOV".to_string(),
        "--prefix".to_string(),
        "sov_ai".to_string(),
        "--game-root".to_string(),
        game.display().to_string(),
        "--final-check".to_string(),
    ])
    .unwrap_err();
    let events_exists = root.join("events").exists();
    fs::remove_dir_all(&root).unwrap();
    fs::remove_dir_all(&game).unwrap();

    assert!(err.contains("strict event-card generation blocked unresolved AI mappings"));
    assert!(err.contains("神秘局势"));
    assert!(err.contains("外星能量+50"));
    assert!(!events_exists);
}

#[test]
fn strict_event_card_gate_blocks_unindexed_semantic_trigger_before_write() {
    let cards = parse_cards(
        "事件：战时会议\n目标：SOV\n命名空间：sov_ai\n触发：战争中\n选项A：继续\n效果A：政治点+50",
        &["事件"],
    );
    let mut index = GameIndex::default();
    index.triggers.insert("has_idea".to_string());
    index.effects.insert("add_political_power".to_string());

    let err = enforce_strict_event_card_gate_with_options(
        ValidationOptions {
            strict_code_index: true,
        },
        &cards,
        Some(&index),
    )
    .unwrap_err();

    assert!(err.contains("strict event-card generation blocked unresolved AI mappings"));
    assert!(err.contains("unindexed trigger `has_war`"));
    assert!(err.contains("check-code-symbol --kind trigger"));
}

#[test]
fn strict_event_card_gate_blocks_missing_trigger_index_category_before_write() {
    let cards = parse_cards(
        "事件：战时会议\n目标：SOV\n命名空间：sov_ai\n触发：战争中\n选项A：继续\n效果A：政治点+50",
        &["事件"],
    );
    let mut index = GameIndex::default();
    index.effects.insert("add_political_power".to_string());

    let err = enforce_strict_event_card_gate_with_options(
        ValidationOptions {
            strict_code_index: true,
        },
        &cards,
        Some(&index),
    )
    .unwrap_err();

    assert!(err.contains("strict event-card generation blocked unresolved AI mappings"));
    assert!(err.contains("strict code index has no indexed triggers"));
    assert!(err.contains("documentation/triggers_documentation.md"));
}

#[test]
fn strict_parse_event_cards_blocks_unindexed_semantic_trigger() {
    let root = unique_temp_dir("strict-parse-event-cards");
    let game = root.join("game");
    fs::create_dir_all(game.join("common").join("country_tags")).unwrap();
    fs::create_dir_all(game.join("documentation")).unwrap();
    fs::write(
        game.join("common")
            .join("country_tags")
            .join("00_countries.txt"),
        "SOV = \"countries/Soviet.txt\"\n",
    )
    .unwrap();
    fs::write(
        game.join("documentation").join("triggers_documentation.md"),
        "## Triggers\n\n## has_idea\n",
    )
    .unwrap();
    fs::write(
        game.join("documentation").join("effects_documentation.md"),
        "## Effects\n\n## add_political_power\n",
    )
    .unwrap();
    let input = root.join("events.txt");
    let output = root.join("events.json");
    fs::write(
        &input,
        "事件：战时会议\n目标：SOV\n命名空间：sov_ai\n触发：战争中\n选项A：继续\n效果A：政治点+50",
    )
    .unwrap();

    let err = cmd_parse_event_cards(&[
        "--input".to_string(),
        input.display().to_string(),
        "--tag".to_string(),
        "SOV".to_string(),
        "--prefix".to_string(),
        "sov_ai".to_string(),
        "--game-root".to_string(),
        game.display().to_string(),
        "--strict-code-index".to_string(),
        "--output".to_string(),
        output.display().to_string(),
    ])
    .unwrap_err();
    let output_exists = output.exists();
    fs::remove_dir_all(&root).unwrap();

    assert!(err.contains("strict event-card generation blocked unresolved AI mappings"));
    assert!(err.contains("unindexed trigger `has_war`"));
    assert!(!output_exists);
}

#[test]
fn text_alignment_reports_missing_user_titles() {
    let root = unique_temp_dir("text-alignment-missing-title");
    fs::create_dir_all(root.join("localisation").join("simp_chinese")).unwrap();
    fs::write(
        target_localisation_path(&root, "KOR"),
        "\u{feff}l_simp_chinese:\n  KOR_industry:0 \"工业复兴\"\n  kor_event.1.t:0 \"工人大会\"\n  kor_event.1.a:0 \"召开大会\"\n",
    )
    .unwrap();
    let input = root.join("input.txt");
    fs::write(
        &input,
        "国策树：\n工业复兴 | kor_industry\n\n事件：工人大会\n选项A：召开大会\n\n民族精神：革命遗产\n",
    )
    .unwrap();

    let mut map = ArgMap {
        flags: BTreeSet::new(),
        values: HashMap::new(),
        value_lists: HashMap::new(),
        positionals: Vec::new(),
    };
    map.value_lists
        .insert("text-source".to_string(), vec![input.display().to_string()]);
    let report = text_alignment_report_from_args(&root, &map).unwrap();
    let json = text_alignment_report_json(&report);
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(report.expected.len(), 4);
    assert_eq!(report.matched_count(), 3);
    assert_eq!(report.missing().len(), 1);
    assert!(json.contains("\"text\": \"革命遗产\""));
    assert!(json.contains("\"missing_count\": 1"));
}

#[test]
fn validate_can_fail_on_text_alignment_source() {
    let root = unique_temp_dir("validate-text-alignment");
    fs::create_dir_all(root.join("localisation").join("simp_chinese")).unwrap();
    fs::write(
        target_localisation_path(&root, "KOR"),
        "\u{feff}l_simp_chinese:\n  KOR_industry:0 \"工业复兴\"\n",
    )
    .unwrap();
    let input = root.join("input.txt");
    fs::write(&input, "国策：工业复兴\n事件：缺失事件\n").unwrap();

    let map = parse_args(&[
        root.display().to_string(),
        "--text-source".to_string(),
        input.display().to_string(),
    ]);
    let mut reporter = validate_mod(&root, None).unwrap();
    check_text_alignment_from_validate_args(&root, &map, &mut reporter).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(reporter.errors.iter().any(|error| {
        error.contains("text alignment missing user-provided text `缺失事件`")
    }));
    assert!(!reporter
        .errors
        .iter()
        .any(|error| error.contains("`工业复兴`")));
}

#[test]
fn clausewitz_reference_table_marks_indexed_primitives() {
    let mut index = GameIndex::default();
    index.effects.insert("add_opinion_modifier".to_string());
    index.effects.insert("create_unit".to_string());
    index.effects.insert("add_political_power".to_string());
    index.triggers.insert("has_war".to_string());
    index.triggers.insert("has_completed_focus".to_string());
    index.modifiers.insert("stability_factor".to_string());
    index
        .focus_goal_sprites
        .insert("GFX_goal_workers".to_string());
    index.idea_pictures.insert("workers_council".to_string());

    let markdown = render_clausewitz_reference_table(Some(&index));

    assert!(markdown.contains("add_opinion_modifier (indexed)"));
    assert!(markdown.contains("create_unit (indexed)"));
    assert!(markdown.contains("has_war (indexed)"));
    assert!(markdown.contains("stability_factor (indexed)"));
    assert!(markdown.contains("USA = { add_opinion = KOR = 20 }"));
    assert!(markdown.contains("picture = bare_name_for_GFX_idea_bare_name"));
}

#[test]
fn render_focus_tree_uses_unknown_icon_and_empty_reward_by_default() {
    let layout = parse_focus_layout("根国策\n子国策\n", "CPC", "cpc_demo");
    let tree = render_focus_tree(&layout, "CPC");

    assert!(tree.contains("icon = GFX_goal_unknown"));
    assert!(!tree.contains("<parent focus id>"));
    assert!(!tree.contains("<focus id for relative placement>"));
    assert!(tree.contains("ai_will_do = {\n\t\t\tfactor = 100\n\t\t}"));
    assert!(tree.contains("completion_reward = {\n\t\t}\n"));
    assert!(!tree.contains("add_political_power = 50"));
}

#[test]
fn validator_requires_bare_idea_picture_reference_and_registered_sprite() {
    let root = unique_temp_dir("validate-idea-picture");
    fs::create_dir_all(root.join("common").join("ideas")).unwrap();
    fs::create_dir_all(root.join("interface")).unwrap();
    fs::write(
        root.join("descriptor.mod"),
        "name=\"Idea Picture Test\"\nsupported_version=\"*\"\n",
    )
    .unwrap();
    fs::write(
        root.join("interface").join("ideas.gfx"),
        r#"spriteType = { name = "GFX_idea_democratic_planned_economy" texturefile = "gfx/interface/ideas/democratic_planned_economy.dds" }"#,
    )
    .unwrap();
    fs::write(
        root.join("common").join("ideas").join("ideas.txt"),
        "ideas = { country = { good_idea = { picture = democratic_planned_economy } bad_idea = { picture = GFX_idea_democratic_planned_economy } } }\n",
    )
    .unwrap();

    let report = validate_mod(&root, None).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(report.errors.iter().any(|error| {
        error.contains("idea picture must omit the GFX_idea_ prefix")
            && error.contains("picture = democratic_planned_economy")
    }));
    assert!(!report
        .warnings
        .iter()
        .any(|warning| warning.contains("idea picture democratic_planned_economy requires")));
}

#[test]
fn validator_errors_for_incomplete_focus_template() {
    let path = Path::new("M:\\mod\\common\\national_focus\\bad_focus.txt");
    let text = r#"
focus = {
    id = BAD_focus
    icon = GFX_goal_unknown
    x = 0
    y = 0
}
"#;
    let mut reporter = Reporter::default();

    check_national_focus_fields(path, &strip_comments(text), &mut reporter);

    assert!(reporter
        .errors
        .iter()
        .any(|msg| msg.contains("missing required template field `cost`")));
    assert!(reporter
        .errors
        .iter()
        .any(|msg| msg.contains("missing required template field `completion_reward`")));
}

#[test]
fn validator_errors_for_position_fallback_focus_id() {
    let path = Path::new("M:\\mod\\common\\national_focus\\bad_focus.txt");
    let text = r#"
focus = {
    id = SOV_focus_3_0
    icon = GFX_goal_unknown
    x = 0
    y = 0
    cost = 10
    ai_will_do = { factor = 100 }
    available = { }
    bypass = { }
    cancel_if_invalid = yes
    continue_if_invalid = no
    available_if_capitulated = no
    completion_reward = { }
}
"#;
    let mut reporter = Reporter::default();

    check_national_focus_fields(path, &strip_comments(text), &mut reporter);

    assert!(reporter
        .errors
        .iter()
        .any(|msg| { msg.contains("focus SOV_focus_3_0 uses a generated position fallback id") }));
}

#[test]
fn validator_rejects_parent_relative_focus_coordinates() {
    let path = Path::new("M:\\mod\\common\\national_focus\\bad_anchor.txt");
    let text = r#"
focus_tree = {
    id = krc_tree
    country = { factor = 0 modifier = { add = 10 tag = KOR } }
    focus = {
        id = krc_uprising
        icon = GFX_goal_unknown
        x = 0
        y = 0
        cost = 10
        ai_will_do = { factor = 100 }
        available = { }
        bypass = { }
        cancel_if_invalid = yes
        continue_if_invalid = no
        available_if_capitulated = no
        completion_reward = { }
    }
    focus = {
        id = krc_unite_people
        icon = GFX_goal_unknown
        x = -2
        y = 1
        prerequisite = { focus = krc_uprising }
        relative_position_id = krc_uprising
        cost = 10
        ai_will_do = { factor = 100 }
        available = { }
        bypass = { }
        cancel_if_invalid = yes
        continue_if_invalid = no
        available_if_capitulated = no
        completion_reward = { }
    }
    focus = {
        id = krc_consolidate_power
        icon = GFX_goal_unknown
        x = -4
        y = 2
        prerequisite = { focus = krc_unite_people }
        relative_position_id = krc_unite_people
        cost = 10
        ai_will_do = { factor = 100 }
        available = { }
        bypass = { }
        cancel_if_invalid = yes
        continue_if_invalid = no
        available_if_capitulated = no
        completion_reward = { }
    }
}
"#;
    let mut reporter = Reporter::default();

    check_national_focus_fields(path, &strip_comments(text), &mut reporter);

    assert!(reporter.errors.iter().any(|error| {
        error.contains("focus krc_consolidate_power uses relative_position_id = krc_unite_people")
            && error.contains("opening focus krc_uprising")
    }));
}

#[test]
fn validator_errors_for_unknown_indexed_focus_and_symbols() {
    let root = unique_temp_dir("validate-indexed-focus-refs");
    let focus_dir = root.join("common").join("national_focus");
    fs::create_dir_all(&focus_dir).unwrap();
    fs::write(
        root.join("descriptor.mod"),
        "name=\"Test\"\nsupported_version=\"1.16.*\"\n",
    )
    .unwrap();
    fs::write(
        focus_dir.join("bad_focus.txt"),
        "focus_tree = {\n\tid = bad_tree\n\tcountry = { factor = 0 modifier = { add = 10 tag = SOV } }\n\tfocus = {\n\t\tid = SOV_real_focus\n\t\ticon = GFX_missing_icon\n\t\tx = 0\n\t\ty = 0\n\t\tprerequisite = { focus = SOV_missing_parent }\n\t\trelative_position_id = SOV_missing_relative\n\t\tcost = 10\n\t\tai_will_do = { factor = 100 }\n\t\tavailable = { ideology = mystery_ideology }\n\t\tbypass = { has_idea = mystery_idea }\n\t\tcancel_if_invalid = yes\n\t\tcontinue_if_invalid = no\n\t\tavailable_if_capitulated = no\n\t\tcompletion_reward = { set_technology = { mystery_tech = 1 } }\n\t}\n}\n",
    )
    .unwrap();
    let mut index = GameIndex::default();
    index.country_tags.insert("SOV".to_string());
    index.focus_ids.insert("SOV_dependency_focus".to_string());
    index.ideologies.insert("democratic".to_string());
    index.technologies.insert("infantry_weapons".to_string());

    let reporter = validate_mod(&root, Some(&index)).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(reporter
        .errors
        .iter()
        .any(|msg| msg.contains("GFX key GFX_missing_icon is referenced but not defined")));
    assert!(reporter
        .errors
        .iter()
        .any(|msg| msg.contains("focus id SOV_missing_parent is referenced but not present")));
    assert!(reporter
        .errors
        .iter()
        .any(|msg| msg.contains("focus id SOV_missing_relative is referenced but not present")));
    assert!(reporter
        .errors
        .iter()
        .any(|msg| msg
            .contains("ideology mystery_ideology is referenced but not present in game index")));
    assert!(reporter
        .errors
        .iter()
        .any(|msg| msg
            .contains("technology mystery_tech is referenced but not present in game index")));
}

#[test]
fn validator_warns_for_duplicate_yaml_and_jsonl_keys() {
    let path = Path::new("M:\\mod\\feature.yml");
    let mut reporter = Reporter::default();

    check_yaml_duplicate_keys(
        path,
        "feature:\n  id: one\n  title: A\n  id: two\nother:\n  id: ok\n",
        &mut reporter,
    );

    assert!(reporter
        .warnings
        .iter()
        .any(|warning| warning.contains("duplicate YAML key `feature.id`")));

    let jsonl_path = Path::new("M:\\mod\\feature.jsonl");
    let mut jsonl_reporter = Reporter::default();
    check_jsonl_duplicate_keys(
        jsonl_path,
        "{\"id\":\"one\",\"title\":\"A\",\"id\":\"two\",\"nested\":{\"id\":\"ok\"}}\n",
        &mut jsonl_reporter,
    );

    assert!(jsonl_reporter
        .warnings
        .iter()
        .any(|warning| warning.contains("duplicate JSONL key `id`")));
}

#[test]
fn validator_warns_for_mod_name_localisation_keys() {
    let root = unique_temp_dir("mod-name-loc-warning");
    let loc_dir = root.join("localisation").join("simp_chinese");
    fs::create_dir_all(&loc_dir).unwrap();
    let path = loc_dir.join("bad_l_simp_chinese.yml");
    fs::write(
        &path,
        "\u{feff}l_simp_chinese:\n  chinaprc_1979_mod_name:0 \"共和国一九七九：委员会民主\"\n",
    )
    .unwrap();
    let mut reporter = Reporter::default();

    check_localisation(&path, &mut reporter);
    fs::remove_dir_all(&root).unwrap();

    assert!(reporter.warnings.iter().any(|warning| {
        warning.contains("_mod_name")
            && warning.contains("descriptor.mod")
            && warning.contains("launcher .mod")
    }));
}

#[test]
fn validator_errors_for_localisation_without_utf8_bom() {
    let root = unique_temp_dir("loc-missing-bom-error");
    let loc_dir = root.join("localisation").join("simp_chinese");
    fs::create_dir_all(&loc_dir).unwrap();
    let path = loc_dir.join("bad_l_simp_chinese.yml");
    fs::write(&path, "l_simp_chinese:\n  TST:0 \"测试\"\n").unwrap();
    let mut reporter = Reporter::default();

    check_localisation(&path, &mut reporter);
    fs::remove_dir_all(&root).unwrap();

    assert!(reporter.errors.iter().any(|error| {
        error.contains("localisation file has no UTF-8 BOM")
            && error.contains("HOI4 may fail to load it")
    }));
    assert!(reporter.warnings.is_empty());
}

#[test]
fn workflow_validation_json_marks_warnings_as_not_ok() {
    let mut reporter = Reporter::default();
    reporter.warn("localisation key is referenced but not defined".to_string());

    let json = workflow_validation_json(Some(&reporter));

    assert!(json.contains("\"ran\": true"));
    assert!(json.contains("\"ok\": false"));
    assert!(json.contains("\"status\": \"warnings\""));
    assert!(json.contains("referenced but not defined"));
}

#[test]
fn focus_copy_prompt_scans_focus_localisation() {
    let root = unique_temp_dir("focus-copy-prompt");
    fs::create_dir_all(root.join("common").join("national_focus")).unwrap();
    fs::create_dir_all(root.join("localisation")).unwrap();
    fs::write(
        root.join("common").join("national_focus").join("sample.txt"),
        "focus_tree = { id = sample_tree\nfocus = { id = TAG_new_order }\nfocus = { id = TAG_army_reform }\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("localisation")
            .join("sample_l_simp_chinese.yml"),
        "l_simp_chinese:\n TAG_new_order:0 \"确立新秩序\"\n TAG_new_order_desc:0 \"旧制度留下的裂痕已经无法继续掩盖。我们必须重新组织国家、群众与军队，让新的路线成为共和国前进的基础。\"\n TAG_army_reform:0 \"整顿军队\"\n",
    )
    .unwrap();

    let entries = scan_focus_copy_entries(&root).unwrap();
    let options = FocusCopyPromptOptions {
        title_examples: 1,
        sample_keys: 1,
        style: FocusCopyPromptStyle::Full,
    };
    let prompt = render_focus_copy_prompt(std::slice::from_ref(&root), &entries, &options);
    let compact_options = FocusCopyPromptOptions {
        title_examples: 1,
        sample_keys: 0,
        style: FocusCopyPromptStyle::Compact,
    };
    let compact_prompt =
        render_focus_copy_prompt(std::slice::from_ref(&root), &entries, &compact_options);
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(entries.len(), 2);
    assert!(entries.iter().any(|entry| entry.id == "TAG_new_order"
        && entry.title.as_deref() == Some("确立新秩序")
        && entry.desc.is_some()));
    assert!(prompt.contains("Matched focus localisation entries: 2"));
    assert!(prompt.contains("TAG_new_order"));
    assert!(prompt.contains("历史矛盾/现实困境"));
    assert!(prompt.contains("长期修正"));
    assert!(prompt.contains("add_ideas"));
    assert!(prompt.contains("可校验 demo"));
    assert!(prompt.contains("保守脚本骨架"));
    assert!(prompt.contains("内部第一视角"));
    assert!(prompt.contains("第三方视角"));
    assert!(prompt.contains("## Learned Style Guide"));
    assert!(compact_prompt.contains("Matched focus localisation entries: 2"));
    assert!(!compact_prompt.contains("## Learned Style Guide"));
    assert!(!compact_prompt.contains("## Sample Focus Keys"));
}

#[test]
fn idea_copy_prompt_scans_national_spirits_separately() {
    let root = unique_temp_dir("idea-copy-prompt");
    fs::create_dir_all(root.join("common").join("ideas")).unwrap();
    fs::create_dir_all(root.join("localisation").join("simp_chinese")).unwrap();
    fs::write(
        root.join("common").join("ideas").join("sample_ideas.txt"),
        "ideas = { country = { tst_recovery = { picture = GFX_idea_recovery modifier = { stability_factor = 0.05 } } } hidden_ideas = { tst_hidden_wound = { picture = GFX_idea_wound modifier = { war_support_factor = -0.05 } } } political_advisor = { tst_advisor = { cost = 100 } } }\n",
    )
    .unwrap();
    fs::write(
        root.join("localisation")
            .join("simp_chinese")
            .join("sample_l_simp_chinese.yml"),
        "l_simp_chinese:\n tst_recovery:0 \"复苏的铁路经济\"\n tst_recovery_desc:0 \"铁路重新把边疆的粮食、煤炭与劳动力连接起来，国家机器也因此恢复了最基本的呼吸。\"\n tst_hidden_wound:0 \"军阀割据的余痛\"\n tst_hidden_wound_desc:0 \"旧军阀留下的地方网络仍在阴影中活动，任何中央命令都必须先穿过层层旧关系。\"\n tst_advisor:0 \"某位顾问\"\n tst_advisor_desc:0 \"这不是民族精神。\"\n",
    )
    .unwrap();

    let entries = scan_idea_copy_entries(&root, false).unwrap();
    let all_entries = scan_idea_copy_entries(&root, true).unwrap();
    let options = FocusCopyPromptOptions {
        title_examples: 8,
        sample_keys: 8,
        style: FocusCopyPromptStyle::Full,
    };
    let prompt = render_idea_copy_prompt(std::slice::from_ref(&root), &entries, &options, false);
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(entries.len(), 2);
    assert_eq!(all_entries.len(), 3);
    assert!(entries.iter().any(|entry| entry.id == "tst_recovery"));
    assert!(entries.iter().any(|entry| entry.id == "tst_hidden_wound"));
    assert!(!entries.iter().any(|entry| entry.id == "tst_advisor"));
    assert!(prompt.contains("HOI4 Chinese National Spirit Copywriting Prompt"));
    assert!(prompt.contains("民族精神是“国家长期状态"));
    assert!(prompt.contains("不要把民族精神写成"));
    assert!(prompt.contains("add_ideas"));
    assert!(prompt.contains("remove_ideas"));
    assert!(prompt.contains("tst_recovery"));
    assert!(prompt.contains("复苏的铁路经济"));
    assert!(!prompt.contains("某位顾问"));
}

#[test]
fn translate_localisation_prompt_reads_source_and_skips_existing_target_keys() {
    let root = unique_temp_dir("translate-localisation-prompt");
    fs::create_dir_all(root.join("localisation").join("english")).unwrap();
    fs::create_dir_all(root.join("localisation").join("simp_chinese")).unwrap();
    fs::write(
        root.join("localisation")
            .join("english")
            .join("sample_l_english.yml"),
        "l_english:\n TST_name:0 \"New Order\"\n TST_desc:0 \"We must protect $STATE|Y$ and [ROOT.GetName].\"\n TST_existing:0 \"Already done\"\n",
    )
    .unwrap();
    fs::write(
        root.join("localisation")
            .join("simp_chinese")
            .join("sample_l_simp_chinese.yml"),
        "l_simp_chinese:\n TST_existing:0 \"已经完成\"\n",
    )
    .unwrap();

    let map = parse_args(&[
        "--mod-root".to_string(),
        root.display().to_string(),
        "--from".to_string(),
        "english".to_string(),
        "--to".to_string(),
        "simp_chinese".to_string(),
    ]);
    let from = normalise_localisation_language(value(&map, "from").unwrap()).unwrap();
    let to = normalise_localisation_language(value(&map, "to").unwrap()).unwrap();
    let source_roots = source_localisation_roots(&map, Some(&root), &from).unwrap();
    let existing = target_existing_keys(Some(&root), &to).unwrap();
    let files =
        collect_localisation_source_files(Some(&root), &source_roots, &[], &existing, 100).unwrap();
    let prompt = render_localisation_translation_prompt(&from, &to, &files);
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(all_translation_entries(&files).len(), 2);
    assert!(prompt.contains("l_simp_chinese:"));
    assert!(prompt.contains("TST_name:0 \"New Order\""));
    assert!(prompt.contains("$STATE|Y$"));
    assert!(prompt.contains("[ROOT.GetName]"));
    assert!(prompt.contains("Do not translate tokens"));
    assert!(!prompt.contains("TST_existing:0"));
}

#[test]
fn translate_localisation_yml_writes_target_named_files() {
    let root = unique_temp_dir("translate-localisation-yml");
    let output_dir = root.join("out").join("simp_chinese");
    fs::create_dir_all(root.join("localisation").join("english")).unwrap();
    fs::write(
        root.join("localisation")
            .join("english")
            .join("events_l_english.yml"),
        "l_english:\n evt.1.t:0 \"A New Dawn\"\n evt.1.d:0 \"The cabinet meets again.\"\n",
    )
    .unwrap();

    let source_roots = vec![root.join("localisation").join("english")];
    let files =
        collect_localisation_source_files(Some(&root), &source_roots, &[], &BTreeSet::new(), 100)
            .unwrap();
    let report =
        write_translation_yml_files(&files, "english", "simp_chinese", &output_dir, false).unwrap();
    let written = fs::read_to_string(output_dir.join("events_l_simp_chinese.yml")).unwrap();
    let second_report =
        write_translation_yml_files(&files, "english", "simp_chinese", &output_dir, false).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(report.contains("events_l_simp_chinese.yml"));
    assert!(written.starts_with('\u{feff}'));
    assert!(written.contains("l_simp_chinese:"));
    assert!(written.contains("evt.1.t:0 \"A New Dawn\""));
    assert!(written.contains("translate value before release"));
    assert!(second_report.contains("skipped_existing_files"));
    assert!(second_report.contains("events_l_simp_chinese.yml"));
}

#[test]
fn translate_localisation_supports_non_english_non_chinese_language_pairs() {
    let root = unique_temp_dir("translate-localisation-arbitrary-language");
    let output_dir = root.join("localisation").join("german");
    fs::create_dir_all(root.join("localisation").join("french")).unwrap();
    fs::write(
        root.join("localisation")
            .join("french")
            .join("events_l_french.yml"),
        "l_french:\n evt.2.t:0 \"Aube nouvelle\"\n evt.2.d:0 \"Le cabinet se réunit.\"\n",
    )
    .unwrap();

    let source_roots = vec![root.join("localisation").join("french")];
    let files =
        collect_localisation_source_files(Some(&root), &source_roots, &[], &BTreeSet::new(), 100)
            .unwrap();
    let prompt = render_localisation_translation_prompt("french", "german", &files);
    let report =
        write_translation_yml_files(&files, "french", "german", &output_dir, false).unwrap();
    let written = fs::read_to_string(output_dir.join("events_l_german.yml")).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(prompt.contains("Source language: `french`"));
    assert!(prompt.contains("Target language: `german`"));
    assert!(prompt.contains("l_german:"));
    assert!(prompt.contains("do not hard-code `l_simp_chinese:`"));
    assert!(report.contains("events_l_german.yml"));
    assert!(written.contains("l_german:"));
    assert!(written.contains("evt.2.t:0 \"Aube nouvelle\""));
}

#[test]
fn translate_localisation_apply_injects_translated_values_and_reports_omissions() {
    let root = unique_temp_dir("translate-localisation-apply");
    fs::create_dir_all(root.join("localisation").join("english")).unwrap();
    fs::create_dir_all(root.join("localisation").join("simp_chinese")).unwrap();
    fs::write(
        root.join("localisation")
            .join("english")
            .join("events_l_english.yml"),
        "l_english:\n evt.1.t:0 \"A New Dawn\"\n evt.1.d:0 \"The cabinet meets again.\"\n evt.1.a:0 \"Continue\"\n evt.1.b:0 \"Missing translation\"\n",
    )
    .unwrap();
    fs::write(
        root.join("localisation")
            .join("simp_chinese")
            .join("events_l_simp_chinese.yml"),
        "l_simp_chinese:\n evt.1.a:0 \"继续\"\n",
    )
    .unwrap();
    fs::write(
        root.join("translated_l_simp_chinese.yml"),
        "l_simp_chinese:\n evt.1.t:0 \"新的黎明\"\n evt.1.d:0 \"内阁再次召开会议。\"\n unused.key:0 \"多余条目\"\n",
    )
    .unwrap();

    let source_roots = vec![root.join("localisation").join("english")];
    let source_files =
        collect_localisation_source_files(Some(&root), &source_roots, &[], &BTreeSet::new(), 100)
            .unwrap();
    let translations =
        collect_translated_localisation_map(&[root.join("translated_l_simp_chinese.yml")]).unwrap();
    let report = apply_localisation_translations(
        &root,
        &source_files,
        "english",
        "simp_chinese",
        &translations,
        false,
    )
    .unwrap();
    let target = fs::read_to_string(
        root.join("localisation")
            .join("simp_chinese")
            .join("events_l_simp_chinese.yml"),
    )
    .unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(target.contains("evt.1.a:0 \"继续\""));
    assert!(target.contains("evt.1.t:0 \"新的黎明\""));
    assert!(target.contains("evt.1.d:0 \"内阁再次召开会议。\""));
    assert!(!target.contains("evt.1.b:0"));
    assert!(report.contains("\"schema\": \"hoi4skill.localisation_translate.apply.v1\""));
    assert!(report.contains("\"existing_keys\": [\"evt.1.a\"]"));
    assert!(report.contains("\"missing_keys\": [\"evt.1.b\"]"));
    assert!(report.contains("\"missing_after_apply\": [\"evt.1.b\"]"));
    assert!(report.contains("\"translated_unused_keys\": [\"unused.key\"]"));
}

#[test]
fn focus_copy_cards_render_prompt_batch() {
    let cards = parse_focus_copy_cards(
        "国策：整顿军队\n国策ID：PRC_army_rectification\n国家/势力：PRC\n时间线背景：中国内战结束后\n所属路线：群众路线\n国策作用：整顿旧军队残余，确立人民军队纪律\n前置矛盾：地方部队仍保留旧军阀习气\n关键词：人民军队，纪律，群众\n长度：中\n\n---\n\n国策：第二次文化革命\nID：PRC_second_cultural_revolution\n国家：PRC\n路线：继续革命\n作用：发动路线斗争，批判官僚主义\n关键词：继续革命，官僚主义\n",
    );
    let markdown = render_focus_copy_card_prompts(&cards);

    assert_eq!(cards.len(), 2);
    assert_eq!(cards[0].focus_id, "PRC_army_rectification");
    assert_eq!(cards[0].tone, "revolutionary_mobilisation");
    assert_eq!(cards[1].tone, "ideological_debate");
    assert!(markdown.contains("国策ID：PRC_army_rectification"));
    assert!(markdown.contains("PRC_second_cultural_revolution_desc:0 \"描述\""));
    assert!(markdown.contains("历史矛盾/现实困境"));
    assert!(markdown.contains("长期修正"));
    assert!(markdown.contains("add_ideas"));
    assert!(markdown.contains("可校验 demo"));
    assert!(markdown.contains("保守脚本骨架"));
    assert!(markdown.contains("内部第一视角"));
    assert!(markdown.contains("第三方观察者"));
    assert!(markdown.contains("完整国策骨架"));
    assert!(markdown.contains("focus = {"));
    assert!(markdown.contains("relative_position_id =  #基于某个国策位置的相对位置"));
    assert!(markdown.contains("country = { factor = 0 modifier = { add = 10 tag = <TAG> } }"));
    assert!(markdown.contains("y=0 一个开篇国策 x=0"));
    assert!(markdown.contains("真实国策图标 sprite"));
    assert!(markdown.contains("icon = <verified focus icon sprite from interface/goals*.gfx"));
}

#[test]
fn emit_hoi4yaml_from_focus_layout_uses_full_chinese_localisation() {
    let yaml = emit_hoi4yaml(
        "整顿军队\n工业计划   互斥       农业改革\n",
        EmitHoi4YamlKind::FocusLayout,
        "SOV",
        "sov_alt",
    );

    assert!(yaml.contains("national_focus:"));
    assert!(yaml.contains("_file: \"sov_alt_SOV_focus\""));
    assert!(yaml.contains("id: \"sov_alt_SOV_focus_tree\""));
    assert!(yaml.contains("prereq: \"SOV_"));
    assert!(yaml.contains("mutually_exclusive: \"SOV_"));
    assert!(yaml.contains("localisation:\n  simp_chinese:"));
    assert!(yaml.contains("整顿军队"));
}

#[test]
fn emit_hoi4yaml_from_feature_cards_outputs_decisions_ideas_and_loc() {
    let yaml = emit_hoi4yaml(
        "决议：鼓励投资\n目标：SOV\n分类：经济政策\n效果：政治点+50\n\n民族精神：新经济政策\n图标：GFX_idea_democratic_planned_economy\n效果：稳定度+5%，消费品工厂-3%\n移除：不可手动移除\n",
        EmitHoi4YamlKind::FeatureCards,
        "SOV",
        "sov_nep",
    );

    assert!(yaml.contains("decisions_categories:"));
    assert!(yaml.contains("decisions:"));
    assert!(yaml.contains("ideas:"));
    assert!(yaml.contains("add_political_power: 50"));
    assert!(yaml.contains("stability_factor: 0.05"));
    assert!(yaml.contains("consumer_goods_factor: -0.03"));
    assert!(yaml.contains("removal_cost: -1"));
    assert!(yaml.contains("picture: \"democratic_planned_economy\""));
    assert!(!yaml.contains("picture: \"GFX_idea_democratic_planned_economy\""));
    assert!(yaml.contains("localisation:\n  simp_chinese:"));
}

#[test]
fn emit_hoi4yaml_from_feature_cards_outputs_technology_and_gui() {
    let yaml = emit_hoi4yaml(
        "独有科技：铁路调度算法\n目标：FER\n分类：engineering\n\n特殊GUI：铁路运力面板\n目标：FER\n用途：显示铁路运力。",
        EmitHoi4YamlKind::FeatureCards,
        "FER",
        "fer_rail",
    );

    assert!(yaml.contains("technologies:"));
    assert!(yaml.contains("fer_rail_technology_0_tech:"));
    assert!(yaml.contains("scripted_guis:"));
    assert!(yaml.contains("fer_rail_gui_1_gui:"));
    assert!(yaml.contains("interface:"));
    assert!(yaml.contains("fer_rail_gui_1_gui_window"));
    assert!(yaml.contains("fer_rail_technology_0_tech: \"铁路调度算法\""));
    assert!(yaml.contains("fer_rail_gui_1_gui: \"铁路运力面板\""));
}

#[test]
fn emit_hoi4yaml_from_feature_cards_outputs_scripted_helpers() {
    let yaml = emit_hoi4yaml(
        "脚本效果：铁路管制奖励\n效果：政治点+15\n\n脚本触发：战时铁路管制可用\n条件：战争中",
        EmitHoi4YamlKind::FeatureCards,
        "FER",
        "fer_rail",
    );

    assert!(yaml.contains("scripted_effects:"));
    assert!(yaml.contains("fer_rail_scripted_effect_0_effect:"));
    assert!(yaml.contains("_scope: \"country\""));
    assert!(yaml.contains("add_political_power: 15"));
    assert!(yaml.contains("scripted_triggers:"));
    assert!(yaml.contains("fer_rail_scripted_trigger_1_trigger:"));
    assert!(yaml.contains("has_war: true"));
}

#[test]
fn emit_hoi4yaml_from_feature_cards_outputs_state_effects() {
    let yaml = emit_hoi4yaml(
        "州效果：莫斯科工业修复\n州ID：64\n目标：FER\n建筑：军工+2\n资源：钢+8\n核心：FER",
        EmitHoi4YamlKind::FeatureCards,
        "FER",
        "fer_rail",
    );

    assert!(yaml.contains("scripted_effects:"));
    assert!(yaml.contains("_file: \"fer_rail_state_effects\""));
    assert!(yaml.contains("fer_rail_state_effect_0_state_effect:"));
    assert!(yaml.contains("_state_id: 64"));
    assert!(yaml.contains("add_core_of: \"FER\""));
    assert!(yaml.contains("TODO raw HOI4 block: add_resource = { type = steel amount = 8 }"));
}

#[test]
fn strict_emit_hoi4yaml_requires_game_root_before_output() {
    let root = unique_temp_dir("strict-emit-hoi4yaml");
    fs::create_dir_all(&root).unwrap();
    let input = root.join("cards.txt");
    let output = root.join("mod.yaml");
    fs::write(
        &input,
        "州效果：莫斯科工业修复\n州ID：64\n目标：FER\n资源：钢+8",
    )
    .unwrap();

    let err = cmd_emit_hoi4yaml(&[
        "--input".to_string(),
        input.to_string_lossy().to_string(),
        "--kind".to_string(),
        "feature-cards".to_string(),
        "--tag".to_string(),
        "FER".to_string(),
        "--prefix".to_string(),
        "fer_rail".to_string(),
        "--strict-code-index".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap_err();

    assert!(err.contains("strict emit-hoi4yaml requires --game-root"));
    assert!(!output.exists());
}

#[test]
fn strict_emit_hoi4yaml_focus_layout_reports_related_code_for_unindexed_reward() {
    let text = "工业复兴\n# completion_reward: 政治点+50\n";
    let yaml = emit_hoi4yaml(text, EmitHoi4YamlKind::FocusLayout, "SOV", "sov_ctx");
    let mut index = GameIndex::default();
    index
        .effects
        .insert("add_scaled_political_power".to_string());

    let err = enforce_strict_emit_hoi4yaml_gate(
        text,
        EmitHoi4YamlKind::FocusLayout,
        "SOV",
        "sov_ctx",
        &yaml,
        &index,
    )
    .unwrap_err();

    assert!(err.contains("strict emit-hoi4yaml blocked unresolved focus mappings"));
    assert!(err.contains("unindexed effect `add_political_power`"));
    assert!(err.contains("related indexed code: effects/effect `add_scaled_political_power`"));
}

#[test]
fn strict_emit_hoi4yaml_accepts_indexed_event_effects() {
    let root = unique_temp_dir("strict-emit-hoi4yaml-indexed");
    let game = root.join("game");
    fs::create_dir_all(game.join("documentation")).unwrap();
    fs::write(
        game.join("documentation").join("effects_documentation.md"),
        "## Effects\n\n## add_political_power\n",
    )
    .unwrap();
    let input = root.join("events.txt");
    let output = root.join("events.yaml");
    fs::write(
        &input,
        "事件：铁路预算\n命名空间：fer_rail\n选项A：通过\n效果A：政治点+25\n",
    )
    .unwrap();

    cmd_emit_hoi4yaml(&[
        "--input".to_string(),
        input.to_string_lossy().to_string(),
        "--kind".to_string(),
        "event-cards".to_string(),
        "--tag".to_string(),
        "FER".to_string(),
        "--prefix".to_string(),
        "fer_rail".to_string(),
        "--game-root".to_string(),
        game.to_string_lossy().to_string(),
        "--strict-code-index".to_string(),
        "--output".to_string(),
        output.to_string_lossy().to_string(),
    ])
    .unwrap();

    let yaml = read_utf8_lossy(&output).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(yaml.contains("events:"));
    assert!(yaml.contains("add_political_power: 25"));
}

#[test]
fn strict_emit_hoi4yaml_blocks_unresolved_yaml_markers() {
    let text = "州效果：莫斯科工业修复\n州ID：64\n目标：FER\n资源：钢+8";
    let yaml = emit_hoi4yaml(text, EmitHoi4YamlKind::FeatureCards, "FER", "fer_rail");
    let mut index = GameIndex::default();
    index.effects.insert("add_resource".to_string());
    index.resources.insert("steel".to_string());

    let err = enforce_strict_emit_hoi4yaml_gate(
        text,
        EmitHoi4YamlKind::FeatureCards,
        "FER",
        "fer_rail",
        &yaml,
        &index,
    )
    .unwrap_err();

    assert!(err.contains("strict emit-hoi4yaml blocked unresolved generated YAML markers"));
    assert!(err.contains("TODO raw HOI4 block"));
}

#[test]
fn strict_emit_hoi4yaml_blocks_unresolved_event_triggers_before_output() {
    let text = "事件：铁路争论\n触发：铁路委员会批准\n选项A：通过\n效果A：政治点+25\n";
    let yaml = emit_hoi4yaml(text, EmitHoi4YamlKind::EventCards, "FER", "fer_rail");
    let mut index = GameIndex::default();
    index.effects.insert("add_political_power".to_string());
    let err = enforce_strict_emit_hoi4yaml_gate(
        text,
        EmitHoi4YamlKind::EventCards,
        "FER",
        "fer_rail",
        &yaml,
        &index,
    )
    .unwrap_err();

    assert!(err.contains("strict event-card generation blocked unresolved AI mappings"));
    assert!(err.contains("铁路委员会批准"));
    assert!(err.contains("raw_trigger"));
}

#[test]
fn emit_hoi4yaml_from_event_cards_groups_event_types() {
    let yaml = emit_hoi4yaml(
        "事件：新政策争论\n类型：国家事件\n命名空间：sov_nep\n标题：新政策争论\n描述：党内出现争论。\n选项A：继续试验\n效果A：政治点+50\n\n事件：改革消息\n类型：新闻事件\n命名空间：sov_nep\n选项A：知道了\n隐藏效果A：设置旗标 nep_news_seen\n",
        EmitHoi4YamlKind::EventCards,
        "SOV",
        "sov_nep",
    );

    assert!(yaml.contains("events:"));
    assert!(yaml.contains("_namespace: \"sov_nep\""));
    assert!(yaml.contains("country_event:\n      - id: \"sov_nep.1\""));
    assert!(yaml.contains("news_event:\n      - id: \"sov_nep.2\""));
    assert_eq!(yaml.matches("country_event:").count(), 1);
    assert_eq!(yaml.matches("news_event:").count(), 1);
    assert!(yaml.contains("add_political_power: 50"));
    assert!(yaml.contains("set_country_flag: \"nep_news_seen\""));
    assert!(yaml.contains("sov_nep.2.a"));
}

#[test]
fn emit_hoi4yaml_from_event_cards_keeps_multiple_namespaces_in_one_file() {
    let yaml = emit_hoi4yaml(
        "事件：铁路会议\n类型：国家事件\n命名空间：fer_rail\n选项A：通过\n\n事件：边境新闻\n类型：新闻事件\n命名空间：fer_news\n选项A：知道了\n\n事件：铁路复会\n类型：国家事件\n命名空间：fer_rail\n选项A：继续\n",
        EmitHoi4YamlKind::EventCards,
        "FER",
        "fer_rail",
    );

    assert_eq!(yaml.matches("_file: \"fer_rail_events\"").count(), 1);
    assert!(yaml.contains("_namespaces:\n      - \"fer_news\"\n      - \"fer_rail\""));
    assert!(yaml.contains("country_event:\n      - id: \"fer_rail.1\""));
    assert!(yaml.contains("news_event:\n      - id: \"fer_news.1\""));
    assert!(yaml.contains("      - id: \"fer_rail.2\""));
}

#[test]
fn validator_warns_for_script_semantic_misuse() {
    let path = Path::new("M:\\mod\\common\\national_focus\\bad_focus.txt");
    let text = r#"
focus = {
	id = BAD_focus
	completion_reward = {
		news_event = { id = bad.1 title = bad.1.t desc = bad.1.d }
		modifier = { stability_factor = 0.05 }
		has_war = no
		add_building_construction = { type = arms_factory level = 1 }
	}
	available = {
		add_political_power = 50
	}
	limit = {
		add_stability = 0.05
	}
}
add_core_of = 123
add_core = SOV
capital = 64
"#;
    let mut reporter = Reporter::default();

    check_script_semantics(path, text, None, &mut reporter);

    assert!(reporter
        .warnings
        .iter()
        .any(|warning| warning.contains("news_event definition appears inside")));
    assert!(reporter
        .warnings
        .iter()
        .any(|warning| warning.contains("modifier = { ... } appears inside")));
    assert!(reporter
        .warnings
        .iter()
        .any(|warning| warning.contains("add_ideas")));
    assert!(reporter.warnings.iter().any(
        |warning| warning.contains("trigger-like condition `has_war` appears directly inside")
    ));
    assert!(reporter.warnings.iter().any(|warning| warning
        .contains("effect-like command `add_political_power` appears directly inside")));
    assert!(reporter
        .warnings
        .iter()
        .any(|warning| warning.contains("add_core_of usually expects a country tag")));
    assert!(reporter
        .warnings
        .iter()
        .any(|warning| warning.contains("check add_core/add_core_of direction")));
    assert!(reporter
        .warnings
        .iter()
        .any(|warning| warning.contains("enter a state scope first")));
    assert!(reporter
        .warnings
        .iter()
        .any(|warning| warning
            .contains("effect-like command `add_stability` appears directly inside")));
    assert!(reporter
        .warnings
        .iter()
        .any(|warning| warning.contains("capital = 64 cannot be verified")));

    let mut index = GameIndex::default();
    index.state_ids.insert(64);
    index.province_ids.insert(123);
    let mut indexed_reporter = Reporter::default();
    check_script_semantics(path, text, Some(&index), &mut indexed_reporter);
    assert!(indexed_reporter
        .warnings
        .iter()
        .any(|warning| warning.contains("capital = 64 matches a known state id")));
    assert!(!indexed_reporter
        .warnings
        .iter()
        .any(|warning| warning.contains("cannot be verified")));

    let mut province_reporter = Reporter::default();
    check_script_semantics(path, "capital = 999", Some(&index), &mut province_reporter);
    assert!(province_reporter
        .warnings
        .iter()
        .any(|warning| warning.contains("not present in the province index")));
}

#[test]
fn validator_errors_for_misspelled_focus_mutually_exclusive_field() {
    let path = Path::new("M:\\mod\\common\\national_focus\\bad_focus.txt");
    let text = r#"
focus = {
	id = BAD_left_branch
	mutual_exclusion = { focus = BAD_right_branch }
}
focus = {
	id = BAD_right_branch
	mutually_exclusive = { focus = BAD_left_branch }
}
"#;
    let mut reporter = Reporter::default();

    check_script_semantics(path, text, None, &mut reporter);

    assert!(reporter.errors.iter().any(|error| {
        error.contains("focus BAD_left_branch")
            && error.contains("unknown near-match field `mutual_exclusion`")
            && error.contains("exact HOI4 field `mutually_exclusive`")
    }));
}

#[test]
fn validator_rejects_scalar_focus_country_and_default_focus() {
    let path = Path::new("M:\\mod\\common\\national_focus\\bad_focus_tree.txt");
    let text = r#"
focus_tree = {
	id = kor_spring_focus
	country = KOR
	default_focus = KOR_people_uprising
	focus = {
		id = KOR_people_uprising
		icon = GFX_goal_unknown
		x = 0
		y = 0
		cost = 10
		ai_will_do = { factor = 100 }
		available = { }
		bypass = { }
		cancel_if_invalid = yes
		continue_if_invalid = no
		available_if_capitulated = no
		completion_reward = { }
	}
}
"#;
    let mut reporter = Reporter::default();

    check_script_semantics(path, text, None, &mut reporter);

    assert!(reporter
        .errors
        .iter()
        .any(|error| error.contains("scalar `country = KOR` is not loadable")));
    assert!(reporter
        .errors
        .iter()
        .any(|error| error.contains("unsupported `default_focus`")));
}

#[test]
fn validator_accepts_fixed_focus_country_selector() {
    let path = Path::new("M:\\mod\\common\\national_focus\\good_focus_tree.txt");
    let text = r#"
focus_tree = {
	id = kor_spring_focus
	country = {
		factor = 0
		modifier = {
			add = 10
			tag = KOR
		}
	}
}
"#;
    let mut reporter = Reporter::default();

    check_script_semantics(path, text, None, &mut reporter);

    assert!(reporter.errors.is_empty(), "{:?}", reporter.errors);
}

#[test]
fn validator_errors_for_other_near_match_focus_fields() {
    let path = Path::new("M:\\mod\\common\\national_focus\\bad_focus.txt");
    let text = r#"
focus = {
	id = BAD_focus
	prerequisites = { focus = BAD_parent }
	relative_position = BAD_parent
	completion_rewards = { add_political_power = 50 }
	ai_willdo = { factor = 1 }
	cancel_if_invald = yes
}
"#;
    let mut reporter = Reporter::default();

    check_script_semantics(path, text, None, &mut reporter);

    for (actual, expected) in [
        ("prerequisites", "prerequisite"),
        ("relative_position", "relative_position_id"),
        ("completion_rewards", "completion_reward"),
        ("ai_willdo", "ai_will_do"),
        ("cancel_if_invald", "cancel_if_invalid"),
    ] {
        assert!(reporter.errors.iter().any(|error| {
            error.contains(&format!("`{actual}`")) && error.contains(&format!("`{expected}`"))
        }));
    }
}

#[test]
fn validator_errors_for_event_namespace_and_near_match_fields() {
    let path = Path::new("M:\\mod\\events\\bad_events.txt");
    let text = r#"
namespace = bad
country_event = {
	id = bad.1
	is_trigger_only = yes
	fire_only_ones = yes
	mean_time_to_hapen = { days = 1 }
}
"#;
    let mut reporter = Reporter::default();
    let mut ids = BTreeMap::new();
    let mut namespaces = BTreeMap::new();

    collect_ids_and_namespaces(path, text, &mut ids, &mut namespaces, &mut reporter);
    check_script_semantics(path, text, None, &mut reporter);

    assert!(reporter
        .errors
        .iter()
        .any(|error| error.contains("use add_namespace")));
    for (actual, expected) in [
        ("is_trigger_only", "is_triggered_only"),
        ("fire_only_ones", "fire_only_once"),
        ("mean_time_to_hapen", "mean_time_to_happen"),
    ] {
        assert!(reporter.errors.iter().any(|error| {
            error.contains(&format!("`{actual}`")) && error.contains(&format!("`{expected}`"))
        }));
    }
}

#[test]
fn focus_layout_infers_branch_parents_before_mutation() {
    let json = parse_focus_layout_json(
        "斯大林宪法\n第一个五年计划   互斥       继续新经济政策\n快速工业化  强化国家       发财吧农民   奈普曼入党\n",
        "SOV",
        "sov_alt",
    );

    assert!(json.contains(
        "\"title\": \"快速工业化\", \"id\": \"SOV_rapid_industry\", \"icon\": null, \"x\": -3, \"y\": 2, \"worksheet_x\": -3, \"worksheet_y\": 2, \"relative_position_id\": \"SOV_stalin_constitution\", \"row\": 2, \"column\": 0, \"prerequisite\": [\"SOV_first_five_year_plan\"]"
    ));
    assert!(json.contains(
        "\"title\": \"发财吧农民\", \"id\": \"SOV_prosper_peasants\", \"icon\": null, \"x\": 1, \"y\": 2, \"worksheet_x\": 1, \"worksheet_y\": 2, \"relative_position_id\": \"SOV_stalin_constitution\", \"row\": 2, \"column\": 2, \"prerequisite\": [\"SOV_continue_new_economic_policy\"]"
    ));
    assert!(json.contains(
        "\"title\": \"奈普曼入党\", \"id\": \"SOV_nepman_join_party\", \"icon\": null, \"x\": 3, \"y\": 2"
    ) || json.contains("\"title\": \"奈普曼入党\"") && json.contains("\"x\": 3, \"y\": 2"));
}

#[test]
fn focus_layout_json_marks_unresolved_rewards_as_blocked() {
    let json = parse_focus_layout_json(
        "整训舰队\n# completion_reward: 获得民族精神 舰队整训\n",
        "SOV",
        "sov_alt",
    );

    assert!(json.contains("\"safety\": {\"status\": \"blocked\""));
    assert!(json.contains("\"final_code_allowed\": false"));
    assert!(json.contains("<idea id for 舰队整训>"));
    assert!(json.contains("completion_reward contains unresolved marker"));
}

#[test]
fn history_edit_plan_blocks_unverified_direct_state_edits() {
    let root = unique_temp_dir("history-plan-blocked");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("descriptor.mod"),
        "name=\"History Plan\"\nsupported_version=\"*\"\n",
    )
    .unwrap();
    let map = parse_args(&[
        "--state-id".to_string(),
        "64".to_string(),
        "--direct-history-edit".to_string(),
        "--tag".to_string(),
        "GER".to_string(),
    ]);

    let json = render_history_edit_plan(
        &root,
        &[],
        None,
        &map,
        "edit history/states owner for state_id 64",
    )
    .unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(json.contains("\"schema\": \"hoi4skill.history_edit_plan.v1\""));
    assert!(json.contains("\"recommended_strategy\": \"blocked_until_state_file_verified\""));
    assert!(json.contains("\"direct_history_edit_allowed\": false"));
    assert!(json.contains("\"safe_generated_targets\": []"));
    assert!(json.contains("local state/province facts are unknown"));
    assert!(
        json.contains("direct edit requested but no local target history/states file was verified")
    );
    assert!(json.contains("capital in history/countries is a province id"));
}

#[test]
fn history_edit_plan_allows_verified_local_state_file() {
    let root = unique_temp_dir("history-plan-verified");
    fs::create_dir_all(root.join("history").join("states")).unwrap();
    fs::create_dir_all(root.join("map")).unwrap();
    fs::write(
        root.join("descriptor.mod"),
        "name=\"History Plan\"\nsupported_version=\"*\"\n",
    )
    .unwrap();
    fs::write(
        root.join("history").join("states").join("64-Test.txt"),
        "state = { id = 64 name = \"STATE_64\" history = { owner = GER controller = GER add_core_of = GER victory_points = { 123 5 } buildings = { infrastructure = 3 } } provinces = { 123 456 } }\n",
    )
    .unwrap();
    fs::write(
        root.join("map").join("definition.csv"),
        "123;1;2;3;land;false;plains;1\n456;4;5;6;land;false;plains;1\n",
    )
    .unwrap();
    let map = parse_args(&[
        "--state-id".to_string(),
        "64".to_string(),
        "--province-id".to_string(),
        "123".to_string(),
        "--capital".to_string(),
        "123".to_string(),
        "--direct-history-edit".to_string(),
        "--tag".to_string(),
        "GER".to_string(),
    ]);

    let json = render_history_edit_plan(
        &root,
        &[],
        None,
        &map,
        "edit history/states owner for state_id 64 province_id 123",
    )
    .unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(json.contains("\"recommended_strategy\": \"direct_local_history_state_edit\""));
    assert!(json.contains("\"direct_history_edit_allowed\": true"));
    assert!(json.contains("\"state_file_local\": \"history/states/64-Test.txt\""));
    assert!(json.contains("\"province_id_known\": true"));
    assert!(json.contains("\"capital_province_id_known\": true"));
    assert!(json.contains("\"capital_value_also_state_id\": false"));
}
