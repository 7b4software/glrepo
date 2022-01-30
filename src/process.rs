use crate::error::{Error, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use std::{thread, time};

///
/// Spawn a shell process and wait for it to end.
/// The stdout is **not** handled by the application.
/// It's up to the child process to handle the stdout/error if needed.
/// Arguments
/// 'project_name' Used when print error message if it fails to start or timeout
/// 'working_directory' Spawn shell in the specified working directory.
/// 'args' Arguments to pass to the shell process.
/// 'timeout' as Duration
/// Errors
/// Error::ShellCommand or Error::ShellCommandTimeout
pub fn spawn_shell_and_wait(
    project_name: &str,
    working_directory: &Path,
    args: Vec<String>,
    timeout: time::Duration,
) -> Result<()> {
    let now = time::Instant::now();
    match Command::new("sh")
        .current_dir(working_directory)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .arg("-c")
        .args(&args)
        .spawn()
    {
        Ok(mut child) => {
            loop {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        let res = match status.code() {
                            Some(0) => {
                                log::info!(
                                    "Project: '{}' Command: '{}' Exit success.",
                                    project_name,
                                    args.join(" "),
                                );
                                Ok(())
                            }
                            Some(code) => Err(Error::ShellCommandExit(
                                project_name.into(),
                                args.join(" "),
                                code,
                            )),
                            None => Err(Error::ShellCommandExit(
                                project_name.into(),
                                args.join(" "),
                                0xDEAD,
                            )),
                        };
                        return res;
                    }
                    Ok(None) => { /* Still running */ }
                    Err(e) => {
                        return Err(Error::ShellCommand(project_name.into(), args.join(" "), e));
                    }
                }
                thread::sleep(std::time::Duration::from_millis(50));
                if now.elapsed() >= timeout {
                    break;
                }
            }
            // Still running
            if let Err(e) = child.kill() {
                return Err(Error::ShellCommand(project_name.into(), args.join(" "), e));
            }
            Err(Error::ShellCommandTimeout(
                project_name.into(),
                args.join(" "),
            ))
        }
        Err(e) => Err(Error::ShellCommand(project_name.into(), args.join(" "), e)),
    }
}
