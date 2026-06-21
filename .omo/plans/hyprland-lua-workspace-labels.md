# hyprland-lua-workspace-labels - Work Plan

## TL;DR (For humans)

**What you'll get:** Hyprline will show the real workspace hotkey letters again after the Hyprland Lua migration, by reading the active binds from Hyprland itself. If labels cannot be read, the bar keeps the existing numeric fallback instead of breaking.

**Why this approach:** Hyprland already exposes the registered binds over its Unix socket; using that socket avoids fragile Lua parsing and avoids spawning the `hyprctl` command.

**What it will NOT do:** It will not parse Lua, will not call `hyprctl`, will not add a D-Bus path for this, and will not hardcode your Q/W/E layout.

**Effort:** Short
**Risk:** Medium - the main risk is matching Hyprland's `j/binds` JSON shape and duplicate-bind behavior without disturbing the dirty working tree.
**Decisions to sanity-check:** Labels are loaded from direct socket IPC once at bar startup; no live bind hot-reload is added in this plan.

Your next move: choose whether to start implementation now or run a high-accuracy review of this plan first. Full execution detail follows below.

---

> TL;DR (machine): Short/medium-risk Rust change: replace `hyprland.conf` workspace-label parsing with direct Hyprland Unix-socket `j/binds` parsing, no subprocess, with parser tests and README update.

## Scope
### Must have
- Workspace labels must come from Hyprland's live registered bind list via direct Unix-socket IPC request `j/binds` from the existing Hyprland socket code path.
- The implementation must not spawn `hyprctl`; it must use the same socket style already used for `j/workspaces`, `j/monitors`, and submap `j/binds`.
- The label extraction must include only default/main-submap workspace-switch binds:
  - `dispatcher == "workspace"`.
  - `arg` is a plain positive integer workspace id such as `"1"` or `"20"`.
  - `mouse == false`.
  - `submap` is absent, `null`, or empty.
- The label text must remain the key only, uppercased, matching existing UI behavior (`Q`, `W`, `;`), not `SUPER+Q`.
- Duplicate binds for the same workspace must be deterministic: prefer the first valid non-numeric key; otherwise keep the first valid key. This favors letter labels over number labels without hardcoding QWERTY.
- Existing numeric fallback in the workspace widget must remain: if no label exists for a workspace id, display the workspace number.
- Add unit tests around the pure `j/binds` JSON-to-label-map parser before/with implementation.
- Update README text that currently says workspace hotkeys are read from `hyprland.conf`.

### Must NOT have (guardrails, anti-slop, scope boundaries)
- Must not parse Lua config or infer key labels from `hl.workspace_rule(...)`; workspace rules do not encode key labels.
- Must not call `hyprctl`, `std::process::Command`, shell, `jq`, or any external command for this feature.
- Must not add a D-Bus/zbus dependency or D-Bus bind-discovery path for workspace labels.
- Must not add config.json workspace-label overrides in this task; user asked for labels from binds.
- Must not add live bind hot-reload, config file watching, or Hyprland reload handling unless a later request asks for it.
- Must not refactor unrelated tray, notification, Bluetooth, brightness, submap UI, or bar layout code.
- Must not overwrite or stage unrelated dirty-worktree changes already present before this plan.

## Verification strategy
> Zero human intervention - all verification is agent-executed.
- Test decision: TDD with Rust unit tests in `hyprline-bar` plus build/static-scope gates.
- Evidence files:
  - `.omo/evidence/baseline-hyprland-lua-workspace-labels.diff`
  - `.omo/evidence/final-target-diff-hyprland-lua-workspace-labels.diff`
  - `.omo/evidence/task-1-hyprland-lua-workspace-labels.log`
  - `.omo/evidence/task-2-hyprland-lua-workspace-labels.log`
  - `.omo/evidence/task-3-hyprland-lua-workspace-labels.log`
  - `.omo/evidence/task-4-hyprland-lua-workspace-labels.log`
  - `.omo/evidence/task-5-hyprland-lua-workspace-labels.log`
  - `.omo/evidence/final-hyprland-lua-workspace-labels.log`
