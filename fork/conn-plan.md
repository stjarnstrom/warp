# Conn — orientation report and first slice

## Context

You run 3–4 Claude Code sessions at once across different Warp panes and repos.
The cost is re-entry: coming back to a pane, you can't see what you asked for,
what the agent decided, or whether it is still on task. Scrollback is a log for
the agent, not a briefing for the person who left.

Conn is a re-entry layer: a docked panel showing the focused pane's prompt
spine, decision log, files touched and last response, plus a cross-window
overview grouped by project. Strictly observational.

**This is a hard fork.** We will not contribute upstream and will not rebase.
The fork exists to credit Warp, not to track it. We will delete large parts of
the codebase and change whatever we like. Nothing in this plan should be shaped
by what upstream does after 2026-09-20.

Licensing: `warpui_core` and `warpui` are MIT; the rest is AGPL-3.0-only,
(c) Denver Technologies. Internal use within one legal entity is not
distribution, so AGPL obligations don't trigger. They do trigger on conveying a
binary outside the entity. Keeping this fork public makes the question moot.

This session was orientation and build setup only. No feature code was written.

---

## 1. Build notes

**`cargo build --bin warp` succeeds, and the setup is now permanent.** Three
blockers, none requiring a source change.

1. **`protoc` missing** — `warp_multi_agent_api`'s build script failed.
   `brew install protobuf` (36.2). Listed at `script/macos/bootstrap:79`.
2. **`metal` not found** — `crates/warpui/build.rs:113` compiles Metal shaders
   via `xcrun metal`. The active developer directory is
   `/Library/Developer/CommandLineTools`, which has never shipped the Metal
   toolchain. Not a beta problem: CLT simply doesn't contain `metal`.
3. **Metal Toolchain component not installed** — recent Xcode ships it as a
   separate download. `xcodebuild -downloadComponent MetalToolchain`, 839 MB,
   no sudo.

**The fix, already applied:** `../.cargo/config.toml (one level above the repo)`

```toml
[env]
DEVELOPER_DIR = "/Applications/Xcode-beta.app/Contents/Developer"
```

Deliberately one level **above** the repo. `warp/.cargo/config.toml` is tracked
upstream and Warp still edits it, so a local line there would conflict on every
rebase. Cargo merges `.cargo/config.toml` walking up from the invocation
directory, so this covers `warp/` with zero upstream diff and without affecting
Rust projects elsewhere on the machine.

Verified by touching `crates/warpui/build.rs` and rebuilding with
`env -u DEVELOPER_DIR` — the Metal shaders compiled, so the value came from the
config.

Rejected: `sudo xcode-select --switch`. It sets the active developer directory
machine-wide, and this Mac's CLT clang (2100.3.34.2) is *newer* than
Xcode-beta's (2100.3.30.1), so switching would slightly downgrade the default C
compiler for every other native build to fix a problem local to this repo.

Covers `./script/run` too: that path uses `codesign`, `security` and `plutil`,
which are real `/usr/bin` binaries rather than xcrun shims. Only
`script/macos/bundle` needs xcrun (`notarytool`), and only for release builds.

- `rust-toolchain.toml` pins `1.92.0`; rustup auto-installed it. No action.
- Environment: macOS 27.0 (26A428), Xcode-beta 27.0 (27A5237l) — the only Xcode
  installed and the correct match for this OS — CLT 27.0.0.
- `target/debug/warp` is 780 MB; full `target/` is **12 GB**. Budget the disk.
- Update the `DEVELOPER_DIR` path if Xcode-beta.app is ever replaced by a
  released Xcode.app.
- Not yet verified: that the binary launches. `./script/run` is the next check.

---

## 2. What already exists

**Warp already ships most of the mechanism described in the brief.**

### F1 — Warp already tracks Claude Code sessions per pane

`app/src/terminal/cli_agent_sessions/` (~2600 LOC) is the ingest half of Conn,
already built and shipping.

- `CLIAgentSession` (`cli_agent_sessions/mod.rs:135`) — per-pane session state.
- `CLIAgentSessionStatus` (`mod.rs:23`) — `InProgress | Success | Failed |
  Blocked | Cancelled`. That is the overview's status column, done.
- `CLIAgentSessionContext` (`mod.rs:58`) — `cwd, project, session_id,
  tool_name, tool_input_preview, summary, query, response`.
- Keyed by `terminal_view_id: EntityId` — already bound to a pane.
- `CLIAgentSessionsModelEvent` (`mod.rs:273`) is a public event enum, each
  variant carrying `terminal_view_id`. **Conn's subscription point.**

### F2 — The transport is OSC 777 on the PTY, not HTTP

Warp installs the `warpdotdev/claude-code-warp` marketplace plugin into Claude
Code (`cli_agent_sessions/plugin_manager/claude.rs`) and listens for
`OSC 777 ; notify ; warp://cli-agent ; <json>` on the PTY.

Plugin v2.1.0 (already installed here at
`~/.claude/plugins/marketplaces/claude-code-warp/plugins/warp/`) registers
`type: "command"` hooks on `SessionStart`, `UserPromptSubmit`, `PostToolUse`,
`PermissionRequest`, `Notification(idle_prompt)`, `Stop`.

Wire schema `CLIAgentNotification`
(`crates/warp_core/src/cli_agent_protocol.rs`), parsed into `CLIAgentEvent` /
`CLIAgentEventType` (`cli_agent_sessions/event/{mod,v1}.rs`). Event types
already understood: `session_start, prompt_submit, tool_complete, stop,
stop_failure, permission_request, permission_replied, question_asked,
idle_prompt`.

**Because it rides the PTY, pane binding is free, exact, and cannot block a
session.** No pane ID, no env var, no header interpolation, no port.

### F3 — Pane identity and jump-to-pane already exist, already in the shell env

`crates/warp_terminal/src/focus_env.rs` injects into every GUI shell:

- `WARP_TERMINAL_SESSION_UUID` — stable across app restarts
  (`app/src/pane_group/pane/terminal_pane.rs:188`)
- `WARP_FOCUS_URL` — `warp://session/<uuid_hex>`; `open "$WARP_FOCUS_URL"`
  focuses that pane today, raising the OS window (`app/src/uri/mod.rs:539`).

The env-var trick from the brief is already implemented upstream, with a better
identifier than a pane ID.

In-process jump: `RootView::focus_pane(&PaneViewLocator, ctx)`
(`app/src/root_view.rs:2983`) → `show_window_and_focus_app` +
`Workspace::focus_pane` (`app/src/workspace/view.rs:6014`) →
`PaneGroup::reveal_and_focus_pane` (`app/src/pane_group/mod.rs:5004`).
Resolve by UUID with `PaneGroup::find_terminal_pane_by_session_uuid`
(`mod.rs:2294`); enumerate windows via `WorkspaceRegistry::all_workspaces`
(`app/src/workspace/registry.rs:48`).

