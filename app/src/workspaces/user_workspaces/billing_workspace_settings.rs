//! Workspace-level plan accessors.
//!
//! These used to read `workspace.billing_metadata` to decide which AI features a
//! plan entitled the user to. Without a Warp account there is no plan to consult,
//! so every entitlement answers yes: the alternative is leaving features such as
//! bring-your-own-key permanently unreachable, which is a limit rather than a
//! deletion. See [`crate::workspaces::user_workspaces::team_workspace_settings`]
//! for the team-scoped policies that still layer on top.

use warpui::AppContext;

use super::UserWorkspaces;
use crate::workspaces::team::Team;
use crate::workspaces::workspace::{BillingMetadata, Workspace};

impl UserWorkspaces {
    pub fn current_workspace_billing_metadata(&self) -> Option<&BillingMetadata> {
        self.current_workspace()
            .map(|workspace| &workspace.billing_metadata)
    }

    /// The given team's billing metadata when the team is known, otherwise the
    /// current workspace's. For surfaces that need team/workspace-scoped state.
    pub fn team_billing_metadata<'a>(
        &'a self,
        team: Option<&'a Team>,
    ) -> Option<&'a BillingMetadata> {
        team.map(|team| &team.billing_metadata)
            .or_else(|| self.current_workspace_billing_metadata())
    }

    /// Not a plan entitlement: this reads `settings.llm_settings.enabled`, which a
    /// team or workspace admin sets. It stays. Only the default changed — with no
    /// team and no workspace there is no admin to have decided, so allow it.
    pub fn is_custom_llm_enabled_for_team(&self, team: Option<&Team>) -> bool {
        team.map(Team::is_custom_llm_enabled)
            .or_else(|| {
                self.current_workspace()
                    .map(Workspace::is_custom_llm_enabled)
            })
            .unwrap_or(true)
    }

    pub fn is_active_ai_allowed(&self) -> bool {
        true
    }

    pub fn ai_allowed_for_team(_team: Option<&Team>) -> bool {
        true
    }

    pub fn is_prompt_suggestions_toggleable(&self) -> bool {
        true
    }

    pub fn is_code_suggestions_toggleable(&self) -> bool {
        true
    }

    pub fn is_next_command_enabled(&self) -> bool {
        true
    }

    pub fn is_git_operations_ai_enabled(&self) -> bool {
        true
    }

    /// Voice still depends on the build: it is `false` when voice input is not
    /// compiled in, because there is nothing to enable.
    pub fn is_voice_enabled(&self) -> bool {
        cfg!(feature = "voice_input")
    }

    pub fn is_byo_api_key_enabled(&self, _app: &AppContext) -> bool {
        true
    }

    pub fn is_byo_endpoint_enabled(&self, _app: &AppContext) -> bool {
        true
    }

    /// Whether the plan manages BYOK/BYOE centrally. Still read from workspace
    /// metadata: it turns on the team-scoped `team_byo` policy, and defaulting it
    /// on would hand key management to a team the user may not be in.
    pub(crate) fn is_managed_byok_byoe_enabled(&self) -> bool {
        self.current_workspace_billing_metadata()
            .is_some_and(|billing| billing.is_managed_byok_byoe_enabled())
    }
}
