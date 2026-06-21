# Draft: hyprland-lua-workspace-labels

status: plan-written-high-accuracy-approved
plan_path: `.omo/plans/hyprland-lua-workspace-labels.md`
pending_action: user chooses whether to start implementation
intent: CLEAR
size: Standard

## User outcome

After Hyprland config moved from `hyprland.conf` bind lines to Lua config, Hyprline should continue displaying workspace letters (`Q W E ...`, `A S D ... ;`) instead of numeric workspace IDs.

## Evidence ledger

- `hyprline-bar/src/config/mod.rs:13-49`: current `parse_workspace_bindings()` only reads `$XDG_CONFIG_HOME/hypr/hyprland.conf` or `~/.config/hypr/hyprland.conf`, parses lines starting with `bind`, and extracts `parts[1]` as key and `parts[3]` as numeric workspace.
- `hyprline-bar/src/main.rs:453-491`: `workspace_keys` is parsed once at startup and cloned into each `Bar`.
- `hyprline-bar/src/ui/bar.rs:32-38,103-120,204-209,369-374`: `WidgetContext` stores the startup `workspace_keys`; `WorkspacesWidget` receives a cloned map during widget creation.
- `hyprline-bar/src/ui/workspaces.rs:36-75`: workspace labels are rendered from `workspace_keys.get(&ws_id)`, falling back to `ws_id.to_string()`.
- `hyprline-bar/src/infrastructure/hyprland_ipc.rs:54-83`: existing IPC request helper can already send `j/...` commands to Hyprland's control socket.
- `hyprline-bar/src/infrastructure/hyprland_submap.rs:257-330`: the project already uses `j/binds` and deserializes bind objects with `dispatcher`, `arg`, `key`, `modmask`, `submap`, etc.; this confirms the runtime-bind approach fits existing code.
- Hyprland `hyprctl.usage`: `binds` is an info command and `-j` outputs JSON; `workspace` is a dispatcher. Source: https://raw.githubusercontent.com/hyprwm/Hyprland/main/hyprctl/hyprctl.usage
- Explorer result `bg_ba1424f6`: confirmed there are no repo tests for the workspace-label path and that config hot-reload rebuilds widgets without re-parsing workspace labels.
- Explorer result `bg_4d5fbc5c`: confirmed `config.json` has no workspace label field today; adding a user override is possible only as a new additive serde-defaulted config field, but not necessary for the stated goal.
- Librarian result `bg_b3aa17e4`: confirmed current Hyprland docs prefer Lua APIs (`hl.bind`, `hl.workspace_rule`), old hyprlang-style bind lines are legacy/deprecated in current docs, and runtime binds/workspaces are available through IPC/hyprctl. Parsing source config is only needed for original authoring structure/comments, not for current registered binds.
- User follow-up: explicitly wants bind extraction but not by spawning `hyprctl`; asked about D-Bus. Decision: use direct Hyprland Unix-socket IPC (`j/binds`) rather than D-Bus because `hyprctl` itself is only a CLI wrapper over Hyprland IPC and the repo already uses direct socket requests for `j/workspaces`, `j/monitors`, and `j/binds`.
- Working tree is dirty before this plan (`git status --short` showed many modified files and untracked Bluetooth files); implementation must preserve unrelated user changes and stage only files intentionally changed for this feature.

## Components ledger

1. `runtime-bind-label-source` — derive workspace labels from live Hyprland binds instead of parsing config syntax. Status: decided. Evidence: existing `j/binds` use in `hyprland_submap.rs` and Hyprland docs.
2. `workspace-ui-data-flow` — make WorkspacesWidget ask the workspace service for labels instead of receiving a stale startup map. Status: decided. Evidence: current static flow in `main.rs`/`bar.rs`/`workspaces.rs`.
3. `fallback-and-compatibility` — keep numeric fallback and optionally preserve old `hyprland.conf` parser as secondary fallback. Status: decided. Evidence: README promises fallback to numbers; old parser exists.
4. `tests-and-docs` — add pure unit coverage for bind-to-label mapping and update README. Status: decided. Evidence: no existing tests found via `#[test]`/`mod tests` search.

## Recommended approach

Use direct Hyprland runtime IPC (`j/binds`, equivalent data to `hyprctl binds -j`, but without spawning the `hyprctl` command) as the primary label source. Do not parse Lua. Do not use D-Bus for this path unless future Hyprland docs expose binds there. Filtering rules:

- include only binds where `dispatcher == "workspace"`;
- include only plain positive numeric `arg` values (`"1"`, `"20"`), ignoring relative/special workspace args;
- ignore mouse binds and non-empty submaps for the main workspace indicator;
- display `key.to_uppercase()` to preserve the old visual behavior;
- if multiple binds target the same workspace, prefer non-numeric keys over numeric keys because the stated goal is letters instead of numbers;
- fall back to old `hyprland.conf` parser only when runtime binds are unavailable/empty;
- final fallback remains the numeric workspace ID.

## Must not have

- Must not attempt to parse arbitrary Lua loops or infer binds from `hl.workspace_rule`, because workspace rules do not encode key labels.
- Must not spawn `hyprctl`; use direct Unix-socket IPC.
- Must not add a D-Bus dependency for workspace bind discovery unless a documented Hyprland D-Bus bind API is found.
- Must not hardcode the user's QWERTY mapping as the only behavior.
- Must not remove numeric fallback.
- Must not overwrite unrelated dirty-worktree changes.

## Approval gate brief

Plan to write: a decision-complete implementation plan that changes Hyprline to derive workspace labels from Hyprland's live bind list (`j/binds`) with legacy and numeric fallbacks, plus tests/docs. Waiting for user approval before creating `.omo/plans/hyprland-lua-workspace-labels.md`.

## Plan generation notes

- User approved plan direction and clarified transport: direct Hyprland socket IPC, not D-Bus and not the `hyprctl` command.
- Scaffold script created `.omo/plans/hyprland-lua-workspace-labels.md`.
- Metis review completed and was folded in: remove old config parser as source, no `hyprctl` fallback, no live hot-reload scope creep, deterministic duplicate handling, parser tests, build gate, README update.
- High-accuracy review round 1 rejected:
  - Independent opencode/gpt-5.5 replacement for unavailable Codex CLI flagged unwanted commit requirement, weak Todo 3 failure QA, and grep exit-code fragility.
  - Momus flagged broad `hyprline-bar/src` zbus greps that would fail on unrelated dirty D-Bus changes.
- Plan edits applied: commit is no longer required by default; forbidden-path greps are scoped to intended feature files; Todo 3 failure QA now uses executable stale-parser grep; final gate grep handles no-match success explicitly.
- High-accuracy review round 2: Momus approved; independent opencode/gpt-5.5 replacement rejected two residual issues. Applied fixes: line-58 grep converted to explicit if/then pass-fail command; final verification note clarifies the post-verification user okay is not an implementation follow-up question.
- High-accuracy review round 3 Momus rejected two execution blockers. Applied fixes: every tee command now creates `.omo/evidence`; verification now captures a pre-edit baseline diff for intended target files and compares final target diff against that baseline to handle pre-existing dirty changes in `main.rs`.
- High-accuracy review final round: Momus returned OKAY; independent opencode/gpt-5.5 replacement for unavailable Codex CLI returned APPROVE. Exact Codex CLI could not be used because `codex`, `omo-codex`, and `lazycodex` were not installed in this environment.
