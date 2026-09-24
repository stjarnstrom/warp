# Deletion log

Order, per-commit notes and leftovers for deleting Warp's account-dependent features and its AI.


Deleting everything that needs a Warp account, in this order:

1. **Done** — every login prompt and signup CTA (§11, commits `fad1ffa0e`,
   `9945f2c7c`, `220b65ff7`). Nothing in the app asks for an account.
2. **Done** — shared sessions, six commits `c82d9dc9a`..`2b7437783`
   (87 files, −12,974). Sharing creation, joining, the share modal, remote
   control, the sharer half of `SharedSessionStatus`, the close-session
   confirmation dialog and the `Kind::Sharer` view state are all gone.
3. `drive` + the cloud-notebook half of `notebooks` — mostly done (commits below); remnants listed
   in "Still standing in this slice".
4. **Done** — `ai`: app module `7bc4f36bf`, crates `5bfd16338`. `code_review` was KEPT (user
   decision), docked beside Conn in the right panel (`4097a3bc0`).
5. Then `server` / `auth` / `cloud_object`.

**What survives shared-session deletion, and why** — the viewer transport
(`terminal/shared_session/viewer/`, `PaneGroup::create_shared_session_viewer`,
`Manager`'s `joined_*` half, the "Sharing started/ended" inline banner that
ambient relabels "Environment started/ended"). Warp's **ambient agents attach
to a cloud VM through the shared-session viewer**, so the viewer machinery
goes with `ai/ambient_agents`, not with sharing. `SharedSessionStatus` keeps
`ViewPending` / `ActiveViewer` / `FinishedViewer` for the same reason, and
`is_sharer_or_viewer()` keeps its name while only ever meaning viewer.

**Two corrections that measurement forced on 2026-09-22** — the earlier
"drive + notebooks + shared sessions, fewest dependents" grouping was wrong:

- **`notebooks` is not deletable as a module.** Of 22.9k lines,
  `notebooks/editor/` (~14k) is the rich-text editor engine and
  `notebooks/file/` (1.4k) is the local markdown viewer behind `file_pane` —
  the file viewing the user said to keep. Only ~4k (`mod.rs`, `notebook.rs`,
  `manager.rs`, `link.rs`) is the cloud object. And `drive` and `notebooks`
  are mutually dependent (`notebooks/mod.rs` imports
  `drive::items::notebook::WarpDriveNotebook`), so they are one unit.
- **`ai` is the biggest consumer of all three** (21 drive refs, 22 notebooks,
  40 files for shared sessions), so doing it first would save work — but it
  cannot go first: `terminal/model/blocks.rs` imports `AIBlock` and
  `SerializedBlockListItem` from `ai/blocklist`, and `terminal` references
  `ai` 559 times. So `ai` stays last and step 3 means touching some files
  twice. Accepted deliberately.

**The method that worked** — delete the user-facing entry point first, then
the producer, then let `cargo check` and dead-code warnings name the cascade.
Splitting one feature across several green commits beats one large one: each
commit's message records what was left half-standing on purpose, so the next
commit has its own starting point. Python scripts doing exact-string
replacement with a count assertion, staging every file in memory and writing
only after all replacements succeed; plus a `delete_fn(path, name)` that walks
back over attributes and doc comments and forward to the column-0 closing
brace. Baseline: nextest 6,633 before the sequence, 6,606 after.

Prompts came first so that "Sign up" walls became visible dead ends rather
than hiding what is actually broken. Backends go last because everything
above depends on them.

Conn survives step 4: `cli_agent_sessions` + `conn` touch `crate::ai` in only
three shallow places (`blocklist::InputConfig`, one `ConversationStatus`
conversion).

**Step 3 commit 1 landed:** `4273d014e` "Remove the Warp Drive panel and every
way into it" — 88 files, +119 / -12,092. nextest 6,606 -> 6,594. Still to come
in this slice: the rest of `drive/` (sharing, settings, folders, export, import,
the workflow modal, styling helpers) and the cloud notebook, both of which the
cloud objects still depend on.

Findings worth keeping from that pass:

