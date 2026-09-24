use warpui::App;

use super::*;
use crate::test_util::terminal::initialize_app_for_terminal_view;

#[test]
fn fresh_install_uses_universal_input_without_populating_secret_patterns() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        app.update(|ctx| {
            initialize_local_defaults(true, ctx);
            assert_eq!(
                *InputSettings::as_ref(ctx).input_box_type.value(),
                InputBoxType::Universal
            );
            assert!(!*SessionSettings::as_ref(ctx).honor_ps1.value());
            let privacy = PrivacySettings::as_ref(ctx);
            assert!(privacy.user_secret_regex_list.is_empty());
            assert!(*privacy.has_initialized_default_secret_regexes.value());
        });
    });
}

#[test]
fn first_run_preserves_explicit_input_and_theme_choices() {
    App::test((), |mut app| async move {
        let _theme = FeatureFlag::DefaultAdeberryTheme.override_enabled(true);
        initialize_app_for_terminal_view(&mut app);
        app.update(|ctx| {
            InputSettings::handle(ctx).update(ctx, |settings, ctx| {
                settings
                    .input_box_type
                    .set_value(InputBoxType::Classic, ctx)
                    .unwrap();
            });
            ThemeSettings::handle(ctx).update(ctx, |settings, ctx| {
                settings
                    .theme_kind
                    .set_value(ThemeKind::Phenomenon, ctx)
                    .unwrap();
            });
            let honor_ps1 = *SessionSettings::as_ref(ctx).honor_ps1.value();
            initialize_local_defaults(true, ctx);
            assert_eq!(
                *InputSettings::as_ref(ctx).input_box_type.value(),
                InputBoxType::Classic
            );
            assert_eq!(*SessionSettings::as_ref(ctx).honor_ps1.value(), honor_ps1);
            assert_eq!(
                *ThemeSettings::as_ref(ctx).theme_kind.value(),
                ThemeKind::Phenomenon
            );
        });
    });
}
