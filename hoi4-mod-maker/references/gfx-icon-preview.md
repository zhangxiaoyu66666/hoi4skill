# GFX Icon Preview

Use this when a feature needs focus icons, idea pictures, decision icons, event pictures, or when the user asks to inspect available art.

## Asset Rules

- HOI4 commonly uses `.dds` for icons.
- `.png` can also be used in mods and is convenient while drafting.
- Existing large mods may mix `.dds`, `.png`, and `.tga`.
- Do not reference a new custom image unless the file exists or the change also adds it.
- Do not assume sprite names start with `GFX_`; large mods often use bare names such as `CPC_Rebuilding_Southeast_China`.

## Sprite Mapping

Icons are usually connected through `interface/*.gfx`:

```hoi4
spriteType = {
	name = "GFX_my_mod_focus_icon"
	texturefile = "gfx/interface/goals/my_mod_focus_icon.dds"
}
```

When generating or changing icons:

1. Scan `interface/*.gfx` for existing `spriteType` names.
2. Resolve `texturefile` to `gfx/interface/...`.
3. Reuse an existing sprite when it matches the feature.
4. If adding a new PNG, DDS, or TGA, add a matching `spriteType`.
5. Update focus `icon`, idea `picture`, decision `icon`, or category `picture` with the reference expected by that system. For ideas, a registered `GFX_idea_<name>` sprite is referenced as `picture = <name>`.

## Batch Registration Command

Run this after placing new images under `gfx/interface`:

```text
hoi4skill register-gfx-icons --mod-root "<mod-root>" --prefix sov_nep --category all --output gfx_report.json
```

Before writing sprite registrations, the command normalizes image filenames:

- If the image filename is already English/ASCII, it is not renamed.
- If the image filename contains Chinese or other non-ASCII text, the command translates the local filename into a semantic English filename, renames the asset in place, updates existing `interface/*.gfx` `texturefile` references that used the old path, then registers the new English path.
- If the filename cannot be translated by the built-in HOI4 term dictionary, the command skips that image and reports it in `skipped_assets`. Do not fall back to random or numeric names such as `SOV_12347.png`.
- The generated `.gfx` block and JSON report include remarks showing the original filename and the English rename.

Categories:

- `dynamic`: writes `GFX_<prefix>_<asset>` into `interface/<prefix>_dynamic_icons.gfx` for scripted GUI and localisation icon control codes.
- `focus`: writes `GFX_goal_<prefix>_<asset>` into `interface/<prefix>_goals.gfx` and also writes the matching `_shine` sprite into `interface/<prefix>_goals_shine.gfx`.
- `idea`: writes `GFX_idea_<prefix>_<asset>` into `interface/<prefix>_focus_idea_icons.gfx`; use only `<prefix>_<asset>` in `common/ideas` `picture =`.
- `event`: writes `GFX_report_event_<prefix>_<asset>` into `interface/<prefix>_event_pictures.gfx`.
- `decision`: writes both `GFX_decision_<prefix>_<asset>` and `GFX_decision_category_<prefix>_<asset>` into `interface/<prefix>_decision_pictures.gfx`.
- `focus-idea`: shorthand for both focus and idea.
- `all`: default full set.

The command scans existing `interface/*.gfx` first. If the desired sprite name already points to the same `texturefile`, it is reported as `existing` and not written again. If the desired sprite name points to a different `texturefile`, the command appends a stable numeric suffix such as `_2` and reports the avoided conflict. This suffix is only for sprite-name collisions after a semantic English base name exists; it is not a random replacement for translation. The JSON report also lists `existing_names_for_texture` so an author can reverse-check which sprite names already point at the same image. When `assets_skipped` is non-zero, the AI-facing final output must tell the user which files were skipped and ask for a semantic English filename or dictionary expansion.

For focus icons, registration is two-part by default:

- `goals`: `SpriteType = { name = "GFX_goal_<prefix>_<asset>" texturefile = "..." }`
- `goals_shine`: matching `spriteType = { name = "GFX_goal_<prefix>_<asset>_shine" ... }` with `effectFile = "gfx/FX/buttonstate.lua"`, the standard double scrolling `shine_overlay.dds` animations, and `legacy_lazy_load = no`

The scanner accepts both `spriteType` and `SpriteType` when reading existing files.

## Preview Command

Run:

```text
hoi4skill icon-preview --mod-root "<mod-root>"
```

Optional:

```text
hoi4skill icon-preview --mod-root "<mod-root>" --output "M:\preview\icons" --max-icons 2000
```

The command creates:

- `index.html`: searchable icon gallery.
- `icons.tsv`: sprite name, texture path, local path, preview status.
- `assets/*.png`: generated preview images when possible.

PNG previews are copied directly. DDS previews are attempted through Windows imaging support. If the local Windows codec cannot decode a DDS format, the icon still appears in the manifest and HTML with `dds preview unavailable`.

## Workflow Placement

Use icon preview:

- before choosing an icon for a generated focus, idea, or decision,
- after adding new icon assets,
- when validating that a sprite key points to a real texture file,
- when adapting to a large mod with many custom icons.

## Fallbacks

If no custom icon is available:

- focus: reuse a safe generic focus sprite such as `GFX_goal_generic_construct_civ_factory`, after checking nearby mod style,
- idea: reuse an existing idea picture key from the target mod or vanilla,
- decision: use a generic decision icon such as `generic_political_discourse`,
- event picture: use a known existing report event picture.

Never invent a `texturefile` path and assume the game will find it.
