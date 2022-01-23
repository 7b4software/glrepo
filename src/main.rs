mod args;
mod error;
mod git;
mod manifest;
mod threadpool;
use args::{Args, Command};
use error::Error;
use git::Git;
use manifest::GlProjects;
fn load_manifest(args: &Args) -> Result<GlProjects, Error> {
    log::info!("Read manifest from: '{}'", args.gl_manifest.display());
    let projects = GlProjects::try_from_yaml(&args.gl_manifest).map_err(|e| {
        Error::General(format!(
            "Can not read: '{}' cause: '{:#?}'",
            args.gl_manifest.display(),
            e
        ))
    })?;
    Ok(projects)
}

fn do_single_action(args: &Args, projects: &GlProjects) -> Result<bool, Error> {
    match &args.command {
        Command::ProjectPath { project } => {
            if let Some(project) = projects.projects.get(project) {
                println!("{}", &project.path.display());
            } else {
                return Err(Error::ProjectNotFound(project.into()));
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn run_main() -> Result<(), Error> {
    let args = Args::init()?;
    let projects = load_manifest(&args)?;
    if do_single_action(&args, &projects)? == true {
        return Ok(());
    }
    for (name, project) in &projects.projects {
        match &args.command {
            Command::Sync { projects } => {
                if projects.is_empty() || projects.iter().any(|p| p == name) {
                    log::info!("Sync: {}", name);
                    Git::sync(project)?;
                }
            }
            Command::ListProjects { fetch_url, path } => {
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
            }
            _ => log::warn!("Command: {:#?}", args.command),
        }
    }

    Ok(())
}

fn main() {
    let _ = simple_logger::SimpleLogger::new()
        .without_timestamps()
        .with_level(log::LevelFilter::Info)
        .init();

    if let Err(e) = run_main() {
        log::error!("{:#?}", e);
        std::process::exit(-1);
    }
}
