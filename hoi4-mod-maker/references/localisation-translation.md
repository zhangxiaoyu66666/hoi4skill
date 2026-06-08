# Localisation Translation

Use this when the user asks to translate HOI4 localisation, copy another language into `simp_chinese`, or create another target language from existing localisation files.

## Fast Workflow

1. Identify the source and target language folder names, such as `english`, `simp_chinese`, `french`, `russian`, or `japanese`.
2. Compare key names and extract only missing source content:

```text
hoi4skill translate-localisation --mod-root "M:\path\mod" --from english --to simp_chinese --format prompt --output loc_translate_prompt.md
```

3. Translate only quoted values. Preserve every key exactly.
4. Put the translated `l_simp_chinese:` block in a temporary file, for example `translated_l_simp_chinese.yml`.
5. Inject the translated content back into the target language files:

```text
hoi4skill translate-localisation --mod-root "M:\path\mod" --from english --to simp_chinese --translated-input translated_l_simp_chinese.yml --apply --report loc_apply_report.json
```

The apply step maps source filenames to target filenames, for example:

```text
localisation/english/events_l_english.yml -> localisation/simp_chinese/events_l_simp_chinese.yml
```

6. Read `loc_apply_report.json`.
   - `written_keys`: keys injected from the translated file.
   - `existing_keys`: keys already present in the target language.
   - `missing_keys`: source keys without translated values.
   - `missing_after_apply`: source keys still absent after write-back.
   - `translated_unused_keys`: translated keys that do not match any source key, often typo evidence.
   - `suspicious_same_as_source`: values equal to the source language after apply; inspect them for untranslated text.
7. Run validation:

```text
hoi4skill validate "M:\path\mod"
```

## Mechanical Scaffold

To create target-language yml skeleton files before manual or AI translation:

```text
hoi4skill translate-localisation --mod-root "M:\path\mod" --from english --to simp_chinese --format yml --output-dir "M:\path\mod\localisation\simp_chinese"
```

The yml scaffold copies source values and writes comments saying they still need translation. Do not treat this scaffold as finished localisation.

Use `--overwrite` only after checking the target files. Without `--overwrite`, existing output files are skipped and reported.

Use `--include-existing` only when intentionally re-translating keys already present in the target language.

Use `--key-prefix TAG_` or `--key-prefix my_namespace.` to limit the batch.

## Closed-Loop Rule

Do not stop at prompt generation. The complete workflow is:

```text
compare keys -> extract source values -> translate quoted values -> apply translated values -> check missing_after_apply -> validate
```

If `missing_after_apply` is not empty, report the missing keys and do not claim the localisation pass is complete.

## Translation Rules

- Keep the first line as the target header, such as `l_simp_chinese:` or `l_english:`.
- Preserve keys exactly, including dots and suffixes such as `.t`, `.d`, `.a`, `_desc`, `_DEF`, and `_ADJ`.
- Translate only the quoted value.
- Preserve HOI4 scripted localisation and formatting tokens exactly.
- Do not translate or alter tokens like:
  - `$STATE|Y$`
  - `$VAR$`
  - `[ROOT.GetName]`
  - `[From.GetAdjective]`
  - `§Y`, `§R`, `§!`
  - `£pol_power`, `£stability_texticon`
  - `%`, `%%`, `\n`, `^`
- Keep escaped quotes valid in yml.
- Do not add `<prefix>_mod_name`, `chinaprc_1979_mod_name`, or any `*_mod_name` keys. Mod names belong in `descriptor.mod` and the launcher-side `.mod` file.

## Simplified Chinese Style

When the target is `simp_chinese`:

- Use natural Simplified Chinese, not literal machine translation.
- Keep country names, ideology terms, focus titles, decisions, and event options readable in HOI4 UI length.
- Translate event prose in a player-facing style, but do not rewrite facts or add new lore.
- Use Chinese punctuation unless a token requires ASCII punctuation.
- For focus and national-spirit descriptions, keep the established HOI4 copywriting rules from `focus-copywriting-prompt.md` and `decision-idea-cards.md`.

## Output Example

Source:

```yaml
l_english:
  SOV_new_order:0 "A New Order"
  SOV_new_order_desc:0 "The committee must protect $STATE|Y$ and [ROOT.GetName]."
```

Target:

```yaml
l_simp_chinese:
  SOV_new_order:0 "新秩序"
  SOV_new_order_desc:0 "委员会必须保卫$STATE|Y$与[ROOT.GetName]。"
```
