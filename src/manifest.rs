use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{fmt, fs, io};
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct GlProject {
    pub fetch_url: String,
    pub path: PathBuf,
    pub revision: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct GlProjects {
    pub projects: HashMap<String, GlProject>,
}

impl GlProjects {
    /// Returns a GlProjects data structure from a specified repo manifest in GlProject YAML
    /// format.
    /// Error
    /// std::io::Error
    /// # Arguments
    ///
    /// * `manifest_file` full path to YAML manifest.
    ///
    pub fn try_from_yaml<P: AsRef<Path>>(manifest_file: &P) -> Result<Self, io::Error> {
        let s = fs::read_to_string(manifest_file)?;
        serde_yaml::from_str::<GlProjects>(&s)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{}", e)))
    }

    pub fn insert(&mut self, name: &str, project: GlProject) {
        self.projects.insert(name.into(), project);
    }

    /// Save to a YAML file if file already exists it will be overwritten.
    /// Error
    /// std::io::Error
    /// # Arguments
    ///
    /// * `manifest_file` full path to YAML manifest.
    ///
    pub fn save_to_yaml<P: AsRef<Path>>(&self, manifest: P) -> Result<(), io::Error> {
        fs::write(manifest, serde_yaml::to_string(self).unwrap())
    }
}

impl fmt::Display for GlProject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "|Fetch   |{:<70}|", self.fetch_url)?;
        writeln!(f, "|Path    |{:<70}|", &self.path.display())?;
        writeln!(f, "|Revision|{:<70}|", self.revision)
    }
}

impl fmt::Display for GlProjects {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (name, project) in &self.projects {
            writeln!(f, "# {}", name)?;
            writeln!(f)?;
            writeln!(f, "{}", project)?;
        }
        write!(f, "")
    }
}
