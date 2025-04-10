use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

const ADDR: &str = "127.0.0.1:6379";

fn main() {
    let listener = TcpListener::bind(ADDR).unwrap();
    println!("tcp listener on {ADDR}");

    for stream in listener.incoming() {
        let conn = stream.unwrap();
        println!("got connection {}", conn.peer_addr().unwrap());
        process_connection(conn);
    }
}

fn process_connection(mut conn: TcpStream) {
    let mut request_buffer: [u8; 512] = [0; 512];

    loop {
        let n = conn.read(&mut request_buffer).unwrap();
        if n == 0 {
            break;
        }

        let req = String::from_utf8_lossy(&request_buffer).to_string();

        println!("got request {req}");

        let elems: Vec<&str> = req.split_whitespace().collect();
        let cmd = elems[0];

        match cmd.to_lowercase().as_str() {
            "ping" => {
                let pong = "PONG\n";
                let _ = conn.write(pong.as_bytes()).unwrap();
            }
            _ => {
                conn.write("not implemented yet\n".as_bytes()).unwrap();
            }
        };

        request_buffer = [0; 512];
    }
}
