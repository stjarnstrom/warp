use std::path::PathBuf;

use ai::workspace::WorkspaceMetadata;
use chrono::Utc;
use cloud_object_persistence::to_cloud_object_permissions;
use warp_core::features::FeatureFlag;
use warp_graphql::scalars::time::ServerTimestamp;

use super::{
    app_database_file_path, database_file_path_for_scope, decode_path, deduplicate_events,
    encode_path, get_all_codebase_index_metadata, setup_database, start_writer,
};
use crate::cloud_object::{CloudObjectPermissions, Owner};
use crate::persistence::model::ObjectPermissions;
use crate::persistence::{BlockCompleted, ModelEvent, PersistenceScope};

#[test]
fn app_scope_database_path_matches_app_database_path() {
    assert_eq!(
        database_file_path_for_scope(&PersistenceScope::App),
        app_database_file_path()
    );
}

#[test]
fn remote_server_daemon_scope_database_path_uses_identity_data_dir() {
    let path = database_file_path_for_scope(&PersistenceScope::RemoteServerDaemon {
        identity_key: "user@example.com/ssh host".to_string(),
    });
    let expected_data_dir =
        remote_server::setup::remote_server_daemon_data_dir("user@example.com/ssh host");

    assert!(path.is_absolute());
    assert_eq!(
        path,
        PathBuf::from(shellexpand::tilde(&expected_data_dir).into_owned()).join("warp.sqlite")
    );
}

#[test]
fn remote_server_daemon_scope_database_path_handles_empty_identity_key() {
    let path = database_file_path_for_scope(&PersistenceScope::RemoteServerDaemon {
        identity_key: String::new(),
    });
    let expected_data_dir = remote_server::setup::remote_server_daemon_data_dir("");

    assert_eq!(
        path,
        PathBuf::from(shellexpand::tilde(&expected_data_dir).into_owned()).join("warp.sqlite")
    );
}

#[cfg(unix)]
#[test]
fn remote_server_daemon_database_permissions_are_owner_only() {
    use std::fs::Permissions;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let daemon_dir = tempdir.path().join("daemon");
    let database_path = daemon_dir.join("warp.sqlite");

    std::fs::create_dir_all(&daemon_dir).expect("daemon dir should be created");
    std::fs::set_permissions(&daemon_dir, Permissions::from_mode(0o755))
        .expect("daemon dir permissions should be set");
    std::fs::write(&database_path, b"").expect("database file should be created");
    std::fs::set_permissions(&database_path, Permissions::from_mode(0o644))
        .expect("database file permissions should be set");

    super::ensure_owner_only_dir(&daemon_dir).expect("daemon dir should be owner-only");
    super::ensure_owner_only_file(&database_path).expect("database file should be owner-only");

    assert_eq!(daemon_dir.metadata().unwrap().mode() & 0o777, 0o700);
    assert_eq!(database_path.metadata().unwrap().mode() & 0o777, 0o600);
}

fn test_codebase_metadata(path: &str) -> WorkspaceMetadata {
    WorkspaceMetadata {
        path: PathBuf::from(path),
        navigated_ts: Some(Utc::now()),
        modified_ts: None,
        queried_ts: None,
    }
}

#[test]
fn sqlite_writer_reuses_codebase_index_metadata_events() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    let database_path = tempdir.path().join("warp.sqlite");
    let conn = setup_database(&database_path).expect("database should initialize");

    let writer = start_writer(conn, database_path.clone()).expect("writer should start");
    let metadata = test_codebase_metadata("/tmp/writer-repo");
    writer
        .sender
        .send(ModelEvent::UpsertCodebaseIndexMetadata {
            index_metadata: Box::new(metadata.clone()),
        })
        .expect("upsert event should send");
    writer
        .sender
        .send(ModelEvent::Terminate)
        .expect("terminate event should send");
    writer.handle.join().expect("writer should terminate");

    let mut conn = setup_database(&database_path).expect("database should reopen");
    let restored = get_all_codebase_index_metadata(&mut conn).expect("metadata should load");
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].path, metadata.path);

    let writer = start_writer(conn, database_path.clone()).expect("writer should restart");
    writer
        .sender
        .send(ModelEvent::DeleteCodebaseIndexMetadata {
            repo_path: metadata.path,
        })
        .expect("delete event should send");
    writer
        .sender
        .send(ModelEvent::Terminate)
        .expect("terminate event should send");
    writer.handle.join().expect("writer should terminate");

    let mut conn = setup_database(&database_path).expect("database should reopen");
    let restored = get_all_codebase_index_metadata(&mut conn).expect("metadata should load");
    assert!(restored.is_empty());
}

