use helixite::{Config, Helixite};
use tempfile::tempdir;

#[test]
fn test_open_new_db() {
    let dir = tempdir().unwrap();
    let db = Helixite::open(dir.path(), None).unwrap();
    assert!(db.path().exists());
}

#[test]
fn test_reopen_db() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    let db1 = Helixite::open(path, None).unwrap();
    drop(db1);

    let db2 = Helixite::open(path, None).unwrap();
    assert!(db2.path().exists());
}

#[test]
fn test_open_with_config() {
    let dir = tempdir().unwrap();
    let config = Config::default().with_map_size(64 * 1024 * 1024);
    let db = Helixite::open(dir.path(), Some(config)).unwrap();
    assert!(db.path().exists());
}
