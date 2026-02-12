use crate::edits::EditOrigin;

pub struct EditSession {
    pub origin: EditOrigin,
    pub label: Option<String>,
}
