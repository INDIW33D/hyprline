# workspace-click-lua-dispatch-fallback - Work Plan

## TL;DR (For humans)
<!-- Fill this LAST, after the detailed plan below is written, so it summarizes the REAL plan. -->
<!-- Plain English for a non-engineer: NO file paths, NO todo numbers, NO wave/agent/tool names. -->

**What you'll get:** Clicking a workspace label will switch to that workspace again even when Hyprland is running with a Lua config. The old command path remains supported for non-Lua configs.

**Why this approach:** Live socket evidence shows the click path sends Hyprland's old `dispatch workspace <id>` command, which now errors in Lua-dispatch mode, while `dispatch hl.dsp.focus({ workspace = <id> })` returns `ok`. The safest fix is to keep the legacy command first and retry once with the Lua-compatible command only for that known error.

**What it will NOT do:** It will not edit your Hyprland Lua config, will not add `hyprctl` or subprocesses to runtime, and will not rewrite the workspace UI event layer unless the IPC fix is proven insufficient.

**Effort:** Short
**Risk:** Medium - command fallback must preserve legacy Hyprland behavior while fixing Lua-dispatch mode and avoiding silent false positives.
**Decisions to sanity-check:** Legacy-first fallback; retry only on known Lua-dispatch errors; keep the public `switch_workspace(&self, id: i32)` trait unchanged and log failures.

Your next move: start work with the worker, or ask for a high-accuracy plan review first. Full execution detail follows below.

---

> TL;DR (machine): Short/medium-risk Rust IPC bug fix: make `switch_workspace` response-aware, keep `dispatch workspace <id>` legacy first, retry once with `dispatch hl.dsp.focus({ workspace = <id> })` only on known Lua-dispatch command errors, prove with fake Unix socket tests + live direct-socket QA.

## Scope
### Must have
- Preserve the existing UI click path: `ui/workspaces.rs` may keep calling `WorkspaceService::switch_workspace(ws_id)`.
- Make `HyprlandIpc::switch_workspace(id)` inspect the Hyprland socket response instead of write-and-ignore.
- Keep legacy behavior first: send `dispatch workspace <id>` before any fallback.
- Treat `ok` (after trimming whitespace) as success and do not send fallback.
- Retry exactly once with `dispatch hl.dsp.focus({ workspace = <id> })` when the legacy response is a known Lua-dispatch command error. Known markers: response contains `hl.dispatch`, `dispatch in lua`, or `syntax might need to be updated`.
- Format the fallback workspace from the typed numeric `i32` id only; do not interpolate label text or user-controlled strings.
- Do not retry on socket connection/write/read failures; log them and return.
- If both legacy and fallback command responses are not `ok`, log enough context for diagnosis and return without panic.
- Add fake Unix socket tests for legacy success, Lua fallback sequence, double command failure/no panic, and transport failure/no panic.
- Keep all existing workspace label parser tests passing.
- Capture live direct-socket QA showing the observed legacy command error and Lua fallback `ok` under the current Hyprland Lua config.

### Must NOT have (guardrails, anti-slop, scope boundaries)
- Must not edit `/home/indiw33d/.config/hypr/hyprland.lua`.
- Must not add `hyprctl`, `Command::new`, `std::process`, shelling out, or any runtime subprocess for workspace switching.
- Must not add a D-Bus/zbus workspace switching path.
- Must not change workspace label parsing, README docs, CSS, tray, Bluetooth, notifications, brightness, settings, or unrelated UI layout.
- Must not rewrite the workspace click event controller in this plan unless the IPC fallback is implemented and evidence still proves click events do not reach `switch_workspace`.
- Must not change the public `WorkspaceService` trait signature unless tests prove it is unavoidable.
- Must not stage, commit, revert, or overwrite unrelated dirty-worktree changes.

