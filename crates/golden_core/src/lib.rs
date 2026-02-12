pub mod data;
pub mod edits;
pub mod engine;
pub mod events;
pub mod graph;
pub mod history;
pub mod meta;
pub mod persistence;
pub mod schema;
pub mod values;

pub use data::{
    AllowedTypes, ChildListHandle, ContainerData, ContainerLimits, FolderHandle, FolderPolicy,
    ParameterData, ParameterValue, PotentialSlotHandle,
};
pub use engine::{Engine, EnginePhase, ProcessCtx};
pub use events::{Event, EventKind, EventTime};
pub use graph::node::{
    ManagerData, ManagerNodeRegistration, Node, NodeBehaviour, NodeBehaviourFactory, NodeBinding,
    NodeContinuous, NodeData, NodeExecution, NodeLifecycle, NodeReactive,
};
pub use schema::{
    ContainerDecl, DeclaredChild, FolderDecl, GoldenNodeDecl, InboxBehavior, NodeSchema, ParamDecl,
    PotentialSlot, SchemaRegistry,
};
pub use values::{
    ChangePolicy, ColorRgba, ReferenceValue, SavePolicy, Trigger, UpdatePolicy, Value,
    ValueConstraints, Vec2, Vec3,
};

#[macro_export]
macro_rules! callbacks {
    (
        impl NodeReactive for $ty:ty {
            $($body:item)*
        }
    ) => {
        impl $crate::graph::node::NodeBehaviour for $ty {
            fn process(&mut self, ctx: &mut $crate::engine::ProcessCtx) {
                <$ty as $crate::graph::node::NodeReactive>::process(self, ctx);
            }
        }

        impl $crate::graph::node::NodeReactive for $ty {
            $($body)*
        }
    };
}

#[macro_export]
macro_rules! trigger {
    ($ctx:expr, $param:expr) => {{
        $ctx.set_param($param, golden_schema::Value::Trigger);
    }};
}
