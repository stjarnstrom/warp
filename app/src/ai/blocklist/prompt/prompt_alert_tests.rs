use std::sync::Arc;

use warpui::App;

use super::*;
use crate::server::server_api::ServerApiProvider;
use crate::server::server_api::team::MockTeamClient;
use crate::server::server_api::workspace::MockWorkspaceClient;
use crate::server::telemetry::context_provider::AppTelemetryContextProvider;
use crate::workspaces::user_workspaces::TeamlessScopeForTest;

fn initialize_app(app: &mut App) {
    app.add_singleton_model(|_| NetworkStatus::new());
    app.add_singleton_model(|_| ServerApiProvider::new_for_test());
    app.add_singleton_model(AppTelemetryContextProvider::new_context_provider);
    app.add_singleton_model(|ctx| {
        UserWorkspaces::mock(
            Arc::new(MockTeamClient::new()),
            Arc::new(MockWorkspaceClient::new()),
            vec![],
            ctx,
        )
    });
}

fn determine_state(app: &mut App) -> PromptAlertState {
    app.read(|ctx| PromptAlertView::determine_state(&TeamlessScopeForTest, ctx))
}

/// Credits and plan limits used to drive this alert. Online is now the only
/// condition it reports on.
#[test]
fn an_online_session_has_no_alert() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        assert_eq!(determine_state(&mut app), PromptAlertState::NoAlert);
    });
}

#[test]
fn an_offline_session_says_so() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        NetworkStatus::handle(&app).update(&mut app, |status, ctx| {
            status.reachability_changed(false, ctx);
        });
        assert_eq!(determine_state(&mut app), PromptAlertState::NoConnection);
    });
}
