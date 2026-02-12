use serde::{Deserialize, Serialize};

use crate::ids::{DeclId, NodeUuid, ShortName};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SemanticsHint {
    pub intent: Option<String>,
    pub unit: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PresentationHint {
    pub widget: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeMeta {
    pub uuid: NodeUuid,
    pub decl_id: DeclId,
    pub short_name: ShortName,
    pub enabled: bool,
    pub label: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub semantics: SemanticsHint,
    pub presentation: PresentationHint,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NodeMetaPatch {
    pub enabled: Option<bool>,
    pub label: Option<String>,
    pub description: Option<Option<String>>,
    pub tags: Option<Vec<String>>,
    pub semantics: Option<SemanticsHint>,
    pub presentation: Option<PresentationHint>,
}