### F4 — Persistence, panels and a local HTTP server all exist

- **SQLite via Diesel.** Schema/migrations in `crates/persistence/`
  (141 migrations); runtime in `app/src/persistence/sqlite.rs` (3246 lines)
  with a dedicated `"SQLite Writer"` thread fed by a bounded channel
  (`sqlite.rs:552`). Adding a table is a migration, documented in
  `app/src/persistence/README.md`.
- **Docked panels.** `PanelPosition::{Left,Right}` (`workspace/view.rs:814`),
  laid out by `render_panels` (`view.rs:22848`) and
  `render_banner_and_active_tab` (`view.rs:22213`), dispatched by
  `render_config_panel` (`view.rs:22996`). Open/closed state is per pane group
  (`pane_group/mod.rs:938`), persisted in the `panels` table
  (`migrations/2025-11-07-005740_create_panels_table`) and
  `app_state.rs:331`. Widths via `ModalType` in `terminal/resizable_data.rs`.
- **Local HTTP server.** `crates/http_server/src/lib.rs` — axum on
  `127.0.0.1:9277+channel_offset`, built at `app/src/lib.rs:2604` from a plain
  `vec![]` of routers. Not needed for Conn, but there if wanted.
- **Block IDs are stable and persisted.** `BlockId` is a string
  (`{WARP_SESSION_ID}-{N}`, or `manual-<uuid>`), stored as `blocks.block_id`.
  Lookup: `BlockList::block_index_for_id` (`terminal/model/blocks.rs:2038`).
  Scroll-and-highlight primitive already exists:
  `TerminalView::jump_to_bookmark` (`terminal/view.rs:23654`).

### F5 — What Warp does *not* keep, and what Conn adds

`CLIAgentSessionContext` holds only the **latest** query / response / tool. It
is a status light, not a history:

- no ordered list of prompts (the spine)
- no accumulated decision log — `clear_permission_scoped_state()`
  (`mod.rs:186`) deliberately *erases* permission state after each reply
- no files-touched set — `tool_input_preview` keeps one `command` or
  `file_path` at a time (`event/v1.rs:31`)
- no persistence across restarts
- `apply_event` (`mod.rs:198`) discards events it doesn't need (`IdlePrompt`,
  non-blocked `ToolComplete`, `Unknown`), so subscribing to `StatusChanged`
  alone loses fidelity — Conn must tap the raw event

**Conn = an append-only event log keyed by pane, plus a panel that renders it.**

---

## 3. Corrections to the hook claims

Verified against `https://code.claude.com/docs/en/hooks`, Claude Code 2.1.278.

**C1 — "Hooks are async and must never block" is wrong.**
> "By default, hooks block Claude's execution until they complete."

`async: true` is **only available on `type: "command"` hooks**. An HTTP hook is
awaited. Default timeout 600s, 30s on `UserPromptSubmit`, where the docs are
explicit: *"a stuck hook stalls the session."*

What *is* safe: **connection failure is a non-blocking error and execution
continues** — a listener that is down costs nothing. A listener that is *hung*
stalls every session up to the timeout.

This is moot given the chosen ingest path (F2 rides the PTY), but it kills the
"never blocks" premise for any future HTTP hooks.

**C2 — The env-var header trick is real, and env inheritance is documented.**
> "A hook process inherits the parent environment, apart from the `OTEL_*`
> exporter variables..."

Unlisted `$VAR` references resolve to empty strings.

**C3 — Your event list is correct but incomplete.** All 14 named events exist;
the doc lists 33. Relevant omissions: `PermissionDenied`, `PostToolBatch`,
`SubagentStart`, `PostCompact`, `FileChanged`, `CwdChanged`, `SessionEnd`.

**C4 — `Stop` does carry `last_assistant_message`.** Also `stop_hook_active`,
`background_tasks[]` and `session_crons[]` — the last two distinguish "done"
from "paused waiting on background work", a status the overview would otherwise
get wrong.

**C5 — The transcript-lag claim is correct, verbatim in the docs.**
> "The transcript file is written asynchronously and may lag the in-memory
> conversation... use `last_assistant_message` on Stop and SubagentStop instead
> of reading the transcript."

**C6 — `Notification(permission_prompt)` is the wrong blocked-on-me signal.**
It waits ~6s and each keystroke defers it. `PermissionRequest` fires
immediately. Warp's plugin already uses `PermissionRequest`.

**C7 — Every payload carries** `session_id`, `transcript_path`, `cwd`,
`permission_mode`, `hook_event_name`, plus `prompt_id` (correlates every event
in one turn — the natural grouping key for the decision log) and
`scratchpad_dir`.

---

## 4. In-process, not a daemon

**In-process, in Warp.**

- The state's only consumer is a panel rendered by Warp, keyed to a pane that
  only exists while Warp runs. A daemon would serialize state out and
  immediately back in to reach its only reader.
- The event source is the PTY. A daemon can't see OSC 777 without Warp
  forwarding it, so it adds a hop rather than independence.
- Warp already has both things a daemon would provide: a loopback HTTP server
  (F4) and a Diesel/SQLite store with a dedicated writer thread. Durability is
  a table, not a process.
- Portability is served by keeping the event log a plain serializable type in
  its own crate (T1). If a daemon is ever wanted, you move the store, not the
  model.

The one thing a daemon buys — surviving a Warp crash — is bought more cheaply
by writing each event to SQLite as it arrives.

---

## 5. Where we'd touch the code

Six places. Five are new files.

| # | File | Change |
|---|---|---|
| T1 | `crates/conn/` (new) | `ConnEvent` log types + store. serde only, no Warp deps — keeps the model portable. |
| T2 | `app/src/conn/mod.rs` (new) | `ConnModel`, a `SingletonEntity` subscribing to `CLIAgentSessionsModel`, appending to a log keyed by pane. |
| T3 | `app/src/terminal/cli_agent_sessions/mod.rs` | **The only upstream edit.** Add `CLIAgentSessionsModelEvent::RawEvent { terminal_view_id, event }` and emit it at the top of `update_from_event` (~`mod.rs:473`), *before* `apply_event` discards anything. ~6 lines. |
| T4 | `app/src/conn/panel.rs` (new) | Docked panel. Copy `app/src/workspace/view/left_panel.rs` for structure (`Resizable`, `ResizableStateHandle`, `PanelPosition`, header + close). Copy the dynamic-target pattern from `right_panel.rs:118` (`RightPanelReviewActionTargetProvider`) so it follows the focused pane rather than capturing a handle at construction. |
| T5 | `app/src/conn/overview.rs` (new) | Cross-window list. Enumerate via `WorkspaceRegistry::all_workspaces`; jump via `RootView::focus_pane`. |
| T6 | `crates/warp_features/src/lib.rs` + `app/src/features.rs` | One `FeatureFlag::Conn` variant, dogfood-only. |

