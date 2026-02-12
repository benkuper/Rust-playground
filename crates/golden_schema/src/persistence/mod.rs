use serde::{Deserialize, Serialize};

pub mod file_format;

use crate::ids::{DeclId, NodeTypeId, NodeUuid};
use crate::meta::{NodeMeta, NodeMetaPatch};
use crate::values::{ParameterData, Value};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NodeDataKind {
    None,
    Container,
    Parameter,
    Custom(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContainerDataDto {
    pub allowed_types: Vec<NodeTypeId>,
    pub folders_allowed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeDataDto {
    pub kind: NodeDataKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<ContainerDataDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter: Option<ParameterData>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NodeRecord {
    Full(FullNodeRecord),
    Delta(DeltaNodeRecord),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FullNodeRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decl_id: Option<DeclId>,
    #[serde(rename = "type")]
    pub node_type: NodeTypeId,
    pub uuid: NodeUuid,
    pub meta: NodeMeta,
    pub data: NodeDataDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<NodeRecord>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeltaNodeRecord {
    pub decl_id: DeclId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<NodeUuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<NodeMetaPatch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<NodeRecord>,
}
