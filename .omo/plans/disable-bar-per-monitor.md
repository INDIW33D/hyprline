# disable-bar-per-monitor - Work Plan

## TL;DR (For humans)

**What you'll get:** You will be able to turn the Hyprline bar on or off for each monitor from Settings, and the choice will be saved. Existing configs keep showing bars on all monitors until you explicitly disable one.

**Why this approach:** The app already stores per-monitor profile settings, so the new visibility flag belongs in the same saved monitor config. Runtime will reconcile bar visibility when settings change so toggles apply immediately instead of requiring a restart. Existing live bars are hidden/reused on config toggles rather than destroyed/recreated, because `Bar::setup_event_listener()` currently installs non-cancellable timers/subscriptions.

**What it will NOT do:** It will not change widget/profile behavior, edit Hyprland config files, or add subprocess/D-Bus monitor discovery paths.

**Effort:** Medium
**Risk:** Medium - runtime window lifecycle must hide/reuse/create bars without duplicating event listeners or changing existing defaults.
**Decisions to sanity-check:** disabled bars apply immediately; all monitors may be disabled; monitor identity is the existing Hyprland monitor name.

Your next move: start work, or request a high-accuracy plan review first. Full execution detail follows below.

---

> TL;DR (machine): Medium-risk Rust/GTK config + lifecycle feature: add persisted `MonitorConfig.bar_enabled`, expose a monitor settings switch, and reconcile Bar visibility on startup/monitor/config changes with unique named unit tests, structural settings checks, and build/data QA.

## Scope
### Must have
- Persist a per-monitor bar visibility flag in the existing JSON config under each `config.monitors[monitor_name]` entry.
- Default every missing/unknown monitor visibility flag to enabled (`true`) for backward compatibility.
- Add config helpers to read and write the flag without duplicating HashMap/default logic in UI/runtime code.
- Extend the existing Settings → Monitors page with one bar visibility switch per detected monitor.
- The settings switch must be initialized from config before signal connection, then save via the existing `save_config()` path when the user toggles it.
- Saving the switch must update config, write it to disk, and notify existing config-change subscribers through the current `save_config()` path.
- Startup must create bars only for monitors whose bar visibility is enabled; when Hyprland reports no monitors, the existing `default` fallback bar remains enabled unless `default` is explicitly disabled.
- Config hot reload must reconcile bar visibility: hide already-created bars for disabled monitors, create bars for enabled monitors that do not currently have one, re-present hidden bars when re-enabled, and only rebuild widgets for visible still-enabled bars.
- Runtime monitor-added handling must create/show a bar only when that monitor is enabled in config using a fresh full monitor snapshot after the GDK delay; monitor-removed handling must keep closing/removing bars as it does now.
- Existing per-monitor profile selection must continue to work for enabled bars.
- Add agent-executable tests for config backward compatibility, config helper behavior, pure bar reconciliation decisions, fake lifecycle application actions, idempotency, monitor-added preservation, and the disabled/enabled monitor cases.
- Keep `cargo test -p hyprline-bar` and `cargo build -p hyprline-bar` passing.

### Must NOT have (guardrails, anti-slop, scope boundaries)
- Must not redesign widget profiles, widget ordering, or the existing per-monitor profile selection model beyond adding the visibility flag.
- Must not edit `/home/indiw33d/.config/hypr/hyprland.lua` or any user Hyprland config.
- Must not add `hyprctl`, `Command::new`, `std::process`, shell scripts, D-Bus/zbus, or any new external monitor discovery path.
- Must not implement restart-only behavior. If immediate reconciliation appears unsafe or impossible, stop and request a plan revision instead of weakening the requirement.
- Must not destroy/recreate an already-created bar solely because a config toggle disables/re-enables it; hide/reuse it so non-cancellable event listeners and shared-state subscriptions are not duplicated. Closing remains required for actual monitor removal.
- Must not remove the no-monitors `default` fallback bar behavior except when the persisted `default` monitor entry explicitly disables it.
- Must not introduce a global "disable all bars" setting separate from the per-monitor setting.
- Must not stage, commit, push, or rewrite history unless the user asks after implementation.

