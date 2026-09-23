use std::path::PathBuf;

use ai::agent::action::InsertReviewComment;
use warp_editor::render::model::LineCount;

use super::*;
use crate::code::buffer_location::LocalOrRemotePath;
use crate::code_review::comments::{
    AttachedReviewCommentTarget, CommentOrigin, LineDiffContent, PendingImportedReviewComment,
    PendingImportedReviewCommentTarget, attach_pending_imported_comments,
};

fn make_pending_comment(
    id: &str,
    author: &str,
    body: &str,
    parent_id: Option<&str>,
    timestamp: &str,
    target: PendingImportedReviewCommentTarget,
) -> PendingImportedReviewComment {
    let mut pending = PendingImportedReviewComment::try_from(InsertReviewComment {
        comment_id: id.to_string(),
        author: author.to_string(),
        comment_body: body.to_string(),
        parent_comment_id: parent_id.map(|s| s.to_string()),
        last_modified_timestamp: timestamp.to_string(),
        comment_location: None,
        html_url: None,
    })
    .expect("valid pending import conversion");

    // Override the location target since we intentionally use `comment_location: None` above.
    pending.target = target;

    pending
}

#[test]
fn test_attach_pending_imported_comment_formats_body_and_uses_absolute_path() {
    let repo_path = PathBuf::from("/repo");

    let pending = make_pending_comment(
        "1",
        "alice",
        "Hello world",
        None,
        "2024-01-01T00:00:00Z",
        PendingImportedReviewCommentTarget::Line {
            relative_file_path: PathBuf::from("test.txt"),
            line: EditorLineLocation::Current {
                line_number: LineCount::from(1),
                line_range: LineCount::from(1)..LineCount::from(2),
            },
            diff_content: LineDiffContent {
                content: "+line 1".to_string(),
                lines_added: LineCount::from(1),
                lines_removed: LineCount::from(0),
            },
        },
    );

    let repo_location = LocalOrRemotePath::Local(repo_path.clone());
    let attached = attach_pending_imported_comments(vec![pending], &repo_location);

    assert_eq!(attached.len(), 1);
    assert_eq!(attached[0].content, "**@alice**:\nHello world");

    match &attached[0].target {
        AttachedReviewCommentTarget::Line {
            absolute_file_path, ..
        } => {
            assert_eq!(
                *absolute_file_path,
                LocalOrRemotePath::Local(repo_path.join("test.txt")),
            );
        }
        _ => panic!("expected line comment target"),
    }

    match &attached[0].origin {
        CommentOrigin::ImportedFromGitHub(details) => {
            assert_eq!(details.author, "alice");
            assert_eq!(details.github_comment_id, "1");
            assert!(details.github_parent_id.is_none());
        }
        _ => panic!("expected imported origin"),
    }
}

#[test]
fn test_attach_pending_imported_thread_flattens_depth_first_sorted_by_timestamp() {
    let repo_path = PathBuf::from("/repo");

    let root = make_pending_comment(
        "1",
        "alice",
        "Root",
        None,
        "2024-01-01T00:00:00Z",
        PendingImportedReviewCommentTarget::Line {
            relative_file_path: PathBuf::from("test.txt"),
            line: EditorLineLocation::Current {
                line_number: LineCount::from(1),
                line_range: LineCount::from(1)..LineCount::from(2),
            },
            diff_content: LineDiffContent {
                content: "+line 1".to_string(),
                lines_added: LineCount::from(1),
                lines_removed: LineCount::from(0),
            },
        },
    );

    // Earlier reply to the root.
    let reply_early = make_pending_comment(
        "4",
        "dana",
        "Reply early",
        Some("1"),
        "2024-01-01T00:30:00Z",
        PendingImportedReviewCommentTarget::General,
    );

    // Later reply to the root.
    let reply_late = make_pending_comment(
        "2",
        "bob",
        "Reply later",
        Some("1"),
        "2024-01-01T01:00:00Z",
        PendingImportedReviewCommentTarget::General,
    );

    // Reply to the later reply.
    let reply_nested = make_pending_comment(
        "3",
        "charlie",
        "Nested reply",
        Some("2"),
        "2024-01-01T02:00:00Z",
        PendingImportedReviewCommentTarget::General,
    );

    let latest_timestamp = reply_nested.last_update_time;

    let repo_location = LocalOrRemotePath::Local(repo_path.clone());
    let attached = attach_pending_imported_comments(
        vec![reply_late, root, reply_nested, reply_early],
        &repo_location,
    );

    assert_eq!(attached.len(), 1);
    assert_eq!(
        attached[0].content,
        "**@alice**:\nRoot\n---\n**@dana**:\nReply early\n---\n**@bob**:\nReply later\n---\n**@charlie**:\nNested reply"
    );
    assert_eq!(attached[0].last_update_time, latest_timestamp);

    match &attached[0].target {
        AttachedReviewCommentTarget::Line {
            absolute_file_path, ..
        } => {
            assert_eq!(
                *absolute_file_path,
                LocalOrRemotePath::Local(repo_path.join("test.txt")),
            );
        }
        _ => panic!("expected root line target to be preserved"),
    }
}
