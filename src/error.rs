use std::fmt;
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug)]
pub enum Error {
    Git(&'static str, git2::Error),
    General(String),
    ProjectNotFound(String),
    Manifest(String),
    ShellCommand(String, std::io::Error),
    ShellCommandTimeout(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Git(s, e) => write!(f, "Git {} {}", s, e.message()),
            Error::General(s) => write!(f, "General: {}", s),
            Error::ProjectNotFound(name) => write!(f, "Project: '{}' not found.", name),
            Error::Manifest(s) => write!(f, "Manifest: {}", s),
            Error::ShellCommand(s, e) => write!(f, "Shell command: '{}' failed cause: '{}'", s, e),
            Error::ShellCommandTimeout(s) => {
                write!(f, "Shell command: '{}' failed cause: 'timeout'", s)
            }
        }
    }
}
