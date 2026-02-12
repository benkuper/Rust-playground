use serde::{Deserialize, Serialize};

use crate::ids::NodeId;
use crate::meta::NodeMetaPatch;
use crate::values::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EventTime {
    pub tick: u64,
    pub micro: u32,
    pub seq: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub time: EventTime,
    pub kind: EventKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EventKind {
    ParamChanged {
        param: NodeId,
        value: Value,
    },
    ChildAdded {
        parent: NodeId,
        child: NodeId,
    },
    ChildRemoved {
        parent: NodeId,
        child: NodeId,
    },
    ChildReplaced {
        parent: NodeId,
        old: NodeId,
        new: NodeId,
    },
    ChildMoved {
        child: NodeId,
        old_parent: NodeId,
        new_parent: NodeId,
    },
    ChildReordered {
        parent: NodeId,
        child: NodeId,
    },
    NodeCreated {
        node: NodeId,
    },
    NodeDeleted {
        node: NodeId,
    },
    MetaChanged {
        node: NodeId,
        patch: NodeMetaPatch,
    },
}
