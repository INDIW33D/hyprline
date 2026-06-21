# hyprland-lua-bind-description-labels - Work Plan

## TL;DR (For humans)

**What you'll get:** Hyprline will read the workspace metadata you added to Hyprland Lua bind descriptions, so workspace buttons can show `Q W E ...` again. The semicolon workspace will show `;`, and letters will be displayed uppercase.

**Why this approach:** Hyprland exposes Lua binds as opaque `__lua` callbacks, but it does expose bind descriptions through the same socket API Hyprline already uses. Parsing a strict `hyprline:workspace:<id>:<label>` description avoids Lua parsing, D-Bus, subprocesses, and QWERTY hardcoding in Rust.

**What it will NOT do:** It will not parse arbitrary Lua, will not call `hyprctl`, will not use D-Bus for bind discovery, and will not remove the numeric fallback.

**Effort:** Short
**Risk:** Medium - parser behavior must preserve existing legacy binds while adding a new explicit metadata path.
**Decisions to sanity-check:** Valid descriptions take precedence over legacy rows for the same workspace; malformed descriptions are ignored; `semicolon` maps to `;`; all other labels are uppercased.

Your next move: start work with the worker, or ask for a high-accuracy plan review first. Full execution detail follows below.

---

> TL;DR (machine): Short/medium-risk Rust parser + README update: parse `hyprline:workspace:<id>:<label>` bind descriptions from `j/binds`, normalize labels (`q -> Q`, `semicolon -> ;`), preserve legacy `workspace` dispatcher parsing and numeric fallback.

## Scope
### Must have
- Extend Hyprline's existing direct Hyprland socket `j/binds` parser to understand described Lua bind rows with the strict format `hyprline:workspace:<positive-int>:<label>`.
- Preserve the existing parser for normal registered binds where `dispatcher == "workspace"` and `arg` is a positive integer workspace id.
- Ignore described rows when they are mouse binds or non-default-submap binds, matching existing workspace-label filtering.
- Normalize workspace labels in Rust:
  - `q`, `w`, etc. become uppercase `Q`, `W`, etc.
  - `semicolon` becomes `;`.
  - numeric labels such as `1` remain `1`.
- Prefer valid explicit `hyprline:workspace:<id>:<label>` descriptions over legacy-derived labels for the same workspace id; if two valid descriptions target the same workspace, keep the first one deterministically.
- Ignore malformed descriptions without failing the whole parse.
- Keep `get_workspace_key_labels()` returning an empty map on socket or JSON parse failure so the UI numeric fallback still works.
- Add pure Rust unit tests for described Lua binds, semicolon mapping, uppercase normalization, malformed-description ignores, duplicate precedence, and legacy-parser regression.
- Update README English and Russian workspace keybinding sections to document Lua description metadata and the semicolon mapping behavior.

### Must NOT have (guardrails, anti-slop, scope boundaries)
- Must not parse arbitrary Lua source files.
- Must not spawn `hyprctl`, shell commands, `jq`, or any subprocess from Hyprline runtime for bind discovery.
- Must not add a D-Bus/zbus bind-discovery path.
- Must not hardcode the user's QWERTY workspace layout as a Rust fallback; Rust may only normalize labels it receives from Hyprland metadata.
- Must not remove the existing normal `dispatcher="workspace"` parser path.
- Must not remove numeric workspace fallback in the UI.
- Must not treat `dispatcher="__lua"` numeric `arg` as workspace metadata.
- Must not edit `/home/indiw33d/.config/hypr/hyprland.lua`; the user already made the required local config change.
- Must not refactor unrelated tray, notification, Bluetooth, brightness, submap UI, bar layout, or PipeWire code.
- Must not stage, commit, revert, or overwrite unrelated dirty-worktree changes.

