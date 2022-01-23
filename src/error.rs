#[derive(Debug)]
pub enum Error {
    GitError(&'static str, git2::Error),
    General(String),
    ProjectNotFound(String),
}
