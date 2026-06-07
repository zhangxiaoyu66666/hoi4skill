# Install hoi4-mod-maker As An Agent Skill

`hoi4-mod-maker` is packaged as a standard `SKILL.md` Agent Skill. The release zip also includes the Windows `hoi4skill.exe` binary under the skill folder so agents can use the Rust backend without rebuilding first.

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

## Universal Zip Layout

The release zip contains drop-in layouts:

```text
.codex/skills/hoi4-mod-maker/
.claude/skills/hoi4-mod-maker/
.opencode/skills/hoi4-mod-maker/
.agents/skills/hoi4-mod-maker/
hoi4-mod-maker/
```

Extract the zip into your home directory for global installation, or into a project root for project-local installation. Use only the folders needed by your tool if you do not want all layouts.

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