## Verification strategy
> Zero human intervention - all verification is agent-executed.
- Test decision: TDD for pure Rust seams (`config/widget_config.rs`, `main.rs` reconciliation/lifecycle helpers, and the minimal `Bar::hide` wrapper if added); structural code checks plus build proof for GTK settings wiring where headless widget construction is not reliable.
- Required unique test function names; every filtered cargo command must use this shell wrapper so module-scoped tests match while zero-test success cannot pass:
  ```bash
  run_named() {
    name="$1"
    log="/tmp/${name}.log"
    cargo test -p hyprline-bar "$name" -- --nocapture 2>&1 | tee "$log"
    grep -q "running 1 test" "$log"
    grep -Eq "test .*::${name} \.\.\. ok|test ${name} \.\.\. ok" "$log"
  }
  ```
- Data-shaped QA is the required real-surface substitute for GUI automation: config JSON serialization/deserialization artifacts plus fake lifecycle action logs showing disable -> enable -> disable without duplicate setup. GUI opening/clicking is optional extra evidence only if automation/display tools are available.
- Evidence files:
  - `.omo/evidence/baseline-status-disable-bar-per-monitor.txt`
  - `.omo/evidence/baseline-diff-disable-bar-per-monitor.diff`
  - `.omo/evidence/task-1-disable-bar-per-monitor.log`
  - `.omo/evidence/task-2-disable-bar-per-monitor.log`
  - `.omo/evidence/task-3-disable-bar-per-monitor.log`
  - `.omo/evidence/task-4-disable-bar-per-monitor.log`
  - `.omo/evidence/task-5-disable-bar-per-monitor.log`
  - `.omo/evidence/task-6-disable-bar-per-monitor.log`
  - `.omo/evidence/final-disable-bar-per-monitor.log`
- Required final commands:
  - `run_named monitor_bar_defaults_missing_visibility_to_enabled`
  - `run_named monitor_bar_unknown_monitor_defaults_enabled`
  - `run_named monitor_bar_set_visibility_preserves_profile`
  - `run_named monitor_bar_disabled_serializes_false`
  - `run_named bar_reconciliation_defaults_all_monitors_enabled`
  - `run_named bar_reconciliation_excludes_disabled_monitor`
  - `run_named bar_reconciliation_hides_existing_disabled_bar`
  - `run_named bar_reconciliation_creates_newly_enabled_missing_bar`
  - `run_named bar_reconciliation_default_fallback_respects_config`
  - `run_named bar_reconciliation_monitor_added_preserves_existing_visible_bar`
  - `run_named bar_reconciliation_apply_records_hide_rebuild_create_actions`
  - `run_named bar_reconciliation_disable_enable_disable_no_duplicate_setup`
  - `cargo test -p hyprline-bar`
  - `cargo build -p hyprline-bar`
  - `if git diff -U0 -- hyprline-bar/src | grep -E '^\+[^+].*(hyprctl|Command::new|std::process|zbus|dbus)'; then echo 'forbidden runtime path found'; exit 1; else echo 'no forbidden runtime path added'; fi`

## Execution strategy
### Parallel execution waves
> Target 5-8 todos per wave. Fewer than 3 (except the final) means you under-split.
- Wave 0 captures dirty-worktree baseline before product edits.
- Wave 1 is TDD on pure config and runtime decision seams.
- Wave 2 wires runtime visibility lifecycle and settings UI after helpers exist.
- Wave 3 runs full gates and data-shaped/live QA.

### Dependency matrix
| Todo | Depends on | Blocks | Can parallelize with |
| --- | --- | --- | --- |
| 1 | none | 2, 3, 4, 5, 6 | none |
| 2 | 1 | 3, 4, 5, 6 | none |
| 3 | 2 | 4, 6 | none |
| 4 | 2, 3 | 6 | 5 |
| 5 | 2 | 6 | 4 |
| 6 | 4, 5 | final verification | none |

