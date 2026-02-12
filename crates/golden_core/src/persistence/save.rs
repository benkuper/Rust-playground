use std::collections::{HashMap, HashSet};

use golden_schema::persistence::file_format::ProjectFile;
use golden_schema::persistence::{
    ContainerDataDto, DeltaNodeRecord, FullNodeRecord, NodeDataDto, NodeDataKind, NodeRecord,
};
use golden_schema::{DeclId, NodeId, NodeTypeId, NodeUuid, Value};
use uuid::Uuid;

use crate::data::{AllowedTypes, ContainerData};
use crate::engine::Engine;
use crate::graph::node::{Node, NodeData};
use crate::schema::NodeSchema;

enum SlotKind {
    Declared,
    Potential,
    Dynamic,
}

struct ExportNode {
    node_id: NodeId,
    record: NodeRecord,
    children: Vec<ExportNode>,
}

impl ExportNode {
    fn into_record(self) -> NodeRecord {
        let children = self
            .children
            .into_iter()
            .map(|child| child.into_record())
            .collect();
        match self.record {
            NodeRecord::Full(mut record) => {
                record.children = children;
                NodeRecord::Full(record)
            }
            NodeRecord::Delta(mut record) => {
                record.children = children;
                NodeRecord::Delta(record)
            }
        }
    }
}

struct ExportContext<'a> {
    engine: &'a Engine,
    referenced: HashSet<NodeUuid>,
    emitted: HashSet<NodeUuid>,
    uuid_map: HashMap<NodeUuid, NodeId>,
}

impl<'a> ExportContext<'a> {
    fn new(engine: &'a Engine) -> Self {
        let uuid_map = engine
            .nodes
            .values()
            .map(|node| (node.meta.uuid, node.id))
            .collect();
        Self {
            engine,
            referenced: HashSet::new(),
            emitted: HashSet::new(),
            uuid_map,
        }
    }
}

pub fn save_project(project: &ProjectFile) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(project)
}

pub fn export_project(engine: &Engine, root: NodeId, version: &str) -> ProjectFile {
    let mut ctx = ExportContext::new(engine);
    let mut root_node = export_root_node(&mut ctx, root);
    apply_reference_closure(&mut ctx, &mut root_node);
    ProjectFile {
        version: version.to_string(),
        root: root_node.into_record(),
    }
}

fn export_root_node(ctx: &mut ExportContext<'_>, node_id: NodeId) -> ExportNode {
    export_full_record(ctx, node_id, None).unwrap_or_else(|| missing_record(ctx.engine))
}

fn export_node(
    ctx: &mut ExportContext<'_>,
    node_id: NodeId,
    parent_type: Option<&NodeTypeId>,
) -> Option<ExportNode> {
    let node = ctx.engine.nodes.get(&node_id)?;
    let slot = slot_kind(ctx.engine, parent_type, node);
    match slot {
        SlotKind::Dynamic => export_full_record(ctx, node_id, None),
        SlotKind::Potential => export_full_record(ctx, node_id, Some(node.meta.decl_id.clone())),
        SlotKind::Declared => export_delta_record(ctx, node_id, parent_type),
    }
}

fn export_full_record(
    ctx: &mut ExportContext<'_>,
    node_id: NodeId,
    decl_id: Option<DeclId>,
) -> Option<ExportNode> {
    let node = ctx.engine.nodes.get(&node_id)?;
    let data = node_data_to_dto(&node.data);
    let children = collect_children(ctx, node);
    collect_references(ctx, node);

    ctx.emitted.insert(node.meta.uuid);

    Some(ExportNode {
        node_id,
        record: NodeRecord::Full(FullNodeRecord {
            decl_id,
            node_type: node.node_type.clone(),
            uuid: node.meta.uuid,
            meta: node.meta.clone(),
            data,
            children: Vec::new(),
        }),
        children,
    })
}

