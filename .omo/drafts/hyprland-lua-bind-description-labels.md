---
slug: hyprland-lua-bind-description-labels
status: plan-written-high-accuracy-approved
intent: clear
pending-action: user chooses whether to start implementation
approach: add description-based Lua bind metadata support in Hyprline, and document/plan the matching Hyprland Lua config description format
---

# Draft: hyprland-lua-bind-description-labels

## Components (topology ledger)
<!-- Lock the SHAPE before depth. One row per top-level component that can succeed or fail independently. -->
<!-- id | outcome (one line) | status: active|deferred | evidence path -->

| lua-bind-metadata-contract | Workspace binds authored through Hyprland Lua callbacks need an explicit machine-readable description because `j/binds` exposes Lua handlers as `dispatcher="__lua"` with opaque numeric `arg`. | active | `/home/indiw33d/.config/hypr/hyprland.lua:277-286`; `https://raw.githubusercontent.com/hyprwm/Hyprland/main/src/config/lua/bindings/LuaBindingsToplevel.cpp`; `https://github.com/hyprwm/Hyprland/discussions/14667` |
| hyprline-description-parser | Hyprline should parse registered `j/binds` rows with `has_description=true` and a strict workspace-label description token, while preserving the existing normal `dispatcher="workspace"` parser. | active | `hyprline-bar/src/infrastructure/hyprland_ipc.rs:96-147`; `https://wiki.hypr.land/Configuring/Basics/Binds/` |
| local-config-authoring | The current local Lua config should add descriptions to the workspace focus binds if the user wants runtime detection without Lua source parsing. | active | `/home/indiw33d/.config/hypr/hyprland.lua:277-286` |
| docs-and-qa | README and tests need to explain Lua callback binds and verify normal dispatcher, described Lua, and fallback behavior. | active | `README.md` workspace keybinding sections; `hyprline-bar/src/infrastructure/hyprland_ipc.rs:231-263` |

## Open assumptions (announced defaults)
<!-- Record any default you adopt instead of asking, so the user can veto it at the gate. -->
<!-- assumption | adopted default | rationale | reversible? -->

| Description format | Use a strict, machine-readable prefix such as `hyprline:workspace:<id>:<label>` in `hl.bind(..., { description = ... })`. | Hyprland docs/source expose descriptions through `j/binds`; a prefix avoids scraping human prose and keeps labels layout-agnostic. | Yes; format can be changed before implementation. |
| Parser priority | Prefer valid `hyprline:workspace:<id>:<label>` description metadata when present, while preserving existing `dispatcher="workspace"` + numeric `arg` parsing as a fallback for non-described rows. | Explicit metadata is the only source that fixes Lua callback binds; legacy parsing must remain compatible. | Yes. |
| Scope of config edits | Do not edit `/home/indiw33d/.config/hypr/hyprland.lua` in the repo implementation plan; use the user's already-applied description change as live QA evidence. | The required local metadata is already present and visible in Hyprland IPC; implementation should stay in repo code/docs. | Yes. |
| Label source | Use labels from description, not hardcoded QWERTY in Rust. | User explicitly did not want QWERTY as the only source; local Lua loop already has the mapping. | Yes. |

## Findings (cited - path:lines)

