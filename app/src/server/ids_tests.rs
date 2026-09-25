use super::{ClientId, HashableId, ObjectIdType, ServerId, SyncId};
use crate::notebooks::NotebookId;
use crate::workflows::WorkflowId;

#[test]
pub fn test_client_sync_id_serialization() {
    let id: SyncId = SyncId::ClientId(ClientId::new());
    let serialized = serde_json::to_string(&id).expect("failed to serialize");
    assert_eq!(serialized, format!("\"{}\"", id.uid()));
    let deserialized: SyncId =
        serde_json::from_str(serialized.as_str()).expect("failed to deserialize");
    assert_eq!(id, deserialized);
}

#[test]
pub fn test_server_sync_id_serialization() {
    let id = SyncId::ServerId(WorkflowId::from(ServerId::from(123)).into());
    let serialized = serde_json::to_string(&id).expect("failed to serialize");
    assert_eq!(serialized, format!("\"{}\"", ServerId::from(123)));
    let deserialized: SyncId =
        serde_json::from_str(serialized.as_str()).expect("failed to deserialize");
    assert_eq!(id, deserialized);
}

#[test]
pub fn test_server_sync_id_uid_serialization() {
    let id = SyncId::ServerId(NotebookId::from(String::from("Ymgrzu0nh2HwDNeYEtXF1x")).into());
    let serialized = serde_json::to_string(&id).expect("failed to serialize");
    assert_eq!(
        serialized,
        format!("\"{}\"", String::from("Ymgrzu0nh2HwDNeYEtXF1x"))
    );
    let deserialized: SyncId =
        serde_json::from_str(serialized.as_str()).expect("failed to deserialize");
    assert_eq!(id, deserialized);
}

#[test]
fn historical_server_ids_keep_sqlite_prefixes() {
    let id = ServerId::try_from("abcdefghijklmnopqrstuv").unwrap();
    let sync_id = SyncId::ServerId(id);

    assert_eq!(
        sync_id.sqlite_uid_hash(ObjectIdType::Notebook),
        "Notebook-abcdefghijklmnopqrstuv"
    );
    assert_eq!(
        sync_id.sqlite_uid_hash(ObjectIdType::Workflow),
        "Workflow-abcdefghijklmnopqrstuv"
    );
    assert_eq!(
        sync_id.sqlite_uid_hash(ObjectIdType::GenericStringObject),
        "GenericStringObject-abcdefghijklmnopqrstuv"
    );
    assert_eq!(
        serde_json::from_str::<SyncId>("\"abcdefghijklmnopqrstuv\"").unwrap(),
        sync_id
    );
}

#[test]
fn historical_client_ids_keep_json_and_sqlite_forms() {
    let id = ClientId::new();
    let serialized = serde_json::to_string(&SyncId::ClientId(id)).unwrap();
    let restored: SyncId = serde_json::from_str(&serialized).unwrap();

    assert_eq!(restored, SyncId::ClientId(id));
    assert_eq!(
        restored.sqlite_uid_hash(ObjectIdType::Workflow),
        id.sqlite_hash()
    );
    assert_eq!(ClientId::from_hash(&id.to_hash()), Some(id));
}
