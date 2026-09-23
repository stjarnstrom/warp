use std::pin::pin;
use std::sync::Arc;

use futures::future::join_all;
use futures_lite::StreamExt;
use warpui::{App, ModelHandle};

use super::{HistoryEntry, HistoryEvent};
use crate::terminal::History;
use crate::terminal::model::session::command_executor::testing::TestCommandExecutor;
use crate::terminal::model::session::{Session, SessionId, SessionInfo};

impl History {
    /// Returns a Future that completes when `History` is initialized for all sessions with IDs in
    /// the given `session_ids` vector.
    pub async fn initialized_sessions(
        history_handle: &mut ModelHandle<History>,
        app: &mut App,
        session_ids: Vec<SessionId>,
    ) {
        let mut history_initialization_receivers = vec![];
        for session_id in session_ids {
            let is_session_initialized = history_handle.read(app, |history, _| {
                history.is_session_initialized(&session_id)
            });
            if !is_session_initialized {
                let (tx, rx) = async_channel::unbounded();
                let history_handle_clone = history_handle.clone();
                app.update(|ctx| {
                    ctx.subscribe_to_model(&history_handle_clone, move |_, event, _| match event {
                        HistoryEvent::Initialized(event_id) => {
                            if session_id == *event_id {
                                let _ = tx.try_send(());
                            }
                        }
                    });
                });
                history_initialization_receivers.push(rx);
            }
        }

        join_all(
            history_initialization_receivers
                .into_iter()
                .map(|rx| async move { rx.recv().await }),
        )
        .await;
    }
}

impl HistoryEntry {}

#[test]
fn is_appendable_vs_is_queryable() {
    App::test((), |mut app| async move {
        let mut history_handle = app.add_model(|_| History::new(vec![]));
        let (tx, rx) = async_channel::bounded(1);

        let session = Arc::new(Session::new(
            SessionInfo::new_for_test().with_id(0),
            Arc::new(TestCommandExecutor::default()),
        ));
        let session_id = session.id();
        history_handle.update(&mut app, move |history, ctx| {
            history.init_session_with(
                session,
                async move {
                    pin!(rx).next().await;
                    vec![]
                },
                ctx,
            );
        });

        // When the session has just been registered with the history model,
        // its history will be appendable but not queryable.
        history_handle.read(&app, |history, _| {
            assert!(history.is_appendable(&session_id));
            assert!(!history.is_queryable(&session_id));
        });

        // Simulate the asynchronous reading of the histfile.
        tx.send(()).await.unwrap();
        History::initialized_sessions(&mut history_handle, &mut app, vec![session_id]).await;

        // Once we've read the histfile, the model should be appendable and queryable.
        history_handle.read(&app, |history, _| {
            assert!(history.is_appendable(&session_id));
            assert!(history.is_queryable(&session_id));
        });
    });
}
