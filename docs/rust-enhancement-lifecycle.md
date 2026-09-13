# Public Rust enhancement lifecycle

`RustCallGraphBuilder::build()` finalizes collected metadata before returning the
graph. The existing `finalize_trait_analysis() -> Result<()>` entry point remains
available; explicit finalization followed by `build()` performs that work once.
Collecting more metadata or installing a base graph invalidates finalization.
Visit pattern marking is idempotent when later inputs require another pass.

The effects wrapper initializes its base graph from the supplied AST before
enhancement. Trait roles and framework exclusions therefore refer to canonical
source definitions, including inline-module methods. Enhancement does not create
replacement definitions from unmatched metadata. Disabled trait analysis also
disables trait and constructor pattern marking during automatic finalization.

The lifecycle tests exercise the effects wrapper, existing builder chains,
explicit and automatic finalization, and collecting more files after finalizing.
This repair changes when existing enhancement runs; it does not expand the
function-pointer tracker's supported syntax or target lookup.