Registration for T4 also needs small additions in `workspace/view.rs:22996`
(`render_config_panel` match arm), `pane_group/mod.rs:938` (open/closed bool)
and `terminal/resizable_data.rs` (a `ModalType` + default width). These follow
the existing left/right panel shape exactly.

**Keep T3 small anyway.** It was originally scoped as a 6-line emit to minimise
the upstream diff, and that reason is gone. But the separation is right on its
own merits: `CLIAgentSession` is a *status* model that deliberately discards
history (`clear_permission_scoped_state`, `apply_event` dropping events), while
Conn needs a *history* model. Merging them would couple Conn to Warp's status
logic and to the tab-title and agent-footer code that depends on it. The tap
stays a clean seam, not a rebase hack.

What the fork does change: if Conn later needs richer events than the plugin
emits (R2), vendor or replace `claude-code-warp` rather than working around
`MINIMUM_PLUGIN_VERSION`.

---

## 6. Harder than the brief assumes

**R1 — Block anchoring won't give you what you want.** The machinery is there
(`BlockId` is stable and persisted; `jump_to_bookmark` scrolls and highlights),
but the *granularity* is wrong. Block IDs are minted by the shell's precmd, so
they exist per shell command. A long `claude` run is **one block**. You cannot
scroll to a decision made inside a running session, because there is no block
boundary there. Out of scope for slice 1; revisit only if you later add a
scrollback-offset anchor.

**R2 CONFIRMED IN USE (2026-09-20).** First real session showed eleven
identical `ran / Bash` rows burying the prompt and response. Cause measured in
the installed plugin scripts, not guessed:

| event | script | what it sends |
|---|---|---|
| `tool_complete` | `on-post-tool-use.sh` | `tool_name` **only** — no `tool_input`, so no command or path, ever |
| `prompt_submit` | `on-prompt-submit.sh` | `prompt` **truncated to 200 chars** (`${QUERY:0:197}...`) |
| `permission_request` | `on-permission-request.sh` | rich: full `tool_input`, `tool_name`, and a `"Wants to run X: <cmd>"` summary |

So the *spine* — the most valuable column — is capped by someone else's shell
script, and tool activity is reduced to one word. Mitigated in the panel by
coalescing consecutive same-tool calls into `ran Bash ×11` at record time (which
also stops redundant rows consuming the `MAX_ENTRIES` budget) and by dropping
`SessionStart`, which told a returning reader nothing and pushed the first real
prompt down the panel.

**RESOLVED by B1/B2, and not the way this said.** Owning the hooks is
unnecessary: `on-stop.sh` sends `transcript_path`, and the transcript holds the
untruncated prompt, the full tool input, and a one-line `description` the model
wrote for each call. Reading a file beats installing hooks into the user's
Claude Code config, and it needs no cooperation from the plugin. The panel's
coalescing stays for the in-flight turn, which is still hook-only.

**R2 (original) — The plugin decides what you can see.** Files touched, full tool inputs,
`PreCompact`, `TaskCreated/Completed` and `SessionEnd` are not emitted by
`claude-code-warp` v2.1.0. `tool_input_preview` keeps one `command` or
`file_path` at a time. To get more, add your own `hooks.json` entries alongside
Warp's rather than forking the plugin — hooks compose, and Warp enforces
`MINIMUM_PLUGIN_VERSION` (`plugin_manager/claude.rs:24`) and will auto-update
the plugin under you.

**R3 — "What changed since I last looked" needs a read-watermark, and the panel
being open is not the same as you looking at it.** Needs a deliberate rule —
pane focus? panel scrolled to bottom? Decide it before building slice 2 or the
feature will feel wrong.

**R4 — Horizontal space.** Conn competes with Warp's existing left and right
panels and with the panes themselves. `MIN_SIDEBAR_WIDTH = 250`,
`MAX_SIDEBAR_WIDTH_RATIO = 0.75`. On a three-way split this will be tight.

**R5 — `EntityId` is not restart-stable.** `CLIAgentSessionsModel` keys by
`terminal_view_id: EntityId`, a process-lifetime counter. Key the persistent
store by `TerminalPane::session_uuid()` / `WARP_TERMINAL_SESSION_UUID` instead,
and map to `EntityId` at runtime.

**R7 — Conn leaks history for closed panes (introduced by A3).** A session's
history deliberately outlives the session — reading it afterwards is the point —
but nothing discards it when the *pane* is closed. Per-session memory is bounded
by `MAX_ENTRIES`; pane count is not. `forget_pane` was written and then removed
rather than left unwired, so the gap is visible instead of looking handled. Close
it when the pane-lifecycle wiring lands, and note that `CLIAgentSessionsModelEvent::Ended`
is the wrong trigger: it fires when the session ends, not the pane.

**R6 — Focus-following has an edge case.** When the focused pane has no Claude
session, the panel must show something deliberate (last agent pane? empty
state?) rather than flickering. Pick one before building T4.

---

## 7. First slice

**Prove the spine end to end. No persistence, no styling, no overview.**

1. **A0 — DONE.** Build, bundle and launch all work. See section 1.
2. **A1 — DROPPED, by decision.** Conn is the reason this fork exists, so it
   must not be switchable off. `FeatureFlag::Conn` was added and then removed;
   the tap is unconditional.
   Two things this costs, both handled:
   - The planned "toggle Conn off mid-session" A/B is gone. Non-interference is
     structural instead, which is stronger: `ConnModel` holds no writable handle
     to a session, terminal or PTY, and runs downstream of the terminal parser,
     so the agent's hook has already returned by the time an event arrives.
   - The flag was implicitly bounding log growth. Retention is now explicit
     (`conn::MAX_ENTRIES`, see A3).
   *Worth keeping if a flag is ever needed here:* `DOGFOOD_FLAGS` would not have
   worked. `app/src/bin/oss.rs` — the binary `./script/run` builds — applies
   only `DEBUG_FLAGS`.