## Verification strategy
> Zero human intervention - all verification is agent-executed.
- Test decision: TDD with Rust unit tests/fake Unix socket tests in `hyprline-bar`; live direct-socket QA after green tests.
- Evidence files:
  - `.omo/evidence/baseline-status-workspace-click-lua-dispatch-fallback.txt`
  - `.omo/evidence/baseline-target-diff-workspace-click-lua-dispatch-fallback.diff`
  - `.omo/evidence/baseline-unrelated-tracked-workspace-click-lua-dispatch-fallback.diff`
  - `.omo/evidence/final-unrelated-tracked-workspace-click-lua-dispatch-fallback.diff`
  - `.omo/evidence/task-1-workspace-click-lua-dispatch-fallback.log`
  - `.omo/evidence/task-2-workspace-click-lua-dispatch-fallback.log`
  - `.omo/evidence/task-3-workspace-click-lua-dispatch-fallback.log`
  - `.omo/evidence/final-target-diff-workspace-click-lua-dispatch-fallback.diff`
  - `.omo/evidence/final-workspace-click-lua-dispatch-fallback.log`
- Before product edits, capture dirty-worktree baseline:
  ```bash
  set -e
  mkdir -p .omo/evidence && \
  git status --short > .omo/evidence/baseline-status-workspace-click-lua-dispatch-fallback.txt && \
  git diff -- hyprline-bar/src/infrastructure/hyprland_ipc.rs > .omo/evidence/baseline-target-diff-workspace-click-lua-dispatch-fallback.diff && \
  git diff -- . ':(exclude)hyprline-bar/src/infrastructure/hyprland_ipc.rs' ':(exclude).omo/evidence/**' > .omo/evidence/baseline-unrelated-tracked-workspace-click-lua-dispatch-fallback.diff
  ```
- Required final commands:
  - `cargo test -p hyprline-bar workspace_switch -- --nocapture`
  - `cargo test -p hyprline-bar workspace_ -- --nocapture`
  - `cargo test -p hyprline-bar`
  - `cargo build -p hyprline-bar`
  - `if git diff -U0 -- hyprline-bar/src/infrastructure/hyprland_ipc.rs | grep -E '^\+[^+].*(hyprctl|Command::new|std::process|read_to_string|zbus|dbus)'; then echo 'forbidden runtime path found'; exit 1; else echo 'no forbidden runtime path added'; fi`

## Execution strategy
### Parallel execution waves
> Target 5-8 todos per wave. Fewer than 3 (except the final) means you under-split.
- Wave 1 is sequential TDD: fake-socket tests first, implementation second.
- Wave 2 runs repository gates and live direct-socket QA after tests are green.
- Final verification runs after all todos; all reviewers must approve before declaring complete.

### Dependency matrix
| Todo | Depends on | Blocks | Can parallelize with |
| --- | --- | --- | --- |
| 1 | none | 2, 3 | none |
| 2 | 1 | 3 | none |
| 3 | 2 | final verification | none |

