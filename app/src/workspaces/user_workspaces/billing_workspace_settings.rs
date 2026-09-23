//! Workspace-level billing metadata accessors.

use super::UserWorkspaces;
use crate::workspaces::team::Team;
use crate::workspaces::workspace::BillingMetadata;

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
}