3. **A2 — DONE.** `CLIAgentSessionsModelEvent::RawEvent { terminal_view_id,
   event: Box<CLIAgentEvent> }`, emitted unconditionally in `update_from_event`
   after the session-exists guard and before `apply_event`. Boxed to match the
   existing `StatusChanged` convention.
   Two tests in `cli_agent_sessions/mod_tests.rs`: fires for `IdlePrompt` and
   non-blocked `ToolComplete` (the events `apply_event` drops, i.e. the whole
   reason the tap exists), and silent for an untracked terminal.
   Adding the variant broke three exhaustive matches — `agent_management_model.rs`,
   `local_agent_task_sync_model.rs`, `agent_sdk/driver.rs`. All three are
   status/lifecycle consumers with no use for raw events, so each got an
   explicit ignore arm rather than a wildcard.
   Verified: `cargo test -p warp --lib cli_agent_sessions` 171 passed / 0
   failed; `cargo clippy -p warp --lib` clean; `./script/format --check` clean.
4. **A3 — DONE.** `crates/conn` (chrono + serde only, no Warp deps, so the
   history model stays portable) holds `ConnEntry`, `ConnEntryKind` and
   `ConnSession`. `app/src/conn/mod.rs` holds `ConnModel`, a singleton that
   subscribes to `CLIAgentSessionsModel` and accumulates history per pane.
   Registered in `lib.rs` immediately after `CLIAgentSessionsModel`, which it
   subscribes to on construction — that ordering is invisible and load-free to
   break, so it carries a comment.
   `ConnEntryKind` is written in the reader's vocabulary (`Prompt`,
   `PermissionRequested`, `PermissionResolved`, `QuestionAsked`,
   `ToolCompleted`, `Responded`, `Failed`), not the hook's.
   Retention: one chronological `Vec` capped at `MAX_ENTRIES` (500), evicting
   the oldest *evictable* entry. `Prompt` is never evictable — it is the reason
   Conn exists, it is bounded by typing speed, and losing the earliest one loses
   the session's intent. `dropped()` counts evictions so the panel can say how
   much history is missing rather than quietly showing a partial log.
   `IdlePrompt` and `Unknown` map to no entry; blank prompts are skipped.
   Naming note: crate `conn` and module `crate::conn` collide, so app-side code
   writes `use ::conn::...`, matching the existing `local_control` precedent.
   Verified: 5 crate tests, 5 model tests (driven through the real
   `CLIAgentSessionsModel` so the subscription wiring is covered, not assumed),
   `cli_agent_sessions` 170 passed, clippy and format clean.
5. **A4 — DONE.** `app/src/conn/panel.rs` holds `ConnPanelView`, docked in the
   right-hand panel slot (decision: Conn replaces code review there).
   - Follows the focused pane via `PaneGroup::focused_session_view`, keyed on
     the `TerminalView` id, which is the same key `CLIAgentSessionsModel` uses
     (`terminal/view.rs:4447`, `view_id: ctx.view_id()`).
   - Holds the pane group **weakly**. A read-only panel must not be what keeps
     a closed tab's pane group, and through it its terminal models, alive.
   - `set_active_pane_group` is wired at both sites the right panel uses —
     workspace restore and tab activation. Wiring only one leaves the panel on
     a stale pane.
   - `ListState` owns its item count and has no `reset`, so a pane change
     rebuilds it (as `agent_management::construct_fresh_list_state` does) and
     new entries extend it and scroll to the tail.
   - Registered with `ctx.add_view`, not `add_typed_action_view`: the panel has
     no actions of its own, and an empty `TypedActionView` impl would be
     pretense.
   - `ConnPanelView` reads `ConnModel` on construction, so any test harness
     building a `Workspace` must register it — added to `workspace/view_tests.rs`
     and `test_util/terminal.rs`. There is no fallible singleton accessor.
   - Toolbar: `HeaderToolbarItemKind::CodeReview` renamed to `Conn`, label
     "Conn", `Icon::ClockRewind`, no longer gated on `local_fs` or on the old
     `show_code_review_button` setting. **The variant is persisted in
     `settings.toml`**, so it carries `#[serde(alias = "code_review")]` —
     without it an existing config silently drops the toolbar button.
   - `RightPanelView` is still constructed and still receives code-review
     actions, just never rendered. Replacing it outright means deleting
     code_review (23k LOC, §9), which belongs after Conn works.
6. **A5** — Manual check with two panes (below).

That is the product's core loop, with no HTTP listener, no env var, no hook
configuration, and one upstream function touched.

### Slice B — the Conn tells the story

A flat list of `ran Bash ×11` rows is a log, not a briefing. What a returning
reader needs is an account: what I asked for, what it did, what came of it. The
transcript already contains that account in the model's own words —
`input.description` on every `Bash` and `Agent` call is the model narrating its
work as it goes — so Conn reads rather than summarises. No summarisation model
is needed for the structure.

1. **B1 — DONE.** `crates/conn/src/story.rs`: `parse_transcript(&str) ->
   ConnStory`, a pure function over the JSONL, plus `ConnTurn` / `ConnStep`.
   Segmentation rules, each one measured against a real 5.5 MB transcript
   rather than guessed:
   - Assistant records carry **no `promptId`**, so turns cannot be grouped by
     it. Segmentation is chronological, starting a turn on
     `type == "user" && promptSource == "typed"`.
   - Background task notifications arrive as **user** records
     (`promptSource: "system"`, `origin.kind: "task-notification"`). Without
     that filter one turn splits into several and text the user never typed
     lands in the spine.
   - `isSidechain` (subagents) and `isMeta` (harness injections) are excluded;
     otherwise subagent work interleaves into the parent's steps.
   - Records are sorted by timestamp before assembly.
   - Parsing is lenient: the file is appended to live, so a read can land
     mid-line. An unparseable line is skipped rather than failing the read,
     because blanking the panel is the worst possible failure here.
   - The session title is Claude Code's own `aiTitle`, last one written.
   15 turns from the real transcript in 38 ms — which is also why it cannot run
   on the main thread.
2. **B2 — DONE.** The read is wired into `ConnModel`, off the main thread.
   - `read_story` is an `async fn` doing `tokio::fs::read_to_string` then
     `parse_transcript`, dispatched with `ctx.spawn`, which runs the future on
     the background executor and resolves the callback on the main thread.
   - **Only `Stop` carries `transcript_path`** — verified in the installed
     plugin, where `on-stop.sh` is the single script that passes it. So the
     path is remembered on the session when it arrives and the read fires on
     turn completion, which is also the only moment the transcript has a
     finished chapter to tell.
   - The whole file is re-read each time. Wasteful on a long session, but it
     keeps the read stateless; incremental reads are an optimisation for when
     it shows.
   - **Merging live state with the story.** `ConnSession` gains `story` and
     `story_through`. `story_through` is the arrival time of the stop event
     that triggered the read — our own clock, so it orders reliably — and
     `in_flight()` returns the live entries after it. Without this the turn's
     response appears twice, once as the story's outcome and once as a live
     `Responded` row, on every single turn.
   - **Stale reads.** Reads are dispatched in order and resolve off-thread, so
     `adopt_story` rejects one whose boundary is not newer than the current
     story rather than rewinding the panel. A generation counter would do the
     same job; the boundary was already needed.
   - **Transcript lag is handled, not just documented.** If the read lands
     before the turn's closing message reaches the file, the story's last turn
     has no `outcome`, and `story_through` is pulled back to that turn's last
     activity so the live entry carrying the response stays visible. `ConnTurn`
     gained `ended_at` / `last_activity()` for this.
   - A transcript that cannot be read is not an error: the hook-derived history
     stays as it was.
   - `reads_in_flight` holds the `FutureId` per pane so tests can await the
     read before asserting, following the `SyncQueue::spawned_futures`
     precedent.
