// Most legacy internals still use include! to preserve their provisional private helper surface.
// Newer slices use ordinary modules when their boundaries are narrow enough to keep explicit.
include!("imports.rs");
include!("model.rs");
include!("builder.rs");
include!("error.rs");
include!("index_build.rs");
include!("index.rs");
mod snapshot;
pub use snapshot::*;
include!("storage.rs");
include!("relations.rs");
include!("document_mapping.rs");
include!("relation_test_support.rs");
include!("identity.rs");
include!("text_search.rs");
include!("filesystem.rs");
include!("tests.rs");
