use std::env;
use std::fs::File;
use std::io::Read;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let path = if args.len() > 1 {
        &args[1]
    } else {
        "./nyx_cli_data/wal.log"
    };

    println!("Inspecting WAL file: {}", path);
    let mut file = File::open(path)?;
    let mut offset = 0;

    loop {
        let mut len_buf = [0u8; 4];
        if let Err(_) = file.read_exact(&mut len_buf) {
            break; // EOF
        }
        let total_len = u32::from_le_bytes(len_buf) as usize;

        let mut data = vec![0u8; total_len];
        file.read_exact(&mut data)?;

        print!("Offset {}: Record [Len: {}] -> ", offset, total_len);

        // Parse the inner record
        let mut cursor = 0;
        if cursor >= data.len() {
            continue;
        }

        let opcode = data[cursor];
        cursor += 1;

        match opcode {
            1 => {
                // PUT
                if cursor + 4 > data.len() {
                    println!("Corrupt(KeyLen)");
                    continue;
                }
                let key_len =
                    u32::from_le_bytes(data[cursor..cursor + 4].try_into().unwrap()) as usize;
                cursor += 4;

                if cursor + key_len > data.len() {
                    println!("Corrupt(Key)");
                    continue;
                }
                let key = &data[cursor..cursor + key_len];
                cursor += key_len;

                if cursor + 4 > data.len() {
                    println!("Corrupt(ValLen)");
                    continue;
                }
                let val_len =
                    u32::from_le_bytes(data[cursor..cursor + 4].try_into().unwrap()) as usize;
                cursor += 4;

                if cursor + val_len > data.len() {
                    println!("Corrupt(Value)");
                    continue;
                }
                let value = &data[cursor..cursor + val_len];

                println!(
                    "PUT Key='{}' Value='{}'",
                    String::from_utf8_lossy(key),
                    String::from_utf8_lossy(value)
                );
            }
            2 => {
                // DELETE
                if cursor + 4 > data.len() {
                    println!("Corrupt(KeyLen)");
                    continue;
                }
                let key_len =
                    u32::from_le_bytes(data[cursor..cursor + 4].try_into().unwrap()) as usize;
                cursor += 4;

                if cursor + key_len > data.len() {
                    println!("Corrupt(Key)");
                    continue;
                }
                let key = &data[cursor..cursor + key_len];

                println!("DELETE Key='{}'", String::from_utf8_lossy(key));
            }
            _ => println!("UNKNOWN Opcode {}", opcode),
        }

        offset += 4 + total_len; // 4 bytes for length prefix + data
    }

    Ok(())
}
