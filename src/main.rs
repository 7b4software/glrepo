mod args;
mod error;
mod git;
mod manifest;
mod threadpool;
use args::{Args, Command};
use error::{Error, Result};
use git::Git;
use manifest::GlProjects;
use std::path::Path;
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
fn do_single_command(args: &Args, projects: &GlProjects) -> Result<bool> {
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
    // For each commands are handled here...
    for (name, project) in &projects.projects {
        match &args.command {
            Command::Sync { projects } => {
                if projects.is_empty() || projects.iter().any(|p| p == name) {
                    log::info!("Sync: {}", name);
                    Git::sync(project)?;
                }
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
    let projects = load_manifest(&args.gl_manifest)?;
    if do_single_command(&args, &projects)? {
        return Ok(());
    }

    do_for_each_command(&args, &projects)
}

fn main() {
    if let Err(e) = run_main() {
        log::error!("{}", e);
        std::process::exit(-1);
    }
}
