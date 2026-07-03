use std::{collections::HashMap, fs, io, path::Path};

use crate::store::Store;

pub fn apply(manifest: &HashMap<String, String>, store: &Store, dest: &Path) -> io::Result<()> {
    for (rel_path, hash) in manifest {
        let path = dest.join(rel_path);
        let bytes = store.read(hash)?;

        // nested files:
        // dest: code , path: code/rust/devsync/src/main.rs
        // parent = code/rust/devsync/src
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, bytes)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    use crate::manifest;

    use super::*;

    #[test]
    fn apply_materializes_tree() {
        let src = tempfile::tempdir().unwrap();
        fs::write(src.path().join("a.txt"), b"Hello from a!").unwrap();
        fs::create_dir_all(src.path().join("sub")).unwrap();
        fs::write(src.path().join("sub/b.txt"), b"Hello from b!").unwrap();

        let store_dir = tempfile::tempdir().unwrap();
        let store = Store::new(store_dir.path().to_path_buf()).unwrap();
        let m = manifest::build(src.path(), &store).unwrap();

        let dest = tempfile::tempdir().unwrap();
        apply(&m, &store, dest.path()).unwrap();

        assert_eq!(
            fs::read(dest.path().join("a.txt")).unwrap(),
            b"Hello from a!"
        );
        assert_eq!(
            fs::read(dest.path().join("sub/b.txt")).unwrap(),
            b"Hello from b!"
        );
    }
}
