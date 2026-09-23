use std::sync::Arc;

use chrono::{DateTime, Utc};
use cloud_object_client::MockObjectClient;
use cloud_object_models::JsonSerializer;
use futures_lite::future;
use settings::{RespectUserSyncSetting, SyncToCloud};
use warp_core::features::FeatureFlag;
use warp_graphql::scalars::time::ServerTimestamp;
use warpui::{App, ModelHandle, SingletonEntity};

use super::{GetCloudObjectResponse, InitialLoadResponse, UpdateManager};
use crate::ASSETS;
use crate::cloud_object::model::actions::{
    ObjectAction, ObjectActionHistory, ObjectActionSubtype, ObjectActionType, ObjectActions,
};
use crate::cloud_object::model::generic_string_model::GenericStringObjectId;
use crate::cloud_object::model::persistence::{CloudModel, CloudModelEvent, UpdateSource};
use crate::cloud_object::{
    BulkCreateCloudObjectResult, CloudModelType, CloudObjectEventEntrypoint,
    CreateCloudObjectResult, CreatedCloudObject, GenericCloudObject, ObjectIdType,
    ObjectMetadataUpdateResult, ObjectType, Owner, Revision, RevisionAndLastEditor,
    ServerCloudObject, ServerNotebook, ServerObject, ServerWorkflow, Space,
    UpdateCloudObjectResult,
};
use crate::drive::CloudObjectTypeAndId;
use crate::drive::folders::{CloudFolderModel, FolderId};
use crate::notebooks::{CloudNotebook, CloudNotebookModel, NotebookId};
use crate::persistence::ModelEvent;
use crate::server::cloud_objects::listener::ObjectUpdateMessage;
use crate::server::cloud_objects::test_utils::{
    UpdateManagerStruct, create_update_manager_struct, initialize_app, mock_server_api,
};
use crate::server::cloud_objects::update_manager::{
    FetchSingleObjectOption, GenericStringObjectInput, InitiatedBy, ServerMetadata,
    ServerPermissions, get_duplicate_object_name,
};
use crate::server::ids::{
    ClientId, HashableId, ObjectUid, ServerId, ServerIdAndType, SyncId, ToServerId,
};
use crate::server::sync_queue::SyncQueue;
use crate::settings::{CloudPreferenceModel, Preference};
use crate::workflows::workflow::{Argument, ArgumentType, Workflow};
use crate::workflows::workflow_enum::{EnumVariants, WorkflowEnum};
use crate::workflows::{CloudWorkflow, CloudWorkflowModel, WorkflowId};

fn create_object<K, M>(
    app: &mut App,
    update_manager: &ModelHandle<UpdateManager>,
    client_id: ClientId,
    model: M,
) -> GenericCloudObject<K, M>
where
    K: HashableId
        + ToServerId
        + std::fmt::Debug
        + Into<String>
        + Clone
        + Copy
        + Send
        + Sync
        + 'static,
    M: CloudModelType<IdType = K, CloudObjectType = GenericCloudObject<K, M>> + 'static,
{
    update_manager.update(app, |update_manager, ctx| {
        update_manager.create_object(
            model,
            Owner::mock_current_user(),
            client_id,
            CloudObjectEventEntrypoint::Unknown,
            true,
            None,
            InitiatedBy::User,
            ctx,
        );
    });
    CloudModel::handle(app).read(app, |cloud_model, _ctx| {
        cloud_model
            .get_object_of_type::<K, M>(&SyncId::ClientId(client_id))
            .expect("object should exist")
            .clone()
    })
}

fn create_object_result<K: HashableId + ToServerId>(
    client_id: ClientId,
    object_id: K,
    object_id_type: ObjectIdType,
) -> CreateCloudObjectResult {
    CreateCloudObjectResult::Success {
        created_cloud_object: CreatedCloudObject {
            client_id,
            revision_and_editor: RevisionAndLastEditor {
                revision: Revision::now(),
                last_editor_uid: Some("34jkaosdfj".to_string()),
            },
            metadata_ts: DateTime::<Utc>::default().into(),
            creator_uid: None,
            server_id_and_type: ServerIdAndType {
                id: object_id.to_server_id(),
                id_type: object_id_type,
            },
            permissions: ServerPermissions::mock_personal(),
        },
    }
}

fn receive_object_update_from_rtc(
    app: &mut App,
    update_manager: &ModelHandle<UpdateManager>,
    item: ObjectUpdateMessage,
) {
    update_manager.update(app, move |update_manager, ctx| {
        update_manager.received_message_from_server(item, ctx);
    })
}

fn receive_initial_load_or_polling_update(
    app: &mut App,
    update_manager: &ModelHandle<UpdateManager>,
    force_refresh: bool,
    mocked_response: InitialLoadResponse,
) {
    update_manager.update(app, move |update_manager, ctx| {
        update_manager.on_changed_objects_fetched(mocked_response, force_refresh, ctx);
    })
}

fn mock_server_permissions(owner: Owner) -> ServerPermissions {
    ServerPermissions {
        space: owner,
        guests: Vec::new(),
        anyone_link_sharing: None,
        permissions_last_updated_ts: Utc::now().into(),
    }
}

fn create_workflow(
    client_id: ClientId,
    app: &mut App,
    update_manager: &ModelHandle<UpdateManager>,
) {
    create_workflow_internal(
        app,
        update_manager,
        client_id,
        "client_workflow".to_string(),
        "echo client".to_string(),
        Owner::mock_current_user(),
        None,
    )
}

fn create_workflow_internal(
    app: &mut App,
    update_manager: &ModelHandle<UpdateManager>,
    client_id: ClientId,
    workflow_name: String,
    workflow_command: String,
    owner: Owner,
    initial_folder_id: Option<SyncId>,
) {
    update_manager.update(app, |update_manager, ctx| {
        update_manager.create_workflow(
            Workflow::new(workflow_name, workflow_command),
            owner,
            initial_folder_id,
            client_id,
            CloudObjectEventEntrypoint::Unknown,
            true,
            ctx,
        );
    });
}

fn get_workflow(app: &App, sync_id: SyncId) -> CloudWorkflow {
    CloudModel::handle(app).read(app, |cloud_model, _ctx| {
        cloud_model
            .get_workflow(&sync_id)
            .expect("workflow should exist")
            .clone()
    })
}

fn create_workflow_enum(
    client_id: ClientId,
    app: &mut App,
    update_manager: &ModelHandle<UpdateManager>,
) {
    create_workflow_enum_internal(
        app,
        update_manager,
        client_id,
        "workflow_enum".to_string(),
        vec!["variant 1".to_string(), "variant 2".to_string()],
        Owner::mock_current_user(),
    )
}

fn create_workflow_enum_internal(
    app: &mut App,
    update_manager: &ModelHandle<UpdateManager>,
    client_id: ClientId,
    enum_name: String,
    enum_variants: Vec<String>,
    owner: Owner,
) {
    update_manager.update(app, |update_manager, ctx| {
        update_manager.create_workflow_enum(
            WorkflowEnum {
                name: enum_name,
                variants: EnumVariants::Static(enum_variants),
                is_shared: false,
            },
            owner,
            client_id,
            CloudObjectEventEntrypoint::Unknown,
            true,
            ctx,
        );
    });
}

fn mock_server_workflow(id: WorkflowId, owner: Owner, metadata: ServerMetadata) -> ServerWorkflow {
    ServerWorkflow::new(
        SyncId::ServerId(id.into()),
        CloudWorkflowModel::new(Workflow::new(format!("w{id}"), format!("c{id}"))),
        metadata,
        mock_server_permissions(owner),
    )
}

fn mock_server_notebook(id: NotebookId, owner: Owner, metadata: ServerMetadata) -> ServerNotebook {
    ServerNotebook::new(
        SyncId::ServerId(id.into()),
        CloudNotebookModel {
            title: format!("n{id}"),
            data: format!("n{id}"),
            ai_document_id: None,
            conversation_id: None,
        },
        metadata,
        mock_server_permissions(owner),
    )
}

#[track_caller]
fn assert_pending_online_only_change_for_object(app: &mut App, uid: &ObjectUid, status: bool) {
    CloudModel::handle(app).update(app, |cloud_model, _| {
        if let Some(object) = cloud_model.get_mut_by_uid(uid) {
            assert_eq!(
                object.metadata().has_pending_online_only_change(),
                status,
                "Expected has_pending_online_only_change for {uid} to be {status}"
            );
        } else {
            panic!("object should have been in cloud model, but wasn't");
        }
    });
}

fn assert_pending_status_for_object(app: &mut App, uid: &ObjectUid, status: bool) {
    CloudModel::handle(app).update(app, |cloud_model, _| {
        if let Some(object) = cloud_model.get_mut_by_uid(uid) {
            assert_eq!(
                object.metadata().has_pending_content_changes(),
                status,
                "Expected has_pending_content_changes for {uid} to be {status}"
            );
        } else {
            panic!("object should have been in cloud model, but wasn't");
        }
    });
}

fn assert_trashed_status_for_object(app: &mut App, uid: &ObjectUid, is_trashed: bool) {
    CloudModel::handle(app).update(app, |cloud_model, _| {
        if let Some(object) = cloud_model.get_mut_by_uid(uid) {
            assert_eq!(
                object.metadata().trashed_ts.is_some(),
                is_trashed,
                "Expected trashed status for {uid} to be {is_trashed}"
            );
        } else {
            panic!("object should have been in cloud model, but wasn't");
        }
    });
}

fn assert_root_level_for_object(app: &mut App, uid: &ObjectUid, is_root_level: bool) {
    CloudModel::handle(app).update(app, |cloud_model, _| {
        if let Some(object) = cloud_model.get_mut_by_uid(uid) {
            assert_eq!(object.metadata().folder_id.is_none(), is_root_level);
        } else {
            panic!("object should have been in cloud model, but wasn't");
        }
    });
}

fn assert_folder_for_object(app: &App, uid: &ObjectUid, folder_id: Option<SyncId>) {
    CloudModel::handle(app).read(app, |cloud_model, _| {
        if let Some(object) = cloud_model.get_by_uid(uid) {
            assert_eq!(object.metadata().folder_id, folder_id);
        } else {
            panic!("object should have been in cloud model, but wasn't");
        }
    });
}

fn assert_space_for_object(app: &App, uid: &ObjectUid, space: Space) {
    CloudModel::handle(app).read(app, |cloud_model, ctx| {
        if let Some(object) = cloud_model.get_by_uid(uid) {
            assert_eq!(object.space(ctx), space);
        } else {
            panic!("object should have been in cloud model, but wasn't");
        }
    });
}

fn assert_workflow_name(app: &mut App, sync_id: SyncId, expected_name: &str) {
    let workflow = get_workflow(app, sync_id);
    assert_eq!(workflow.model().data.name(), expected_name);
}

fn db_events(update_manager_struct: &UpdateManagerStruct) -> Vec<ModelEvent> {
    let mut db_events = Vec::new();

    while let Ok(event) = update_manager_struct.receiver.try_recv() {
        db_events.push(event);
    }

    db_events
}

fn cloud_events(update_manager_struct: &UpdateManagerStruct) -> Vec<CloudModelEvent> {
    let mut events = Vec::new();
    while let Ok(event) = update_manager_struct.cloud_model_events.try_recv() {
        events.push(event);
    }
    events
}

