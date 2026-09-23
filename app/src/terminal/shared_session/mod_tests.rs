use std::sync::Arc;

use session_sharing_protocol::common::{Scrollback, ScrollbackBlock};
use url::Url;
use warp_core::command::ExitCode;
use warp_core::features::FeatureFlag;
use warpui::r#async::executor::Background;

use super::decode_scrollback;
use crate::channel::ChannelState;
use crate::terminal::TerminalModel;
use crate::terminal::color::List;
use crate::terminal::event_listener::ChannelEventListener;
use crate::terminal::model::ObfuscateSecrets;
use crate::terminal::model::block::{BlockId, BlockState};
use crate::terminal::model::test_utils::block_size;
use crate::themes::default_themes::dark_theme;
use crate::uri::web_intent_parser::maybe_rewrite_web_url_to_intent;

pub const MAX_BYTES_SHAREABLE: usize = 5000;

#[test]
fn maybe_rewrite_web_url_to_shared_session_intent_rewrites_matching_web_url() {
    let server_root = ChannelState::server_root_url();
    let web_url = Url::parse(&format!(
        "{server_root}/session/00000000-0000-0000-0000-000000000000?pwd=secret&preview=true"
    ))
    .expect("valid shared session web URL");

    let maybe_intent = maybe_rewrite_web_url_to_intent(&web_url)
        .expect("expected shared session web URL to rewrite to an intent URL");

    assert_eq!(maybe_intent.scheme(), ChannelState::url_scheme());
    assert_eq!(maybe_intent.host_str(), Some("shared_session"));
    assert_eq!(maybe_intent.path(), "/00000000-0000-0000-0000-000000000000");
    assert_eq!(maybe_intent.query(), Some("pwd=secret&preview=true"));
}

#[test]
fn maybe_rewrite_web_url_to_shared_session_intent_ignores_non_matching_host() {
    let web_url =
        Url::parse("https://example.com/session/00000000-0000-0000-0000-000000000000?pwd=secret")
            .expect("valid web URL with non-matching host");

    let maybe_intent = maybe_rewrite_web_url_to_intent(&web_url);

    assert!(maybe_intent.is_none());
}

#[test]
fn maybe_rewrite_web_url_to_shared_session_intent_ignores_invalid_session_id() {
    let server_root = ChannelState::server_root_url();
    let web_url = Url::parse(&format!(
        "{server_root}/session/not-a-valid-session-id?pwd=secret&preview=true",
    ))
    .expect("valid web URL with invalid session id path segment");

    let maybe_intent = maybe_rewrite_web_url_to_intent(&web_url);

    assert!(maybe_intent.is_none());
}

pub fn terminal_model_for_viewer(event_proxy: ChannelEventListener) -> TerminalModel {
    TerminalModel::new_for_shared_session_viewer(
        block_size(),
        List::from(&dark_theme().into()),
        event_proxy,
        Arc::new(Background::default()),
        false, /* show_memory_stats */
        false, /* honor_ps1 */
        false, /* is_inverted */
        ObfuscateSecrets::No,
    )
}

#[test]
fn shared_session_viewer_recovers_from_raw_precmd_with_completion_metadata_without_ordered_hint() {
    let _recovery_enabled = FeatureFlag::TerminalLifecycleRecovery.override_enabled(true);
    let channel_event_proxy = ChannelEventListener::new_for_test();
    let mut model = terminal_model_for_viewer(channel_event_proxy);
    model.load_shared_session_scrollback(&[]);
    let completed_block_id = model.active_block_id().clone();
    let next_block_id = BlockId::new();
    let payload = serde_json::json!({
        "hook": "Precmd",
        "value": {
            "exit_code": 47,
            "next_block_id": next_block_id.to_string(),
            "pwd": "/viewer-recovered"
        }
    });
    let mut bytes = b"\x1bP$d".to_vec();
    bytes.extend(hex::encode(payload.to_string()).bytes());
    bytes.push(0x9c);

    model.process_bytes(bytes.as_slice());

    let completed_block = model
        .block_list()
        .block_with_id(&completed_block_id)
        .expect("The viewer's recovered block should remain in the block list.");
    assert_eq!(completed_block.state(), BlockState::DoneWithExecution);
    assert_eq!(completed_block.exit_code(), ExitCode::from(47));
    assert_eq!(model.active_block_id(), &next_block_id);
    assert_eq!(
        model.block_list().active_block().pwd().map(String::as_str),
        Some("/viewer-recovered")
    );
}

#[test]
fn test_scrollback_deserialization() {
    let raw = serde_json::json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "stylized_command": [104, 101, 108, 108, 111],
        "stylized_output": [119, 111, 114, 108, 100],
        "pwd": null,
        "git_head": null,
        "virtual_env": null,
        "conda_env": null,
        "node_version": null,
        "exit_code": 0,
        "did_execute": true,
        "completed_ts": null,
        "start_ts": null,
        "ps1": null,
        "rprompt": null,
        "honor_ps1": false,
        "is_background": false,
        "session_id": null,
        "shell_host": null,
        "prompt_snapshot": null,
        "ai_metadata": null
    });

    let scrollback = Scrollback {
        blocks: vec![ScrollbackBlock {
            raw: serde_json::to_vec(&raw).expect("serialize scrollback json"),
        }],
        is_alt_screen_active: false,
    };

    let decoded = decode_scrollback(&scrollback);

    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0].stylized_command, b"hello");
    assert_eq!(decoded[0].stylized_output, b"world");
}