3. **B3 — DONE.** `ConnPanelView` renders chapters: a dated divider, the
   instruction untruncated, the steps unlabelled and indented, the closing
   message. The header carries the project and the session's `aiTitle`.
   - **Rows are addressed into the model, not cached.** A cached row list keeps
     showing one pane's story after focus moves to another, because focus can
     move between panes of a group without emitting anything a panel can
     subscribe to — `PaneGroup::Event` has no focus variant and there is no
     `observe_view`. Walking the story to find a row is linear and costs
     nothing, because `ListState` only asks for the rows in view.
   - `ListState` still owns its item count, so reconciliation moved into
     `render` behind a `Cell`. `ListState`'s mutators all take `&self`
     (`Rc<RefCell<_>>`), and render is the only place that always sees the
     current focused pane. This also closes a pre-existing bug: before, the row
     count went stale on a same-group pane switch until the next hook event.
   - A change in the turn count rebuilds rather than extends, because row
     heights are cached by index and a re-read story moves rows. A story whose
     text changed with no count change is the one case this misses; it corrects
     on the next event.
   - Steps carry no label. Fifteen repetitions of "did" beside the model's own
     words is noise.
   - 9 tests on row addressing (`panel_tests.rs`), which is the part that
     decides what a returning reader sees; rendering itself needs a window.

Open question, now that chapters exist: a 15-turn session is 15 chapters.
Scrolling back through summaries was chosen over a "since I last looked" marker
(R3), which is right at 15 and unproven at 60.

4. **B4 — DONE. Reverses B3's decision to defer this.** First run on a real
   session showed the panel in hook-only mode for a fifteen-minute turn: a
   wall of `ran Agent` / `ran Bash ×2` / `ran Read ×5` rows. Reading only on
   `stop` was wrong, because a long turn is exactly when you walk away and
   come back. The in-flight turn is the common case, not an edge case.
   - Claude Code writes the transcript *as it works*, descriptions included —
     verified against a live file. So the running turn reads like any other.
   - Paced at one read per 3s, skipped while a read is in flight. `stop`
     always reads: it is the boundary and the last chance at the closing
     message.
   - **The path had to be derived.** The plugin reports `transcript_path` on
     `stop` only, so the first turn of a session — the one most worth reading
     while it runs — had no path at all. Derived from cwd + session id
     (`~/.claude/projects/<cwd with / and . as ->/<session id>.jsonl`,
     honouring `CLAUDE_CONFIG_DIR`), existence-checked, falling back to
     finding the session id under `projects/` when the directory-name guess is
     wrong. The directory name is a guess; the session id is not. A
     plugin-reported path still wins, and the resolved path is remembered.
   - Two leaks into the panel, both fixed at ingest: `<pasted_content>` tags
     sitting where the instruction should be, and the plugin serialising a
     whole `tool_input` into the permission summary for any tool with no
     command or file path (an 80-character JSON blob where the tool's name
     would have said more).

**Slice 2 (deferred):** SQLite persistence keyed by session UUID, the
cross-window overview (T5), the read-watermark (R3), styling.

**No longer needed:** owning our own `conn/hooks.json` to work around the
plugin's 200-character prompt cap and missing tool inputs (R2). The transcript
has both in full, so the fix is reading rather than intercepting, and Conn
installs nothing into the user's Claude Code config.

---

## 8. Verification

### Already verified (2026-09-20, live session)

- Build, bundle, launch: `./script/run` opens the app.
- Runs fully logged out. "Skip for now" does not create a Firebase anonymous
  user, because `skip_firebase_anonymous_user` is in `app/Cargo.toml`'s
  `default` list and `enabled_features()` applies it on every channel.
- `hoa_notifications` and `cli_agent_rich_input` are compiled in, so Warp arms
  the shell and the OSC 777 listener runs. `warp_control_cli` is off, so the
  warpctrl loopback server is not running.
- **End-to-end ingest confirmed:** Claude Code in a Warp pane, prompt sent, tab
  title changed to the prompt text. That exercises the full path — plugin hook
  fired, OSC 777 reached Warp, `parse_event` produced a `CLIAgentEvent`,
  `update_from_event` applied it to the session bound to that pane. F1/F2/F3
  are observed behaviour, not source reading.

### Still to verify (slice 1)

- **Build:** `cargo build --bin warp` exits 0; `./script/run` launches.
- **Unit:** `cargo nextest run -p app cli_agent_sessions` — A2's event fires for
  every parsed `CLIAgentEvent`, including `IdlePrompt` and non-blocked
  `ToolComplete`, which `apply_event` drops.
- **Manual, two panes:** split a tab, `cd` to two different repos, run `claude`
  in each. Send two prompts per session and trigger one permission prompt.
  Confirm the panel shows the focused pane's two prompts in order plus its
  permission entry, that switching focus swaps the contents, and that nothing
  leaks between panes.
- **Non-interference:** toggle `FeatureFlag::Conn` off mid-session and confirm
  both `claude` sessions continue unaffected. Conn reads an existing PTY event
  stream and sends nothing, so this should be trivially true — verify anyway,
  since it is the project's core promise.
- **Lint:** `cargo clippy -p warp --lib --all-targets`, then `./script/format`.

### Use nextest, not `cargo test`

`AGENTS.md` mandates `cargo nextest`, and the reason is not style. Warp's tests
share global state — `FLAG_STATES` is a global array of atomics — and
`cargo test` runs them in one process, so results are unstable run to run.
Measured on 2026-09-20:

| runner | clean HEAD | with Conn |
|---|---|---|
| `cargo test -p warp --lib` | 10 failed | 12 failed (set varied between runs) |
| `cargo nextest run -p warp --lib` | **6800 passed, 0 failed** | **6807 passed, 0 failed** |

Every one of those failures was cross-test pollution, on both trees. Install the
runner with `./script/install_cargo_test_deps` and never attribute a failure
without a same-runner baseline.

