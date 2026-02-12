use std::sync::{Arc, Mutex};

use golden_app::{start_runtime, wait_for_ctrl_c, RuntimeConfig};
use golden_core::Engine;

fn main() {
    let config = RuntimeConfig::from_workspace_default();
    let engine = Arc::new(Mutex::new(Engine::new()));
    start_runtime(engine, config.clone());
    println!("Golden runtime server on http://127.0.0.1:{}", config.port);
    wait_for_ctrl_c();
}
