# F4 Scope Fidelity

Verdict: APPROVE

## Command results
- `git status --short`:
  ```text
   M Cargo.lock
   M README.md
   M hyprline-bar/Cargo.toml
   M hyprline-bar/src/config/bar_config.rs
   M hyprline-bar/src/config/mod.rs
   M hyprline-bar/src/config/widget_config.rs
   M hyprline-bar/src/domain/mod.rs
   M hyprline-bar/src/domain/models.rs
   M hyprline-bar/src/infrastructure/dbus_status_notifier_watcher.rs
   M hyprline-bar/src/infrastructure/hyprland_ipc.rs
   M hyprline-bar/src/infrastructure/lumen_brightness.rs
   M hyprline-bar/src/infrastructure/mod.rs
   M hyprline-bar/src/infrastructure/pipewire_volume.rs
   M hyprline-bar/src/main.rs
   M hyprline-bar/src/shared_state.rs
   M hyprline-bar/src/styles.css
   M hyprline-bar/src/ui/bar.rs
   M hyprline-bar/src/ui/mod.rs
   M hyprline-bar/src/ui/settings.rs
   M hyprline-notifications/Cargo.toml
   M hyprline-notifications/src/dbus_service.rs
  ?? .omo/
  ?? hyprline-bar/src/domain/bluetooth_service.rs
  ?? hyprline-bar/src/infrastructure/bluez_bluetooth.rs
  ?? hyprline-bar/src/ui/bluetooth.rs
  ```
- `git diff --stat`:
  ```text
   Cargo.lock                                         | 570 ++++---------------
   README.md                                          |  49 +-
   hyprline-bar/Cargo.toml                            |   4 +-
   hyprline-bar/src/config/bar_config.rs              |  12 +-
   hyprline-bar/src/config/mod.rs                     |  41 --
   hyprline-bar/src/config/widget_config.rs           | 245 ++++++--
   hyprline-bar/src/domain/mod.rs                     |  20 +-
   hyprline-bar/src/domain/models.rs                  |  62 +-
   .../infrastructure/dbus_status_notifier_watcher.rs |  53 +-
   hyprline-bar/src/infrastructure/hyprland_ipc.rs    | 631 ++++++++++++++++++++-
   .../src/infrastructure/lumen_brightness.rs         |  72 ++-
   hyprline-bar/src/infrastructure/mod.rs             |  23 +-
   hyprline-bar/src/infrastructure/pipewire_volume.rs |  28 +-
   hyprline-bar/src/main.rs                           | 267 +++++----
   hyprline-bar/src/shared_state.rs                   |  93 ++-
   hyprline-bar/src/styles.css                        | 235 ++++++--
   hyprline-bar/src/ui/bar.rs                         |  99 +++-
   hyprline-bar/src/ui/mod.rs                         |  20 +-
   hyprline-bar/src/ui/settings.rs                    | 167 +++---
   hyprline-notifications/Cargo.toml                  |   3 +-
   hyprline-notifications/src/dbus_service.rs         |  31 +-
   21 files changed, 1781 insertions(+), 944 deletions(-)
  ```
- `diff -u baseline... final...`:
  ```text
  $ diff -u .omo/evidence/baseline-unrelated-tracked-workspace-click-lua-dispatch-fallback.diff .omo/evidence/final-unrelated-tracked-workspace-click-lua-dispatch-fallback.diff
  (no output)
  ```
- `git diff --cached --name-only`:
  ```text
  (no output)
  ```

## Scoped diff summary
- I computed the plan delta with:
  ```text
  diff -u .omo/evidence/baseline-target-diff-workspace-click-lua-dispatch-fallback.diff .omo/evidence/final-target-diff-workspace-click-lua-dispatch-fallback.diff > /tmp/this-plan-delta.diff
  ```
- That delta shows the baseline already contained the pre-existing `hyprland_ipc.rs` workspace-label parsing changes (`get_workspace_key_labels`, `parse_workspace_bind_labels`, `HyprlandBind`, label tests, and related imports/types). Those are not part of this plan's delta.
- The only new product-file changes introduced by this plan are:
  - `should_try_lua_workspace_fallback(response: &str)` helper for the known Lua-dispatch error markers.
  - `switch_workspace(&self, id: i32)` changed from write-and-ignore to response-aware legacy-first dispatch with exactly one Lua fallback command: `dispatch hl.dsp.focus({ workspace = <id> })`.
  - Related fake-socket test support and the `workspace_switch_*` tests:
    - `workspace_switch_legacy_success_sends_workspace_dispatch_once`
    - `workspace_switch_lua_dispatch_error_tries_lua_fallback_after_workspace_dispatch`
    - `workspace_switch_lua_dispatch_error_and_failed_fallback_do_not_panic`
    - `workspace_switch_missing_socket_does_not_panic`
- No new delta was introduced in excluded tracked files between baseline and final: the unrelated tracked diff comparison is byte-identical.
- No staged files are present.

## Overall reasoning
F4 should be judged against what this plan changed relative to its captured baseline, not against the full dirty worktree. On that basis, this plan passes.

The worktree still contains many pre-existing dirty changes in files like `README.md`, `hyprline-bar/src/styles.css`, and others, but the required unrelated tracked diff comparison shows those changes did not change between baseline and final for this plan. They are therefore not part of this plan's delta.

Within `hyprline-bar/src/infrastructure/hyprland_ipc.rs`, the baseline-to-final delta is limited to the intended `switch_workspace` Lua-dispatch fallback behavior and the directly related fake-socket `workspace_switch_*` tests/helpers needed to verify that behavior. There are no staged files, and no unrelated tracked-file delta was introduced by this plan. Therefore the F4 scope-fidelity check is APPROVE.
