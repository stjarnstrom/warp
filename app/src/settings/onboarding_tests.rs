use warpui::App;

use super::*;

#[test]
fn onboarding_migration_distinguishes_existing_and_fresh_installations() {
    for first_run in [true, false] {
        App::test((), |mut app| async move {
            app.update(crate::settings::init_and_register_user_preferences);
            app.update(|ctx| {
                initialize_local_onboarding(first_run, ctx);
                assert_eq!(has_completed_local_onboarding(ctx), !first_run);
                // Restarting cannot silently complete a fresh profile's onboarding.
                initialize_local_onboarding(false, ctx);
                assert_eq!(has_completed_local_onboarding(ctx), !first_run);
                mark_local_onboarding_completed(ctx);
                initialize_local_onboarding(true, ctx);
                assert!(has_completed_local_onboarding(ctx));
            });
        });
    }
}
