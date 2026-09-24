use warp_core::user_preferences::GetUserPreferences;
use warpui::AppContext;

const LOCAL_ONBOARDING_COMPLETED: &str = "LocalOnboardingCompleted";

pub fn initialize_local_onboarding(first_run: bool, ctx: &AppContext) {
    let preferences = ctx.private_user_preferences();
    if preferences
        .read_value(LOCAL_ONBOARDING_COMPLETED)
        .unwrap_or_default()
        .is_none()
    {
        let _ = preferences.write_value(LOCAL_ONBOARDING_COMPLETED, (!first_run).to_string());
    }
}

pub fn has_completed_local_onboarding(ctx: &AppContext) -> bool {
    ctx.private_user_preferences()
        .read_value(LOCAL_ONBOARDING_COMPLETED)
        .unwrap_or_default()
        .is_some_and(|value| value == "true")
}

pub fn mark_local_onboarding_completed(ctx: &AppContext) {
    let _ = ctx
        .private_user_preferences()
        .write_value(LOCAL_ONBOARDING_COMPLETED, "true".into());
}

#[cfg(test)]
#[path = "onboarding_tests.rs"]
mod tests;