## Todos
> Implementation + Test = ONE todo. Never separate.
<!-- APPEND TASK BATCHES BELOW THIS LINE WITH edit/apply_patch - never rewrite the headers above. -->
- [x] 1. `.omo/evidence`: Capture dirty-worktree baseline before product edits - expect reproducible status and diff artifacts
  What to do / Must NOT do: Capture status/diff evidence before touching product files. Do not modify product files in this todo.
  Parallelization: Wave 0 | Blocked by: none | Blocks: 2, 3, 4, 5, 6
  References (executor has NO interview context - be exhaustive):
  - This repo was clean after the last commit/push, but the worker must still capture baseline because every start-work session can inherit new user changes.
  - Plan scope is product files `hyprline-bar/src/config/widget_config.rs`, `hyprline-bar/src/main.rs`, `hyprline-bar/src/ui/settings.rs`, and the minimal lifecycle helper in `hyprline-bar/src/ui/bar.rs` plus `.omo` artifacts.
  Acceptance criteria (agent-executable):
  - `.omo/evidence/baseline-status-disable-bar-per-monitor.txt` exists and contains `git status --short` output.
  - `.omo/evidence/baseline-diff-disable-bar-per-monitor.diff` exists and contains the scoped diff before product edits.
  - If the worktree is not clean, unrelated dirty paths are recorded in `.omo/evidence/baseline-status-disable-bar-per-monitor.txt` and must be preserved.
  QA scenarios (name the exact tool + invocation):
  - Happy: `set -o pipefail; mkdir -p .omo/evidence && (git status --short > .omo/evidence/baseline-status-disable-bar-per-monitor.txt && git diff -- hyprline-bar/src/config/widget_config.rs hyprline-bar/src/main.rs hyprline-bar/src/ui/settings.rs hyprline-bar/src/ui/bar.rs > .omo/evidence/baseline-diff-disable-bar-per-monitor.diff && wc -l .omo/evidence/baseline-status-disable-bar-per-monitor.txt .omo/evidence/baseline-diff-disable-bar-per-monitor.diff) 2>&1 | tee .omo/evidence/task-1-disable-bar-per-monitor.log` passes.
  - Failure: If baseline capture fails or files are missing, stop before product edits and record the command failure in the same log.
  Commit: N | baseline evidence only

- [x] 2. `hyprline-bar/src/config/widget_config.rs`: Add persisted monitor bar visibility config - expect backward-compatible JSON and helper methods
  What to do / Must NOT do: First add tests named exactly `monitor_bar_defaults_missing_visibility_to_enabled`, `monitor_bar_unknown_monitor_defaults_enabled`, `monitor_bar_set_visibility_preserves_profile`, and `monitor_bar_disabled_serializes_false`. Then add `bar_enabled: bool` to `MonitorConfig` with `#[serde(default = "default_bar_enabled")]`, `fn default_bar_enabled() -> bool { true }`, and helpers on `HyprlineConfig`, e.g. `is_bar_enabled_for_monitor(&self, monitor_name: &str) -> bool` and `set_monitor_bar_enabled(&mut self, monitor_name: &str, enabled: bool)`. Do not touch UI/runtime files in this todo.
  Parallelization: Wave 1 | Blocked by: 1 | Blocks: 3, 4, 5, 6
  References (executor has NO interview context - be exhaustive):
  - `hyprline-bar/src/config/widget_config.rs:205-217`: current `MonitorConfig` only stores `profile_name`.
  - `hyprline-bar/src/config/widget_config.rs:274-302`: `HyprlineConfig` stores `monitors: HashMap<String, MonitorConfig>` and uses serde defaults for other fields.
  - `hyprline-bar/src/config/widget_config.rs:387-504`: config JSON load/save path and migration behavior.
  - `hyprline-bar/src/config/widget_config.rs:523-552`: existing per-monitor profile helper pattern.
  Acceptance criteria (agent-executable):
  - Deserializing a config JSON that has monitor entries without `bar_enabled` succeeds and yields `true` for those monitors.
  - Unknown monitors return `true` from `is_bar_enabled_for_monitor`.
  - `set_monitor_bar_enabled("DP-1", false)` persists/serializes `bar_enabled: false` under the `DP-1` monitor entry without losing an existing `profile_name`.
  - Re-enabling a monitor returns `true` from the helper and preserves existing profile mapping.
  - `run_named monitor_bar_defaults_missing_visibility_to_enabled`, `run_named monitor_bar_unknown_monitor_defaults_enabled`, `run_named monitor_bar_set_visibility_preserves_profile`, and `run_named monitor_bar_disabled_serializes_false` all prove exactly one test ran and passed.
  QA scenarios (name the exact tool + invocation):
  - Happy: `set -o pipefail; mkdir -p .omo/evidence && (run_named() { name="$1"; log="/tmp/${name}.log"; cargo test -p hyprline-bar "$name" -- --nocapture 2>&1 | tee "$log"; grep -q "running 1 test" "$log"; grep -Eq "test .*::${name} \.\.\. ok|test ${name} \.\.\. ok" "$log"; }; run_named monitor_bar_defaults_missing_visibility_to_enabled && run_named monitor_bar_unknown_monitor_defaults_enabled && run_named monitor_bar_set_visibility_preserves_profile && run_named monitor_bar_disabled_serializes_false) 2>&1 | tee .omo/evidence/task-2-disable-bar-per-monitor.log` passes.
  - Failure: Before implementation, run at least one new named test and record the expected red compile/fail state; after implementation rerun all named tests green. Do not leave temporary broken source edits behind.
  Commit: N | included only if the user later requests commits

