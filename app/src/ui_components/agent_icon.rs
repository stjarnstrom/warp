//! Derives the agent-icon shape ([`IconWithStatusVariant`]) for a terminal from its CLI agent
//! session, so every surface that renders the icon (vertical tabs, pane header, notifications)
//! shows the same brand color, glyph and status.
use warpui::{AppContext, SingletonEntity};

use crate::terminal::CLIAgent;
use crate::terminal::cli_agent_sessions::CLIAgentSessionsModel;
use crate::terminal::view::TerminalView;
use crate::ui_components::icon_with_status::IconWithStatusVariant;

/// Returns the agent-icon variant for a live [`TerminalView`], or `None` when the terminal has
/// no CLI agent session with a known agent.
///
/// Status is only surfaced when the session is plugin-backed and its handler exposes rich
/// status; command-detected sessions only know that an agent is running.
pub(crate) fn terminal_view_agent_icon_variant(
    terminal_view: &TerminalView,
    app: &AppContext,
) -> Option<IconWithStatusVariant> {
    let session = CLIAgentSessionsModel::as_ref(app).session(terminal_view.id())?;
    if matches!(session.agent, CLIAgent::Unknown) {
        return None;
    }
    let status = (session.listener.is_some() && session.supports_rich_status())
        .then(|| session.status.to_conversation_status());
    Some(IconWithStatusVariant::CLIAgent {
        agent: session.agent,
        status,
        is_ambient: false,
    })
}