- `MIN_SIDEBAR_WIDTH` / `MAX_SIDEBAR_WIDTH_RATIO` lived in `drive::panel` and
  are used by Conn's own panel and the right panel. Moved to `workspace::view`
  first, before anything else was touched.
- `LeftPanelDisplayedTab` is `Serialize`/`Deserialize` and read with
  `serde_json::from_str(..).ok()`, so dropping its `WarpDrive` variant makes an
  old session's left-panel snapshot deserialize to `None` — panel reopens at its
  default tab and width, once, for anyone whose last tab was Drive.
- Deleting an enum variant whose match arm is written bare under `use Enum::*`
  (`ExportAllWarpDriveObjects => {}`) turns that arm into a catch-all binding and
  the compiler reports *unreachable pattern* on every arm after it, not an error
  at the arm itself. Read the first unreachable-pattern warning, not the list.
- Only `index.rs` + `panel.rs` + the dialogs were dead once the entry points went.
  `drive/{sharing,settings,folders,workflows,export,import,cloud_object_styling,
  drive_helpers}` are still referenced from outside `drive/` and belong to the
  cloud-object slice, not this one.
- `WarpDriveItem` was the index's row trait; `CloudObject::to_warp_drive_item`
  existed only to build those rows. Both gone. Two real consumers had to be
  rewritten rather than deleted: the agent citation chip in
  `ai/blocklist/block/view_impl.rs` (now matches `ObjectType` for its icon) and
  the rules page sync icon in `ai/facts/view/rule.rs` (now calls
  `metadata.pending_changes_statuses.render_icon` directly).
- Drive breadcrumbs appeared in three surviving views (notebook details bar,
  workflow view, env var collection). All three only navigated into the panel,
  so the rows went with it.
- `warp_drive_index_width` is a persisted SQLite column and was left in place;
  removing it is a migration, not part of this slice.
- `local_control::tests::capabilities_advertises_the_complete_catalog` asserts
  an exact count of the `warpctl` action catalog. Removing surface actions fails
  it with a bare `left != right`; update the number.
- `ai::agent_sdk::driver::mcp_startup::tests::initial_global_scan_and_readiness_share_one_bounded_timeout_budget`
  is timing-sensitive and fails under full-suite load while passing in
  isolation. Not a regression; re-run before chasing it.

## `a7e006a28` — the sharing dialog and the object ACL API (2026-09-22)

76 files, +109 / −6,186. `cargo nextest run -p warp`: 6582 passed / 0 failed /
6 skipped (was 6594; twelve tests deleted). Clippy and format clean on `warp`
and `warp_tui`; `git status --porcelain | grep -c 'crates/warpui'` = 0.

Cut: `SharingDialog` + ACL rows, `ShareableObject`, the `Subject`/`UserKind`/
`TeamKind` display traits, `SharedPaneContent` and the pane-header share/
copy-link/eye buttons, the `pane:share_pane_contents` binding and its Drive menu
item, "Share conversation" / "Copy share link" in the AI block context menu and
the conversation-list overflow menu, the shared-session QR code, the three
`PaneConfiguration` sharing events, the eleven `UpdateManager` permission
methods, the five `ObjectClient` ACL calls with their cynic mutations, and
`TelemetryEvent::OpenedSharingDialog`.

Findings:

- `SharingAccessLevel` and `ContentEditability` **stay**. They decide whether a
  cloud object opens read-only, and ~15 files outside `drive/` read them. They
  go with the cloud-object slice.
- `PaneView` had to keep `impl TypedActionView { type Action = (); }` even
  though `ShareContents` was its only action: `AppContext::add_window` requires
  its root view to be a `TypedActionView`, and `header/mod_tests.rs` puts a
  `PaneView` at a window root. The `type Action = ()` no-op impl has precedent
  (`auth_override_warning_modal.rs:138`). Its fifteen construction sites moved
  from `add_typed_action_view` to `add_view`.
- `HeaderRenderContext<'a>` lost its lifetime — the sharing-controls closure was
  the only thing borrowing. That means editing the ~23 `HeaderRenderContext<'_>`
  signatures; a `sed` over `grep -rl` does it.