fn mock_create_workflow(
    client_id: ClientId,
    server_api: &mut MockObjectClient,
    workflow_id: WorkflowId,
) {
    server_api
        .expect_create_workflow()
        .times(1)
        .return_once(move |_| {
            Ok(CreateCloudObjectResult::Success {
                created_cloud_object: CreatedCloudObject {
                    client_id,
                    revision_and_editor: RevisionAndLastEditor {
                        revision: Revision::now(),
                        last_editor_uid: Some("34jkaosdfj".to_string()),
                    },
                    metadata_ts: DateTime::<Utc>::default().into(),
                    server_id_and_type: ServerIdAndType {
                        id: workflow_id.to_server_id(),
                        id_type: ObjectIdType::Workflow,
                    },
                    creator_uid: None,
                    permissions: ServerPermissions::mock_personal(),
                },
            })
        });
}

fn mock_fetch_single_cloud_object(
    server_api: &mut MockObjectClient,
    workflow_id: WorkflowId,
    server_id: ServerId,
) {
    server_api
        .expect_fetch_single_cloud_object()
        .times(1)
        .return_once(move |_| {
            Ok(GetCloudObjectResponse {
                object: ServerCloudObject::Workflow(Box::new(ServerWorkflow::new(
                    SyncId::ServerId(workflow_id.into()),
                    CloudWorkflowModel::new(Workflow::new("server workflow", "echo server")),
                    ServerMetadata {
                        uid: server_id,
                        revision: Revision::now(),
                        metadata_last_updated_ts: Utc::now().into(),
                        trashed_ts: None,
                        folder_id: None,
                        is_welcome_object: false,
                        creator_uid: None,
                        last_editor_uid: None,
                        current_editor_uid: None,
                    },
                    ServerPermissions {
                        space: Owner::mock_current_user(),
                        guests: Vec::new(),
                        anyone_link_sharing: None,
                        permissions_last_updated_ts: Utc::now().into(),
                    },
                ))),
                descendants: vec![],
                action_histories: vec![ObjectActionHistory {
                    uid: server_id.uid(),
                    hashed_sqlite_id: server_id.sqlite_type_and_uid_hash(ObjectIdType::Workflow),
                    latest_processed_at_timestamp: Utc::now(),
                    actions: vec![],
                }],
            })
        });
}

#[test]
fn test_sync_state_after_creation_item_not_in_sync_queue_folder() {
    App::test(ASSETS, |mut app| async move {
        let client_id = ClientId::new();
        initialize_app(&mut app);
        let object_id: FolderId = 123.into();
        let mut server_api = mock_server_api();
        server_api
            .expect_create_folder()
            .times(1)
            .return_once(move |_| {
                Ok(create_object_result(
                    client_id,
                    object_id,
                    ObjectIdType::Folder,
                ))
            });
        run_sync_state_after_creation_item_not_in_sync_queue(
            app,
            object_id.into(),
            CloudFolderModel::new("test folder", false),
            client_id,
            server_api,
        )
        .await;
    })
}

#[test]
fn test_sync_state_after_creation_item_not_in_sync_queue_workflow() {
    App::test(ASSETS, |mut app| async move {
        let client_id = ClientId::new();
        initialize_app(&mut app);
        let object_id: WorkflowId = 123.into();
        let mut server_api = mock_server_api();
        server_api
            .expect_create_workflow()
            .times(1)
            .return_once(move |_| {
                Ok(create_object_result(
                    client_id,
                    object_id,
                    ObjectIdType::Workflow,
                ))
            });
        run_sync_state_after_creation_item_not_in_sync_queue(
            app,
            object_id.into(),
            CloudWorkflowModel::new(Workflow::new("name".to_owned(), "cmd".to_owned())),
            client_id,
            server_api,
        )
        .await;
    })
}

#[test]
fn test_sync_state_after_creation_item_not_in_sync_queue_notebook() {
    App::test(ASSETS, |mut app| async move {
        let client_id = ClientId::new();
        initialize_app(&mut app);

        let object_id: NotebookId = 123.into();
        let mut server_api = mock_server_api();
        server_api
            .expect_create_notebook()
            .times(1)
            .return_once(move |_| {
                Ok(create_object_result(
                    client_id,
                    object_id,
                    ObjectIdType::Notebook,
                ))
            });
        run_sync_state_after_creation_item_not_in_sync_queue(
            app,
            object_id.into(),
            CloudNotebookModel::default(),
            client_id,
            server_api,
        )
        .await;
    })
}

#[test]
fn test_sync_state_after_creation_item_not_in_sync_queue_generic_object() {
    App::test(ASSETS, |mut app| async move {
        let client_id = ClientId::new();
        initialize_app(&mut app);
        let object_id: GenericStringObjectId = 123.into();
        let mut server_api = mock_server_api();
        server_api
            .expect_create_generic_string_object()
            .times(1)
            .return_once(move |_, _, request| {
                assert!(request.serialized_model.is_some());
                Ok(create_object_result(
                    client_id,
                    object_id,
                    ObjectIdType::GenericStringObject,
                ))
            });
        run_sync_state_after_creation_item_not_in_sync_queue(
            app,
            object_id.into(),
            CloudPreferenceModel::new(
                Preference::new(
                    "foo".to_owned(),
                    "{\"test_key\": \"test_value\"}",
                    SyncToCloud::Globally(RespectUserSyncSetting::Yes),
                )
                .expect("error creating preference"),
            ),
            client_id,
            server_api,
        )
        .await;
    })
}

// Runs a test case where we validate the sync state of an object after it's been created.
async fn run_sync_state_after_creation_item_not_in_sync_queue<K, M>(
    mut app: App,
    object_id: ServerId,
    model: M,
    client_id: ClientId,
    server_api: MockObjectClient,
) where
    K: HashableId
        + ToServerId
        + std::fmt::Debug
        + Into<String>
        + Clone
        + Copy
        + Send
        + Sync
        + 'static,
    M: CloudModelType<IdType = K, CloudObjectType = GenericCloudObject<K, M>> + 'static,
{
    let server_id: SyncId = SyncId::ServerId(object_id);
    let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));

    // create an object
    let object = create_object::<K, M>(
        &mut app,
        &update_manager_struct.update_manager,
        client_id,
        model,
    );

    // verify it's pending
    assert_pending_status_for_object(&mut app, &client_id.to_string(), true);

    // complete the create request
    SyncQueue::handle(&app)
        .update(&mut app, |sync_queue, ctx| {
            ctx.await_spawned_future(sync_queue.spawned_futures()[0])
        })
        .await;

    // because there aren't any items in the sync queue left for this object,
    // it should be marked as having no pending changes
    assert_pending_status_for_object(&mut app, &server_id.uid(), false);

    let events = db_events(&update_manager_struct);

    assert_eq!(events.len(), 4);
    // we created an object in the db
    assert_eq!(
        std::mem::discriminant(&events[0]),
        std::mem::discriminant(&M::upsert_event(
            object.upsert_params(object.model().object_type())
        ))
    );
    // when we got the correct response back from the server,
    // we updated the db with the server id
    assert!(matches!(
        &events[1],
        ModelEvent::UpdateObjectAfterServerCreation {
            client_id: _,
            server_creation_info: _
        }
    ));
    // since this object is no longer pending, we mark it as synced
    assert!(matches!(
        &events[2],
        ModelEvent::MarkObjectAsSynced {
            hashed_sqlite_id: _,
            revision_and_editor: _,
            metadata_ts: _,
        }
    ));
    assert!(matches!(
        &events[3],
        ModelEvent::SyncObjectActions { actions_to_sync: _ }
    ));
}

#[test]
fn test_sync_state_after_creation_fails_due_to_limit() {
    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);
        let mut server_api = mock_server_api();
        let client_id = ClientId::new();
        let workflow_id: WorkflowId = 123.into();
        let server_id = SyncId::ServerId(workflow_id.into());
        let team_uid: ServerId = ServerId::from(789);

        let mut create_workflow_calls = 0;
        server_api
            .expect_create_workflow()
            .times(2)
            .returning(move |_| {
                create_workflow_calls += 1;
                match create_workflow_calls {
                    // Return an over limit user error on the first attempt.
                    1 => Ok(CreateCloudObjectResult::UserFacingError(
                        "limit exceeded".to_string(),
                    )),
                    // Return a successful response on the second attempt.
                    2 => Ok(CreateCloudObjectResult::Success {
                        created_cloud_object: CreatedCloudObject {
                            client_id,
                            revision_and_editor: RevisionAndLastEditor {
                                revision: Revision::now(),
                                last_editor_uid: Some("34jkaosdfj".to_string()),
                            },
                            metadata_ts: DateTime::<Utc>::default().into(),
                            server_id_and_type: ServerIdAndType {
                                id: workflow_id.to_server_id(),
                                id_type: ObjectIdType::Workflow,
                            },
                            creator_uid: None,
                            permissions: ServerPermissions::mock_personal(),
                        },
                    }),
                    _ => unreachable!(),
                }
            });

        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));

        // create a workflow in team space
        update_manager_struct
            .update_manager
            .update(&mut app, |update_manager, ctx| {
                update_manager.create_workflow(
                    Workflow::new("workflow_name", "echo hello world"),
                    Owner::Team { team_uid },
                    None,
                    client_id,
                    CloudObjectEventEntrypoint::Unknown,
                    false,
                    ctx,
                );
            });

        // verify it's pending
        assert_pending_status_for_object(&mut app, &client_id.to_string(), true);

        // complete the workflow create request (this should fail due to limit hit)
        SyncQueue::handle(&app)
            .update(&mut app, |sync_queue, ctx| {
                ctx.await_spawned_future(sync_queue.spawned_futures()[0])
            })
            .await;

        // complete the second workflow create request (in personal space)
        SyncQueue::handle(&app)
            .update(&mut app, |sync_queue, ctx| {
                ctx.await_spawned_future(sync_queue.spawned_futures()[1])
            })
            .await;

        let events = db_events(&update_manager_struct);

        // because there aren't any items in the sync queue left for this object,
        // it should be marked as having no pending changes
        assert_pending_status_for_object(&mut app, &server_id.uid(), false);

        assert_eq!(events.len(), 5);

        // We created a workflow in the db.
        match &events[0] {
            ModelEvent::UpsertWorkflow { workflow } => {
                // Verify initial location of workflow is the shared drive.
                assert_eq!(workflow.permissions.owner, Owner::Team { team_uid });
            }
            _ => panic!("Expected an UpsertWorkflow event"),
        }
        // We also triggered an update event in the db when moving it from team to personal drive.
        match &events[1] {
            ModelEvent::UpsertWorkflow { workflow } => {
                // Verify new location of workflow is the personal drive.
                assert_eq!(workflow.permissions.owner, Owner::mock_current_user());
            }
            _ => panic!("Expected an UpsertWorkflow event"),
        }
        // when we got the correct response back from the server,
        // we updated the db with the server id
        assert!(matches!(
            &events[2],
            ModelEvent::UpdateObjectAfterServerCreation {
                client_id: _,
                server_creation_info: _
            }
        ));
        // since this object is no longer pending, we mark it as synced
        assert!(matches!(
            &events[3],
            ModelEvent::MarkObjectAsSynced {
                hashed_sqlite_id: _,
                revision_and_editor: _,
                metadata_ts: _,
            }
        ));
        assert!(matches!(
            &events[4],
            ModelEvent::SyncObjectActions { actions_to_sync: _ }
        ));
    })
}