## Verification strategy
> Zero human intervention - all verification is agent-executed.
- Test decision: TDD with Rust unit tests in `hyprline-bar`; docs/static/runtime evidence after tests are green.
- Evidence files:
  - `.omo/evidence/baseline-status-hyprland-lua-bind-description-labels.txt`
  - `.omo/evidence/baseline-hyprland-lua-bind-description-labels.diff`
  - `.omo/evidence/baseline-unrelated-tracked-hyprland-lua-bind-description-labels.diff`
  - `.omo/evidence/final-unrelated-tracked-hyprland-lua-bind-description-labels.diff`
  - `.omo/evidence/baseline-untracked-hyprland-lua-bind-description-labels.sha256`
  - `.omo/evidence/final-untracked-hyprland-lua-bind-description-labels.sha256`
  - `.omo/evidence/baseline-hyprland-lua-config.sha256`
  - `.omo/evidence/final-hyprland-lua-config.sha256`
  - `.omo/evidence/task-1-hyprland-lua-bind-description-labels.log`
  - `.omo/evidence/task-2-hyprland-lua-bind-description-labels.log`
  - `.omo/evidence/task-3-hyprland-lua-bind-description-labels.log`
  - `.omo/evidence/task-4-hyprland-lua-bind-description-labels.log`
  - `.omo/evidence/final-target-diff-hyprland-lua-bind-description-labels.diff`
  - `.omo/evidence/final-hyprland-lua-bind-description-labels.log`
- Required final commands:
  - Before product edits, run the baseline command block below to capture dirty-worktree state, unrelated tracked diffs, untracked file hashes, and prove the external Hyprland config is not edited by repo implementation:
    ```bash
    set -e
    mkdir -p .omo/evidence && \
    git status --short > .omo/evidence/baseline-status-hyprland-lua-bind-description-labels.txt && \
    git diff -- hyprline-bar/src/infrastructure/hyprland_ipc.rs README.md > .omo/evidence/baseline-hyprland-lua-bind-description-labels.diff && \
    git diff -- . ':(exclude)hyprline-bar/src/infrastructure/hyprland_ipc.rs' ':(exclude)README.md' ':(exclude).omo/evidence/**' > .omo/evidence/baseline-unrelated-tracked-hyprland-lua-bind-description-labels.diff && \
    python - <<'PY' > .omo/evidence/baseline-untracked-hyprland-lua-bind-description-labels.sha256
from pathlib import Path
import hashlib
import subprocess
excluded_prefixes = ('.omo/evidence/',)
paths = subprocess.check_output(['git', 'ls-files', '--others', '--exclude-standard'], text=True).splitlines()
for item in sorted(paths):
    if any(item.startswith(prefix) for prefix in excluded_prefixes):
        continue
    path = Path(item)
    if path.is_file():
        print(f'{hashlib.sha256(path.read_bytes()).hexdigest()}  {item}')
    else:
        print(f'<nonfile>  {item}')
PY
    sha256sum /home/indiw33d/.config/hypr/hyprland.lua > .omo/evidence/baseline-hyprland-lua-config.sha256
    ```
  - `cargo test -p hyprline-bar workspace_bind_labels -- --nocapture`
  - `cargo test -p hyprline-bar`
  - `cargo build -p hyprline-bar`
  - `if git diff -U0 -- hyprline-bar/src/infrastructure/hyprland_ipc.rs | grep -E '^\+[^+].*(hyprctl|Command::new|std::process|zbus|dbus|read_to_string|hyprland\.lua|\.lua)'; then echo 'forbidden runtime path found'; exit 1; else echo 'no forbidden runtime path added'; fi`
  - After verification, run `sha256sum /home/indiw33d/.config/hypr/hyprland.lua > .omo/evidence/final-hyprland-lua-config.sha256 && diff -u .omo/evidence/baseline-hyprland-lua-config.sha256 .omo/evidence/final-hyprland-lua-config.sha256`.

## Execution strategy
### Parallel execution waves
> Target 5-8 todos per wave. Fewer than 3 (except the final) means you under-split.
- Wave 1 is sequential: test contract first, parser implementation second.
- Wave 2 can update docs and run live/debug validation after parser behavior is green.
- Final verification runs after all todos and must approve plan compliance, code quality, real QA evidence, and scope fidelity.

### Dependency matrix
| Todo | Depends on | Blocks | Can parallelize with |
| --- | --- | --- | --- |
| 1 | none | 2, 4 | none |
| 2 | 1 | 3, 4 | none |
| 3 | 2 | 4 | none |
| 4 | 2, 3 | final verification | none |

