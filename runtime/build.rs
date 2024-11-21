// use substrate_wasm_builder::WasmBuilder;

fn main() {
    // WasmBuilder::new()
    //     .with_current_project()
    //     .export_heap_base()
    //     .import_memory()
    //     .build()
    #[cfg(feature = "std")]
    {
        substrate_wasm_builder::WasmBuilder::build_using_defaults();
    }
}
