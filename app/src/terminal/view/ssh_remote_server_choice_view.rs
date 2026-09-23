//! Inline block view that asks the user whether they want to install
//! Warp's SSH extension on the remote host the shell just connected to,
//! or continue without installing (falling back to the existing
//! ControlMaster warpification path).
//!
//! Designed from frame 6050:2448 of the Figma file
//! [Remote session initialization](https://www.figma.com/design/r0BO9cTZCK6pDE6qerg2K0/Remote-session-initialization).
//!
//! The view owns:
//! - the two selectable cards ("Install Warp's SSH extension" / "Continue without installing"),
//! - the [`SessionId`] this prompt is scoped to (used for event forwarding),
//! - the current "Don't ask me this again" checked state (purely local to
//!   this prompt instance; persisted to `ssh_extension_install_mode` only
//!   when the user clicks Install or Skip).
//!
//! Dismissing the block (on click of either option, or when the session is
//! deregistered) is the parent's responsibility.
use settings::Setting;
use warp_core::ui::theme::color::internal_colors;
use warp_errors::report_error;
use warpui::elements::{
    Border, Container, CornerRadius, CrossAxisAlignment, Flex, Hoverable, MainAxisAlignment,
    MainAxisSize, MouseStateHandle, ParentElement, Radius, Text,
};
use warpui::fonts::{Properties, Weight};
use warpui::platform::Cursor;
use warpui::ui_components::components::{UiComponent, UiComponentStyles};
use warpui::{AppContext, Element, Entity, SingletonEntity, TypedActionView, View, ViewContext};

use crate::server::telemetry::TelemetryEvent;
use crate::terminal::model::session::SessionId;
use crate::terminal::warpify::settings::{SshExtensionInstallMode, WarpifySettings};
use crate::ui_components::blended_colors;
use crate::{Appearance, send_telemetry_from_ctx};

const PROMPT_BORDER_RADIUS: f32 = 8.;
const HORIZONTAL_PADDING: f32 = 16.;

#[derive(Clone, Debug)]
pub enum SshRemoteServerChoiceViewAction {
    Install,
    Skip,
    ToggleDoNotAskAgain,
    OpenWarpifySettings,
}

#[derive(Clone, Debug)]
pub enum SshRemoteServerChoiceViewEvent {
    Install,
    Skip,
    OpenWarpifySettings,
}

/// Choice block prompting the user to install the remote-server binary on the remote host or skip.
pub struct SshRemoteServerChoiceView {
    session_id: SessionId,
    install_mouse_state: MouseStateHandle,
    skip_mouse_state: MouseStateHandle,
    do_not_ask_again_mouse_state: MouseStateHandle,
    do_not_ask_again_label_mouse_state: MouseStateHandle,
    manage_settings_mouse_state: MouseStateHandle,
    /// Current checked state of the "Don't ask me this again" checkbox.
    do_not_ask_again: bool,
}

impl SshRemoteServerChoiceView {
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            install_mouse_state: MouseStateHandle::default(),
            skip_mouse_state: MouseStateHandle::default(),
            do_not_ask_again_mouse_state: MouseStateHandle::default(),
            do_not_ask_again_label_mouse_state: MouseStateHandle::default(),
            manage_settings_mouse_state: MouseStateHandle::default(),
            do_not_ask_again: false,
        }
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    fn render_header(&self, appearance: &Appearance) -> Box<dyn Element> {
        Container::new(
            Text::new(
                "Choose your experience for this remote session:",
                appearance.ui_font_family(),
                appearance.monospace_font_size(),
            )
            .with_style(Properties::default().weight(Weight::Semibold))
            .with_color(appearance.theme().foreground().into())
            .finish(),
        )
        .with_horizontal_padding(HORIZONTAL_PADDING)
        .with_padding_top(10.)
        .finish()
    }

    fn render_option(
        title: &'static str,
        sub_label: &'static str,
        mouse_state: MouseStateHandle,
        action: SshRemoteServerChoiceViewAction,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let ui_font_family = appearance.ui_font_family();
        let font_size = appearance.monospace_font_size();
        let title_color = theme.foreground().into_solid();
        let sub_label_color = internal_colors::neutral_5(theme);
        let hover_background = internal_colors::fg_overlay_2(theme);
        Hoverable::new(mouse_state, move |state| {
            let content = Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_child(
                    Text::new(title, ui_font_family, font_size)
                        .soft_wrap(true)
                        .with_color(title_color)
                        .finish(),
                )
                .with_child(
                    Container::new(
                        Text::new(sub_label, ui_font_family, font_size - 2.)
                            .soft_wrap(true)
                            .with_color(sub_label_color)
                            .finish(),
                    )
                    .with_margin_top(4.)
                    .finish(),
                )
                .finish();
            let mut container = Container::new(content)
                .with_uniform_padding(8.)
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)));
            if state.is_hovered() {
                container = container.with_background(hover_background);
            }
            container.finish()
        })
        .with_cursor(Cursor::PointingHand)
        .on_click(move |ctx, _, _| ctx.dispatch_typed_action(action.clone()))
        .finish()
    }

    fn render_buttons(&self, appearance: &Appearance) -> Box<dyn Element> {
        let options = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(Self::render_option(
                "Install Warp's SSH extension",
                "Install Warp's extension to enable file browsing, code review, and intelligent \
                 command completions in this session.",
                self.install_mouse_state.clone(),
                SshRemoteServerChoiceViewAction::Install,
                appearance,
            ))
            .with_child(
                Container::new(Self::render_option(
                    "Continue without installing",
                    "You'll still get a Warpified experience just without the coding features.",
                    self.skip_mouse_state.clone(),
                    SshRemoteServerChoiceViewAction::Skip,
                    appearance,
                ))
                .with_margin_top(4.)
                .finish(),
            )
            .finish();
        Container::new(options)
            .with_uniform_padding(HORIZONTAL_PADDING)
            .finish()
    }

    fn render_footer(&self, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        let muted_color = internal_colors::neutral_5(theme);
        let accent_color = theme.accent().into_solid();
        let ui_font_family = appearance.ui_font_family();
        let footer_font_size = appearance.monospace_font_size() - 2.;

        let checkbox = appearance
            .ui_builder()
            .checkbox(
                self.do_not_ask_again_mouse_state.clone(),
                Some(footer_font_size),
            )
            .check(self.do_not_ask_again)
            .build()
            .on_click(|ctx, _, _| {
                ctx.dispatch_typed_action(SshRemoteServerChoiceViewAction::ToggleDoNotAskAgain);
            })
            .finish();

        let checkbox_label =
            Hoverable::new(self.do_not_ask_again_label_mouse_state.clone(), move |_| {
                Text::new("Don't ask me this again", ui_font_family, footer_font_size)
                    .with_color(muted_color)
                    .finish()
            })
            .with_cursor(Cursor::PointingHand)
            .on_click(|ctx, _, _| {
                ctx.dispatch_typed_action(SshRemoteServerChoiceViewAction::ToggleDoNotAskAgain);
            })
            .finish();

        let checkbox_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(checkbox)
            .with_child(Container::new(checkbox_label).with_margin_left(4.).finish())
            .finish();

        // Right: "Manage Warpify settings" link.
        let manage_settings_link = appearance
            .ui_builder()
            .link(
                "Manage Warpify settings".into(),
                None,
                Some(Box::new(|ctx| {
                    ctx.dispatch_typed_action(SshRemoteServerChoiceViewAction::OpenWarpifySettings);
                })),
                self.manage_settings_mouse_state.clone(),
            )
            .soft_wrap(false)
            .with_style(UiComponentStyles {
                font_size: Some(footer_font_size),
                font_family_id: Some(ui_font_family),
                font_color: Some(accent_color),
                ..Default::default()
            })
            .build()
            .finish();

        let row = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(checkbox_row)
            .with_child(manage_settings_link)
            .finish();

        let border_color = blended_colors::neutral_2(theme);
        Container::new(row)
            .with_padding_top(8.)
            .with_padding_bottom(8.)
            .with_padding_left(HORIZONTAL_PADDING)
            .with_padding_right(HORIZONTAL_PADDING)
            .with_border(Border::top(1.).with_border_fill(border_color))
            .finish()
    }
}