## Todos
> Implementation + Test = ONE todo by default. Todo 1 is an explicit TDD red-test exception; Todo 2 implements the production behavior that makes Todo 1 green.
<!-- APPEND TASK BATCHES BELOW THIS LINE WITH edit/apply_patch - never rewrite the headers above. -->
- [x] 1. `hyprline-bar/src/infrastructure/hyprland_ipc.rs`: Add red parser tests for described Lua workspace binds - expect explicit metadata labels are parsed and normalized
  What to do / Must NOT do: Capture the new contract in pure unit tests before changing production parser behavior. Add tests under the existing `workspace_bind_labels_*` test family so the existing targeted test command discovers them. Do not touch sockets, GTK, UI, README, or local Hyprland config in this todo.
  Parallelization: Wave 1 | Blocked by: none | Blocks: 2, 4
  References (executor has NO interview context - be exhaustive):
  - `hyprline-bar/src/infrastructure/hyprland_ipc.rs:96-147` current parser only accepts `dispatcher == "workspace"` plus positive numeric `arg`.
  - `hyprline-bar/src/infrastructure/hyprland_ipc.rs:231-263` current workspace bind parser tests.
  - `/home/indiw33d/.config/hypr/hyprland.lua:283-286` user-added metadata format: `description = "hyprline:workspace:" .. i .. ":" .. key`.
  - Draft findings: live `j/binds` now exposes 20 rows with `description` values from `hyprline:workspace:1:q` through `hyprline:workspace:20:semicolon`.
  - Metis `ses_116c614d3ffegFGXKdD0j0gd2z`: tests must cover described Lua binds, semicolon mapping, uppercase letters, malformed descriptions, duplicates, and legacy regression.
  Acceptance criteria (agent-executable):
  - New test proves a JSON row like `{"dispatcher":"__lua","arg":"29","key":"q","has_description":true,"description":"hyprline:workspace:1:q","submap":"","mouse":false}` maps to `{1: "Q"}`.
  - New test proves `description:"hyprline:workspace:20:semicolon"` maps to `{20: ";"}`.
  - New test proves uppercase/already-uppercase and numeric labels normalize predictably: `q -> Q`, `Q -> Q`, `1 -> 1`.
  - New test proves malformed descriptions are ignored: missing prefix, missing id/label, non-numeric id, zero id, negative id, empty label.
  - New test proves `dispatcher="__lua"` with numeric `arg` but no valid `hyprline:workspace:` description is ignored, so opaque Lua registry ids are never treated as workspace ids.
  - New test proves extra-colon and whitespace-sensitive malformed descriptions such as `hyprline:workspace:1:q:extra`, ` hyprline:workspace:1:q`, and `hyprline:workspace: 1:q` are rejected, not guessed.
  - New test proves mouse binds and non-empty submap rows with otherwise-valid descriptions are ignored.
  - New test proves valid description metadata takes precedence over legacy-derived labels for the same workspace even when the legacy label is non-numeric and regardless of whether the described row appears before or after the legacy row.
  - New test proves two valid descriptions for one workspace keep the first described label deterministically.
  - Existing legacy tests still exist unchanged or with only naming/fixture refactors.
  QA scenarios (name the exact tool + invocation):
  - Happy: `set -o pipefail; mkdir -p .omo/evidence && (cargo test -p hyprline-bar workspace_bind_labels -- --nocapture) 2>&1 | tee .omo/evidence/task-1-hyprland-lua-bind-description-labels.log` shows the new tests are discovered. If tests pass before production edits due prior edits, record that and continue; otherwise red failure is expected.
  - Failure: Temporarily expect `semicolon` to become `SEMICOLON` in the new semicolon test, run the same command, confirm the test fails, then restore expected `;` before continuing.
  Commit: N | included with implementation commit only if user later requests commit