- The permission half of `UpdateManager` died as one unit (11 methods) because
  the dialog was its only caller, and took `ObjectPermissionsUpdateData`,
  `ObjectPermissionUpdateResult` and `GuestIdentifier` in `cloud_object_client`
  with it. `update_permissions_pessimistic` was the shared helper — deleting the
  public methods orphans it, so the dead-code warning names the whole set.
- `word_block_editor.rs` survives (the teams page still uses it) but its
  `WordBlockLayout::Horizontal` path and four builder methods were dialog-only.
  `WordBlockEditorViewEvent::Navigate(NavigationKey)` became a unit variant: both
  remaining consumers match `Navigate(_)`.
- `Adapter::started_at` on the shared session had no reader left once
  `ShareableObject::Session` went — field, accessor and both constructor
  parameters removed.
- Deleting `pub use` re-exports in `drive/sharing/mod.rs` warns in the lib target
  even when `*_tests.rs` files still use them. Point the tests at
  `cloud_objects::drive::sharing::` directly rather than keeping the re-export.

## cf833eab1 — import modal + workflow modal (26 files, −5,592)

- Criterion check: Drive object creation needs `UserWorkspaces::personal_drive`, which
  is `None` with no account, so import, "Save as workflow", Create* notebook/workflow/
  env-var commands all silently no-op logged out. That puts all Drive authoring in scope.
- `WorkflowModal` had no opener left after the Drive panel went. Kept
  `drive/workflows/{arguments,enum_creation_dialog,workflow_arg_selector,
  workflow_arg_type_helpers,ai_assist}`: `workflows/workflow_view.rs` uses them.
- Breadcrumbs were write-only: import modal was the last renderer. Removed
  `ui_components/breadcrumb.rs`, view fields, `ActiveEnvVarCollectionDataEvent::
  {BreadcrumbsChanged, CreatedOnServer}`. `CloudObject::containing_objects_path` stays.
- nextest 6572 passed / 0 failed (−10 = deleted test files 7+1+2).
- **Pre-existing, unfixed:** `crates/integration` fails to compile since `4273d014e`
  (imports `assert_warp_drive_is_{open,closed}`, test at `test.rs:6833`). Check it with
  `cargo check -p integration` — not covered by `-p warp`.

## daa932f17 — integration crate fixed (deleted Drive-panel test). ALWAYS run `cargo check -p integration --all-targets` too.

## 7303f9e0f — cloud notebook pane (84 files, −8,518)

- Kept: `notebooks/{editor,file,link,context_menu,styles,telemetry}` (the local Markdown viewer
  uses them). `subscribe_to_link_model` moved into `file_pane.rs`.
- Persistence: `NotebookPaneSnapshot::CloudNotebook` kept so old sqlite rows still read;
  `PaneGroup` restore bails on it, and a restore error drops just that leaf (`restore_pane_tree`).
- Cascade went further than the module: `IPaneType::Notebook`, `TypedPane/SummaryPaneKind::Notebook`
  (vertical tabs), `QueryFilter::{Notebooks,Plans}` (warp_search_core), palette + AI context menu
  notebook sources, `ArtifactButtonsRowEvent::OpenPlan` + `OpenPlanNotebook` events (plans = cloud
  notebooks), `WorkspaceAction::{OpenNotebook,CreatePersonalNotebook}`, `NewWorkspaceSource::NotebookById`,
  Drive links in markdown (`LinkEvent/pane_group::Event::OpenWarpDriveLink`, `uri/parse_url_paths.rs`),
  `grab_edit_access_modal.rs`, notebook edit-access in UpdateManager/ObjectClient/cynic, `MenuSource::TextEditor`.
- `UpdateManager::update_notebook_title` kept `#[cfg(test)]`: sync-queue tests use notebooks as the
  sample object. Goes with the cloud_object slice.