## Todos
> Implementation + Test = ONE todo. Never separate.
<!-- APPEND TASK BATCHES BELOW THIS LINE WITH edit/apply_patch - never rewrite the headers above. -->
- [x] 1. `hyprline-bar/src/infrastructure/hyprland_ipc.rs`: Add fake Unix socket red tests for workspace switching - expect legacy success, Lua fallback, and failures are observable
  What to do / Must NOT do: Add tests before production changes. Keep tests in the existing `#[cfg(test)] mod tests` module or a nested test helper module. Do not change `switch_workspace` implementation in this todo. Do not require a live Hyprland instance. Do not add external test dependencies unless a standard-library fake socket is genuinely insufficient.
  Parallelization: Wave 1 | Blocked by: none | Blocks: 2, 3
  References (executor has NO interview context - be exhaustive):
  - `hyprline-bar/src/ui/workspaces.rs:89-98`: workspace labels call `service.switch_workspace(ws_id)` on button press.
  - `hyprline-bar/src/infrastructure/hyprland_ipc.rs:23-62`: `get_control_socket()` resolves the socket from `HYPRLAND_INSTANCE_SIGNATURE` and `XDG_RUNTIME_DIR`, which tests can point at a fake temporary socket.
  - `hyprline-bar/src/infrastructure/hyprland_ipc.rs:64-93`: `send_request()` writes a command and reads the response until EOF; fake socket must write response and close.
  - `hyprline-bar/src/infrastructure/hyprland_ipc.rs:289-296`: current `switch_workspace` sends `dispatch workspace {id}` and ignores response/errors.
  - Live evidence from planning: `dispatch workspace 11` returned a Lua-dispatch syntax error; `dispatch hl.dsp.focus({ workspace = 11 })` returned `ok`.
  - Metis `ses_115e0c2a5ffemwl5LuZsXsYdS6`: tests must prove legacy success no fallback, fallback command sequence, double failure/no panic, transport failure/no panic, and no live Hyprland dependency.
  Acceptance criteria (agent-executable):
  - Add a fake Unix socket helper that accepts one command per connection, records commands in order, writes configured responses, then closes the connection.
  - Add an environment guard (`static Mutex` or equivalent) around tests that modify `HYPRLAND_INSTANCE_SIGNATURE` / `XDG_RUNTIME_DIR`; restore previous environment values after each test.
  - Red test: when fake socket responds `ok\n` to `dispatch workspace 7`, `switch_workspace(7)` sends exactly one command: `dispatch workspace 7`.
  - Red test: when fake socket responds with the observed Lua-dispatch error to `dispatch workspace 7`, then `ok` to fallback, `switch_workspace(7)` sends exactly two commands: `dispatch workspace 7`, then `dispatch hl.dsp.focus({ workspace = 7 })`.
  - Red test: when fake socket returns Lua-dispatch error then `error: fallback failed`, `switch_workspace(7)` sends exactly those two commands and does not panic.
  - Red test: when no fake socket exists / connect fails, `switch_workspace(7)` does not panic.
  - Existing `workspace_bind_labels` tests remain unchanged.
  QA scenarios (name the exact tool + invocation):
  - Happy/red proof: `set -o pipefail; mkdir -p .omo/evidence && (cargo test -p hyprline-bar workspace_switch -- --nocapture) 2>&1 | tee .omo/evidence/task-1-workspace-click-lua-dispatch-fallback.log` discovers the new tests and fails before production changes, unless prior implementation already exists; record the actual result.
  - Failure proof: Temporarily change the expected fallback string to `dispatch workspace 7`, run the targeted command, confirm the fallback-sequence test fails, then restore the correct expected string before continuing.
  Commit: N | included with implementation commit only if user later requests commit

- [x] 2. `hyprline-bar/src/infrastructure/hyprland_ipc.rs`: Implement response-aware workspace dispatch fallback - expect clicks work in legacy and Lua-dispatch Hyprland modes
  What to do / Must NOT do: Refactor `switch_workspace` to use response-aware socket sending. Keep `WorkspaceService::switch_workspace(&self, id: i32)` returning `()`. Try legacy `dispatch workspace <id>` first; retry exactly once with Lua-compatible `dispatch hl.dsp.focus({ workspace = <id> })` only for known Lua-dispatch command errors. Do not touch `ui/workspaces.rs`, parser tests, README, CSS, or local Hyprland config.
  Parallelization: Wave 1 | Blocked by: 1 | Blocks: 3
  References (executor has NO interview context - be exhaustive):
  - `hyprline-bar/src/infrastructure/hyprland_ipc.rs:64-93`: prefer reusing or narrowly extracting `send_request()` so commands read a response.
  - `hyprline-bar/src/infrastructure/hyprland_ipc.rs:289-296`: current write-and-ignore behavior to replace.
  - Live evidence: legacy response contains `hl.dispatch`, `dispatch in lua`, and `syntax might need to be updated`; Lua fallback response is `ok`.
  - `hyprline-bar/src/domain/workspace_service.rs:4-12`: trait signature should remain unchanged.
  Acceptance criteria (agent-executable):
  - `switch_workspace(id)` sends `dispatch workspace {id}` first.
  - A response whose trimmed text equals `ok` is treated as success and does not fallback.
  - Fallback trigger is exact and documented in code via a helper such as `should_try_lua_workspace_fallback(response: &str)`: non-`ok` responses containing `hl.dispatch`, `dispatch in lua`, or `syntax might need to be updated`.
  - Fallback command is exactly `dispatch hl.dsp.focus({ workspace = {id} })`.
  - No fallback on socket connect/write/read errors; log with `eprintln!` and return.
  - If fallback response is non-`ok`, log both relevant failure context and return without panic.
  - `cargo test -p hyprline-bar workspace_switch -- --nocapture` passes.
  - `cargo test -p hyprline-bar workspace_ -- --nocapture` passes and runs both existing `workspace_bind_labels_*` tests and new `workspace_switch_*` tests.
  - Addition-only forbidden-runtime-path grep finds no `hyprctl`, `Command::new`, `std::process`, `read_to_string`, `zbus`, or `dbus` additions in `hyprland_ipc.rs`.
  QA scenarios (name the exact tool + invocation):
  - Happy: `set -o pipefail; mkdir -p .omo/evidence && (cargo test -p hyprline-bar workspace_switch -- --nocapture && cargo test -p hyprline-bar workspace_ -- --nocapture && if git diff -U0 -- hyprline-bar/src/infrastructure/hyprland_ipc.rs | grep -E '^\+[^+].*(hyprctl|Command::new|std::process|read_to_string|zbus|dbus)'; then echo 'forbidden runtime path found'; exit 1; else echo 'no forbidden runtime path added'; fi) 2>&1 | tee .omo/evidence/task-2-workspace-click-lua-dispatch-fallback.log` passes.
  - Failure: Temporarily remove the Lua fallback retry branch, rerun `cargo test -p hyprline-bar workspace_switch -- --nocapture`, confirm the fallback-sequence test fails, then restore the branch and rerun green.
  Commit: N | included with implementation commit only if user later requests commit