- Previous plan `.omo/plans/hyprland-lua-workspace-labels.md:21-44` implemented direct socket `j/binds` parsing for normal `workspace` dispatcher rows and explicitly excluded Lua parsing, `hyprctl`, and D-Bus.
- Execution ledger `.omo/start-work/ledger.jsonl:9` records final QA: Hyprland socket was present and live `j/binds` worked, but the current runtime had 70 binds and 0 `workspace` dispatcher binds; parser tests passed, so the remaining failure is the Lua bind representation, not socket access or Rust build.
- Earlier local Hyprland Lua config bound workspace focus through `hl.bind(capsMod .. " + " .. key, hl.dsp.focus({ workspace = i }))` in a loop over `workspace_keys` (`/home/indiw33d/.config/hypr/hyprland.lua:277-286`); that was the state that produced 0 normal workspace dispatcher rows.
- User updated local Hyprland Lua config: workspace focus binds now pass `description = "hyprline:workspace:" .. i .. ":" .. key` (`/home/indiw33d/.config/hypr/hyprland.lua:283-286`).
- Live direct socket query after the user change found `total_binds=70` and `hyprline_descriptions=20`; every described row is still `dispatcher="__lua"`, but `has_description=true` and descriptions range from `hyprline:workspace:1:q` through `hyprline:workspace:20:semicolon`. This confirms the metadata is visible through Hyprland IPC.
- Hyprland upstream source for `hl.bind` always wraps the passed dispatcher/function in a Lua registry callback: `luaL_ref`, then `kb.handler = "__lua"`, `kb.arg = std::to_string(ref)`, and `kb.displayKey = keys` (`https://raw.githubusercontent.com/hyprwm/Hyprland/main/src/config/lua/bindings/LuaBindingsToplevel.cpp`). Therefore even `hl.bind(..., hl.dsp.focus({ workspace = i }))` is expected to appear as `dispatcher="__lua"` in `j/binds`.
- Hyprland upstream registers the `__lua` dispatcher to call the stored Lua registry reference (`https://raw.githubusercontent.com/hyprwm/Hyprland/main/src/config/lua/bindings/LuaBindingsRegistration.cpp`). The numeric `arg` is an implementation reference, not workspace metadata.
- Librarian verification `bg_e9f64593` confirmed the same result with exact upstream permalinks: `hl.bind` stores Lua binds as `handler="__lua"` and `arg=<registry ref>` in Hyprland `d486a5fe` (`https://github.com/hyprwm/Hyprland/blob/d486a5fe69c9800e75aed76ae4beb3183ac8886d/src/config/lua/bindings/LuaBindingsToplevel.cpp#L125-L198`), and `__lua` dispatches that registry callback (`https://github.com/hyprwm/Hyprland/blob/d486a5fe69c9800e75aed76ae4beb3183ac8886d/src/config/lua/bindings/LuaBindingsRegistration.cpp#L54-L83`).
- Librarian verification `bg_e9f64593` found the exact `j/binds`/`hyprctl binds -j` JSON fields, including `dispatcher`, `arg`, `description`, and `has_description`, in Hyprland source (`https://github.com/hyprwm/Hyprland/blob/d486a5fe69c9800e75aed76ae4beb3183ac8886d/src/debug/HyprCtl.cpp#L1012-L1067`).
- Librarian verification `bg_e9f64593` found the official wiki source for `hl.bind(keys, dispatcher[, opts])` and bind flags, including `description` (`https://github.com/hyprwm/hyprland-wiki/blob/2fba83ea59cf4ccc359ce1aa93ddee9869cb82d6/content/Configuring/Basics/Binds.md#L12-L93`), plus workspace dispatcher examples (`https://github.com/hyprwm/hyprland-wiki/blob/2fba83ea59cf4ccc359ce1aa93ddee9869cb82d6/content/Configuring/Basics/Dispatchers.md#L70-L142`).
- Hyprland discussion #14667 directly matches this symptom: after migrating to Lua, `hyprctl binds`/`j/binds` shows `dispatcher="__lua"` and numeric `arg`; maintainer guidance says using descriptions is the intended way to make binds human/machine readable, and printing Lua source/effective dispatcher is not planned (`https://github.com/hyprwm/Hyprland/discussions/14667`).
- Hyprland current Binds docs state `hl.bind(keys, dispatcher)` is the Lua form; `hl.bind()` supports a `description` option; descriptions are visible through `hyprctl binds`/the bind list (`https://wiki.hypr.land/Configuring/Basics/Binds/`).
- Hyprline current parser only considers rows with `dispatcher == "workspace"`, plain positive integer `arg`, non-mouse, default submap, then uses `key.to_uppercase()` (`hyprline-bar/src/infrastructure/hyprland_ipc.rs:96-147`). This cannot recover `workspace_id` from `__lua` rows without additional metadata.
- Hyprline UI fallback remains numeric by design (`hyprline-bar/src/ui/workspaces.rs:72` from prior grep), so adding metadata parsing can fail safe.

## Decisions (with rationale)

- Do not try to make Lua `hl.dsp.focus({ workspace = i })` appear as a normal `workspace` dispatcher row; upstream source and discussion show `hl.bind` registers Lua-backed rows as `__lua` by design.
- Add a second parser path for described Lua binds. Recommended exact metadata: `description = "hyprline:workspace:" .. i .. ":" .. key`. Hyprline parses only this strict prefix, extracts the workspace id and label, uppercases the label, and ignores malformed descriptions.
- Preserve the existing normal-dispatcher parser because it remains correct for non-Lua/older config rows and already has tests.
- Keep numeric fallback; if the local config omits descriptions, Hyprline should still display numbers and not error.
- Update README to state that Lua callback binds require the optional `hyprline:workspace:<id>:<label>` bind description for labels, because `j/binds` cannot expose the callback's effective workspace dispatcher.

