use warp_core::ui::appearance::Appearance;
use warp_core::ui::color::coloru_with_opacity;
use warp_core::ui::theme::color::internal_colors;
use warp_core::ui::theme::{Fill, WarpTheme};
use warpui::Element;
use warpui::color::ColorU;
use warpui::elements::{ConstrainedBox, Container, CornerRadius, Radius};

use crate::ui_components::icons::Icon;

/// Padding around the status icon
pub const STATUS_ELEMENT_PADDING: f32 = 2.;

#[derive(Clone, Copy)]
pub enum StatusColorStyle {
    /// Foreground-blend colors (`ansi_fg`) used by the regular status badge.
    Standard,
    /// Background-blend colors (`ansi_bg`) used by the cloud overlay badge.
    Cloud,
}

/// The status of a CLI agent session, as shown on tabs and pane headers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConversationStatus {
    /// Agent is running.
    InProgress,

    /// The last turn of the agent finished with success.
    Success,

    /// The last turn of the agent completed with error.
    Error,

    /// The last turn of the agent was cancelled by the user.
    Cancelled,

    /// The agent is waiting on the user to approve an action.
    Blocked { blocked_action: String },
}

impl std::fmt::Display for ConversationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversationStatus::InProgress => write!(f, "In progress"),
            ConversationStatus::Success => write!(f, "Done"),
            ConversationStatus::Error => write!(f, "Error"),
            ConversationStatus::Cancelled => write!(f, "Cancelled"),
            ConversationStatus::Blocked { .. } => write!(f, "Blocked"),
        }
    }
}

impl ConversationStatus {
    pub fn status_icon_and_color(
        &self,
        theme: &WarpTheme,
        color_style: StatusColorStyle,
    ) -> (Icon, ColorU) {
        let pick = |standard: ColorU, cloud: ColorU| match color_style {
            StatusColorStyle::Standard => standard,
            StatusColorStyle::Cloud => cloud,
        };
        match self {
            ConversationStatus::InProgress => (
                Icon::ClockLoader,
                pick(theme.ansi_fg_magenta(), theme.ansi_bg_magenta()),
            ),
            ConversationStatus::Success => (
                Icon::Check,
                pick(theme.ansi_fg_green(), theme.ansi_bg_green()),
            ),
            ConversationStatus::Error => (
                Icon::Triangle,
                pick(theme.ansi_fg_red(), theme.ansi_bg_red()),
            ),
            ConversationStatus::Cancelled => (Icon::StopFilled, internal_colors::neutral_5(theme)),
            ConversationStatus::Blocked { .. } => (
                Icon::StopFilled,
                pick(theme.ansi_fg_yellow(), theme.ansi_bg_yellow()),
            ),
        }
    }
}

/// Render the status element used by agent views.
pub fn render_status_element(
    status: &ConversationStatus,
    icon_size: f32,
    appearance: &Appearance,
) -> Box<dyn Element> {
    let theme = appearance.theme();
    let (icon, color) = status.status_icon_and_color(theme, StatusColorStyle::Standard);

    Container::new(
        ConstrainedBox::new(icon.to_warpui_icon(Fill::from(color)).finish())
            .with_width(icon_size)
            .with_height(icon_size)
            .finish(),
    )
    .with_uniform_padding(STATUS_ELEMENT_PADDING)
    .with_background(coloru_with_opacity(color, 10))
    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)))
    .finish()
}