- [x] 3. Repository gates and live direct-socket QA: Verify fallback behavior against current Hyprland Lua config - expect tests/build pass and live Lua command returns `ok`
  What to do / Must NOT do: Run full local gates and capture final target diff. Use direct Hyprland Unix socket QA only; do not use `hyprctl`. If a Hyprland socket is available, capture both the legacy error response and Lua fallback `ok` response for the current active workspace id. Do not run `make reinstall` unless a later executor needs manual UI deployment and the user explicitly asks.
  Parallelization: Wave 2 | Blocked by: 2 | Blocks: final verification
  References (executor has NO interview context - be exhaustive):
  - Planning live probe: `dispatch workspace 11` failed with Lua-dispatch syntax error; `dispatch hl.dsp.focus({ workspace = 11 })` returned `ok`.
  - `hyprline-bar/src/infrastructure/hyprland_ipc.rs`: final implementation target.
  Acceptance criteria (agent-executable):
  - `cargo test -p hyprline-bar workspace_switch -- --nocapture` passes.
  - `cargo test -p hyprline-bar workspace_ -- --nocapture` passes and runs both existing `workspace_bind_labels_*` tests and new `workspace_switch_*` tests.
  - `cargo test -p hyprline-bar` passes.
  - `cargo build -p hyprline-bar` passes.
  - `git diff -- hyprline-bar/src/infrastructure/hyprland_ipc.rs > .omo/evidence/final-target-diff-workspace-click-lua-dispatch-fallback.diff` runs.
  - `git diff -- . ':(exclude)hyprline-bar/src/infrastructure/hyprland_ipc.rs' ':(exclude).omo/evidence/**' > .omo/evidence/final-unrelated-tracked-workspace-click-lua-dispatch-fallback.diff` equals baseline unrelated tracked diff.
  - Forbidden-runtime-path grep over addition-only diff finds no runtime subprocess/D-Bus additions.
  - If Hyprland socket is available, live direct socket QA records `legacy_response` containing a Lua-dispatch error and `lua_response='ok'` for the current active workspace id. If unavailable, record limitation and rely on fake socket tests.
  QA scenarios (name the exact tool + invocation):
  - Happy: `set -o pipefail; mkdir -p .omo/evidence && (cargo test -p hyprline-bar workspace_switch -- --nocapture && cargo test -p hyprline-bar workspace_ -- --nocapture && cargo test -p hyprline-bar && cargo build -p hyprline-bar && git diff -- hyprline-bar/src/infrastructure/hyprland_ipc.rs > .omo/evidence/final-target-diff-workspace-click-lua-dispatch-fallback.diff && git diff -- . ':(exclude)hyprline-bar/src/infrastructure/hyprland_ipc.rs' ':(exclude).omo/evidence/**' > .omo/evidence/final-unrelated-tracked-workspace-click-lua-dispatch-fallback.diff && diff -u .omo/evidence/baseline-unrelated-tracked-workspace-click-lua-dispatch-fallback.diff .omo/evidence/final-unrelated-tracked-workspace-click-lua-dispatch-fallback.diff && if git diff -U0 -- hyprline-bar/src/infrastructure/hyprland_ipc.rs | grep -E '^\+[^+].*(hyprctl|Command::new|std::process|read_to_string|zbus|dbus)'; then echo 'forbidden runtime path found'; exit 1; else echo 'no forbidden runtime path added'; fi) 2>&1 | tee .omo/evidence/task-3-workspace-click-lua-dispatch-fallback.log` passes.
  - Live direct-socket QA: append a Python one-off script to `.omo/evidence/task-3-workspace-click-lua-dispatch-fallback.log` that finds the control socket, reads `j/activeworkspace`, sends `dispatch workspace <active_id>` and records its response, then sends `dispatch hl.dsp.focus({ workspace = <active_id> })` and records `ok`; it must not be product code.
  - Failure: If any gate fails, stop and fix before final verification; evidence log must include failing command and later passing rerun.
  Commit: N | do not commit unless the user explicitly asks after implementation