- Tests: pane_group/workspace tests swapped NotebookPane -> `FilePane::new(None, None, None, ctx)`;
  context-menu tests ported into `notebooks/file/mod_tests.rs`. wasm-only
  `simplified_wasm_tab_bar_is_some_for_drive_object_without_deep_link` deleted unported (can't build wasm).
- nextest `-p warp -p warp_search_core`: 6566 passed / 0 failed / 5 skipped.

## 934c3f297 — workflow + env-var panes (82 files, −15,433)

- Also went: Save as workflow everywhere (block button, input/block context menu, cmd-shift-S,
  Warp AI transcript, info box Edit/Save), AI suggested-prompt chip + `suggested_agent_mode_workflow_modal`,
  WorkspaceAction::Create{Personal,Team}{Workflow,AIPrompt}/CreatePersonalEnvVarCollection/
  HandleConflicting*, CustomAction::New*, `external_secrets` + `search/external_secrets`,
  `drive/workflows/{enum_creation_dialog,workflow_arg_selector,workflow_arg_type_helpers}`,
  `env_vars/{manager,view,active_env_var_collection_data}`, `WorkflowViewMode`, IPaneType::{Workflow,EnvVarCollection}.
- Kept: `workflows/env_var_selector.rs` (moved out of workflow_view; info box uses it), running cloud
  workflows (`open_workflow_from_intent` now only runs, no pane fallback).
- UpdateManager: 7 methods `#[cfg(test)]` (create_workflow is `any(test, integration_tests)`), plus
  `ObjectOperation::Untrash`, `FetchSingleObjectOption::None`, re-export `ObjectMetadataUpdateResult`.
  All go with the cloud_object slice.
- nextest 6563 passed / 0 failed / 5 skipped (−3 = env_var_collection_tests).

## Still standing in this slice

`drive/{settings,folders,export,workflows helpers,cloud_object_styling,drive_helpers,
cloud_action_confirmation_dialog}`. `folders` is a cloud object type: goes with cloud_object slice.


## 870f5bd5d — Drive embed search (19 files, −1,317)

- `search/notebook_embedding` fed the "Embed" item of the markdown block-insertion menu (default on in
  the local file viewer); searched CloudModel only. Removed with `embedded_objects_enabled` config,
  `OpenEmbeddedObjectSearch`, `InsertedEmbeddedObject`, `EmbeddedObjectInfo`, `set_space`, `Icon::EmbedBlock`.
- Kept: rendering of existing embeds (`editor/embedded_item`, `EditWorkflow`, `RemoveEmbeddingAt`).
- nextest 6561 passed (−2 = workflows_data_source_tests).


## b0880b80f — Teams settings page (33 files, −7,711)

- §11 was already done (`fad1ffa0e`..`220b65ff7`) — check this file before proposing a slice.
- Went: `teams_page`(+tests), `join_teams_modal`, `tab_menu`, `admin_actions`, `transfer_ownership_confirmation_modal`,
  `drive/cloud_action_confirmation_dialog`, `word_block_editor`, `clickable_text_input`, `SettingsSection::Teams`,
  `WorkspaceAction::BrowseTeams` + `TeamNavigationMode`, `UriHost::Team` + `warp://settings/teams`, root_view team
  actions, `CustomAction::OpenTeamSettings`, terminal `OpenTeamSettingsPage`, TeamUpdateManager create/leave/rename
  + its event enum, `UpdateManager::remove_team_objects`, telemetry `ChangedInviteViewOption`.
- Ambient `CloudAgentTeamRequiredView` kept, button removed (goes with ai/ambient).
- Kept: team switcher pill (only renders with an account that has >1 team) — server/auth slice.
- **Dead pub API left:** UserWorkspaces/TeamClient invite/member/transfer methods are `pub` in the lib crate so
  rustc doesn't flag them. Sweep in server/auth slice. Also Drive menu still has ToggleWarpDrive/SearchDrive.
- nextest 6524 passed (−37, all in deleted modules + test_team_navigation_mode + test_leaving_team_removes_objects).


## 741b14f8c — Warp Drive settings + gates (59 files, −3,019)

- `is_warp_drive_enabled` was false whenever logged out → all its gates dead. Went: settings page,
  Appearance toggle, onboarding chip (+4 pngs), `drive/settings.rs`, `DriveSortOrder`, `UpdateSortingChoice`,
  context flag `ENABLE_WARP_DRIVE`, local_control WarpDrive rejection.
- Command search: local/project/global `WorkflowsDataSource` now UNGATED (it's local files); cloud workflows +
  env-var sources deleted; `WorkflowIdentity` gone (field `workflow: Box<WorkflowType>`), `AcceptedWorkflow` is a struct.
- AI context menu: Workflows + Rules categories and modules gone (CloudModel-only), `InsertDriveObject`/`InsertPlan`.
- `CloudViewModel` is now a unit struct with only `object_space` (editor/access/sort-timestamp code dead).
- `CustomAction::ToggleWarpDrive` renamed `ToggleLeftPanel` (it's cmd-\); `SearchDrive` gone. Drive menu now only
  AIFactCollection + MCPServerCollection (ai slice).
- **Left for later:** palette `warp_drive::DataSource` still built (prompts data source + recents use it),
  `CommandPaletteItemAction::{ExecuteWorkflow,InvokeEnvironmentVariables}` / `ItemSummary::{Workflow,EnvVarCollection}`
  now unproduced → cloud_object slice. `AISettings::warp_drive_context_enabled` + Knowledge page toggle → ai slice.
  `FeatureFlag::DriveObjectsAsContext` unused → flag pass. `CommandSearchResultType::{EnvVarCollection,ViewInWarpDrive}` → telemetry pass.
- nextest 6517 passed (−7 = 5 CloudViewModel tests + 2 rules data_source_tests); onboarding 21 passed.


## 044b7420e — saved prompts + palette Drive source (54 files, −3,818)

- Went: `/prompts` + `terminal/input/prompts/` inline menu (`InputSuggestionsMode/InlineMenuType/TelemetryInputSuggestionsMode::PromptsMenu`),
  slash `saved_prompts` source (GUI, cloud-mode v2 `Section::Prompts`, TUI arms), `SlashCommandsEvent::SelectedSavedPrompt`,
  `SlashCommandAcceptedDetails::SavedPrompt`, palette `warp_drive::DataSource` + items + `ItemSummary::{Workflow,EnvVarCollection,CloudObject}`,
  `CommandPaletteItemAction::{ExecuteWorkflow,InvokeEnvironmentVariables}`, `search/env_var_collections`, QueryFilter `{AgentModeWorkflows,Drive,EnvironmentVariables,Rules}`.
- Renamed `AcceptSlashCommandOrSavedPrompt` -> `AcceptSlashCommandOrSkill`.
- Test fix: changelog slash test needed `FeatureFlag::Changelog.override_enabled(true)`; passed before only because async saved-prompt source kept menu open.
- `InlineMenuType` is a persisted HashMap key (private resize heights) — variant removal may reset those. Accepted.
- **cloud_object slice proper is blocked on ai + settings sync:** UpdateManager/CloudModel still written by ai_document_model (plans->notebooks, folders),
  facts/rules, MCP templatable manager, environments page, scheduled agents, cloud_preferences_syncer, tui_onboarding_markers. `drive_helpers` notebook limit
  goes with ai_document_model. Do ai slice first.
- nextest 6491 passed (−26 = palette data_sources_tests + saved_prompts_tests + prompts data_source_tests); `test_debounced_resizes` flaky once.


## b5d4f98fa — warp_tui + LaunchMode::Tui (259 files, −99,678)

- Started the ai slice (decision in `README.md`). Went: `crates/warp_tui`, `script/run-tui`, windows tui installer,
  app `tui` feature + every `cfg(feature = "tui")` item, `app/src/tui*`, `tui_onboarding_markers`, `settings/tui_*`,
  `ai/tui_api_keys`, `server_api/tui_onboarding`, `PersistenceScope::Tui`/`PersistedDataScope::TuiFrontend`,
  `paths::tui_state_dir`, TUI IAP non-blocking auth, TUI secure-storage suffix.
- **Left for a "TUI remnants" pass (or dies with ai):** `SettingsMode`/`SettingSurfaces` (+444 `surface:` attrs),
  `ExecutionMode::Tui`, `LogFrontend::Tui`, `SlashCommandSurfaces::TuiOnly` + `slash_commands/data_source/tui.rs`,
  `BundledSkillActivation::TuiOnly`, `CLIAgent::WarpTui` + `is_running_warp_tui`, `paths::tui_config_local_dir`,
  Cargo `release-tui` profiles, `script/*/bundle` tui branches, create_release.yml tui jobs.
- nextest 6593 passed (warp+search_core+warp_core+settings).
- Next: big-bang `rm -rf app/src/ai`, relocating keepers first/reactively: `persisted_workspace` (repos/LSP),
  `agent::redaction`, code-review `DiffSetHunk`/`AgentReviewCommentBatch`/`CurrentHead`/`DiffBase`,
  `ConversationStatus` + `conversation_status_ui` (CLI agent tab status), `CLAUDE_ORANGE`. `crates/ai` stays until after.


## 7bc4f36bf — ai deletion (2026-09-23, pushed)

- `app/src/ai/` + ~70 agent modules gone; keepers relocated (code_review/diff_types.rs, ui_components/agent_status.rs,
  persisted_workspace.rs, settings/cli_agent.rs). CLI-agent plugin auto-install/update re-wired into UseAgentToolbar.
- Lost with it: slash-command menu (incl. `/open-file`), rich-input image paste, AI commit-msg/PR generation,
  `warp` command-line SDK mode (returns error).
- Validated: workspace `cargo check --all-targets` clean; clippy -D warnings clean (warp gui, integration, warp_cli,
  warp_search_core, conn); nextest 2835 passed (drop from 6593 = deleted AI suites, no orphaned test files); fmt clean.
- Gotcha: in zsh `$h:app/...` applies the `:a` modifier — write `${h}:path`. `git diff <stash-commit>` shows new
  untracked files as deleted; not real.
- Follow-ups: `CliCommand` in warp_cli, `SurfaceCommand::AiAssistant`, `SettingsMode`/`SettingSurfaces`,
  `ExecutionMode::Tui`, `CLIAgent::WarpTui`, `crates/ai`, `crates/warp_multi_agent_api`, `DiffType` in
  local_code_editor, `TerminalAgentText.conversation_*`, `WorkflowType::AIGenerated`, `QueryFilter::Conversations`,
  remote-server `git_generate_commit_message`, `AppExecutionMode` `is_sandboxed`, `release-tui` profiles/bundle scripts.
- **Launch panicked after 7bc4f36bf**: `CodebaseIndexManager`/`ProjectContextModel` registrations went with ai but
  callers stayed (settings dir-color picker, logout, remote daemon). `cargo check` + unit tests can't catch
  unregistered singletons — after removing any `add_singleton_model`, grep `T::handle(`/`T::as_ref(` for that T,
  and launch the app. Fixed uncommitted: daemon now answers index/fragment requests "not supported".
  WASM build (`wasm_view.rs`, view_tests wasm mod) still references `crate::ai` — broken, not a target.

## Smoke test + right panel (2026-09-24)

- Pushed: `a9f4f27f0` (launch crash: unregistered CodebaseIndexManager/ProjectContextModel),
  `730399eb7` (warpui keycode crash on null UTF8String — user approved the warpui edit).
- `4097a3bc0` pushed, user verified: right side is ONE window-wide slot, `Workspace::right_panel_content:
  Option<RightPanelContent {CodeReview, Conn}>`; toolbar items `CodeReview` (Diff icon) + `Conn` side by side.
  Code review's per-pane-group `right_panel_open` kept, synced on tab switch by `reconcile_right_panel_for_active_tab`.
  User first rejected, then accepted, the single-slot design: no side-by-side needed.
- Noted, not fixed: Conn "No pane focused" for non-terminal panes; Rendered/Raw toggle overlaps file title.

## 5bfd16338 — crates/ai removal (2026-09-24, pushed)

- Deleted crates `ai`, `ai_types`, `warp_multi_agent_client`, dep `warp_multi_agent_api` (−44.6k lines).
- Relocated: `DiffDelta` → `app/src/code/editor/diff.rs`; `WorkspaceMetadata` → `persisted_workspace.rs`;
  `LLMId`/`AIDocumentId` → plain `String` in cloud_object_models (serde-transparent, same JSON).
- Deleted as dead: `LocalCodeEditorView` diff_type + enable_diff_nav param (all callers passed None/false);
  imported-PR-comment pipeline in code_review/comments (only the agent produced them); onboarding agent flow
  (agent_onboarding_view, model, 13 slides, demo bin — root_view no longer constructed it); persistence AI
  conversation models + graphql→persistence conversions; `CLIAgent::supported_skill_providers`.
- **protoc still required**: `crates/remote_server/build.rs` uses prost-build. Earlier claim was wrong.
- Kept for Drive/account slice: graphql AI types (server schema union, managed_secrets harness),
  `ai_execution_profile` cloud object, `notebook.ai_document_id`, FeatureFlags full_source_code_embedding etc.
- Item-1 follow-ups (completed in the entry below): ExecutionMode::Tui, CLIAgent::WarpTui, release-tui, warp_cli CliCommand/AiAssistant,
  WorkflowType::AIGenerated, QueryFilter::Conversations, git_generate_commit_message, TerminalAgentText.conversation_*,
  SettingsMode/SettingSurfaces, is_sandboxed, wasm crate::ai refs.


## Item-1 follow-ups — Remove remaining AI and TUI scaffolding (2026-09-24)

- Removed `ExecutionMode::Tui`, `AppExecutionMode::is_sandboxed` and unused AI capability methods;
  `CLIAgent::WarpTui`, its listeners/tests and nested-TUI input/footer exceptions. Other CLI-agent
  detection, rich input, status, and code-review destinations remain.
- Removed `SettingsMode` / `SettingSurfaces`, per-setting `surface:` declarations, schema surface
  annotations, TUI config paths/watchers, and the redundant cloud-sync surface switch. GUI config
  paths, native-store migration, and settings sync behavior remain. The default-settings generator
  no longer accepts `--surface`.
- Removed Warp/Oz `CliCommand` / `Command::CommandLine`, their command modules and parser tests.
  Desktop URLs, worker subprocesses, completions, settings-schema output, and `warpctrl` remain.
  Shared `Harness` and `OutputFormat` moved to small modules in `warp_cli`; cloud-object harness
  serialization remains for the account/Drive slice. Removed `surface.ai_assistant.toggle` from
  the CLI, action catalog, bridge and surface metadata.
- Removed `WorkflowType::AIGenerated`, `QueryFilter::Conversations`, unused Warp-conversation tab
  title fields/fallbacks, and WASM transcript details/tests that referenced deleted `app::ai`.
  This does not restore the unsupported WASM build.
- Removed the remote commit-message-generation RPC end to end; request field 18 and response field
  31 (and names) are reserved. Manual git commit/push/PR and editor operations remain.
- Removed `release-tui` profiles and TUI branches from all three platform bundlers; removed the six
  TUI build/release jobs and TUI release-result/changelog plumbing. Surviving build/release jobs are
  unchanged. Existing standalone CLI packaging remains outside this TUI packaging cleanup.
- Validation: workspace/all-targets/gui check clean. Nextest exercised 2,821 tests (5 skipped);
  fixed the local-control catalog count and four cloud-sync fixtures that used random client IDs
  despite fixed mock response IDs. All affected CLI/local-control tests and all 17 cloud-sync tests
  pass on rerun. Targeted all-targets Clippy passes with `-D warnings`;
  `cargo check -p integration --all-targets` passes. Finished with `./script/format`;
  `git diff --check` is clean.
- Release YAML parses with all `needs` resolved; a semantic comparison confirms that surviving jobs
  are unchanged except result collection and changelog generation. macOS/Linux bundlers pass `bash -n`.
- `./script/run --dont-open` builds and bundles successfully on macOS. Launched the rebuilt app
  without a panic and visually checked the terminal, file tree, code-review diff panel, and rendered
  Markdown viewer. Editor save and live CLI-agent/Conn sessions were not exercised. `crates/warpui`
  is untouched.
- Validation above was local; no cloud runners were used. Linux/Windows runtime packaging and
  WASM are not verified.