#[test]
fn test_bulk_create_generic_string_objects() {
    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);
        let mut server_api = mock_server_api();
        let client_id_1 = ClientId::new();
        let object_id_1: GenericStringObjectId = 123.into();
        let client_id_2 = ClientId::new();
        let object_id_2: GenericStringObjectId = 456.into();

        server_api
            .expect_bulk_create_generic_string_objects()
            .times(1)
            .return_once(move |_, _| {
                Ok(BulkCreateCloudObjectResult::Success {
                    created_cloud_objects: vec![
                        CreatedCloudObject {
                            client_id: client_id_1,
                            revision_and_editor: RevisionAndLastEditor {
                                revision: Revision::now(),
                                last_editor_uid: Some("34jkaosdfj".to_string()),
                            },
                            metadata_ts: DateTime::<Utc>::default().into(),
                            server_id_and_type: ServerIdAndType {
                                id: object_id_1.to_server_id(),
                                id_type: ObjectIdType::GenericStringObject,
                            },
                            creator_uid: None,
                            permissions: ServerPermissions::mock_personal(),
                        },
                        CreatedCloudObject {
                            client_id: client_id_2,
                            revision_and_editor: RevisionAndLastEditor {
                                revision: Revision::now(),
                                last_editor_uid: Some("34jkaosdfk".to_string()),
                            },
                            server_id_and_type: ServerIdAndType {
                                id: object_id_2.to_server_id(),
                                id_type: ObjectIdType::GenericStringObject,
                            },
                            metadata_ts: DateTime::<Utc>::default().into(),
                            creator_uid: None,
                            permissions: ServerPermissions::mock_personal(),
                        },
                    ],
                })
            });

        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));

        let inputs = vec![
            GenericStringObjectInput::<Preference, JsonSerializer> {
                id: client_id_1,
                model: CloudPreferenceModel::new(
                    Preference::new(
                        "storage_key_1".to_string(),
                        "{\"test_key\": \"test_value_1\"}",
                        SyncToCloud::Globally(RespectUserSyncSetting::Yes),
                    )
                    .expect("error creating preference"),
                ),
                initial_folder_id: None,
                entrypoint: CloudObjectEventEntrypoint::Unknown,
            },
            GenericStringObjectInput::<Preference, JsonSerializer> {
                id: client_id_2,
                model: CloudPreferenceModel::new(
                    Preference::new(
                        "storage_key_2".to_string(),
                        "{\"test_key\": \"test_value_2\"}",
                        SyncToCloud::Globally(RespectUserSyncSetting::Yes),
                    )
                    .expect("error creating preference"),
                ),
                initial_folder_id: None,
                entrypoint: CloudObjectEventEntrypoint::Unknown,
            },
        ];

        // Bulk create objects
        update_manager_struct
            .update_manager
            .update(&mut app, move |update_manager, ctx| {
                update_manager.bulk_create_generic_string_objects(
                    Owner::mock_current_user(),
                    inputs,
                    ctx,
                );
            });

        // Make sure that we won't block quitting even though there are pending changes at this point.
        CloudModel::handle(&app).read(&app, |cloud_model, _| {
            assert_eq!(cloud_model.num_unsaved_objects(), 2);
            assert_eq!(
                cloud_model.num_unsaved_objects_to_warn_about_before_quitting(),
                0
            );
        });

        // complete the object create request
        SyncQueue::handle(&app)
            .update(&mut app, |sync_queue, ctx| {
                ctx.await_spawned_future(sync_queue.spawned_futures()[0])
            })
            .await;

        let events = db_events(&update_manager_struct);

        assert_eq!(events.len(), 7);
        // we created two items in bulk in the db
        assert!(matches!(
            &events[0],
            ModelEvent::UpsertGenericStringObjects { .. }
        ));

        // when we got the correct responses back from the server,
        // we updated the db with the server id, for each object
        assert!(matches!(
            &events[1],
            ModelEvent::UpdateObjectAfterServerCreation {
                client_id: _,
                server_creation_info: _
            }
        ));
        assert!(matches!(
            &events[2],
            ModelEvent::MarkObjectAsSynced {
                hashed_sqlite_id: _,
                revision_and_editor: _,
                metadata_ts: _,
            }
        ));
        assert!(matches!(
            &events[3],
            ModelEvent::SyncObjectActions { actions_to_sync: _ }
        ));
        assert!(matches!(
            &events[4],
            ModelEvent::UpdateObjectAfterServerCreation {
                client_id: _,
                server_creation_info: _
            }
        ));
        assert!(matches!(
            &events[5],
            ModelEvent::MarkObjectAsSynced {
                hashed_sqlite_id: _,
                revision_and_editor: _,
                metadata_ts: _,
            }
        ));
        assert!(matches!(
            &events[6],
            ModelEvent::SyncObjectActions { actions_to_sync: _ }
        ));
    })
}

#[test]
fn test_sync_state_after_update_item_not_in_sync_queue_generic_string_object() {
    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);
        let mut server_api = mock_server_api();
        let client_id = ClientId::new();
        let object_id: GenericStringObjectId = 123.into();
        let server_id: SyncId = SyncId::ServerId(object_id.into());

        server_api
            .expect_create_generic_string_object()
            .times(1)
            .return_once(move |_, _, _| {
                Ok(CreateCloudObjectResult::Success {
                    created_cloud_object: CreatedCloudObject {
                        client_id,
                        revision_and_editor: RevisionAndLastEditor {
                            revision: Revision::now(),
                            last_editor_uid: Some("34jkaosdfj".to_string()),
                        },
                        metadata_ts: DateTime::<Utc>::default().into(),
                        server_id_and_type: ServerIdAndType {
                            id: object_id.to_server_id(),
                            id_type: ObjectIdType::GenericStringObject,
                        },
                        creator_uid: None,
                        permissions: ServerPermissions::mock_personal(),
                    },
                })
            });
        server_api
            .expect_update_generic_string_object()
            .times(1)
            .return_once(move |_, _, _| {
                Ok(UpdateCloudObjectResult::<Box<dyn ServerObject>>::Success {
                    revision_and_editor: RevisionAndLastEditor {
                        revision: Revision::now(),
                        last_editor_uid: None,
                    },
                })
            });

        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));

        // create a test json object
        create_object(
            &mut app,
            &update_manager_struct.update_manager,
            client_id,
            CloudPreferenceModel::new(
                Preference::new(
                    "foo".to_owned(),
                    "{\"test_key\": \"test_value\"}",
                    SyncToCloud::Globally(RespectUserSyncSetting::Yes),
                )
                .expect("error creating preference"),
            ),
        );
        // update the test json object's data
        update_manager_struct
            .update_manager
            .update(&mut app, |update_manager, ctx| {
                update_manager.update_object(
                    CloudPreferenceModel::new(
                        Preference::new(
                            "foo".to_owned(),
                            "{\"test_key\": \"test_value_2\"}",
                            SyncToCloud::Globally(RespectUserSyncSetting::Yes),
                        )
                        .expect("error creating preference"),
                    ),
                    SyncId::ClientId(client_id),
                    None,
                    ctx,
                );
            });
        // complete the object create request
        SyncQueue::handle(&app)
            .update(&mut app, |sync_queue, ctx| {
                ctx.await_spawned_future(sync_queue.spawned_futures()[0])
            })
            .await;
        // complete the object update request
        SyncQueue::handle(&app)
            .update(&mut app, |sync_queue, ctx| {
                ctx.await_spawned_future(sync_queue.spawned_futures()[1])
            })
            .await;

        // because there aren't any items in the sync queue left for this object,
        // it should be marked as having no pending changes
        assert_pending_status_for_object(&mut app, &server_id.uid(), false);

        let events = db_events(&update_manager_struct);

        assert_eq!(events.len(), 5);
        // we created a notebook in the db
        assert!(matches!(
            &events[0],
            ModelEvent::UpsertGenericStringObject { .. }
        ));
        // we also triggered an update event in the db
        assert!(matches!(
            &events[1],
            ModelEvent::UpsertGenericStringObject { .. }
        ));
        // when we got the correct response back from the server,
        // we updated the db with the server id
        assert!(matches!(
            &events[2],
            ModelEvent::UpdateObjectAfterServerCreation {
                client_id: _,
                server_creation_info: _
            }
        ));
        assert!(matches!(
            &events[3],
            ModelEvent::SyncObjectActions { actions_to_sync: _ }
        ));
        // since this object is no longer pending, we mark it as synced
        assert!(matches!(
            &events[4],
            ModelEvent::MarkObjectAsSynced {
                hashed_sqlite_id: _,
                revision_and_editor: _,
                metadata_ts: _,
            }
        ));
    })
}

#[test]
fn test_sync_state_after_object_with_dependencies_created() {
    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);
        let mut server_api = mock_server_api();

        let enum_client_id = ClientId::new();
        let enum_server_id: ServerId = 456.into();
        let enum_id: GenericStringObjectId = enum_server_id.into();
        let workflow_client_id = ClientId::new();
        let workflow = Workflow::new("workflow_name".to_string(), "description".to_string())
            .with_arguments(vec![Argument {
                name: "enum".to_string(),
                arg_type: ArgumentType::Enum {
                    enum_id: SyncId::ClientId(enum_client_id),
                },
                default_value: None,
                description: None,
            }]);
        // workflow object, replaced with enum server ID
        let updated_workflow =
            Workflow::new("workflow_name".to_string(), "description".to_string()).with_arguments(
                vec![Argument {
                    name: "enum".to_string(),
                    arg_type: ArgumentType::Enum {
                        enum_id: SyncId::ServerId(enum_id.into()),
                    },
                    default_value: None,
                    description: None,
                }],
            );

        server_api
            .expect_create_generic_string_object()
            .times(1)
            .return_once(move |_, _, _| {
                Ok(CreateCloudObjectResult::Success {
                    created_cloud_object: CreatedCloudObject {
                        client_id: enum_client_id,
                        revision_and_editor: RevisionAndLastEditor {
                            revision: Revision::now(),
                            last_editor_uid: Some("34jkaosdfj".to_string()),
                        },
                        metadata_ts: DateTime::<Utc>::default().into(),
                        server_id_and_type: ServerIdAndType {
                            id: enum_id.to_server_id(),
                            id_type: ObjectIdType::GenericStringObject,
                        },
                        creator_uid: None,
                        permissions: ServerPermissions::mock_personal(),
                    },
                })
            });

        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));

        // create an enum
        create_workflow_enum(
            enum_client_id,
            &mut app,
            &update_manager_struct.update_manager,
        );

        // create a workflow with a dependency on the enum

        update_manager_struct
            .update_manager
            .update(&mut app, |update_manager, ctx| {
                update_manager.create_workflow(
                    workflow,
                    Owner::mock_current_user(),
                    None,
                    workflow_client_id,
                    CloudObjectEventEntrypoint::Unknown,
                    true,
                    ctx,
                );
            });

        // complete the workflow enum create request
        SyncQueue::handle(&app)
            .update(&mut app, |sync_queue, ctx| {
                // stop dequeueing so we only execute the first request
                sync_queue.stop_dequeueing();

                ctx.await_spawned_future(sync_queue.spawned_futures()[0])
            })
            .await;

        // check that we updated cloud events
        let cloud_events = cloud_events(&update_manager_struct);
        assert_eq!(cloud_events.len(), 6);
        assert!(matches!(
            &cloud_events[0],
            CloudModelEvent::ObjectCreated { .. }
        ));
        assert!(matches!(
            &cloud_events[1],
            CloudModelEvent::ObjectForceExpanded { .. }
        ));
        assert!(matches!(
            &cloud_events[2],
            CloudModelEvent::ObjectCreated { .. }
        ));
        assert!(matches!(
            &cloud_events[3],
            CloudModelEvent::ObjectForceExpanded { .. }
        ));
        assert!(matches!(
            &cloud_events[4],
            CloudModelEvent::ObjectSynced { .. }
        ));
        assert!(matches!(
            &cloud_events[5],
            CloudModelEvent::ObjectUpdated { .. }
        ));

        // check db update events
        let db_events = db_events(&update_manager_struct);
        assert_eq!(db_events.len(), 6);
        assert!(matches!(
            &db_events[0],
            ModelEvent::UpsertGenericStringObject { .. }
        ));
        assert!(matches!(&db_events[1], ModelEvent::UpsertWorkflow { .. }));
        assert!(matches!(
            &db_events[2],
            ModelEvent::UpdateObjectAfterServerCreation { .. }
        ));
        assert!(matches!(
            &db_events[3],
            ModelEvent::MarkObjectAsSynced { .. }
        ));
        assert!(matches!(&db_events[4], ModelEvent::UpsertWorkflow { .. }));
        assert!(matches!(
            &db_events[5],
            ModelEvent::SyncObjectActions { .. }
        ));

        // assert that we properly updated the dependency after the enum completed
        assert!({
            if let ModelEvent::UpsertWorkflow { workflow } = &db_events[4] {
                workflow.model().data == updated_workflow
            } else {
                false
            }
        });
    })
}

