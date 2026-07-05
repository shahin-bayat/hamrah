use std::{collections::HashMap, fs, io, path::Path};

use walkdir::WalkDir;

use crate::store::Store;

pub fn build(root: &Path, store: &Store) -> io::Result<HashMap<String, String>> {
    let mut manifest = HashMap::new();

    for entry in WalkDir::new(root) {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() || store.is_internal(path) {
            continue;
        }

        let bytes = fs::read(path)?;
        let h = store.write(&bytes)?;

        let rel = path.strip_prefix(root).unwrap();
        manifest.insert(rel.to_string_lossy().into_owned(), h);
    }
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn builds_manifest_of_tree() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        fs::write(root.join("a.txt"), b"hello").unwrap();
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("sub/b.txt"), b"world").unwrap();

        let store_dir = tempfile::tempdir().unwrap();
        let store = Store::new(store_dir.path().to_path_buf()).unwrap();

        let manifest = build(root, &store).unwrap();

        assert_eq!(manifest.len(), 2);
        assert_eq!(manifest["a.txt"], crate::store::hash(b"hello"));
        assert_eq!(manifest["sub/b.txt"], crate::store::hash(b"world"));
    }

    #[test]
    fn excludes_internal_store() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        fs::write(root.join("a.txt"), b"hello").unwrap();

        // store placed inside the synced root
        let store = Store::new(root.join(".hamrah")).unwrap();

        let manifest = build(root, &store).unwrap();

        assert_eq!(manifest.len(), 1); // only a.txt — NOT the store's own blobs
        assert!(manifest.contains_key("a.txt"));
    }
}