fn export_delta_record(
    ctx: &mut ExportContext<'_>,
    node_id: NodeId,
    parent_type: Option<&NodeTypeId>,
) -> Option<ExportNode> {
    let node = ctx.engine.nodes.get(&node_id)?;
    let value = match &node.data {
        NodeData::Parameter(param) => {
            if param.default.as_ref() != Some(&param.value) {
                Some(param.value.clone())
            } else {
                None
            }
        }
        _ => None,
    };

    let schema = parent_type.and_then(|parent| ctx.engine.schema.schema_for(parent));
    let declared = schema.and_then(|schema| find_declared_child(schema, node));
    let meta = meta_patch_from_node(node, declared);
    let children = collect_children(ctx, node);

    if value.is_none() && meta.is_none() && children.is_empty() {
        return None;
    }

    collect_references(ctx, node);
    ctx.emitted.insert(node.meta.uuid);

    Some(ExportNode {
        node_id,
        record: NodeRecord::Delta(DeltaNodeRecord {
            decl_id: node.meta.decl_id.clone(),
            uuid: Some(node.meta.uuid),
            meta,
            value,
            children: Vec::new(),
        }),
        children,
    })
}

fn node_data_to_dto(data: &NodeData) -> NodeDataDto {
    match data {
        NodeData::None => NodeDataDto {
            kind: NodeDataKind::None,
            container: None,
            parameter: None,
        },
        NodeData::Container(container) => NodeDataDto {
            kind: NodeDataKind::Container,
            container: Some(container_to_dto(container)),
            parameter: None,
        },
        NodeData::Parameter(param) => NodeDataDto {
            kind: NodeDataKind::Parameter,
            container: None,
            parameter: Some(param.clone()),
        },
        NodeData::Custom(_) => NodeDataDto {
            kind: NodeDataKind::Custom("Custom".to_string()),
            container: None,
            parameter: None,
        },
    }
}

fn container_to_dto(container: &ContainerData) -> ContainerDataDto {
    let allowed_types = match &container.allowed_types {
        AllowedTypes::Any => Vec::new(),
        AllowedTypes::Only(list) => list.clone(),
    };

    ContainerDataDto {
        allowed_types,
        folders_allowed: matches!(container.folders, crate::data::FolderPolicy::Allowed),
    }
}

fn collect_children(ctx: &mut ExportContext<'_>, node: &Node) -> Vec<ExportNode> {
    let mut children = Vec::new();
    let mut current = node.first_child;
    while let Some(child_id) = current {
        if let Some(child_record) = export_node(ctx, child_id, Some(&node.node_type)) {
            children.push(child_record);
        }
        current = ctx
            .engine
            .nodes
            .get(&child_id)
            .and_then(|child| child.next_sibling);
    }
    children
}

fn meta_patch_from_node(
    node: &Node,
    declared: Option<&crate::schema::DeclaredChild>,
) -> Option<golden_schema::NodeMetaPatch> {
    let mut patch = golden_schema::NodeMetaPatch::default();
    let default_label = declared
        .and_then(|child| child.default_label.clone())
        .unwrap_or_else(|| node.meta.decl_id.0.clone());
    let default_enabled = declared.map(|child| child.default_enabled).unwrap_or(true);

    if node.meta.enabled != default_enabled {
        patch.enabled = Some(node.meta.enabled);
    }
    if node.meta.label != default_label {
        patch.label = Some(node.meta.label.clone());
    }
    if node.meta.description.is_some() {
        patch.description = Some(node.meta.description.clone());
    }
    if !node.meta.tags.is_empty() {
        patch.tags = Some(node.meta.tags.clone());
    }
    if node.meta.semantics != Default::default() {
        patch.semantics = Some(node.meta.semantics.clone());
    }
    if node.meta.presentation != Default::default() {
        patch.presentation = Some(node.meta.presentation.clone());
    }

    if patch == golden_schema::NodeMetaPatch::default() {
        None
    } else {
        Some(patch)
    }
}

fn slot_kind(engine: &Engine, parent_type: Option<&NodeTypeId>, node: &Node) -> SlotKind {
    let Some(parent_type) = parent_type else {
        return SlotKind::Dynamic;
    };
    let Some(schema) = engine.schema.schema_for(parent_type) else {
        return SlotKind::Dynamic;
    };

    if is_potential_slot(schema, &node.meta.decl_id, &node.node_type) {
        return SlotKind::Potential;
    }

    if is_declared_child(schema, &node.meta.decl_id, &node.node_type) {
        return SlotKind::Declared;
    }

    SlotKind::Dynamic
}

