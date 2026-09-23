use warp_util::standardized_path::StandardizedPath;

use super::super::proto::{
    BundledSkillMetadata, RemoteSkillProto, WriteFileResponse, WriteFileSuccess,
    remote_skill_proto, server_message, write_file_response,
};
use crate::code_review::diff_state::DiffMode;
use crate::remote_server::diff_state_tracker::DiffModelKey;

/// Uses `try_new` instead of `try_from_local` so that Unix-style paths
/// like `/repo` are recognised as absolute on all platforms (including Windows).
fn test_key(repo: &str, mode: DiffMode) -> DiffModelKey {
    DiffModelKey {
        repo_path: StandardizedPath::try_new(repo).unwrap(),
        mode,
    }
}

fn test_bundled_skill_proto(id: &str) -> RemoteSkillProto {
    RemoteSkillProto {
        path: format!(
            "/home/user/.warp/remote-server/bundled_resources/bundled/skills/{id}/SKILL.md"
        ),
        content: format!("# {id}"),
        source: Some(remote_skill_proto::Source::Bundled(BundledSkillMetadata {
            id: id.to_string(),
            requires_mcp: None,
        })),
    }
}

// ── Diff state: connection cleanup ──────────────────────────────────

// ── Git status / GitHub: navigation-driven model cleanup ────────────

// ── Daemon host-scoped response failover ────────────────────────────

/// A throwaway host-scoped response payload used to assert routing.
fn write_file_success_message() -> server_message::Message {
    server_message::Message::WriteFileResponse(WriteFileResponse {
        result: Some(write_file_response::Result::Success(WriteFileSuccess {})),
    })
}
