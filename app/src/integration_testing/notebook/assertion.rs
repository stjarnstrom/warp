use warpui::async_assert;
use warpui::integration::AssertionCallback;

use crate::cloud_object::model::generic_string_model::GenericStringObjectId;
use crate::cloud_object::model::persistence::CloudModel;
use crate::integration_testing::cloud_object::assert_metadata_revision;
use crate::integration_testing::view_getters::terminal_view;
use crate::notebooks::{CloudNotebookModel, NotebookId};
use crate::settings::{CloudPreferenceModel, Preference};

/// Asserts that there is a json preference object in the SQLite db with the given contents.
pub fn assert_cloud_preference_exists(expected_preference: Preference) -> AssertionCallback {
    Box::new(move |app, _window_id| {
        let stored_preference =
            app.get_singleton_model_handle::<CloudModel>()
                .read(app, |cloud_model, _| {
                    let object = cloud_model
                        .get_all_objects_of_type::<GenericStringObjectId, CloudPreferenceModel>()
                        .find(|p| p.model().string_model == expected_preference)
                        .expect("Expected to find a matching preference object");
                    object.model().string_model.clone()
                });
        async_assert!(
            expected_preference == stored_preference,
            "Expected json object contents to match:\n{expected_preference:?}\nBut got:\n{stored_preference:?}"
        )
    })
}

/// Asserts metadata exists for the notebook with the given key and that the revision in that
/// metadata matches the given expected revision.
pub fn assert_notebook_metadata_revision(
    id: impl AsRef<str>,
    expected_revision: i64,
) -> AssertionCallback {
    assert_metadata_revision::<NotebookId, CloudNotebookModel>(id.as_ref(), expected_revision)
}

pub fn assert_open_in_warp_banner_open(tab_index: usize, pane_index: usize) -> AssertionCallback {
    Box::new(move |app, window_id| {
        let terminal = terminal_view(app, window_id, tab_index, pane_index);
        terminal.read(app, |view, _ctx| {
            async_assert!(
                view.is_open_in_warp_banner_open(),
                "Expected the 'Open in Warp' banner to be open"
            )
        })
    })
}