#[test]
fn test_fetch_single_cloud_object_not_pending_no_overwrite() {
    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);
        let mut server_api = mock_server_api();
        let client_id = ClientId::new();
        let server_id: ServerId = 123.into();
        let workflow_id: WorkflowId = server_id.into();
        let sync_id = SyncId::ServerId(workflow_id.into());

        mock_create_workflow(client_id, &mut server_api, workflow_id);
        mock_fetch_single_cloud_object(&mut server_api, workflow_id, server_id);

        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));

        // create a workflow
        create_workflow(client_id, &mut app, &update_manager_struct.update_manager);
        // complete the workflow create request
        SyncQueue::handle(&app)
            .update(&mut app, |sync_queue, ctx| {
                ctx.await_spawned_future(sync_queue.spawned_futures()[0])
            })
            .await;

        // Flush cloud model events.
        let _ = cloud_events(&update_manager_struct);

        // call to fetch the server's representation of this object.
        // because our in-memory workflow doesn't have any pending changes,
        // this will simply overwrite it with the server version.
        update_manager_struct
            .update_manager
            .update(&mut app, |update_manager, ctx| {
                let fetch_cloud_object_rx = update_manager.fetch_single_cloud_object(
                    &server_id,
                    FetchSingleObjectOption::None,
                    ctx,
                );
                std::mem::drop(fetch_cloud_object_rx);
                ctx.await_spawned_future(update_manager.spawned_futures[0])
            })
            .await;

        assert_workflow_name(&mut app, sync_id, "server workflow");

        let events = db_events(&update_manager_struct);

        assert_eq!(events.len(), 6);
        // we created a workflow in the db
        assert!(matches!(
            &events[0],
            ModelEvent::UpsertWorkflow { workflow: _ }
        ));
        // the successful create triggered a set of the server id
        assert!(matches!(
            &events[1],
            ModelEvent::UpdateObjectAfterServerCreation {
                client_id: _,
                server_creation_info: _
            }
        ));
        // because the object had no in flight requests, it was marked as synced in the db
        assert!(matches!(
            &events[2],
            ModelEvent::MarkObjectAsSynced {
                hashed_sqlite_id: _,
                revision_and_editor: _,
                metadata_ts: _
            }
        ));
        assert!(matches!(
            &events[3],
            ModelEvent::SyncObjectActions { actions_to_sync: _ }
        ));
        // lastly, we upserted the workflow when we got the server version back
        assert!(matches!(
            &events[4],
            ModelEvent::UpsertWorkflow { workflow: _ }
        ));
        assert!(matches!(
            &events[5],
            ModelEvent::SyncObjectActions { actions_to_sync: _ }
        ));

        // We emitted an event that the workflow changed.
        let events = cloud_events(&update_manager_struct);
        assert_eq!(
            events,
            vec![CloudModelEvent::ObjectUpdated {
                type_and_id: CloudObjectTypeAndId::from_id_and_type(sync_id, ObjectType::Workflow),
                source: UpdateSource::Server
            }]
        );
    })
}

#[test]
fn test_metadata_update_with_rtc_no_pending() {
    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);
        let server_api = mock_server_api();

        let server_id: ServerId = 123.into();
        let notebook_id: NotebookId = server_id.into();
        let sync_id: SyncId = SyncId::ServerId(notebook_id.into());

        let current_metadata_ts = Utc::now();
        let metadata = ServerMetadata {
            uid: ServerId::default(),
            revision: Revision::now(),
            metadata_last_updated_ts: current_metadata_ts.into(),
            trashed_ts: None,
            folder_id: None,
            is_welcome_object: false,
            creator_uid: None,
            last_editor_uid: None,
            current_editor_uid: None,
        };
        let notebook: ServerNotebook =
            mock_server_notebook(notebook_id, Owner::mock_current_user(), metadata);

        CloudModel::handle(&app).update(&mut app, |cloud_model, _| {
            cloud_model.add_object(sync_id, CloudNotebook::new_from_server(notebook));
        });

        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));

        assert_pending_online_only_change_for_object(
            &mut app,
            &notebook_id.to_server_id().uid(),
            false,
        );

        let new_metadata_ts = current_metadata_ts + chrono::Duration::seconds(1);

        // While this trash request is "in-flight", mock getting an RTC update from the server that includes a new editor
        let mocked_metadata = ServerMetadata {
            uid: server_id,
            revision: Revision::now(),
            metadata_last_updated_ts: new_metadata_ts.into(),
            trashed_ts: None,
            folder_id: None,
            is_welcome_object: false,
            creator_uid: None,
            last_editor_uid: None,
            current_editor_uid: Some("ian@warp.dev".to_string()),
        };

        let mocked_metadata_update_message = ObjectUpdateMessage::ObjectMetadataChanged {
            metadata: mocked_metadata.clone(),
        };
        receive_object_update_from_rtc(
            &mut app,
            &update_manager_struct.update_manager,
            mocked_metadata_update_message,
        );

        // Assert we still don't have a pending change now that we've updated the metadata
        assert_pending_online_only_change_for_object(
            &mut app,
            &notebook_id.to_server_id().uid(),
            false,
        );

        // Assert that the metadata changes are correctly applied
        CloudModel::handle(&app).read(&app, |cloud_model, _ctx| {
            if let Some(object) = cloud_model.get_by_uid(&notebook_id.to_server_id().uid()) {
                assert_eq!(
                    object.metadata().current_editor_uid,
                    mocked_metadata.current_editor_uid
                );
                assert_eq!(
                    object
                        .metadata()
                        .metadata_last_updated_ts
                        .expect("metadata should exist"),
                    mocked_metadata.metadata_last_updated_ts
                );
            } else {
                panic!("object should have been in cloud model, but wasn't");
            }
        });

        let events = db_events(&update_manager_struct);

        assert_eq!(events.len(), 1);
        // we trigger a metadata update event from the rtc message
        assert!(matches!(
            &events[0],
            ModelEvent::UpdateObjectMetadata { id: _, metadata: _ }
        ));
    })
}

#[test]
fn test_metadata_update_with_polling_no_pending() {
    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);
        let server_api = mock_server_api();

        let server_id: ServerId = 123.into();
        let notebook_id: NotebookId = server_id.into();
        let sync_id: SyncId = SyncId::ServerId(notebook_id.into());

        let current_metadata_ts = Utc::now();
        let metadata = ServerMetadata {
            uid: ServerId::default(),
            revision: Revision::now(),
            metadata_last_updated_ts: current_metadata_ts.into(),
            trashed_ts: None,
            folder_id: None,
            is_welcome_object: false,
            creator_uid: None,
            last_editor_uid: None,
            current_editor_uid: None,
        };
        let notebook: ServerNotebook =
            mock_server_notebook(notebook_id, Owner::mock_current_user(), metadata);

        CloudModel::handle(&app).update(&mut app, |cloud_model, _| {
            cloud_model.add_object(sync_id, CloudNotebook::new_from_server(notebook));
        });

        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));

        assert_pending_online_only_change_for_object(
            &mut app,
            &notebook_id.to_server_id().uid(),
            false,
        );

        let new_metadata_ts = current_metadata_ts + chrono::Duration::seconds(1);
        // While this trash request is "in-flight", mock getting a polling update from the server that includes a new editor
        let mocked_metadata = ServerMetadata {
            uid: server_id,
            revision: Revision::now(),
            metadata_last_updated_ts: new_metadata_ts.into(),
            trashed_ts: None,
            folder_id: None,
            is_welcome_object: false,
            creator_uid: None,
            last_editor_uid: None,
            current_editor_uid: Some("ian@warp.dev".to_string()),
        };
        let mocked_response = InitialLoadResponse {
            updated_notebooks: vec![ServerNotebook::new(
                SyncId::ServerId(server_id),
                CloudNotebookModel {
                    title: "".into(),
                    data: "".into(),
                    ai_document_id: None,
                    conversation_id: None,
                },
                mocked_metadata.clone(),
                mock_server_permissions(Owner::mock_current_user()),
            )],
            deleted_notebooks: vec![],
            updated_workflows: vec![],
            deleted_workflows: vec![],
            updated_folders: vec![],
            deleted_folders: vec![],
            user_profiles: vec![],
            updated_generic_string_objects: Default::default(),
            deleted_generic_string_objects: Default::default(),
            action_histories: Default::default(),
            mcp_gallery: Default::default(),
        };
        receive_initial_load_or_polling_update(
            &mut app,
            &update_manager_struct.update_manager,
            false, /* force_refresh */
            mocked_response,
        );

        // Assert we still don't have a pending change now that we've updated the metadata
        assert_pending_online_only_change_for_object(
            &mut app,
            &notebook_id.to_server_id().uid(),
            false,
        );

        // Assert that the metadata changes are correctly applied
        CloudModel::handle(&app).read(&app, |cloud_model, _ctx| {
            if let Some(object) = cloud_model.get_by_uid(&notebook_id.to_server_id().uid()) {
                assert_eq!(
                    object.metadata().current_editor_uid,
                    mocked_metadata.current_editor_uid
                );
                assert_eq!(
                    object
                        .metadata()
                        .metadata_last_updated_ts
                        .expect("metadata should exist"),
                    mocked_metadata.metadata_last_updated_ts
                );
            } else {
                panic!("object should have been in cloud model, but wasn't");
            }
        });

        let events = db_events(&update_manager_struct);

        // All the upserts from polling
        assert!(matches!(&events[0], ModelEvent::SyncObjectActions { .. }));
        assert!(matches!(&events[1], ModelEvent::UpsertNotebooks(_)));
        assert!(matches!(&events[2], ModelEvent::UpsertWorkflows(_)));
        assert!(matches!(&events[3], ModelEvent::UpsertFolders(_)));
        assert!(matches!(&events[4], ModelEvent::DeleteObjects { ids: _ }));
    });
}

