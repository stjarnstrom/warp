use warpui::elements::{Flex, MainAxisSize, ParentElement};
use warpui::ui_components::components::UiComponent;
use warpui::{AppContext, Element, Entity, SingletonEntity, View, ViewContext};

use super::style;
use crate::appearance::Appearance;

const SESSION_LIMIT_SUBHEADER: &str = "This account has reached its limit on shared sessions.";

pub struct DeniedBody;

pub enum DeniedBodyEvent {}

impl DeniedBody {
    pub fn new(_ctx: &mut ViewContext<Self>) -> Self {
        Self
    }
}

impl Entity for DeniedBody {
    type Event = DeniedBodyEvent;
}

impl View for DeniedBody {
    fn ui_name() -> &'static str {
        "ShareModalDeniedBody"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);

        let text = appearance
            .ui_builder()
            .wrappable_text(SESSION_LIMIT_SUBHEADER, true)
            .with_style(style::subheader_styles(appearance))
            .build()
            .finish();

        let mut col = Flex::column();
        col.add_child(text);
        col.with_main_axis_size(MainAxisSize::Min).finish()
    }
}
