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
    let json = error_log_report_json(Path::new("M:\\logs\\error.log"), Some(&root), &diagnostics);
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
        &[root.clone()],
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
        game.join("interface").join("game_goals.gfx"),
        r#"spriteType = { name = "GFX_goal_game_factory" texturefile = "gfx/interface/goals/factory.dds" }"#,
    )
    .unwrap();
    let index = build_game_index(&game).unwrap();
    let layout = parse_focus_layout("工业复兴\n政治改革\n", "SOV", "sov_alt");

    apply_focus_layout_to_mod_with_index(&root, &layout, "SOV", "sov_alt", Some(&index)).unwrap();

    let focus_file = fs::read_to_string(
        root.join("common")
            .join("national_focus")
            .join("sov_alt_SOV_focus.txt"),
    )
    .unwrap();
    fs::remove_dir_all(&root).unwrap();
    fs::remove_dir_all(&game).unwrap();

    assert!(focus_file.contains("id = SOV_industry_revival"));
    assert!(focus_file.contains("icon = GFX_goal_game_factory"));
    assert!(focus_file.contains("id = SOV_political_reform"));
    assert!(focus_file.contains("icon = GFX_goal_local_political_reform"));
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
    let localisation_files = collect_files(&root.join("localisation")).unwrap();
    let localisation_contains_mod_name = localisation_files.iter().any(|path| {
        read_utf8_lossy(path)
            .map(|text| text.contains("_mod_name"))
            .unwrap_or(false)
    });
    fs::remove_file(&launcher_path).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(descriptor.contains("name=\"共和国一九七九：委员会民主\""));
    assert!(launcher.contains("name=\"共和国一九七九：委员会民主\""));
    assert!(launcher.contains("path="));
    assert!(!created.iter().any(|path| path
        .file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| name.ends_with("_l_simp_chinese.yml"))));
    assert!(!localisation_contains_mod_name);
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
        "民族精神：新经济政策复兴\n目标：SOV\n效果：稳定度+5%，建造速度+5%，消费品工厂-3%\n移除：不可手动移除\n\n决议：整训舰队\n目标：ITA\n可用：战争中\n效果：海军经验+25，陆军经验+5，空军经验+5，获得民族精神 舰队整训，触发新闻 海军改革，军工+3",
        "SOV",
        "sov_nep",
    );

    assert!(json.contains("\"code\": \"production_speed_buildings_factor = 0.05\""));
    assert!(json.contains("\"code\": \"consumer_goods_factor = -0.03\""));
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
    let focus_idea = root.join("interface").join("sov_nep_focus_idea_icons.gfx");
    let event = root.join("interface").join("sov_nep_event_pictures.gfx");
    let decision = root.join("interface").join("sov_nep_decision_pictures.gfx");

    assert_eq!(report.assets_scanned, 2);
    assert_eq!(report.entries.len(), 12);
    assert_eq!(report.changed_files.len(), 5);
    assert!(read_utf8_lossy(&dynamic)
        .unwrap()
        .contains(r#"name = "GFX_sov_nep_goals_sov_factory""#));
    assert!(read_utf8_lossy(&dynamic)
        .unwrap()
        .contains(r#"name = "GFX_sov_nep_goals_rebuild_southeast""#));
    assert!(read_utf8_lossy(&focus_idea)
        .unwrap()
        .contains(r#"name = "GFX_goal_sov_nep_goals_sov_factory""#));
    assert!(read_utf8_lossy(&focus_idea)
        .unwrap()
        .contains(r#"name = "GFX_idea_sov_nep_goals_sov_factory""#));
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
    assert!(!root
        .join("interface")
        .join("sov_nep_focus_idea_icons.gfx")
        .exists());

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
        read_utf8_lossy(&root.join("interface").join("sov_nep_focus_idea_icons.gfx"))
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
fn apply_feature_cards_writes_decisions_ideas_and_localisation() {
    let root = unique_temp_dir("apply-feature-cards");
    fs::create_dir_all(&root).unwrap();
    let cards = parse_cards(
        "决议：整训舰队\n目标：ITA\n分类：海军改革\n花费：50政治点\n冷却：30天\n可用：战争中\n效果：海军经验+25，军工+3\n描述：集中资源整训舰队。\n\n民族精神：舰队整训\n目标：ITA\n效果：稳定度+5%，战争支持+2%\n移除：不可手动移除\n描述：舰队整训正在提升国家动员能力。",
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
    assert!(focus_file.contains("cost = 2.5"));
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
    assert!(tree.contains("cost = 2.5"));
    assert!(tree.contains("factor = 100"));
    assert!(tree.contains("available = {\n\t\t}"));
    assert!(tree.contains("bypass = {\n\t\t}"));
    assert!(tree.contains("cancel_if_invalid = yes"));
    assert!(tree.contains("continue_if_invalid = no"));
    assert!(tree.contains("available_if_capitulated = no"));
    assert!(tree.contains("arms_factory"));
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
        "## stability_factor\n\n## political_power_factor\n",
    )
    .unwrap();
    fs::write(
        root.join("interface").join("goals.gfx"),
        r#"spriteType = { name = "GFX_game_focus_icon" texturefile = "gfx/interface/goals/game.dds" }"#,
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
    assert!(index.sprites.contains("GFX_game_focus_icon"));
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
    assert!(index.modifiers.contains("stability_factor"));
    assert!(json.contains("\"country_tags\": [\"ITA\", \"SOV\"]"));
    assert!(json.contains("\"state_ids\": [64]"));
    assert!(json.contains("\"state_names\": {\"STATE_64\": 64}"));
    assert!(json.contains("\"province_ids\": [123, 456, 789]"));
    assert!(json.contains("\"sprites\": [\"GFX_game_focus_icon\"]"));
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
    assert!(json.contains("\"modifiers\": [\"political_power_factor\", \"stability_factor\"]"));
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
fn render_focus_tree_uses_unknown_icon_and_empty_reward_by_default() {
    let layout = parse_focus_layout("根国策\n子国策\n", "CPC", "cpc_demo");
    let tree = render_focus_tree(&layout, "CPC");

    assert!(tree.contains("icon = GFX_goal_unknown"));
    assert!(tree.contains("# prerequisite = { focus = <parent focus id> }"));
    assert!(tree.contains("ai_will_do = {\n\t\t\tfactor = 100\n\t\t}"));
    assert!(tree.contains("completion_reward = {\n\t\t}\n"));
    assert!(!tree.contains("add_political_power = 50"));
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
        "focus_tree = {\n\tid = bad_tree\n\tcountry = { factor = 0 modifier = { add = 10 tag = SOV } }\n\tfocus = {\n\t\tid = SOV_real_focus\n\t\ticon = GFX_missing_icon\n\t\tx = 0\n\t\ty = 0\n\t\tprerequisite = { focus = SOV_missing_parent }\n\t\trelative_position_id = SOV_missing_relative\n\t\tcost = 2.5\n\t\tai_will_do = { factor = 100 }\n\t\tavailable = { ideology = mystery_ideology }\n\t\tbypass = { has_idea = mystery_idea }\n\t\tcancel_if_invalid = yes\n\t\tcontinue_if_invalid = no\n\t\tavailable_if_capitulated = no\n\t\tcompletion_reward = { set_technology = { mystery_tech = 1 } }\n\t}\n}\n",
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
    assert!(markdown.contains("真实 `GFX_goal*` 国策图标"));
    assert!(markdown.contains("icon = <verified GFX_goal* from interface/*.gfx"));
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
        "决议：鼓励投资\n目标：SOV\n分类：经济政策\n效果：政治点+50\n\n民族精神：新经济政策\n效果：稳定度+5%，消费品工厂-3%\n移除：不可手动移除\n",
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
        "\"title\": \"快速工业化\", \"id\": \"SOV_rapid_industry\", \"icon\": null, \"x\": -3, \"y\": 2, \"relative_position_id\": \"SOV_stalin_constitution\", \"row\": 2, \"column\": 0, \"prerequisite\": [\"SOV_first_five_year_plan\"]"
    ));
    assert!(json.contains(
        "\"title\": \"发财吧农民\", \"id\": \"SOV_prosper_peasants\", \"icon\": null, \"x\": 1, \"y\": 2, \"relative_position_id\": \"SOV_stalin_constitution\", \"row\": 2, \"column\": 2, \"prerequisite\": [\"SOV_continue_new_economic_policy\"]"
    ));
    assert!(json.contains(
        "\"title\": \"奈普曼入党\", \"id\": \"SOV_nepman_join_party\", \"icon\": null, \"x\": 3, \"y\": 2"
    ) || json.contains("\"title\": \"奈普曼入党\"") && json.contains("\"x\": 3, \"y\": 2"));
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