#[test]
fn test_metadata_after_untrash_item_success() {
    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);
        let mut server_api = mock_server_api();
        let server_id: ServerId = 123.into();
        let workflow_id: WorkflowId = server_id.into();
        let sync_id = SyncId::ServerId(workflow_id.into());

        let workflow_metadata = ServerMetadata {
            uid: ServerId::default(),
            revision: Revision::now(),
            metadata_last_updated_ts: Utc::now().into(),
            trashed_ts: Some(ServerTimestamp::from_unix_timestamp_micros(10).unwrap()),
            folder_id: None,
            is_welcome_object: false,
            creator_uid: None,
            last_editor_uid: None,
            current_editor_uid: None,
        };
        let workflow: ServerWorkflow = mock_server_workflow(
            workflow_id,
            Owner::mock_current_user(),
            workflow_metadata.clone(),
        );

        CloudModel::handle(&app).update(&mut app, |cloud_model, _| {
            cloud_model.add_object(sync_id, CloudWorkflow::new_from_server(workflow));
        });

        let mut untrashed_metadata = workflow_metadata;
        untrashed_metadata.trashed_ts = None;

        server_api
            .expect_untrash_object()
            .times(1)
            .return_once(move |_| {
                Ok(ObjectMetadataUpdateResult::Success {
                    metadata: Box::new(untrashed_metadata),
                })
            });

        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));

        assert_trashed_status_for_object(&mut app, &sync_id.uid(), true);

        // untrash the workflow
        let type_and_id = CloudObjectTypeAndId::from_id_and_type(sync_id, ObjectType::Workflow);
        update_manager_struct
            .update_manager
            .update(&mut app, |update_manager, ctx| {
                update_manager.untrash_object(type_and_id, ctx);
                ctx.await_spawned_future(update_manager.spawned_futures[0])
            })
            .await;

        assert_trashed_status_for_object(&mut app, &sync_id.uid(), false);
        assert_root_level_for_object(&mut app, &sync_id.uid(), true);
        assert_pending_status_for_object(&mut app, &sync_id.uid(), false);

        assert_eq!(
            cloud_events(&update_manager_struct),
            vec![CloudModelEvent::ObjectUntrashed {
                type_and_id,
                source: UpdateSource::Local
            }]
        );
    })
}

#[test]
fn test_metadata_after_untrash_item_and_move_to_root() {
    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);
        let mut server_api = mock_server_api();
        let server_id: ServerId = 123.into();
        let workflow_id: WorkflowId = server_id.into();
        let sync_id = SyncId::ServerId(workflow_id.into());
        let folder_id: FolderId = 456.into();

        let workflow_metadata = ServerMetadata {
            uid: ServerId::default(),
            revision: Revision::now(),
            metadata_last_updated_ts: Utc::now().into(),
            trashed_ts: Some(ServerTimestamp::from_unix_timestamp_micros(10).unwrap()),
            folder_id: Some(folder_id),
            is_welcome_object: false,
            creator_uid: None,
            last_editor_uid: None,
            current_editor_uid: None,
        };
        let workflow: ServerWorkflow = mock_server_workflow(
            workflow_id,
            Owner::mock_current_user(),
            workflow_metadata.clone(),
        );

        CloudModel::handle(&app).update(&mut app, |cloud_model, _| {
            cloud_model.add_object(sync_id, CloudWorkflow::new_from_server(workflow));
        });

        let mut untrashed_metadata = workflow_metadata.clone();
        untrashed_metadata.trashed_ts = None;
        untrashed_metadata.folder_id = None;

        server_api
            .expect_untrash_object()
            .times(1)
            .return_once(move |_| {
                Ok(ObjectMetadataUpdateResult::Success {
                    metadata: Box::new(untrashed_metadata),
                })
            });

        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));

        assert_trashed_status_for_object(&mut app, &sync_id.uid(), true);
        assert_root_level_for_object(&mut app, &sync_id.uid(), false);

        // untrash the workflow
        let type_and_id = CloudObjectTypeAndId::from_id_and_type(sync_id, ObjectType::Workflow);
        update_manager_struct
            .update_manager
            .update(&mut app, |update_manager, ctx| {
                update_manager.untrash_object(type_and_id, ctx);
                ctx.await_spawned_future(update_manager.spawned_futures[0])
            })
            .await;

        assert_trashed_status_for_object(&mut app, &sync_id.uid(), false);
        assert_root_level_for_object(&mut app, &sync_id.uid(), true);
        assert_pending_status_for_object(&mut app, &sync_id.uid(), false);

        assert_eq!(
            cloud_events(&update_manager_struct),
            vec![CloudModelEvent::ObjectUntrashed {
                type_and_id,
                source: UpdateSource::Local
            }]
        );
    })
}

#[test]
fn test_metadata_after_untrash_item_failure() {
    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);
        let mut server_api = mock_server_api();
        let server_id: ServerId = 123.into();
        let workflow_id: WorkflowId = server_id.into();
        let sync_id = SyncId::ServerId(workflow_id.into());

        let workflow_metadata = ServerMetadata {
            uid: ServerId::default(),
            revision: Revision::now(),
            metadata_last_updated_ts: Utc::now().into(),
            trashed_ts: Some(ServerTimestamp::from_unix_timestamp_micros(10).unwrap()),
            folder_id: None,
            is_welcome_object: false,
            creator_uid: None,
            last_editor_uid: None,
            current_editor_uid: None,
        };
        let workflow: ServerWorkflow =
            mock_server_workflow(workflow_id, Owner::mock_current_user(), workflow_metadata);

        CloudModel::handle(&app).update(&mut app, |cloud_model, _| {
            cloud_model.add_object(sync_id, CloudWorkflow::new_from_server(workflow));
        });

        // mock an unsuccessful untrashing attempt
        server_api
            .expect_untrash_object()
            .times(1)
            .return_once(move |_| Ok(ObjectMetadataUpdateResult::Failure));

        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));

        // unsuccessfully attempt to untrash the workflow
        let type_and_id = CloudObjectTypeAndId::from_id_and_type(sync_id, ObjectType::Workflow);
        update_manager_struct
            .update_manager
            .update(&mut app, |update_manager, ctx| {
                update_manager.untrash_object(type_and_id, ctx);
                ctx.await_spawned_future(update_manager.spawned_futures[0])
            })
            .await;

        // check that object is still in trash
        assert_trashed_status_for_object(&mut app, &sync_id.uid(), true);
        assert_pending_status_for_object(&mut app, &sync_id.uid(), false);

        // We do not optimistically update trashed_ts when untrashing, so there should be no event.
        assert!(cloud_events(&update_manager_struct).is_empty());
    })
}

#[test]
fn test_report_initial_load() {
    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);
        let server_api = mock_server_api();
        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));

        // Before the initial load, the listener should be pending.
        let mut listener = Box::pin(
            update_manager_struct
                .update_manager
                .read(&app, |update_manager, _| {
                    update_manager.initial_load_complete()
                }),
        );
        assert!(future::poll_once(&mut listener).await.is_none());

        receive_initial_load_or_polling_update(
            &mut app,
            &update_manager_struct.update_manager,
            false, /* force_refresh */
            InitialLoadResponse {
                updated_notebooks: Default::default(),
                deleted_notebooks: Default::default(),
                updated_workflows: Default::default(),
                deleted_workflows: Default::default(),
                updated_folders: Default::default(),
                deleted_folders: Default::default(),
                user_profiles: Default::default(),
                updated_generic_string_objects: Default::default(),
                deleted_generic_string_objects: Default::default(),
                action_histories: Default::default(),
                mcp_gallery: Default::default(),
            },
        );

        // Afterwards, the listener should get notified.
        assert!(future::poll_once(listener).await.is_some());

        // Subsequent listeners should complete immediately.
        let listener = update_manager_struct
            .update_manager
            .read(&app, |update_manager, _| {
                update_manager.initial_load_complete()
            });
        assert!(future::poll_once(listener).await.is_some());
    });
}

#[test]
fn test_get_duplicate_object_name() {
    assert_eq!(
        get_duplicate_object_name("my object name"),
        "my object name (1)"
    );
    assert_eq!(
        get_duplicate_object_name("my object name (1)"),
        "my object name (2)"
    );
    assert_eq!(
        get_duplicate_object_name("my object name (23)"),
        "my object name (24)"
    );
    assert_eq!(
        get_duplicate_object_name("my object name(1234)"),
        "my object name(1234) (1)"
    );
    assert_eq!(
        get_duplicate_object_name("my object name (0)"),
        "my object name (1)"
    );
    assert_eq!(
        get_duplicate_object_name("my object name (-3)"),
        "my object name (-3) (1)"
    );
    assert_eq!(
        get_duplicate_object_name("my object name (18446744073709551615)"),
        "my object name (18446744073709551615) (1)"
    );
    assert_eq!(
        get_duplicate_object_name("my object name (18446744073709551616)"),
        "my object name (18446744073709551616) (1)"
    );
}

#[test]
fn test_duplicate_workflow_not_pending_no_overwrite() {
    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);

        let mut server_api = mock_server_api();
        let workflow_id: WorkflowId = WorkflowId::from(ServerId::from(123));
        let client_id = ClientId::new();
        let sync_id = SyncId::ServerId(workflow_id.into());
        let duplicate_workflow_id: WorkflowId = WorkflowId::from(ServerId::from(456));
        let duplicate_sync_id = SyncId::ServerId(duplicate_workflow_id.into());

        // Mock return two workflows from server_api:
        // - one for when original workflow is created
        // - another for when duplicate workflow is created
        mock_create_workflow(client_id, &mut server_api, workflow_id);
        mock_create_workflow(client_id, &mut server_api, duplicate_workflow_id);
        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));

        // create a workflow, workflow values here are used to validate the duplicate.
        let workflow_name = "original workflow";
        let workflow_command = "echo original workflow";
        let owner_id = Owner::Team {
            team_uid: ServerId::from(789),
        };
        let initial_folder_id = Some(SyncId::from(FolderId::from(101)));
        create_workflow_internal(
            &mut app,
            &update_manager_struct.update_manager,
            client_id,
            workflow_name.to_string(),
            workflow_command.to_string(),
            owner_id,
            initial_folder_id,
        );
        // complete the workflow create request
        SyncQueue::handle(&app)
            .update(&mut app, |sync_queue, ctx| {
                ctx.await_spawned_future(sync_queue.spawned_futures()[0])
            })
            .await;
        assert_workflow_name(&mut app, sync_id, workflow_name);

        // Duplicate the first workflow
        update_manager_struct
            .update_manager
            .update(&mut app, |update_manager, ctx| {
                update_manager.duplicate_object(&CloudObjectTypeAndId::Workflow(sync_id), ctx);
            });
        // complete the duplicate workflow create request
        SyncQueue::handle(&app)
            .update(&mut app, |sync_queue, ctx| {
                ctx.await_spawned_future(sync_queue.spawned_futures()[1])
            })
            .await;

        // Verify that duplicated workflow has expected contents/owner/folder_id
        let duplicate_workflow = get_workflow(&app, duplicate_sync_id);
        assert_eq!(
            duplicate_workflow.model().data.name(),
            format!("{workflow_name} (1)").as_str()
        );
        assert_eq!(
            duplicate_workflow.model().data.command(),
            Some(workflow_command)
        );
        assert_eq!(
            duplicate_workflow.permissions.owner,
            Owner::mock_current_user()
        );
        assert_eq!(duplicate_workflow.metadata.folder_id, initial_folder_id);

        let events = db_events(&update_manager_struct);

        // Just sanity check # of expected events (3 for each workflow creation - UpsertWorkflow, SetServerId, MarkObjectAsSynced)
        // Detailed checks of events on workflow creation is already covered in other tests.
        assert_eq!(events.len(), 8);
    });
}

