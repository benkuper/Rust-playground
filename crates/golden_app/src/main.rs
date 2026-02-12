use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use golden_core::edits::{Edit, EditOrigin, Propagation};
use golden_core::persistence::{export_project, save_project};
use golden_core::schema::{DeclaredChild, NodeSchema, PotentialSlot};
use golden_core::{
    AllowedTypes, ChangePolicy, ContainerData, ContainerLimits, Engine, FolderPolicy,
    NodeBehaviour, NodeData, NodeExecution, ParameterData, SavePolicy, UpdatePolicy, Value,
    ValueConstraints,
};
use golden_net::{AppServerConfig, start_app_server};
use golden_schema::NodeMetaPatch;
use golden_schema::NodeTypeId;
use uuid::Uuid;

struct LogBehaviour {
    name: String,
}

impl NodeBehaviour for LogBehaviour {
    fn process(&mut self, ctx: &mut golden_core::ProcessCtx) {
        if ctx.inbox.is_empty() {
            return;
        }
        for event in &ctx.inbox {
            println!("[{name}] event: {event:?}", name = self.name);
        }
    }
}

fn create_container(engine: &mut Engine, node_type: &str, label: &str) -> golden_schema::NodeId {
    engine.create_node(
        NodeTypeId(node_type.to_string()),
        NodeExecution::Reactive,
        NodeData::Container(ContainerData {
            allowed_types: AllowedTypes::Any,
            folders: FolderPolicy::Allowed,
            limits: ContainerLimits { max_children: None },
        }),
        engine.create_meta(label),
        Some(Box::new(LogBehaviour {
            name: label.to_string(),
        })),
    )
}

fn create_param(engine: &mut Engine, label: &str, value: Value) -> golden_schema::NodeId {
    engine.create_node(
        NodeTypeId("Parameter".to_string()),
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

fn print_tree(engine: &Engine, node_id: golden_schema::NodeId, indent: usize) {
    let Some(node) = engine.nodes.get(&node_id) else {
        println!(
            "{space}- <missing node {id:?}>",
            space = " ".repeat(indent),
            id = node_id
        );
        return;
    };

    let details = match &node.data {
        NodeData::None => "none".to_string(),
        NodeData::Container(_) => "container".to_string(),
        NodeData::Parameter(param) => format!("param={:?}", param.value),
        NodeData::Custom(_) => "custom".to_string(),
    };

    println!(
        "{space}- {label} [{kind}] {details}",
        space = " ".repeat(indent),
        label = node.meta.label,
        kind = node.node_type.0,
        details = details
    );

    let mut child = node.first_child;
    while let Some(child_id) = child {
        print_tree(engine, child_id, indent + 2);
        child = engine
            .nodes
            .get(&child_id)
            .and_then(|child_node| child_node.next_sibling);
    }
}

#[tokio::main]
async fn main() {
    let mut engine = Engine::new();

    let mut osc_schema = NodeSchema::new();
    osc_schema.declared_children = vec![
        DeclaredChild {
            decl_id: golden_schema::DeclId("intensity".to_string()),
            node_type: NodeTypeId("Parameter".to_string()),
            default_label: Some("intensity".to_string()),
            default_enabled: true,
        },
        DeclaredChild {
            decl_id: golden_schema::DeclId("enabled".to_string()),
            node_type: NodeTypeId("Parameter".to_string()),
            default_label: Some("enabled".to_string()),
            default_enabled: true,
        },
        DeclaredChild {
            decl_id: golden_schema::DeclId("host".to_string()),
            node_type: NodeTypeId("Parameter".to_string()),
            default_label: Some("host".to_string()),
            default_enabled: true,
        },
        DeclaredChild {
            decl_id: golden_schema::DeclId("port".to_string()),
            node_type: NodeTypeId("Parameter".to_string()),
            default_label: Some("port".to_string()),
            default_enabled: true,
        },
        DeclaredChild {
            decl_id: golden_schema::DeclId("target".to_string()),
            node_type: NodeTypeId("Parameter".to_string()),
            default_label: Some("target".to_string()),
            default_enabled: true,
        },
    ];
    osc_schema.potential_slots = vec![PotentialSlot {
        decl_id: golden_schema::DeclId("value".to_string()),
        allowed_types: vec![NodeTypeId("Parameter".to_string())],
    }];
    engine.register_schema(NodeTypeId("OscOutput".to_string()), osc_schema);

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
    let host_uuid = engine
        .nodes
        .get(&host)
        .map(|node| node.meta.uuid)
        .unwrap_or(golden_schema::NodeUuid(Uuid::new_v4()));
    let target = create_param(
        &mut engine,
        "target",
        Value::Reference(golden_schema::ReferenceValue {
            uuid: host_uuid,
            cached_id: Some(host),
        }),
    );
    let value_slot = create_param(&mut engine, "value", Value::Float(0.5));

    engine.add_child(osc, intensity);
    engine.add_child(osc, enabled);
    engine.add_child(osc, host);
    engine.add_child(osc, port);
    engine.add_child(osc, target);
    engine.add_child(osc, value_slot);

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
            patch: NodeMetaPatch {
                label: Some("OSC Output A".to_string()),
                ..Default::default()
            },
        },
        Propagation::EndOfTick,
        EditOrigin::UI,
    );

    engine.tick();

    println!("\nTree:");
    print_tree(&engine, root, 0);

    let project = export_project(&engine, root, "0.1");
    match save_project(&project) {
        Ok(json) => {
            println!("\nProject JSON:\n{json}");
        }
        Err(err) => {
            println!("Failed to serialize project: {err}");
        }
    }

    let engine = Arc::new(Mutex::new(engine));
    let static_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../golden_ui/build");
    let port = std::env::var("GOLDEN_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(9010);
    let config = AppServerConfig {
        addr: SocketAddr::from(([127, 0, 0, 1], port)),
        static_dir,
    };
    println!("\nServer running on http://{}", config.addr);

    let server_engine = Arc::clone(&engine);
    tokio::spawn(async move {
        if let Err(err) = start_app_server(server_engine, config).await {
            eprintln!("app server failed: {err}");
        }
    });

    println!("\nPress Ctrl+C to stop.");
    let _ = tokio::signal::ctrl_c().await;
}
