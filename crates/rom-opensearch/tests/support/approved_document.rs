//! Approved original-key document fixture.
use rom::{JournalView, Key, ProjectedView};
use rom_projection_core::DocumentMapping;
use serde_json::json;
pub(crate) fn document(
    mapping: &DocumentMapping,
    revision: u64,
    title: Option<&str>,
) -> rom_projection_core::ApprovedDocument {
    mapping
        .document(
            &JournalView {
                position: revision,
                view: ProjectedView {
                    key: Key {
                        kind: "native_docs".into(),
                        id: "a/雪".into(),
                    },
                    revision,
                    value: title.map(|s| {
                        json!({"title":s, "other":18446744073709551615u64})
                            .as_object()
                            .unwrap()
                            .clone()
                    }),
                },
            },
            None,
        )
        .unwrap()
}