#[test]
fn test_accepts_new_metadata_with_force_refresh() {
    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);
        let server_api = mock_server_api();
        let server_id: ServerId = 123.into();
        let notebook_id: NotebookId = server_id.into();
        let sync_id: SyncId = SyncId::ServerId(notebook_id.into());

        let current_metadata_ts = Utc::now();
        let metadata = ServerMetadata {
            uid: ServerId::default(),
            revision: Revision::now(),
            metadata_last_updated_ts: current_metadata_ts.into(),
            trashed_ts: None,
            folder_id: None,
            is_welcome_object: false,
            creator_uid: None,
            last_editor_uid: None,
            current_editor_uid: None,
        };
        let notebook: ServerNotebook =
            mock_server_notebook(notebook_id, Owner::mock_current_user(), metadata);

        CloudModel::handle(&app).update(&mut app, |cloud_model, _| {
            cloud_model.add_object(sync_id, CloudNotebook::new_from_server(notebook));
        });

        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));

        let mocked_metadata = ServerMetadata {
            uid: server_id,
            revision: Revision::now(),
            metadata_last_updated_ts: current_metadata_ts.into(),
            trashed_ts: None,
            folder_id: None,
            is_welcome_object: false,
            creator_uid: Some("BoogaBooga".to_string()),
            last_editor_uid: None,
            current_editor_uid: None,
        };
        let mocked_response = InitialLoadResponse {
            updated_notebooks: vec![ServerNotebook::new(
                SyncId::ServerId(server_id),
                CloudNotebookModel {
                    title: "".into(),
                    data: "".into(),
                    ai_document_id: None,
                    conversation_id: None,
                },
                mocked_metadata.clone(),
                mock_server_permissions(Owner::mock_current_user()),
            )],
            deleted_notebooks: vec![],
            updated_workflows: vec![],
            deleted_workflows: vec![],
            updated_folders: vec![],
            deleted_folders: vec![],
            user_profiles: vec![],
            updated_generic_string_objects: Default::default(),
            deleted_generic_string_objects: Default::default(),
            action_histories: Default::default(),
            mcp_gallery: Default::default(),
        };

        // Force a sync for all objects
        receive_initial_load_or_polling_update(
            &mut app,
            &update_manager_struct.update_manager,
            true, /* force_refresh */
            mocked_response,
        );

        // Assert that the metadata changes are correctly applied
        CloudModel::handle(&app).read(&app, |cloud_model, _ctx| {
            if let Some(object) = cloud_model.get_by_uid(&notebook_id.to_server_id().uid()) {
                assert_eq!(object.metadata().creator_uid, mocked_metadata.creator_uid);
            } else {
                panic!("object should have been in cloud model, but wasn't");
            }
        });

        let events = db_events(&update_manager_struct);

        // All the upserts from polling
        assert!(matches!(&events[0], ModelEvent::SyncObjectActions { .. }));
        assert!(matches!(&events[1], ModelEvent::UpsertNotebooks(_)));
        assert!(matches!(&events[2], ModelEvent::UpsertWorkflows(_)));
        assert!(matches!(&events[3], ModelEvent::UpsertFolders(_)));
        assert!(matches!(&events[4], ModelEvent::DeleteObjects { ids: _ }));
    });
}

#[test]
fn test_record_object_action() {
    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);
        let mut server_api = mock_server_api();

        let timestamp = Utc::now();

        let hashed_object_id = "Workflow-asdfasdfasdfasdfasdf21".to_string();
        let hashed_object_id_clone = hashed_object_id.clone();

        let actions: Vec<ObjectAction> = vec![
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::SingleAction {
                    timestamp,
                    processed_at_timestamp: Some(timestamp),
                    data: None,
                    pending: false,
                },
            },
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::SingleAction {
                    timestamp: timestamp - chrono::Duration::minutes(10),
                    processed_at_timestamp: Some(timestamp - chrono::Duration::minutes(10)),
                    data: None,
                    pending: false,
                },
            },
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::BundledActions {
                    count: 5,
                    oldest_timestamp: timestamp - chrono::Duration::minutes(35),
                    latest_timestamp: timestamp - chrono::Duration::minutes(15),
                    latest_processed_at_timestamp: timestamp - chrono::Duration::minutes(15),
                },
            },
        ];
        server_api
            .expect_record_object_action()
            .times(1)
            .return_once(move |_, _, _, _| {
                Ok(ObjectActionHistory {
                    uid: hashed_object_id.clone(),
                    hashed_sqlite_id: hashed_object_id.clone(),
                    latest_processed_at_timestamp: timestamp,
                    actions,
                })
            });

        // Record object action
        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));
        update_manager_struct
            .update_manager
            .update(&mut app, |update_manager, ctx| {
                update_manager.record_object_action(
                    CloudObjectTypeAndId::Workflow(SyncId::ServerId(ServerId::from_string_lossy(
                        "asdfasdfasdfasdfasdf21",
                    ))),
                    ObjectActionType::Execute,
                    None,
                    ctx,
                );
            });

        // Wait for the futures to finish
        SyncQueue::handle(&app)
            .update(&mut app, |sync_queue, ctx| {
                ctx.await_spawned_future(sync_queue.spawned_futures()[0])
            })
            .await;

        // Check that there are three actions stored.
        ObjectActions::handle(&app).update(&mut app, |model, _ctx| {
            assert_eq!(model.count_actions_for_object(&hashed_object_id_clone), 3);
        });
    });
}

#[test]
fn test_overwrite_object_action_history_no_actions_on_client() {
    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);
        let server_api = mock_server_api();

        let timestamp = Utc::now();

        let hashed_object_id = "Workflow-asdf".to_string();
        let hashed_object_id_clone = hashed_object_id.clone();

        let actions: Vec<ObjectAction> = vec![
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::SingleAction {
                    timestamp,
                    processed_at_timestamp: Some(timestamp),
                    data: None,
                    pending: false,
                },
            },
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::SingleAction {
                    timestamp: timestamp - chrono::Duration::minutes(10),
                    processed_at_timestamp: Some(timestamp - chrono::Duration::minutes(10)),
                    data: None,
                    pending: false,
                },
            },
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::BundledActions {
                    count: 5,
                    oldest_timestamp: timestamp - chrono::Duration::minutes(35),
                    latest_timestamp: timestamp - chrono::Duration::minutes(15),
                    latest_processed_at_timestamp: timestamp - chrono::Duration::minutes(15),
                },
            },
        ];

        let mock_history = ObjectActionHistory {
            uid: hashed_object_id.clone(),
            hashed_sqlite_id: hashed_object_id.clone(),
            latest_processed_at_timestamp: timestamp,
            actions,
        };

        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));
        update_manager_struct
            .update_manager
            .update(&mut app, |update_manager, ctx| {
                update_manager.maybe_overwrite_object_action_history(&mock_history, ctx);
            });

        // Check that these actions are accepted
        ObjectActions::handle(&app).update(&mut app, |model, _ctx| {
            assert_eq!(model.count_actions_for_object(&hashed_object_id_clone), 3);
        });
    });
}

#[test]
fn test_overwrite_object_action_history_reject() {
    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);
        let server_api = mock_server_api();

        let timestamp = Utc::now();

        let hashed_object_id = "Workflow-asdf".to_string();
        let hashed_object_id_clone = hashed_object_id.clone();

        // the server's most recent action was 1 minute ago
        let server_actions: Vec<ObjectAction> = vec![
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::SingleAction {
                    timestamp: timestamp - chrono::Duration::minutes(1),
                    processed_at_timestamp: Some(timestamp - chrono::Duration::minutes(1)),
                    data: None,
                    pending: false,
                },
            },
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::SingleAction {
                    timestamp: timestamp - chrono::Duration::minutes(10),
                    processed_at_timestamp: Some(timestamp - chrono::Duration::minutes(10)),
                    data: None,
                    pending: false,
                },
            },
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::SingleAction {
                    timestamp: timestamp - chrono::Duration::minutes(12),
                    processed_at_timestamp: Some(timestamp - chrono::Duration::minutes(12)),
                    data: None,
                    pending: false,
                },
            },
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::BundledActions {
                    count: 5,
                    oldest_timestamp: timestamp - chrono::Duration::minutes(35),
                    latest_timestamp: timestamp - chrono::Duration::minutes(15),
                    latest_processed_at_timestamp: timestamp - chrono::Duration::minutes(15),
                },
            },
        ];

        // The client actions have one action that is more recent than what the server is sending.
        let client_actions: Vec<ObjectAction> = vec![
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::SingleAction {
                    timestamp,
                    processed_at_timestamp: Some(timestamp),
                    data: None,
                    pending: false,
                },
            },
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::SingleAction {
                    timestamp: timestamp - chrono::Duration::minutes(1),
                    processed_at_timestamp: Some(timestamp - chrono::Duration::minutes(1)),
                    data: None,
                    pending: false,
                },
            },
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::SingleAction {
                    timestamp: timestamp - chrono::Duration::minutes(10),
                    processed_at_timestamp: Some(timestamp - chrono::Duration::minutes(10)),
                    data: None,
                    pending: false,
                },
            },
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::SingleAction {
                    timestamp: timestamp - chrono::Duration::minutes(12),
                    processed_at_timestamp: Some(timestamp - chrono::Duration::minutes(12)),
                    data: None,
                    pending: false,
                },
            },
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::BundledActions {
                    count: 5,
                    oldest_timestamp: timestamp - chrono::Duration::minutes(35),
                    latest_timestamp: timestamp - chrono::Duration::minutes(15),
                    latest_processed_at_timestamp: timestamp - chrono::Duration::minutes(15),
                },
            },
        ];

        let mock_history = ObjectActionHistory {
            uid: hashed_object_id.clone(),
            hashed_sqlite_id: hashed_object_id.clone(),
            latest_processed_at_timestamp: timestamp - chrono::Duration::minutes(1),
            actions: server_actions,
        };

        // We should have 0 actions for this object
        ObjectActions::handle(&app).update(&mut app, |model, _ctx| {
            assert_eq!(model.count_actions_for_object(&hashed_object_id_clone), 0);
        });

        // Now manually overwrite the data for this object
        ObjectActions::handle(&app).update(&mut app, |model, ctx| {
            model.overwrite_action_history_for_object(&hashed_object_id, client_actions, ctx)
        });

        // We should have 5 actions for this object
        ObjectActions::handle(&app).update(&mut app, |model, _ctx| {
            assert_eq!(model.count_actions_for_object(&hashed_object_id_clone), 5);
        });

        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));
        update_manager_struct
            .update_manager
            .update(&mut app, |update_manager, ctx| {
                update_manager.maybe_overwrite_object_action_history(&mock_history, ctx);
            });

        // Check that the new actions were rejected, and we still have 5 actions
        ObjectActions::handle(&app).update(&mut app, |model, _ctx| {
            assert_eq!(model.count_actions_for_object(&hashed_object_id_clone), 5);
        });
    });
}