---

## 9. Trimming (slice 3, not slice 1)

**The module tables that used to live here are deleted.** They were measured
2026-09-20 and misled twice. LOC goes stale on every commit, and the refs
column counted module-path mentions rather than coupling, so it said
`code_review` was 102 when the real figure is 1,170 across 146 files, and it
filed `ai` as dead weight without noticing that `ai/blocklist` is the AI layer
over the terminal's own block list (`terminal/model/blocks.rs:38` imports
`AIBlock` and `SerializedBlockListItem` from it). No count predicts deletion
cost; where the seam is does, and that means opening the module.

The live order is in memory `warp-deletion-sequence.md`, measured as it goes.
§10 records the method: cut the user-facing entry point, then the producer,
then let `cargo check` and dead-code warnings name the cascade.

### Sequencing

**Do not delete before Conn works.** The working build is currently the only
regression test. Removing `ai` or `server` means a long non-compiling stretch
with no baseline, and it does not make Conn easier to write — it means
debugging two things at once.

Order:

1. Build Conn (sections 5–7) against the tree as it stands.
2. Turn features off before deleting code. `app/Cargo.toml` has a long
   `default = [...]` list, and `FeatureFlag` (`crates/warp_features/src/lib.rs`)
   gates behaviour at runtime. Both are reversible in seconds and prove what
   actually depends on what before surgery that isn't.
3. Delete in the cheap-table order, running the app after each.
4. Take on `ai` / `server` / `auth` last, as a deliberate project, with Conn
   already working so regressions are visible.

### Rebrand (whenever)

`ChannelState::channel()` drives paths, ports and bundle IDs. `Channel::Oss`
already exists and `app/src/bin/oss.rs` builds `warp-oss`, so there is a clean
seam for renaming without inventing one.

---

## 10. Deleting Warp's business model

### Context

The criterion is the user's, from 2026-09-20: cut *"everything that requires a
warp account and leads somewhere to their payments and limits"*, while keeping
*"looking at files, viewing them, seeing uncommitted changes"*. Not module size
— purpose. A module that does real work stays even if it is large; a small one
that exists to sell a plan goes.

Done so far:

- `10428ed45` — `WorkspaceAction::ShowUpgrade` and its two CTAs
- `f994f9010` — the Billing & Usage settings page
- `c4689207d` — the credit and request-limit accounting (74 files, −5,287):
  `AIRequestUsageModel`, the TUI `/usage` panel, the Agent Profiles usage
  widget, the plan badge, bonus grants, the trial-credits banner, the refund
  footer. Every gate it answered (`has_any_ai_remaining`, `can_request_voice`,
  `has_base_plan_requests_remaining`) became the unconditional yes it would
  give with no account.

This slice finishes the payments half. The account half (§11) follows.

### What is left, and where it funnels

Every remaining path to warp.dev checkout runs through **eight URL builders**:
seven in `app/src/workspaces/user_workspaces/mod.rs` (`upgrade_link` :276,
`upgrade_link_for_team` :287, `upgrade_link_for_scope` :296,
`warp_agent_cli_upgrade_link` :312, `admin_billing_link_for_team` :325,
`admin_billing_link_for_default_team` :332, and the `/upgrade` path constant
:62) plus `AuthManager::upgrade_url` (`app/src/auth/auth_manager.rs:878`).
There are no hard-coded pricing URLs left — everything composes from
`ChannelState::server_root_url()`. Deleting those eight is the cut that orphans
all 18 call sites, so a partial pass leaves the funnel standing.

Alongside them, one real server call survives: `WorkspaceClient::
generate_stripe_billing_portal_link` (`app/src/server/server_api/workspace.rs:55`).

### P1 — The upsell call sites

Delete consumers before producers, so each commit compiles.

**Local deletes** — one render branch or match arm, no type churn:

| File | What |
|---|---|
| `terminal/view/ambient_agent/loading_screen.rs:135-210` | `render_tier_limits_footer` — *"Upgrade for more powerful cloud agents"* |
| `settings_view/warp_agent_page.rs:5628-5701` | the whole `if !is_byo_enabled && show_provider_keys` BYOK upsell block |
| `ai/agent_sdk/driver.rs:983-993` | `sandbox_deadline_message` collapses to the single string; the `on_free_plan` arg dies |
| `ai/agent_sdk/ambient.rs:463-466, 613-618` | the CLI *"upgrade your plan: {url}"* stdout line |
| `workspace/view/launch_modal/oz_launch.rs:103-130` | the `LaunchCredits` slide and its `oz_launch_credits.png` |
| `ai/blocklist/prompt/prompt_alert.rs` | `PromptAlertAction` / `PromptAlertEvent::OpenBillingPortal` — already `#[allow(dead_code)]` with no constructor |

**Cascading deletes** — each orphans a type or an enum variant with exhaustive
matches:

1. **`settings_view/teams_page.rs`** — `TeamsPageAction::{GenerateUpgradeLink,
   GenerateStripeBillingPortalLink}`, each needing four edits (variant,
   `blocked_for_anonymous_user` :271/:272, the `LoginGatedFeature` label
   :297/:298, the handler :676/:679). Then the "Upgrade" CTA :2456, the outgrow
   CTA :3483 + `outgrow_upgrade_line_copy` :2520, `GrowTeamWarningCta::Upgrade`
   :417, `render_billing_links` :2944, and the `UserWorkspacesEvent` arms
   :1111/:1119.
2. **`drive/index.rs`** — `DriveIndexAction::{ViewPlans, ManageBilling}`, same
   four-edit shape (:366/:369, :419, :429-430, :5689/:5696). Then the
   shared-object-limit banner copy :193 + "Compare plans" :4235, and
   `render_payment_issue_banner` :4322 with its call site :5248.
3. **`search/command_search/view.rs`** — `render_error_header_with_upgrade_link`
   :633-712 and both call sites :583/:596 collapse to the plain-text sibling at
   :590; delete `CommandSearchAction::{OpenUpgradeLink, AttemptLoginGatedUpgrade}`
   :105-106 and the `upgrade_link: MouseStateHandle` field.
4. **The out-of-credits toast, which exists twice** —
   `workflows/workflow_view.rs:2691-2718` (`display_upgrade_error`) and the
   near-identical `drive/workflows/ai_assist.rs:137-157`. Both collapse to a
   plain error toast; that kills `WorkflowModalEvent::AiAssistUpgradeError` and
   its handler at `workspace/view.rs:11305-11320`.
