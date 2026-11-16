fn apply_descriptions(openapi: &mut utoipa::openapi::OpenApi, metadata: &str) {
    use utoipa::openapi::{RefOr, schema::Schema};

    #[derive(serde::Deserialize)]
    struct Entry {
        description: String,
        paths: Vec<String>,
    }

    fn target<'a>(schema: &'a mut RefOr<Schema>, path: &[&str]) -> &'a mut RefOr<Schema> {
        if path.is_empty() {
            return schema;
        }
        let RefOr::T(schema) = schema else {
            panic!("Description path cannot traverse a reference")
        };
        match (schema, path) {
            (Schema::Object(object), ["properties", name, rest @ ..]) => target(
                object
                    .properties
                    .get_mut(*name)
                    .expect("Missing described property"),
                rest,
            ),
            (Schema::OneOf(object), ["oneOf", index, rest @ ..]) => {
                target(&mut object.items[index.parse::<usize>().unwrap()], rest)
            }
            _ => panic!("Invalid OpenAPI description path: {path:?}"),
        }
    }

    let entries: Vec<Entry> = serde_json::from_str(metadata).unwrap();
    for entry in entries {
        for path in entry.paths {
            let segments: Vec<_> = path.split('/').skip(1).collect();
            let ["components", "schemas", name, rest @ ..] = segments.as_slice() else {
                panic!("Invalid OpenAPI description path: {path}")
            };
            let schema = openapi
                .components
                .as_mut()
                .expect("Missing OpenAPI components")
                .schemas
                .get_mut(*name)
                .unwrap_or_else(|| panic!("Missing described schema: {name}"));
            let schema = match target(schema, rest) {
                RefOr::Ref(reference) => {
                    assert!(
                        reference.description.is_empty(),
                        "Duplicate OpenAPI description: {path}"
                    );
                    reference.description = entry.description.clone();
                    continue;
                }
                RefOr::T(schema) => schema,
            };
            let description = match schema {
                Schema::Object(schema) => &mut schema.description,
                Schema::Array(schema) => &mut schema.description,
                Schema::OneOf(schema) => &mut schema.description,
                Schema::AllOf(schema) => &mut schema.description,
                Schema::AnyOf(schema) => &mut schema.description,
                _ => panic!("Unsupported described schema: {path}"),
            };
            assert!(
                description.replace(entry.description.clone()).is_none(),
                "Duplicate OpenAPI description: {path}"
            );
        }
    }
}
