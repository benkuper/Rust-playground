use serde::{Deserialize, Serialize};

use crate::events::{Event, EventTime};
use crate::ids::{NodeId, NodeTypeId, NodeUuid};
use crate::meta::NodeMetaPatch;
use crate::ui::dtos::{EnumDef, NodeDto, NodeTypeDef, ParamDto};
use crate::values::Value;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MessageEnvelope<T> {
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub req_id: Option<String>,
    pub payload: T,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ScopeMode {
    Root,
    Subtree,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Scope {
    pub mode: ScopeMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_uuid: Option<NodeUuid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hello {
    pub protocol_version: String,
    pub client_name: String,
    pub client_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_scope: Option<Scope>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HelloAck {
    pub protocol_version: String,
    pub server_name: String,
    pub server_version: String,
    pub features: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GetSnapshot {
    pub scope: Scope,
    pub include_schema: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub as_of: EventTime,
    pub nodes: Vec<NodeDto>,
    pub params: Vec<ParamDto>,
    pub enums: Vec<EnumDef>,
    pub node_types: Vec<NodeTypeDef>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Subscribe {
    pub scope: Scope,
    pub from: EventTime,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventBatch {
    pub events: Vec<Event>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EditOrigin {
    UI,
    Network,
    Script,
    Internal,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Propagation {
    Immediate,
    EndOfTick,
    NextTick,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BeginEdit {
    pub origin: EditOrigin,
    pub label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BeginEditAck {
    pub edit_session_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EndEdit {
    pub edit_session_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SetParam {
    pub edit_session_id: Option<String>,
    pub param_node_id: NodeId,
    pub value: Value,
    pub propagation: Propagation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PatchMeta {
    pub edit_session_id: Option<String>,
    pub node_id: NodeId,
    pub patch: NodeMetaPatch,
    pub propagation: Propagation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CreateNode {
    pub edit_session_id: Option<String>,
    pub parent_id: NodeId,
    pub node_type: NodeTypeId,
    pub label: Option<String>,
    pub propagation: Propagation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoveNode {
    pub edit_session_id: Option<String>,
    pub node_id: NodeId,
    pub new_parent_id: NodeId,
    pub new_index: usize,
    pub propagation: Propagation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeleteNode {
    pub edit_session_id: Option<String>,
    pub node_id: NodeId,
    pub propagation: Propagation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ack {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorInfo>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
}