## Scope IN

- Plan a follow-up Rust parser/test/doc change in the repo.
- Treat the already-applied local Hyprland Lua metadata change at `/home/indiw33d/.config/hypr/hyprland.lua:283-286` as runtime QA evidence; do not plan further edits to that external config file.
- Plan validation with `cargo test -p hyprline-bar workspace_bind_labels -- --nocapture`, full `cargo test -p hyprline-bar`, full `cargo build -p hyprline-bar`, and live `j/binds` evidence if Hyprland socket is available.

## Scope OUT (Must NOT have)

- Must not parse arbitrary Lua source.
- Must not spawn `hyprctl` for Hyprline runtime behavior.
- Must not add D-Bus bind discovery.
- Must not hardcode QWERTY in Rust as the only source.
- Must not remove the existing `dispatcher="workspace"` parser or numeric fallback.
- Must not assume `__lua` numeric `arg` is stable workspace metadata.
- Must not stage/commit or overwrite unrelated dirty worktree files.

## Open questions

- None blocking. I recommend the strict `hyprline:workspace:<id>:<label>` description format; skipped/vague approval means use this default.

## Approval gate
status: awaiting-approval
<!-- When exploration is exhausted and unknowns are answered, set status: awaiting-approval. -->
<!-- That durable record is the loop guard: on a later turn read it and resume at the gate instead of re-running exploration. -->

Pending action: fill `.omo/plans/hyprland-lua-bind-description-labels.md` with a decision-complete follow-up implementation plan.

Brief: Hyprland Lua `hl.bind` rows are intentionally exposed as `dispatcher="__lua"` with opaque numeric `arg`; no socket-only parser can infer `workspace=i` from that. The documented/upstream-supported metadata channel is bind `description`. Plan will add strict description parsing to Hyprline and require local Lua workspace binds to include descriptions like `hyprline:workspace:1:q`.

User approval: User accepted Rust-side mapping for `semicolon -> ;`, required letters uppercase, and asked to implement. Per planner rules this approves writing the work plan, not implementation.

Metis gap analysis: `ses_116c614d3ffegFGXKdD0j0gd2z` recommended preserving legacy parser, adding defaulted description fields, strict parsing of `hyprline:workspace:<positive-int>:<key>`, normalizing labels, covering malformed descriptions, duplicates, semicolon, uppercase, and avoiding local config edits/live Hyprland test dependency.

High-accuracy review round 1:
- Momus `bg_0664145c` returned OKAY.
- Independent Oracle replacement for unavailable Codex CLI `bg_881c2d21` returned REJECT. Required fixes: dirty-worktree baseline status/diffs, sha256 proof local `hyprland.lua` is not edited, addition-only forbidden-runtime grep, extra parser edge cases, parser-priority draft contradiction, stronger README EN/RU checks, and rename/clarify real manual QA.
- Applied fixes to `.omo/plans/hyprland-lua-bind-description-labels.md` and this draft: description metadata is the preferred source while legacy parser remains fallback; added dirty-worktree and local-config hash protocol; strengthened forbidden-path grep; added missing edge-case acceptance tests; replaced README grep-only QA with section-aware Python checks; renamed F3 to live metadata QA + parser fixture proof.

High-accuracy review round 2:
- Momus `bg_dd2b4c05` returned REJECT only for pipefail-masked QA pipelines.
- Independent Oracle replacement `bg_232bdbb9` returned REJECT for stronger dirty-worktree proof, README subsection-only QA, and Todo 1/Todo 2 test/implementation exception clarity.
- Applied fixes: all tee pipelines now use `set -o pipefail`; baseline/final unrelated tracked diffs and untracked SHA manifests are required and compared; README checks extract only the EN/RU workspace-keybinding subsections; Todo 1 is explicitly marked as a TDD red-test exception to the implementation+test rule; baseline shell block uses `set -e`.

High-accuracy review final round:
- Momus `bg_6b988077` returned OKAY.
- Independent Oracle replacement for unavailable Codex CLI `bg_adcb6dce` returned APPROVE. It noted one stale non-blocking draft phrase, but confirmed the plan file is decision-complete and executable.
