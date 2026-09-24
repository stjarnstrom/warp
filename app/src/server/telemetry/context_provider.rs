use warp_core::telemetry::{TelemetryContextModel, TelemetryContextProvider};
use warpui::{AppContext, ModelContext};

pub struct AppTelemetryContextProvider {}

impl AppTelemetryContextProvider {
    pub fn new_context_provider(
        _ctx: &mut ModelContext<TelemetryContextModel>,
    ) -> TelemetryContextModel {
        Box::new(Self {})
    }
}

impl TelemetryContextProvider for AppTelemetryContextProvider {
    fn user_id(&self, _ctx: &AppContext) -> Option<String> {
        None
    }

    fn anonymous_id(&self, ctx: &AppContext) -> String {
        crate::local_identity::get_or_create_anonymous_id(ctx).to_string()
    }
}
