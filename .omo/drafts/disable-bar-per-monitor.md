---
slug: disable-bar-per-monitor
status: high-accuracy-approved-ready-for-start-work
intent: clear
pending-action: present plan summary and ask whether to start work or run high-accuracy review
approach: Add a persisted per-monitor bar visibility flag to the existing monitor config, expose it as a switch on the Monitors settings page, and make startup/monitor/config-change reconciliation create, hide, show, or close managed Bar windows according to that flag. Config toggles hide/reuse existing bars instead of destroying/recreating them because Bar event listeners/subscriptions are currently non-cancellable; actual monitor removal still closes/removes.
---

# Draft: disable-bar-per-monitor

## Components (topology ledger)
<!-- Lock the SHAPE before depth. One row per top-level component that can succeed or fail independently. -->
<!-- id | outcome (one line) | status: active|deferred | evidence path -->
- C1 | Config schema persists per-monitor bar visibility with default enabled/backward compatibility | active | hyprline-bar/src/config/widget_config.rs:205-217,274-302,488-504,538-552
- C2 | Runtime bar lifecycle skips disabled monitors at startup and reconciles added/removed/config-change events with managed hide/reuse semantics | active | hyprline-bar/src/main.rs:453-509,511-529,532-644,646-655; hyprline-bar/src/ui/bar.rs:452-703
- C3 | Settings UI exposes a saved per-monitor toggle alongside existing monitor profile selection | active | hyprline-bar/src/ui/settings.rs:268-364,1180-1315
- C4 | Verification proves config persistence, runtime filtering/reconciliation, and settings save behavior without relying on human UI clicks | active | hyprline-bar/src/infrastructure/hyprland_ipc.rs:339-789, hyprline-bar/src/config/widget_config.rs:387-504

## Open assumptions (announced defaults)
<!-- Record any default you adopt instead of asking, so the user can veto it at the gate. -->
<!-- assumption | adopted default | rationale | reversible? -->
- Newly introduced `bar_enabled` defaults to `true` for all monitors | Existing users should see no behavior change until they opt out | Required for backward-compatible config load | yes
- Disabling a monitor should take effect immediately after saving settings, not only after restart | Existing config-change subscription already hot-rebuilds widgets; a monitor visibility setting belongs in the same live settings flow | yes
- Config-toggle disable should hide/reuse an existing live bar rather than close/recreate it | `Bar::setup_event_listener()` installs non-cancellable timers and shared-state subscriptions, so destroying/recreating on toggles can duplicate background work | yes
- Settings UI should allow disabling every detected monitor; app remains running but may have no visible bar until config is edited/re-enabled | User asked for a monitor-level disable feature, and blocking the last toggle would add extra policy not present in existing settings | yes
- Monitor identity is the Hyprland monitor `name` already used as the key in `config.monitors` | Existing monitor profile settings use the same key and runtime bars are addressed by `Bar::monitor_name()` | yes

## Findings (cited - path:lines)
- `HyprlineConfig` already has `monitors: HashMap<String, MonitorConfig>` and `MonitorConfig { profile_name }`, making it the natural persisted schema point for per-monitor bar visibility (`hyprline-bar/src/config/widget_config.rs:205-217`, `274-302`).
- Config load/save is JSON-backed at `$XDG_CONFIG_HOME/hyprline/config.json`, using serde defaults for backward-compatible fields (`hyprline-bar/src/config/widget_config.rs:374-504`).
- Current monitor settings page enumerates `workspace_service.get_monitors()`, displays each monitor name, and saves profile selection through `config.set_monitor_profile(...); save_config()` (`hyprline-bar/src/ui/settings.rs:268-364`).
- Settings navigation already includes a Monitors page and creates a `HyprlandIpc` workspace service for it (`hyprline-bar/src/ui/settings.rs:1218-1228`, `1270-1297`).
- Startup currently creates one `Bar` per Hyprland monitor, or a `default` bar when no monitors are reported (`hyprline-bar/src/main.rs:453-509`).
- Runtime monitor-added events currently always create a bar for the new monitor if one does not already exist; monitor-removed events close existing bars (`hyprline-bar/src/main.rs:558-640`).
- Config-change subscription currently only calls `bar.rebuild_widgets()` for existing bars, so a visibility setting needs a reconciliation path that can hide disabled existing bars, re-show hidden bars, and create newly enabled missing bars (`hyprline-bar/src/main.rs:511-529`).
- `Bar` exposes `monitor_name()`, `close()`, `present()`, and `setup_event_listener()` methods used by lifecycle code (`hyprline-bar/src/main.rs:587-619`, `625-635`, `646-655`). Existing app activation presents all windows (`hyprline-bar/src/main.rs:63`), so the plan includes a guard to prevent disabled hidden bars from being re-presented.

