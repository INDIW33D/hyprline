# F3 Real Manual QA

Verdict: APPROVE

## Live direct-socket evidence
- `.omo/evidence/task-3-workspace-click-lua-dispatch-fallback.log:1907-1913` shows `active_workspace_id=1`, then `legacy_command=dispatch workspace 1`.
- The same live QA section records the direct Lua-dispatch failure at `:1909` as `error: [string "return hl.dispatch(workspace 1)"]:1: ')' expected near '1'`, which is the expected raw-command syntax breakage.
- The fallback path is then exercised immediately: `fallback_command=dispatch hl.dsp.focus({ workspace = 1 })` at `:1912` and `fallback_response=ok` at `:1913`.

## Fake-socket click-dispatch proof
- `.omo/evidence/task-2-workspace-click-lua-dispatch-fallback.log:10-14` shows the fallback was restored and the final workspace test subset passed green (`cargo test: 12 passed`).
- The task-2 summary is a rerun after the task-1 red phase, where `.omo/evidence/task-1-workspace-click-lua-dispatch-fallback.log:9-11` explicitly recorded the missing second click-dispatch command before the fix: only `dispatch workspace 7` was observed and the expected fallback `dispatch hl.dsp.focus({ workspace = 7 })` was absent.
- The resulting implementation diff in `.omo/evidence/final-target-diff-workspace-click-lua-dispatch-fallback.diff:517-536` ties that green rerun to the fake-socket assertion for the exact command sequence: first `dispatch workspace 7`, then `dispatch hl.dsp.focus({ workspace = 7 })`.

## GUI automation attempt (if any)
- Tool checked: `ydotool`, `wtype`, `xdotool`, Python `pyautogui`, Python `pynput`.
- Result: unavailable in this executor environment (`ydotool=False`, `wtype=False`, `xdotool=False`, `pyautogui=False`, `pynput=False`), so no real bar click could be performed here.

## Overall reasoning
The user-facing fallback is approved because the closest real surface available in this environment shows the exact failure mode and the exact recovery path on a live Hyprland socket: raw `dispatch workspace <id>` fails with a Lua syntax error, while `dispatch hl.dsp.focus({ workspace = <id> })` succeeds with `ok`. The fake-socket proof is also sufficient: task-1 captured the red failure where the second dispatch was missing, task-2 captured the restored green rerun, and the final diff shows the fake-socket tests assert the full click-dispatch sequence. GUI click automation was not available, and that limitation is explicitly noted rather than guessed around.