- [x] 3. `hyprline-bar/src/main.rs`: Add pure bar reconciliation planner with named tests - expect deterministic create/hide/show/rebuild decisions from monitor config
  What to do / Must NOT do: Add pure helper(s) that decide desired/created/hidden/shown/rebuilt monitor bar names without constructing GTK windows. Required shape: a `BarReconciliationPlan { create: Vec<String>, hide: Vec<String>, show: Vec<String>, rebuild: Vec<String>, close_removed: Vec<String> }` or equivalent, plus helpers such as `desired_bar_monitor_names(monitors, config)` and `plan_bar_reconciliation(existing_bar_states, monitors, config)`. Existing state must include monitor name and visible/hidden state so config-disable means `hide`, config-enable for a hidden bar means `show`, and monitor removal means `close_removed`. Add tests named exactly `bar_reconciliation_defaults_all_monitors_enabled`, `bar_reconciliation_excludes_disabled_monitor`, `bar_reconciliation_hides_existing_disabled_bar`, `bar_reconciliation_creates_newly_enabled_missing_bar`, `bar_reconciliation_default_fallback_respects_config`, and `bar_reconciliation_monitor_added_preserves_existing_visible_bar`. Do not construct GTK objects in this todo.
  Parallelization: Wave 1 | Blocked by: 2 | Blocks: 4, 6
  References (executor has NO interview context - be exhaustive):
  - `hyprline-bar/src/main.rs:453-509`: startup currently creates bars for every monitor or a `default` fallback.
  - `hyprline-bar/src/main.rs:511-529`: config-change subscriber currently only rebuilds existing bars.
  - `hyprline-bar/src/main.rs:558-640`: monitor added/removed handling currently always creates/removes bars.
  - `hyprline-bar/src/domain/models.rs:13-18`: `Monitor { name, id }` shape for pure tests.
  - `hyprline-bar/src/config/widget_config.rs:523-552`: new helper from Todo 2 must be used instead of direct map access.
  - `hyprline-bar/src/ui/bar.rs:452-660`: `setup_event_listener()` installs non-cancellable timers/shared-state subscriptions, so config toggles must not destroy/recreate an existing bar and call setup repeatedly.
  Acceptance criteria (agent-executable):
  - Tests prove all monitors enabled by default produce all monitor names in Hyprland order.
  - Tests prove a disabled monitor is excluded from desired bars.
  - Tests prove an existing visible bar whose monitor becomes disabled is planned for hide, not destroy/recreate.
  - Tests prove a newly enabled monitor without a current bar is planned for create.
  - Tests prove no monitors still yields `default` desired when enabled, and yields no desired bars when `default` is disabled.
  - Existing enabled visible bars are planned for rebuild, not hide/show/create.
  - Existing hidden bars whose monitor becomes enabled are planned for show/present without setup duplication.
  - Monitor-added planning with a fresh full monitor snapshot preserves existing visible bars and creates only the newly enabled missing monitor bar.
  - Helper output is deterministic and does not construct GTK objects.
  - Each named `bar_reconciliation_*` test runs exactly one test and passes.
  QA scenarios (name the exact tool + invocation):
  - Happy: `set -o pipefail; mkdir -p .omo/evidence && (run_named() { name="$1"; log="/tmp/${name}.log"; cargo test -p hyprline-bar "$name" -- --nocapture 2>&1 | tee "$log"; grep -q "running 1 test" "$log"; grep -Eq "test .*::${name} \.\.\. ok|test ${name} \.\.\. ok" "$log"; }; run_named bar_reconciliation_defaults_all_monitors_enabled && run_named bar_reconciliation_excludes_disabled_monitor && run_named bar_reconciliation_hides_existing_disabled_bar && run_named bar_reconciliation_creates_newly_enabled_missing_bar && run_named bar_reconciliation_default_fallback_respects_config && run_named bar_reconciliation_monitor_added_preserves_existing_visible_bar) 2>&1 | tee .omo/evidence/task-3-disable-bar-per-monitor.log` passes.
  - Failure: Before implementation, run at least one new named test and record the expected red compile/fail state; after implementation rerun all named tests green. Do not leave temporary broken source edits behind.
  Commit: N | included only if the user later requests commits

