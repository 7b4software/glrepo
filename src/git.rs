use crate::error::Error;
use crate::manifest::GlProject;
use git2::{FetchOptions, Repository, Statuses};
use std::path::Path;
use std::{fmt, io};
pub struct Git {
    repo: Repository,
}

type Result<T> = std::result::Result<T, Error>;

struct GitStatus(git2::Status, String);
impl fmt::Display for GitStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let path = &self.1;
        if self.0.is_index_modified() || self.0.is_index_new() {
            write!(f, "|{:<20}|{:<20}|{:<20}|", path, "", "")
        } else if self.0.is_wt_modified() || self.0.is_wt_new() {
            write!(f, "|{:<20}|{:<20}|{:<20}|", "", path, "")
        } else if self.0.is_ignored() {
            write!(f, "|{:<20}|{:<20}|{:<20}|", "", "", path)
        } else {
            write!(f, "|{}|", path)
        }
    }
}

impl Git {
    fn fetch(repo: &Repository, proj: &GlProject) -> Result<()> {
        let mut cb = git2::RemoteCallbacks::new();
        cb.transfer_progress(|tp| {
            println!(
                "Received: {} of {}",
                tp.received_objects(),
                tp.total_objects()
            );
            true
        });
        let mut fopt = FetchOptions::new();
        fopt.remote_callbacks(cb);
        fopt.download_tags(git2::AutotagOption::All);
        let refs = vec![&proj.revision];
        repo.find_remote("origin")
            .and_then(|mut remote| remote.fetch(&refs, Some(&mut fopt), None))
            .map_err(|e| Error::GitError("fetch", e))
    }
}

impl Git {
    pub fn open<P: AsRef<Path>>(path: &P) -> Result<Self> {
        Ok(Self {
            repo: Repository::open(path).map_err(|e| Error::GitError("open", e))?,
        })
    }

    /// Sync with upstream
    /// Doing clone path not exists
    /// Doing fetch if exists
    pub fn sync(project: &GlProject) -> Result<Self> {
        let repo = if project.path.exists() {
            Self::open(&project.path)?.repo
        } else {
            Repository::clone(&project.fetch_url, &project.path)
                .map_err(|e| Error::GitError("clone", e))?
        };
        Self::fetch(&repo, project)?;
        Ok(Self { repo })
    }

    pub fn status(&self) -> io::Result<Statuses> {
        let mut opt = git2::StatusOptions::new();
        opt.show(git2::StatusShow::IndexAndWorkdir);
        opt.include_untracked(true);
        Ok(self.repo.statuses(Some(&mut opt)).unwrap())
    }
}
