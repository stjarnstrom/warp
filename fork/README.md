# Fork notes

This is a hard fork of `warpdotdev/warp`, built around Conn, a re-entry layer for Claude Code
sessions. We do not track upstream. These notes are the project's working memory: keep them current
in the same commit as the change they describe.

- `conn-plan.md` — Conn's design, findings about Warp's internals, slices done and deferred.
- `deletion-log.md` — order and per-commit notes for deleting Warp's account features and its AI,
  plus leftovers per slice.

## Status (2026-09-24)

Tree at `5bfd16338` "Remove the ai crate and the multi-agent API", pushed. The app has not been
launched since; first check code review comments and file edit/save by hand.

Candidate next slices (the user picks):
- AI follow-ups (end of `deletion-log.md`)
- Drive remnants ("Still standing in this slice")
- Account gate, then `server` / `auth` / `cloud_object`
- Conn slice 2 (`conn-plan.md` §7): SQLite persistence, cross-window overview, read-watermark
- Small UI fixes: Conn empty state for non-terminal panes, Rendered/Raw overlap

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
