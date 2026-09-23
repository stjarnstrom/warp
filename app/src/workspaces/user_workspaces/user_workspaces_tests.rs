use settings::PrivatePreferences;
use warp_graphql::billing::{
    BillingMetadata as GqlBillingMetadata, BonusGrantsInfo as GqlBonusGrantsInfo,
    CustomerType as GqlCustomerType, DelinquencyStatus as GqlDelinquencyStatus,
    PurchaseAddOnCreditsPolicy as GqlPurchaseAddOnCreditsPolicy, Tier as GqlTier,
};
use warp_graphql::queries::get_workspaces_metadata_for_user::{
    User as GqlUser, UserProfile as GqlUserProfile, UserPurchasePolicyBillingMetadata,
    UserPurchasePolicyTier,
};
use warp_graphql::user::DiscoverableTeamData as GqlDiscoverableTeamData;
use warp_graphql::workspace::{
    AddonCreditsSettings as GqlAddonCreditsSettings,
    AdminEnablementSetting as GqlAdminEnablementSetting,
    AdminEnablementSettingInfo as GqlAdminEnablementSettingInfo,
    AiAutonomySettingInfo as GqlAiAutonomySettingInfo, AiAutonomySettings as GqlAiAutonomySettings,
    AiAutonomySettingsInfo as GqlAiAutonomySettingsInfo, AiAutonomyValue as GqlAiAutonomyValue,
    AiPermissionsSettings as GqlAiPermissionsSettings,
    AiPermissionsSettingsInfo as GqlAiPermissionsSettingsInfo, AvailableLlms as GqlAvailableLlms,
    BooleanSettingInfo as GqlBooleanSettingInfo,
    CloudConversationStorageSettings as GqlCloudConversationStorageSettings,
    CodebaseContextSettings as GqlCodebaseContextSettings,
    ComputerUseAutonomyValue as GqlComputerUseAutonomyValue,
    ComputerUseSettingInfo as GqlComputerUseSettingInfo,
    FeatureModelChoice as GqlFeatureModelChoice, LinkSharingSettings as GqlLinkSharingSettings,
    LinkSharingSettingsInfo as GqlLinkSharingSettingsInfo, LlmSettings as GqlLlmSettings,
    MembershipRole as GqlMembershipRole,
    SandboxedAgentSettingsInfo as GqlSandboxedAgentSettingsInfo,
    SecretRedactionRegexListInfo as GqlSecretRedactionRegexListInfo,
    SecretRedactionSettings as GqlSecretRedactionSettings,
    SecretRedactionSettingsInfo as GqlSecretRedactionSettingsInfo,
    StringListSettingInfo as GqlStringListSettingInfo, Team as GqlTeam,
    TeamMember as GqlTeamMember, TeamSettings as GqlTeamSettings,
    TeamVisibility as GqlTeamVisibility, TelemetrySettings as GqlTelemetrySettings,
    UgcCollectionEnablementSetting as GqlUgcCollectionEnablementSetting,
    UgcCollectionSettingInfo as GqlUgcCollectionSettingInfo,
    UgcCollectionSettings as GqlUgcCollectionSettings,
    UsageBasedPricingSettings as GqlUsageBasedPricingSettings, Workspace as GqlWorkspace,
    WorkspaceSettings as GqlWorkspaceSettings,
    WriteToPtyAutonomyValue as GqlWriteToPtyAutonomyValue,
    WriteToPtySettingInfo as GqlWriteToPtySettingInfo,
};
use warpui::elements::Empty;
use warpui::platform::WindowStyle;
use warpui::{AddSingletonModel, App, Element, TypedActionView, View, ViewHandle, WindowId};

use super::*;
use crate::server::server_api::ServerApiProvider;
use crate::server::server_api::team::MockTeamClient;
use crate::workspaces::gql_convert::workspaces_metadata_response_from_gql;
use crate::workspaces::user_workspaces::UserWorkspaces;
use crate::workspaces::workspace::Workspace;

