use golden_schema::NodeTypeId;

#[derive(Clone, Debug)]
pub struct ContainerData {
    pub allowed_types: AllowedTypes,
    pub folders: FolderPolicy,
    pub limits: ContainerLimits,
}

#[derive(Clone, Debug)]
pub enum AllowedTypes {
    Any,
    Only(Vec<NodeTypeId>),
}

#[derive(Clone, Debug)]
pub enum FolderPolicy {
    Forbidden,
    Allowed,
}

#[derive(Clone, Debug)]
pub struct ContainerLimits {
    pub max_children: Option<usize>,
}
