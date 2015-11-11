extern crate atpp;

use atpp::{AtppHandle, AtppStartPackage, AtppDataPackage, AtppEndPackage, AtppAdapter, AtppError};

use std::thread;
use std::io::prelude::{Read, Write};
use std::net::{TcpListener, TcpStream, Shutdown};

struct TestRecvHandle;

impl TestRecvHandle {
    fn new() -> TestRecvHandle {
        TestRecvHandle
    }
}

impl AtppHandle<TcpStream> for TestRecvHandle {

    fn OnStart(&self, stream: &mut TcpStream, pkg: AtppStartPackage) {
        println!("{:?}", pkg);
    }

    fn OnData(&self, stream: &mut TcpStream, pkg: AtppDataPackage, data: &mut Vec<u8>) {
        println!("{:?}", pkg);
    }

    fn OnEnd(&self, stream: &mut TcpStream, pkg: AtppEndPackage) {
        println!("{:?}", pkg);
    }
}

fn main() {
    let sock = match TcpListener::bind("127.0.0.1:35589") {
        Ok(e) => e,
        Err(_) => panic!("Can't bind address! host 127.0.0.1, port 35589"),
    };

    println!("Server running on the 127.0.0.1:35589");

    for stream in sock.incoming() {
        match stream {
            Err(e) => {
                println!("Accept client has fail! {}", e);
            },
            Ok(stream) => {
                println!("Got connection!");
                thread::spawn(move || {
                    let mut handle: TestRecvHandle = TestRecvHandle::new();
                    handle_client(stream,&mut handle)
                });
            }
        }
    }
}

fn handle_client(stream: TcpStream, handle: &mut AtppHandle<TcpStream>) {
    let mut stream = stream;
    let mut last_buf: Vec<u8> = Vec::new();

    let adapter: AtppAdapter<TcpStream> = AtppAdapter::new(handle);

    loop {
        let mut buf = Vec::new();

        let mut raw_buf: [u8; 4096] = [0u8; 4096];
        let mut size = match stream.read(&mut raw_buf) {
            Ok(e) => e,
            Err(_) => {
                println!("Receive wrong shutdown socket!");
                break;
            },
        };
        if !last_buf.is_empty() {
            buf.write(&*last_buf);
            last_buf.clear();
        }
        buf.write(&raw_buf[..size]);
        match adapter.unpack(&mut stream, &mut buf) {
            Ok(e) => {
                match e {
                    Some(e) => {
                        last_buf.write(&*e);
                    },
                    None => {},
                }
            },
            Err(e) => {
                match e {
                    AtppError::BROKE_DATA(s) => {
                        println!("{:?}", s);
                    },
                    _ => {
                        println!("Other Error");
                    }
                }
            }
        };
    }
}