#[test]
fn test_overwrite_object_action_history_ignores_pending_local_actions() {
    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);
        let server_api = mock_server_api();

        let timestamp = Utc::now();

        let hashed_object_id = "Workflow-asdf".to_string();
        let hashed_object_id_clone = hashed_object_id.clone();

        let server_actions: Vec<ObjectAction> = vec![
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::SingleAction {
                    timestamp: timestamp - chrono::Duration::minutes(1),
                    processed_at_timestamp: Some(timestamp - chrono::Duration::minutes(1)),
                    data: None,
                    pending: false,
                },
            },
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::SingleAction {
                    timestamp: timestamp - chrono::Duration::minutes(10),
                    processed_at_timestamp: Some(timestamp - chrono::Duration::minutes(10)),
                    data: None,
                    pending: false,
                },
            },
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::SingleAction {
                    timestamp: timestamp - chrono::Duration::minutes(12),
                    processed_at_timestamp: Some(timestamp - chrono::Duration::minutes(12)),
                    data: None,
                    pending: false,
                },
            },
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::BundledActions {
                    count: 5,
                    oldest_timestamp: timestamp - chrono::Duration::minutes(35),
                    latest_timestamp: timestamp - chrono::Duration::minutes(15),
                    latest_processed_at_timestamp: timestamp - chrono::Duration::minutes(15),
                },
            },
        ];

        // The client actions have one action that is more recent than what the server is sending.
        let client_actions: Vec<ObjectAction> = vec![
            // This action is pending so we should still accept the new
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::SingleAction {
                    timestamp,
                    processed_at_timestamp: Some(timestamp),
                    data: None,
                    pending: true,
                },
            },
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::SingleAction {
                    timestamp: timestamp - chrono::Duration::minutes(2),
                    processed_at_timestamp: Some(timestamp - chrono::Duration::minutes(2)),
                    data: None,
                    pending: false,
                },
            },
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::SingleAction {
                    timestamp: timestamp - chrono::Duration::minutes(10),
                    processed_at_timestamp: Some(timestamp - chrono::Duration::minutes(10)),
                    data: None,
                    pending: false,
                },
            },
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::SingleAction {
                    timestamp: timestamp - chrono::Duration::minutes(12),
                    processed_at_timestamp: Some(timestamp - chrono::Duration::minutes(12)),
                    data: None,
                    pending: false,
                },
            },
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::SingleAction {
                    timestamp: timestamp - chrono::Duration::minutes(13),
                    processed_at_timestamp: Some(timestamp - chrono::Duration::minutes(13)),
                    data: None,
                    pending: false,
                },
            },
            ObjectAction {
                uid: hashed_object_id.clone(),
                hashed_sqlite_id: hashed_object_id.clone(),
                action_type: ObjectActionType::Execute,
                action_subtype: ObjectActionSubtype::BundledActions {
                    count: 5,
                    oldest_timestamp: timestamp - chrono::Duration::minutes(35),
                    latest_timestamp: timestamp - chrono::Duration::minutes(15),
                    latest_processed_at_timestamp: timestamp - chrono::Duration::minutes(15),
                },
            },
        ];

        let mock_history = ObjectActionHistory {
            uid: hashed_object_id.clone(),
            hashed_sqlite_id: hashed_object_id.clone(),
            latest_processed_at_timestamp: timestamp - chrono::Duration::minutes(1),
            actions: server_actions,
        };

        ObjectActions::handle(&app).update(&mut app, |model, ctx| {
            model.overwrite_action_history_for_object(&hashed_object_id, client_actions, ctx)
        });

        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));
        update_manager_struct
            .update_manager
            .update(&mut app, |update_manager, ctx| {
                update_manager.maybe_overwrite_object_action_history(&mock_history, ctx);
            });

        // The new actions should be accepted, but the pending action should be persisted as well.
        ObjectActions::handle(&app).update(&mut app, |model, _ctx| {
            assert_eq!(model.count_actions_for_object(&hashed_object_id_clone), 5);
        });
    });
}

#[test]
fn test_object_action_histories_with_initial_load() {
    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);
        let server_api = mock_server_api();

        let workflow_id_a: SyncId =
            SyncId::ServerId(ServerId::from_string_lossy("JLKS23FSJLKS23FSJLKS23"));
        let workflow_id_b: SyncId =
            SyncId::ServerId(ServerId::from_string_lossy("LSJLK23fZDLSJLK23fZDLS"));
        let workflow_id_c: SyncId =
            SyncId::ServerId(ServerId::from_string_lossy("SDFlJ23SDfSDFlJ23SDfSD"));

        let timestamp = Utc::now();
        let timestamp_old = Utc::now() - chrono::Duration::seconds(1);
        let timestamp_older = Utc::now() - chrono::Duration::seconds(2);
        let timestamp_oldest = Utc::now() - chrono::Duration::seconds(3);

        let actions_a = vec![ObjectAction {
            action_type: ObjectActionType::Execute,
            uid: workflow_id_a.uid(),
            hashed_sqlite_id: workflow_id_a.uid(),
            action_subtype: ObjectActionSubtype::SingleAction {
                timestamp: timestamp_old,
                processed_at_timestamp: Some(timestamp_old),
                data: None,
                pending: false,
            },
        }];
        let actions_b = vec![ObjectAction {
            action_type: ObjectActionType::Execute,
            uid: workflow_id_a.uid(),
            hashed_sqlite_id: workflow_id_a.uid(),
            action_subtype: ObjectActionSubtype::SingleAction {
                timestamp: timestamp_older,
                processed_at_timestamp: Some(timestamp_older),
                data: None,
                pending: false,
            },
        }];

        ObjectActions::handle(&app).update(&mut app, |object_actions, ctx| {
            object_actions.overwrite_action_history_for_object(
                &workflow_id_a.uid(),
                actions_a,
                ctx,
            );

            object_actions.overwrite_action_history_for_object(
                &workflow_id_b.uid(),
                actions_b,
                ctx,
            );
        });

        let actions_a_server = vec![
            ObjectAction {
                action_type: ObjectActionType::Execute,
                uid: workflow_id_a.uid(),
                hashed_sqlite_id: workflow_id_a.uid(),
                action_subtype: ObjectActionSubtype::SingleAction {
                    timestamp,
                    processed_at_timestamp: Some(timestamp),
                    data: None,
                    pending: false,
                },
            },
            ObjectAction {
                action_type: ObjectActionType::Execute,
                uid: workflow_id_a.uid(),
                hashed_sqlite_id: workflow_id_a.uid(),
                action_subtype: ObjectActionSubtype::SingleAction {
                    timestamp: timestamp_old,
                    processed_at_timestamp: Some(timestamp_old),
                    data: None,
                    pending: false,
                },
            },
        ];

        let actions_b_server = vec![
            ObjectAction {
                action_type: ObjectActionType::Execute,
                uid: workflow_id_a.uid(),
                hashed_sqlite_id: workflow_id_a.uid(),
                action_subtype: ObjectActionSubtype::SingleAction {
                    timestamp: timestamp_old,
                    processed_at_timestamp: Some(timestamp_old),
                    data: None,
                    pending: false,
                },
            },
            ObjectAction {
                action_type: ObjectActionType::Execute,
                uid: workflow_id_a.uid(),
                hashed_sqlite_id: workflow_id_a.uid(),
                action_subtype: ObjectActionSubtype::SingleAction {
                    timestamp: timestamp_older,
                    processed_at_timestamp: Some(timestamp_older),
                    data: None,
                    pending: false,
                },
            },
        ];

        let actions_c_server = vec![ObjectAction {
            action_type: ObjectActionType::Execute,
            uid: workflow_id_a.uid(),
            hashed_sqlite_id: workflow_id_a.uid(),
            action_subtype: ObjectActionSubtype::SingleAction {
                timestamp: timestamp_oldest,
                processed_at_timestamp: Some(timestamp_oldest),
                data: None,
                pending: false,
            },
        }];

        let server_action_histories = vec![
            ObjectActionHistory {
                uid: workflow_id_a.uid(),
                hashed_sqlite_id: workflow_id_a.uid(),
                latest_processed_at_timestamp: timestamp,
                actions: actions_a_server,
            },
            ObjectActionHistory {
                uid: workflow_id_b.uid(),
                hashed_sqlite_id: workflow_id_b.uid(),
                latest_processed_at_timestamp: timestamp_old,
                actions: actions_b_server,
            },
            ObjectActionHistory {
                uid: workflow_id_c.uid(),
                hashed_sqlite_id: workflow_id_c.uid(),
                latest_processed_at_timestamp: timestamp_oldest,
                actions: actions_c_server,
            },
        ];
        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));

        let mocked_response = InitialLoadResponse {
            updated_notebooks: vec![],
            deleted_notebooks: vec![],
            updated_workflows: vec![],
            deleted_workflows: vec![],
            updated_folders: vec![],
            deleted_folders: vec![],
            user_profiles: vec![],
            updated_generic_string_objects: Default::default(),
            deleted_generic_string_objects: Default::default(),
            action_histories: server_action_histories,
            mcp_gallery: Default::default(),
        };
        receive_initial_load_or_polling_update(
            &mut app,
            &update_manager_struct.update_manager,
            false, /* force_refresh */
            mocked_response,
        );

        // Assert new ObjectAction state
        ObjectActions::handle(&app).update(&mut app, |object_actions, _| {
            assert_eq!(
                object_actions.count_actions_for_object(&workflow_id_a.uid()),
                2
            );
            assert_eq!(
                object_actions.count_actions_for_object(&workflow_id_b.uid()),
                2
            );
            assert_eq!(
                object_actions.count_actions_for_object(&workflow_id_c.uid()),
                1
            );
        });

        let events = db_events(&update_manager_struct);

        // All the upserts from polling
        assert!(matches!(&events[0], ModelEvent::SyncObjectActions { .. }));
        assert!(matches!(&events[1], ModelEvent::UpsertNotebooks(_)));
        assert!(matches!(&events[2], ModelEvent::UpsertWorkflows(_)));
        assert!(matches!(&events[3], ModelEvent::UpsertFolders(_)));
        assert!(matches!(&events[4], ModelEvent::DeleteObjects { ids: _ }));
    });
}

/// Tests that the cloud model is updated correctly if we receive an RTC message indicating that an
/// object was trashed by another client.
#[test]
fn test_trash_object_over_rtc() {
    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);
        let server_api = mock_server_api();
        let workflow_id: WorkflowId = 123.into();
        let sync_id = SyncId::ServerId(workflow_id.into());

        let current_metadata_ts = Utc::now();
        let current_metadata = ServerMetadata {
            uid: ServerId::default(),
            revision: Revision::now(),
            metadata_last_updated_ts: current_metadata_ts.into(),
            trashed_ts: None,
            folder_id: None,
            is_welcome_object: false,
            creator_uid: None,
            last_editor_uid: None,
            current_editor_uid: None,
        };

        CloudModel::handle(&app).update(&mut app, |cloud_model, _| {
            cloud_model.add_object(
                sync_id,
                CloudWorkflow::new_from_server(mock_server_workflow(
                    workflow_id,
                    Owner::mock_current_user(),
                    current_metadata,
                )),
            );
        });

        assert_trashed_status_for_object(&mut app, &sync_id.uid(), false);

        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));

        let new_metadata_ts = current_metadata_ts + chrono::Duration::seconds(1);
        let new_metadata = ServerMetadata {
            uid: workflow_id.into(),
            revision: Revision::now(),
            metadata_last_updated_ts: new_metadata_ts.into(),
            trashed_ts: Some(new_metadata_ts.into()),
            folder_id: None,
            is_welcome_object: false,
            creator_uid: None,
            last_editor_uid: None,
            current_editor_uid: None,
        };
        receive_object_update_from_rtc(
            &mut app,
            &update_manager_struct.update_manager,
            ObjectUpdateMessage::ObjectMetadataChanged {
                metadata: new_metadata,
            },
        );

        // The metadata changes should be applied in-memory and to the database.
        let type_and_id = CloudObjectTypeAndId::from_id_and_type(sync_id, ObjectType::Workflow);
        assert_trashed_status_for_object(&mut app, &sync_id.uid(), true);
        assert_eq!(
            cloud_events(&update_manager_struct),
            vec![CloudModelEvent::ObjectTrashed {
                type_and_id,
                source: UpdateSource::Server
            }]
        );
        let events = db_events(&update_manager_struct);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            ModelEvent::UpdateObjectMetadata { id, .. } if id == &sync_id.sqlite_uid_hash(ObjectIdType::Workflow)
        ));
    });
}

