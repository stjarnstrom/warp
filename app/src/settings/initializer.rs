use settings::Setting;
use warp_core::features::FeatureFlag;
use warp_errors::report_if_error;
use warpui::{AppContext, SingletonEntity};

use super::input::InputBoxType;
use super::{InputSettings, PrivacySettings, ThemeSettings};
use crate::terminal::session_settings::SessionSettings;
use crate::themes::theme::ThemeKind;

pub fn initialize_local_defaults(first_run: bool, ctx: &mut AppContext) {
    super::onboarding::initialize_local_onboarding(first_run, ctx);
    PrivacySettings::handle(ctx).update(ctx, |settings, ctx| {
        if first_run {
            settings.disable_default_regex_trigger(ctx);
        } else {
            settings.initialize_default_regexes_once(ctx);
        }
    });
    if !first_run {
        return;
    }

    if FeatureFlag::DefaultAdeberryTheme.is_enabled() {
        ThemeSettings::handle(ctx).update(ctx, |settings, ctx| {
            if !settings.theme_kind.is_value_explicitly_set()
                && *settings.theme_kind.value() == ThemeKind::Phenomenon
            {
                report_if_error!(settings.theme_kind.set_value(ThemeKind::Adeberry, ctx));
            }
        });
    }
    #[cfg(windows)]
    super::FontSettings::handle(ctx).update(ctx, |settings, ctx| {
        if !settings.monospace_font_size.is_value_explicitly_set() {
            report_if_error!(settings.monospace_font_size.set_value(16., ctx));
        }
    });

    let updated_input = InputSettings::handle(ctx).update(ctx, |settings, ctx| {
        if !settings.input_box_type.is_value_explicitly_set()
            && *settings.input_box_type.value() == InputBoxType::Classic
        {
            report_if_error!(
                settings
                    .input_box_type
                    .set_value(InputBoxType::Universal, ctx)
            );
            true
        } else {
            false
        }
    });
    if updated_input {
        SessionSettings::handle(ctx).update(ctx, |settings, ctx| {
            report_if_error!(settings.honor_ps1.set_value(false, ctx));
        });
    }
}

#[cfg(test)]
#[path = "initializer_tests.rs"]
mod tests;
