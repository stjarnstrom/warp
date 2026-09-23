use warpui::{AppContext, Entity, SingletonEntity};

use super::persistence::CloudModel;
use crate::cloud_object::Space;
use crate::server::ids::ObjectUid;

/// Singleton model for storing and querying the data and logic logic needed by various view, based on the information
/// stored in [CloudModel]. As a general, rule, any new API that requires logic beyond just retrieving the raw value
/// in [CloudModel], should be stored here.
pub struct CloudViewModel;

impl CloudViewModel {
    /// Get the [`Space`] that contains an object.
    pub fn object_space(&self, id: &ObjectUid, app: &AppContext) -> Option<Space> {
        CloudModel::as_ref(app)
            .get_by_uid(id)
            .map(|object| object.space(app))
    }
}

impl Entity for CloudViewModel {
    type Event = ();
}

/// Mark CloudViewModel as global application state.
impl SingletonEntity for CloudViewModel {}