fn initialize_window_team_test_app(app: &mut App, workspaces: Vec<Workspace>) {
    app.add_singleton_model(PrivacySettings::mock);
    app.add_singleton_model(|ctx| {
        UserWorkspaces::mock(
            Arc::new(MockTeamClient::new()),
            Arc::new(MockWorkspaceClient::new()),
            workspaces,
            ctx,
        )
    });
}

fn register_ai_usage_model(app: &mut App) {
    app.add_singleton_model(|_| ServerApiProvider::new_for_test());
    if app.models_of_type::<PrivatePreferences>().is_empty() {
        app.update(crate::settings::init_and_register_user_preferences);
    }
}

#[derive(Default)]
struct TeamContextTestView;

impl Entity for TeamContextTestView {
    type Event = ();
}

impl View for TeamContextTestView {
    fn ui_name() -> &'static str {
        "TeamContextTestView"
    }

    fn render(&self, _: &AppContext) -> Box<dyn Element> {
        Empty::new().finish()
    }
}

impl TypedActionView for TeamContextTestView {
    type Action = ();
}

fn create_test_window(app: &mut App) -> (WindowId, ViewHandle<TeamContextTestView>) {
    app.add_window(WindowStyle::NotStealFocus, |_| TeamContextTestView)
}

#[test]
fn test_team_contexts_represent_a_registered_teamless_window() {
    App::test((), |mut app| async move {
        initialize_window_team_test_app(&mut app, vec![]);

        let (window_id, view) = create_test_window(&mut app);
        UserWorkspaces::handle(&app).update(&mut app, |user_workspaces, ctx| {
            user_workspaces.register_window(window_id, None, ctx);
        });

        let context = view.update(&mut app, |_, ctx| {
            UserWorkspaces::as_ref(ctx).team_context_for_operation(ctx)
        });
        assert_eq!(context.team_uid(), None);

        let weak_view = view.downgrade();
        app.read(|ctx| {
            let context = UserWorkspaces::as_ref(ctx).team_context(&weak_view, ctx);
            assert_eq!(context.team_uid(), None);
        });
    })
}

#[test]
fn test_team_switcher_hidden_with_zero_teams() {
    // When the user is in no workspace / no teams, `can_switch_teams` must return
    // false so the pill does not render.
    App::test((), |mut app| async move {
        initialize_window_team_test_app(&mut app, vec![]);
        app.read(|ctx| {
            assert!(
                !UserWorkspaces::as_ref(ctx).can_switch_teams(),
                "0 teams: switcher should be hidden"
            );
        });
    })
}

fn gql_tier(purchase_policy: Option<GqlPurchaseAddOnCreditsPolicy>) -> GqlTier {
    GqlTier {
        name: "Free".to_string(),
        description: "Free tier".to_string(),
        warp_ai_policy: None,
        team_size_policy: None,
        shared_notebooks_policy: None,
        shared_workflows_policy: None,
        session_sharing_policy: None,
        ai_autonomy_policy: None,
        telemetry_data_collection_policy: None,
        ugc_data_collection_policy: None,
        usage_based_pricing_policy: None,
        codebase_context_policy: None,
        byo_api_key_policy: None,
        byo_endpoint_policy: None,
        managed_byok_byoe_policy: None,
        purchase_add_on_credits_policy: purchase_policy,
        enterprise_pay_as_you_go_policy: None,
        enterprise_credits_auto_reload_policy: None,
        multi_admin_policy: None,
        native_workspaces_policy: None,
        ambient_agents_policy: None,
        usage_visibility_policy: None,
    }
}

