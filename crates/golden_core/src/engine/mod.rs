pub mod process_ctx;
pub mod scheduling;

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use golden_schema::{
    DeclId, Event, EventKind, EventTime, NodeId, NodeMeta, NodeTypeId, NodeUuid, ShortName, Value,
};
use slotmap::{Key, KeyData, SlotMap, new_key_type};
use uuid::Uuid;

use crate::edits::{Edit, EditOrigin, EditQueue, EditRequest, Propagation};
use crate::events::inbox::Inbox;
use crate::events::routing::subscriptions::{EventFilter, ListenerSpec};
use crate::graph::node::{ManagerData, Node, NodeBehaviour, NodeBinding, NodeData, NodeExecution};
use crate::meta::apply_patch;
use crate::schema::{NodeSchema, SchemaRegistry};

pub use process_ctx::{EnginePhase, ProcessCtx};

new_key_type! {
    struct NodeKey;
}

#[derive(Default)]
pub struct NodeStore {
    inner: SlotMap<NodeKey, Node>,
}

impl NodeStore {
    pub fn new() -> Self {
        Self {
            inner: SlotMap::with_key(),
        }
    }

    pub fn insert(&mut self, mut node: Node) -> NodeId {
        node.id = NodeId(0);
        let key = self.inner.insert(node);
        let id = Self::id_from_key(key);
        if let Some(inserted) = self.inner.get_mut(key) {
            inserted.id = id;
        }
        id
    }

    pub fn get(&self, id: &NodeId) -> Option<&Node> {
        self.inner.get(Self::key_from_id(*id))
    }

    pub fn get_mut(&mut self, id: &NodeId) -> Option<&mut Node> {
        self.inner.get_mut(Self::key_from_id(*id))
    }

    pub fn values(&self) -> impl Iterator<Item = &Node> {
        self.inner.values()
    }

    pub fn keys(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.inner.keys().map(Self::id_from_key)
    }

    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &Node)> {
        self.inner.iter().map(|(key, node)| (Self::id_from_key(key), node))
    }

    fn id_from_key(key: NodeKey) -> NodeId {
        NodeId(key.data().as_ffi())
    }

    fn key_from_id(id: NodeId) -> NodeKey {
        NodeKey::from(KeyData::from_ffi(id.0))
    }
}

pub struct Engine {
    pub time: EventTime,
    pub nodes: NodeStore,
    pub inboxes: HashMap<NodeId, Inbox>,
    pub subscriptions: Vec<ListenerSpec>,
    pub pending_edits: Vec<EditRequest>,
    pub schema: SchemaRegistry,
    pub event_log: VecDeque<Event>,
    param_values: Arc<HashMap<NodeId, Value>>,
    meta_values: Arc<HashMap<NodeId, NodeMeta>>,
    root: NodeId,
}

impl Engine {
    pub fn new() -> Self {
        let mut engine = Self {
            time: EventTime {
                tick: 0,
                micro: 0,
                seq: 0,
            },
            nodes: NodeStore::new(),
            inboxes: HashMap::new(),
            subscriptions: Vec::new(),
            pending_edits: Vec::new(),
            schema: SchemaRegistry::new(),
            event_log: VecDeque::new(),
            param_values: Arc::new(HashMap::new()),
            meta_values: Arc::new(HashMap::new()),
            root: NodeId(0),
        };

        let mut root_meta = engine.create_meta("root");
        root_meta.decl_id = DeclId("root".to_string());
        root_meta.short_name = ShortName("root".to_string());
        root_meta.label = "root".to_string();

        let root = engine.create_node(
            NodeTypeId("Root".to_string()),
            NodeExecution::Passive,
            NodeData::Container(Self::default_container_data()),
            root_meta,
            None,
        );
        engine.root = root;

        engine
    }

    pub fn root_id(&self) -> NodeId {
        self.root
    }

    pub fn find_descendant_by_decl(&self, parent: NodeId, decl_id: &str) -> Option<NodeId> {
        let mut current = self.nodes.get(&parent).and_then(|node| node.first_child);
        while let Some(node_id) = current {
            if let Some(node) = self.nodes.get(&node_id) {
                if node.meta.decl_id.0 == decl_id {
                    return Some(node_id);
                }
                if let Some(found) = self.find_descendant_by_decl(node_id, decl_id) {
                    return Some(found);
                }
                current = node.next_sibling;
            } else {
                break;
            }
        }
        None
    }

