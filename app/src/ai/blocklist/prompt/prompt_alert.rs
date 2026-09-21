use ai::api_keys::ApiKeyManager;
use markdown_parser::{FormattedText, FormattedTextFragment, FormattedTextLine};
use warp_core::ui::appearance::Appearance;
use warpui::elements::{
    ConstrainedBox, Container, CrossAxisAlignment, Flex, FormattedTextElement,
    HighlightedHyperlink, HyperlinkLens, MainAxisAlignment, MainAxisSize, ParentElement,
};
use warpui::{
    AppContext, Element, Entity, SingletonEntity, TypedActionView, View, ViewContext,
    WeakViewHandle,
};

use crate::ai::blocklist::error_color;
use crate::network::NetworkStatus;
use crate::server::ids::ServerId;
use crate::ui_components::icons::Icon;
use crate::workspace::WorkspaceAction;
use crate::workspaces::user_workspaces::{TeamScope, UserWorkspaces};

const NO_CONNECTION_PRIMARY_TEXT: &str = "No internet connection";

/// Only the signup and billing links produced these, and both were credit
/// prompts. The variants stay until the account gating that still routes
/// `PromptAlertEvent` is removed.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptAlertAction {
    SignUpClickedForAnonymousUser,
    ManageBillingClicked { team_uid: ServerId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptAlertEvent {
    SignupAnonymousUser,
    OpenBillingPortal { team_uid: ServerId },
}

/// The alert state of the chip that appears to the right of certain parts of the prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptAlertState {
    /// The user is offline (no connection).
    NoConnection,
    /// No alert should be displayed.
    NoAlert,
}

pub struct PromptAlertView {
    view_handle: WeakViewHandle<Self>,
    state: PromptAlertState,
    action_hyperlink: HighlightedHyperlink,
}

impl PromptAlertView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let user_workspaces = UserWorkspaces::handle(ctx);
        let network_status = NetworkStatus::handle(ctx);
        let api_key_manager = ApiKeyManager::handle(ctx);

        ctx.subscribe_to_model(&user_workspaces, |me, _, _, ctx| {
            me.state =
                Self::determine_state(&UserWorkspaces::as_ref(ctx).team_context_for_view(ctx), ctx);
            ctx.notify();
        });

        ctx.subscribe_to_model(&network_status, |me, _, _, ctx| {
            me.state =
                Self::determine_state(&UserWorkspaces::as_ref(ctx).team_context_for_view(ctx), ctx);
            ctx.notify();
        });

        ctx.subscribe_to_model(&api_key_manager, |me, _, _, ctx| {
            me.state =
                Self::determine_state(&UserWorkspaces::as_ref(ctx).team_context_for_view(ctx), ctx);
            ctx.notify();
        });

        let state = {
            let user_workspaces = UserWorkspaces::as_ref(ctx);
            let scope = user_workspaces.team_context_for_view(ctx);
            Self::determine_state(&scope, ctx)
        };

        Self {
            view_handle: ctx.handle(),
            state,
            action_hyperlink: Default::default(),
        }
    }

    pub fn determine_state<S: TeamScope + ?Sized>(scope: &S, app: &AppContext) -> PromptAlertState {
        let _ = scope;
        // Offline is the only condition left that blocks a request. Credit
        // limits, delinquency and spend caps all came from the account.
        if !NetworkStatus::as_ref(app).is_online() {
            return PromptAlertState::NoConnection;
        }
        PromptAlertState::NoAlert
    }

    pub fn is_no_alert(&self) -> bool {
        matches!(self.state, PromptAlertState::NoAlert)
    }

    pub fn state(&self) -> &PromptAlertState {
        &self.state
    }

    pub fn does_alert_block_ai_requests<S: TeamScope + ?Sized>(
        scope: &S,
        app: &AppContext,
    ) -> bool {
        does_alert_block_ai_requests(&Self::determine_state(scope, app))
    }

    fn primary_text(
        &self,
        state: &PromptAlertState,
        text_fragments: &mut Vec<FormattedTextFragment>,
    ) {
        // Add leading space to separate text from icon.
        //
        // Use this instead of hardcoded margin so it scales with font size and is consistent
        // with the space between this primary fragment and the option hyperlink fragment.
        text_fragments.push(FormattedTextFragment::plain_text("  "));
        match state {
            PromptAlertState::NoConnection => {
                text_fragments.push(FormattedTextFragment::plain_text(
                    NO_CONNECTION_PRIMARY_TEXT,
                ));
            }
            PromptAlertState::NoAlert => {}
        }
    }
}

fn does_alert_block_ai_requests(state: &PromptAlertState) -> bool {
    match state {
        PromptAlertState::NoAlert => false,
        PromptAlertState::NoConnection => true,
    }
}

impl Entity for PromptAlertView {
    type Event = PromptAlertEvent;
}

impl View for PromptAlertView {
    fn ui_name() -> &'static str {
        "PromptAlertView"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let workspaces = UserWorkspaces::as_ref(app);
        let scope = workspaces.team_context(&self.view_handle, app);
        let state = Self::determine_state(&scope, app);
        let mut text_fragments = vec![];

        self.primary_text(&state, &mut text_fragments);

        let formatted_text_element = FormattedTextElement::new(
            FormattedText::new([FormattedTextLine::Line(text_fragments)]),
            appearance.ui_font_size(),
            appearance.ui_font_family(),
            appearance.ui_font_family(),
            error_color(appearance.theme()),
            self.action_hyperlink.clone(),
        )
        .with_line_height_ratio(1.)
        .with_hyperlink_font_color(appearance.theme().ansi_fg_blue())
        .with_no_text_wrapping()
        .register_default_click_handlers_with_action_support(|hyperlink_lens, event, ctx| {
            match hyperlink_lens {
                HyperlinkLens::Url(url) => {
                    ctx.open_url(url);
                }
                HyperlinkLens::Action(action_ref) => {
                    if let Some(action) = action_ref.as_any().downcast_ref::<PromptAlertAction>() {
                        event.dispatch_typed_action(action.clone());
                    } else if let Some(action) =
                        action_ref.as_any().downcast_ref::<WorkspaceAction>()
                    {
                        event.dispatch_typed_action(action.clone());
                    }
                }
            }
        })
        .finish();

        let icon_size = appearance.ui_font_size();

        let mut chip_row = Flex::row()
            .with_main_axis_alignment(MainAxisAlignment::End)
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::End);
        if does_alert_block_ai_requests(&self.state) {
            chip_row.add_child(
                ConstrainedBox::new(
                    Icon::AlertTriangle
                        .to_warpui_icon(error_color(appearance.theme()).into())
                        .finish(),
                )
                .with_width(icon_size)
                .with_height(icon_size)
                .finish(),
            )
        }

        chip_row.add_child(formatted_text_element);

        Container::new(chip_row.finish())
            .with_margin_right(16.)
            .finish()
    }
}

impl TypedActionView for PromptAlertView {
    type Action = PromptAlertAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            PromptAlertAction::SignUpClickedForAnonymousUser => {
                ctx.emit(PromptAlertEvent::SignupAnonymousUser);
            }
            PromptAlertAction::ManageBillingClicked { team_uid } => {
                ctx.emit(PromptAlertEvent::OpenBillingPortal {
                    team_uid: *team_uid,
                });
            }
        }
    }
}

#[cfg(test)]
#[path = "prompt_alert_tests.rs"]
mod tests;
