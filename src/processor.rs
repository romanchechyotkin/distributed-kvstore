use crate::store::Store;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub struct Processor {
    store: Store,
}

impl Processor {
    pub fn new() -> Processor {
        Processor {
            store: Store::new(),
        }
    }

    pub async fn process_connection(&mut self, mut conn: TcpStream) {
        let mut request_buffer: [u8; 512] = [0; 512];

        loop {
            let _ = match conn.read(&mut request_buffer).await {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    println!("{e}");
                    break;
                }
            };

            let req = String::from_utf8_lossy(&request_buffer)
                .trim_end_matches(char::from(0))
                .to_string();

            println!("got request {req}");

            let elems: Vec<&str> = req.split_whitespace().collect();
            let cmd = elems[0];

            match cmd.to_lowercase().as_str() {
                "ping" => {
                    let pong = "PONG\n";
                    let _ = conn.write(pong.as_bytes()).await.unwrap();
                }
                "set" => {
                    println!("{:?}", elems);
                    if elems.len() < 3 {
                        conn.write("SET <key> <value>\n".as_bytes()).await.unwrap();
                        continue;
                    }

                    let key = elems[1];
                    let value = elems[2];

                    self.store.set(key, value);
                }
                "get" => {
                    println!("{:?}", elems);
                    if elems.len() < 2 {
                        conn.write("GET <key>\n".as_bytes()).await.unwrap();
                        continue;
                    }
                    let key = elems[1];

                    match self.store.get(key) {
                        Some(value) => {
                            conn.write(format!("{value}\n").as_bytes()).await.unwrap();
                        }
                        None => {
                            conn.write("no such key\n".as_bytes()).await.unwrap();
                        }
                    }
                }
                _ => {
                    conn.write("not implemented yet\n".as_bytes())
                        .await
                        .unwrap();
                }
            };

            request_buffer = [0; 512];
        }
    }
}
