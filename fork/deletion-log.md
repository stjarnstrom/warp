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


## Drive remnants (2026-09-24)

- Removed `app/src/drive`, its export singleton/registration/tests, empty AI/helper modules,
  sharing/type re-exports, and the Drive object/icon enum. Removed cloud YAML import/export and the
  notebook export wrapper, export capabilities, and the account-switch dialog's export action.
- Removed native Drive deep links, web-to-native Drive URL rewriting, root/workspace workflow
  intent entry points, unused pane-opening settings, welcome-folder auto-opening, the stale
  startup guard, and the WASM Drive intent label/context preset. Added regression coverage for
  rejecting the removed native host and leaving web Drive URLs outside native intent rewriting.
- Removed unused Drive panel resize state. The historical SQLite column remains and is written as
  NULL; file-tree/Conn/code-review panel widths and local pane restoration remain unchanged.
- Kept editor command-block argument handling and its tests under `workflows/arguments`; kept
  existing local workflow/notebook icon colors through a small UI color helper. Moved cloud-folder
  model implementations to `cloud_object/folder` for the next account/cloud-object slice.
- Removed the unused `DriveObjectsAsContext` feature, Drive icon variant and Drive export/open/share
  onboarding telemetry. Renamed the generic left-panel integration helper and tip action, retaining
  the old tip's serialized name as a deserialization alias. The duplicate-name helper now compiles
  only with its remaining cloud-object tests.
- Next slice still owns server/auth/cloud-object infrastructure, cloud settings sync, cloud-backed
  workflow/notebook/embedding types, GraphQL schemas, account telemetry and database tables. Local
  file viewing, Markdown commands, editor/LSP, code review, CLI agents and Conn remain.
- Validation: nextest ran 2,556 tests across `warp`, `warp_core`, `warp_features` and `conn`
  (6 skipped). One existing cloud-preference fixture failed on an unmocked background fetch and
  order-dependent bulk-create expectations. Replaced that test's mock with `FakeObjectClient`,
  preserving its platform-value assertions and waiting on state instead of a fixed sleep/request
  count. All 102 affected cloud-preference/cloud-object/URI/workflow-argument tests pass on rerun.
- All-targets Clippy passes with `-D warnings` for `warp`, `warp_core`, `warp_features` and
  `integration` with GUI enabled. `./script/run --dont-open` built and bundled the macOS app.
  Launched that bundle and visually checked restored terminal, file tree, rendered Markdown and
  code-review diffs. The smoke test used the app's restored workspace; no files there were edited.
- Finished with `./script/format`. `crates/warpui` is untouched. Verification was local macOS only;
  Linux/Windows runtime behavior and unsupported WASM were not exercised. The account-switch
  dialog, editor save, and live CLI-agent/Conn sessions were not exercised.


## Account gate (2026-09-24)

- `RootView` now owns a `Workspace` directly. Removed auth/onboarding state transitions, SSO
  lockouts, account-switch modals, login screen rendering and auth-dependent focus/action guards.
  New/restored windows and settings/file/session deep links reach the workspace unconditionally.
  Changelog checks and update polling start with the workspace.
- Deleted login/signup/SSO/anonymous-user UI, browser token-paste/handoff flows and their unused
  auth-manager helpers. Removed the native `auth` URI host, `ForceLogin` (including Preview's
  override), the skip-Firebase flag and obsolete login/onboarding experiments.
- Removed logout menus/actions, account name/avatar display, reauthentication banners and dead
  modal state. The header gear opens the existing settings/help menu. Removed the Account settings
  page (identity, logout, cloud-sync and staging IAP controls) and account-deletion web link. About
  still shows version information; existing workspace/palette update commands remain. Appearance
  is the default settings page, and persisted `Account` slugs restore to Appearance.
- Removed logout's SQLite deletion/pause/reconstruction protocol and sync-queue reset helper.
  Disabled-account server events only mark backend credentials as needing refresh; they cannot
  destroy the workspace or delete its persisted state. No database schema or migration changed.
- Auth-state providers, stored credentials, startup refresh/API-key validation, server clients,
  cloud preferences, cloud objects, account telemetry and remaining server-backed settings remain
  for the next backend slice. Conn, local files/editor/LSP, CLI agents and code review are retained.
- Added regressions for rejecting auth redirects without exposing credentials and restoring the
  old Account settings target. Removed tests specific to deleted login redirects/logout URL/state;
  retained credential validation/persistence tests. Updated settings navigation integration cases.
- Validation: nextest passed all 2,510 tests across `warp`, `warp_features` and `conn` (6 skipped).
  After pruning the newly unused login and database-reset helpers, all 405 affected auth,
  persistence, server, URI, settings and workspace tests passed. Targeted all-targets Clippy passes
  with `-D warnings` for `warp`, `warp_features` and `integration` with GUI enabled.
- `./script/run --dont-open` built and bundled the macOS app. Launched an isolated
  `WARP_DATA_PROFILE=account-gate-20260924` with `WARP_API_KEY` unset: logs show no stored credentials,
  workspace creation and successful shell bootstrap. Relaunched that profile and the normal restored
  profile without a crash. Stopped the test-owned processes after verification.
- Visual smoke testing is incomplete: computer-use repeatedly returned `cgWindowNotFound` for the
  rebuilt app despite live processes and successful bootstrap logs. Settings interactions and visual
  layout were not verified; the integration crate was compiled/linted, not run on a display.
- Finished with `./script/format`; no `crates/warpui` changes. Verification is local macOS only;
  Linux/Windows and unsupported WASM were not exercised (`oz-dev` is unavailable here). Live Conn
  and CLI-agent sessions were not exercised. Existing account credentials may still drive background
  cloud requests until the backend slice removes them; disabled accounts retain local state.


