// Internal implementation is split by responsibility; include! keeps the provisional public
// surface and private helper visibility unchanged for this behavior-preserving T151 pass.
include!("imports.rs");
include!("platform_adapter.rs");
include!("language_adapter.rs");
include!("platform_context_source.rs");
include!("mapping.rs");
include!("tests.rs");