fn gql_workspace(
    uid: &str,
    purchase_policy: Option<GqlPurchaseAddOnCreditsPolicy>,
) -> GqlWorkspace {
    let empty_llms = GqlAvailableLlms {
        default_id: String::new(),
        choices: vec![],
        preferred_codex_model_id: None,
    };
    GqlWorkspace {
        uid: uid.into(),
        name: "workspace".to_string(),
        stripe_customer_id: None,
        members: vec![],
        teams: vec![],
        open_teams: vec![],
        billing_metadata: GqlBillingMetadata {
            customer_type: GqlCustomerType::Free,
            delinquency_status: GqlDelinquencyStatus::NoDelinquency,
            tier: gql_tier(purchase_policy),
            service_agreements: vec![],
            ai_overages: None,
        },
        bonus_grants_info: GqlBonusGrantsInfo {
            grants: vec![],
            spending_info: None,
        },
        billing_cycle_usage_history: None,
        settings: GqlWorkspaceSettings {
            is_discoverable: false,
            is_invite_link_enabled: false,
            llm_settings: GqlLlmSettings {
                enabled: false,
                host_configs: vec![],
            },
            team_byo: None,
            telemetry_settings: GqlTelemetrySettings {
                force_enabled: false,
            },
            ugc_collection_settings: GqlUgcCollectionSettings {
                setting: GqlUgcCollectionEnablementSetting::RespectUserSetting,
            },
            cloud_conversation_storage_settings: GqlCloudConversationStorageSettings {
                setting: GqlAdminEnablementSetting::RespectUserSetting,
            },
            ai_permissions_settings: GqlAiPermissionsSettings {
                allow_ai_in_remote_sessions: true,
                remote_session_regex_list: vec![],
            },
            link_sharing_settings: GqlLinkSharingSettings {
                anyone_with_link_sharing_enabled: true,
                direct_link_sharing_enabled: true,
            },
            secret_redaction_settings: GqlSecretRedactionSettings {
                enabled: false,
                regexes: vec![],
            },
            ai_autonomy_settings: GqlAiAutonomySettings {
                apply_code_diffs_setting: None,
                read_files_setting: None,
                read_files_allowlist: None,
                create_plans_setting: None,
                execute_commands_setting: None,
                execute_commands_allowlist: None,
                execute_commands_denylist: None,
                write_to_pty_setting: None,
                computer_use_setting: None,
            },
            usage_based_pricing_settings: GqlUsageBasedPricingSettings {
                enabled: false,
                max_monthly_spend_cents: None,
            },
            addon_credits_settings: GqlAddonCreditsSettings {
                auto_reload_enabled: false,
                max_monthly_spend_cents: None,
                selected_auto_reload_credit_denomination: None,
            },
            codebase_context_settings: GqlCodebaseContextSettings {
                enabled: true,
                setting: GqlAdminEnablementSetting::RespectUserSetting,
            },
            sandboxed_agent_settings: None,
            ambient_agent_settings: None,
        },
        has_billing_history: false,
        pending_email_invites: vec![],
        invite_link_domain_restrictions: vec![],
        is_eligible_for_discovery: false,
        feature_model_choice: GqlFeatureModelChoice {
            agent_mode: empty_llms.clone(),
            planning: empty_llms.clone(),
            coding: empty_llms.clone(),
            cli_agent: empty_llms.clone(),
            computer_use_agent: empty_llms,
        },
        total_requests_used_since_last_refresh: 0,
    }
}

