# Fork notes

This is a hard fork of `warpdotdev/warp`, built around Conn, a re-entry layer for Claude Code
sessions. We do not track upstream. These notes are the project's working memory: keep them current
in the same commit as the change they describe.

- `conn-plan.md` — Conn's design, findings about Warp's internals, slices done and deferred.
- `deletion-log.md` — order and per-commit notes for deleting Warp's account features and its AI,
  plus leftovers per slice.

## Status (2026-09-24)

AI removal base: `5bfd16338` "Remove the ai crate and the multi-agent API".

Item-1 follow-up cleanup is complete: TUI execution/packaging, obsolete
Warp/Oz CLI commands, settings surfaces, AI workflow/search/tab remnants, commit-message RPC, and
WASM AI references. Workspace/all-targets check, targeted tests, Clippy, and macOS build/launch
verification pass. See the final entry in `deletion-log.md` for coverage and platform limits.

Drive-remnant cleanup removes `app/src/drive`, export and deep-link entry points, dead panel
state, and Drive-only styling/flags/telemetry. Local workflow arguments and icon colors remain;
the subsequent backend slice also deletes the cloud-folder models. See the deletion log for validation.

Account-gate removal makes every root window own a workspace directly, independent of login,
SSO or account status. Login/logout/account UI, browser auth callbacks and logout database-reset
plumbing are gone. Settings opens Appearance; legacy Account settings targets resolve there.
The backend slice now removes account credential refresh, cloud sync, team services, cloud-backed
workflows/embeds, hosted block sharing and shared-session transport. Local preferences and first-run
defaults no longer depend on an account. SSH file/editor/LSP connections carry an installation ID
and privacy preferences, without Warp credentials. Public update/time/telemetry clients remain.
See the final deletion-log entry for validation and retained compatibility types.

The lower-level cloud cleanup removes the unused cloud object, server-client, auth, GraphQL,
managed-secret and Firebase crates. Local workflow, environment variable and historical ID types
now live with their consumers. Settings have no cloud-sync metadata or cloud-origin write path;
remote initialization and old indexing requests reserve removed credential fields. The HTTP
client has no IAP token provider, and the obsolete RTC URL override is gone. Local snapshot ID
formats are unchanged. See the latest deletion-log entry for validation and remaining cleanup.

Candidate next slices (the user picks):
- Account-era SQLite tables and window columns: migrate unused cloud/account data away while
  preserving local snapshot, command-history and pane restoration reads
- Dormant shared-session rendering/model markers and remote code-index status protocol
- Conn slice 2 (`conn-plan.md` §7): SQLite persistence, cross-window overview, read-watermark
- Small UI fixes: Conn empty state for non-terminal panes, Rendered/Raw overlap, Settings content
  clipping beside a wide restored Conn panel

## Decisions

- **Keep:** file viewing, diffs and uncommitted changes, Claude Code and other CLI agents
  (`terminal/cli_agent*`), Conn, code review, the editor and LSP.
- **Delete:** everything that needs a Warp account or leads to payments and limits, and Warp's own
  AI agent (2026-09-23: "We'll use claude code (and maybe some other agents in the future)"). Warp's
  agent routed even BYOK keys through Warp's server, so it could never work without an account.
- Where kept code needs a type from deleted code, move the small piece out instead of keeping the
  module.
- Conn cannot be switched off; there is no feature flag for it.

## Working rules

- Don't touch `crates/warpui` without explicit permission.
- `cargo check --workspace --all-targets --features warp/gui` does not catch unregistered
  singletons; grep `T::handle(` / `T::as_ref(` for a removed model and launch the app.
- `protoc` is still required (`crates/remote_server/build.rs`).
- The WASM target is broken and not maintained.
