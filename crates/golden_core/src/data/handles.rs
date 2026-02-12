use golden_schema::{DeclId, NodeId, NodeUuid};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FolderHandle {
    pub node_id: NodeId,
}

impl FolderHandle {
    pub fn new(node_id: NodeId) -> Self {
        Self { node_id }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChildListHandle {
    pub node_id: NodeId,
}

impl ChildListHandle {
    pub fn new(node_id: NodeId) -> Self {
        Self { node_id }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PotentialSlotHandle {
    pub decl_id: DeclId,
    pub node_id: Option<NodeId>,
    pub uuid: Option<NodeUuid>,
}

impl PotentialSlotHandle {
    pub fn new(decl_id: DeclId) -> Self {
        Self {
            decl_id,
            node_id: None,
            uuid: None,
        }
    }
}
