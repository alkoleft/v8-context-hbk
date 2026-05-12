// Internal implementation is split by responsibility; include! keeps the provisional public
// surface and private helper visibility unchanged for this behavior-preserving T151 pass.
include!("imports.rs");
include!("args.rs");
include!("main_dispatch.rs");
include!("hbk_commands.rs");
include!("site_commands.rs");
include!("syntax_commands.rs");
include!("syntax_get_query.rs");
include!("text_output.rs");
include!("provider_json.rs");
include!("type_ref_gaps_output.rs");
include!("utility.rs");
include!("tests.rs");
