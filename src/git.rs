use crate::error::{Error, Result};
use crate::manifest::GlProject;
use git2::{build::CheckoutBuilder, FetchOptions, Repository, Statuses};
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
    fn fetch_options(project_name: &str) -> FetchOptions<'static> {
        let mut cb = git2::RemoteCallbacks::new();
        let project_name = project_name.to_string();
        println!();
        cb.transfer_progress(move |tp| {
            print!(
                "\r{}: received objects: {:08} of {:08}",
                project_name,
                tp.received_objects(),
                tp.total_objects()
            );
            true
        });

        let mut fopt = git2::FetchOptions::new();
        fopt.remote_callbacks(cb);
        fopt.download_tags(git2::AutotagOption::All);
        fopt
    }

    fn fetch(&self, name: &str, proj: &GlProject) -> Result<()> {
        let mut fopt = Self::fetch_options(name);
        let refs = vec![&proj.revision];
        self.repo
            .find_remote("origin")
            .and_then(|mut remote| {
                println!("{:?}", refs);
                remote.fetch(&refs, Some(&mut fopt), None)
            })
            .map_err(|e| Error::Git("fetch", e))
    }
}

impl Git {
    pub fn open<P: AsRef<Path>>(path: &P) -> Result<Self> {
        Ok(Self {
            repo: Repository::open(path).map_err(|e| Error::Git("open", e))?,
        })
    }

    pub fn init<P: AsRef<Path>>(path: P) -> Result<Self> {
        if path.as_ref().exists() {
            return Self::open(&path);
        }
        Ok(Self {
            repo: Repository::init(path).map_err(|e| Error::Git("init", e))?,
        })
    }

    /// 'remote_name' Remote name example: "origin"
    /// 'fetch_url' Fetch URL.
    pub fn remote(&self, name: &str, fetch_url: &str) -> Result<git2::Remote> {
        self.repo
            .remote(name, fetch_url)
            .map_err(|e| Error::Git("", e))
    }

    fn oid(identity: &str) -> Result<git2::Oid> {
        git2::Oid::from_str(identity).map_err(|e| Error::Git("Get OID from reference", e))
    }

    fn get_commit(&self, revision: &str) -> Result<git2::Commit> {
        self.repo
            .find_commit(Self::oid(revision)?)
            .map_err(|e| Error::Git("find commit", e))
    }

    ///
    /// Sync with upstream
    /// Doing clone path not exists
    /// Doing fetch if exists
    ///
    /// Return git object or an Error
    pub fn sync(project_name: &str, project: &GlProject) -> Result<()> {
        if project.path.exists() {
            let git = Self::open(&project.path)?;
            git.fetch(project_name, &project)?;
            let commit = git.get_commit(&project.revision)?;
            git.repo
                .branch(&project.revision, &commit, true)
                .map_err(|e| Error::Git("set branch", e))?;
        } else {
            let fops = Self::fetch_options(project_name);
            let co = CheckoutBuilder::new();
            let mut builder = git2::build::RepoBuilder::new();
            builder.fetch_options(fops).with_checkout(co);
            builder
                .clone(&project.fetch_url, &project.path)
                .map_err(|e| Error::Git("clone", e))?;
        }
        Ok(())
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
