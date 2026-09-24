use warp_core::features::FeatureFlag;
use warpui::{App, SingletonEntity};

use super::{OneTimeModalModel, hoa_onboarding};
use crate::test_util::terminal::{add_window_with_terminal, initialize_app_for_terminal_view};

#[test]
fn onboarding_uses_local_completion_without_account_services() {
    App::test((), |mut app| async move {
        let _hoa = FeatureFlag::HOAOnboardingFlow.override_enabled(true);
        let _vertical_tabs = FeatureFlag::VerticalTabs.override_enabled(true);
        let _notifications = FeatureFlag::HOANotifications.override_enabled(true);
        let _tab_configs = FeatureFlag::TabConfigs.override_enabled(true);
        initialize_app_for_terminal_view(&mut app);
        let terminal = add_window_with_terminal(&mut app, None);
        terminal.update(&mut app, |_, ctx| {
            let window_id = ctx.window_id();
            OneTimeModalModel::handle(ctx).update(ctx, |model, ctx| {
                assert!(!model.is_any_modal_open());
                model.update_target_window_id(window_id, ctx);
                assert!(model.is_hoa_onboarding_open());
                hoa_onboarding::mark_hoa_onboarding_completed(ctx);
                model.mark_hoa_onboarding_dismissed(ctx);
                model.update_target_window_id(window_id, ctx);
                assert!(!model.is_any_modal_open());
            });
        });
    });
}
