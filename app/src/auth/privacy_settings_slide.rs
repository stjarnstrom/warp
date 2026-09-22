//! The privacy-settings step of first run.
//!
//! Upstream reached this view as a login slide: it offered to create a Warp
//! account after the theme step, with a browser sign-up flow, a token paste
//! box, and a confirmation dialog for skipping. This fork asks for no account,
//! so the only way in is the "Privacy Settings" link on the theme slide, and
//! the only thing left is the toggles that link points at.

use onboarding::OnboardingIntention;
use onboarding::slides::{layout, slide_content};
use ui_components::{Component as _, Options as _, button};
use warp_core::ui::theme::color::internal_colors;
use warpui::elements::{
    CacheOption, ClippedScrollStateHandle, Container, Flex, FormattedTextElement, Image,
    MainAxisSize, ParentElement, Shrinkable, Stack,
};
use warpui::fonts::Weight;
use warpui::keymap::FixedBinding;
use warpui::text_layout::TextAlignment;
use warpui::{
    AppContext, Element, Entity, FocusContext, SingletonEntity, TypedActionView, UpdateModel, View,
    ViewContext,
};

use crate::appearance::Appearance;
use crate::auth::auth_view_shared_helpers::{
    PrivacySettingsActions, PrivacySettingsHandles, render_privacy_settings_toggles,
};
use crate::settings::PrivacySettings;

// ---------------------------------------------------------------------------
// Init (keybindings)
// ---------------------------------------------------------------------------

pub fn init(app: &mut AppContext) {
    use warpui::keymap::macros::*;

    app.register_fixed_bindings([FixedBinding::new(
        "escape",
        PrivacySettingsSlideAction::Back,
        id!(PrivacySettingsSlideView::ui_name()),
    )]);
}

#[derive(Clone, Debug)]
pub enum PrivacySettingsSlideAction {
    Back,
    ToggleTelemetry,
    ToggleCrashReporting,
    ToggleCloudConversationStorage,
}

#[derive(Clone, Debug)]
pub enum PrivacySettingsSlideEvent {
    BackToOnboarding,
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub struct PrivacySettingsSlideView {
    /// Whether this path wants AI (agent intent) vs. not (terminal intention).
    /// Used to gate the cloud-conversation-storage toggle and AI wording.
    ai_enabled: bool,
    theme_visual_path: &'static str,
    back_button: button::Button,
    privacy_settings_handles: PrivacySettingsHandles,
    scroll_state: ClippedScrollStateHandle,
}

/// All image paths used by the slide visual. These mirror the set in
/// `ThemePickerSlide::VISUAL_IMAGE_PATHS` so this slide keeps showing the same
/// themed right panel the user was looking at on the theme slide.
const VISUAL_IMAGE_PATHS: &[&str] = &[
    // Terminal intention
    "async/png/onboarding/terminal_intention/theme/theme_phenomenon_vertical.png",
    "async/png/onboarding/terminal_intention/theme/theme_phenomenon_horizontal.png",
    "async/png/onboarding/terminal_intention/theme/theme_dark_vertical.png",
    "async/png/onboarding/terminal_intention/theme/theme_dark_horizontal.png",
    "async/png/onboarding/terminal_intention/theme/theme_light_vertical.png",
    "async/png/onboarding/terminal_intention/theme/theme_light_horizontal.png",
    "async/png/onboarding/terminal_intention/theme/theme_adeberry_vertical.png",
    "async/png/onboarding/terminal_intention/theme/theme_adeberry_horizontal.png",
    // Agent intention
    "async/png/onboarding/agent_intention/theme/theme_phenomenon_vertical.png",
    "async/png/onboarding/agent_intention/theme/theme_phenomenon_horizontal.png",
    "async/png/onboarding/agent_intention/theme/theme_dark_vertical.png",
    "async/png/onboarding/agent_intention/theme/theme_dark_horizontal.png",
    "async/png/onboarding/agent_intention/theme/theme_light_vertical.png",
    "async/png/onboarding/agent_intention/theme/theme_light_horizontal.png",
    "async/png/onboarding/agent_intention/theme/theme_adeberry_vertical.png",
    "async/png/onboarding/agent_intention/theme/theme_adeberry_horizontal.png",
];

fn resolve_visual_path(
    intention: OnboardingIntention,
    theme_name: &str,
    use_vertical_tabs: bool,
) -> &'static str {
    let intention_dir = match intention {
        OnboardingIntention::AgentDrivenDevelopment => "agent_intention",
        OnboardingIntention::Terminal => "terminal_intention",
    };
    let name_key = match theme_name {
        "Phenomenon" => "phenomenon",
        "Dark" => "dark",
        "Light" => "light",
        "Adeberry" => "adeberry",
        _ => "dark",
    };
    let orientation = if use_vertical_tabs {
        "vertical"
    } else {
        "horizontal"
    };
    VISUAL_IMAGE_PATHS
        .iter()
        .find(|p| p.contains(intention_dir) && p.contains(name_key) && p.contains(orientation))
        .unwrap_or(&VISUAL_IMAGE_PATHS[0])
}

