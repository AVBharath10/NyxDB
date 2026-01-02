
use nyxdb::db::db::NyxDB;
use std::fs;

#[test]
fn nyxdb_persists_data_across_restarts() {
    let path = "test_nyxdb.log";

    let _ = fs::remove_file(path);

    {
        let mut db = NyxDB::open(path).unwrap();
        db.put(b"key1".to_vec(), b"value1".to_vec()).unwrap();
        db.put(b"key2".to_vec(), b"value2".to_vec()).unwrap();
        db.delete(b"key1".to_vec()).unwrap();
    }

    {
        let db = NyxDB::open(path).unwrap();
        assert!(db.get(b"key1").is_none());
        assert_eq!(db.get(b"key2").unwrap(), b"value2");
    }

    let _ = fs::remove_file(path);
}
