use golden_schema::{NodeMeta, NodeMetaPatch};

pub fn apply_patch(meta: &mut NodeMeta, patch: &NodeMetaPatch) {
    if let Some(enabled) = patch.enabled {
        meta.enabled = enabled;
    }
    if let Some(label) = &patch.label {
        meta.label = label.clone();
    }
    if let Some(description) = &patch.description {
        meta.description = description.clone();
    }
    if let Some(tags) = &patch.tags {
        meta.tags = tags.clone();
    }
    if let Some(semantics) = &patch.semantics {
        meta.semantics = semantics.clone();
    }
    if let Some(presentation) = &patch.presentation {
        meta.presentation = presentation.clone();
    }
}
