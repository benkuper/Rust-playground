use golden_schema::persistence::file_format::ProjectFile;

pub fn load_project(data: &str) -> Result<ProjectFile, serde_json::Error> {
    serde_json::from_str(data)
}
