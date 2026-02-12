pub mod process_ctx;
pub mod scheduling;

use std::collections::{HashMap, VecDeque};

use golden_schema::{
    DeclId, Event, EventKind, EventTime, NodeId, NodeMeta, NodeTypeId, NodeUuid, ShortName, Value,
};
use uuid::Uuid;

use crate::edits::{Edit, EditOrigin, EditQueue, EditRequest, Propagation};
use crate::events::inbox::Inbox;
use crate::events::routing::subscriptions::{EventFilter, ListenerSpec};
use crate::graph::node::{Node, NodeBehaviour, NodeData, NodeExecution};
use crate::meta::apply_patch;
use crate::schema::{NodeSchema, SchemaRegistry};

pub use process_ctx::{EnginePhase, ProcessCtx};

pub struct Engine {
    pub time: EventTime,
    pub nodes: HashMap<NodeId, Node>,
    pub inboxes: HashMap<NodeId, Inbox>,
    pub subscriptions: Vec<ListenerSpec>,
    pub pending_edits: Vec<EditRequest>,
    pub schema: SchemaRegistry,
    pub event_log: VecDeque<Event>,
    next_id: u64,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            time: EventTime {
                tick: 0,
                micro: 0,
                seq: 0,
            },
            nodes: HashMap::new(),
            inboxes: HashMap::new(),
            subscriptions: Vec::new(),
            pending_edits: Vec::new(),
            schema: SchemaRegistry::new(),
            event_log: VecDeque::new(),
            next_id: 1,
        }
    }

    pub fn register_schema(&mut self, node_type: NodeTypeId, schema: NodeSchema) {
        self.schema.register(node_type, schema);
    }

    pub fn create_meta(&self, label: &str) -> NodeMeta {
        let short = if label.is_empty() { "node" } else { label };
        NodeMeta {
            uuid: NodeUuid(Uuid::new_v4()),
            decl_id: DeclId(short.to_string()),
            short_name: ShortName(short.to_string()),
            enabled: true,
            label: short.to_string(),
            description: None,
            tags: Vec::new(),
            semantics: Default::default(),
            presentation: Default::default(),
        }
    }

    pub fn create_node(
        &mut self,
        node_type: NodeTypeId,
        execution: NodeExecution,
        data: NodeData,
        meta: NodeMeta,
        behaviour: Option<Box<dyn NodeBehaviour>>,
    ) -> NodeId {
        let node_id = NodeId(self.next_id);
        self.next_id += 1;
        let node = Node {
            id: node_id,
            node_type,
            execution,
            parent: None,
            first_child: None,
            last_child: None,
            prev_sibling: None,
            next_sibling: None,
            meta,
            data,
            behaviour,
        };
        self.nodes.insert(node_id, node);
        self.inboxes.insert(node_id, Inbox::new());
        self.emit_event(EventKind::NodeCreated { node: node_id });
        node_id
    }

    pub fn add_child(&mut self, parent: NodeId, child: NodeId) {
        let Some(last_child) = self.nodes.get(&parent).and_then(|node| node.last_child) else {
            if let Some(parent_node) = self.nodes.get_mut(&parent) {
                parent_node.first_child = Some(child);
                parent_node.last_child = Some(child);
            }
            if let Some(child_node) = self.nodes.get_mut(&child) {
                child_node.parent = Some(parent);
                child_node.prev_sibling = None;
                child_node.next_sibling = None;
            }
            self.emit_event(EventKind::ChildAdded { parent, child });
            return;
        };

        if let Some(parent_node) = self.nodes.get_mut(&parent) {
            parent_node.last_child = Some(child);
        }
        if let Some(last_node) = self.nodes.get_mut(&last_child) {
            last_node.next_sibling = Some(child);
        }
        if let Some(child_node) = self.nodes.get_mut(&child) {
            child_node.parent = Some(parent);
            child_node.prev_sibling = Some(last_child);
            child_node.next_sibling = None;
        }
        self.emit_event(EventKind::ChildAdded { parent, child });
    }

    pub fn subscribe(&mut self, spec: ListenerSpec) {
        self.subscriptions.push(spec);
    }

    pub fn enqueue_edit(&mut self, edit: Edit, propagation: Propagation, origin: EditOrigin) {
        self.pending_edits.push(EditRequest {
            edit,
            propagation,
            origin,
        });
    }

    pub fn tick(&mut self) {
        self.time.tick += 1;
        self.time.micro = 0;
        self.time.seq = 0;

        let external = std::mem::take(&mut self.pending_edits);
        self.apply_edit_requests(external);

        self.run_update_pass();

        self.process_pending(EnginePhase::EngineTick);

        const MAX_STABILIZATION_ROUNDS: u32 = 8;
        for round in 1..=MAX_STABILIZATION_ROUNDS {
            if !self.has_pending_inboxes() {
                break;
            }
            self.time.micro = round;
            self.time.seq = 0;
            self.process_pending(EnginePhase::EndOfTickStabilization);
        }
    }

    fn process_pending(&mut self, phase: EnginePhase) {
        let ready: Vec<NodeId> = self
            .inboxes
            .iter()
            .filter_map(|(id, inbox)| {
                if inbox.events.is_empty() {
                    None
                } else {
                    Some(*id)
                }
            })
            .collect();

        for node_id in ready {
            let inbox_events = self.take_inbox(node_id);
            let mut ctx = ProcessCtx {
                phase,
                edits: EditQueue::new(),
                inbox: inbox_events,
                time: self.time,
                param_values: self.snapshot_param_values(),
            };

            if let Some(node) = self.nodes.get_mut(&node_id) {
                if let Some(behaviour) = node.behaviour.as_mut() {
                    behaviour.process(&mut ctx);
                }
            }

            let edits = ctx.edits.drain();
            self.apply_edit_requests(edits);
        }
    }

    fn run_update_pass(&mut self) {
        let node_ids: Vec<NodeId> = self.nodes.keys().copied().collect();
        for node_id in node_ids {
            let should_update = self
                .nodes
                .get(&node_id)
                .is_some_and(|node| node.execution == NodeExecution::Continuous);
            if !should_update {
                continue;
            }

            let mut ctx = ProcessCtx {
                phase: EnginePhase::EngineTick,
                edits: EditQueue::new(),
                inbox: Vec::new(),
                time: self.time,
                param_values: self.snapshot_param_values(),
            };

            if let Some(node) = self.nodes.get_mut(&node_id) {
                if let Some(behaviour) = node.behaviour.as_mut() {
                    behaviour.update(&mut ctx);
                }
            }

            let edits = ctx.edits.drain();
            self.apply_edit_requests(edits);
        }
    }

    fn has_pending_inboxes(&self) -> bool {
        self.inboxes.values().any(|inbox| !inbox.events.is_empty())
    }

    fn snapshot_param_values(&self) -> HashMap<NodeId, Value> {
        let mut values = HashMap::new();
        for (id, node) in &self.nodes {
            if let NodeData::Parameter(param) = &node.data {
                values.insert(*id, param.value.clone());
            }
        }
        values
    }

    fn take_inbox(&mut self, node_id: NodeId) -> Vec<Event> {
        self.inboxes
            .get_mut(&node_id)
            .map(|inbox| std::mem::take(&mut inbox.events))
            .unwrap_or_default()
    }

    fn apply_edit_requests(&mut self, edits: Vec<EditRequest>) {
        for request in edits {
            let _ = request.origin;
            match request.edit {
                Edit::SetParam { node, value } => {
                    if self.set_param(node, value.clone()) {
                        self.emit_event(EventKind::ParamChanged { param: node, value });
                    }
                }
                Edit::PatchMeta { node, patch } => {
                    if let Some(node_ref) = self.nodes.get_mut(&node) {
                        apply_patch(&mut node_ref.meta, &patch);
                        self.emit_event(EventKind::MetaChanged { node, patch });
                    }
                }
            }

            if matches!(request.propagation, Propagation::Immediate) {
                self.flush_immediate();
            }
        }
    }

    fn set_param(&mut self, node: NodeId, value: Value) -> bool {
        let Some(node_ref) = self.nodes.get_mut(&node) else {
            return false;
        };
        let NodeData::Parameter(param) = &mut node_ref.data else {
            return false;
        };

        let changed = match param.change {
            golden_schema::ChangePolicy::Always => true,
            golden_schema::ChangePolicy::ValueChange => param.value != value,
        };

        if changed {
            param.value = value;
        }

        changed
    }

    fn emit_event(&mut self, kind: EventKind) {
        let event = Event {
            time: EventTime {
                tick: self.time.tick,
                micro: self.time.micro,
                seq: self.time.seq,
            },
            kind,
        };
        self.time.seq += 1;
        self.event_log.push_back(event.clone());
        const MAX_EVENT_LOG: usize = 4096;
        if self.event_log.len() > MAX_EVENT_LOG {
            self.event_log.pop_front();
        }
        self.deliver_event(event);
    }

    fn deliver_event(&mut self, event: Event) {
        self.deliver_to_own_targets(&event);
        self.deliver_to_subscribers(&event);
        self.deliver_bubbled(&event);
    }

    fn deliver_to_own_targets(&mut self, event: &Event) {
        for target in event_targets(&event.kind) {
            self.inboxes
                .entry(target)
                .or_insert_with(Inbox::new)
                .push(event.clone());
        }
    }

    fn deliver_to_subscribers(&mut self, event: &Event) {
        for spec in &self.subscriptions {
            if matches_filter(&spec.filter, event) {
                let _ = spec.delivery;
                self.inboxes
                    .entry(spec.subscriber)
                    .or_insert_with(Inbox::new)
                    .push(event.clone());
            }
        }
    }

    fn deliver_bubbled(&mut self, event: &Event) {
        let Some(node_for_parent) = event_bubble_source(&event.kind) else {
            return;
        };
        let Some(parent) = self.nodes.get(&node_for_parent).and_then(|n| n.parent) else {
            return;
        };
        self.inboxes
            .entry(parent)
            .or_insert_with(Inbox::new)
            .push(event.clone());
    }

    fn flush_immediate(&mut self) {
        self.time.micro = self.time.micro.saturating_add(1);
        self.time.seq = 0;
        self.process_pending(EnginePhase::FlushImmediate);
    }

    pub fn events_since(&self, since: EventTime) -> Vec<Event> {
        self.event_log
            .iter()
            .filter(|event| event.time > since)
            .cloned()
            .collect()
    }
}

