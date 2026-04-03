use nyxdb::db::db::NyxDB;
use std::io::{self, Write};

fn main() {
    let mut db = NyxDB::open("./nyx_cli_data").expect("Failed to open database");
    println!("NyxDB CLI v0.1.0");
    println!("Commands: put <key> <value>, get <key>, delete <key>, compact, exit");

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }

        let parts: Vec<&str> = input.trim().split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "put" => {
                if parts.len() < 3 {
                    println!("Usage: put <key> <value>");
                    continue;
                }
                let key = parts[1].as_bytes().to_vec();
                let value = parts[2..].join(" ").as_bytes().to_vec(); // Allow spaces in value
                match db.put(key, value) {
                    Ok(_) => println!("OK"),
                    Err(e) => println!("Error: {}", e),
                }
            }
            "get" => {
                if parts.len() != 2 {
                    println!("Usage: get <key>");
                    continue;
                }
                let key = parts[1].as_bytes();
                match db.get(key) {
                    Some(val) => println!("{}", String::from_utf8_lossy(&val)),
                    None => println!("(nil)"),
                }
            }
            "delete" => {
                if parts.len() != 2 {
                    println!("Usage: delete <key>");
                    continue;
                }
                let key = parts[1].as_bytes().to_vec();
                match db.delete(key) {
                    Ok(_) => println!("OK"),
                    Err(e) => println!("Error: {}", e),
                }
            }
            "compact" => match db.compact() {
                Ok(_) => println!("Compaction completed"),
                Err(e) => println!("Error during compaction: {}", e),
            },
            "exit" | "quit" => break,
            _ => println!("Unknown command. Try: put, get, delete, compact, exit"),
        }
    }
}
