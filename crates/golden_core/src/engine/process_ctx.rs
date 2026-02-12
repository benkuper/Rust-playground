use std::sync::Arc;

use crate::edits::{Edit, EditOrigin, EditQueue, Propagation};
use crate::graph::node::NodeExecution;
use golden_schema::{Event, EventTime, NodeId, NodeMeta, NodeMetaPatch, Value};
use golden_schema::NodeTypeId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnginePhase {
    EngineTick,
    EndOfTickStabilization,
    FlushImmediate,
}

pub struct ProcessCtx {
    pub phase: EnginePhase,
    pub edits: EditQueue,
    pub inbox: Vec<Event>,
    pub time: EventTime,
    pub param_values: Arc<std::collections::HashMap<NodeId, Value>>,
    pub meta_values: Arc<std::collections::HashMap<NodeId, NodeMeta>>,
}

impl ProcessCtx {
    pub fn set_param(&mut self, node: NodeId, value: Value) {
        self.set_param_with(node, value, Propagation::EndOfTick);
    }

    pub fn set_param_with(&mut self, node: NodeId, value: Value, propagation: Propagation) {
        self.edits.push(
            Edit::SetParam {
                node,
                value,
            },
            propagation,
            EditOrigin::Internal,
        );
    }

    pub fn set_param_immediate(&mut self, node: NodeId, value: Value) {
        self.set_param_with(node, value, Propagation::Immediate);
    }

    pub fn set_param_next_tick(&mut self, node: NodeId, value: Value) {
        self.set_param_with(node, value, Propagation::NextTick);
    }

    pub fn patch_meta(&mut self, node: NodeId, patch: NodeMetaPatch) {
        self.edits.push(
            Edit::PatchMeta {
                node,
                patch,
            },
            Propagation::EndOfTick,
            EditOrigin::Internal,
        );
    }

    pub fn instantiate_child_from_manager(
        &mut self,
        manager: NodeId,
        node_type: NodeTypeId,
        label: impl Into<String>,
    ) {
        self.instantiate_child_from_manager_with(
            manager,
            node_type,
            label,
            NodeExecution::Passive,
            Propagation::EndOfTick,
        );
    }

    pub fn instantiate_child_from_manager_with(
        &mut self,
        manager: NodeId,
        node_type: NodeTypeId,
        label: impl Into<String>,
        execution: NodeExecution,
        propagation: Propagation,
    ) {
        self.edits.push(
            Edit::InstantiateChildFromManager {
                manager,
                node_type,
                label: label.into(),
                execution,
            },
            propagation,
            EditOrigin::Internal,
        );
    }

    pub fn read_param(&self, node: NodeId) -> Option<&Value> {
        self.param_values.get(&node)
    }

    pub fn read_meta(&self, node: NodeId) -> Option<&NodeMeta> {
        self.meta_values.get(&node)
    }
}
