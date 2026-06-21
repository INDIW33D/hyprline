---
slug: workspace-click-lua-dispatch-fallback
status: plan-written-high-accuracy-approved
intent: clear
pending-action: user chooses whether to start implementation
approach: Minimal IPC-only fix: keep legacy `dispatch workspace <id>` first, inspect its socket response, and retry exactly once with Lua-compatible `dispatch hl.dsp.focus({ workspace = <id> })` only for known Hyprland Lua-dispatch command errors.
---

# Draft: workspace-click-lua-dispatch-fallback

## Components (topology ledger)
<!-- Lock the SHAPE before depth. One row per top-level component that can succeed or fail independently. -->
<!-- id | outcome (one line) | status: active|deferred | evidence path -->
- C1 | Workspace click dispatch reaches Hyprland with a valid command under both legacy and Lua config modes | active | `hyprline-bar/src/ui/workspaces.rs:89-98`, `hyprline-bar/src/infrastructure/hyprland_ipc.rs:289-296`
- C2 | Tests/QA prove response-aware fallback without requiring live Hyprland in unit tests | active | fake Unix socket plan, live socket probe evidence below

## Open assumptions (announced defaults)
<!-- Record any default you adopt instead of asking, so the user can veto it at the gate. -->
<!-- assumption | adopted default | rationale | reversible? -->
- Keep `WorkspaceService::switch_workspace(&self, id: i32)` returning `()` | avoid rippling trait/caller changes through UI; errors are logged, not surfaced | yes, a later plan could change the trait if UI error display is desired
- Do not change `EventControllerLegacy` in this fix | live socket evidence proves current dispatch command fails under Lua config; UI event rewrite is a separate risk and should not be bundled | yes, if IPC fallback passes but clicks still fail, plan a second narrow UI-event fix
- Retry only on known Lua-dispatch command errors, not all failures | avoids masking transport errors or unrelated Hyprland failures | yes

## Findings (cited - path:lines)
- `hyprline-bar/src/ui/workspaces.rs:89-98`: the only workspace click handler clones the workspace service and calls `service.switch_workspace(ws_id)` on `gdk::EventType::ButtonPress`.
- `hyprline-bar/src/infrastructure/hyprland_ipc.rs:289-296`: `switch_workspace` currently connects to the Hyprland control socket, sends `dispatch workspace {id}`, and ignores both response and errors.
- Live direct socket probe: `dispatch workspace 11` returned `error: [string "return hl.dispatch(workspace 11)"]:1: ')' expected near '11' ... Note: dispatch in lua is a shorthand for hl.dispatch(...), your syntax might need to be updated.`
- Live direct socket probe: `dispatch hl.dsp.focus({ workspace = 11 })` returned `ok`.
- `hyprline-bar/src/infrastructure/hyprland_ipc.rs:64-93`: existing `send_request` already writes a command and reads the full response, so `switch_workspace` can reuse it instead of write-and-ignore.
- `hyprline-bar/src/ui/brightness.rs:43-50`, `hyprline-bar/src/ui/network.rs:35-42`, `hyprline-bar/src/ui/system_tray.rs:101-114`: other clickable widgets use `GestureClick`; this remains a secondary risk, not the primary evidenced cause.

## Decisions (with rationale)
- Use legacy-first fallback, not Lua-only replacement, to preserve old/non-Lua Hyprland behavior.
- Trigger fallback only when the first response contains known Lua-dispatch/syntax markers (`hl.dispatch`, `dispatch in lua`, or `syntax might need to be updated`) and is not `ok`.
- Retry at most once with `dispatch hl.dsp.focus({ workspace = <id> })` using numeric `i32` formatting only; no workspace label/string interpolation.
- Log command-level and transport failures with `eprintln!` because the public trait currently returns `()`.
- Test via fake Unix socket using temporary `XDG_RUNTIME_DIR`/`HYPRLAND_INSTANCE_SIGNATURE` and an environment mutex; do not require live Hyprland in unit tests.
- Metis `ses_115e0c2a5ffemwl5LuZsXsYdS6` rejected the initial loose approach until exact fallback trigger, no retry loops, transport handling, and fake-socket tests were specified; these are now included.

## Scope IN
- `hyprline-bar/src/infrastructure/hyprland_ipc.rs`: tests and implementation for response-aware `switch_workspace` dispatch.
- Optional `.omo/evidence/**` artifacts for baseline, test logs, static checks, and live direct-socket QA.

## Scope OUT (Must NOT have)
- Do not edit `/home/indiw33d/.config/hypr/hyprland.lua`.
- Do not add `hyprctl`, `Command::new`, `std::process`, or any runtime subprocess fallback.
- Do not change workspace label parsing, README docs, UI rendering, CSS, tray, Bluetooth, notifications, brightness, or settings.
- Do not rewrite workspace click handling (`EventControllerLegacy`/`GestureClick`) in this plan unless IPC fallback tests pass and live QA still proves clicks fail.
- Do not stage or commit unless the user explicitly asks later.

## Open questions
- None blocking. User approved the IPC fallback approach after the live socket evidence was explained.

## Approval gate
status: approved-plan-written
<!-- When exploration is exhausted and unknowns are answered, set status: awaiting-approval. -->
<!-- That durable record is the loop guard: on a later turn read it and resume at the gate instead of re-running exploration. -->
- User approved the approach with: `approve`.

## High-accuracy review
- User requested high-accuracy review.
- Native Momus review running: `bg_643dfad1`.
- Independent Oracle replacement for unavailable Codex CLI review running: `bg_919ad881`.
- `codex` CLI was not found in PATH, so the independent reviewer substitutes for the Codex CLI pass.
- Final verdicts:
  - Momus `bg_643dfad1`: `VERDICT: OKAY`.
  - Independent Oracle `bg_919ad881`: `VERDICT: APPROVE`.
- No mandatory plan changes requested.