- [x] 2. `hyprline-bar/src/infrastructure/hyprland_ipc.rs`: Implement description-aware workspace label parsing - expect Lua metadata rows map to uppercase labels and `;`
  What to do / Must NOT do: Extend `HyprlandBind` with defaulted description fields such as `description: Option<String>` and optionally `has_description: bool`. Refactor row extraction into a helper like `workspace_label(&self) -> Option<(i32, String, LabelSource)>`, or equivalent, that first tries strict `hyprline:workspace:<positive-int>:<label>` description parsing, then falls back to the existing normal-dispatcher parsing. Add a shared label normalizer: `semicolon -> ;`, otherwise uppercase the received label. Do not parse Lua, do not use `__lua` numeric `arg`, do not add subprocess/D-Bus code.
  Parallelization: Wave 1 | Blocked by: 1 | Blocks: 3, 4
  References (executor has NO interview context - be exhaustive):
  - `hyprline-bar/src/infrastructure/hyprland_ipc.rs:16-20` socket-backed `get_workspace_key_labels()` already fails safe to empty map.
  - `hyprline-bar/src/infrastructure/hyprland_ipc.rs:96-119` parser entry point and duplicate-handling logic.
  - `hyprline-bar/src/infrastructure/hyprland_ipc.rs:121-147` current numeric-label and workspace-id helpers.
  - Hyprland source via draft: `j/binds` rows expose `description` and `has_description`; Lua rows remain `dispatcher="__lua"` with opaque `arg`.
  Acceptance criteria (agent-executable):
  - `cargo test -p hyprline-bar workspace_bind_labels -- --nocapture` passes and runs all described-bind tests from Todo 1.
  - Legacy `dispatcher="workspace"` rows still parse exactly as before except that any `semicolon` label is normalized to `;` if present.
  - Valid description metadata is accepted regardless of whether `dispatcher` is `__lua`, because the strict prefix is the source of truth.
  - Valid description metadata overrides legacy rows independently of row order; do not let an earlier legacy `Entry::Vacant` insert block a later explicit description.
  - Invalid JSON still returns `Err` from the pure parser and `get_workspace_key_labels()` still returns an empty map on request/parse failure.
  - `git diff -U0 -- hyprline-bar/src/infrastructure/hyprland_ipc.rs | grep -E '^\+[^+].*(hyprctl|Command::new|std::process|zbus|dbus|read_to_string|hyprland\.lua|\.lua)'` has no output.
  QA scenarios (name the exact tool + invocation):
  - Happy: `set -o pipefail; mkdir -p .omo/evidence && (cargo test -p hyprline-bar workspace_bind_labels -- --nocapture && if git diff -U0 -- hyprline-bar/src/infrastructure/hyprland_ipc.rs | grep -E '^\+[^+].*(hyprctl|Command::new|std::process|zbus|dbus|read_to_string|hyprland\.lua|\.lua)'; then echo 'forbidden runtime path found'; exit 1; else echo 'no forbidden runtime path added'; fi) 2>&1 | tee .omo/evidence/task-2-hyprland-lua-bind-description-labels.log` passes.
  - Failure: Temporarily remove the `semicolon -> ;` normalization branch, rerun the targeted test command, confirm the semicolon test fails, then restore the branch before continuing.
  Commit: N | included with implementation commit only if user later requests commit