fn is_declared_child(schema: &NodeSchema, decl_id: &DeclId, node_type: &NodeTypeId) -> bool {
    schema
        .declared_children
        .iter()
        .any(|child| &child.decl_id == decl_id && &child.node_type == node_type)
}

fn is_potential_slot(schema: &NodeSchema, decl_id: &DeclId, node_type: &NodeTypeId) -> bool {
    schema
        .potential_slots
        .iter()
        .any(|slot| &slot.decl_id == decl_id && slot.allowed_types.iter().any(|t| t == node_type))
}

fn find_declared_child<'a>(
    schema: &'a NodeSchema,
    node: &Node,
) -> Option<&'a crate::schema::DeclaredChild> {
    schema
        .declared_children
        .iter()
        .find(|child| child.decl_id == node.meta.decl_id && child.node_type == node.node_type)
}

fn collect_references(ctx: &mut ExportContext<'_>, node: &Node) {
    if let NodeData::Parameter(param) = &node.data {
        if let Value::Reference(reference) = &param.value {
            ctx.referenced.insert(reference.uuid);
        }
    }
}

fn apply_reference_closure(ctx: &mut ExportContext<'_>, root: &mut ExportNode) {
    let referenced: Vec<NodeUuid> = ctx.referenced.iter().copied().collect();
    for uuid in referenced {
        if ctx.emitted.contains(&uuid) {
            continue;
        }
        let Some(node_id) = ctx.uuid_map.get(&uuid).copied() else {
            continue;
        };
        let Some(node) = ctx.engine.nodes.get(&node_id) else {
            continue;
        };
        let Some(parent_id) = node.parent else {
            continue;
        };
        let Some(parent) = ctx.engine.nodes.get(&parent_id) else {
            continue;
        };
        let Some(schema) = ctx.engine.schema.schema_for(&parent.node_type) else {
            continue;
        };
        if !is_declared_child(schema, &node.meta.decl_id, &node.node_type) {
            continue;
        }

        let binding = NodeRecord::Delta(DeltaNodeRecord {
            decl_id: node.meta.decl_id.clone(),
            uuid: Some(uuid),
            meta: None,
            value: None,
            children: Vec::new(),
        });

        insert_binding_record(root, parent_id, binding);
    }
}

fn insert_binding_record(root: &mut ExportNode, parent_id: NodeId, binding: NodeRecord) -> bool {
    if root.node_id == parent_id {
        if !child_has_decl_id(&root.children, &binding) {
            root.children.push(ExportNode {
                node_id: NodeId(0),
                record: binding,
                children: Vec::new(),
            });
        }
        return true;
    }

    for child in &mut root.children {
        if insert_binding_record(child, parent_id, binding.clone()) {
            return true;
        }
    }

    false
}

fn child_has_decl_id(children: &[ExportNode], record: &NodeRecord) -> bool {
    let decl_id = match record {
        NodeRecord::Full(full) => full.decl_id.as_ref(),
        NodeRecord::Delta(delta) => Some(&delta.decl_id),
    };
    let Some(decl_id) = decl_id else {
        return false;
    };

    children.iter().any(|child| match &child.record {
        NodeRecord::Full(full) => full.decl_id.as_ref() == Some(decl_id),
        NodeRecord::Delta(delta) => &delta.decl_id == decl_id,
    })
}

fn missing_record(engine: &Engine) -> ExportNode {
    ExportNode {
        node_id: NodeId(0),
        record: NodeRecord::Full(FullNodeRecord {
            decl_id: None,
            node_type: NodeTypeId("Missing".to_string()),
            uuid: NodeUuid(Uuid::new_v4()),
            meta: engine.create_meta("missing"),
            data: NodeDataDto {
                kind: NodeDataKind::None,
                container: None,
                parameter: None,
            },
            children: Vec::new(),
        }),
        children: Vec::new(),
    }
}