## Account and cloud backends (2026-09-24)

- Deleted `app/src/auth`, `app/src/cloud_object`, plural `app/src/workspaces`, account/teams/object
  server clients, GraphQL application schemas, cloud listeners/sync queues, server experiments,
  cloud preferences, managed-secret startup and authenticated tracing export. Removed `--api-key`
  and `WARP_API_KEY` launch handling. No account providers or credential-refresh tasks are registered.
- Deleted cloud workflow loading, cloud workflow aliases, workflow environment collection selection,
  notebook cloud embeddings, team switching, cloud API-key settings, hosted block sharing, and
  shared-session network/viewer managers and their controls. Local workflows and shell aliases,
  command arguments, Markdown/Jupyter, editor/LSP, code review, CLI agents and Conn remain.
- Preferences persist locally. Removed organization-forced telemetry/redaction and cloud conversation
  settings; user secret patterns and privacy toggles remain. Preserved installation IDs under their
  historical preference key. First-run terminal/theme/font defaults now run locally and respect
  explicit choices; local onboarding completion is separate from the one-time HOA feature tour.
- The public HTTP client still supports changelogs, updates, clock skew and optional telemetry, with
  network-log hooks and anonymous installation identity. It cannot refresh or attach Warp credentials.
  SSH sidecars use the local installation ID to partition sockets; their handshake omits account
  tokens/user/email, and token rotation and cloud-index mutation senders are deleted.
- Local SQLite snapshots, command/block history, projects and LSP metadata remain. Removed profile
  and cloud-object reads/writes from app persistence without deleting old tables or migrations.
  Historical cloud pane/ID variants still deserialize and are skipped during local pane restoration.
- Compatibility cleanup remains: lower-level cloud crates/types, shared-session rendering/model
  markers, old protobuf fields, settings sync annotations and account-era schema tables are still
  present. They no longer have the removed app services or transports driving them. They should be
  removed in a separate slice while keeping old local snapshots/history readable.
- Validation: the app/Conn/CLI/remote-server nextest run passed 2,486 tests (3 skipped). After the
  SSH and persistence cleanup, 1,153 affected tests passed; both local onboarding migration/modal
  tests passed after the final onboarding changes. GUI-enabled all-targets check and Clippy with
  `-D warnings` passed for the app, integration crate, remote server and CLI.
- Built and bundled the macOS app with `./script/run --dont-open`. With `WARP_API_KEY` unset, an
  isolated fresh profile opened the local welcome screen; a terminal bootstrapped and executed a
  smoke command. Computer-use verified Appearance settings and secret-redaction search. Relaunch
  restored the terminal output and Settings tab. The previous account-gate test profile also launched
  and bootstrapped successfully. Stopped all test-owned instances. The visual check caught and fixed
  obsolete Drive/Oz wording in the privacy description; rebuilt and visually verified the new text.
  Settings content clipped beside the wide Conn panel after restore; recorded as a remaining UI slice.
- Finished with `./script/format`; `crates/warpui` is untouched. Verification is local macOS only:
  `oz-dev` is unavailable, so Linux/Windows were not exercised; WASM remains unsupported. Integration
  tests were compiled/linted, not run on a display. Live SSH, Conn and CLI-agent sessions were not
  exercised; SSH handshake behavior and Conn remain covered by the targeted unit suites.


## Dormant cloud crates and compatibility fields (2026-09-25)

- Deleted the disconnected cloud object client/model/persistence, server client/auth, GraphQL,
  schema, managed-secret, WASM managed-secret and Firebase crates. The network log model now
  belongs to the app; the remaining local workflow, environment variable, MCP transport and
  historical ID types sit with their consumers. Removed the obsolete cloud workspace dependencies,
  GraphQL codegen/WebSocket adapter, dormant AWS BYO-LLM dependencies and unused Tink patches.
- Removed settings cloud-sync annotations, platform sync policies, cloud-origin setters and
  settings-manager sync callbacks. Local storage, TOML reload and private preference routing stay
  intact. Removed IAP configuration and token injection from the HTTP client, along with the unused
  OAuth HTTP adapter and RTC URL override.
- Remote-server initialization no longer carries account token, user ID or email. The removed
  notification and authentication fields in old code-index messages are reserved in protobuf so
  their wire numbers cannot be reused. Crash reporting and code-index limits still initialize.
- Historical notebook/workflow/generic-object ID prefixes and client-ID JSON forms remain for
  local SQLite and snapshot decoding. Added ID round-trip tests. Legacy cloud panes still decode
  and are skipped during restoration. Account-era SQLite tables and window columns are not dropped
  in this slice; they require an explicit migration that protects local snapshot/history reads.
- Dormant shared-session rendering/model markers and remote code-index status types remain for a
  separate cleanup. The shared-session transport has already been removed.
- Validation: 2,600 targeted app/settings/remote/MCP/CLI/core tests passed before the final
  dependency pruning (3 skipped); 2,516 app/settings/remote/WebSocket tests passed on the final
  graph (3 skipped). Four settings doctests passed. All-targets Clippy with `-D warnings` passed
  for the affected crates and integration target, including a rerun after the final dependency
  change. `./script/run --dont-open` built and signed the macOS bundle.
- Launched that bundle with a fresh isolated `WARP_DATA_PROFILE` and no API key. Logs show local
  database creation, terminal-server startup, a window opening and its first frame; no panic or
  error was logged. Stopped the test-owned process and removed its isolated profile. The launch
  did not exercise shell input or settings interactions. Cross-platform cloud verification could
  not run because `oz-dev` is unavailable here; Linux/Windows and unsupported WASM were not
  exercised. `crates/warpui` is untouched.
- Finished with `./script/format`.