5. **`terminal/input/models/data_source.rs`** — drop `ModelSearchItem.upgrade_url`
   (:327, :352, :378, :403, :708) and the "Upgrade" hyperlink on
   `DisableReason::RequiresUpgrade`. Keep the disable reason itself; it still
   describes why a model is unavailable.
6. **`terminal/shared_session/share_modal/denied_body.rs:10-81`** — *"upgrade to
   the Build plan"* + "View plans"; cascades through `DeniedBodyAction::Upgrade`
   → `DeniedBodyEvent::Upgrade` → `ShareSessionModalEvent::Upgrade` →
   `pane_group/mod.rs:2752-2760`.
7. **The TUI slash commands** `/upgrade` and `/manage-billing` — six non-test
   files: the `SlashCommandKind` variants (`search/slash_command_menu/
   static_commands/mod.rs:59-60`), the two consts + registry array
   (`static_commands/commands.rs:141-158, 974-975`), the GUI catch-all arm
   (`terminal/input/slash_commands/mod.rs:1335`), the availability filter
   (`slash_commands/data_source/tui.rs:98-129`), and the execution arms
   (`crates/warp_tui/src/terminal_session_view.rs:4620-4644`). Tests:
   `terminal_session_view_tests.rs:313-395`,
   `static_commands/commands_tests.rs:76, 170, 189`.
8. **The TUI credit gate** — `crates/warp_tui/src/agent_block.rs`: consts
   :62-69, `upgrade_url` :72, `render_first_credit_gate` :94,
   `should_consume_first_credit_gate` :83, `sync_first_credit_gate` :585,
   `has_out_of_credits_failure` :1276, and the `TuiAIBlockSection::FirstCreditGate`
   variant :177 with its four exhaustive matches (:1406, :1776, :1943, :1634).
   Also `TuiOnboardingMarker::FirstCreditGate` and
   `FIRST_CREDIT_GATE_STORAGE_KEY` in `app/src/tui_onboarding_markers.rs:23-47,
   220-247` — the `FirstZeroState` variant keeps the marker machinery alive.

**One correction to carry into execution.** `FailedOutputPresentation::
OutOfCredits` (`app/src/ai/blocklist/view_util.rs:67`) is **not** dead after
`c4689207d`. Warp's server still produces it: HTTP 429 with header
`X-Warp-Error-Code: OUT_OF_CREDITS` (`app/src/server/server_api.rs:273-289`).
So **collapse it into `Message` rather than deleting the error path** — the
server's own text keeps being shown, and only the *"subscribe to a Warp plan" /
"Get started with AI"* CTA and the upgrade URL go. That removes the variant,
`should_show_subscribe_cta` (`view_util.rs:189`), and
`render_out_of_credits_error` (`ai/blocklist/block/view_impl/common.rs:3170`).

### P2 — The funnel itself

With no callers left, delete the eight URL builders, `UserWorkspacesEvent::
{GenerateUpgradeLink, GenerateUpgradeLinkRejected, GenerateStripeBillingPortalLink,
GenerateStripeBillingPortalLinkRejected}` (`user_workspaces/mod.rs:73-76`) and
their `on_*` / model methods (:1325-1398), and the server call:

- `WorkspaceClient::generate_stripe_billing_portal_link` — trait decl
  `workspace.rs:55`, impl `:89-107`. `MockWorkspaceClient` is `automock`-generated
  and nothing sets an expectation on it, so **no test double needs editing**.
- `crates/graphql/src/api/mutations/stripe_billing_portal.rs` — hand-written
  cynic, not generated. Delete the file and the `pub mod` at `mutations/mod.rs:54`.
- Optional tidy: drop `'stripeBillingPortal'` from the allowlist at
  `crates/warp_graphql_schema/api/client-schema.ts:60` and regenerate, which
  prunes four blocks from `schema.graphql`. Not needed to compile.

Also delete `update_usage_based_pricing_settings` (`workspace.rs:63-68, 146-186`
and `user_workspaces/mod.rs:1379`) — it is already dead from the top, its UI
having gone with the Billing & Usage page. Keep
`update_workspace_settings.rs`; `update_addon_credits_settings` still uses it.

### P3 — The free-AI-removal modal

*"Warp is no longer providing inference on the free plan… please upgrade to a
paid plan"* with a **View pricing** button. Its Notice path is already dead at
runtime — `one_time_modal_model.rs:552` pins `has_zero_base_credits = false`
since `c4689207d`, so the decision at :858 always defers.

Delete `app/src/workspace/view/free_ai_removal_modal.rs`, the modal branch in
`app/src/workspace/one_time_modal_model.rs` (including
`is_free_ai_removal_modal_open` in the aggregate check at :348), the eight sites
in `workspace/view.rs` (:8, :511, :1135, :2928, :3302, :19156, :25723, :27168),
the two dev commands (`workspace/mod.rs:259, 265`), the persisted setting
`did_check_to_trigger_free_ai_removal_modal` (`settings/ai.rs:1834`), the
`FreeAiRemovalModalTelemetryEvent` enum, and
`one_time_modal_model_tests.rs:51, 208`.

### P4 — The onboarding purchase flow

The largest piece, and the one with a real trap.

**The trap:** `OfferSetUpLaterSelected` is the only way a free user leaves
account-first onboarding by their own action (`root_view.rs:2902-2910`).
Deleting the offer slide without rerouting strands `PostAuthOnboarding` with no
exit. **Fix first, delete second:** make every `FtueAccountClass` at
`root_view.rs:2368-2388` call `complete_account_first` directly, the way
`FtueAccountClass::Paid` already does at :2371.

Then delete:

- `crates/onboarding/src/slides/{offer_slide.rs, ai_access_slide.rs,
  upgrade_auth_prompt.rs}` and their `mod` / `pub use` lines
  (`slides/mod.rs:2, 9, 17, 20, 26`)
- `OnboardingStep::{AiAccess, PostAuthOffer}` (`model.rs:108-119`) — exhaustive
  matches at `next` :852, `back` :803, `set_step` :910, `set_models` :711,
  `progress` :951, `send_account_first_action` :1002, and in the view at
  `agent_onboarding_view.rs:707-739, 847-899`. The legacy sequence becomes
  `Agent → Customize` (was `Agent → AiAccess → Customize`) and the agent path's
  progress dots go 6 → 5 (`model.rs:971-979`)
- `OnboardingStateEvent::UpgradeRequested` :169, `request_upgrade` :679,
  `pricing_promotion_message` :196/:348-362, `on_credit_availability_observed`
  :368, `on_checkout_succeeded` :383, `finish_ai_sell_offer` :401,
  `is_showing_ai_sell_offer` :395, `show_post_auth_offer` :231,
  `offer_variant: Option<OfferVariant>` :192, `ai_access_choice` :188
