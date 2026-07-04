#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectId(pub String);

#[derive(Debug, Clone)]
pub struct Project {
    pub id: ProjectId,
    pub video_url: String,
    pub title: String,
}
