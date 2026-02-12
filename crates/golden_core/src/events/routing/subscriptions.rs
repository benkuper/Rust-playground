use golden_schema::{EventKind, NodeId};

pub struct ListenerSpec {
    pub subscriber: NodeId,
    pub filter: EventFilter,
    pub delivery: DeliveryMode,
}

pub enum EventFilter {
    Node(NodeId),
    Param(NodeId),
    Subtree { root: NodeId },
    Kind(EventKind),
    Any(Vec<EventFilter>),
    All(Vec<EventFilter>),
}

pub enum DeliveryMode {
    Raw,
    Summarized,
}
