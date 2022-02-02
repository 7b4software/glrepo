mod args;
mod error;
mod git;
mod manifest;
mod process;
mod threadpool;
use args::{Args, Command};
use error::{Error, Result};
use git::Git;
use manifest::GlProjects;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;
///
/// Read YAML Manifest from and return GlProjects structure on success.
/// # Arguments
///
/// - `manifest_file` Path to manifest file.
///
/// # Error
///
/// Error::Manifest.
fn load_manifest<P: AsRef<Path>>(p: P) -> Result<GlProjects> {
    log::info!("Read manifest from: '{}'", p.as_ref().display());
    let projects = GlProjects::try_from_yaml(&p).map_err(|e| {
        Error::Manifest(format!(
            "Can not read: '{}' cause: '{:#?}'",
            p.as_ref().display(),
            e
        ))
    })?;
    Ok(projects)
}

///
/// Do the command specified via command line
/// Only global actions is run here.
/// If an command was run return true else false.
/// If command failed it returns an Error.
///
/// # Arguments
/// * `args` - Command line argument data.
/// * `projects` Projects data structure.
///
/// # Error
///
/// see GlRepo::error::Error
fn do_single_command(args: &Args, projects: &mut GlProjects) -> Result<bool> {
    match &args.command {
        Command::Path { project } => {
            if let Some(project) = projects.projects.get(project) {
                println!("{}", &project.path.display());
            } else {
                return Err(Error::ProjectNotFound(project.into()));
            }
            Ok(true)
        }
        Command::List { fetch_url, path } => {
            for (name, project) in &projects.projects {
                print!("{}", name);
                if *fetch_url {
                    print!(",{}", project.fetch_url);
                }
                if *path {
                    print!(",{}", &project.path.display());
                }
                println!();
            }
            Ok(true)
        }
        Command::Create {
            run_command,
            project_name,
            path,
            fetch_url,
            revision,
            timeout_ms,
        } => {
            if projects.projects.get(project_name).is_some() {
                return Err(Error::General(format!(
                    "Project: '{}' already exists",
                    project_name
                )));
            }
            let repo = Git::init(path)?;
            repo.remote("origin", fetch_url)?;
            projects.insert(
                project_name,
                manifest::GlProject {
                    path: path.clone(),
                    fetch_url: fetch_url.clone(),
                    revision: revision.clone(),
                },
            );
            projects.save_to_yaml(&args.gl_manifest)?;
            if let Err(e) = process::spawn_shell_and_wait(
                &project_name,
                &path,
                run_command.into(),
                std::time::Duration::from_millis(*timeout_ms),
            ) {
                return Err(e);
            }

            Ok(true)
        }
        _ => Ok(false),
    }
}

///
/// Run command on every project in the manifest.
///
/// # Error
/// return GlRepo::Error on failure.
///
fn do_for_each_command(args: &Args, projects: &GlProjects) -> Result<()> {
    let (tx, rx) = channel();
    if projects.projects.is_empty() {
        log::warn!("There is no projects in the manifest");
        return Ok(());
    }

    let pool = ThreadPool::new(args.jobs);

    // Increment by one to make sure we don't accidently
    // terminate before all threads has been pushed
    // to the threadpool.
    let pending = Arc::new(Mutex::new(AtomicUsize::new(1)));
    let projects = projects.projects.clone();
    for (name, project) in projects {
        match &args.command {
            Command::Sync { projects } => {
                if projects.is_empty() || projects.iter().any(|p| *p == name) {
                    let tx2 = tx.clone();
                    let p2 = pending.clone();
                    p2.lock().unwrap().fetch_add(1, Ordering::Relaxed);
                    // Add function to ThreadPool
                    pool.execute(move || {
                        log::info!("Sync: {}", name);
                        if let Err(e) = Git::sync(&name, &project) {
                            tx2.send(e).ok();
                        }
                        p2.lock().unwrap().fetch_sub(1, Ordering::Relaxed);
                    });
                }
            }
            Command::ForEach { args, timeout_ms } => {
                let timeout_ms = timeout_ms.clone();
                let args = args.clone();
                let tx2 = tx.clone();
                let p2 = pending.clone();
                p2.lock().unwrap().fetch_add(1, Ordering::Relaxed);
                // Add function to ThreadPool
                pool.execute(move || {
                    if let Err(e) = process::spawn_shell_and_wait(
                        &name,
                        &project.path,
                        args,
                        std::time::Duration::from_millis(timeout_ms),
                    ) {
                        tx2.send(e).ok();
                    }
                    p2.lock().unwrap().fetch_sub(1, Ordering::Relaxed);
                });
            }
            Command::Changed { ls_files } => match Git::open(&project.path) {
                Ok(repo) => {
                    let files = repo.changed()?;
                    if !files.is_empty() {
                        println!("{}", name);

                        if *ls_files {
                            println!("{}", files);
                        }
                    }
                }
                Err(e) => {
                    log::error!("{} Make sure sync has been run", e);
                }
            },
            _ => panic!("Command: {:#?} not implemented", args.command),
        }
    }
    // okey, all projects are now pushed decrement by one
    // and then wait until all Projects has done it's job.
    let mut res = Ok(());
    pending.lock().unwrap().fetch_sub(1, Ordering::Relaxed);
    while *pending.lock().unwrap().get_mut() > 0 {
        match rx.try_recv() {
            Ok(e) => {
                log::error!("{}", e);
                res = Err(Error::General(format!("Command: {:?}", args.command)));
            }
            Err(_) => {
                // timeout or channel rip
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }

    // we need to empty
    loop {
        match rx.try_recv() {
            Ok(e) => {
                log::error!("{}", e);
                res = Err(Error::General(format!("Command: {:?}", args.command)));
            }
            Err(_) => {
                // All threads dead
                break;
            }
        }
    }
    res
}

///
/// Run application.
///
/// Load manifest.
/// Do command.
///
/// # Error
/// return Error on failure.
///
fn run_main() -> Result<()> {
    let args = Args::init()?;
    let mut projects = load_manifest(&args.gl_manifest)?;
    if do_single_command(&args, &mut projects)? {
        return Ok(());
    }

    log::info!("for each");
    do_for_each_command(&args, &projects)
}

fn main() {
    if let Err(e) = run_main() {
        log::error!("{}", e);
        std::process::exit(-1);
    }
    log::info!("Success");
}