- `AgentOnboardingEvent::{UpgradeRequested, UpgradeCopyUrlRequested,
  UpgradePasteTokenFromClipboardRequested}` (`agent_onboarding_view.rs:66-68`)
  and their `root_view.rs:2734-2790` handlers, plus the orphan
  `on_ai_credit_availability_observed` :365 (already callerless)
- `AccountFirstCompletion::{UpgradeCompleted, FreeStandardCreditsPurchased}`
  (`root_view.rs:1734-1744`) — both lose every producer; edit the three
  exhaustive accessors at :1747, :1760, :1774. `upgrade_started` on
  `PostAuthOnboarding` (:1826-1831) becomes permanently false, so delete it.
  Keep `FreeIcpSetupLater` / `FreeStandardSetupLater` as the free-account
  completions; the names read oddly with no offer to decline, but renaming
  churns telemetry strings for no gain
- `url_reports_checkout_success` (`app/src/uri/mod.rs:95`) and
  `CHECKOUT_SUCCESSFUL_PARAM`, now that `root_view.rs:3010-3031` is gone
- `OnboardingEvent::{AgentSlideUpgradeClicked, OnboardingUpgradeStarted,
  OnboardingUpgradeCompleted}` — **five** exhaustive matches each, not six:
  `telemetry.rs:106, 133, 230, 290, 328`. Note `AgentSlideUpgradeClicked` is
  already never emitted
- demo-binary bits: `demo_offer_variant` (`bin/main.rs:47-59`), the flag
  force-enable :73-76, `show_post_auth_offer` :149, the offer arm :193-199

Tests to delete or update: `offer_slide_tests.rs` (10 tests, whole file);
`model_tests.rs` — 7 deleted (:19, :81, :101, :163, :179, :207, :225) plus the
`CompletionObserver` harness :129-158, and 2 updated for the new ordering (:294,
:518); `telemetry_tests.rs:21, 75`; `root_view_tests.rs:96, 113`;
`uri_tests.rs:878-908`.

Note the hard-coded price this removes: *"Starting at $18 / mo"*
(`ai_access_slide.rs:258-259`).

### P5 — Unlock what the plan policy switched off

`app/src/workspaces/user_workspaces/billing_workspace_settings.rs` (195 lines)
is the limits layer proper. Every accessor reads a plan policy, and
`is_byo_api_key_enabled` :158 / `is_byo_endpoint_enabled` :173 return `false`
outright for a logged-out user.

**Decision (user, this session): unlock unconditionally.** With no account
there is no plan to consult, so the choice is everything-on or everything-off,
and off means BYOK is unreachable forever — a limit, not a deletion. Collapse
`is_custom_llm_enabled_for_team`, `is_active_ai_allowed`, `ai_allowed_for_team`,
`is_prompt_suggestions_toggleable`, `is_code_suggestions_toggleable`,
`is_next_command_enabled`, `is_git_operations_ai_enabled`, `is_voice_enabled`,
`is_byo_api_key_enabled` and `is_byo_endpoint_enabled` to `true`, and delete
`purchase_policy` :50 outright. Then drop the `BillingMetadata` accessors that
lose their last caller: `can_upgrade_to_build_plan`, `can_upgrade_to_build_max_plan`,
`can_upgrade_to_higher_tier_plan` (`workspaces/workspace.rs:665-693`),
`is_usage_based_pricing_toggleable` :655, `are_overages_toggleable` :148 and
`are_overages_remaining` :159. Keep `are_overages_enabled` :155 —
`ai/blocklist/controller.rs:3493` still reads it.

### Order and commits — DONE (two commits, not five)

`b2003dd2a` **P1+P2+P3+P5** "Remove the paths to Warp's checkout and billing
portal" — 55 files, −4,335. Merged because P1 alone leaves the URL builders
orphaned, and a commit that lands with dead-code warnings is not a better
history than one that doesn't.

`575df62e0` **P4** "Remove the purchase flow from first-run onboarding" —
20 files, −3,178.

Three things the execution found that the plan had wrong:

- **`purchaseAddonCredits` was missed entirely.** A live Stripe mutation whose
  `CheckoutRequired` arm returns a checkout URL, with no production caller left
  since the Billing & Usage page went. Removed with the rest.
- **`is_custom_llm_enabled_for_team` is not a plan entitlement.** P5 collapsed
  it to `true` and the suite caught it: it reads `settings.llm_settings.enabled`,
  a team admin's toggle, not `billing_metadata.tier`. Restored, with only its
  no-workspace default flipped to permit. The other ten accessors were tier
  reads and stay collapsed.
- **The paste-auth-token modal died with the checkout fallback**, which was its
  only constructor.

### Explicitly out of this slice

- **§11, the account gate** — `LoginGatedFeature` and the signup CTAs. Ordered
  after payments on purpose: roughly half the gated features *are* billing
  actions, so P1–P2 shrink that work before it starts.
- `ai/blocklist/usage` (6,299 lines) and `ai/credit_availability.rs` — they
  price a Warp agent conversation and have no seam away from that agent.
  They go when the agent goes.
- `FeatureFlag::{UsageBasedPricing, PricingTransparency}` — flag removal is its
  own mechanical pass.
- Telemetry to Warp's servers as a whole. A separate question from payments.

### Verification

Per AGENTS.md's validation order — targeted check while editing, then tests,
then clippy, formatter last, and **no `./script/presubmit`**.

1. `cargo check -p warp --lib --message-format=short` after each file, then
   `--all-targets`. **`cargo check -p warp --lib` does not cover
   `crates/warp_tui`** — check it separately.
2. `cargo nextest run -p warp` — baseline to beat is 6,731 passing / 0 failing.
   Never `cargo test`: `FLAG_STATES` is a global array of atomics and results
   are unstable run to run.
3. `cargo clippy -p warp --lib --all-targets` and `cargo clippy -p warp_tui`.
4. `./script/format`, then `./script/format --check`.
5. **Manual, and the real test of P4:** delete the local onboarding-completed
   state and launch `./script/run`. First run must reach the terminal without a
   plan-choice slide and without stranding on `PostAuthOnboarding`. Then confirm
   Settings → Warp Agent offers BYOK key entry with no account (P5), and that
   `/upgrade` and `/manage-billing` are gone from the TUI command list.

**Known pre-existing failure, not ours:** `cargo nextest run -p warp_tui`
cannot build its test targets — `crates/warpui/src/platform/mac/delegate.rs:72,
:197` and `headless/delegate.rs:66` are missing `get_cursor_shape`, required by
`crates/warpui_core/src/platform/mod.rs:264`. Confirm it stays untouched with
`git status --porcelain | grep -c 'crates/warpui'` returning `0`.