impl PrivacySettingsSlideView {
    pub fn new(
        ai_enabled: bool,
        theme_name: &str,
        use_vertical_tabs: bool,
        intention: OnboardingIntention,
        _ctx: &mut ViewContext<Self>,
    ) -> Self {
        Self {
            ai_enabled,
            theme_visual_path: resolve_visual_path(intention, theme_name, use_vertical_tabs),
            back_button: button::Button::default(),
            privacy_settings_handles: PrivacySettingsHandles::default(),
            scroll_state: ClippedScrollStateHandle::new(),
        }
    }

    fn render_content(&self, appearance: &Appearance, app: &AppContext) -> Box<dyn Element> {
        slide_content::onboarding_slide_content(
            self.render_privacy_settings_content(appearance, app),
            self.render_bottom_nav(appearance),
            self.scroll_state.clone(),
            appearance,
        )
    }

    fn render_privacy_settings_content(
        &self,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Vec<Box<dyn Element>> {
        let theme = appearance.theme();

        let title =
            FormattedTextElement::from_str("Privacy Settings", appearance.ui_font_family(), 36.)
                .with_color(internal_colors::text_main(
                    theme,
                    theme.background().into_solid(),
                ))
                .with_weight(Weight::Medium)
                .with_alignment(TextAlignment::Left)
                .finish();

        let actions = PrivacySettingsActions {
            toggle_telemetry: PrivacySettingsSlideAction::ToggleTelemetry,
            toggle_crash_reporting: PrivacySettingsSlideAction::ToggleCrashReporting,
            toggle_cloud_conversation_storage:
                PrivacySettingsSlideAction::ToggleCloudConversationStorage,
            hide_overlay: PrivacySettingsSlideAction::Back,
        };

        let toggles = render_privacy_settings_toggles(
            appearance,
            app,
            &self.privacy_settings_handles,
            &actions,
            self.ai_enabled,
        );

        vec![title, Container::new(toggles).with_margin_top(24.).finish()]
    }

    fn render_bottom_nav(&self, appearance: &Appearance) -> Box<dyn Element> {
        let back_button = self.back_button.render(
            appearance,
            button::Params {
                content: button::Content::Label("Back".into()),
                theme: &button::themes::Naked,
                options: button::Options {
                    on_click: Some(Box::new(|ctx, _app, _pos| {
                        ctx.dispatch_typed_action(PrivacySettingsSlideAction::Back);
                    })),
                    ..button::Options::default(appearance)
                },
            },
        );

        Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_child(back_button)
            .finish()
    }

    fn render_visual(&self) -> Box<dyn Element> {
        layout::onboarding_right_panel_with_bg(
            self.theme_visual_path,
            layout::FOREGROUND_LAYOUT_DEFAULT,
        )
    }
}

impl Entity for PrivacySettingsSlideView {
    type Event = PrivacySettingsSlideEvent;
}

impl View for PrivacySettingsSlideView {
    fn ui_name() -> &'static str {
        "PrivacySettingsSlideView"
    }

    fn on_focus(&mut self, focus_ctx: &FocusContext, ctx: &mut ViewContext<Self>) {
        if focus_ctx.is_self_focused() {
            ctx.notify();
        }
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();

        let mut stack = Stack::new();

        // Background (same as onboarding parent)
        match theme.background_image() {
            Some(img) => {
                stack.add_child(
                    Shrinkable::new(
                        1.,
                        Image::new(img.source(), CacheOption::Original)
                            .cover()
                            .finish(),
                    )
                    .finish(),
                );
                let overlay_opacity = (100u8).saturating_sub(img.opacity);
                stack.add_child(
                    warpui::elements::Rect::new()
                        .with_background(theme.background().with_opacity(overlay_opacity))
                        .finish(),
                );
            }
            _ => {
                stack.add_child(
                    Container::new(warpui::elements::Empty::new().finish())
                        .with_background(theme.background())
                        .finish(),
                );
            }
        }

        stack.add_child(layout::static_left(
            || self.render_content(appearance, app),
            || self.render_visual(),
        ));

        stack.finish()
    }
}

impl TypedActionView for PrivacySettingsSlideView {
    type Action = PrivacySettingsSlideAction;

    fn handle_action(&mut self, action: &PrivacySettingsSlideAction, ctx: &mut ViewContext<Self>) {
        match action {
            PrivacySettingsSlideAction::Back => {
                ctx.emit(PrivacySettingsSlideEvent::BackToOnboarding)
            }
            PrivacySettingsSlideAction::ToggleTelemetry => {
                let handle = PrivacySettings::handle(ctx);
                ctx.update_model(&handle, |settings, ctx| {
                    settings.set_is_telemetry_enabled(!settings.is_telemetry_enabled, ctx);
                });
                ctx.notify();
            }
            PrivacySettingsSlideAction::ToggleCrashReporting => {
                let handle = PrivacySettings::handle(ctx);
                ctx.update_model(&handle, |settings, ctx| {
                    settings
                        .set_is_crash_reporting_enabled(!settings.is_crash_reporting_enabled, ctx);
                });
                ctx.notify();
            }
            PrivacySettingsSlideAction::ToggleCloudConversationStorage => {
                let handle = PrivacySettings::handle(ctx);
                ctx.update_model(&handle, |settings, ctx| {
                    settings.set_is_cloud_conversation_storage_enabled(
                        !settings.is_cloud_conversation_storage_enabled,
                        ctx,
                    );
                });
                ctx.notify();
            }
        }
    }
}