- Commands every worker must be able to run at the end:
  - Before Todo 1, run `mkdir -p .omo/evidence && git diff -- hyprline-bar/src/infrastructure/hyprland_ipc.rs hyprline-bar/src/main.rs hyprline-bar/src/config/mod.rs README.md > .omo/evidence/baseline-hyprland-lua-workspace-labels.diff` to capture pre-existing dirty changes in target files.
  - `cargo test -p hyprline-bar`
  - `cargo build -p hyprline-bar`
  - `if git diff -U0 -- hyprline-bar/src/infrastructure/hyprland_ipc.rs hyprline-bar/src/main.rs hyprline-bar/src/config/mod.rs | grep -E 'hyprctl|Command::new\("hyprctl"|std::process|zbus'; then echo 'forbidden runtime path found'; exit 1; else echo 'no forbidden runtime path added in feature files'; fi`. The grep is intentionally scoped to planned feature files because the working tree already contains unrelated D-Bus/zbus changes elsewhere.

## Execution strategy
### Parallel execution waves
> Target 5-8 todos per wave. Fewer than 3 (except the final) means you under-split.
- Wave 1 is sequential because parser tests define the contract, then implementation wires the parser into startup.
- Wave 2 can run docs and static-scope checks after the Rust behavior is green.
- Final verification runs after all todos and must approve plan compliance, code quality, real QA evidence, and scope fidelity.

### Dependency matrix
| Todo | Depends on | Blocks | Can parallelize with |
| --- | --- | --- | --- |
| 1 | none | 2, 3, 5 | none |
| 2 | 1 | 3, 5 | 4 after API names settle |
| 3 | 2 | 5 | 4 |
| 4 | 2 | 5 | 3 |
| 5 | 3, 4 | final verification | none |

## Todos
> Implementation + Test = ONE todo. Never separate.
<!-- APPEND TASK BATCHES BELOW THIS LINE WITH edit/apply_patch - never rewrite the headers above. -->
- [x] 1. `hyprline-bar/src/infrastructure/hyprland_ipc.rs`: Add red unit tests for parsing `j/binds` into workspace labels - expect deterministic label map and ignored non-workspace binds
  What to do / Must NOT do: Add a pure parser test suite before production behavior. The tests must define the JSON contract for a narrow bind DTO compatible with Hyprland's `j/binds` output. Do not test GTK, sockets, D-Bus, subprocesses, or user config files here.
  Parallelization: Wave 1 | Blocked by: none | Blocks: 2, 3, 5
  References (executor has NO interview context - be exhaustive):
  - `hyprline-bar/src/infrastructure/hyprland_ipc.rs:54-83` existing direct socket request helper.
  - `hyprline-bar/src/infrastructure/hyprland_submap.rs:257-330` existing `j/binds` JSON parsing shape (`dispatcher`, `arg`, `key`, `modmask`, `submap`, `mouse`, `release`, `repeat`).
  - Metis correction: only `dispatcher == "workspace"`; ignore `movetoworkspace`, special workspace args, relative args like `e+1`, mouse binds, and non-default submaps.
  Acceptance criteria (agent-executable):
  - Tests include JSON rows proving workspace `1` with key `"1"` maps to `{1: "1"}`.
  - Tests include JSON rows proving workspace `2` with key `"q"` maps to `{2: "Q"}`.
  - Tests include JSON rows proving `movetoworkspace`, `special:magic`, `e+1`, `-1`, `mouse: true`, and `submap: "resize"` are ignored.
  - Tests include duplicate rows proving a non-numeric key wins over a numeric key for the same workspace.
  - Running `cargo test -p hyprline-bar workspace` fails before production parser implementation or equivalent red evidence is captured.
  QA scenarios (name the exact tool + invocation):
  - Happy: `mkdir -p .omo/evidence && cargo test -p hyprline-bar workspace_bind_labels -- --nocapture 2>&1 | tee .omo/evidence/task-1-hyprland-lua-workspace-labels.log` shows the new tests are discovered.
  - Failure: Temporarily assert a wrong expected label in one test, run the same command, confirm failure in the evidence log, then restore the correct assertion before continuing.
  Commit: N | included with implementation commit `fix(workspaces): read labels from hyprland binds socket`