- [x] 4. `hyprline-bar/src/main.rs` and `hyprline-bar/src/ui/bar.rs`: Wire runtime lifecycle through a tested fake action seam - expect disabled monitors are hidden/reused and enabled monitors are shown/created live
  What to do / Must NOT do: Add a minimal `Bar::hide()` wrapper in `hyprline-bar/src/ui/bar.rs` using GTK window visibility (`set_visible(false)` or the project-appropriate GTK4 equivalent); keep `Bar::close()` for actual monitor removal. In `main.rs`, replace `Vec<Bar>` with a small managed runtime state such as `ManagedBar { bar: Bar, visible: bool, listeners_setup: bool }`. Add an application helper seam that applies `BarReconciliationPlan` to managed bars, plus a fake lifecycle recorder for unit testing. The fake tests must be named exactly `bar_reconciliation_apply_records_hide_rebuild_create_actions` and `bar_reconciliation_disable_enable_disable_no_duplicate_setup`. They must assert exact deterministic actions such as `hide:DP-2`, `rebuild:DP-1`, `create:HDMI-A-1`, `setup:HDMI-A-1`, `present:HDMI-A-1`, and a disable -> enable -> disable sequence where setup for the same existing monitor occurs at most once. Then use the same application path for startup/config-change and the delayed monitor-added full-snapshot runtime logic. Existing visible bars for still-enabled monitors should call `rebuild_widgets()` on config changes. Bars disabled by config should call `hide()` once and remain managed as hidden. Hidden bars that become enabled should call `present()` and become visible without another `setup_event_listener()`. Newly enabled/added monitors without any managed bar should create, set up listeners once, present, and push exactly one managed bar. Monitor-removed events should close and remove the managed bar. Do not duplicate event listeners for existing bars. Do not change widget construction or profile selection logic in `Bar` beyond the minimal hide wrapper.
  Parallelization: Wave 2 | Blocked by: 2, 3 | Blocks: 6 | Can parallelize with: 5
  References (executor has NO interview context - be exhaustive):
  - `hyprline-bar/src/main.rs:461-509`: current startup bar creation branch.
  - `hyprline-bar/src/main.rs:520-527`: current config-change hot reload loop.
  - `hyprline-bar/src/main.rs:581-621`: current delayed monitor-added create path.
  - `hyprline-bar/src/main.rs:625-635`: current monitor-removed close path.
  - `hyprline-bar/src/main.rs:646-655`: all startup bars currently call `setup_event_listener()` and `present()` after creation.
  - `hyprline-bar/src/main.rs:63`: existing app activation path presents all application windows; after this feature it must not re-present windows hidden because their monitor has `bar_enabled = false`.
  - `hyprline-bar/src/ui/bar.rs:278-356`: `Bar::rebuild_widgets()` keeps per-monitor profile behavior through config lookup.
  - `hyprline-bar/src/ui/bar.rs:452-660`: event listeners/shared-state subscriptions are installed without unsubscribe handles, so config disable/enable must hide/reuse existing bars and guard setup with a managed `listeners_setup` flag.
  - `hyprline-bar/src/ui/bar.rs:691-703`: `present()`, `monitor_name()`, and `close()` are existing lifecycle helpers; add only a minimal hide helper beside them.
  Acceptance criteria (agent-executable):
  - Startup skips disabled monitors and logs that a monitor bar is disabled/skipped; no hidden bar is pre-created for startup-disabled monitors.
  - Startup still creates the `default` fallback bar when no monitors are found and `default` is enabled.
  - Config changes hide bars that are now disabled, keep them in managed state, and do not call `setup_event_listener()` again on later re-enable.
  - Config changes create/setup/present bars that are newly enabled and currently missing.
  - Config changes present hidden bars that become enabled without creating a duplicate or registering duplicate listeners.
  - Config changes rebuild widgets only for bars that remain enabled and visible.
  - App activation/present-all behavior is guarded so bars hidden by per-monitor disable are not accidentally presented again while still disabled.
  - Monitor-added events use `service.get_monitors()` after the delay to obtain a fresh full monitor snapshot, then apply the shared reconciliation path; the added disabled monitor creates no visible bar and existing visible bars are not hidden/closed/recreated.
  - Existing monitor-removed handling remains functional.
  - The fake lifecycle action tests prove the actual application order for hide/show/rebuild/create/setup/present and idempotent disable -> enable -> disable behavior.
  - `run_named bar_reconciliation_apply_records_hide_rebuild_create_actions`, `run_named bar_reconciliation_disable_enable_disable_no_duplicate_setup`, and `cargo build -p hyprline-bar` pass.
  QA scenarios (name the exact tool + invocation):
  - Happy: `set -o pipefail; mkdir -p .omo/evidence && (run_named() { name="$1"; log="/tmp/${name}.log"; cargo test -p hyprline-bar "$name" -- --nocapture 2>&1 | tee "$log"; grep -q "running 1 test" "$log"; grep -Eq "test .*::${name} \.\.\. ok|test ${name} \.\.\. ok" "$log"; }; run_named bar_reconciliation_apply_records_hide_rebuild_create_actions && run_named bar_reconciliation_disable_enable_disable_no_duplicate_setup && cargo build -p hyprline-bar) 2>&1 | tee .omo/evidence/task-4-disable-bar-per-monitor.log` passes.
  - Failure: Before implementation, run the named fake action test and record the expected red compile/fail state; after implementation rerun green. Do not leave temporary broken source edits behind.
  Commit: N | included only if the user later requests commits

