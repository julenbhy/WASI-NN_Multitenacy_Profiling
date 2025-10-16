use anyhow;
//use tokio::time;
use wasmtime::{Engine, Linker, Module, Store, Instance};
use wasmtime_wasi::p1::{self, WasiP1Ctx};
use wasmtime_wasi::{ WasiCtxBuilder, DirPerms, FilePerms };
use wasmtime_wasi_nn::witx::WasiNnCtx;



/// Represents the WebAssembly context for WASI and WASI-NN.
pub struct NnWasmCtx {
    wasi: WasiP1Ctx,
    wasi_nn: WasiNnCtx,
}
impl NnWasmCtx {
    pub fn new(wasi: WasiP1Ctx, wasi_nn: WasiNnCtx) -> Self {
        Self { wasi, wasi_nn }
    }
    pub fn wasi(&mut self) -> &mut WasiP1Ctx { &mut self.wasi }
    pub fn wasi_nn(&mut self) -> &mut WasiNnCtx { &mut self.wasi_nn }
}


pub struct WasmRuntime {
    pub id: usize,
    pub _engine: Engine,
    pub instance: Instance,
    pub store: Store<NnWasmCtx>,
}

impl WasmRuntime {
    pub fn new(id: usize, wasm_file: &str) -> anyhow::Result<Self>
    {
        //println!("Creating WASM runtime with ID: {} with wasm file: {}", id, wasm_file);

        let engine = Engine::default();

        let module = Module::from_file(&engine, wasm_file)?;

        // Create a linker and add WASI and WASI-NN to it
        let mut linker: Linker<NnWasmCtx> = Linker::new(&engine);
        p1::add_to_linker_sync(&mut linker, NnWasmCtx::wasi)?;
        wasmtime_wasi_nn::witx::add_to_linker(&mut linker, NnWasmCtx::wasi_nn)?;

        // Pass the id as an argument to the WASM module
        let args = vec![id.to_string()];


        // Create the WASI and WASI-NN contexts
        let wasi = WasiCtxBuilder::new()
            .args(&args)
            .inherit_stdout()
            .inherit_stderr()
            .inherit_env()
            .preopened_dir(".", ".", DirPerms::all(), FilePerms::all())?
            .build_p1();

        let (backends, registry) = wasmtime_wasi_nn::preload(&[])?;
        let wasi_nn = WasiNnCtx::new(backends, registry);

        let wasm_ctx = NnWasmCtx::new(wasi, wasi_nn);

        // Create a store with the WASI and WASI-NN contexts
        let mut store = Store::new(&engine, wasm_ctx);

        let instance = linker.instantiate(&mut store, &module)?;


        Ok(WasmRuntime { id, _engine:engine, instance, store })
    }
    pub fn run(&mut self) -> anyhow::Result<String> {

        println!("Getting _start function for ID: {}", self.id);
        let func = self.instance.get_typed_func::<(), ()>(&mut self.store, "_start")?;
        println!("Running WASM runtime with ID: {}", self.id);
        func.call(&mut self.store, ())?;
        println!("Finished running WASM runtime with ID: {}", self.id);
        Ok(format!("Runtime {} executed!", self.id))
    }


}