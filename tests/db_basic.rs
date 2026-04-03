use nyxdb::db::db::NyxDB;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn test_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("nyxdb-{name}-{unique}"))
}

fn clean_dir(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

#[test]
fn nyxdb_persists_data_across_restarts() {
    let path = test_dir("restart");
    clean_dir(&path);

    {
        let mut db = NyxDB::open(&path).unwrap();
        db.put(b"key1".to_vec(), b"value1".to_vec()).unwrap();
        db.put(b"key2".to_vec(), b"value2".to_vec()).unwrap();
        db.delete(b"key1".to_vec()).unwrap();
    }

    {
        let db = NyxDB::open(&path).unwrap();
        assert!(db.get(b"key1").is_none());
        assert_eq!(db.get(b"key2").unwrap(), b"value2");
    }

    clean_dir(&path);
}

#[test]
fn delete_in_memtable_hides_older_sstable_value() {
    let path = test_dir("delete-tombstone");
    clean_dir(&path);

    let mut db = NyxDB::open(&path).unwrap();
    db.put(b"target".to_vec(), b"before-delete".to_vec())
        .unwrap();
    for i in 0..999 {
        db.put(
            format!("filler-{i}").into_bytes(),
            format!("value-{i}").into_bytes(),
        )
        .unwrap();
    }

    db.delete(b"target".to_vec()).unwrap();
    assert!(db.get(b"target").is_none());

    clean_dir(&path);
}

#[test]
fn flush_rotates_wal_and_keeps_data_available_after_restart() {
    let path = test_dir("flush-wal");
    clean_dir(&path);

    {
        let mut db = NyxDB::open(&path).unwrap();
        for i in 0..1000 {
            db.put(
                format!("key-{i}").into_bytes(),
                format!("value-{i}").into_bytes(),
            )
            .unwrap();
        }
    }

    let wal_len = fs::metadata(path.join("wal.log")).unwrap().len();
    assert_eq!(wal_len, 0, "WAL should be truncated after flush");

    {
        let db = NyxDB::open(&path).unwrap();
        assert_eq!(db.get(b"key-0").unwrap(), b"value-0");
        assert_eq!(db.get(b"key-999").unwrap(), b"value-999");
    }

    clean_dir(&path);
}

#[test]
fn compact_flushes_live_memtable_state_before_rewriting_sstables() {
    let path = test_dir("compact-live-state");
    clean_dir(&path);

    let mut db = NyxDB::open(&path).unwrap();
    for i in 0..1000 {
        db.put(
            format!("base-{i}").into_bytes(),
            format!("value-{i}").into_bytes(),
        )
        .unwrap();
    }

    db.put(b"still-in-memtable".to_vec(), b"present".to_vec())
        .unwrap();
    db.compact().unwrap();

    let db = NyxDB::open(&path).unwrap();
    assert_eq!(db.get(b"base-0").unwrap(), b"value-0");
    assert_eq!(db.get(b"still-in-memtable").unwrap(), b"present");

    clean_dir(&path);
}

#[test]
fn recovery_ignores_truncated_wal_tail() {
    let path = test_dir("truncated-wal");
    clean_dir(&path);
    fs::create_dir_all(&path).unwrap();

    let wal_path = path.join("wal.log");
    let mut wal = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&wal_path)
        .unwrap();

    let mut record = Vec::new();
    record.push(1);
    record.extend_from_slice(&(3u32).to_le_bytes());
    record.extend_from_slice(b"key");
    record.extend_from_slice(&(5u32).to_le_bytes());
    record.extend_from_slice(b"value");

    wal.write_all(&(record.len() as u32).to_le_bytes()).unwrap();
    wal.write_all(&record).unwrap();
    wal.write_all(&(10u32).to_le_bytes()).unwrap();
    wal.write_all(b"bad").unwrap();
    wal.flush().unwrap();
    File::open(&wal_path).unwrap().sync_all().unwrap();

    let db = NyxDB::open(&path).unwrap();
    assert_eq!(db.get(b"key").unwrap(), b"value");

    clean_dir(&path);
}
