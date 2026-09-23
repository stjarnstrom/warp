use warpui::App;

use super::*;
use crate::context_chips::prompt_type::PromptType;
use crate::pane_group::BackingView;
use crate::terminal::model::blocks::{INLINE_BANNER_HEIGHT, ToTotalIndex as _};
use crate::terminal::shared_session::SharedSessionSource;
use crate::terminal::view::TerminalAction;
use crate::terminal::view::shared_session::test_utils::terminal_view_for_viewer;
use crate::test_util::add_window_with_terminal;
use crate::test_util::terminal::initialize_app_for_terminal_view;
use crate::{FeatureFlag, assert_lines_approx_eq};

#[test]
fn test_prompt_context_menu_items_shared_session_viewer_no_edit_prompt() {
    App::test((), |mut app| async move {
        let terminal = terminal_view_for_viewer(&mut app);

        terminal.update(&mut app, |view, ctx| {
            let mut model = view.model.lock();
            view.current_prompt.update(ctx, |prompt, ctx| {
                model.set_shared_session_status(SharedSessionStatus::ActiveViewer {
                    role: Default::default(),
                });

                let PromptType::Dynamic { prompt } = prompt else {
                    return;
                };
                prompt.update(ctx, |prompt, ctx| {
                    prompt.update_context(model.block_list().active_block(), ctx)
                });
            })
        });

        let session_settings = SessionSettings::handle(&app);
        session_settings.update(&mut app, |settings, ctx| {
            let _ = settings.honor_ps1.set_value(false, ctx);
        });

        terminal.read(&app, |view, ctx| {
            let items: Vec<MenuItem<TerminalAction>> = view.prompt_context_menu_items(ctx);
            assert_eq!(items.len(), 3);

            // We expect the prompt menu items to be something like the following when no context chips exist:
            // Copy prompt
            // ------------
            // Edit prompt (disabled for shared-session viewers)
            assert_eq!(items[0].fields().unwrap().label(), "Copy prompt");
            assert!(items[1].is_separator());
            assert_eq!(items[2].fields().unwrap().label(), "Edit prompt");
            assert!(items[2].fields().unwrap().is_disabled());
        });
    })
}

#[test]
fn test_shared_session_banners() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);

        let terminal = add_window_with_terminal(&mut app, None);
        let mut expected_block_heights_len = terminal.read(&app, |view, _| {
            assert!(matches!(
                view.inline_banners_state.shared_session_banner_state,
                SharedSessionBanners::None
            ));
            view.model.lock().block_list().block_heights().items().len()
        });

        // Make a block and then insert the shared session starter banner.
        terminal.update(&mut app, |view, ctx| {
            view.model.lock().simulate_block("ls", "foo");
            view.insert_shared_session_started_banner(
                SharedSessionScrollbackType::All,
                false,
                Local::now(),
                ctx,
            );
            expected_block_heights_len += 2;
        });

        terminal.read(&app, |view, _ctx| {
            let model = view.model.lock();

            // Make sure the state has changed.
            assert!(matches!(
                view.inline_banners_state.shared_session_banner_state,
                SharedSessionBanners::ActiveShare { .. }
            ));

            // We should have inserted a block and a banner.
            let block_height_items = model.block_list().block_heights().items();
            assert_eq!(block_height_items.len(), expected_block_heights_len);

            // The banner should have been inserted before the first visible block.
            let first_block_total_index = model
                .block_list()
                .first_non_hidden_block_by_index()
                .unwrap()
                .to_total_index(model.block_list());
            assert_lines_approx_eq!(
                block_height_items[first_block_total_index.0 - 1]
                    .height()
                    .into_lines(),
                INLINE_BANNER_HEIGHT
            );
        });

        // Insert another block and then the shared session ended banner.
        terminal.update(&mut app, |view, ctx| {
            view.model.lock().simulate_block("ls", "foo");
            view.insert_shared_session_ended_banner(ctx);
            expected_block_heights_len += 2;
        });

        terminal.read(&app, |view, _ctx| {
            let model = view.model.lock();

            // Make sure the state has changed.
            assert!(matches!(
                view.inline_banners_state.shared_session_banner_state,
                SharedSessionBanners::LastShared { .. }
            ));

            // by now, we've inserted two new blocks and two new banners since the initialization of the view.
            let block_height_items = model.block_list().block_heights().items();
            assert_eq!(block_height_items.len(), expected_block_heights_len);

            // The first banner should continue to be at the start of the blocklist.
            let first_block_total_index = model
                .block_list()
                .first_non_hidden_block_by_index()
                .unwrap()
                .to_total_index(model.block_list());
            assert_lines_approx_eq!(
                block_height_items[first_block_total_index.0 - 1]
                    .height()
                    .into_lines(),
                INLINE_BANNER_HEIGHT
            );

            // The second banner should be at the end of the blocklist, before the active block.
            let last_block_total_index = model
                .block_list()
                .last_non_hidden_block_by_index()
                .unwrap()
                .to_total_index(model.block_list());
            assert_lines_approx_eq!(
                block_height_items[last_block_total_index.0 + 1]
                    .height()
                    .into_lines(),
                INLINE_BANNER_HEIGHT
            );
        });

        // Mimic starting a shared session again in the same view.
        terminal.update(&mut app, |view, ctx| {
            view.insert_shared_session_started_banner(
                SharedSessionScrollbackType::None,
                false,
                Local::now(),
                ctx,
            );

            // We should have removed two banners and inserted one. So overall,
            // we lost one item in the blocklist since the last time.
            expected_block_heights_len -= 1;
        });

        terminal.read(&app, |view, _ctx| {
            let model = view.model.lock();

            // Make sure the state has changed.
            assert!(matches!(
                view.inline_banners_state.shared_session_banner_state,
                SharedSessionBanners::ActiveShare { .. }
            ));

            // We should have removed two banners and inserted one. So overall,
            // we lost one item in the blocklist since the last time.
            let block_height_items = model.block_list().block_heights().items();
            assert_eq!(block_height_items.len(), expected_block_heights_len);

            // The banner should have been inserted at the end of the blocklist, before the active block.
            let last_block_total_index = model
                .block_list()
                .last_non_hidden_block_by_index()
                .unwrap()
                .to_total_index(model.block_list());
            assert_lines_approx_eq!(
                block_height_items[last_block_total_index.0 + 1]
                    .height()
                    .into_lines(),
                INLINE_BANNER_HEIGHT
            );
        });
    })
}

