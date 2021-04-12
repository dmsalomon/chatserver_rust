#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::io::prelude::*;
use std::io::{BufReader, LineWriter};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Mutex;
use std::thread;

#[derive(Debug)]
struct Client {
    name: String,
    writer: LineWriter<TcpStream>,
}

#[derive(Debug)]
enum MessageKind {
    ENTER,
    RELAY,
    EXIT,
}

#[derive(Debug)]
struct Message {
    kind: MessageKind,
    buf: String,
    sender: String,
}

lazy_static! {
    static ref CLIENTS: Mutex<HashMap<String, Client>> = Mutex::new(HashMap::new());
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:3000")?;
    let (tx, rx) = sync_channel(1_000);

    thread::spawn(move || broadcast(rx));

    for stream in listener.incoming() {
        let tx = tx.clone();
        thread::spawn(move || serve(stream.unwrap(), tx));
    }

    Ok(())
}

fn broadcast(rx: Receiver<Message>) {
    while let Ok(msg) = rx.recv() {
        println!("sending message: {:?}", msg);
        match &msg.kind {
            MessageKind::ENTER => msg_enter(&msg),
            MessageKind::RELAY => msg_relay(&msg),
            MessageKind::EXIT => msg_exit(&msg),
        }
    }
}

fn msg_enter(msg: &Message) {
    let _first = true;

    let clients = &mut *CLIENTS.lock().unwrap();

    for (_, c) in clients.iter_mut() {
        if c.name == msg.sender {
            c.writer
                .write_fmt(format_args!("server\nWelcome {}!\n", msg.sender))
                .unwrap();
        } else {
            c.writer
                .write_fmt(format_args!("server\n{} has entered\n", msg.sender))
                .unwrap();
        }
        println!("name: {}, {:?}", c.name, c);
    }

    if clients.len() > 1 {
        let users = clients
            .keys()
            .filter(|s| **s != msg.sender)
            .map(|s| &**s)
            .collect::<Vec<_>>()
            .join(", ");

        clients
            .get_mut(&msg.sender)
            .unwrap()
            .writer
            .write_fmt(format_args!("server\nusers in chat: {}\n", users))
            .unwrap();
    } else {
        clients
            .get_mut(&msg.sender)
            .unwrap()
            .writer
            .write(b"server\nno users in chat\n")
            .unwrap();
    }
}

fn msg_relay(msg: &Message) {
    let clients = &mut *CLIENTS.lock().unwrap();
    for (_name, c) in clients {
        if c.name == msg.sender {
            continue;
        }
        c.writer
            .write_fmt(format_args!("{}\n{}\n", msg.sender, msg.buf))
            .unwrap();
        println!("name: {}, {:?}", c.name, c);
    }
}

fn msg_exit(msg: &Message) {
    let clients = &mut *CLIENTS.lock().unwrap();
    for (_name, c) in clients.iter_mut() {
        if c.name == msg.sender {
            continue;
        }
        c.writer
            .write_fmt(format_args!("server\n{} has left\n", msg.sender))
            .unwrap();
        println!("name: {}, {:?}", c.name, c);
    }
    clients.remove(&msg.sender);
}

fn serve(stream: TcpStream, tx: SyncSender<Message>) {
    println!(
        "received conn from {}",
        stream.peer_addr().expect("peed_addr unavailable")
    );

    let mut reader = BufReader::new(stream.try_clone().expect("clone failed..."));
    let mut name = String::new();

    reader.read_line(&mut name).unwrap();
    name.truncate(32);
    let name = String::from(name.trim_end());

    let mut c = Client {
        name: name.clone(),
        writer: LineWriter::new(stream.try_clone().expect("clone failed ...")),
    };

    if name == "server" || CLIENTS.lock().unwrap().contains_key(&name) {
        c.writer
            .write_fmt(format_args!("server\n{} is already taken\n", name))
            .unwrap();
        return;
    }

    CLIENTS.lock().unwrap().insert(name.clone(), c);

    tx.send(Message {
        kind: MessageKind::ENTER,
        buf: String::new(),
        sender: name.clone(),
    })
    .unwrap();

    for line in reader.lines().map(|line| line.unwrap()) {
        tx.send(Message {
            kind: MessageKind::RELAY,
            buf: line,
            sender: name.clone(),
        })
        .unwrap();
    }

    tx.send(Message {
        kind: MessageKind::EXIT,
        buf: String::new(),
        sender: name.clone(),
    })
    .unwrap();
}
