2026-06-21T16:08:51+03:00
- Added a test-only `UnixListener` fake inside `hyprland_ipc.rs` that accepts one command per connection, records commands in order, writes a configured response, and then drops the stream.
- Serialized env-mutating workspace-switch tests with a static `Mutex`, but held the lock for the whole test (socket setup + env set + assertion) to avoid cross-test races around `HYPRLAND_INSTANCE_SIGNATURE` / `XDG_RUNTIME_DIR`.
- Restored previous env values in a drop guard so targeted tests can point `get_control_socket()` at `{tmpdir}/hypr/{sig}/.socket.sock` without requiring a live Hyprland instance.
- Kept fake socket paths short because Unix socket path length limits caused flaky connect/record behavior when temporary directory names were too long.
- Current red state is correct: legacy success and missing-socket cases pass now, while both Lua-fallback tests fail because production `switch_workspace()` still emits only `dispatch workspace {id}` and never issues `dispatch hl.dsp.focus({ workspace = id })`.

2026-06-21T16:14:47+03:00
- Implemented `switch_workspace()` via `send_request()` so the legacy `dispatch workspace {id}` path now reads and trims the IPC response, returns immediately on `ok`, and only retries once when `should_try_lua_workspace_fallback()` sees the known Lua-dispatch markers (`hl.dispatch`, `dispatch in lua`, `syntax might need to be updated`).
- Kept transport failures non-fatal: connect/write/read/utf8 errors from either command now log with `eprintln!` and return without retrying on transport failure, while a non-`ok` fallback response logs both the original and fallback responses.
- Failure proof succeeded: after temporarily disabling the fallback trigger, `cargo test -p hyprline-bar workspace_switch -- --nocapture` went red (`2 passed; 2 failed`), then restoring the branch returned the same command set to green (`4 passed, 8 filtered out`).


2026-06-21T16:18:57+03:00
- Ran the required repository gate pipeline into `.omo/evidence/task-3-workspace-click-lua-dispatch-fallback.log`: `cargo test -p hyprline-bar workspace_switch -- --nocapture`, `cargo test -p hyprline-bar workspace_ -- --nocapture`, `cargo test -p hyprline-bar`, and `cargo build -p hyprline-bar` all passed; the baseline-vs-final unrelated tracked diff comparison also passed, and the addition-only forbidden-runtime grep reported `no forbidden runtime path added`.
- Captured `.omo/evidence/final-target-diff-workspace-click-lua-dispatch-fallback.diff` for `hyprline-bar/src/infrastructure/hyprland_ipc.rs` plus `.omo/evidence/final-unrelated-tracked-workspace-click-lua-dispatch-fallback.diff` for the rest of the tracked worktree without modifying product files in this task.
- Live direct Unix-socket QA succeeded against `/run/user/1000/hypr/a0136d8c04687bb36eb8a28eb9d1ff92aea99704_1782039162_1172114950/.socket.sock`: `j/activeworkspace` returned workspace id `1`, `dispatch workspace 1` returned the expected Lua-dispatch syntax error, and `dispatch hl.dsp.focus({ workspace = 1 })` returned `ok`.