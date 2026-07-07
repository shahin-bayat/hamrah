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

pub struct Diff {
    pub request: HashMap<String, String>,
    pub conflicts: Vec<String>,
}

pub fn diff(mine: &HashMap<String, String>, theirs: &HashMap<String, String>) -> Diff {
    let mut request: HashMap<String, String> = HashMap::new();
    let mut conflicts = Vec::new();

    for (path, hash) in theirs {
        match mine.get(path) {
            Some(h) => {
                if hash != h {
                    conflicts.push(path.to_string());
                }
            }
            None => {
                request.insert(path.to_string(), hash.to_string());
            }
        }
    }

    Diff { request, conflicts }
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

    #[test]
    fn diff_classifies_paths() {
        let mine = HashMap::from([
            ("a".to_string(), "h1".to_string()),
            ("shared".to_string(), "h2".to_string()),
            ("conf".to_string(), "hA".to_string()),
        ]);
        let theirs = HashMap::from([
            ("b".to_string(), "h3".to_string()),
            ("shared".to_string(), "h2".to_string()),
            ("conf".to_string(), "hB".to_string()),
        ]);

        let d = diff(&mine, &theirs);

        assert_eq!(d.request.get("b"), Some(&"h3".to_string()));
        assert_eq!(d.request.len(), 1);
        assert_eq!(d.conflicts, vec!["conf".to_string()]);
    }
}
