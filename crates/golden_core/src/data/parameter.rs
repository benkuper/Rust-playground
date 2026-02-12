use golden_schema::NodeId;

pub type ParameterData = golden_schema::ParameterData;

pub struct ParameterHandle<T> {
    pub node_id: NodeId,
    _marker: std::marker::PhantomData<T>,
}

impl<T> ParameterHandle<T> {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            _marker: std::marker::PhantomData,
        }
    }
}
