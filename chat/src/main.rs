//! A small two-person TCP chat application.
//!
//! The same executable can be started in either server or client mode:
//!
//!     cargo run -- server
//!     cargo run -- client 127.0.0.1
//!
//! Chat messages are UTF-8 text terminated by a newline. TCP gives us a
//! reliable stream of bytes, while the newline is the simple message boundary
//! used by this program.

use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::process;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

const DEFAULT_PORT: u16 = 64000;

/// Events sent to the chat loop by the keyboard and network threads.
enum ChatEvent {
    KeyboardInput(String),
    PeerMessage(String),
    PeerDisconnected,
    KeyboardClosed,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage(&args[0]);
        return;
    }

    // The mode determines whether this process waits for a connection or
    // actively connects to another process.
    match args[1].as_str() {
        "server" => run_server(args.get(2).map(String::as_str)),
        "client" => run_client(&args),
        "help" | "--help" | "-h" => print_usage(&args[0]),
        mode => {
            eprintln!("Unknown mode: {mode}");
            print_usage(&args[0]);
            process::exit(1);
        }
    }
}

/// Start the listening side of the chat.
///
/// The keyboard reader is created once and reused. This lets the server return
/// to `accept` after a client leaves, while keeping the same server terminal
/// available for the next client.
fn run_server(bind_address: Option<&str>) {
    let address = bind_address
        .map(str::to_owned)
        .unwrap_or_else(|| format!("0.0.0.0:{DEFAULT_PORT}"));

    let listener = TcpListener::bind(&address).unwrap_or_else(|error| {
        eprintln!("Could not listen on {address}: {error}");
        process::exit(1);
    });

    let (input_sender, input_receiver) = mpsc::channel();
    spawn_keyboard_reader(input_sender);

    println!("Listening on {address}");

    // Keep accepting clients until the server's terminal reaches EOF or an
    // unrecoverable connection error occurs.
    loop {
        println!("Waiting for a chat client...");

        let (stream, peer) = listener.accept().unwrap_or_else(|error| {
            eprintln!("Could not accept a connection: {error}");
            process::exit(1);
        });

        println!("Connected to {peer}");
        run_chat(stream, &input_receiver);
        println!("Ready for another client.");
    }
}

/// Start the connecting side of the chat.
fn run_client(args: &[String]) {
    if args.len() < 3 || args.len() > 4 {
        eprintln!("Client mode needs a server address.");
        print_usage(&args[0]);
        process::exit(1);
    }

    let address = if args[2].contains(':') {
        args[2].clone()
    } else {
        let port = args.get(3).map(String::as_str).unwrap_or("64000");
        format!("{}:{port}", args[2])
    };

    println!("Connecting to {address}...");
    let stream = TcpStream::connect(&address).unwrap_or_else(|error| {
        eprintln!("Could not connect to {address}: {error}");
        process::exit(1);
    });

    println!("Connected to {address}");

    let (input_sender, input_receiver) = mpsc::channel();
    spawn_keyboard_reader(input_sender);
    run_chat(stream, &input_receiver);
}

/// Read keyboard input on its own thread and send each line to the chat loop.
///
/// The server keeps this thread alive between connections. A client process
/// exits after `/quit`, so its keyboard thread naturally ends with the process.
fn spawn_keyboard_reader(sender: Sender<ChatEvent>) {
    thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            match line {
                Ok(message) => {
                    if sender.send(ChatEvent::KeyboardInput(message)).is_err() {
                        break;
                    }
                }
                Err(error) => {
                    eprintln!("Could not read keyboard input: {error}");
                    let _ = sender.send(ChatEvent::KeyboardClosed);
                    break;
                }
            }
        }

        let _ = sender.send(ChatEvent::KeyboardClosed);
    });
}

/// Run the shared chat logic once a TCP connection exists.
///
/// The network stream is cloned so a reader thread can receive messages while
/// this function handles keyboard events and sends messages. When either side
/// quits, the stream is shut down and this function returns. In server mode,
/// that return sends control back to the loop that accepts the next client.
fn run_chat(stream: TcpStream, input_receiver: &Receiver<ChatEvent>) {
    let reader_stream = stream.try_clone().expect("Could not clone the TCP stream");
    let (event_sender, event_receiver) = mpsc::channel();

    // Forward network events into the same event loop as keyboard input.
    let network_sender = event_sender.clone();
    let reader_thread = thread::spawn(move || {
        let reader = BufReader::new(reader_stream);

        for message in reader.lines() {
            match message {
                Ok(message) => {
                    if network_sender
                        .send(ChatEvent::PeerMessage(message))
                        .is_err()
                    {
                        return;
                    }
                }
                Err(error) => {
                    eprintln!("\nCould not read from peer: {error}");
                    break;
                }
            }
        }

        let _ = network_sender.send(ChatEvent::PeerDisconnected);
    });

    println!("Type a message and press Enter. Type /quit to leave.");
    io::stdout().flush().expect("Could not flush stdout");

    let mut writer = stream;

    loop {
        // Check keyboard input first so `/quit` closes this terminal promptly.
        match input_receiver.try_recv() {
            Ok(event) => {
                if handle_keyboard_event(event, &mut writer) {
                    break;
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => break,
            Err(mpsc::TryRecvError::Empty) => {}
        }

        // Wait briefly for a network event. The timeout lets us continue
        // checking the keyboard channel without blocking forever.
        match event_receiver.recv_timeout(std::time::Duration::from_millis(50)) {
            Ok(ChatEvent::PeerMessage(message)) => println!("\nPeer: {message}"),
            Ok(ChatEvent::PeerDisconnected) => {
                println!("\nThe other program disconnected.");
                break;
            }
            Ok(_) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }

    // This unblocks the network reader thread so it can finish cleanly.
    let _ = writer.shutdown(Shutdown::Both);
    let _ = reader_thread.join();
    println!("Chat ended.");
}

/// Send keyboard messages and report whether the chat should end.
fn handle_keyboard_event(event: ChatEvent, writer: &mut TcpStream) -> bool {
    match event {
        ChatEvent::KeyboardInput(message) => {
            if message == "/quit" {
                return true;
            }

            if let Err(error) = writeln!(writer, "{message}") {
                eprintln!("Could not send message: {error}");
                return true;
            }

            if let Err(error) = writer.flush() {
                eprintln!("Could not flush message: {error}");
                return true;
            }

            io::stdout().flush().expect("Could not flush stdout");
            false
        }
        ChatEvent::KeyboardClosed => true,
        _ => false,
    }
}

fn print_usage(program: &str) {
    println!(
        "Usage:\n  {program} server [bind_address:port]\n  {program} client <server_ip_or_host> [port]\n\nExamples:\n  cargo run -- server\n  cargo run -- client 127.0.0.1\n  cargo run -- client 192.168.1.25 64000\n\nType /quit to end a chat."
    );
}