## Final verification wave
> Runs in parallel after ALL todos. ALL must APPROVE. Surface results and wait for the user's explicit okay before declaring complete.
- [x] F1. Plan compliance audit
  - Read this plan, the final target diff, and evidence logs.
  - Verify every Must Have is satisfied and every Must NOT Have is absent.
  - Evidence: append verdict to `.omo/evidence/final-workspace-click-lua-dispatch-fallback.log`.
- [x] F2. Code quality review
  - Review Rust IPC code for no panics, no subprocesses, no D-Bus path, no retry loops, precise fallback trigger, and no oversized complexity.
  - Required commands: `cargo test -p hyprline-bar` and `cargo build -p hyprline-bar`.
  - Evidence: append verdict to `.omo/evidence/final-workspace-click-lua-dispatch-fallback.log`.
- [x] F3. Real manual QA
  - Validate user-facing behavior through the closest available real surface: direct Hyprland socket command evidence plus fake-socket click-dispatch proof. If GUI automation tooling is available in the executor environment, additionally click a workspace label in the running bar and confirm active workspace changes; if unavailable, state that limitation.
  - Evidence: append verdict/environment note to `.omo/evidence/final-workspace-click-lua-dispatch-fallback.log`.
- [x] F4. Scope fidelity
  - Verify only intended product file changed for this plan and no unrelated dirty files were staged/modified.
  - Required commands: `git status --short`, `git diff --stat`, scoped diff inspection for `hyprline-bar/src/infrastructure/hyprland_ipc.rs`, and `diff -u .omo/evidence/baseline-unrelated-tracked-workspace-click-lua-dispatch-fallback.diff .omo/evidence/final-unrelated-tracked-workspace-click-lua-dispatch-fallback.diff`.
  - Evidence: append verdict to `.omo/evidence/final-workspace-click-lua-dispatch-fallback.log`.

## Commit strategy
- Do not commit by default; the user has not requested a commit.
- If the user explicitly asks for a commit after implementation, use one atomic commit:
  - `fix(workspaces): support lua workspace dispatch`
- Stage only intended product files:
  - `hyprline-bar/src/infrastructure/hyprland_ipc.rs`
- Do not stage `.omo/` unless the user explicitly asks to commit planning artifacts.
- Before committing, inspect `git status --short`, `git diff`, and `git log --oneline -10`.

## Success criteria
- Clicking a workspace label switches to that workspace again under the user's current Hyprland Lua config.
- Legacy/non-Lua Hyprland dispatch remains supported (`dispatch workspace <id>` success path sends no fallback).
- Lua-dispatch fallback uses direct Hyprland socket IPC only; no runtime `hyprctl`, shell command, subprocess, or D-Bus path is added.
- Socket command failures are no longer totally silent; failures are logged.
- Fake Unix socket tests prove the command sequence without depending on live Hyprland.
- Live direct-socket QA confirms the current Hyprland accepts `dispatch hl.dsp.focus({ workspace = <id> })`.
- `cargo test -p hyprline-bar` and `cargo build -p hyprline-bar` pass.