#[test]
fn test_deduplicate_no_snapshots() {
    let original_events = vec![ModelEvent::SaveBlock(BlockCompleted {
        pane_id: vec![1, 2, 3],
        block: Default::default(),
        is_local: true,
    })];
    let filtered_events = deduplicate_events(original_events);
    assert_eq!(filtered_events.len(), 1);
    assert!(matches!(&filtered_events[0], &ModelEvent::SaveBlock(_)));
}

fn assert_encode_then_decode_preserves_original_path(original_path: PathBuf) {
    let bytes = encode_path(original_path.clone());
    let decoded_path = decode_path(bytes);
    assert_eq!(original_path, decoded_path);
}

/// Test that a local path can be encoded and decoded. We use this when persisting a local
/// file path for notebooks in sqlite. We need this test because Windows `OsString`s are
/// often arbitrary sequences of 16-bit values, unlike Unix which uses sequences of 8-bit
/// values (bytes). Since `diesel::sql_types::Binary` deals with sequences of bytes (`u8`)
/// we need to perform special casting on `OsString`s on Windows.
#[test]
fn test_path_encode_decode() {
    // Empty path
    assert_encode_then_decode_preserves_original_path(PathBuf::new());

    // Windows-style paths
    assert_encode_then_decode_preserves_original_path(PathBuf::from(r"C:\windows\system32.dll"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from("c:temp"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from(r"\temp"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from(r"\temp\emoji\🙈.txt"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from(r"\temp\ñoñàscii\temp.txt"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from(r"\temp\hindi\हिन्दी"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from(r"\temp\cjk\狗没有耐心"));

    // Unix-style paths
    assert_encode_then_decode_preserves_original_path(PathBuf::from(
        "/home/persistence/example.sql",
    ));
    assert_encode_then_decode_preserves_original_path(PathBuf::from("./database/log.txt"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from("/temp/emoji/🙈.txt"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from("/temp/ñoñàscii/temp.txt"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from("/temp/hindi/हिन्दी"));
    assert_encode_then_decode_preserves_original_path(PathBuf::from("/temp/cjk/狗没有耐心"));
}

#[test]
fn test_deserialize_corrupted_guests() {
    let _ = FeatureFlag::SharedWithMe.override_enabled(true);
    // Use a hardcoded timestamp to ensure this test works on systems with more-than-microsecond
    // precision.
    let permissions_ts_micros = 123456;
    let permissions_ts =
        ServerTimestamp::from_unix_timestamp_micros(permissions_ts_micros).unwrap();

    let db_permissions = ObjectPermissions {
        id: 42,
        object_metadata_id: 10,
        subject_type: "TEAM".to_string(),
        subject_id: Some("7".to_string()),
        subject_uid: "team_uid12345678912345".to_string(),
        permissions_last_updated_at: Some(permissions_ts_micros),
        // This is not a valid set of encoded object guests.
        object_guests: Some(vec![1, 2, 3]),
        anyone_with_link_access_level: None,
        anyone_with_link_source: None,
    };

    // The overall permissions should successfully convert, minus the object guests.
    let cloud_permissions = to_cloud_object_permissions(&db_permissions, None);
    assert_eq!(
        cloud_permissions,
        Some(CloudObjectPermissions {
            owner: Owner::Team {
                team_uid: crate::server::ids::ServerId::from_string_lossy("team_uid12345678912345"),
            },
            permissions_last_updated_ts: Some(permissions_ts),
            anyone_with_link: None,
            guests: vec![],
        })
    );
}
