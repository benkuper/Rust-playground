use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use golden_prelude::*;
use golden_prelude::params;
use uuid::Uuid;

pub struct OscOutput {
    pub id: schema::NodeId,
    pub connection: FolderHandle,
    pub intensity: ParameterHandle<f64>,
    pub enabled: ParameterHandle<bool>,
    pub host: ParameterHandle<String>,
    pub port: ParameterHandle<i64>,
    pub drive: ParameterHandle<f64>,
    pub value: ParameterHandle<f64>,
    pub panic: ParameterHandle<Trigger>,
}

impl OscOutput {
    params! {
        intensity: f64 = 0.0 [0.0..1.0];
        enabled: bool = true;

        folder(connection) {
            host: String = "127.0.0.1";
            port: i64 = 9000 (min=1, max=65535);
        }

        drive: f64 = 0.0;
        value: f64 = 0.5;
        panic: Trigger (behavior="Append");
    }
}

impl GoldenNodeDecl for OscOutput {
    fn node_type() -> schema::NodeTypeId {
        schema::NodeTypeId("OscOutput".to_string())
    }

    fn schema() -> NodeSchema {
        let mut schema = NodeSchema::new();
        schema.declared_children = Self::declared_children();
        schema.params = Self::param_decls();
        schema.folders = Self::folder_decls();
        schema
    }
}

struct LogBehaviour {
    _name: String,
}

impl NodeBehaviour for LogBehaviour {
    fn process(&mut self, _ctx: &mut ProcessCtx) {}
}

struct ParamMapper {
    source: schema::NodeId,
    target: schema::NodeId,
}

impl NodeBehaviour for ParamMapper {
    fn process(&mut self, ctx: &mut ProcessCtx) {
        let mut changed = false;
        for event in &ctx.inbox {
            if let schema::EventKind::ParamChanged { param, .. } = &event.kind {
                if *param == self.source {
                    changed = true;
                }
            }
        }

        if changed {
            if let Some(value) = ctx.read_param(self.source) {
                if let Value::Float(v) = value {
                    ctx.set_param(self.target, Value::Float(v * 100.0));
                }
            }
        }
    }
}

struct PulseAnimator {
    _target: schema::NodeId,
}

impl NodeBehaviour for PulseAnimator {
    fn process(&mut self, _ctx: &mut ProcessCtx) {}

    fn update(&mut self, ctx: &mut ProcessCtx) {
        let phase = (ctx.time.tick % 200) as f64 / 200.0;
        let _value = (phase * std::f64::consts::TAU).sin() * 0.5 + 0.5;
        // ctx.set_param(self._target, Value::Float(value));
    }
}

fn create_container(engine: &mut Engine, node_type: &str, label: &str) -> schema::NodeId {
    engine.create_node(
        schema::NodeTypeId(node_type.to_string()),
        NodeExecution::Reactive,
        NodeData::Container(ContainerData {
            allowed_types: AllowedTypes::Any,
            folders: FolderPolicy::Allowed,
            limits: ContainerLimits { max_children: None },
        }),
        engine.create_meta(label),
        Some(Box::new(LogBehaviour {
            _name: label.to_string(),
        })),
    )
}

fn create_param(engine: &mut Engine, label: &str, value: Value) -> schema::NodeId {
    engine.create_node(
        schema::NodeTypeId("Parameter".to_string()),
        NodeExecution::Passive,
        NodeData::Parameter(ParameterData {
            value: value.clone(),
            default: Some(value),
            read_only: false,
            update: UpdatePolicy::Immediate,
            save: SavePolicy::Delta,
            change: ChangePolicy::ValueChange,
            constraints: ValueConstraints::None,
        }),
        engine.create_meta(label),
        None,
    )
}

