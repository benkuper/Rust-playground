use golden_app::RuntimeConfig;
use golden_prelude::params;
use golden_prelude::*;

#[derive(GoldenNode)]
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

impl NodeReactive for OscOutput {
    fn process(&mut self, ctx: &mut ProcessCtx) {
        //check events
    }
}

fn build_demo_engine() -> Engine {
    let mut engine = Engine::new();

    OscOutput::register_schema(&mut engine.schema);

    let root = engine.root_id();
    let outputs = engine.create_child_container(root, "Outputs", "outputs");
    let _devices = engine.create_child_container(root, "Devices", "devices");
    let _mappings = engine.create_child_container(root, "Mappings", "mappings");

    let osc = engine.create_child_container(outputs, "OscOutput", "osc_output");

    let intensity = engine
        .find_descendant_by_decl(osc, "intensity")
        .expect("missing declared param: intensity");
    let host = engine.find_descendant_by_decl(osc, "host").expect("missing declared param: host");

    let host_uuid =
        engine.nodes.get(&host).map(|node| node.meta.uuid).expect("missing host node meta uuid");

    //Creating dynamically
    let _target = engine.create_child_parameter(
        osc,
        "target",
        Value::Reference(schema::ReferenceValue {
            uuid: host_uuid,
            cached_id: Some(host),
        }),
    );

    engine.enqueue_edit(
        Edit::SetParam {
            node: intensity,
            value: Value::Float(0.8),
        },
        Propagation::EndOfTick,
        EditOrigin::UI,
    );

    engine.tick();

    engine
}

fn main() {
    let config = RuntimeConfig::from_workspace_default();
    golden_app::launch(build_demo_engine(), config);
}
