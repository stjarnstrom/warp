use remote_server::auth::RemoteServerAuthContext;
use warpui::AppContext;

pub fn local_auth_context(
    crash_reporting_enabled: bool,
    ctx: &AppContext,
) -> RemoteServerAuthContext {
    let identity = crate::local_identity::get_or_create_anonymous_id(ctx).to_string();
    RemoteServerAuthContext::new(move || identity.clone(), crash_reporting_enabled)
}