## Decisions (with rationale)
- Store the setting as `MonitorConfig::bar_enabled: bool` with `#[serde(default = "default_bar_enabled")]`, preserving old config files where the field is absent.
- Add config helpers such as `is_bar_enabled_for_monitor(&str) -> bool` and `set_monitor_bar_enabled(&str, bool)` to keep settings/runtime code from duplicating map/default behavior.
- Extend the Monitors settings page row with a `gtk4::Switch` labelled like `Show bar` / `Bar enabled`, initialized from config and saved via the new helper plus `save_config()`.
- Update startup, monitor-added handling, and config-change handling to use one shared reconciliation helper in `main.rs` so behavior is consistent and testable; monitor-added must use a fresh full monitor snapshot after the delay, not a partial single-name list.
- Add minimal `Bar::hide()` in `hyprline-bar/src/ui/bar.rs`; use hide/show reuse for config toggles and reserve `Bar::close()` for monitor removal.
- Treat `default` fallback bar as enabled unless config explicitly has a `default` monitor entry with `bar_enabled: false`.
- Metis review required corrections: remove plan placeholders, add baseline capture, replace restart escape hatch with stop-for-replan, require filtered tests to prove one test ran so cargo cannot pass zero tests, add fake lifecycle action seam, and replace grep-only settings QA with structural helper/callback verification. High-accuracy Oracle later required switching from `--exact` to unique-name filtered tests because module-scoped exact matching can report zero tests.
- Final Metis re-review returned `OKAY` after those corrections.
- High-accuracy rereview approved after lifecycle/test revisions: Momus `ses_1155d8f61ffe60YPLaCpYfuvuJ` returned `OKAY`; independent Oracle `ses_1155d8f5bffeGuPbcZHf7Vekz5` returned `APPROVE` with no blocking changes. Non-blocking activation hardening was folded into the plan.

## Scope IN
- Persisted per-monitor flag to enable/disable hyprline-bar visibility.
- Settings UI control in the existing Monitors page.
- Immediate runtime application on settings save: hiding disabled existing monitor bars, re-showing hidden enabled bars, and creating bars for enabled monitors that do not yet have one.
- Backward-compatible config loading for existing `config.json` files.
- Agent-executed Rust tests plus command/build/live or scripted QA evidence.

## Scope OUT (Must NOT have)
- Do not redesign the profiles/widgets settings model beyond the monitor visibility flag.
- Do not change widget ordering/profile behavior except where an enabled bar chooses its existing monitor profile.
- Do not edit user Hyprland config files.
- Do not add `hyprctl` subprocesses or D-Bus paths for monitor detection; keep existing `WorkspaceService::get_monitors()` / Hyprland IPC path.
- Do not implement restart-only behavior. If immediate reconciliation appears unsafe or impossible, stop and request a plan revision instead of weakening the requirement.

## Open questions
- None blocking. User can veto announced defaults at approval gate.

## Approval gate
status: high-accuracy-approved-ready-for-start-work
pending action: user may run `$start-work disable-bar-per-monitor` when ready.
<!-- When exploration is exhausted and unknowns are answered, set status: awaiting-approval. -->
<!-- That durable record is the loop guard: on a later turn read it and resume at the gate instead of re-running exploration. -->
