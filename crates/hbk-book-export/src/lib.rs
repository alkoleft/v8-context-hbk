// Internal implementation is split by responsibility; include! keeps the provisional public
// surface and private helper visibility unchanged for this behavior-preserving T151 pass.
include!("imports.rs");
include!("request.rs");
include!("exporter.rs");
include!("link_targets.rs");
include!("markdown_loader.rs");
include!("result.rs");
include!("error.rs");
include!("planning.rs");
include!("filesystem.rs");
include!("markdown_render.rs");
include!("link_rewrite.rs");
include!("heading_anchors.rs");
include!("html_normalization.rs");
include!("heading_pages.rs");
include!("tests.rs");
