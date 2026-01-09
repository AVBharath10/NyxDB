use nyxdb::db::db::NyxDB;

fn main() {
    let mut db = NyxDB::open("./db_data").expect("open db");


    println!("Putting values...");
    db.put(b"name".to_vec(), b"nyxdb".to_vec()).unwrap();
    db.put(b"type".to_vec(), b"lsm".to_vec()).unwrap();

    println!("Reading values...");
    println!("name = {:?}", db.get(b"name"));
    println!("type = {:?}", db.get(b"type"));

    println!("Deleting name...");
    db.delete(b"name".to_vec()).unwrap();

    println!("After delete:");
    println!("name = {:?}", db.get(b"name"));
}