/// Tests that the cloud model is updated correctly if we receive an RTC message indicating that an
/// object was un-trashed by another client.
#[test]
fn test_untrash_object_over_rtc() {
    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);
        let server_api = mock_server_api();
        let workflow_id: WorkflowId = 123.into();
        let sync_id = SyncId::ServerId(workflow_id.into());

        let current_metadata_ts = Utc::now();
        let current_metadata = ServerMetadata {
            uid: ServerId::default(),
            revision: Revision::now(),
            metadata_last_updated_ts: current_metadata_ts.into(),
            trashed_ts: Some(current_metadata_ts.into()),
            folder_id: None,
            is_welcome_object: false,
            creator_uid: None,
            last_editor_uid: None,
            current_editor_uid: None,
        };

        CloudModel::handle(&app).update(&mut app, |cloud_model, _| {
            cloud_model.add_object(
                sync_id,
                CloudWorkflow::new_from_server(mock_server_workflow(
                    workflow_id,
                    Owner::mock_current_user(),
                    current_metadata,
                )),
            );
        });

        assert_trashed_status_for_object(&mut app, &sync_id.uid(), true);

        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));

        let new_metadata_ts = current_metadata_ts + chrono::Duration::seconds(1);
        let new_metadata = ServerMetadata {
            uid: workflow_id.into(),
            revision: Revision::now(),
            metadata_last_updated_ts: new_metadata_ts.into(),
            trashed_ts: None,
            folder_id: None,
            is_welcome_object: false,
            creator_uid: None,
            last_editor_uid: None,
            current_editor_uid: None,
        };
        receive_object_update_from_rtc(
            &mut app,
            &update_manager_struct.update_manager,
            ObjectUpdateMessage::ObjectMetadataChanged {
                metadata: new_metadata,
            },
        );

        // The metadata changes should be applied in-memory and to the database.
        let type_and_id = CloudObjectTypeAndId::from_id_and_type(sync_id, ObjectType::Workflow);
        assert_trashed_status_for_object(&mut app, &sync_id.uid(), false);
        assert_eq!(
            cloud_events(&update_manager_struct),
            vec![CloudModelEvent::ObjectUntrashed {
                type_and_id,
                source: UpdateSource::Server
            }]
        );
        let events = db_events(&update_manager_struct);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            ModelEvent::UpdateObjectMetadata { id, .. } if id == &sync_id.sqlite_uid_hash(ObjectIdType::Workflow)
        ));
    });
}

/// Tests that the cloud model is correctly updated if we receive an RTC message indicating that an
/// object was moved from one folder to another in the same space by another client.
#[test]
fn test_move_object_from_folder_to_folder_over_rtc() {
    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);
        let server_api = mock_server_api();
        let workflow_id: WorkflowId = 123.into();
        let sync_id = SyncId::ServerId(workflow_id.into());
        let folder_a_id: FolderId = 456.into();
        let folder_b_id: FolderId = 789.into();

        let current_metadata_ts = Utc::now();
        let current_metadata = ServerMetadata {
            uid: ServerId::default(),
            revision: Revision::now(),
            metadata_last_updated_ts: current_metadata_ts.into(),
            trashed_ts: None,
            folder_id: Some(folder_a_id),
            is_welcome_object: false,
            creator_uid: None,
            last_editor_uid: None,
            current_editor_uid: None,
        };

        CloudModel::handle(&app).update(&mut app, |cloud_model, _| {
            cloud_model.add_object(
                sync_id,
                CloudWorkflow::new_from_server(mock_server_workflow(
                    workflow_id,
                    Owner::mock_current_user(),
                    current_metadata,
                )),
            );
        });

        assert_folder_for_object(&app, &sync_id.uid(), Some(folder_a_id.into()));

        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));

        let new_metadata_ts = current_metadata_ts + chrono::Duration::seconds(1);
        let new_metadata = ServerMetadata {
            uid: workflow_id.into(),
            revision: Revision::now(),
            metadata_last_updated_ts: new_metadata_ts.into(),
            trashed_ts: None,
            folder_id: Some(folder_b_id),
            is_welcome_object: false,
            creator_uid: None,
            last_editor_uid: None,
            current_editor_uid: None,
        };
        receive_object_update_from_rtc(
            &mut app,
            &update_manager_struct.update_manager,
            ObjectUpdateMessage::ObjectMetadataChanged {
                metadata: new_metadata,
            },
        );

        // The metadata changes should be applied in-memory and to the database.
        let type_and_id = CloudObjectTypeAndId::from_id_and_type(sync_id, ObjectType::Workflow);
        assert_folder_for_object(&app, &sync_id.uid(), Some(folder_b_id.into()));
        assert_eq!(
            cloud_events(&update_manager_struct),
            vec![CloudModelEvent::ObjectMoved {
                type_and_id,
                source: UpdateSource::Server,
                from_folder: Some(folder_a_id.into()),
                to_folder: Some(folder_b_id.into()),
            }]
        );
        let events = db_events(&update_manager_struct);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            ModelEvent::UpdateObjectMetadata { id, .. } if id == &sync_id.sqlite_uid_hash(ObjectIdType::Workflow)
        ));
    });
}

/// Tests that the cloud model is updated correctly if we receive an RTC message indicating that an
/// object was moved from a folder to the root of its space by another client.
#[test]
fn test_move_object_from_folder_to_root_over_rtc() {
    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);
        let server_api = mock_server_api();
        let workflow_id: WorkflowId = 123.into();
        let sync_id = SyncId::ServerId(workflow_id.into());
        let folder_id: FolderId = 456.into();

        let current_metadata_ts = Utc::now();
        let current_metadata = ServerMetadata {
            uid: ServerId::default(),
            revision: Revision::now(),
            metadata_last_updated_ts: current_metadata_ts.into(),
            trashed_ts: None,
            folder_id: Some(folder_id),
            is_welcome_object: false,
            creator_uid: None,
            last_editor_uid: None,
            current_editor_uid: None,
        };

        CloudModel::handle(&app).update(&mut app, |cloud_model, _| {
            cloud_model.add_object(
                sync_id,
                CloudWorkflow::new_from_server(mock_server_workflow(
                    workflow_id,
                    Owner::mock_current_user(),
                    current_metadata,
                )),
            );
        });

        assert_folder_for_object(&app, &sync_id.uid(), Some(folder_id.into()));

        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));

        let new_metadata_ts = current_metadata_ts + chrono::Duration::seconds(1);
        let new_metadata = ServerMetadata {
            uid: workflow_id.into(),
            revision: Revision::now(),
            metadata_last_updated_ts: new_metadata_ts.into(),
            trashed_ts: None,
            folder_id: None,
            is_welcome_object: false,
            creator_uid: None,
            last_editor_uid: None,
            current_editor_uid: None,
        };
        receive_object_update_from_rtc(
            &mut app,
            &update_manager_struct.update_manager,
            ObjectUpdateMessage::ObjectMetadataChanged {
                metadata: new_metadata,
            },
        );

        // The metadata changes should be applied in-memory and to the database.
        let type_and_id = CloudObjectTypeAndId::from_id_and_type(sync_id, ObjectType::Workflow);
        assert_folder_for_object(&app, &sync_id.uid(), None);
        assert_eq!(
            cloud_events(&update_manager_struct),
            vec![CloudModelEvent::ObjectMoved {
                type_and_id,
                source: UpdateSource::Server,
                from_folder: Some(folder_id.into()),
                to_folder: None,
            }]
        );
        let events = db_events(&update_manager_struct);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            ModelEvent::UpdateObjectMetadata { id, .. } if id == &sync_id.sqlite_uid_hash(ObjectIdType::Workflow)
        ));
    });
}

/// Tests that we update the cloud model correctly after receiving an RTC message indicating that
/// an object was moved from the root of its space into a folder by another client.
#[test]
fn test_move_object_from_root_to_folder_over_rtc() {
    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);
        let server_api = mock_server_api();
        let workflow_id: WorkflowId = 123.into();
        let sync_id = SyncId::ServerId(workflow_id.into());
        let folder_id: FolderId = 456.into();

        let current_metadata_ts = Utc::now();
        let current_metadata = ServerMetadata {
            uid: ServerId::default(),
            revision: Revision::now(),
            metadata_last_updated_ts: current_metadata_ts.into(),
            trashed_ts: None,
            folder_id: None,
            is_welcome_object: false,
            creator_uid: None,
            last_editor_uid: None,
            current_editor_uid: None,
        };

        CloudModel::handle(&app).update(&mut app, |cloud_model, _| {
            cloud_model.add_object(
                sync_id,
                CloudWorkflow::new_from_server(mock_server_workflow(
                    workflow_id,
                    Owner::mock_current_user(),
                    current_metadata,
                )),
            );
        });

        assert_folder_for_object(&app, &sync_id.uid(), None);

        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));

        let new_metadata_ts = current_metadata_ts + chrono::Duration::seconds(1);
        let new_metadata = ServerMetadata {
            uid: workflow_id.into(),
            revision: Revision::now(),
            metadata_last_updated_ts: new_metadata_ts.into(),
            trashed_ts: None,
            folder_id: Some(folder_id),
            is_welcome_object: false,
            creator_uid: None,
            last_editor_uid: None,
            current_editor_uid: None,
        };
        receive_object_update_from_rtc(
            &mut app,
            &update_manager_struct.update_manager,
            ObjectUpdateMessage::ObjectMetadataChanged {
                metadata: new_metadata,
            },
        );

        // The metadata changes should be applied in-memory and to the database.
        let type_and_id = CloudObjectTypeAndId::from_id_and_type(sync_id, ObjectType::Workflow);
        assert_folder_for_object(&app, &sync_id.uid(), Some(folder_id.into()));
        assert_eq!(
            cloud_events(&update_manager_struct),
            vec![CloudModelEvent::ObjectMoved {
                type_and_id,
                source: UpdateSource::Server,
                from_folder: None,
                to_folder: Some(folder_id.into()),
            }]
        );
        let events = db_events(&update_manager_struct);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            ModelEvent::UpdateObjectMetadata { id, .. } if id == &sync_id.sqlite_uid_hash(ObjectIdType::Workflow)
        ));
    });
}

#[test]
fn test_permissions_update_existing_object() {
    let _guard = FeatureFlag::SharedWithMe.override_enabled(true);

    App::test(ASSETS, |mut app| async move {
        initialize_app(&mut app);

        let server_api = mock_server_api();
        let notebook_id: NotebookId = 123.into();
        let sync_id = SyncId::ServerId(notebook_id.into());

        // Model the object already existing in memory.
        let original_server_notebook = mock_server_notebook(
            notebook_id,
            Owner::mock_current_user(),
            ServerMetadata {
                uid: notebook_id.into(),
                revision: Revision::now(),
                metadata_last_updated_ts: Utc::now().into(),
                trashed_ts: None,
                folder_id: None,
                is_welcome_object: false,
                creator_uid: None,
                last_editor_uid: None,
                current_editor_uid: None,
            },
        );
        CloudModel::handle(&app).update(&mut app, |cloud_model, _| {
            cloud_model.add_object(
                sync_id,
                CloudNotebook::new_from_server(original_server_notebook),
            );
        });

        let update_manager_struct = create_update_manager_struct(&mut app, Arc::new(server_api));

        assert_space_for_object(&app, &sync_id.uid(), Space::Personal);

        // Receive an RTC update that moves the object.
        receive_object_update_from_rtc(
            &mut app,
            &update_manager_struct.update_manager,
            ObjectUpdateMessage::ObjectPermissionsChangedV2 {
                object_uid: notebook_id.into(),
                permissions: mock_server_permissions(Owner::Team {
                    team_uid: ServerId::from(99),
                }),
                user_profiles: vec![],
            },
        );

        // The object's space should change.
        assert_space_for_object(&app, &sync_id.uid(), Space::Shared);

        let events = db_events(&update_manager_struct);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], ModelEvent::UpsertNotebook { .. }));

        // We don't currently emit CloudModel events for permission changes - should we?
    });
}
