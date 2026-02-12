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
    prog: f64,
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
    fn on_param_change(&mut self, ctx: &mut ProcessCtx, node_id: schema::NodeId, value: Value) {
        //check if the changed parameter is the panic trigger
        if node_id == self.panic.node_id {
            self.drive.set_immediate(ctx, 0.0);
            println!("Panic triggered! Drive reset to 0.0");
        }
    }
}

impl NodeContinuous for OscOutput {
    fn update(&mut self, ctx: &mut ProcessCtx) {
        if self.enabled.get(ctx).unwrap_or(true) {
            let intensity = self.intensity.get(ctx).unwrap_or(0.0);
            let anim_cos = self.prog.cos() * 0.5 + 0.5;
            self.prog += 0.01;
            self.drive.set(ctx, anim_cos);
            let value = self.value.get(ctx).unwrap_or(0.0);
            // Simple processing logic: output is intensity multiplied by drive and value
            let output = intensity * anim_cos * value;
            // println!("OscOutput processing: intensity={intensity}, drive={anim_cos}, value={value}, output={output}");
        }
    }
}

struct OscOutputBehaviour {
    inner: OscOutput,
}

impl NodeBehaviour for OscOutputBehaviour {
    fn process(&mut self, ctx: &mut ProcessCtx) {
        NodeReactive::process(&mut self.inner, ctx);
    }

    fn update(&mut self, ctx: &mut ProcessCtx) {
        NodeContinuous::update(&mut self.inner, ctx);
    }
}

#[derive(Default)]
struct OutputManagerBehaviour {
    manager_id: Option<schema::NodeId>,
    spawned: bool,
}

callbacks! {
    impl NodeReactive for OutputManagerBehaviour {
        fn on_node_created(&mut self, ctx: &mut ProcessCtx, node: schema::NodeId) {
            if self.manager_id.is_none() {
                self.manager_id = Some(node);
            }

            if self.manager_id == Some(node) && !self.spawned {
                ctx.instantiate_child_from_manager_with(
                    node,
                    schema::NodeTypeId("OscOutput".to_string()),
                    "osc_output_a",
                    NodeExecution::Continuous,
                    Propagation::EndOfTick,
                );
                ctx.instantiate_child_from_manager_with(
                    node,
                    schema::NodeTypeId("OscOutput".to_string()),
                    "osc_output_b",
                    NodeExecution::Continuous,
                    Propagation::EndOfTick,
                );
                self.spawned = true;
            }
        }
    }
}

fn build_demo_engine() -> Engine {
    let mut engine = Engine::new();

    let root = engine.root_id();
    let outputs = engine.create_child_container(root, "Outputs", "outputs");
    let _devices = engine.create_child_container(root, "Devices", "devices");
    let _mappings = engine.create_child_container(root, "Mappings", "mappings");

    let mut manager_data = ManagerData::new();
    manager_data.register_node_type(
        schema::NodeTypeId("OscOutput".to_string()),
        OscOutput::schema(),
        |binding| {
            let osc = OscOutput {
                id: binding.node_id,
                connection: binding.folder("connection").expect("missing folder 'connection'"),
                intensity: binding.param("intensity").expect("missing param 'intensity'"),
                enabled: binding.param("enabled").expect("missing param 'enabled'"),
                host: binding.param("host").expect("missing param 'host'"),
                port: binding.param("port").expect("missing param 'port'"),
                drive: binding.param("drive").expect("missing param 'drive'"),
                value: binding.param("value").expect("missing param 'value'"),
                panic: binding.param("panic").expect("missing param 'panic'"),
                prog: 0.0,
            };

            Box::new(OscOutputBehaviour {
                inner: osc,
            })
        },
    );

    let manager = engine.create_node(
        schema::NodeTypeId("OutputManager".to_string()),
        NodeExecution::Reactive,
        NodeData::Manager(manager_data),
        engine.create_meta("output_manager"),
        Some(Box::new(OutputManagerBehaviour::default())),
    );
    engine.add_child(outputs, manager);

    engine
}

fn main() {
    let config = RuntimeConfig::from_workspace_default();
    golden_app::launch(build_demo_engine(), config);
}
