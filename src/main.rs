mod args;
mod error;
mod git;
mod manifest;
mod process;
mod threadpool;
use args::{Args, Command};
use colored::*;
use error::{Error, Result};
use git::Git;
use manifest::GlProjects;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;

/// Errors that comes from a thread in case a command fails to execute.
struct ThreadError(String, Error);

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
    let projects = GlProjects::try_from_yaml(&p)?;
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
            reference,
            auto_sync,
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
                    name: project_name.to_string(),
                    path: path.clone(),
                    fetch_url: fetch_url.clone(),
                    reference: reference.clone(),
                    auto_sync: *auto_sync,
                },
            );
            projects.save_to_yaml(&args.gl_manifest)?;
            if let Err(e) = process::spawn_shell_and_wait(
                project_name,
                path,
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

    // Increment by one to make sure we don't terminate
    // until all threads has been handled by the thread pool.
    let pending = Arc::new(Mutex::new(AtomicUsize::new(1)));
    let projects = projects.projects.clone();
    for (name, project) in projects {
        match &args.command {
            Command::Sync { projects } => {
                let filtered_projects = !projects.is_empty();
                if !filtered_projects || projects.iter().any(|p| *p == name) {
                    let tx2 = tx.clone();
                    let p2 = pending.clone();
                    p2.lock().unwrap().fetch_add(1, Ordering::Relaxed);
                    // Add function to the thread pool.
                    pool.execute(move || {
                        // Only sync projects that has auto_sync set to true
                        // or sync if projects was explicit selected.
                        if project.auto_sync || filtered_projects {
                            log::info!("Sync: {}", name);
                            if let Err(e) = Git::sync(&name, &project) {
                                tx2.send(ThreadError(name.clone(), e)).ok();
                            }
                        }
                        p2.lock().unwrap().fetch_sub(1, Ordering::Relaxed);
                    });
                }
            }
            Command::ForEach { args, timeout_ms } => {
                let timeout_ms = *timeout_ms;
                let args = args.clone();
                let tx2 = tx.clone();
                let p2 = pending.clone();
                p2.lock().unwrap().fetch_add(1, Ordering::Relaxed);
                // Add function to the thread pool.
                pool.execute(move || {
                    if let Err(e) = process::spawn_shell_and_wait(
                        &name,
                        &project.path,
                        args,
                        std::time::Duration::from_millis(timeout_ms),
                    ) {
                        tx2.send(ThreadError(name.clone(), e)).ok();
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
    // All projects are now pushed decrement by one
    // and then wait until all Projects has done it's job.
    pending.lock().unwrap().fetch_sub(1, Ordering::Relaxed);
    let mut errors = vec![];
    while *pending.lock().unwrap().get_mut() > 0 {
        match rx.try_recv() {
            Ok(e) => {
                log::error!("Project: {}: {}", e.0, e.1);
                errors.push(e);
            }
            Err(_) => {
                // timeout or channel endpoint terminated
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }

    // We need to empty any errors coming from threads.
    // If there are error messages left.
    while let Ok(e) = rx.try_recv() {
        log::error!("Project: {}: {}", e.0.bold(), e.1);
        errors.push(e);
    }

    if !errors.is_empty() {
        eprintln!();
        let mut summary = format!(
            "The following {} has errors:\n\n",
            if errors.len() == 1 {
                String::from("project")
            } else {
                format!("{} projects", errors.len())
            }
        );
        for e in errors.iter() {
            summary += &format!("{}\n", e.0);
        }
        return Err(Error::Summary(summary));
    }
    Ok(())
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

    do_for_each_command(&args, &projects)
}

fn main() {
    if let Err(e) = run_main() {
        log::error!("{}", e);
        std::process::exit(-1);
    }
    log::info!("Success");
}
