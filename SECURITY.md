# Security Policy

## Supported Versions

Only the latest public source version is supported.

## Reporting A Vulnerability

Open a private report or contact the maintainer before publishing exploit details.

Useful report details:

- affected command,
- exact CLI arguments,
- input file or minimal reproduction,
- expected behavior,
- actual behavior,
- whether the issue can overwrite files outside the chosen mod root.

## Security Boundaries

hoi4skill is a local modding tool. It reads and writes files selected by the user and may scan game or mod directories when asked.

Important expectations:

- It should not require network access for normal CLI operation.
- It should not execute generated HOI4 script.
- It should not silently overwrite unrelated files.
- It should report unknown state/province/game facts instead of inventing them.

Do not include private game installs, workshop mods, personal documents, or logs with sensitive paths in public reports unless they are minimized first.