- [x] 5. `hyprline-bar/src/ui/settings.rs`: Add per-monitor bar visibility switch in monitor settings - expect toggles save config and notify hot reload
  What to do / Must NOT do: Extend `SettingsWindow::create_monitors_settings` rows with a labelled `gtk4::Switch` (e.g. label `Show bar`) initialized from `config.is_bar_enabled_for_monitor(&monitor.name)`. Connect the signal only after initial `set_active`. Use `connect_active_notify` unless GTK API evidence requires a different signal; if different, document why in the evidence log. The callback must call a small named helper, e.g. `save_monitor_bar_enabled(monitor_name, enabled)`, and that helper must call `config.set_monitor_bar_enabled(...)`, drop the write lock, then call `save_config()`. Keep existing profile ComboBox behavior unchanged. Do not add a new settings page.
  Parallelization: Wave 2 | Blocked by: 2 | Blocks: 6 | Can parallelize with: 4
  References (executor has NO interview context - be exhaustive):
  - `hyprline-bar/src/ui/settings.rs:268-364`: current monitor settings page and per-monitor profile ComboBox save path.
  - `hyprline-bar/src/ui/settings.rs:337-350`: existing callback pattern writes config and calls `save_config()`.
  - `hyprline-bar/src/ui/settings.rs:1218-1228`: Monitors menu entry already exists.
  - `hyprline-bar/src/ui/settings.rs:1270-1297`: settings page selection creates monitor settings with a `WorkspaceService`.
  - `hyprline-bar/src/config/widget_config.rs:703-709`: `save_config()` writes config and notifies config-change subscribers.
  Acceptance criteria (agent-executable):
  - Each detected monitor row includes the existing profile selector and a bar visibility switch with a visible label.
  - Switch initial state reflects persisted config, defaulting to active/on for missing entries.
  - Signal is connected after initial `set_active`, preventing initialization from saving.
  - Toggling off calls a named helper that saves `bar_enabled: false` under that monitor entry and triggers `save_config()`.
  - Toggling on calls the same helper to save `bar_enabled: true` and triggers `save_config()`.
  - Existing profile selection still compiles and uses `set_monitor_profile` unchanged.
  - Structural verification proves the switch callback path contains `set_monitor_bar_enabled` and `save_config` through the named helper, not only grep-only unrelated hits.
  - `cargo build -p hyprline-bar` passes.
  QA scenarios (name the exact tool + invocation):
  - Happy: `set -o pipefail; mkdir -p .omo/evidence && (cargo build -p hyprline-bar && python3 - <<'PY'
from pathlib import Path
text = Path('hyprline-bar/src/ui/settings.rs').read_text()
def body_after(marker: str) -> str:
    idx = text.find(marker)
    if idx < 0:
        raise SystemExit(f'missing marker: {marker}')
    start = text.find('{', idx)
    if start < 0:
        raise SystemExit(f'missing opening brace after: {marker}')
    depth = 0
    for pos in range(start, len(text)):
        ch = text[pos]
        if ch == '{':
            depth += 1
        elif ch == '}':
            depth -= 1
            if depth == 0:
                return text[start + 1:pos]
    raise SystemExit(f'missing closing brace after: {marker}')
helper = body_after('fn save_monitor_bar_enabled')
ordered = ['get_config().write()', 'set_monitor_bar_enabled', 'drop(config)', 'save_config()']
cursor = -1
for needle in ordered:
    nxt = helper.find(needle, cursor + 1)
    if nxt < 0:
        raise SystemExit(f'helper missing ordered call: {needle}')
    cursor = nxt
switch_idx = text.find('connect_active_notify')
if switch_idx < 0:
    raise SystemExit('missing connect_active_notify')
region = text[max(0, switch_idx - 1000): switch_idx + 1400]
before_signal = region[:region.find('connect_active_notify')]
if 'set_active' not in before_signal:
    raise SystemExit('set_active not found before connect_active_notify in switch region')
callback = body_after('connect_active_notify')
if 'save_monitor_bar_enabled' not in callback:
    raise SystemExit('switch callback does not call save_monitor_bar_enabled')
if 'Show bar' not in text:
    raise SystemExit('missing visible Show bar label')
print('settings switch scoped structural check passed')
PY
) 2>&1 | tee .omo/evidence/task-5-disable-bar-per-monitor.log` passes.
  - Failure: Temporarily remove the named helper call from the switch callback, rerun the structural Python check, confirm it fails, then restore and rerun green. Do not leave temporary broken source edits behind.
  Commit: N | included only if the user later requests commits