impl Entity for SshRemoteServerChoiceView {
    type Event = SshRemoteServerChoiceViewEvent;
}

impl View for SshRemoteServerChoiceView {
    fn ui_name() -> &'static str {
        "SshRemoteServerChoiceView"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();

        let content = Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(self.render_header(appearance))
            .with_child(self.render_buttons(appearance))
            .with_child(self.render_footer(appearance))
            .finish();

        let border_color = blended_colors::neutral_2(theme);
        let card = Container::new(content)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(PROMPT_BORDER_RADIUS)))
            .with_background_color(theme.background().into_solid())
            .with_border(Border::all(1.).with_border_fill(border_color))
            .finish();

        Container::new(card)
            .with_padding_top(8.)
            .with_padding_bottom(16.)
            .with_padding_left(16.)
            .with_padding_right(16.)
            .finish()
    }
}

impl TypedActionView for SshRemoteServerChoiceView {
    type Action = SshRemoteServerChoiceViewAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            SshRemoteServerChoiceViewAction::Install => {
                if self.do_not_ask_again {
                    let mode = SshExtensionInstallMode::AlwaysInstall;
                    WarpifySettings::handle(ctx).update(ctx, |settings, ctx| {
                        if let Err(e) = settings.ssh_extension_install_mode.set_value(mode, ctx) {
                            report_error!(
                                e.context("Failed to persist ssh_extension_install_mode")
                            );
                        }
                    });
                    send_telemetry_from_ctx!(
                        TelemetryEvent::SetSshExtensionInstallMode {
                            mode: mode.display_name(),
                        },
                        ctx
                    );
                }
                ctx.emit(SshRemoteServerChoiceViewEvent::Install);
            }
            SshRemoteServerChoiceViewAction::Skip => {
                if self.do_not_ask_again {
                    let mode = SshExtensionInstallMode::NeverInstall;
                    WarpifySettings::handle(ctx).update(ctx, |settings, ctx| {
                        if let Err(e) = settings.ssh_extension_install_mode.set_value(mode, ctx) {
                            report_error!(
                                e.context("Failed to persist ssh_extension_install_mode")
                            );
                        }
                    });
                    send_telemetry_from_ctx!(
                        TelemetryEvent::SetSshExtensionInstallMode {
                            mode: mode.display_name(),
                        },
                        ctx
                    );
                }
                ctx.emit(SshRemoteServerChoiceViewEvent::Skip);
            }
            SshRemoteServerChoiceViewAction::ToggleDoNotAskAgain => {
                self.do_not_ask_again = !self.do_not_ask_again;
                send_telemetry_from_ctx!(
                    TelemetryEvent::SshRemoteServerChoiceDoNotAskAgainToggled {
                        checked: self.do_not_ask_again,
                    },
                    ctx
                );
                ctx.notify();
            }
            SshRemoteServerChoiceViewAction::OpenWarpifySettings => {
                ctx.emit(SshRemoteServerChoiceViewEvent::OpenWarpifySettings);
            }
        }
    }
}