fn build_engine() -> Engine {
    let mut engine = Engine::new();

    OscOutput::register_schema(&mut engine.schema);

    let root = create_container(&mut engine, "Root", "root");
    let outputs = create_container(&mut engine, "Outputs", "outputs");
    let devices = create_container(&mut engine, "Devices", "devices");
    let mappings = create_container(&mut engine, "Mappings", "mappings");

    engine.add_child(root, outputs);
    engine.add_child(root, devices);
    engine.add_child(root, mappings);

    let osc = create_container(&mut engine, "OscOutput", "osc_output");
    engine.add_child(outputs, osc);

    let intensity = create_param(&mut engine, "intensity", Value::Float(0.0));
    let enabled = create_param(&mut engine, "enabled", Value::Bool(true));
    let host = create_param(&mut engine, "host", Value::String("127.0.0.1".to_string()));
    let port = create_param(&mut engine, "port", Value::Int(9000));
    let drive = create_param(&mut engine, "drive", Value::Float(0.0));
    let host_uuid = engine
        .nodes
        .get(&host)
        .map(|node| node.meta.uuid)
        .unwrap_or(schema::NodeUuid(Uuid::new_v4()));
    let target = create_param(
        &mut engine,
        "target",
        Value::Reference(schema::ReferenceValue {
            uuid: host_uuid,
            cached_id: Some(host),
        }),
    );
    let value_slot = create_param(&mut engine, "value", Value::Float(0.5));

    let mapper = engine.create_node(
        schema::NodeTypeId("ParamMapper".to_string()),
        NodeExecution::Reactive,
        NodeData::None,
        engine.create_meta("mapper"),
        Some(Box::new(ParamMapper {
            source: intensity,
            target: drive,
        })),
    );

    let animator = engine.create_node(
        schema::NodeTypeId("PulseAnimator".to_string()),
        NodeExecution::Continuous,
        NodeData::None,
        engine.create_meta("pulse"),
        Some(Box::new(PulseAnimator { _target: intensity })),
    );

    engine.add_child(osc, intensity);
    engine.add_child(osc, enabled);
    engine.add_child(osc, host);
    engine.add_child(osc, port);
    engine.add_child(osc, target);
    engine.add_child(osc, value_slot);
    engine.add_child(osc, drive);
    engine.add_child(mappings, mapper);
    engine.add_child(mappings, animator);

    engine.subscribe(ListenerSpec {
        subscriber: mapper,
        filter: EventFilter::Param(intensity),
        delivery: DeliveryMode::Raw,
    });

    engine.enqueue_edit(
        Edit::SetParam {
            node: intensity,
            value: Value::Float(0.8),
        },
        Propagation::EndOfTick,
        EditOrigin::UI,
    );

    engine.enqueue_edit(
        Edit::SetParam {
            node: port,
            value: Value::Int(9100),
        },
        Propagation::EndOfTick,
        EditOrigin::UI,
    );

    engine.enqueue_edit(
        Edit::PatchMeta {
            node: osc,
            patch: schema::NodeMetaPatch {
                label: Some("OSC Output A".to_string()),
                ..Default::default()
            },
        },
        Propagation::EndOfTick,
        EditOrigin::UI,
    );

    engine.tick();

    engine
}

fn start_server(engine: Arc<Mutex<Engine>>) {
    let static_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../golden_ui/build");
    let port = std::env::var("GOLDEN_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(9010);
    let config = net::AppServerConfig {
        addr: SocketAddr::from(([127, 0, 0, 1], port)),
        static_dir,
    };
    tauri::async_runtime::spawn(async move {
        if let Err(err) = net::start_app_server(engine, config).await {
            eprintln!("app server failed: {err}");
        }
    });
}

fn start_engine_loop(engine: Arc<Mutex<Engine>>) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(16));
        loop {
            interval.tick().await;
            if let Ok(mut engine) = engine.lock() {
                engine.tick();
            }
        }
    });
}

fn is_headless() -> bool {
    std::env::args().any(|arg| arg == "--headless")
}

fn run_headless(engine: Arc<Mutex<Engine>>) {
    start_server(engine);
    let port = std::env::var("GOLDEN_PORT").unwrap_or_else(|_| "9010".to_string());
    println!("Server running on http://127.0.0.1:{port}");
    match tokio::runtime::Runtime::new() {
        Ok(rt) => {
            let _ = rt.block_on(async { tokio::signal::ctrl_c().await });
        }
        Err(err) => {
            eprintln!("Failed to start runtime: {err}");
        }
    }
}

fn main() {
    let engine = Arc::new(Mutex::new(build_engine()));
    start_server(Arc::clone(&engine));
    start_engine_loop(Arc::clone(&engine));

    if is_headless() {
        run_headless(engine);
        return;
    }

    let port = std::env::var("GOLDEN_PORT").unwrap_or_else(|_| "9010".to_string());
    println!("Launching Tauri window (UI at http://localhost:{port})");

    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
