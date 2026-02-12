use std::collections::HashMap;

use golden_schema::{
    ChangePolicy, DeclId, NodeTypeId, PresentationHint, SavePolicy, SemanticsHint, UpdatePolicy,
    Value, ValueConstraints,
};

use crate::data::{AllowedTypes, FolderPolicy};

#[derive(Clone, Debug)]
pub struct DeclaredChild {
    pub decl_id: DeclId,
    pub node_type: NodeTypeId,
    pub default_label: Option<String>,
    pub default_enabled: bool,
}

#[derive(Clone, Debug)]
pub enum InboxBehavior {
    Coalesce,
    Append,
}

#[derive(Clone, Debug)]
pub struct ParamDecl {
    pub decl_id: DeclId,
    pub default: Value,
    pub constraints: ValueConstraints,
    pub read_only: bool,
    pub update: UpdatePolicy,
    pub change: ChangePolicy,
    pub save: SavePolicy,
    pub semantics: SemanticsHint,
    pub presentation: PresentationHint,
    pub folder: Option<DeclId>,
    pub behavior: InboxBehavior,
    pub alias: Option<String>,
}

#[derive(Clone, Debug)]
pub struct FolderDecl {
    pub decl_id: DeclId,
    pub label: Option<String>,
    pub alias_prefix: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ContainerDecl {
    pub allowed_types: AllowedTypes,
    pub folders: FolderPolicy,
}

#[derive(Clone, Debug)]
pub struct PotentialSlot {
    pub decl_id: DeclId,
    pub allowed_types: Vec<NodeTypeId>,
}

#[derive(Clone, Debug)]
pub struct NodeSchema {
    pub declared_children: Vec<DeclaredChild>,
    pub potential_slots: Vec<PotentialSlot>,
    pub params: Vec<ParamDecl>,
    pub folders: Vec<FolderDecl>,
    pub container: Option<ContainerDecl>,
}

impl NodeSchema {
    pub fn new() -> Self {
        Self {
            declared_children: Vec::new(),
            potential_slots: Vec::new(),
            params: Vec::new(),
            folders: Vec::new(),
            container: None,
        }
    }
}

pub trait GoldenNodeDecl {
    fn node_type() -> NodeTypeId;
    fn schema() -> NodeSchema;

    fn register_schema(registry: &mut SchemaRegistry)
    where
        Self: Sized,
    {
        registry.register(Self::node_type(), Self::schema());
    }
}

pub struct SchemaRegistry {
    types: HashMap<NodeTypeId, NodeSchema>,
}

impl SchemaRegistry {
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
        }
    }

    pub fn register(&mut self, node_type: NodeTypeId, schema: NodeSchema) {
        self.types.insert(node_type, schema);
    }

    pub fn schema_for(&self, node_type: &NodeTypeId) -> Option<&NodeSchema> {
        self.types.get(node_type)
    }
}
