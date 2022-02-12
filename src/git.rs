use crate::error::{Error, Result};
use crate::manifest::GlProject;
use git2::{build::CheckoutBuilder, Cred, FetchOptions, Repository, Statuses};
use std::collections::HashMap;
use std::fmt;
use std::io::Write;
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

fn do_fetch<'a>(
    repo: &'a Repository,
    name: &str,
    proj: &GlProject,
) -> Result<git2::AnnotatedCommit<'a>> {
    let mut fopt = fetch_options(name);
    let refs: Vec<&str> = vec![];
    repo.find_remote("origin")
        .and_then(|mut remote| remote.fetch(&refs, Some(&mut fopt), None))
        .and_then(|_| {
            repo.resolve_reference_from_short_name(&format!("origin/{}", &proj.reference))
        })
        .and_then(|ref_head| repo.reference_to_annotated_commit(&ref_head))
        .map_err(|e| Error::Git("fetch reference", e))
}

fn fast_forward(
    repo: &Repository,
    lb: &mut git2::Reference,
    rc: &git2::AnnotatedCommit,
) -> Result<()> {
    let name = match lb.name() {
        Some(s) => s.to_string(),
        None => String::from_utf8_lossy(lb.name_bytes()).to_string(),
    };
    let msg = format!("Fast-Forward: Setting {} to id: {}", name, rc.id());
    println!("{}", msg);
    lb.set_target(rc.id(), &msg)
        .map_err(|e| Error::Git("Set target", e))?;
    repo.set_head(&name)
        .map_err(|e| Error::Git("set head", e))?;
    repo.checkout_head(Some(
        git2::build::CheckoutBuilder::default()
            // For some reason the force is required to make the working directory actually get updated
            // I suspect we should be adding some logic to handle dirty working directory states
            // but this is just an example so maybe not.
            .force(),
    ))
    .map_err(|e| Error::Git("checkout head", e))?;
    Ok(())
}

fn do_merge<'a>(
    repo: &'a Repository,
    remote_branch: &str,
    fetch_commit: git2::AnnotatedCommit<'a>,
) -> Result<()> {
    // 1. do a merge analysis
    let analysis = repo
        .merge_analysis(&[&fetch_commit])
        .map_err(|e| Error::Git("do_merge", e))?;

    // 2. Do the appropriate merge
    if analysis.0.is_fast_forward() {
        log::info!("Doing a fast forward");
        // do a fast forward
        let refname = format!("refs/heads/{}", remote_branch);
        match repo.find_reference(&refname) {
            Ok(mut r) => {
                fast_forward(repo, &mut r, &fetch_commit)?;
            }
            Err(_) => {
                // The branch doesn't exist so just set the reference to the
                // commit directly. Usually this is because you are pulling
                // into an empty repository.
                repo.reference(
                    &refname,
                    fetch_commit.id(),
                    true,
                    &format!("Setting {} to {}", remote_branch, fetch_commit.id()),
                )
                .map_err(|e| Error::Git("Reference", e))?;
                repo.set_head(&refname)
                    .map_err(|e| Error::Git("set head", e))?;
                repo.checkout_head(Some(
                    git2::build::CheckoutBuilder::default()
                        .allow_conflicts(true)
                        .conflict_style_merge(true)
                        .force(),
                ))
                .map_err(|e| Error::Git("checkout head", e))?;
            }
        };
    } else if analysis.0.is_normal() {
        // do a normal merge
        return Err(Error::NotSupported(
            "Git merge: Only fast forward is supported",
        ));
    } else {
    }
    Ok(())
}

fn fetch_options(project_name: &str) -> FetchOptions<'static> {
    let mut cb = git2::RemoteCallbacks::new();
    let project_name = project_name.to_string();
    cb.transfer_progress(move |stats| {
        if stats.received_objects() == stats.total_objects() {
            print!(
                "{}: Resolving deltas \x1b[0K{}/{}\r",
                project_name,
                stats.indexed_deltas(),
                stats.total_deltas()
            );
        } else if stats.total_objects() > 0 {
            print!(
                "{}: Received \x1b[0K{}/{} objects ({}) in {} bytes\r",
                project_name,
                stats.received_objects(),
                stats.total_objects(),
                stats.indexed_objects(),
                stats.received_bytes()
            );
        }
        std::io::stdout().flush().ok();
        true
    });

    cb.credentials(|_url, username_from_url, _allowed_types| {
        Cred::ssh_key(
            username_from_url.unwrap_or(""),
            None,
            std::path::Path::new(&format!(
                "{}/.ssh/id_ed25519",
                std::env::var("HOME").unwrap_or_default()
            )),
            None,
        )
    });

    let mut fopt = git2::FetchOptions::new();
    fopt.remote_callbacks(cb);
    fopt.download_tags(git2::AutotagOption::All);
    fopt
}

impl Git {
    pub fn open<P: AsRef<Path>>(path: &P) -> Result<Self> {
        Ok(Self {
            repo: Repository::open(path).map_err(|e| Error::Git("open", e))?,
        })
    }

    pub fn init<P: AsRef<Path>>(path: &P) -> Result<Self> {
        if path.as_ref().exists() {
            return Self::open(&path);
        }
        Ok(Self {
            repo: Repository::init(path).map_err(|e| Error::Git("init", e))?,
        })
    }

    /// 'remote_name' Remote name example: "origin"
    /// 'fetch_url' Fetch URL.
    pub fn remote(&self, remote_name: &str, fetch_url: &str) -> Result<git2::Remote> {
        self.repo
            .remote(remote_name, fetch_url)
            .map_err(|e| Error::Git("", e))
    }

    ///
    /// Sync with upstream
    /// Doing clone path not exists
    /// Doing fetch if exists
    ///
    /// Return git object or an Error
    pub fn sync(project_name: &str, project: &GlProject) -> Result<()> {
        print!("\n{}: Syncing...\r", project_name);
        if project.path.exists() {
            let git = Self::open(&project.path)?;
            let fetch_commit = do_fetch(&git.repo, project_name, project)?;
            do_merge(&git.repo, &project.reference, fetch_commit)?;
        } else {
            let fops = fetch_options(project_name);
            let co = CheckoutBuilder::new();
            let mut builder = git2::build::RepoBuilder::new();
            builder.fetch_options(fops).with_checkout(co);
            let repo = builder
                .clone(&project.fetch_url, &project.path)
                .map_err(|e| Error::Git("clone", e))?;
            let fetch_commit = do_fetch(&repo, project_name, project)?;
            do_merge(&repo, &project.reference, fetch_commit)?;
        }
        Ok(())
    }

    pub fn status(&self) -> Result<Statuses> {
        let mut opt = git2::StatusOptions::new();
        opt.show(git2::StatusShow::IndexAndWorkdir);
        opt.include_untracked(true);
        self.repo
            .statuses(Some(&mut opt))
            .map_err(|e| Error::Git("status", e))
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
