use golden_schema::{NodeId, NodeUuid};

pub struct ReferenceMap;

impl ReferenceMap {
    pub fn resolve(&self, _uuid: NodeUuid) -> Option<NodeId> {
        // TODO: implement uuid -> NodeId mapping lookup.
        None
    }
}
