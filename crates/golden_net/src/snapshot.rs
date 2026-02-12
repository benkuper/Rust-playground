use golden_core::Engine;
use golden_schema::events::EventTime;
use golden_schema::persistence::{ContainerDataDto, NodeDataDto, NodeDataKind};
use golden_schema::ui::dtos::{EnumDef, NodeDto, NodeTypeDef, ParamDto};
use golden_schema::ui::messages::Snapshot;
use golden_schema::{NodeId, NodeTypeId, Value};

pub fn build_snapshot(engine: &Engine) -> Snapshot {
    let nodes = engine
        .nodes
        .values()
        .map(|node| NodeDto {
            node_id: node.id,
            uuid: node.meta.uuid,
            node_type: node.node_type.clone(),
            decl_id: Some(node.meta.decl_id.clone()),
            meta: node.meta.clone(),
            data: node_data_dto(node),
            children: collect_children(engine, node.id),
        })
        .collect();

    let params = engine
        .nodes
        .values()
        .filter_map(|node| match &node.data {
            golden_core::NodeData::Parameter(param) => Some(ParamDto {
                param_node_id: node.id,
                value: param.value.clone(),
                read_only: param.read_only,
                update_policy: param.update,
                change_policy: param.change,
                constraints: param.constraints.clone(),
                presentation: node.meta.presentation.clone(),
                semantics: node.meta.semantics.clone(),
            }),
            _ => None,
        })
        .collect();

    Snapshot {
        as_of: EventTime {
            tick: engine.time.tick,
            micro: engine.time.micro,
            seq: engine.time.seq,
        },
        nodes,
        params,
        enums: Vec::<EnumDef>::new(),
        node_types: Vec::<NodeTypeDef>::new(),
    }
}

fn node_data_dto(node: &golden_core::Node) -> NodeDataDto {
    match &node.data {
        golden_core::NodeData::None => NodeDataDto {
            kind: NodeDataKind::None,
            container: None,
            parameter: None,
        },
        golden_core::NodeData::Container(container) => NodeDataDto {
            kind: NodeDataKind::Container,
            container: Some(ContainerDataDto {
                allowed_types: match &container.allowed_types {
                    golden_core::AllowedTypes::Any => Vec::<NodeTypeId>::new(),
                    golden_core::AllowedTypes::Only(list) => list.clone(),
                },
                folders_allowed: matches!(container.folders, golden_core::FolderPolicy::Allowed),
            }),
            parameter: None,
        },
        golden_core::NodeData::Parameter(param) => NodeDataDto {
            kind: NodeDataKind::Parameter,
            container: None,
            parameter: Some(param.clone()),
        },
        golden_core::NodeData::Custom(_) => NodeDataDto {
            kind: NodeDataKind::Custom("Custom".to_string()),
            container: None,
            parameter: None,
        },
        _ => NodeDataDto {
            kind: NodeDataKind::None,
            container: None,
            parameter: None,
        },
    }
}

fn collect_children(engine: &Engine, node_id: NodeId) -> Vec<NodeId> {
    let mut children = Vec::new();
    let mut current = engine.nodes.get(&node_id).and_then(|node| node.first_child);
    while let Some(child_id) = current {
        children.push(child_id);
        current = engine.nodes.get(&child_id).and_then(|node| node.next_sibling);
    }
    children
}

pub fn snapshot_value_for_param(param: &ParamDto) -> Value {
    param.value.clone()
}
