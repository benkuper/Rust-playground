use golden_schema::{EventKind, NodeId};

#[derive(Clone, Debug, PartialEq)]
pub struct ListenerSpec {
    pub subscriber: NodeId,
    pub filter: EventFilter,
    pub delivery: DeliveryMode,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EventFilter {
    Node(NodeId),
    Param(NodeId),
    Subtree {
        root: NodeId,
    },
    Kind(EventKind),
    ParamChanged {
        param: Option<NodeId>,
    },
    ChildAdded {
        parent: Option<NodeId>,
        child: Option<NodeId>,
    },
    ChildRemoved {
        parent: Option<NodeId>,
        child: Option<NodeId>,
    },
    ChildReplaced {
        parent: Option<NodeId>,
        old: Option<NodeId>,
        new: Option<NodeId>,
    },
    ChildMoved {
        child: Option<NodeId>,
        old_parent: Option<NodeId>,
        new_parent: Option<NodeId>,
    },
    ChildReordered {
        parent: Option<NodeId>,
        child: Option<NodeId>,
    },
    NodeCreated {
        node: Option<NodeId>,
    },
    NodeDeleted {
        node: Option<NodeId>,
    },
    MetaChanged {
        node: Option<NodeId>,
    },
    Any(Vec<EventFilter>),
    All(Vec<EventFilter>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeliveryMode {
    Raw,
    Summarized,
}

impl ListenerSpec {
    pub fn raw(subscriber: NodeId, filter: EventFilter) -> Self {
        Self {
            subscriber,
            filter,
            delivery: DeliveryMode::Raw,
        }
    }

    pub fn summarized(subscriber: NodeId, filter: EventFilter) -> Self {
        Self {
            subscriber,
            filter,
            delivery: DeliveryMode::Summarized,
        }
    }

    pub fn on_param_change(subscriber: NodeId, param: NodeId) -> Self {
        Self::raw(
            subscriber,
            EventFilter::ParamChanged {
                param: Some(param),
            },
        )
    }

    pub fn on_child_added(subscriber: NodeId, parent: NodeId) -> Self {
        Self::raw(
            subscriber,
            EventFilter::ChildAdded {
                parent: Some(parent),
                child: None,
            },
        )
    }

    pub fn on_child_removed(subscriber: NodeId, parent: NodeId) -> Self {
        Self::raw(
            subscriber,
            EventFilter::ChildRemoved {
                parent: Some(parent),
                child: None,
            },
        )
    }

    pub fn on_child_replaced(subscriber: NodeId, parent: NodeId) -> Self {
        Self::raw(
            subscriber,
            EventFilter::ChildReplaced {
                parent: Some(parent),
                old: None,
                new: None,
            },
        )
    }

    pub fn on_child_moved(subscriber: NodeId, parent: NodeId) -> Self {
        Self::raw(
            subscriber,
            EventFilter::Any(vec![
                EventFilter::ChildMoved {
                    child: None,
                    old_parent: Some(parent),
                    new_parent: None,
                },
                EventFilter::ChildMoved {
                    child: None,
                    old_parent: None,
                    new_parent: Some(parent),
                },
            ]),
        )
    }

    pub fn on_child_reordered(subscriber: NodeId, parent: NodeId) -> Self {
        Self::raw(
            subscriber,
            EventFilter::ChildReordered {
                parent: Some(parent),
                child: None,
            },
        )
    }

    pub fn on_node_created(subscriber: NodeId) -> Self {
        Self::raw(
            subscriber,
            EventFilter::NodeCreated {
                node: None,
            },
        )
    }

    pub fn on_node_deleted(subscriber: NodeId) -> Self {
        Self::raw(
            subscriber,
            EventFilter::NodeDeleted {
                node: None,
            },
        )
    }

    pub fn on_meta_changed(subscriber: NodeId, node: NodeId) -> Self {
        Self::raw(
            subscriber,
            EventFilter::MetaChanged {
                node: Some(node),
            },
        )
    }
}