- [x] 3. `README.md`: Document Lua bind description metadata - expect users know why descriptions are needed and what format to use
  What to do / Must NOT do: Update both English and Russian workspace keybinding sections. Explain that Hyprline first reads registered `workspace` dispatcher binds, and for Hyprland Lua callback binds it can also read strict descriptions like `hyprline:workspace:1:q`. Include a compact Lua loop example matching the user's config. Mention that letters are shown uppercase and `semicolon` is displayed as `;`. Do not claim Hyprline parses Lua files, calls `hyprctl`, or uses D-Bus.
  Parallelization: Wave 2 | Blocked by: 2 | Blocks: 4
  References (executor has NO interview context - be exhaustive):
  - `README.md` current English/Russian workspace keybinding sections from the previous plan describe registered socket binds but not Lua descriptions.
  - `/home/indiw33d/.config/hypr/hyprland.lua:277-288` working local Lua authoring pattern.
  - Draft findings: Hyprland Lua rows expose description metadata through `j/binds`, but effective workspace dispatcher/arg is not exposed.
  Acceptance criteria (agent-executable):
  - README English workspace-keybinding section contains `hyprline:workspace:<id>:<label>` or a concrete `hyprline:workspace:1:q` example.
  - README English workspace-keybinding section documents `semicolon -> ;` display and numeric fallback.
  - README Russian workspace-keybinding section contains the same metadata concept and notes `semicolon -> ;` display and numeric fallback.
  - README does not claim Hyprline parses Lua source or calls `hyprctl` for runtime label discovery.
  QA scenarios (name the exact tool + invocation):
  - Happy: `set -o pipefail; mkdir -p .omo/evidence && (python - <<'PY'
from pathlib import Path
text = Path('README.md').read_text()
english = text.split('### Workspace Keybindings', 1)[1].split('### Dependencies', 1)[0]
russian = text.split('### Горячие клавиши воркспейсов', 1)[1].split('### Зависимости', 1)[0]
checks = [
    ('english metadata', 'hyprline:workspace' in english),
    ('english semicolon', 'semicolon' in english and ';' in english),
    ('english fallback', 'fallback' in english.lower() and 'number' in english.lower()),
    ('russian metadata', 'hyprline:workspace' in russian),
    ('russian semicolon', 'semicolon' in russian and ';' in russian),
    ('russian fallback', 'Откат' in russian or 'номер' in russian),
]
failed = [name for name, ok in checks if not ok]
if failed:
    raise SystemExit('README checks failed: ' + ', '.join(failed))
print('README EN/RU workspace metadata checks passed')
PY
if grep -nE 'parse Lua|парсит Lua|calls `hyprctl`|вызывает `hyprctl`' README.md; then echo 'stale runtime-discovery claim found'; exit 1; else echo 'no stale Lua/hyprctl runtime-discovery claim'; fi) 2>&1 | tee .omo/evidence/task-3-hyprland-lua-bind-description-labels.log` passes.
  - Failure: Temporarily remove the Russian `hyprline:workspace` example, run the same Python README check, confirm it fails, then restore the Russian docs before continuing.
  Commit: N | included with implementation commit only if user later requests commit

