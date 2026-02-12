use crate::edits::{Edit, EditOrigin, EditQueue, Propagation};
use golden_schema::{Event, EventTime, NodeId, NodeMetaPatch, Value};

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
    pub param_values: std::collections::HashMap<NodeId, Value>,
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

    pub fn read_param(&self, node: NodeId) -> Option<&Value> {
        self.param_values.get(&node)
    }
}
