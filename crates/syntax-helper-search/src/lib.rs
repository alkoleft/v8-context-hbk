// Internal implementation is split by responsibility; include! keeps the provisional public
// surface and private helper visibility unchanged for this behavior-preserving T151 pass.
include!("imports.rs");
include!("model.rs");
include!("builder.rs");
include!("error.rs");
include!("index_build.rs");
include!("index.rs");
include!("storage.rs");
include!("relations.rs");
include!("document_mapping.rs");
include!("relation_test_support.rs");
include!("identity.rs");
include!("text_search.rs");
include!("filesystem.rs");
include!("tests.rs");
