pub mod apply;
pub mod coalesce;

use crate::graph::node::NodeExecution;
use golden_schema::NodeId;
use golden_schema::NodeMetaPatch;
use golden_schema::NodeTypeId;
use golden_schema::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Propagation {
    Immediate,
    EndOfTick,
    NextTick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditOrigin {
    UI,
    Network,
    Script,
    Internal,
}

pub enum Edit {
    SetParam { node: NodeId, value: Value },
    PatchMeta { node: NodeId, patch: NodeMetaPatch },
    InstantiateChildFromManager {
        manager: NodeId,
        node_type: NodeTypeId,
        label: String,
        execution: NodeExecution,
    },
}

pub struct EditRequest {
    pub edit: Edit,
    pub propagation: Propagation,
    pub origin: EditOrigin,
}

pub struct EditQueue {
    pub pending: Vec<EditRequest>,
}

impl EditQueue {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
        }
    }

    pub fn push(&mut self, edit: Edit, propagation: Propagation, origin: EditOrigin) {
        self.pending.push(EditRequest {
            edit,
            propagation,
            origin,
        });
    }

    pub fn drain(&mut self) -> Vec<EditRequest> {
        std::mem::take(&mut self.pending)
    }
}