- [x] 6. Full gates and data-shaped QA: Verify per-monitor disable is persisted and runtime decisions match settings - expect tests/build pass, disable-enable-disable lifecycle evidence, and no forbidden runtime path
  What to do / Must NOT do: Run full local gates, capture final diffs, and perform data-shaped QA against test logs/config serialization proving the stored setting shape. If GUI session and automation tools are available, optionally open Settings and capture a screenshot/log of the Monitors page; if unavailable, record the limitation and rely on the required tests/structural checks/build evidence. Do not use `hyprctl`, do not edit user config, and do not deploy/reinstall.
  Parallelization: Wave 3 | Blocked by: 4, 5 | Blocks: final verification
  References (executor has NO interview context - be exhaustive):
  - `.omo/evidence/task-2-disable-bar-per-monitor.log`: config tests evidence.
  - `.omo/evidence/task-3-disable-bar-per-monitor.log`: reconciliation planner tests evidence.
  - `.omo/evidence/task-4-disable-bar-per-monitor.log`: runtime fake lifecycle/build evidence.
  - `.omo/evidence/task-5-disable-bar-per-monitor.log`: settings wiring evidence.
  Acceptance criteria (agent-executable):
  - Every named test listed in Verification strategy passes and logs a nonzero one-test run.
  - `cargo test -p hyprline-bar` passes.
  - `cargo build -p hyprline-bar` passes.
  - Final diff is captured to `.omo/evidence/final-target-diff-disable-bar-per-monitor.diff`.
  - Addition-only forbidden-runtime grep finds no `hyprctl`, `Command::new`, `std::process`, `zbus`, or `dbus` additions.
  - Config evidence records old missing `bar_enabled` loading as enabled and a disabled monitor serializing `"bar_enabled": false`.
  - Lifecycle evidence records disable -> enable -> disable for the same existing monitor with no duplicate `setup` action.
  - If GUI automation is unavailable, evidence explicitly says so instead of claiming a click was performed.
  QA scenarios (name the exact tool + invocation):
  - Happy: `set -o pipefail; mkdir -p .omo/evidence && (run_named() { name="$1"; log="/tmp/${name}.log"; cargo test -p hyprline-bar "$name" -- --nocapture 2>&1 | tee "$log"; grep -q "running 1 test" "$log"; grep -Eq "test .*::${name} \.\.\. ok|test ${name} \.\.\. ok" "$log"; }; run_named monitor_bar_defaults_missing_visibility_to_enabled && run_named monitor_bar_unknown_monitor_defaults_enabled && run_named monitor_bar_set_visibility_preserves_profile && run_named monitor_bar_disabled_serializes_false && run_named bar_reconciliation_defaults_all_monitors_enabled && run_named bar_reconciliation_excludes_disabled_monitor && run_named bar_reconciliation_hides_existing_disabled_bar && run_named bar_reconciliation_creates_newly_enabled_missing_bar && run_named bar_reconciliation_default_fallback_respects_config && run_named bar_reconciliation_monitor_added_preserves_existing_visible_bar && run_named bar_reconciliation_apply_records_hide_rebuild_create_actions && run_named bar_reconciliation_disable_enable_disable_no_duplicate_setup && cargo test -p hyprline-bar && cargo build -p hyprline-bar && git diff -- hyprline-bar/src/config/widget_config.rs hyprline-bar/src/main.rs hyprline-bar/src/ui/settings.rs hyprline-bar/src/ui/bar.rs > .omo/evidence/final-target-diff-disable-bar-per-monitor.diff && if git diff -U0 -- hyprline-bar/src | grep -E '^\+[^+].*(hyprctl|Command::new|std::process|zbus|dbus)'; then echo 'forbidden runtime path found'; exit 1; else echo 'no forbidden runtime path added'; fi) 2>&1 | tee .omo/evidence/task-6-disable-bar-per-monitor.log` passes.
  - Failure: If any named test reports zero tests or any gate fails, stop and fix before final verification; evidence log must include the failing command and the later passing rerun.
  Commit: N | do not commit unless the user explicitly asks after implementation

