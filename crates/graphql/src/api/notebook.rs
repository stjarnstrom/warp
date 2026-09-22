use super::object::ObjectMetadata;
use super::object_permissions::ObjectPermissions;
use crate::schema;

#[derive(cynic::QueryFragment, Debug, Clone)]
pub struct Notebook {
    pub data: String,
    pub title: String,
    pub ai_document_id: Option<String>,
    pub metadata: ObjectMetadata,
    pub permissions: ObjectPermissions,
}
