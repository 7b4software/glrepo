use crate::error::Error;
use clap::Parser;
use std::path::{Path, PathBuf};
#[derive(Parser, Debug)]
pub enum Command {
    /// Fetch
    Init,
    /// Sync one or all upstream
    Sync { projects: Vec<String> },
    /// run command in shell on each project
    ForEach {
        /// Arguments passed to the shell process.
        args: String,
        /// Max time, in milliseconds, before child process gets killed.
        #[clap(short, long, default_value = "500")]
        timeout_ms: u64,
    },
    /// List projects configuration
    List {
        #[clap(short, long)]
        fetch_url: bool,
        #[clap(short, long)]
        path: bool,
    },
    /// List a projects path
    Path { project: String },
    /// List all projects that has changes (Note! untracked files is also seen as changes).
    Changed {
        /// List changed files in the project.
        #[clap(short, long)]
        ls_files: bool,
    },
    /// Create a new project. Command will fail if project already exist in manifest.
    Create {
        /// Run command after creation. (use " around command. Example: -c "ls -l --color")
        #[clap(short = 'c', long)]
        run_command: String,
        /// Max time, in milliseconds, before child process gets killed.
        #[clap(short, long, default_value = "500")]
        timeout_ms: u64,
        /// Project name
        project_name: String,
        /// Path to store local repository
        path: PathBuf,
        /// URL to remote repo
        fetch_url: String,
        /// Branch
        #[clap(default_value = "main")]
        reference: String,
        /// Auto sync
        #[clap(short, long)]
        auto_sync: bool,
    },
}

#[derive(Parser)]
#[clap(version, about, author)]
pub struct Args {
    /// If not specified try read from environment GLREPO_CONFIG_HOME or else ~/.config/glrepo/
    #[clap(short = 'c', long = "config-directory", default_value = "")]
    pub gl_config_home: PathBuf,
    /// If non absolute path, read gl_config_home will be prepend to this path.
    #[clap(short = 'm', long = "manifest", default_value = "default.yaml")]
    pub gl_manifest: PathBuf,
    /// Verbose flag 0 info, 1 debug, >= 2 trace.
    #[clap(long, short, parse(from_occurrences))]
    pub verbose: usize,
    /// Number of parallel jobs
    #[clap(long, short, default_value = "1")]
    pub jobs: usize,
    #[clap(subcommand)]
    pub command: Command,
}

impl Args {
    pub fn init() -> Result<Self, Error> {
        let mut args = Args::parse();
        let level = match args.verbose {
            0 => log::LevelFilter::Info,
            1 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        };

        // At least one job
        args.jobs = std::cmp::max(args.jobs, 1);
        let _ = simple_logger::SimpleLogger::new()
            .without_timestamps()
            .with_level(level)
            .init();

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

        if !args.gl_manifest.starts_with("/") && !args.gl_manifest.starts_with("./") {
            args.gl_manifest = Path::new(&args.gl_config_home).join(&args.gl_manifest);
        }
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
