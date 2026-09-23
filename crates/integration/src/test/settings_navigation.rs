//! Integration tests for settings sidebar navigation and search.
//!
//! These pin down the user-visible behavior of the settings nav rail —
//! clicking rows, arrow-key cycling, umbrella expand/collapse, and search
//! filtering across both top-level pages and umbrella subpages — so that
//! refactors of the settings page model cannot silently regress them.

use warp::integration_testing::settings::{
    assert_settings_nav_page_visible, assert_settings_section, assert_settings_widget_rendered,
    assert_umbrella_expanded, clear_settings_search, open_settings_page, press_settings_nav_up,
    type_settings_search,
};
use warp::integration_testing::terminal::wait_until_bootstrapped_single_pane_for_tab;
use warp::settings_view::{SettingsSection, cli_agent_settings_widget_id};

use super::{Builder, new_builder};

/// Label of the umbrella that groups the agent subpages.
const AGENTS_UMBRELLA: &str = "Agents";

/// Label of the umbrella that groups the cloud platform subpages.
const CLOUD_PLATFORM_UMBRELLA: &str = "Cloud platform";

// ---------------------------------------------------------------------------
// Mouse navigation
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Keyboard navigation
// ---------------------------------------------------------------------------

/// Arrowing Up into a collapsed umbrella enters it at its *last* subpage,
/// matching the reading order the user was moving through.
pub fn test_settings_keyboard_navigation_up_into_collapsed_umbrella() -> Builder {
    new_builder()
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        // Appearance sits directly below the Cloud platform umbrella.
        .with_step(open_settings_page(SettingsSection::Appearance))
        .with_step(assert_umbrella_expanded(CLOUD_PLATFORM_UMBRELLA, false))
        .with_step(press_settings_nav_up())
        .with_step(assert_settings_section(
            SettingsSection::WarpCloudAgentAPIKeys,
        ))
        .with_step(assert_umbrella_expanded(CLOUD_PLATFORM_UMBRELLA, true))
}

// ---------------------------------------------------------------------------
// Search filtering
// ---------------------------------------------------------------------------

/// A query that only matches a top-level page hides the other top-level rows
/// and moves the selection onto the surviving page.
pub fn test_settings_search_filters_top_level_pages() -> Builder {
    new_builder()
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(open_settings_page(SettingsSection::Account))
        .with_step(type_settings_search("keyboard shortcut"))
        .with_step(assert_settings_nav_page_visible(
            SettingsSection::Keybindings,
            true,
        ))
        .with_step(assert_settings_nav_page_visible(
            SettingsSection::About,
            false,
        ))
        // Account no longer matches, so the selection follows the filter.
        .with_step(assert_settings_section(SettingsSection::Keybindings))
}

/// A search that matches only one subpage must still render that subpage's
/// content, not an empty pane.
///
/// Sidebar visibility and content rendering are decided separately, so a page
/// can keep its row while the content pane renders nothing. No sidebar-only
/// assertion would catch that, which is why this asserts on a widget.
pub fn test_settings_search_subpage_still_renders_content() -> Builder {
    new_builder()
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(open_settings_page(SettingsSection::Account))
        // The CLI agent widget lives on the Third party CLI agents subpage, and
        // nothing has rendered it yet.
        .with_step(assert_settings_widget_rendered(
            cli_agent_settings_widget_id(),
            false,
        ))
        .with_step(type_settings_search("codex"))
        .with_step(assert_settings_section(
            SettingsSection::ThirdPartyCLIAgents,
        ))
        .with_step(assert_settings_widget_rendered(
            cli_agent_settings_widget_id(),
            true,
        ))
}

/// Clearing the search restores the umbrella expansion state the user had
/// before searching, rather than leaving auto-expanded umbrellas open.
pub fn test_settings_search_clear_restores_umbrella_state() -> Builder {
    new_builder()
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(open_settings_page(SettingsSection::Account))
        .with_step(assert_umbrella_expanded(AGENTS_UMBRELLA, false))
        .with_step(type_settings_search("codex"))
        .with_step(assert_umbrella_expanded(AGENTS_UMBRELLA, true))
        .with_step(clear_settings_search())
        .with_step(assert_umbrella_expanded(AGENTS_UMBRELLA, false))
        .with_step(assert_settings_nav_page_visible(
            SettingsSection::About,
            true,
        ))
}

// ---------------------------------------------------------------------------
// MCP servers
// ---------------------------------------------------------------------------