- [x] 4. Repository gates and live metadata QA: Run final verification - expect tests/build pass and live described binds are parseable
  What to do / Must NOT do: Run full local gates and capture a scoped final diff. If a Hyprland socket is available, query `j/binds` directly over the socket with a one-off QA script and confirm 20 `hyprline:workspace:` descriptions are present; this QA script must not become product code. Do not stage/commit or touch local Hyprland config.
  Parallelization: Wave 2 | Blocked by: 2, 3 | Blocks: final verification
  References (executor has NO interview context - be exhaustive):
  - `.omo/start-work/ledger.jsonl:9` previous live QA saw 70 binds and 0 normal workspace dispatcher rows before description parsing.
  - User verification in this session: live direct socket query now sees `total_binds=70` and `hyprline_descriptions=20`.
  - `Makefile` has `reinstall`; use only if the worker needs to deploy before manual UI verification and the user has explicitly started execution.
  Acceptance criteria (agent-executable):
  - `cargo test -p hyprline-bar workspace_bind_labels -- --nocapture` passes.
  - `cargo test -p hyprline-bar` passes.
  - `cargo build -p hyprline-bar` passes.
  - `git diff -- hyprline-bar/src/infrastructure/hyprland_ipc.rs README.md > .omo/evidence/final-target-diff-hyprland-lua-bind-description-labels.diff` runs.
  - Scoped addition-only forbidden-runtime-path grep over `hyprline-bar/src/infrastructure/hyprland_ipc.rs` finds no `hyprctl`, `Command::new`, `std::process`, `zbus`, `dbus`, `read_to_string`, `hyprland.lua`, or `.lua` additions.
  - `sha256sum` before/after evidence proves `/home/indiw33d/.config/hypr/hyprland.lua` was not edited by implementation.
  - Final unrelated tracked diff equals baseline unrelated tracked diff, and final untracked manifest equals baseline untracked manifest, excluding `.omo/evidence/**`.
  - If Hyprland socket is available, direct socket `j/binds` QA confirms at least the 20 expected `hyprline:workspace:` descriptions; if unavailable, record environment limitation and rely on parser fixtures.
  QA scenarios (name the exact tool + invocation):
  - Happy: `set -o pipefail; mkdir -p .omo/evidence && (cargo test -p hyprline-bar workspace_bind_labels -- --nocapture && cargo test -p hyprline-bar && cargo build -p hyprline-bar && git diff -- hyprline-bar/src/infrastructure/hyprland_ipc.rs README.md > .omo/evidence/final-target-diff-hyprland-lua-bind-description-labels.diff && git diff -- . ':(exclude)hyprline-bar/src/infrastructure/hyprland_ipc.rs' ':(exclude)README.md' ':(exclude).omo/evidence/**' > .omo/evidence/final-unrelated-tracked-hyprland-lua-bind-description-labels.diff && diff -u .omo/evidence/baseline-unrelated-tracked-hyprland-lua-bind-description-labels.diff .omo/evidence/final-unrelated-tracked-hyprland-lua-bind-description-labels.diff && python - <<'PY' > .omo/evidence/final-untracked-hyprland-lua-bind-description-labels.sha256
from pathlib import Path
import hashlib
import subprocess
excluded_prefixes = ('.omo/evidence/',)
paths = subprocess.check_output(['git', 'ls-files', '--others', '--exclude-standard'], text=True).splitlines()
for item in sorted(paths):
    if any(item.startswith(prefix) for prefix in excluded_prefixes):
        continue
    path = Path(item)
    if path.is_file():
        print(f'{hashlib.sha256(path.read_bytes()).hexdigest()}  {item}')
    else:
        print(f'<nonfile>  {item}')
PY
diff -u .omo/evidence/baseline-untracked-hyprland-lua-bind-description-labels.sha256 .omo/evidence/final-untracked-hyprland-lua-bind-description-labels.sha256 && if git diff -U0 -- hyprline-bar/src/infrastructure/hyprland_ipc.rs | grep -E '^\+[^+].*(hyprctl|Command::new|std::process|zbus|dbus|read_to_string|hyprland\.lua|\.lua)'; then echo 'forbidden runtime path found'; exit 1; else echo 'no forbidden runtime path added'; fi && sha256sum /home/indiw33d/.config/hypr/hyprland.lua > .omo/evidence/final-hyprland-lua-config.sha256 && diff -u .omo/evidence/baseline-hyprland-lua-config.sha256 .omo/evidence/final-hyprland-lua-config.sha256) 2>&1 | tee .omo/evidence/task-4-hyprland-lua-bind-description-labels.log` passes.
  - Live metadata QA: append this command to `.omo/evidence/task-4-hyprland-lua-bind-description-labels.log`; it must print `hyprline_descriptions=20` when a Hyprland socket is available, otherwise record the limitation and continue with fixture proof:
    `python - <<'PY'
import json, os, pathlib, socket
sig = os.environ.get('HYPRLAND_INSTANCE_SIGNATURE')
runtime = os.environ.get('XDG_RUNTIME_DIR')
candidates = []
if sig and runtime:
    candidates.append(pathlib.Path(runtime) / 'hypr' / sig / '.socket.sock')
if sig:
    candidates.append(pathlib.Path('/tmp/hypr') / sig / '.socket.sock')
if runtime:
    root = pathlib.Path(runtime) / 'hypr'
    if root.exists():
        candidates.extend(p / '.socket.sock' for p in root.iterdir() if p.is_dir())
root = pathlib.Path('/tmp/hypr')
if root.exists():
    candidates.extend(p / '.socket.sock' for p in root.iterdir() if p.is_dir())
for path in dict.fromkeys(candidates):
    if not path.exists():
        continue
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as s:
        s.settimeout(2)
        s.connect(str(path))
        s.sendall(b'[[BATCH]]j/binds')
        chunks = []
        while True:
            data = s.recv(65536)
            if not data:
                break
            chunks.append(data)
    rows = json.loads(b''.join(chunks).decode())
    count = sum(1 for row in rows if row.get('description', '').startswith('hyprline:workspace:'))
    print(f'socket={path}')
    print(f'hyprline_descriptions={count}')
    raise SystemExit(0 if count >= 20 else 1)
print('no Hyprland socket available; relying on parser fixture proof')
PY`
  - Failure: If any gate fails, stop and fix before final verification; evidence log must include the failing command and the later passing rerun.
  Commit: N | do not commit unless the user explicitly asks after implementation