- [x] 2. `hyprline-bar/src/infrastructure/hyprland_ipc.rs`: Implement pure `j/binds` parser and socket-backed label loader - expect no config-file parsing and no subprocess
  What to do / Must NOT do: Add a private/narrow deserializable bind DTO or reuse/share the existing shape intentionally; implement a pure parser such as `parse_workspace_bind_labels(response: &str) -> Result<HashMap<i32, String>, serde_json::Error>` plus a public/concrete `HyprlandIpc` method such as `get_workspace_key_labels(&self) -> HashMap<i32, String>`. The loader must call existing direct socket request style with `send_request("j/binds")`. Do not change `send_request` batching behavior in this task; existing `j/workspaces`/`j/monitors` already depend on it. Do not spawn `hyprctl` and do not introduce D-Bus.
  Parallelization: Wave 1 | Blocked by: 1 | Blocks: 3, 4, 5
  References (executor has NO interview context - be exhaustive):
  - `hyprline-bar/src/infrastructure/hyprland_ipc.rs:6-13` concrete `HyprlandIpc` type.
  - `hyprline-bar/src/infrastructure/hyprland_ipc.rs:13-52` socket discovery.
  - `hyprline-bar/src/infrastructure/hyprland_ipc.rs:54-83` request/response helper.
  - `hyprline-bar/src/infrastructure/hyprland_ipc.rs:86-165` existing no-panic fallback style for IPC failures.
  - `hyprline-bar/src/infrastructure/hyprland_submap.rs:309-330` currently deserializes `j/binds` fields; keep field names compatible.
  Acceptance criteria (agent-executable):
  - `cargo test -p hyprline-bar workspace_bind_labels` passes.
  - Invalid JSON handling is non-panicking; parser returns `Err` and socket-backed loader returns an empty map on request/parse failure.
  - `git diff -U0 -- hyprline-bar/src/infrastructure/hyprland_ipc.rs | grep -E 'hyprctl|Command::new\("hyprctl"|std::process|zbus'` has no output.
  QA scenarios (name the exact tool + invocation):
  - Happy: `mkdir -p .omo/evidence && cargo test -p hyprline-bar workspace_bind_labels 2>&1 | tee .omo/evidence/task-2-hyprland-lua-workspace-labels.log` passes with mappings for numeric and letter keys.
  - Failure: Run `cargo test -p hyprline-bar workspace_bind_labels` after temporarily changing the parser filter to include `movetoworkspace`; confirm the ignore test fails in the evidence log, then restore the filter.
  Commit: N | included with implementation commit `fix(workspaces): read labels from hyprland binds socket`

- [x] 3. `hyprline-bar/src/main.rs` and `hyprline-bar/src/config/mod.rs`: Replace startup label source with direct socket labels - expect WorkspacesWidget receives labels from `j/binds`
  What to do / Must NOT do: In `build_ui`, keep a concrete `Arc<HyprlandIpc>` before erasing it to `Arc<dyn WorkspaceService + Send + Sync>`, then initialize `workspace_keys` from the new concrete socket-backed method. Remove the `use config::parse_workspace_bindings;` import and either delete `parse_workspace_bindings()` if it becomes unused or leave only if another caller exists. Do not add old `hyprland.conf` fallback; runtime binds are the source of truth and numeric UI fallback already exists.
  Parallelization: Wave 1 | Blocked by: 2 | Blocks: 5
  References (executor has NO interview context - be exhaustive):
  - `hyprline-bar/src/main.rs:7` current `parse_workspace_bindings` import.
  - `hyprline-bar/src/main.rs:113` currently constructs `Arc<dyn WorkspaceService>` directly from `HyprlandIpc::new()`.
  - `hyprline-bar/src/main.rs:453-491` current one-time label parse and bar construction.
  - `hyprline-bar/src/config/mod.rs:13-49` old `hyprland.conf` parser.
  - `hyprline-bar/src/ui/workspaces.rs:71-75` numeric fallback that must remain the only fallback when no label is found.
  Acceptance criteria (agent-executable):
  - `cargo test -p hyprline-bar workspace_bind_labels` passes after wiring.
  - `grep -R "parse_workspace_bindings" -n hyprline-bar/src` returns no production callers; if the function remains for tests or docs, justify in evidence.
  - `git diff -U0 -- hyprline-bar/src/main.rs hyprline-bar/src/config/mod.rs | grep -E 'hyprland.conf|parse_workspace_bindings|hyprctl|Command::new\("hyprctl"|std::process|zbus'` shows no newly added forbidden runtime path; removed old parser lines are acceptable.
  QA scenarios (name the exact tool + invocation):
  - Happy: `mkdir -p .omo/evidence && cargo test -p hyprline-bar workspace_bind_labels 2>&1 | tee .omo/evidence/task-3-hyprland-lua-workspace-labels.log` passes after wiring.
  - Failure: Temporarily reintroduce a `parse_workspace_bindings()` startup call or import in `main.rs`, run `grep -R "parse_workspace_bindings" -n hyprline-bar/src`, confirm the acceptance check catches the stale path in the evidence log, then restore the direct socket-backed call before continuing.
  Commit: N | included with implementation commit `fix(workspaces): read labels from hyprland binds socket`

