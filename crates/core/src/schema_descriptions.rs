include!("../../../api/schema-descriptions.rs");

pub(crate) struct SchemaDescriptions;

impl utoipa::Modify for SchemaDescriptions {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        apply_descriptions(openapi, include_str!("schema-descriptions.json"));
    }
}
