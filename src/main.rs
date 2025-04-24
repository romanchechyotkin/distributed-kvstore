use core::str;
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

const ADDR: &str = "127.0.0.1:6379";

struct Processor {
    store: Store,
}

impl Processor {
    fn new() -> Processor {
        Processor {
            store: Store::new(),
        }
    }

    fn process_connection(&mut self, mut conn: TcpStream) {
        let mut request_buffer: [u8; 512] = [0; 512];

        loop {
            let _ = match conn.read(&mut request_buffer) {
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
                    let _ = conn.write(pong.as_bytes()).unwrap();
                }
                "set" => {
                    println!("{:?}", elems);
                    if elems.len() < 3 {
                        conn.write("SET <key> <value>\n".as_bytes()).unwrap();
                        continue;
                    }

                    let key = elems[1];
                    let value = elems[2];

                    self.store.set(key, value);
                }
                "get" => {
                    println!("{:?}", elems);
                    if elems.len() < 2 {
                        conn.write("GET <key>\n".as_bytes()).unwrap();
                        continue;
                    }
                    let key = elems[1];

                    match self.store.get(key) {
                        Some(value) => {
                            conn.write(format!("{value}\n").as_bytes()).unwrap();
                        }
                        None => {
                            conn.write("no such key\n".as_bytes()).unwrap();
                        }
                    }
                }
                _ => {
                    conn.write("not implemented yet\n".as_bytes()).unwrap();
                }
            };

            request_buffer = [0; 512];
        }
    }
}

struct Store {
    store: HashMap<String, String>,
}

impl Store {
    fn new() -> Store {
        Store {
            store: HashMap::new(),
        }
    }

    fn set(&mut self, key: &str, value: &str) -> Option<String> {
        self.store.insert(key.to_string(), value.to_string())
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.store.get(key).map(|s| s.as_str())
    }
}

fn main() {
    let listener = TcpListener::bind(ADDR).unwrap();
    let mut processor = Processor::new();

    println!("tcp listener on {ADDR}");

    for stream in listener.incoming() {
        let conn = stream.unwrap();
        println!("got connection {}", conn.peer_addr().unwrap());
        processor.process_connection(conn);
    }
}