/// Team settings with every group at its neutral value, so a fixture only has to
/// override the field the test cares about.
fn gql_team_settings() -> GqlTeamSettings {
    fn admin_info() -> GqlAdminEnablementSettingInfo {
        GqlAdminEnablementSettingInfo {
            value: GqlAdminEnablementSetting::RespectUserSetting,
            is_enforced_by_workspace: false,
        }
    }

    fn bool_info(value: bool) -> GqlBooleanSettingInfo {
        GqlBooleanSettingInfo {
            value,
            is_enforced_by_workspace: false,
        }
    }

    fn autonomy_info() -> GqlAiAutonomySettingInfo {
        GqlAiAutonomySettingInfo {
            value: GqlAiAutonomyValue::RespectUserSetting,
            is_enforced_by_workspace: false,
        }
    }

    fn str_list() -> GqlStringListSettingInfo {
        GqlStringListSettingInfo {
            values: vec![],
            workspace_entries: vec![],
            team_entries: vec![],
        }
    }

    GqlTeamSettings {
        ugc_collection: GqlUgcCollectionSettingInfo {
            value: GqlUgcCollectionEnablementSetting::RespectUserSetting,
            is_enforced_by_workspace: false,
        },
        cloud_conversation_storage: admin_info(),
        codebase_context: admin_info(),
        ai_permissions: GqlAiPermissionsSettingsInfo {
            allow_ai_in_remote_sessions: bool_info(true),
            remote_session_regex_list: str_list(),
        },
        secret_redaction: GqlSecretRedactionSettingsInfo {
            enabled: bool_info(false),
            regexes: GqlSecretRedactionRegexListInfo {
                values: vec![],
                workspace_entries: vec![],
                team_entries: vec![],
            },
        },
        ai_autonomy: GqlAiAutonomySettingsInfo {
            apply_code_diffs: autonomy_info(),
            read_files: autonomy_info(),
            create_plans: autonomy_info(),
            execute_commands: autonomy_info(),
            write_to_pty: GqlWriteToPtySettingInfo {
                value: GqlWriteToPtyAutonomyValue::RespectUserSetting,
                is_enforced_by_workspace: false,
            },
            computer_use: GqlComputerUseSettingInfo {
                value: GqlComputerUseAutonomyValue::RespectUserSetting,
                is_enforced_by_workspace: false,
            },
            read_files_allowlist: str_list(),
            execute_commands_allowlist: str_list(),
            execute_commands_denylist: str_list(),
        },
        link_sharing: GqlLinkSharingSettingsInfo {
            anyone_with_link_sharing_enabled: bool_info(true),
            direct_link_sharing_enabled: bool_info(true),
        },
        sandboxed_agent: GqlSandboxedAgentSettingsInfo {
            execute_commands_denylist: str_list(),
        },
        llm_settings: GqlLlmSettings {
            enabled: false,
            host_configs: vec![],
        },
        telemetry_settings: GqlTelemetrySettings {
            force_enabled: false,
        },
        usage_based_pricing_settings: GqlUsageBasedPricingSettings {
            enabled: false,
            max_monthly_spend_cents: None,
        },
        addon_credits_settings: GqlAddonCreditsSettings {
            auto_reload_enabled: false,
            max_monthly_spend_cents: None,
            selected_auto_reload_credit_denomination: None,
        },
        ambient_agent_settings: None,
        team_byo: None,
    }
}

fn gql_team(uid: &str, name: &str, member_uids: &[&str]) -> GqlTeam {
    let empty_llms = GqlAvailableLlms {
        default_id: String::new(),
        choices: vec![],
        preferred_codex_model_id: None,
    };
    GqlTeam {
        // `ServerId` rejects anything but a 22-character id.
        uid: format!("{uid:0>22}").into(),
        name: name.to_string(),
        color: None,
        members: member_uids
            .iter()
            .map(|member_uid| GqlTeamMember {
                uid: (*member_uid).into(),
                email: format!("{member_uid}@example.com"),
                role: GqlMembershipRole::User,
                is_disabled: false,
            })
            .collect(),
        settings: gql_team_settings(),
        invite_link: None,
        visibility: GqlTeamVisibility::Open,
        feature_model_choice: GqlFeatureModelChoice {
            agent_mode: empty_llms.clone(),
            planning: empty_llms.clone(),
            coding: empty_llms.clone(),
            cli_agent: empty_llms.clone(),
            computer_use_agent: empty_llms,
        },
    }
}

fn apply_workspaces_metadata(app: &mut App, metadata: WorkspacesMetadataResponse) {
    UserWorkspaces::handle(app).update(app, |user_workspaces, ctx| {
        user_workspaces.on_workspaces_updated(
            Ok(WorkspacesMetadataWithPricing {
                metadata,
                pricing_info: None,
            }),
            ctx,
        );
    });
}