fn event_targets(kind: &EventKind) -> Vec<NodeId> {
    match kind {
        EventKind::ParamChanged { param, .. } => vec![*param],
        EventKind::ChildAdded { parent, child } => vec![*parent, *child],
        EventKind::ChildRemoved { parent, child } => vec![*parent, *child],
        EventKind::ChildReplaced { parent, old, new } => vec![*parent, *old, *new],
        EventKind::ChildMoved {
            child,
            old_parent,
            new_parent,
        } => vec![*child, *old_parent, *new_parent],
        EventKind::ChildReordered { parent, child } => vec![*parent, *child],
        EventKind::NodeCreated { node } => vec![*node],
        EventKind::NodeDeleted { node } => vec![*node],
        EventKind::MetaChanged { node, .. } => vec![*node],
    }
}

fn event_bubble_source(kind: &EventKind) -> Option<NodeId> {
    match kind {
        EventKind::ParamChanged { param, .. } => Some(*param),
        EventKind::MetaChanged { node, .. } => Some(*node),
        EventKind::ChildAdded { child, .. } => Some(*child),
        EventKind::ChildRemoved { child, .. } => Some(*child),
        EventKind::ChildReplaced { new, .. } => Some(*new),
        EventKind::ChildMoved { child, .. } => Some(*child),
        EventKind::ChildReordered { child, .. } => Some(*child),
        EventKind::NodeCreated { node } => Some(*node),
        EventKind::NodeDeleted { node } => Some(*node),
    }
}

fn matches_filter(filter: &EventFilter, event: &Event) -> bool {
    match filter {
        EventFilter::Node(node_id) => event_targets(&event.kind).contains(node_id),
        EventFilter::Param(node_id) => {
            matches!(&event.kind, EventKind::ParamChanged { param, .. } if param == node_id)
        }
        EventFilter::Subtree { .. } => false,
        EventFilter::Kind(kind) => {
            std::mem::discriminant(kind) == std::mem::discriminant(&event.kind)
        }
        EventFilter::Any(filters) => filters.iter().any(|f| matches_filter(f, event)),
        EventFilter::All(filters) => filters.iter().all(|f| matches_filter(f, event)),
    }
}
