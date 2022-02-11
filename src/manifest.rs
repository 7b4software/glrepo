use crate::error::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{fmt, fs};
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct GlProject {
    #[serde(skip)]
    pub name: String,
    pub fetch_url: String,
    #[serde(default = "PathBuf::default")]
    pub path: PathBuf,
    #[serde(default = "String::default")]
    pub reference: String,
    #[serde(default = "default_true")]
    pub auto_sync: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct GlProjects {
    #[serde(default = "PathBuf::default")]
    pub projects_dir: PathBuf,
    #[serde(default = "String::default")]
    pub default_reference: String,
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
    pub fn try_from_yaml<P: AsRef<Path>>(manifest_file: &P) -> Result<Self, Error> {
        let s = fs::read_to_string(manifest_file)
            .map_err(|e| Error::Manifest(format!("Could not load manifest file cause: {}", e)))?;
        Self::verify(
            serde_yaml::from_str::<GlProjects>(&s).map_err(|e| Error::General(format!("{}", e)))?,
        )
    }

    fn verify(mut self) -> Result<Self, Error> {
        if self.projects_dir != PathBuf::default() && self.projects_dir.canonicalize().is_err() {
            return Err(Error::General(
                "The projects_dir must point to an existing directory!".to_string(),
            ));
        }
        for (name, project) in self.projects.iter_mut() {
            let mut full_path = self.projects_dir.clone();
            // No path set for project
            project.path = if project.path.has_root() {
                project.path.clone()
            } else if project.path.file_name().is_none() {
                // Append name to it
                full_path.push(name);
                full_path
            } else {
                full_path.push(&project.path);
                full_path
            };

            if project.reference.is_empty() {
                project.reference = self.default_reference.clone();
                if project.reference.is_empty() {
                    return Err(Error::General(format!("Project: {} are missing reference and the manifest file does not have the field: default_reference!", name)));
                }
            }
        }
        Ok(self)
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
    pub fn save_to_yaml<P: AsRef<Path>>(&self, manifest: &P) -> Result<(), Error> {
        fs::write(manifest, serde_yaml::to_string(self).unwrap()).map_err(|e| {
            Error::Manifest(format!(
                "output to: '{}' cause: '{}'",
                manifest.as_ref().display(),
                e
            ))
        })
    }
}

impl fmt::Display for GlProject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "|Fetch    |{:<70}|", self.fetch_url)?;
        writeln!(f, "|Path     |{:<70}|", &self.path.display())?;
        writeln!(f, "|Reference|{:<70}|", self.reference)?;
        writeln!(f, "|Auto sync|{:<70}|", self.auto_sync)
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_verify_without_path() {
        let yaml: &str = r"---
            projects:
                foo:
                    fetch_url: https://apa
                    reference: main";

        let projs: GlProjects = serde_yaml::from_str::<GlProjects>(yaml)
            .unwrap()
            .verify()
            .unwrap();
        let project = projs.projects.get("foo").unwrap();
        assert_eq!(PathBuf::from("foo"), project.path);
        assert_eq!(String::from("main"), project.reference);
    }

    #[test]
    fn test_verify_with_def_path_without_name() {
        let yaml: &str = r"---
            projects_dir: /tmp
            projects:
                bas:
                    fetch_url: https://apa
                    reference: main";

        let projs: GlProjects = serde_yaml::from_str::<GlProjects>(yaml)
            .unwrap()
            .verify()
            .unwrap();
        let project = projs.projects.get("bas").unwrap();
        assert_eq!(PathBuf::from("/tmp/bas"), project.path);
    }

    #[test]
    fn test_verify_path_with_root() {
        let yaml: &str = r"---
            projects_dir: /usr/bin
            projects:
                bas:
                    fetch_url: https://apa
                    reference: main
                    path: /foo/bar/apa";

        let projs = serde_yaml::from_str::<GlProjects>(yaml)
            .unwrap()
            .verify()
            .unwrap();
        let project = projs.projects.get("bas").unwrap();
        assert_eq!(PathBuf::from("/foo/bar/apa"), project.path);
    }
    #[test]
    fn test_verify_with_def_path_with_path() {
        let yaml: &str = r"---
            projects_dir: /tmp
            projects:
                bas:
                    fetch_url: https://apa
                    reference: main
                    path: apa";

        let projs = serde_yaml::from_str::<GlProjects>(yaml)
            .unwrap()
            .verify()
            .unwrap();
        let project = projs.projects.get("bas").unwrap();
        assert_eq!(PathBuf::from("/tmp/apa"), project.path);
    }
}
