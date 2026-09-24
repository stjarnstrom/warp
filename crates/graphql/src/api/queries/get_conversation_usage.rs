use crate::request_context::RequestContext;
use crate::scalars::Time;
use crate::schema;

/*
query GetConversationUsage(
  $requestContext: RequestContext!,
  $days: Int,
  $limit: Int,
  $lastUpdatedEndTimestamp: Time
) {
  user(requestContext: $requestContext) {
    ... on UserOutput {
      user {
        conversationUsage(
          days: $days,
          limit: $limit,
          lastUpdatedEndTimestamp: $lastUpdatedEndTimestamp
        ) {
          conversationId
          title
          lastUpdated
          usageMetadata {
            contextWindowUsage
            creditsSpent
            platformCreditsSpent
            totalProviderCostInCents
            summarized
            tokenUsage { modelId totalTokens }
            warpTokenUsage { modelId totalTokens tokenUsageByCategory { category tokens } }
            byokTokenUsage { modelId totalTokens tokenUsageByCategory { category tokens } }
            toolUsageMetadata {
              runCommandStats { count }
              runCommandsExecuted
              readFilesStats { count }
              searchCodebaseStats { count }
              grepStats { count }
              fileGlobStats { count }
              callMcpToolStats { count }
              readMcpResourceStats { count }
              suggestPlanStats { count }
              suggestCreatePlanStats { count }
              writeToLongRunningShellCommandStats { count }
              applyFileDiffStats { count linesAdded linesRemoved filesChanged }
              readShellCommandOutputStats { count }
              useComputerStats { count }
            }
          }
        }
      }
    }
  }
}
*/

#[derive(cynic::QueryVariables, Debug)]
pub struct GetConversationUsageVariables {
    pub request_context: RequestContext,
    pub days: Option<i32>,
    pub limit: Option<i32>,
    pub last_updated_end_timestamp: Option<Time>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    graphql_type = "RootQuery",
    variables = "GetConversationUsageVariables"
)]
pub struct GetConversationUsage {
    #[arguments(requestContext: $request_context)]
    pub user: UserResult,
}
crate::client::define_operation! {
    get_conversation_usage_history(GetConversationUsageVariables) -> GetConversationUsage;
}

#[derive(cynic::InlineFragments, Debug)]
#[cynic(variables = "GetConversationUsageVariables")]
pub enum UserResult {
    UserOutput(UserOutput),
    #[cynic(fallback)]
    Unknown,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    graphql_type = "UserOutput",
    variables = "GetConversationUsageVariables"
)]
pub struct UserOutput {
    pub user: User,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "User", variables = "GetConversationUsageVariables")]
pub struct User {
    #[arguments(
        days: $days,
        limit: $limit,
        lastUpdatedEndTimestamp: $last_updated_end_timestamp
    )]
    pub conversation_usage: Vec<ConversationUsage>,
}

#[derive(cynic::QueryFragment, Debug, Clone)]
pub struct ConversationUsage {
    pub conversation_id: String,
    pub last_updated: Time,
    pub title: String,
    pub usage_metadata: ConversationUsageMetadata,
}

#[derive(cynic::QueryFragment, Debug, Clone)]
pub struct ConversationUsageMetadata {
    pub context_window_usage: f64,
    pub context_window_segments: Vec<ContextWindowSegment>,
    pub credits_spent: f64,
    pub platform_credits_spent: f64,
    pub total_provider_cost_in_cents: Option<f64>,
    pub summarized: bool,
    pub token_usage: Vec<ModelTokenUsage>,
    pub warp_token_usage: Vec<TokenUsage>,
    pub byok_token_usage: Vec<TokenUsage>,
    pub tool_usage_metadata: ToolUsageMetadata,
}

#[derive(cynic::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextWindowSegmentType {
    Unknown,
    SystemPrompt,
    ToolDefinitions,
    ConversationHistory,
    LatestInput,
    Images,
    Other,
}

#[derive(cynic::QueryFragment, Debug, Clone)]
pub struct ContextWindowSegment {
    pub segment_type: ContextWindowSegmentType,
    pub token_count: i32,
}

#[derive(cynic::QueryFragment, Debug, Clone)]
pub struct ModelTokenUsage {
    pub model_id: String,
    pub total_tokens: i32,
}

#[derive(cynic::QueryFragment, Debug, Clone)]
pub struct TokenUsage {
    pub model_id: String,
    pub total_tokens: i32,
    pub token_usage_by_category: Vec<CategoryTokenBreakdown>,
}

#[derive(cynic::QueryFragment, Debug, Clone)]
pub struct CategoryTokenBreakdown {
    pub category: String,
    pub tokens: i32,
}

#[derive(cynic::QueryFragment, Debug, Clone)]
pub struct ToolCallStats {
    pub count: i32,
}

#[derive(cynic::QueryFragment, Debug, Clone)]
pub struct ApplyFileDiffStats {
    pub count: i32,
    pub lines_added: i32,
    pub lines_removed: i32,
    pub files_changed: i32,
}

#[derive(cynic::QueryFragment, Debug, Clone)]
pub struct ToolUsageMetadata {
    pub run_command_stats: ToolCallStats,
    pub run_commands_executed: i32,
    pub read_files_stats: ToolCallStats,
    pub search_codebase_stats: ToolCallStats,
    pub grep_stats: ToolCallStats,
    pub file_glob_stats: ToolCallStats,
    pub call_mcp_tool_stats: ToolCallStats,
    pub read_mcp_resource_stats: ToolCallStats,
    pub suggest_plan_stats: ToolCallStats,
    pub suggest_create_plan_stats: ToolCallStats,
    pub write_to_long_running_shell_command_stats: ToolCallStats,
    pub apply_file_diff_stats: ApplyFileDiffStats,
    pub read_shell_command_output_stats: ToolCallStats,
    pub use_computer_stats: ToolCallStats,
}
