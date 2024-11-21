use substrate_wasm_builder::WasmBuilder;

#[cfg(all(feature = "std", not(feature = "metadata-hash")))]
fn main() {
	WasmBuilder::new()
		.with_current_project()
		.export_heap_base()
		.import_memory()
		.build()
}

#[cfg(all(feature = "std", feature = "metadata-hash"))]
fn main() {
	WasmBuilder::new()
		.with_current_project()
		.export_heap_base()
		.import_memory()
		.enable_metadata_hash("AGUNG", 18)
		.build();
}

#[cfg(not(feature = "std"))]
fn main() {}
