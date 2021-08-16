use std::ffi::OsString;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use chrono::{DateTime, Local, TimeZone};
use color_eyre::eyre::{self, ContextCompat, WrapErr};
use tracing::{info, instrument, span, Level};

pub use chrono;

static DATE_FMT: &str = "%Y-%m-%d";
static FILENAME_DATETIME_FMT: &str = "%Y-%m-%dT%H_%M_%S";

#[instrument]
pub fn create_repo_path(repo_path: impl AsRef<Path> + Debug) -> eyre::Result<()> {
    std::fs::create_dir_all(repo_path).wrap_err("Failed to create `repo_path`")
}

#[instrument]
pub fn is_inside_git_work_tree(path: impl AsRef<Path> + Debug) -> eyre::Result<bool> {
    let status = Command::new("git")
        .args(&["git", "rev-parse", "--is-inside-work-tree"])
        .current_dir(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .wrap_err("Failed to execute `git`")?;
    Ok(status.success())
}

#[instrument]
pub fn git_init(path: impl AsRef<Path> + Debug) -> eyre::Result<()> {
    let mut cmd = Command::new("git")
        .arg("init")
        .current_dir(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .wrap_err("Failed to execute `git`")?;
    let status = cmd.wait().wrap_err("Failed to wait on `git`")?;
    let output = cmd
        .wait_with_output()
        .wrap_err("Failed to get output from `git`")?;
    if status.success() {
        Ok(())
    } else {
        let mut err = Err(eyre::eyre!("`git init` failed"));
        if !output.stdout.is_empty() {
            err = err.wrap_err(String::from_utf8_lossy(&output.stdout).into_owned());
        }
        if !output.stderr.is_empty() {
            err = err.wrap_err(String::from_utf8_lossy(&output.stderr).into_owned());
        }
        err
    }
}

#[instrument]
pub fn ensure_symlink(
    path: impl AsRef<Path> + Debug,
    dest: impl AsRef<Path> + Debug,
) -> eyre::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        if !parent.exists() {
            info!(?parent, "Creating parent dir");
            std::fs::create_dir_all(parent)
                .wrap_err_with(|| format!("Failed to create {}", &parent.display()))?;
        }
    }

    if let Ok(actual_dest) = std::fs::read_link(&path).and_then(std::fs::canonicalize) {
        let canonical_path = path
            .as_ref()
            .canonicalize()
            .wrap_err("Failed to canonicalize path")?;
        if actual_dest == canonical_path {
            info!(?path, ?dest, "Symlink is already OK");
            return Ok(());
        } else {
            info!(
                ?path, actual = ?actual_dest, expected = ?dest,
                "Symlink has incorrect target"
            );
            std::fs::remove_file(&path).wrap_err("Failed to remove path")?;
        }
    } else if path.as_ref().exists() {
        let backup_path = get_backup_path(&path)
            .wrap_err_with(|| format!("Failed to get backup path for {:?}", &path))?;
        let span = span!(
            Level::INFO,
            "Renaming to avoid name collision",
            from = ?&path,
            to = ?&backup_path
        );
        let _guard = span.enter();
        std::fs::rename(&path, &backup_path)
            .wrap_err_with(|| format!("Failed to rename {:?} to {:?}", &path, &backup_path))?;
    }

    std::os::unix::fs::symlink(&path, &dest)
        .wrap_err_with(|| format!("Failed to symlink {:?} to {:?}", &path, &dest))
}

/// Renames a path to avoid a name collision.
#[instrument]
pub fn rename_to_avoid_collision(path: impl AsRef<Path> + Debug) -> eyre::Result<()> {
    Ok(())
}

fn get_backup_path(path: impl AsRef<Path> + Debug) -> eyre::Result<PathBuf> {
    let basename = path
        .as_ref()
        .file_name()
        .wrap_err_with(|| format!("Failed to get basename of {:?}", &path))?;
    let append = format!("{}", Local::now().format(FILENAME_DATETIME_FMT));
    let new_basename = {
        let mut ret = OsString::new();
        ret.push(basename);
        ret.push("-");
        ret.push(append);
        ret
    };

    let mut new_path = path.as_ref().with_file_name(&new_basename);
    let mut i = 0;
    while new_path.exists() {
        i += 1;
        let new_basename = {
            let mut ret = OsString::with_capacity(new_basename.len() + 2);
            ret.push(format!("-{}", i));
            ret
        };
        new_path = path.as_ref().with_file_name(new_basename);
    }

    Ok(new_path)
}