fn current_team_names(user_workspaces: &UserWorkspaces) -> Vec<String> {
    user_workspaces
        .current_workspace()
        .map(|workspace| {
            workspace
                .teams
                .iter()
                .map(|team| team.name.clone())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn test_team_switcher_drops_teams_the_admin_is_not_a_member_of() {
    App::test((), |mut app| async move {
        initialize_window_team_test_app(&mut app, vec![]);
        register_ai_usage_model(&mut app);

        // The server hands a workspace admin every team in the workspace, but only
        // the team they actually joined is one they can operate as in the client.
        let mut workspace = gql_workspace("workspace_uid123456789", None);
        workspace.teams = vec![
            gql_team("member-team", "Member Team", &["test-user"]),
            gql_team("other-team", "Other Team", &["someone-else"]),
        ];

        apply_workspaces_metadata(
            &mut app,
            workspaces_metadata_response_from_gql(gql_user(None, vec![workspace]), false),
        );

        app.read(|ctx| {
            let user_workspaces = UserWorkspaces::as_ref(ctx);
            assert_eq!(current_team_names(user_workspaces), ["Member Team"]);
            assert!(
                !user_workspaces.can_switch_teams(),
                "a single membership should hide the switcher"
            );
        });
    })
}

#[test]
fn test_team_switcher_keeps_every_team_the_user_is_a_member_of() {
    App::test((), |mut app| async move {
        initialize_window_team_test_app(&mut app, vec![]);
        register_ai_usage_model(&mut app);

        let mut workspace = gql_workspace("workspace_uid123456789", None);
        workspace.teams = vec![
            gql_team("first-team", "First Team", &["test-user"]),
            gql_team("other-team", "Other Team", &["someone-else"]),
            gql_team("second-team", "Second Team", &["test-user"]),
        ];

        apply_workspaces_metadata(
            &mut app,
            workspaces_metadata_response_from_gql(gql_user(None, vec![workspace]), false),
        );

        app.read(|ctx| {
            let user_workspaces = UserWorkspaces::as_ref(ctx);
            assert_eq!(
                current_team_names(user_workspaces),
                ["First Team", "Second Team"]
            );
            assert!(
                user_workspaces.can_switch_teams(),
                "multiple memberships should keep the switcher visible"
            );
        });
    })
}

fn gql_user(
    user_purchase_policy: Option<GqlPurchaseAddOnCreditsPolicy>,
    workspaces: Vec<GqlWorkspace>,
) -> GqlUser {
    GqlUser {
        profile: GqlUserProfile {
            uid: "test-user".to_string(),
        },
        ai_credit_availability: warp_graphql::ai::AICreditAvailability {
            available: true,
            denial_reason: warp_graphql::ai::AICreditAvailabilityDenialReason::None,
            credit_source: None,
        },
        billing_metadata: user_purchase_policy.map(|policy| UserPurchasePolicyBillingMetadata {
            tier: UserPurchasePolicyTier {
                purchase_add_on_credits_policy: Some(policy),
            },
        }),
        workspaces,
        experiments: None,
        discoverable_teams: vec![],
    }
}

#[test]
fn test_workspace_open_teams_survive_metadata_conversion() {
    let mut workspace = gql_workspace("workspace_uid123456789", None);
    workspace.open_teams = vec![GqlDiscoverableTeamData {
        team_uid: "0000000000000000000002".into(),
        num_members: 2,
        name: "Second Team".to_string(),
        team_accepting_invites: true,
    }];

    let response = workspaces_metadata_response_from_gql(gql_user(None, vec![workspace]), false);

    let open_teams = &response.workspaces[0].open_teams;
    assert_eq!(open_teams.len(), 1);
    assert_eq!(open_teams[0].name, "Second Team");
}
