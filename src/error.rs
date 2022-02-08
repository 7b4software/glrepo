use std::fmt;
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug)]
pub enum Error {
    /// Git errors
    /// 'command'
    /// 'git2::error'
    Git(&'static str, git2::Error),
    /// General error
    General(String),
    /// Summary error
    Summary(String),
    /// Not supported
    NotSupported(&'static str),
    /// Project 'name' not found
    ProjectNotFound(String),
    /// Manifest error
    Manifest(String),
    /// Shell command failed 'project' 'command' 'io::Error'
    ShellCommand(String, String, std::io::Error),
    /// Command timeout 'project_name' 'command'
    ShellCommandTimeout(String, String),
    /// Command timeout 'project_name' 'command' 'exit code'
    ShellCommandExit(String, String, i32),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Git(command, e) => {
                write!(f, "Git {}: {}", command, e.message())
            }
            Error::General(s) => write!(f, "{}", s),
            Error::NotSupported(s) => write!(f, "{}", s),
            Error::Summary(s) => write!(f, "{}", s),
            Error::ProjectNotFound(name) => write!(f, "Project: '{}' not found.", name),
            Error::Manifest(s) => write!(f, "Manifest: {}", s),
            Error::ShellCommand(p, s, e) => {
                write!(f, "{}: Shell command: '{}' failed cause: '{}'", p, s, e)
            }
            Error::ShellCommandTimeout(p, s) => {
                write!(f, "{}: Shell command: '{}' failed cause: 'timeout'", p, s)
            }
            Error::ShellCommandExit(p, s, code) => {
                write!(
                    f,
                    "{}: Shell command: '{}' failed with exit code: {}",
                    p, s, code
                )
            }
        }
    }
}
