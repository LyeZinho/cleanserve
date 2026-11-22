use super::{Result, UpdateCheckerError};
use std::path::{Path, PathBuf};

pub struct UpdateInstaller;

impl UpdateInstaller {
    pub fn backup_current_binary(binary_path: &Path, backup_dir: Option<&Path>) -> Result<PathBuf> {
        let actual_backup_dir = match backup_dir {
            Some(dir) => dir.to_path_buf(),
            None => dirs::home_dir()
                .ok_or_else(|| UpdateCheckerError {
                    message: "Cannot determine home directory".to_string(),
                })?
                .join(".cleanserve")
                .join("backups"),
        };
        
        std::fs::create_dir_all(&actual_backup_dir).map_err(|e| UpdateCheckerError {
            message: format!("Failed to create backup dir: {}", e),
        })?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| UpdateCheckerError {
                message: format!("Time error: {}", e),
            })?
            .as_secs();

        let backup_path = actual_backup_dir.join(format!("cleanserve.{}", timestamp));
        
        if binary_path.exists() {
            std::fs::copy(binary_path, &backup_path).map_err(|e| UpdateCheckerError {
                message: format!("Failed to backup current binary: {}", e),
            })?;
        }
        
        Ok(backup_path)
    }

    pub async fn install_binary(new_binary: &Path, target_location: &Path) -> Result<()> {
        Self::install_file(new_binary, target_location)
    }

    /// Copy `source` next to `target`, mark it executable and atomically move it into place.
    fn install_file(source: &Path, target: &Path) -> Result<()> {
        let parent = target.parent().ok_or_else(|| UpdateCheckerError {
            message: "Invalid target location".to_string(),
        })?;

        std::fs::create_dir_all(parent).map_err(|e| UpdateCheckerError {
            message: format!("Failed to create target dir: {}", e),
        })?;

        let temp_target = parent.join(format!(".cleanserve.update.{}", std::process::id()));

        std::fs::copy(source, &temp_target).map_err(|e| UpdateCheckerError {
            message: format!("Failed to copy new binary: {}", e),
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Err(e) =
                std::fs::set_permissions(&temp_target, std::fs::Permissions::from_mode(0o755))
            {
                let _ = std::fs::remove_file(&temp_target);
                return Err(UpdateCheckerError {
                    message: format!("Failed to set permissions: {}", e),
                });
            }
        }

        // Windows cannot overwrite a running executable, but it can rename it out of the way.
        #[cfg(windows)]
        {
            let old = target.with_extension("old");
            let _ = std::fs::remove_file(&old);
            if target.exists() {
                let _ = std::fs::rename(target, &old);
            }
        }

        std::fs::rename(&temp_target, target).map_err(|e| {
            let _ = std::fs::remove_file(&temp_target);
            UpdateCheckerError {
                message: format!("Failed to atomically replace binary: {}", e),
            }
        })?;

        Ok(())
    }

    /// Run `<binary> --version`; the new binary must start and exit successfully.
    pub async fn verify_installation(binary_path: &Path) -> Result<()> {
        let output = tokio::process::Command::new(binary_path)
            .arg("--version")
            .output()
            .await
            .map_err(|e| UpdateCheckerError {
                message: format!("Failed to execute new binary: {}", e),
            })?;

        if !output.status.success() {
            return Err(UpdateCheckerError {
                message: format!("New binary failed execution check: {:?}", output.status),
            });
        }

        Ok(())
    }

    /// Restore `backup_path` over `target_location`.
    ///
    /// The backup lives in `~/.cleanserve/backups`, usually another filesystem than the
    /// installed binary, so a plain `rename` would fail with EXDEV. Copy next to the target
    /// and rename into place instead (atomic, and the backup is kept).
    pub fn rollback(backup_path: &Path, target_location: &Path) -> Result<()> {
        if !backup_path.exists() {
            return Ok(());
        }
        Self::install_file(backup_path, target_location).map_err(|mut e| {
            e.message = format!("Failed to rollback binary: {}", e.message);
            e
        })
    }

    pub fn cleanup_old_backups(backup_dir: Option<&Path>) -> Result<()> {
        let actual_backup_dir = match backup_dir {
            Some(dir) => dir.to_path_buf(),
            None => dirs::home_dir()
                .ok_or_else(|| UpdateCheckerError {
                    message: "Cannot determine home directory".to_string(),
                })?
                .join(".cleanserve")
                .join("backups"),
        };

        if !actual_backup_dir.exists() {
            return Ok(());
        }

        let mut backups: Vec<_> = std::fs::read_dir(&actual_backup_dir)
            .map_err(|e| UpdateCheckerError { message: e.to_string() })?
            .filter_map(std::result::Result::ok)
            .filter(|entry| {
                entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) &&
                entry.file_name().to_string_lossy().starts_with("cleanserve.")
            })
            .collect();

        backups.sort_by_key(|a| a.metadata().and_then(|m| m.modified()).ok());

        if backups.len() > 3 {
            for entry in backups.iter().take(backups.len() - 3) {
                let _ = std::fs::remove_file(entry.path());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_backup_and_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        let bin_path = temp_dir.path().join("cleanserve");
        std::fs::write(&bin_path, "dummy").unwrap();

        std::fs::create_dir_all(&backup_dir).unwrap();

        for i in 1..=5 {
            let backup_file = backup_dir.join(format!("cleanserve.100{}", i));
            std::fs::write(&backup_file, "backup").unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let count = std::fs::read_dir(&backup_dir).unwrap().count();
        assert_eq!(count, 5);

        UpdateInstaller::cleanup_old_backups(Some(&backup_dir)).unwrap();
        
        let count_after = std::fs::read_dir(&backup_dir).unwrap().count();
        assert_eq!(count_after, 3);
    }

    #[tokio::test]
    async fn test_install_and_rollback() {
        let temp_dir = TempDir::new().unwrap();
        let new_bin = temp_dir.path().join("new_bin");
        let target = temp_dir.path().join("target_bin");
        let backup = temp_dir.path().join("backup_bin");
        
        std::fs::write(&new_bin, "new").unwrap();
        std::fs::write(&backup, "old").unwrap();

        let result = UpdateInstaller::install_binary(&new_bin, &target).await;
        assert!(result.is_ok());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new");

        let rollback_res = UpdateInstaller::rollback(&backup, &target);
        assert!(rollback_res.is_ok());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "old");
    }

    #[tokio::test]
    async fn test_verify_installation_fails_for_missing_binary() {
        let temp_dir = TempDir::new().unwrap();
        let res = UpdateInstaller::verify_installation(&temp_dir.path().join("nope")).await;
        assert!(res.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_verify_installation_runs_binary() {
        use std::os::unix::fs::PermissionsExt;
        let temp_dir = TempDir::new().unwrap();

        let ok = temp_dir.path().join("ok");
        std::fs::write(&ok, "#!/bin/sh\necho cleanserve 9.9.9\n").unwrap();
        std::fs::set_permissions(&ok, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(UpdateInstaller::verify_installation(&ok).await.is_ok());

        let bad = temp_dir.path().join("bad");
        std::fs::write(&bad, "#!/bin/sh\nexit 3\n").unwrap();
        std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(UpdateInstaller::verify_installation(&bad).await.is_err());
    }
}
