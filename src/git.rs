use crate::error::{Error, Result};
use crate::manifest::GlProject;
use git2::{FetchOptions, Repository, Statuses};
use std::collections::HashMap;
use std::fmt;
use std::path::Path;
pub struct Git {
    repo: Repository,
}

#[derive(Debug, Default)]
pub struct ChangedFiles {
    files: HashMap<String, git2::Status>,
}

impl ChangedFiles {
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

impl fmt::Display for ChangedFiles {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "|{:<32}|{:<12}|", "File", "State")?;
        writeln!(f, "|--------------------------------|------------|",)?;
        for (file, status) in &self.files {
            // ugly hack padding does not work of debug outputs it seems
            // so we first make a string and pass it on to write.
            let s = format!("{:#?}", status);
            writeln!(f, "|{:<32}|{:<12}|", file, s)?;
        }
        write!(f, "")
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
            .map_err(|e| Error::Git("fetch", e))
    }
}

impl Git {
    pub fn open<P: AsRef<Path>>(path: &P) -> Result<Self> {
        Ok(Self {
            repo: Repository::open(path).map_err(|e| Error::Git("open", e))?,
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
                .map_err(|e| Error::Git("clone", e))?
        };
        Self::fetch(&repo, project)?;
        Ok(Self { repo })
    }

    pub fn status(&self) -> Result<Statuses> {
        let mut opt = git2::StatusOptions::new();
        opt.show(git2::StatusShow::IndexAndWorkdir);
        opt.include_untracked(true);
        Ok(self
            .repo
            .statuses(Some(&mut opt))
            .map_err(|e| Error::Git("status", e))?)
    }

    pub fn changed(&self) -> Result<ChangedFiles> {
        let mut files = ChangedFiles::default();
        for entry in self.status()?.iter() {
            let status = entry.status();
            files
                .files
                .insert(entry.path().unwrap_or("???Invalid UTF8?").into(), status);
        }

        Ok(files)
    }
}
