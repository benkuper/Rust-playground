use crate::data::{ContainerData, CustomData, ParameterData};
use crate::engine::ProcessCtx;
use golden_schema::{NodeId, NodeMeta, NodeMetaPatch, NodeTypeId, Value};

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
    fn process(&mut self, ctx: &mut ProcessCtx) {
        self.dispatch_inbox(ctx);
    }

    fn dispatch_inbox(&mut self, ctx: &mut ProcessCtx) {
        let inbox_events = ctx.inbox.clone();
        for event in inbox_events {
            match event.kind {
                golden_schema::EventKind::ParamChanged {
                    param,
                    value,
                } => {
                    self.on_param_change(ctx, param, value);
                }
                golden_schema::EventKind::ChildAdded {
                    parent,
                    child,
                } => {
                    self.on_child_added(ctx, parent, child);
                }
                golden_schema::EventKind::ChildRemoved {
                    parent,
                    child,
                } => {
                    self.on_child_removed(ctx, parent, child);
                }
                golden_schema::EventKind::ChildReplaced {
                    parent,
                    old,
                    new,
                } => {
                    self.on_child_replaced(ctx, parent, old, new);
                }
                golden_schema::EventKind::ChildMoved {
                    child,
                    old_parent,
                    new_parent,
                } => {
                    self.on_child_moved(ctx, child, old_parent, new_parent);
                }
                golden_schema::EventKind::ChildReordered {
                    parent,
                    child,
                } => {
                    self.on_child_reordered(ctx, parent, child);
                }
                golden_schema::EventKind::NodeCreated {
                    node,
                } => {
                    self.on_node_created(ctx, node);
                }
                golden_schema::EventKind::NodeDeleted {
                    node,
                } => {
                    self.on_node_deleted(ctx, node);
                }
                golden_schema::EventKind::MetaChanged {
                    node,
                    patch,
                } => {
                    self.on_meta_changed(ctx, node, patch);
                }
            }
        }
    }

    fn on_param_change(&mut self, _ctx: &mut ProcessCtx, _param: NodeId, _value: Value) {}

    fn on_child_added(&mut self, _ctx: &mut ProcessCtx, _parent: NodeId, _child: NodeId) {}

    fn on_child_removed(&mut self, _ctx: &mut ProcessCtx, _parent: NodeId, _child: NodeId) {}

    fn on_child_replaced(
        &mut self,
        _ctx: &mut ProcessCtx,
        _parent: NodeId,
        _old: NodeId,
        _new: NodeId,
    ) {
    }

    fn on_child_moved(
        &mut self,
        _ctx: &mut ProcessCtx,
        _child: NodeId,
        _old_parent: NodeId,
        _new_parent: NodeId,
    ) {
    }

    fn on_child_reordered(&mut self, _ctx: &mut ProcessCtx, _parent: NodeId, _child: NodeId) {}

    fn on_node_created(&mut self, _ctx: &mut ProcessCtx, _node: NodeId) {}

    fn on_node_deleted(&mut self, _ctx: &mut ProcessCtx, _node: NodeId) {}

    fn on_meta_changed(&mut self, _ctx: &mut ProcessCtx, _node: NodeId, _patch: NodeMetaPatch) {}
}

pub trait NodeContinuous: NodeReactive {
    fn update(&mut self, ctx: &mut ProcessCtx);
}

pub trait NodeLifecycle {
    fn init(&mut self, _ctx: &mut ProcessCtx) {}
    fn destroy(&mut self, _ctx: &mut ProcessCtx) {}
}
