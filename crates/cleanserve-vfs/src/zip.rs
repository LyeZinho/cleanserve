use crate::traits::{FileSystem, VfsError, VfsMetadata};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use zip::ZipArchive;

pub struct ZipBackend {
    archive: RwLock<ZipArchive<Cursor<Vec<u8>>>>,
}

impl ZipBackend {
    pub fn from_bytes(data: Vec<u8>) -> Result<Self, VfsError> {
        let cursor = Cursor::new(data);
        let archive = ZipArchive::new(cursor)
            .map_err(|e| VfsError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        Ok(Self {
            archive: RwLock::new(archive),
        })
    }

    pub fn from_path(path: &Path) -> Result<Self, VfsError> {
        let data = std::fs::read(path)?;
        Self::from_bytes(data)
    }

    fn list_files_internal(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        let mut guard = self.archive.write().unwrap();
        for i in 0..guard.len() {
            if let Ok(file) = guard.by_index_raw(i) {
                let name = file.name().to_string();
                if !name.ends_with('/') {
                    files.push(PathBuf::from(name));
                }
            }
        }
        files
    }
}

impl FileSystem for ZipBackend {
    fn read(&self, path: &Path) -> Result<Vec<u8>, VfsError> {
        let path_str = path.to_string_lossy().replace('\\', "/");

        let mut archive = self.archive.write().unwrap();
        let mut file = archive
            .by_name(&path_str)
            .map_err(|_| VfsError::NotFound(path_str.clone()))?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).map_err(VfsError::Io)?;
        Ok(buffer)
    }

    fn exists(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().replace('\\', "/");
        let mut guard = self.archive.write().unwrap();
        let result = guard.by_name(&path_str).is_ok();
        result
    }

    fn metadata(&self, path: &Path) -> Result<VfsMetadata, VfsError> {
        let path_str = path.to_string_lossy().replace('\\', "/");

        let mut archive = self.archive.write().unwrap();
        let file = archive
            .by_name(&path_str)
            .map_err(|_| VfsError::NotFound(path_str.clone()))?;

        Ok(VfsMetadata {
            size: file.size(),
            is_dir: false,
            is_file: true,
            modified: None,
        })
    }

    fn list(&self, _path: &Path) -> Result<Vec<PathBuf>, VfsError> {
        Ok(self.list_files_internal())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn create_test_zip() -> Vec<u8> {
        let mut buffer = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut buffer));
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

            zip.start_file("test.php", options).unwrap();
            zip.write_all(b"<?php echo 'test';").unwrap();

            zip.start_file("config.json", options).unwrap();
            zip.write_all(b"{\"key\": \"value\"}").unwrap();

            zip.finish().unwrap();
        }
        buffer
    }

    #[test]
    fn test_read_from_zip() {
        let data = create_test_zip();
        let backend = ZipBackend::from_bytes(data).unwrap();

        let content = backend.read(Path::new("test.php")).unwrap();
        assert_eq!(content, b"<?php echo 'test';");
    }

    #[test]
    fn test_exists_in_zip() {
        let data = create_test_zip();
        let backend = ZipBackend::from_bytes(data).unwrap();

        assert!(backend.exists(Path::new("test.php")));
        assert!(!backend.exists(Path::new("missing.php")));
    }

    #[test]
    fn test_list_files() {
        let data = create_test_zip();
        let backend = ZipBackend::from_bytes(data).unwrap();

        let files = backend.list(Path::new("")).unwrap();
        assert_eq!(files.len(), 2);
    }
}