- [x] 4. `README.md`: Update workspace hotkey docs - expect docs describe Hyprland socket binds, not `hyprland.conf` parsing
  What to do / Must NOT do: Update both English and Russian README sections that currently document `hyprland.conf` parsing. Say Hyprline reads registered workspace binds from Hyprland through its IPC socket, extracts `workspace` dispatcher keys, and falls back to numbers when unavailable. Do not claim Lua parsing, D-Bus, `hyprctl` command execution, or config.json overrides.
  Parallelization: Wave 2 | Blocked by: 2 | Blocks: 5
  References (executor has NO interview context - be exhaustive):
  - `README.md:130-153` English workspace keybinding documentation currently says `hyprland.conf` parsing.
  - `README.md:321-344` Russian workspace keybinding documentation currently says `hyprland.conf` parsing.
  - User decision: use socket, not command, not D-Bus.
  Acceptance criteria (agent-executable):
  - The English and Russian workspace-hotkey sections no longer contain stale phrases equivalent to “Locates `hyprland.conf`”, “Parses lines starting with `bind`”, “Находит `hyprland.conf`”, or “Парсит строки”.
  - README still documents fallback to numbers.
  QA scenarios (name the exact tool + invocation):
  - Happy: `mkdir -p .omo/evidence && grep -n "registered workspace binds\|зарегистрированные.*бин" README.md 2>&1 | tee .omo/evidence/task-4-hyprland-lua-workspace-labels.log` shows both language sections updated.
  - Failure: Search for `Parses lines starting with bind` and `Парсит строки`, confirm no stale old-parser wording remains; append result to same evidence log.
  Commit: N | included with implementation commit `fix(workspaces): read labels from hyprland binds socket`

- [x] 5. Repository gates and dirty-worktree safety: Run final local verification - expect tests/build pass and diff contains only intended scope
  What to do / Must NOT do: Run the full feature gate and inspect the diff. Because the repository already has unrelated modified/untracked files, do not stage or revert anything. Only report the files intentionally changed for this feature.
  Parallelization: Wave 2 | Blocked by: 3, 4 | Blocks: final verification
  References (executor has NO interview context - be exhaustive):
  - Dirty worktree was present before planning: many modified files and untracked Bluetooth files from `git status --short`; preserve all unrelated changes.
  - `Makefile:7-10` project build uses Cargo.
  - `hyprline-bar/Cargo.toml:1-22` package name is `hyprline-bar`.
  Acceptance criteria (agent-executable):
  - `cargo test -p hyprline-bar` passes.
  - `cargo build -p hyprline-bar` passes.
  - `git diff -- hyprline-bar/src/infrastructure/hyprland_ipc.rs hyprline-bar/src/main.rs hyprline-bar/src/config/mod.rs README.md > .omo/evidence/final-target-diff-hyprland-lua-workspace-labels.diff` runs after implementation.
  - `diff -u .omo/evidence/baseline-hyprland-lua-workspace-labels.diff .omo/evidence/final-target-diff-hyprland-lua-workspace-labels.diff` is inspected as the incremental feature delta. Allowed incremental hunks only: `hyprland_ipc.rs` parser/loader/tests, `main.rs` concrete `HyprlandIpc` construction and workspace label call, `config/mod.rs` removal of stale parser/export if needed, and README workspace-hotkey docs.
  - `git diff -U0 -- hyprline-bar/src/infrastructure/hyprland_ipc.rs hyprline-bar/src/main.rs hyprline-bar/src/config/mod.rs | grep -E 'Command::new\("hyprctl"|std::process|zbus'` has no output. The check is scoped to intended feature files to avoid unrelated pre-existing D-Bus/zbus diffs.
  QA scenarios (name the exact tool + invocation):
  - Happy: `mkdir -p .omo/evidence && (cargo test -p hyprline-bar && cargo build -p hyprline-bar && git diff -- hyprline-bar/src/infrastructure/hyprland_ipc.rs hyprline-bar/src/main.rs hyprline-bar/src/config/mod.rs README.md > .omo/evidence/final-target-diff-hyprland-lua-workspace-labels.diff && diff -u .omo/evidence/baseline-hyprland-lua-workspace-labels.diff .omo/evidence/final-target-diff-hyprland-lua-workspace-labels.diff || true && git diff --stat && if git diff -U0 -- hyprline-bar/src/infrastructure/hyprland_ipc.rs hyprline-bar/src/main.rs hyprline-bar/src/config/mod.rs | grep -E 'Command::new\("hyprctl"|std::process|zbus'; then echo 'forbidden runtime path found'; exit 1; else echo 'no forbidden runtime path added in feature files'; fi) 2>&1 | tee .omo/evidence/task-5-hyprland-lua-workspace-labels.log` records passing gates, baseline-aware delta, and forbidden-path check.
  - Failure: If any command fails, stop and fix before final verification; evidence log must include the failing command and the later passing rerun.
  Commit: N | do not commit unless the user explicitly asks after implementation

