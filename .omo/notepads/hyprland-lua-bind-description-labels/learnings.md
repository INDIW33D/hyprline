- Added red-only `workspace_bind_labels_*` tests in `hyprland_ipc.rs` for described `__lua` workspace binds using Hyprland-shaped JSON fixtures with `has_description` and `description` fields.
- Current parser behavior ignores all `dispatcher="__lua"` rows, so the new positive description cases fail with `{}` or legacy-only labels, which is the expected TDD red state before parser changes.
- Captured the required temporary failure probe by changing `semicolon -> "SEMICOLON"`, running the targeted test command, then restoring the real expectation to `";"`.
- `git diff -- hyprline-bar/src/infrastructure/hyprland_ipc.rs` still shows older unstaged non-test changes versus HEAD, so task verification relied on keeping new edits confined to the test module rather than assuming a clean baseline in this working tree.
- Production parsing now treats `description` metadata as the preferred source when `has_description` is true, the bind is not a mouse bind, and `submap` is empty; only exact `hyprline:workspace:<positive-int>:<label>` descriptions are accepted.
- Shared label normalization must stay centralized because both description-derived and legacy-derived labels need identical `semicolon -> ;` handling plus uppercase normalization for all other labels.
- Source-aware precedence is required to satisfy all fixtures: description labels always beat legacy labels for the same workspace, the first description wins deterministically, and legacy rows still keep their old non-numeric-over-numeric override rule.
- Updated README.md `### Workspace Keybindings` and `### Горячие клавиши воркспейсов` to document the strict `hyprline:workspace:<id>:<label>` description format, uppercase display, `semicolon -> ;` normalization, and numeric fallback.
- Confirmed the README check catches missing Russian metadata via a temporary failure probe, then restored the example and re-ran the full verification command to produce the evidence log.

