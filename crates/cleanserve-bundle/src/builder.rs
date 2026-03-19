use crate::config::{BundleConfig, CompressionType};
use crate::phar::{create_phar_from_directory, PharError};
use cleanserve_inliner::{EnvParser, PhpGenerator};
use cleanserve_preload::PreloadGenerator;
use cleanserve_vfs::MemoryBackend;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Error, Debug)]
pub enum BuilderError {
    #[error("PHAR error: {0}")]
    Phar(#[from] PharError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Preload error: {0}")]
    Preload(String),
    #[error("Inliner error: {0}")]
    Inliner(String),
    #[error("Build error: {0}")]
    Build(String),
}

pub struct BundleBuilder {
    project_root: PathBuf,
    output_dir: PathBuf,
    config: BundleConfig,
    memory_backend: MemoryBackend,
}

impl BundleBuilder {
    pub fn new(project_root: impl Into<PathBuf>, output_dir: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            output_dir: output_dir.into(),
            config: BundleConfig::default(),
            memory_backend: MemoryBackend::new(),
        }
    }

    pub fn config(mut self, config: BundleConfig) -> Self {
        self.config = config;
        self
    }

    pub fn collect_files(&mut self) -> Result<&mut Self, BuilderError> {
        let exclude_patterns = &self.config.exclude_patterns;

        for entry in WalkDir::new(&self.project_root)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            let relative = path.strip_prefix(&self.project_root).unwrap_or(path);
            let relative_str = relative.to_string_lossy().replace('\\', "/");

            if exclude_patterns.iter().any(|p| {
                if p.ends_with('/') {
                    relative_str.starts_with(p.trim_end_matches('/'))
                } else {
                    relative_str == *p || relative_str.ends_with(p)
                }
            }) {
                continue;
            }

            if path.is_file() {
                let content = std::fs::read(path)?;
                self.memory_backend.insert(&relative_str, content);
            }
        }

        Ok(self)
    }

    pub fn generate_preload(&mut self) -> Result<&mut Self, BuilderError> {
        let generator = PreloadGenerator::from_project(&self.project_root)
            .map_err(|e| BuilderError::Preload(e.to_string()))?;

        let script = generator
            .generate_script()
            .map_err(|e| BuilderError::Preload(e.to_string()))?;

        self.memory_backend
            .insert("preload.php", script.into_bytes());

        Ok(self)
    }

    pub fn inline_env(&mut self) -> Result<&mut Self, BuilderError> {
        let env_path = self.project_root.join(".env");

        if !env_path.exists() {
            return Ok(self);
        }

        let parser =
            EnvParser::from_file(&env_path).map_err(|e| BuilderError::Inliner(e.to_string()))?;

        let generator = PhpGenerator::new(parser);
        let php_content = generator.generate();

        self.memory_backend
            .insert("bootstrap/env.php", php_content.into_bytes());

        Ok(self)
    }

    pub fn create_manifest(&self) -> String {
        let files: Vec<_> = self.memory_backend.files();
        let total_size = self.memory_backend.total_size();
        let checksums: Vec<_> = files
            .iter()
            .filter_map(|path| {
                self.memory_backend.get(path).map(|content| {
                    let hash = Sha256::digest(&content);
                    format!("  {} {}", hex::encode(&hash[..8]), path.to_string_lossy())
                })
            })
            .collect();

        let manifest = format!(
            r#"CleanServe Bundle Manifest
========================

Name: {}
Version: {}
PHP Version: {}
Files: {}
Total Size: {:.2} MB

Files:
{}

Generated: {}
"#,
            self.config.name,
            self.config.version,
            self.config.php_version,
            files.len(),
            total_size as f64 / 1024.0 / 1024.0,
            checksums.join("\n"),
            chrono::Utc::now().to_rfc3339()
        );

        manifest
    }

    pub fn build(&mut self) -> Result<PathBuf, BuilderError> {
        std::fs::create_dir_all(&self.output_dir)?;

        let phar_path = self.output_dir.join("app.phar");
        create_phar_from_directory(
            &self.project_root,
            &phar_path,
            &self.config.exclude_patterns,
            &self.config.entry_point,
        )?;

        let manifest = self.create_manifest();
        std::fs::write(self.output_dir.join("MANIFEST.txt"), manifest)?;

        Ok(phar_path)
    }

    pub fn build_with_vfs(&mut self) -> Result<PathBuf, BuilderError> {
        std::fs::create_dir_all(&self.output_dir)?;

        let manifest = self.create_manifest();
        std::fs::write(self.output_dir.join("MANIFEST.txt"), manifest)?;

        let manifest_path = self.output_dir.join("app.phar");
        Ok(manifest_path)
    }

    pub fn memory_backend(&self) -> &MemoryBackend {
        &self.memory_backend
    }
}

impl Default for BundleBuilder {
    fn default() -> Self {
        Self::new(".", "dist")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_manifest_generation() {
        let dir = TempDir::new().unwrap();
        let backend = MemoryBackend::new();

        let mut builder = BundleBuilder::new(dir.path(), dir.path());
        builder
            .memory_backend
            .insert("index.php", b"<?php".to_vec());

        let manifest = builder.create_manifest();
        assert!(manifest.contains("CleanServe Bundle Manifest"));
        assert!(manifest.contains("Files:"));
    }
}
