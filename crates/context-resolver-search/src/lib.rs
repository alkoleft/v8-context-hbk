// Internal implementation is split by responsibility; include! keeps the provisional public
// surface and private helper visibility unchanged for this behavior-preserving T151 pass.
mod hbk_catalogs;
pub use hbk_catalogs::HbkBslContextCatalog;
include!("imports.rs");
include!("platform_adapter.rs");
include!("language_adapter.rs");
include!("platform_context_source.rs");
include!("snapshot_adapter.rs");
include!("mapping.rs");
include!("generated_self_template.rs");
include!("tests.rs");
