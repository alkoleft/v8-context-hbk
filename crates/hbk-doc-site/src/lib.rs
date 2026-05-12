// Internal implementation is split by responsibility; include! keeps the provisional public
// surface and private helper visibility unchanged for this behavior-preserving T151 pass.
include!("imports.rs");
include!("request.rs");
include!("ids.rs");
include!("result.rs");
include!("error.rs");
include!("generator.rs");
include!("source_discovery.rs");
include!("site_model.rs");
include!("toc_merge.rs");
include!("artifact_write.rs");
include!("stable_ids.rs");
include!("tests.rs");