## Final verification wave
> Runs in parallel after ALL todos. ALL must APPROVE. Surface results and wait for the user's explicit okay before declaring complete.
- [x] F1. Plan compliance audit
  - Read this plan, final target diff, and all task logs. Verify every Must Have is satisfied and every Must NOT Have is absent. Evidence: append verdict to `.omo/evidence/final-disable-bar-per-monitor.log`.
- [x] F2. Code quality review
  - Review Rust config/runtime/settings changes for clean ownership, no duplicated lifecycle logic, no panic-prone config access beyond existing patterns, no subprocess/D-Bus additions, no restart-only weakening, and no event listener duplication. Required commands: `cargo test -p hyprline-bar` and `cargo build -p hyprline-bar`. Evidence: append verdict to `.omo/evidence/final-disable-bar-per-monitor.log`.
- [x] F3. Agent-executed real-surface QA
  - Validate the required non-GUI real surface: persisted config shape, named reconciliation tests, fake lifecycle action/idempotency proof, and settings structural wiring. If GUI automation/display is available, additionally open Settings → Monitors and toggle one monitor while recording the config/bar result; if unavailable, state that limitation. Evidence: append verdict to `.omo/evidence/final-disable-bar-per-monitor.log`.
- [x] F4. Scope fidelity
  - Verify only intended files changed for this plan (`hyprline-bar/src/config/widget_config.rs`, `hyprline-bar/src/main.rs`, `hyprline-bar/src/ui/settings.rs`, minimal `hyprline-bar/src/ui/bar.rs`, plus `.omo`). Inspect `git status --short`, `git diff --stat`, and scoped diffs. Evidence: append verdict to `.omo/evidence/final-disable-bar-per-monitor.log`.

## Commit strategy
- Do not commit by default; the user has not requested a commit for this plan yet.
- If the user explicitly asks after implementation, prefer one atomic commit:
  - `add per-monitor bar visibility setting`
- Stage only intended product files and relevant `.omo` artifacts if the user says to commit planning/evidence too.
- Before any commit, inspect `GIT_MASTER=1 git status --short`, `GIT_MASTER=1 git diff`, and `GIT_MASTER=1 git log --oneline -10`.

## Success criteria
- A user can open Settings → Monitors and enable/disable the bar for each detected monitor.
- The setting is saved in config JSON and survives app restart/config reload.
- Old configs without the new field continue to show bars on all monitors.
- Disabled monitors have no bar at startup and have any existing live bar hidden on a settings change.
- Re-enabled monitors get a hidden bar re-presented or a missing bar created without restarting the app, and without duplicate listener setup.
- Existing per-monitor profile selection still works for enabled bars.
- Tests and build pass, and no forbidden monitor-control subprocess/D-Bus path is added.
