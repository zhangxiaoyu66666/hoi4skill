# Install hoi4-mod-maker As An Agent Skill

`hoi4-mod-maker` is packaged as a standard `SKILL.md` Agent Skill. The release zip also includes the Windows `hoi4skill.exe` binary under the skill folder so agents can use the Rust backend without rebuilding first.

Version 0.3 focuses on large HOI4 mod workflows: local game/mod/dependency indexing, one-sentence and document-driven authoring, strict code-index validation, semantic repair context for bad AI output, focus/event/decision/national-spirit/dynamic-modifier/history/OOB/map/GUI planning, asset registration, runtime gates, and export manifests.

## Download

Download the latest `hoi4skill-agent-skill-v*.zip` from:

```text
https://github.com/zhangxiaoyu66666/hoi4skill/releases
```

## Codex

In Codex, ask the built-in skill installer to install the GitHub skill path:

```text
$skill-installer install https://github.com/zhangxiaoyu66666/hoi4skill/tree/main/hoi4-mod-maker
```

For manual installation, copy the skill folder to:

```text
~/.codex/skills/hoi4-mod-maker/
```

Then restart Codex so it discovers the new skill.

## Claude Code

For global installation, copy the skill folder to:

```text
~/.claude/skills/hoi4-mod-maker/
```

For project-only installation, copy it to:

```text
.claude/skills/hoi4-mod-maker/
```

Start Claude Code in the project and invoke it with:

```text
/hoi4-mod-maker
```

## OpenCode

For global installation, copy the skill folder to one of:

```text
~/.config/opencode/skills/hoi4-mod-maker/
~/.agents/skills/hoi4-mod-maker/
```

For project-only installation, copy it to one of:

```text
.opencode/skills/hoi4-mod-maker/
.agents/skills/hoi4-mod-maker/
.claude/skills/hoi4-mod-maker/
```

OpenCode loads matching `SKILL.md` files on demand through its skill tool.

## Release Zip Layout

The release zip contains one canonical skill folder:

```text
hoi4-mod-maker/
```

Copy that one folder into exactly one skill location supported by your agent. Do not install the same skill simultaneously under `.opencode/skills`, `.agents/skills`, `.claude/skills`, or versioned backup folders: agents may discover multiple `name: hoi4-mod-maker` entries and load a stale copy nondeterministically.

Before upgrading, remove or move the previous `hoi4-mod-maker` directory out of every skill-discovery root, then install the new canonical folder once. Backups must live outside skill-discovery directories.

After installing or upgrading, run the bundled self-check:

```text
hoi4-mod-maker/bin/windows-x64/hoi4skill.exe doctor-skill-install --fix
```

The command keeps the skill directory containing the running executable and automatically deletes other verified `hoi4-mod-maker` copies from Codex, Claude Code, OpenCode, and generic Agent Skill discovery roots. It refuses cleanup if it cannot identify the copy to keep.

## CLI Backend

On Windows, release packages include:

```text
bin/windows-x64/hoi4skill.exe
```

When the binary is not bundled or when using Linux/macOS, build it from source:

```text
cd hoi4skill-cli
cargo build --release
```

The skill remains GPL-3.0-only. Do not redistribute Hearts of Iron IV game files or third-party mod assets with your installed skill.
