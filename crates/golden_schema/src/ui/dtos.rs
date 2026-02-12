use serde::{Deserialize, Serialize};

use crate::ids::{DeclId, EnumId, EnumVariantId, NodeId, NodeTypeId, NodeUuid};
use crate::meta::{NodeMeta, PresentationHint, SemanticsHint};
use crate::persistence::NodeDataDto;
use crate::values::{ChangePolicy, UpdatePolicy, Value, ValueConstraints};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeDto {
    pub node_id: NodeId,
    pub uuid: NodeUuid,
    pub node_type: NodeTypeId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decl_id: Option<DeclId>,
    pub meta: NodeMeta,
    pub data: NodeDataDto,
    pub children: Vec<NodeId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParamDto {
    pub param_node_id: NodeId,
    pub value: Value,
    pub read_only: bool,
    pub update_policy: UpdatePolicy,
    pub change_policy: ChangePolicy,
    pub constraints: ValueConstraints,
    pub presentation: PresentationHint,
    pub semantics: SemanticsHint,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnumVariantDef {
    pub variant_id: EnumVariantId,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnumDef {
    pub enum_id: EnumId,
    pub variants: Vec<EnumVariantDef>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeTypeDef {
    pub node_type: NodeTypeId,
    pub label: String,
    pub palette_allowed_children: Vec<NodeTypeId>,
}
