use std::{collections::HashSet, path::Path};
use thiserror::Error;
use wasmtime::{Config, Engine, Linker, Module, Store};
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Capability {
    Log,
    ReadResource,
    EmitEvent,
    SpawnEntity,
    QueryWorld,
    Admin,
}

#[derive(Debug, Error)]
pub enum WasmError {
    #[error("wasm: {0}")]
    Runtime(String),
    #[error("capability denied")]
    Denied,
}

pub struct PluginState {
    pub capabilities: HashSet<Capability>,
    pub fuel_per_tick: u64,
    pub logs: Vec<String>,
}

pub struct WasmPlugin {
    store: Store<PluginState>,
    instance: wasmtime::Instance,
}

impl WasmPlugin {
    pub fn load(
        path: &Path,
        capabilities: HashSet<Capability>,
        fuel: u64,
    ) -> Result<Self, WasmError> {
        let mut c = Config::new();
        c.consume_fuel(true);
        c.epoch_interruption(true);
        let engine = Engine::new(&c).map_err(|e| WasmError::Runtime(e.to_string()))?;
        let module =
            Module::from_file(&engine, path).map_err(|e| WasmError::Runtime(e.to_string()))?;
        let mut linker = Linker::new(&engine);
        linker
            .func_wrap(
                "honknet",
                "log",
                |mut caller: wasmtime::Caller<'_, PluginState>, value: i32| {
                    if caller.data().capabilities.contains(&Capability::Log) {
                        caller.data_mut().logs.push(format!("wasm:{value}"));
                    }
                },
            )
            .map_err(|e| WasmError::Runtime(e.to_string()))?;
        let mut store = Store::new(
            &engine,
            PluginState {
                capabilities,
                fuel_per_tick: fuel,
                logs: vec![],
            },
        );
        store
            .set_fuel(fuel)
            .map_err(|e| WasmError::Runtime(e.to_string()))?;
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| WasmError::Runtime(e.to_string()))?;
        Ok(Self { store, instance })
    }
    pub fn tick(&mut self, tick: u64) -> Result<(), WasmError> {
        self.store
            .set_fuel(self.store.data().fuel_per_tick)
            .map_err(|e| WasmError::Runtime(e.to_string()))?;
        if let Ok(f) = self
            .instance
            .get_typed_func::<i64, ()>(&mut self.store, "honknet_tick")
        {
            f.call(&mut self.store, tick as i64)
                .map_err(|e| WasmError::Runtime(e.to_string()))?
        }
        Ok(())
    }
    pub fn logs(&self) -> &[String] {
        &self.store.data().logs
    }
}
