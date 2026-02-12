use serde::{Deserialize, Serialize};

use crate::persistence::NodeRecord;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectFile {
    pub version: String,
    pub root: NodeRecord,
}
