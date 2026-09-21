//! The closing step of first run.
//!
//! Upstream ended onboarding on a "Create an account" slide, with Skip behind a
//! confirmation dialog and a browser sign-in step after Continue. This fork has
//! no account, so the last thing a new user sees is one button into the
//! terminal.

use ui_components::{Component as _, Options as _, button};
use warp_core::send_telemetry_from_ctx;
use warp_core::ui::appearance::Appearance;
use warp_core::ui::theme::color::internal_colors;
use warpui_core::elements::{
    ClippedScrollStateHandle, Container, CrossAxisAlignment, Flex, FormattedTextElement,
    MainAxisSize, ParentElement,
};
use warpui_core::fonts::Weight;
use warpui_core::keymap::Keystroke;
use warpui_core::text_layout::TextAlignment;
use warpui_core::ui_components::components::{UiComponent as _, UiComponentStyles};
use warpui_core::{
    AppContext, Element, Entity, ModelHandle, SingletonEntity as _, TypedActionView, View,
    ViewContext,
};

use super::OnboardingSlide;
use crate::model::OnboardingStateModel;
use crate::slides::theme_picker_slide::theme_visual_path_for;
use crate::slides::{bottom_nav, layout, slide_content};
use crate::telemetry::OnboardingEvent;

#[derive(Debug, Clone)]
pub enum ReadySlideAction {
    BackClicked,
    FinishClicked,
}

pub struct ReadySlide {
    onboarding_state: ModelHandle<OnboardingStateModel>,
    back_button: button::Button,
    finish_button: button::Button,
    scroll_state: ClippedScrollStateHandle,
}

impl ReadySlide {
    pub(crate) fn new(onboarding_state: ModelHandle<OnboardingStateModel>) -> Self {
        Self {
            onboarding_state,
            back_button: button::Button::default(),
            finish_button: button::Button::default(),
            scroll_state: ClippedScrollStateHandle::default(),
        }
    }

    fn finish(&mut self, ctx: &mut ViewContext<Self>) {
        send_telemetry_from_ctx!(
            OnboardingEvent::OnboardingAction {
                slide_name: "ready".to_string(),
                action: "finish".to_string(),
                account_class: None,
            },
            ctx
        );
        self.onboarding_state.update(ctx, |model, ctx| {
            model.complete(ctx);
        });
    }

    fn back(&mut self, ctx: &mut ViewContext<Self>) {
        self.onboarding_state.update(ctx, |model, ctx| {
            model.back(ctx);
        });
    }

    fn render_content(&self, appearance: &Appearance, app: &AppContext) -> Box<dyn Element> {
        slide_content::onboarding_slide_content(
            vec![self.render_header_text(appearance)],
            self.render_bottom_nav(appearance, app),
            self.scroll_state.clone(),
            appearance,
        )
    }

    fn render_header_text(&self, appearance: &Appearance) -> Box<dyn Element> {
        let title = appearance
            .ui_builder()
            .paragraph("You're all set")
            .with_style(UiComponentStyles {
                font_size: Some(36.),
                font_weight: Some(Weight::Medium),
                ..Default::default()
            })
            .build()
            .finish();

        let subtitle = FormattedTextElement::from_str(
            "Everything runs on this machine. Change any of it later in Settings.",
            appearance.ui_font_family(),
            16.,
        )
        .with_color(internal_colors::text_sub(
            appearance.theme(),
            appearance.theme().background().into_solid(),
        ))
        .with_weight(Weight::Normal)
        .with_alignment(TextAlignment::Left)
        .with_line_height_ratio(1.0)
        .finish();

        Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_cross_axis_alignment(CrossAxisAlignment::Start)
            .with_child(title)
            .with_child(Container::new(subtitle).with_margin_top(16.).finish())
            .finish()
    }

    fn render_bottom_nav(&self, appearance: &Appearance, app: &AppContext) -> Box<dyn Element> {
        let back_button = self.back_button.render(
            appearance,
            button::Params {
                content: button::Content::Label("Back".into()),
                theme: &button::themes::Naked,
                options: button::Options {
                    on_click: Some(Box::new(|ctx, _app, _pos| {
                        ctx.dispatch_typed_action(ReadySlideAction::BackClicked);
                    })),
                    ..button::Options::default(appearance)
                },
            },
        );

        let enter = Keystroke::parse("enter").unwrap_or_default();
        let finish_button = self.finish_button.render(
            appearance,
            button::Params {
                content: button::Content::Label("Let's get to work".into()),
                theme: &button::themes::Primary,
                options: button::Options {
                    keystroke: Some(enter),
                    on_click: Some(Box::new(|ctx, _app, _pos| {
                        ctx.dispatch_typed_action(ReadySlideAction::FinishClicked);
                    })),
                    ..button::Options::default(appearance)
                },
            },
        );

        let (step_index, step_count) = self.onboarding_state.as_ref(app).progress();

        bottom_nav::onboarding_bottom_nav(
            appearance,
            step_index,
            step_count,
            Some(back_button),
            Some(finish_button),
        )
    }

    /// The same preview the theme step showed, so the closing slide does not
    /// blank the right-hand column on the way out.
    fn render_visual(&self, app: &AppContext) -> Box<dyn Element> {
        let state = self.onboarding_state.as_ref(app);
        let theme_name = Appearance::as_ref(app)
            .theme()
            .name()
            .unwrap_or_else(|| "Dark".to_string());
        let path = theme_visual_path_for(
            state.intention(),
            &theme_name,
            state.ui_customization().use_vertical_tabs,
        );
        layout::onboarding_right_panel_with_bg(path, layout::FOREGROUND_LAYOUT_DEFAULT)
    }
}

impl Entity for ReadySlide {
    type Event = ();
}

impl View for ReadySlide {
    fn ui_name() -> &'static str {
        "ReadySlide"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);

        // Background is rendered by the parent onboarding view (including background images).
        layout::static_left(
            || self.render_content(appearance, app),
            || self.render_visual(app),
        )
    }
}

impl OnboardingSlide for ReadySlide {
    fn on_enter(&mut self, ctx: &mut ViewContext<Self>) {
        self.finish(ctx);
    }
}

impl TypedActionView for ReadySlide {
    type Action = ReadySlideAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            ReadySlideAction::BackClicked => self.back(ctx),
            ReadySlideAction::FinishClicked => self.finish(ctx),
        }
    }
}
