use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use golden_core::Engine;
use golden_net::{start_app_server, AppServerConfig};

#[derive(Clone, Debug)]
pub struct RuntimeConfig {
    pub port: u16,
    pub static_dir: PathBuf,
    pub tick_ms: u64,
}

impl RuntimeConfig {
    pub fn from_workspace_default() -> Self {
        let port = std::env::var("GOLDEN_PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(9010);

        Self {
            port,
            static_dir: PathBuf::from("src-ui/build"),
            tick_ms: 16,
        }
    }

    pub fn addr(&self) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], self.port))
    }
}

pub fn start_runtime(engine: Arc<Mutex<Engine>>, config: RuntimeConfig) {
    let server_engine = Arc::clone(&engine);
    let server_config = AppServerConfig {
        addr: config.addr(),
        static_dir: config.static_dir.clone(),
    };
    tauri::async_runtime::spawn(async move {
        if let Err(err) = start_app_server(server_engine, server_config).await {
            eprintln!("app server failed: {err}");
        }
    });

    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(config.tick_ms));
        loop {
            interval.tick().await;
            if let Ok(mut engine) = engine.lock() {
                engine.tick();
            }
        }
    });
}

pub fn wait_for_ctrl_c() {
    match tokio::runtime::Runtime::new() {
        Ok(rt) => {
            let _ = rt.block_on(async { tokio::signal::ctrl_c().await });
        }
        Err(err) => {
            eprintln!("Failed to start runtime: {err}");
        }
    }
}

pub fn is_headless() -> bool {
    std::env::args().any(|arg| arg == "--headless")
}

pub fn launch(engine: Engine, config: RuntimeConfig) {
    let engine = Arc::new(Mutex::new(engine));
    start_runtime(Arc::clone(&engine), config.clone());

    if is_headless() {
        println!("Server running on http://127.0.0.1:{}", config.port);
        wait_for_ctrl_c();
        return;
    }

    println!("Launching Tauri window (UI at http://localhost:{})", config.port);

    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