    pub fn register_schema(&mut self, node_type: NodeTypeId, schema: NodeSchema) {
        self.schema.register(node_type, schema);
    }

    pub fn create_meta(&self, label: &str) -> NodeMeta {
        let short = if label.is_empty() {
            "node"
        } else {
            label
        };
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
        let param_value = match &data {
            NodeData::Parameter(param) => Some(param.value.clone()),
            _ => None,
        };
        let meta_value = meta.clone();

        let node = Node {
            id: NodeId(0),
            node_type: node_type.clone(),
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
        let node_id = self.nodes.insert(node);
        if let Some(value) = param_value {
            Arc::make_mut(&mut self.param_values).insert(node_id, value);
        }
        Arc::make_mut(&mut self.meta_values).insert(node_id, meta_value);
        self.inboxes.insert(node_id, Inbox::new());
        self.emit_event(EventKind::NodeCreated {
            node: node_id,
        });
        self.instantiate_declared_children(node_id, &node_type);
        node_id
    }

    pub fn create_container_node(&mut self, node_type: &str, label: &str) -> NodeId {
        self.create_node(
            NodeTypeId(node_type.to_string()),
            NodeExecution::Reactive,
            NodeData::Container(Self::default_container_data()),
            self.create_meta(label),
            None,
        )
    }

    pub fn create_manager_node(
        &mut self,
        node_type: &str,
        label: &str,
        manager_data: ManagerData,
    ) -> NodeId {
        self.create_node(
            NodeTypeId(node_type.to_string()),
            NodeExecution::Reactive,
            NodeData::Manager(manager_data),
            self.create_meta(label),
            None,
        )
    }

    pub fn create_child_manager(
        &mut self,
        parent: NodeId,
        node_type: &str,
        label: &str,
        manager_data: ManagerData,
    ) -> NodeId {
        let child = self.create_manager_node(node_type, label, manager_data);
        self.add_child(parent, child);
        child
    }

    pub fn create_child_container(
        &mut self,
        parent: NodeId,
        node_type: &str,
        label: &str,
    ) -> NodeId {
        let child = self.create_container_node(node_type, label);
        self.add_child(parent, child);
        child
    }

    pub fn create_parameter_node(&mut self, label: &str, value: Value) -> NodeId {
        self.create_node(
            NodeTypeId("Parameter".to_string()),
            NodeExecution::Passive,
            NodeData::Parameter(crate::data::ParameterData {
                value: value.clone(),
                default: Some(value),
                read_only: false,
                update: golden_schema::UpdatePolicy::Immediate,
                save: golden_schema::SavePolicy::Delta,
                change: golden_schema::ChangePolicy::ValueChange,
                constraints: golden_schema::ValueConstraints::None,
            }),
            self.create_meta(label),
            None,
        )
    }

    pub fn create_child_parameter(&mut self, parent: NodeId, label: &str, value: Value) -> NodeId {
        let child = self.create_parameter_node(label, value);
        self.add_child(parent, child);
        child
    }

    pub fn create_behaviour_node(
        &mut self,
        node_type: &str,
        label: &str,
        execution: NodeExecution,
        behaviour: Box<dyn NodeBehaviour>,
    ) -> NodeId {
        self.create_node(
            NodeTypeId(node_type.to_string()),
            execution,
            NodeData::None,
            self.create_meta(label),
            Some(behaviour),
        )
    }

    pub fn create_child_behaviour_node(
        &mut self,
        parent: NodeId,
        node_type: &str,
        label: &str,
        execution: NodeExecution,
        behaviour: Box<dyn NodeBehaviour>,
    ) -> NodeId {
        let child = self.create_behaviour_node(node_type, label, execution, behaviour);
        self.add_child(parent, child);
        child
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
            self.emit_event(EventKind::ChildAdded {
                parent,
                child,
            });
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
        self.emit_event(EventKind::ChildAdded {
            parent,
            child,
        });
    }

    fn instantiate_declared_children(&mut self, parent: NodeId, parent_type: &NodeTypeId) {
        let Some(schema) = self.schema.schema_for(parent_type).cloned() else {
            return;
        };

        self.instantiate_declared_children_from_schema(parent, &schema);
    }

    fn instantiate_declared_children_from_schema(&mut self, parent: NodeId, schema: &NodeSchema) {
        let mut folder_nodes = HashMap::<String, NodeId>::new();
        for folder in &schema.folders {
            let folder_id =
                self.ensure_folder_path(parent, &folder.decl_id.0, folder.label.clone());
            folder_nodes.insert(folder.decl_id.0.clone(), folder_id);
        }

        for param in &schema.params {
            let mut meta = self.create_meta(&param.decl_id.0);
            meta.decl_id = param.decl_id.clone();
            meta.short_name = ShortName(param.decl_id.0.clone());
            meta.label = param.decl_id.0.clone();
            meta.semantics = param.semantics.clone();
            meta.presentation = param.presentation.clone();

            let node = self.create_node(
                NodeTypeId("Parameter".to_string()),
                NodeExecution::Passive,
                NodeData::Parameter(crate::data::ParameterData {
                    value: param.default.clone(),
                    default: Some(param.default.clone()),
                    read_only: param.read_only,
                    update: param.update,
                    save: param.save,
                    change: param.change,
                    constraints: param.constraints.clone(),
                }),
                meta,
                None,
            );

            let target_parent = param
                .folder
                .as_ref()
                .and_then(|decl| folder_nodes.get(&decl.0).copied())
                .unwrap_or(parent);

            self.add_child(target_parent, node);
        }

        for child in &schema.declared_children {
            if child.node_type.0 == "Parameter" || child.node_type.0 == "Folder" {
                continue;
            }

            let mut meta = self
                .create_meta(child.default_label.as_deref().unwrap_or(child.decl_id.0.as_str()));
            meta.decl_id = child.decl_id.clone();
            meta.enabled = child.default_enabled;
            if let Some(label) = &child.default_label {
                meta.label = label.clone();
            }

            let data = self.default_node_data_for_type(&child.node_type);

            let child_node =
                self.create_node(child.node_type.clone(), NodeExecution::Passive, data, meta, None);
            self.add_child(parent, child_node);
        }
    }

    fn ensure_folder_path(&mut self, parent: NodeId, path: &str, label: Option<String>) -> NodeId {
        let mut current_parent = parent;
        let mut prefix = String::new();
        for segment in path.split('.') {
            if !prefix.is_empty() {
                prefix.push('.');
            }
            prefix.push_str(segment);

            if let Some(existing) = self.find_direct_child_by_decl(current_parent, &prefix) {
                current_parent = existing;
                continue;
            }

            let mut meta = self.create_meta(segment);
            meta.decl_id = DeclId(prefix.clone());
            meta.short_name = ShortName(segment.to_string());
            if prefix == path {
                if let Some(folder_label) = &label {
                    meta.label = folder_label.clone();
                } else {
                    meta.label = segment.to_string();
                }
            } else {
                meta.label = segment.to_string();
            }

            let folder_node = self.create_node(
                NodeTypeId("Folder".to_string()),
                NodeExecution::Passive,
                NodeData::Container(Self::default_container_data()),
                meta,
                None,
            );
            self.add_child(current_parent, folder_node);
            current_parent = folder_node;
        }
        current_parent
    }

    fn find_direct_child_by_decl(&self, parent: NodeId, decl_id: &str) -> Option<NodeId> {
        let mut current = self.nodes.get(&parent).and_then(|node| node.first_child);
        while let Some(node_id) = current {
            let Some(node) = self.nodes.get(&node_id) else {
                break;
            };
            if node.meta.decl_id.0 == decl_id {
                return Some(node_id);
            }
            current = node.next_sibling;
        }
        None
    }

    fn default_container_data() -> crate::data::ContainerData {
        crate::data::ContainerData {
            allowed_types: crate::data::AllowedTypes::Any,
            folders: crate::data::FolderPolicy::Allowed,
            limits: crate::data::ContainerLimits {
                max_children: None,
            },
        }
    }

    fn default_node_data_for_type(&self, node_type: &NodeTypeId) -> NodeData {
        self.schema
            .schema_for(node_type)
            .map(Self::default_node_data_from_schema)
            .unwrap_or(NodeData::None)
    }

    fn default_node_data_from_schema(schema: &NodeSchema) -> NodeData {
        schema
            .container
            .as_ref()
            .map(|container| {
                NodeData::Container(crate::data::ContainerData {
                    allowed_types: container.allowed_types.clone(),
                    folders: container.folders.clone(),
                    limits: crate::data::ContainerLimits {
                        max_children: None,
                    },
                })
            })
            .unwrap_or(NodeData::None)
    }

    fn instantiate_child_from_manager(
        &mut self,
        manager: NodeId,
        node_type: NodeTypeId,
        label: String,
        execution: NodeExecution,
    ) -> Option<NodeId> {
        let manager_schema = {
            let manager_node = self.nodes.get(&manager)?;
            let NodeData::Manager(manager_data) = &manager_node.data else {
                return None;
            };
            let registration = manager_data.registration_for(&node_type)?;
            registration.schema.clone()
        };

        let data = Self::default_node_data_from_schema(&manager_schema);
        let child =
            self.create_node(node_type.clone(), execution, data, self.create_meta(&label), None);
        self.add_child(manager, child);
        self.instantiate_declared_children_from_schema(child, &manager_schema);

        let binding = self.build_node_binding_from_schema(child, &manager_schema);
        let manager_behaviour = {
            let manager_node = self.nodes.get(&manager)?;
            let NodeData::Manager(manager_data) = &manager_node.data else {
                return None;
            };
            manager_data.create_behaviour(&node_type, binding)?
        };

        if let Some(child_node) = self.nodes.get_mut(&child) {
            child_node.behaviour = Some(manager_behaviour);
        }

        Some(child)
    }

    fn build_node_binding_from_schema(&self, node: NodeId, schema: &NodeSchema) -> NodeBinding {
        let mut by_decl = HashMap::new();

        for folder in &schema.folders {
            if let Some(id) = self.find_descendant_by_decl(node, &folder.decl_id.0) {
                by_decl.insert(folder.decl_id.0.clone(), id);
            }
        }

        for param in &schema.params {
            if let Some(id) = self.find_descendant_by_decl(node, &param.decl_id.0) {
                by_decl.insert(param.decl_id.0.clone(), id);
            }
        }

        for child in &schema.declared_children {
            if let Some(id) = self.find_descendant_by_decl(node, &child.decl_id.0) {
                by_decl.insert(child.decl_id.0.clone(), id);
            }
        }

        NodeBinding::new(node, by_decl)
    }

    pub fn subscribe(&mut self, spec: ListenerSpec) {
        self.subscriptions.push(spec);
    }

    pub fn on_param_change(&mut self, subscriber: NodeId, param: NodeId) {
        self.subscribe(ListenerSpec::on_param_change(subscriber, param));
    }

    pub fn on_child_added(&mut self, subscriber: NodeId, parent: NodeId) {
        self.subscribe(ListenerSpec::on_child_added(subscriber, parent));
    }

    pub fn on_child_removed(&mut self, subscriber: NodeId, parent: NodeId) {
        self.subscribe(ListenerSpec::on_child_removed(subscriber, parent));
    }

    pub fn on_child_replaced(&mut self, subscriber: NodeId, parent: NodeId) {
        self.subscribe(ListenerSpec::on_child_replaced(subscriber, parent));
    }

    pub fn on_child_moved(&mut self, subscriber: NodeId, parent: NodeId) {
        self.subscribe(ListenerSpec::on_child_moved(subscriber, parent));
    }

    pub fn on_child_reordered(&mut self, subscriber: NodeId, parent: NodeId) {
        self.subscribe(ListenerSpec::on_child_reordered(subscriber, parent));
    }

    pub fn on_node_created(&mut self, subscriber: NodeId) {
        self.subscribe(ListenerSpec::on_node_created(subscriber));
    }

    pub fn on_node_deleted(&mut self, subscriber: NodeId) {
        self.subscribe(ListenerSpec::on_node_deleted(subscriber));
    }

    pub fn on_meta_changed(&mut self, subscriber: NodeId, node: NodeId) {
        self.subscribe(ListenerSpec::on_meta_changed(subscriber, node));
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
                param_values: Arc::clone(&self.param_values),
                meta_values: Arc::clone(&self.meta_values),
            };

            if let Some(node) = self.nodes.get_mut(&node_id) {
                if let Some(behaviour) = node.behaviour.as_mut() {
                    behaviour.process(&mut ctx);
                }
            }

            let edits = ctx.edits.drain();
            drop(ctx);
            self.apply_edit_requests(edits);
        }
    }

    fn run_update_pass(&mut self) {
        let node_ids: Vec<NodeId> = self.nodes.keys().collect();
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
                param_values: Arc::clone(&self.param_values),
                meta_values: Arc::clone(&self.meta_values),
            };

            if let Some(node) = self.nodes.get_mut(&node_id) {
                if let Some(behaviour) = node.behaviour.as_mut() {
                    behaviour.update(&mut ctx);
                }
            }

            let edits = ctx.edits.drain();
            drop(ctx);
            self.apply_edit_requests(edits);
        }
    }

    fn has_pending_inboxes(&self) -> bool {
        self.inboxes.values().any(|inbox| !inbox.events.is_empty())
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
                Edit::SetParam {
                    node,
                    value,
                } => {
                    if self.set_param(node, value.clone()) {
                        self.emit_event(EventKind::ParamChanged {
                            param: node,
                            value,
                        });
                    }
                }
                Edit::PatchMeta {
                    node,
                    patch,
                } => {
                    if let Some(node_ref) = self.nodes.get_mut(&node) {
                        apply_patch(&mut node_ref.meta, &patch);
                        Arc::make_mut(&mut self.meta_values).insert(node, node_ref.meta.clone());
                        self.emit_event(EventKind::MetaChanged {
                            node,
                            patch,
                        });
                    }
                }
                Edit::InstantiateChildFromManager {
                    manager,
                    node_type,
                    label,
                    execution,
                } => {
                    let _ =
                        self.instantiate_child_from_manager(manager, node_type, label, execution);
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
            Arc::make_mut(&mut self.param_values).insert(node, param.value.clone());
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
            self.inboxes.entry(target).or_insert_with(Inbox::new).push(event.clone());
        }
    }

    fn deliver_to_subscribers(&mut self, event: &Event) {
        for spec in &self.subscriptions {
            if matches_filter(&spec.filter, event, &self.nodes) {
                let _ = spec.delivery;
                self.inboxes.entry(spec.subscriber).or_insert_with(Inbox::new).push(event.clone());
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
        self.inboxes.entry(parent).or_insert_with(Inbox::new).push(event.clone());
    }

    fn flush_immediate(&mut self) {
        self.time.micro = self.time.micro.saturating_add(1);
        self.time.seq = 0;
        self.process_pending(EnginePhase::FlushImmediate);
    }

    pub fn events_since(&self, since: EventTime) -> Vec<Event> {
        self.event_log.iter().filter(|event| event.time > since).cloned().collect()
    }
}

fn event_targets(kind: &EventKind) -> Vec<NodeId> {
    match kind {
        EventKind::ParamChanged {
            param,
            ..
        } => vec![*param],
        EventKind::ChildAdded {
            parent,
            child,
        } => vec![*parent, *child],
        EventKind::ChildRemoved {
            parent,
            child,
        } => vec![*parent, *child],
        EventKind::ChildReplaced {
            parent,
            old,
            new,
        } => vec![*parent, *old, *new],
        EventKind::ChildMoved {
            child,
            old_parent,
            new_parent,
        } => vec![*child, *old_parent, *new_parent],
        EventKind::ChildReordered {
            parent,
            child,
        } => vec![*parent, *child],
        EventKind::NodeCreated {
            node,
        } => vec![*node],
        EventKind::NodeDeleted {
            node,
        } => vec![*node],
        EventKind::MetaChanged {
            node,
            ..
        } => vec![*node],
    }
}

fn event_bubble_source(kind: &EventKind) -> Option<NodeId> {
    match kind {
        EventKind::ParamChanged {
            param,
            ..
        } => Some(*param),
        EventKind::MetaChanged {
            node,
            ..
        } => Some(*node),
        EventKind::ChildAdded {
            child,
            ..
        } => Some(*child),
        EventKind::ChildRemoved {
            child,
            ..
        } => Some(*child),
        EventKind::ChildReplaced {
            new,
            ..
        } => Some(*new),
        EventKind::ChildMoved {
            child,
            ..
        } => Some(*child),
        EventKind::ChildReordered {
            child,
            ..
        } => Some(*child),
        EventKind::NodeCreated {
            node,
        } => Some(*node),
        EventKind::NodeDeleted {
            node,
        } => Some(*node),
    }
}

fn matches_filter(filter: &EventFilter, event: &Event, nodes: &NodeStore) -> bool {
    match filter {
        EventFilter::Node(node_id) => event_targets(&event.kind).contains(node_id),
        EventFilter::Param(node_id) => {
            matches!(&event.kind, EventKind::ParamChanged { param, .. } if param == node_id)
        }
        EventFilter::Subtree {
            root,
        } => event_targets(&event.kind)
            .into_iter()
            .any(|target| is_node_in_subtree(nodes, *root, target)),
        EventFilter::Kind(kind) => {
            std::mem::discriminant(kind) == std::mem::discriminant(&event.kind)
        }
        EventFilter::ParamChanged {
            param,
        } => {
            matches!(&event.kind, EventKind::ParamChanged { param: actual, .. } if param.is_none_or(|expected| expected == *actual))
        }
        EventFilter::ChildAdded {
            parent,
            child,
        } => {
            matches!(&event.kind, EventKind::ChildAdded { parent: actual_parent, child: actual_child }
                if parent.is_none_or(|expected| expected == *actual_parent)
                    && child.is_none_or(|expected| expected == *actual_child))
        }
        EventFilter::ChildRemoved {
            parent,
            child,
        } => {
            matches!(&event.kind, EventKind::ChildRemoved { parent: actual_parent, child: actual_child }
                if parent.is_none_or(|expected| expected == *actual_parent)
                    && child.is_none_or(|expected| expected == *actual_child))
        }
        EventFilter::ChildReplaced {
            parent,
            old,
            new,
        } => {
            matches!(&event.kind, EventKind::ChildReplaced { parent: actual_parent, old: actual_old, new: actual_new }
                if parent.is_none_or(|expected| expected == *actual_parent)
                    && old.is_none_or(|expected| expected == *actual_old)
                    && new.is_none_or(|expected| expected == *actual_new))
        }
        EventFilter::ChildMoved {
            child,
            old_parent,
            new_parent,
        } => {
            matches!(&event.kind, EventKind::ChildMoved { child: actual_child, old_parent: actual_old_parent, new_parent: actual_new_parent }
                if child.is_none_or(|expected| expected == *actual_child)
                    && old_parent.is_none_or(|expected| expected == *actual_old_parent)
                    && new_parent.is_none_or(|expected| expected == *actual_new_parent))
        }
        EventFilter::ChildReordered {
            parent,
            child,
        } => {
            matches!(&event.kind, EventKind::ChildReordered { parent: actual_parent, child: actual_child }
                if parent.is_none_or(|expected| expected == *actual_parent)
                    && child.is_none_or(|expected| expected == *actual_child))
        }
        EventFilter::NodeCreated {
            node,
        } => {
            matches!(&event.kind, EventKind::NodeCreated { node: actual } if node.is_none_or(|expected| expected == *actual))
        }
        EventFilter::NodeDeleted {
            node,
        } => {
            matches!(&event.kind, EventKind::NodeDeleted { node: actual } if node.is_none_or(|expected| expected == *actual))
        }
        EventFilter::MetaChanged {
            node,
        } => {
            matches!(&event.kind, EventKind::MetaChanged { node: actual, .. } if node.is_none_or(|expected| expected == *actual))
        }
        EventFilter::Any(filters) => filters.iter().any(|f| matches_filter(f, event, nodes)),
        EventFilter::All(filters) => filters.iter().all(|f| matches_filter(f, event, nodes)),
    }
}

fn is_node_in_subtree(nodes: &NodeStore, root: NodeId, mut node: NodeId) -> bool {
    if root == node {
        return true;
    }

    loop {
        let Some(current) = nodes.get(&node) else {
            return false;
        };
        let Some(parent) = current.parent else {
            return false;
        };
        if parent == root {
            return true;
        }
        node = parent;
    }
}
