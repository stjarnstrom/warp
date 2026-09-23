use pathfinder_geometry::vector::vec2f;
use warp_core::ui::appearance::Appearance;
use warp_core::ui::color::blend::Blend as _;
use warp_core::ui::{self};
use warpui::elements::{
    Align, ChildView, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Flex, Icon,
    MainAxisAlignment, MainAxisSize, MouseStateHandle, ParentElement as _, Radius,
};
use warpui::keymap::EditableBinding;
use warpui::platform::Cursor;
use warpui::ui_components::button::{ButtonVariant, TextAndIcon, TextAndIconAlignment};
use warpui::ui_components::components::{Coords, UiComponent, UiComponentStyles};
use warpui::{
    AppContext, Element, Entity, ModelHandle, SingletonEntity as _, TypedActionView, View,
    ViewContext, ViewHandle,
};

use crate::coding_entrypoints::project_buttons::{ProjectButtons, ProjectButtonsEvent};
use crate::pane_group::focus_state::PaneFocusHandle;
use crate::pane_group::pane::view;
use crate::pane_group::{BackingView, PaneConfiguration, PaneEvent};
use crate::util::bindings::{BindingGroup, CustomAction, keybinding_name_to_display_string};
use crate::view_components::DismissibleToast;
use crate::workspace::{ToastStack, WorkspaceAction};
use crate::{TelemetryEvent, send_telemetry_from_ctx};

pub fn init(app: &mut AppContext) {
    use warpui::keymap::macros::*;

    app.register_editable_bindings([EditableBinding::new(
        "workspace:new_tab",
        "Terminal session",
        GetStartedAction::TerminalSession,
    )
    .with_context_predicate(id!("GetStartedView"))
    .with_group(BindingGroup::Terminal.as_str())
    .with_custom_action(CustomAction::NewTab)]);
}

pub struct GetStartedView {
    pane_configuration: ModelHandle<PaneConfiguration>,
    focus_handle: Option<PaneFocusHandle>,
    project_buttons: ViewHandle<ProjectButtons>,
    terminal_session_button: MouseStateHandle,
}

impl GetStartedView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let pane_configuration = ctx.add_model(|_ctx| PaneConfiguration::new("Get started"));
        let project_buttons = ctx.add_typed_action_view(ProjectButtons::new);
        ctx.subscribe_to_view(&project_buttons, Self::handle_project_buttons_event);

        Self {
            pane_configuration,
            focus_handle: None,
            project_buttons,
            terminal_session_button: Default::default(),
        }
    }

    fn handle_project_buttons_event(
        &mut self,
        _: ViewHandle<ProjectButtons>,
        event: &ProjectButtonsEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        match event {
            ProjectButtonsEvent::OpenRepository(path_result) => match path_result {
                Ok(path) => {
                    send_telemetry_from_ctx!(
                        TelemetryEvent::OpenRepoFolderSubmitted { is_ftux: true },
                        ctx
                    );
                    ctx.dispatch_typed_action(&WorkspaceAction::OpenRepository {
                        path: Some(path.clone()),
                    });
                    self.close(ctx);
                }
                Err(err) => {
                    let window_id = ctx.window_id();
                    ToastStack::handle(ctx).update(ctx, |toast_stack, ctx| {
                        toast_stack.add_ephemeral_toast(
                            DismissibleToast::error(format!("{err}")),
                            window_id,
                            ctx,
                        );
                    });
                }
            },
        }
    }

    pub fn pane_configuration(&self) -> ModelHandle<PaneConfiguration> {
        self.pane_configuration.clone()
    }

    fn render_main_content(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();

        Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_children([
                Container::new(
                    ConstrainedBox::new(
                        Icon::new("bundled/svg/warp-logo-neutral.svg", theme.foreground()).finish(),
                    )
                    .with_height(40.)
                    .with_width(40.)
                    .finish(),
                )
                .with_margin_bottom(12.)
                .finish(),
                appearance
                    .ui_builder()
                    .paragraph("Welcome to Warp")
                    .with_style(UiComponentStyles {
                        font_size: Some(20.),
                        ..Default::default()
                    })
                    .build()
                    .finish(),
                Container::new(
                    ConstrainedBox::new(ChildView::new(&self.project_buttons).finish())
                        .with_max_width(480.)
                        .with_max_height(70.)
                        .finish(),
                )
                .with_vertical_margin(16.)
                .finish(),
                appearance
                    .ui_builder()
                    .button(ButtonVariant::Text, self.terminal_session_button.clone())
                    .with_style(UiComponentStyles {
                        padding: Some(Coords::uniform(8.)),
                        ..Default::default()
                    })
                    .with_hovered_styles(UiComponentStyles {
                        border_radius: Some(CornerRadius::with_all(Radius::Pixels(4.))),
                        background: Some(
                            theme.background().blend(&theme.surface_overlay_1()).into(),
                        ),
                        ..Default::default()
                    })
                    .with_text_and_icon_label(TextAndIcon::new(
                        TextAndIconAlignment::IconFirst,
                        format!(
                            " New session in {}  {}",
                            dirs::home_dir()
                                .map(|dir| dir.display().to_string())
                                .unwrap_or("~".to_string()),
                            keybinding_name_to_display_string("workspace:new_tab", app)
                                .unwrap_or_default()
                        ),
                        ui::Icon::Terminal.to_warpui_icon(theme.foreground()),
                        MainAxisSize::Min,
                        MainAxisAlignment::Center,
                        vec2f(16., 16.),
                    ))
                    .build()
                    .on_click(|ctx, _, _| {
                        ctx.dispatch_typed_action(GetStartedAction::TerminalSession)
                    })
                    .with_cursor(Cursor::PointingHand)
                    .finish(),
            ])
            .finish()
    }
}

impl Entity for GetStartedView {
    type Event = PaneEvent;
}

#[derive(Debug)]
pub enum GetStartedAction {
    TerminalSession,
}

impl TypedActionView for GetStartedView {
    type Action = GetStartedAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            GetStartedAction::TerminalSession => {
                send_telemetry_from_ctx!(TelemetryEvent::GetStartedSkipToTerminal, ctx);
                ctx.dispatch_typed_action(&WorkspaceAction::AddTerminalTab {
                    hide_homepage: true,
                });
                self.close(ctx);
            }
        }
    }
}

impl View for GetStartedView {
    fn ui_name() -> &'static str {
        "GetStartedView"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        Align::new(self.render_main_content(app)).finish()
    }
}

impl BackingView for GetStartedView {
    type PaneHeaderOverflowMenuAction = ();
    type CustomAction = ();
    type AssociatedData = ();

    fn handle_pane_header_overflow_menu_action(
        &mut self,
        _action: &Self::PaneHeaderOverflowMenuAction,
        _ctx: &mut ViewContext<Self>,
    ) {
        // TODO
    }

    fn close(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(PaneEvent::Close);
    }

    fn focus_contents(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.focus(&self.project_buttons);
    }

    fn render_header_content(
        &self,
        _ctx: &view::HeaderRenderContext,
        _app: &AppContext,
    ) -> view::HeaderContent {
        view::HeaderContent::simple("Get started")
    }

    fn set_focus_handle(&mut self, focus_handle: PaneFocusHandle, _ctx: &mut ViewContext<Self>) {
        self.focus_handle = Some(focus_handle);
    }
}
