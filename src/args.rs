use crate::error::Error;
use clap::{Parser};
use std::path::{Path, PathBuf};
#[derive(Parser, Debug)]
pub enum Command {
    /// Fetch
    Init,
    /// Sync one or all upstream
    Sync {
        projects: Vec<String>,
    },
    ListProjects {
        #[clap(short, long)]
        fetch_url: bool,
        #[clap(short, long)]
        path: bool,
    },
    ProjectPath {
        project: String,
    },
}

#[derive(Parser)]
#[clap(version, about)]
pub struct Args {
    #[clap(short = 'c', long = "config-directory", default_value = "")]
    pub gl_config_home: PathBuf,
    #[clap(short = 'm', long = "manifest", default_value = "default.yaml")]
    pub gl_manifest: PathBuf,
    #[clap(subcommand)]
    pub command: Command,
}

impl Args {
    pub fn init() -> Result<Self, Error> {
        let mut args = Args::parse();
        if args.gl_config_home.starts_with("") {
            if let Ok(home) = std::env::var("GLREPO_CONFIG_HOME") {
                args.gl_config_home = PathBuf::from(home);
            } else {
                args.gl_config_home = std::env::var("HOME")
                    .map(|p| PathBuf::from(&p))
                    .unwrap_or_else(|_| PathBuf::new());
                args.gl_config_home.push(".config/glrepo/");
            }
        }

        args.gl_manifest = Path::new(&args.gl_config_home).join(&args.gl_manifest);
        args.gl_manifest = args.gl_manifest.canonicalize().map_err(|e| {
            Error::General(format!(
                "Expand: '{}' failed cause: {}",
                args.gl_manifest.display(),
                e
            ))
        })?;
        Ok(args)
    }
}