#[test]
fn test_resize_shared_session_viewer_from_server() {
    App::test((), |mut app| async move {
        let terminal = terminal_view_for_viewer(&mut app);
        terminal.update(&mut app, |view, ctx| {
            // Refresh the size at the start of the test to make sure
            // we're using a consistent size throughout.
            view.refresh_size(ctx);
        });

        let model = terminal.read(&app, |view, _| view.model.clone());
        model
            .lock()
            .set_shared_session_status(SharedSessionStatus::ActiveViewer {
                role: Default::default(),
            });

        // The viewer's current size info.
        let original_size_info = *model.lock().block_list().size();
        let original_num_rows = original_size_info.rows();
        let original_num_cols = original_size_info.columns();

        // Case 1: suppose the sharer has a larger size.
        // The size info we expect is the old one with the greater
        // number of rows and columns (nothing else changed).
        let new_num_rows = original_num_rows + 1;
        let new_num_cols = original_num_cols + 1;
        let expected_size_info =
            original_size_info.with_rows_and_columns(new_num_rows, new_num_cols);

        terminal.update(&mut app, |view, ctx| {
            view.resize_from_sharer_update(
                WindowSize {
                    num_rows: new_num_rows,
                    num_cols: new_num_cols,
                },
                ctx,
            );
        });

        // Make sure the view and model reflect the new, expected size info.
        terminal.read(&app, |view, _ctx| {
            assert_eq!(*view.size_info(), expected_size_info);
            assert_eq!(*view.model.lock().block_list().size(), expected_size_info);
        });

        // Case 2: suppose the sharer has a smaller size.
        // The size info we expect is our old, larger one; nothing changed.
        let new_num_rows = original_num_rows - 1;
        let new_num_cols = original_num_cols - 1;
        let expected_size_info = original_size_info;

        terminal.update(&mut app, |view, ctx| {
            view.resize_from_sharer_update(
                WindowSize {
                    num_rows: new_num_rows,
                    num_cols: new_num_cols,
                },
                ctx,
            );
        });

        // Make sure the view and model reflect the old, expected size info.
        terminal.read(&app, |view, _ctx| {
            assert_eq!(*view.size_info(), expected_size_info);
            assert_eq!(*view.model.lock().block_list().size(), expected_size_info);
        });
    })
}

