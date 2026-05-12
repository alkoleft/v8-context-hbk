// Internal implementation is split by responsibility; include! keeps the provisional public
// surface and private helper visibility unchanged for this behavior-preserving T151 pass.
include!("imports.rs");
include!("model.rs");
include!("extract.rs");
include!("shlang.rs");
include!("shquery.rs");
include!("dcsui.rs");
include!("fact_builder.rs");
include!("html_helpers.rs");
include!("tests.rs");