## Final verification wave
> Runs in parallel after ALL todos. ALL must APPROVE. Surface results and wait for the user's explicit okay before declaring complete. This final okay is a post-verification completion acknowledgement only; the implementer must not ask design/scope questions during execution because this plan is decision-complete.
- [x] F1. Plan compliance audit
  - Read this plan and the final diff.
  - Verify every Must Have is satisfied and every Must NOT Have is absent.
  - Required command: `git diff -- hyprline-bar/src/infrastructure/hyprland_ipc.rs hyprline-bar/src/main.rs hyprline-bar/src/config/mod.rs README.md > .omo/evidence/final-target-diff-hyprland-lua-workspace-labels.diff && diff -u .omo/evidence/baseline-hyprland-lua-workspace-labels.diff .omo/evidence/final-target-diff-hyprland-lua-workspace-labels.diff || true`.
  - Evidence: append verdict to `.omo/evidence/final-hyprland-lua-workspace-labels.log`.
- [x] F2. Code quality review
  - Review Rust changes for no panics, no subprocesses, no unnecessary clones beyond existing UI map cloning, no oversized function, and deterministic duplicate handling.
  - Required commands: `cargo test -p hyprline-bar` and `cargo build -p hyprline-bar`.
  - Evidence: append verdict to `.omo/evidence/final-hyprland-lua-workspace-labels.log`.
- [x] F3. Real manual QA
  - If running under Hyprland socket environment, run the bar or a small debug invocation using existing binary workflow and confirm labels appear as letters for current binds. If not under Hyprland, document environment limitation and rely on parser fixture tests plus compile gate.
  - Required non-interactive fallback command: `cargo test -p hyprline-bar workspace_bind_labels -- --nocapture`.
  - Evidence: append verdict and environment note to `.omo/evidence/final-hyprland-lua-workspace-labels.log`.
- [x] F4. Scope fidelity
  - Verify no unrelated dirty files were staged/modified for this feature.
  - Required commands: `git status --short`, `git diff --stat`, and `diff -u .omo/evidence/baseline-hyprland-lua-workspace-labels.diff .omo/evidence/final-target-diff-hyprland-lua-workspace-labels.diff || true`.
  - Evidence: append verdict to `.omo/evidence/final-hyprland-lua-workspace-labels.log`.

## Commit strategy
- Do not commit by default; the user has not requested a commit.
- If the user explicitly asks for a commit after implementation, use one atomic commit:
  - `fix(workspaces): read labels from hyprland binds socket`
- If committing is requested, stage only intended feature files. Expected candidate paths:
  - `hyprline-bar/src/infrastructure/hyprland_ipc.rs`
  - `hyprline-bar/src/main.rs`
  - `hyprline-bar/src/config/mod.rs` only if deleting/removing old parser/imports requires it
  - `README.md`
- Do not stage `.omo/` unless the user explicitly asks to commit planning artifacts.
- Before committing, inspect `git status --short`, `git diff`, and `git log --oneline -10`.

## Success criteria
- Hyprline no longer depends on `hyprland.conf` syntax for workspace labels.
- Hyprline does not spawn `hyprctl` and does not use D-Bus for workspace bind discovery.
- Workspace `j/binds` fixture tests prove letters such as `q` become `Q` and invalid/non-workspace binds are ignored.
- Workspace buttons still fall back to numeric IDs when no valid label exists.
- README accurately documents direct Hyprland IPC bind discovery in both English and Russian.
- `cargo test -p hyprline-bar` and `cargo build -p hyprline-bar` pass.