## Final verification wave
> Runs in parallel after ALL todos. ALL must APPROVE. Surface results and wait for the user's explicit okay before declaring complete.
- [x] F1. Plan compliance audit
  - Read this plan, the final target diff, and evidence logs.
  - Verify every Must Have is satisfied and every Must NOT Have is absent.
  - Required command: `diff -u .omo/evidence/baseline-hyprland-lua-bind-description-labels.diff .omo/evidence/final-target-diff-hyprland-lua-bind-description-labels.diff || true`.
  - Evidence: append verdict to `.omo/evidence/final-hyprland-lua-bind-description-labels.log`.
- [x] F2. Code quality review
  - Review Rust parser for no panics, no subprocesses, no D-Bus path, deterministic precedence, narrow helpers, and no oversized complexity.
  - Required commands: `cargo test -p hyprline-bar` and `cargo build -p hyprline-bar`.
  - Evidence: append verdict to `.omo/evidence/final-hyprland-lua-bind-description-labels.log`.
- [x] F3. Live metadata QA + parser fixture proof
  - If running under Hyprland socket environment, query live `j/binds` directly and confirm the `hyprline:workspace:` metadata is visible. If UI deployment is explicitly performed during execution, additionally confirm Hyprline labels render as `Q W E ... ;` rather than numbers. If UI deployment is not performed, do not call this manual UI QA; document that live metadata is present and parser fixtures prove interpretation.
  - Required non-interactive fallback command: `cargo test -p hyprline-bar workspace_bind_labels -- --nocapture`.
  - Evidence: append verdict and environment note to `.omo/evidence/final-hyprland-lua-bind-description-labels.log`.
- [x] F4. Scope fidelity
  - Verify only intended target files changed for this plan and no unrelated dirty files were staged/modified by the worker.
  - Required commands: `git status --short`, `git diff --stat`, scoped diff inspection for `hyprline-bar/src/infrastructure/hyprland_ipc.rs README.md`, `diff -u .omo/evidence/baseline-unrelated-tracked-hyprland-lua-bind-description-labels.diff .omo/evidence/final-unrelated-tracked-hyprland-lua-bind-description-labels.diff`, `diff -u .omo/evidence/baseline-untracked-hyprland-lua-bind-description-labels.sha256 .omo/evidence/final-untracked-hyprland-lua-bind-description-labels.sha256`, and `diff -u .omo/evidence/baseline-hyprland-lua-config.sha256 .omo/evidence/final-hyprland-lua-config.sha256`.
  - Evidence: append verdict to `.omo/evidence/final-hyprland-lua-bind-description-labels.log`.

## Commit strategy
- Do not commit by default; the user has not requested a commit.
- If the user explicitly asks for a commit after implementation, use one atomic commit:
  - `fix(workspaces): parse hyprland lua bind labels`
- Stage only intended product files:
  - `hyprline-bar/src/infrastructure/hyprland_ipc.rs`
  - `README.md`
- Do not stage `.omo/` unless the user explicitly asks to commit planning artifacts.
- Before committing, inspect `git status --short`, `git diff`, and `git log --oneline -10`.

## Success criteria
- Hyprline can derive workspace labels from described Hyprland Lua bind rows such as `hyprline:workspace:1:q`.
- Workspace labels from described Lua rows display uppercase letters (`q -> Q`) and semicolon as `;`.
- Existing normal `dispatcher="workspace"` bind parsing still works.
- Malformed descriptions are ignored and numeric fallback remains available.
- Hyprline runtime still uses direct Hyprland socket IPC only; no `hyprctl` subprocess and no D-Bus bind discovery are added.
- README documents the Lua description format in English and Russian.
- `cargo test -p hyprline-bar` and `cargo build -p hyprline-bar` pass.