#[test]
fn test_on_session_share_ended_does_not_insert_tombstone_for_non_ambient_session_under_cloud_mode_setup_v2()
 {
    let _flag = FeatureFlag::CloudModeSetupV2.override_enabled(true);

    App::test((), |mut app| async move {
        let terminal = terminal_view_for_viewer(&mut app);
        let initial_block_height_items = terminal.read(&app, |view, _| {
            view.model.lock().block_list().block_heights().items().len()
        });

        terminal.update(&mut app, |view, ctx| {
            view.model
                .lock()
                .set_shared_session_source(SharedSessionSource::user(None));
            view.on_session_share_ended(ctx);
        });

        terminal.read(&app, |view, _| {
            let final_block_height_items =
                view.model.lock().block_list().block_heights().items().len();
            // Only shared session ended banner.
            assert_eq!(final_block_height_items, initial_block_height_items + 1);
        });
    });
}

// APP-5027 regression: "Copy link" / "Copy session sharing link" must not silently do
// nothing when the Manager has no session id (e.g. during ViewPending / SharePending).

#[test]
fn test_pane_header_copy_link_disabled_when_view_pending_no_session_id() {
    // APP-5027 call-site regression: the pane-header "Copy link" item must be disabled
    // when the terminal is in ViewPending state and Manager has no session_id for this view.
    // This exercises the actual has_session_link call-site computation inside
    // pane_header_overflow_menu_items, not just the session_sharing_context_menu_items helper.
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        app.add_singleton_model(Manager::new);

        let terminal = add_window_with_terminal(&mut app, None);

        // ViewPending simulates a cloud-agent viewer mid-setup: the session exists in the model
        // but Manager has not yet received a session_id for this view.
        terminal.update(&mut app, |view, _| {
            view.model
                .lock()
                .set_shared_session_status(SharedSessionStatus::ViewPending);
        });

        terminal.read(&app, |view, ctx| {
            let items = view.pane_header_overflow_menu_items(ctx);

            let copy_link_item = items
                .iter()
                .find(|item| item.fields().is_some_and(|f| f.label() == "Copy link"));
            assert!(
                copy_link_item.is_some(),
                "Copy link item should appear when terminal is in ViewPending state"
            );
            assert!(
                copy_link_item.unwrap().fields().unwrap().is_disabled(),
                "Copy link must be disabled when Manager has no session_id (ViewPending setup)"
            );
        });
    });
}

#[test]
fn test_session_sharing_context_menu_copy_link_disabled_when_no_session_link() {
    // The "Copy session sharing link" context-menu item must be disabled (greyed out)
    // when the session link is not yet available (has_session_link=false).
    App::test((), |mut app| async move {
        let terminal = terminal_view_for_viewer(&mut app);

        terminal.read(&app, |view, _| {
            let model = view.model.lock();
            // has_session_link=false simulates ViewPending with no registered session_id.
            let items = view.session_sharing_context_menu_items(&model, false);

            let copy_link_item = items.iter().find(|item| {
                item.fields()
                    .is_some_and(|f| f.label() == "Copy session sharing link")
            });
            assert!(
                copy_link_item.is_some(),
                "Copy session sharing link item should be present when is_sharer_or_viewer"
            );
            assert!(
                copy_link_item.unwrap().fields().unwrap().is_disabled(),
                "Copy session sharing link must be disabled when no session link is available"
            );
        });
    });
}

#[test]
fn test_session_sharing_context_menu_copy_link_enabled_when_session_link_available() {
    // The "Copy session sharing link" item must be enabled when the session link is available.
    App::test((), |mut app| async move {
        let terminal = terminal_view_for_viewer(&mut app);

        terminal.read(&app, |view, _| {
            let model = view.model.lock();
            // has_session_link=true simulates an active or ended session with a registered id.
            let items = view.session_sharing_context_menu_items(&model, true);

            let copy_link_item = items.iter().find(|item| {
                item.fields()
                    .is_some_and(|f| f.label() == "Copy session sharing link")
            });
            assert!(
                copy_link_item.is_some(),
                "Copy session sharing link item should be present when is_sharer_or_viewer"
            );
            assert!(
                !copy_link_item.unwrap().fields().unwrap().is_disabled(),
                "Copy session sharing link must be enabled when session link is available"
            );
        });
    });
}
