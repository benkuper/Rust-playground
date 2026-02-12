use crate::data::{ContainerData, CustomData, ParameterData};
use crate::engine::ProcessCtx;
use golden_schema::{NodeId, NodeMeta, NodeTypeId};

pub struct Node {
    pub id: NodeId,
    pub node_type: NodeTypeId,
    pub execution: NodeExecution,
    pub parent: Option<NodeId>,
    pub first_child: Option<NodeId>,
    pub last_child: Option<NodeId>,
    pub prev_sibling: Option<NodeId>,
    pub next_sibling: Option<NodeId>,
    pub meta: NodeMeta,
    pub data: NodeData,
    pub behaviour: Option<Box<dyn NodeBehaviour>>,
}

pub enum NodeData {
    None,
    Container(ContainerData),
    Parameter(ParameterData),
    Custom(CustomData),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeExecution {
    Passive,
    Reactive,
    Continuous,
}

pub trait NodeBehaviour: Send {
    fn process(&mut self, ctx: &mut ProcessCtx);
    fn update(&mut self, _ctx: &mut ProcessCtx) {}
}

pub trait NodeReactive {
    fn process(&mut self, ctx: &mut ProcessCtx);
}

pub trait NodeContinuous: NodeReactive {
    fn update(&mut self, ctx: &mut ProcessCtx);
}

pub trait NodeLifecycle {
    fn init(&mut self, _ctx: &mut ProcessCtx) {}
    fn destroy(&mut self, _ctx: &mut ProcessCtx) {}
}
