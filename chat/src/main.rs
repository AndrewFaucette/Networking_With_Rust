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
use std::thread;

const DEFAULT_PORT: u16 = 64000;

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
/// By default, the server listens on every local network interface at port
/// 64000. A custom bind address can be supplied, such as `127.0.0.1:64000`.
fn run_server(bind_address: Option<&str>) {
    let address = bind_address
        .map(str::to_owned)
        .unwrap_or_else(|| format!("0.0.0.0:{DEFAULT_PORT}"));

    let listener = TcpListener::bind(&address).unwrap_or_else(|error| {
        eprintln!("Could not listen on {address}: {error}");
        process::exit(1);
    });

    println!("Listening on {address}");
    println!("Waiting for one chat client...");

    // `accept` returns the TCP stream and the remote address of the client.
    let (stream, peer) = listener.accept().unwrap_or_else(|error| {
        eprintln!("Could not accept a connection: {error}");
        process::exit(1);
    });

    println!("Connected to {peer}");
    run_chat(stream);
}

/// Start the connecting side of the chat.
///
/// The client accepts either an IP address/host name or a complete socket
/// address. For example, both `127.0.0.1` and `127.0.0.1:64000` work.
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
    run_chat(stream);
}

/// Run the shared chat logic once a TCP connection exists.
///
/// The stream is cloned so the program can read and write at the same time:
/// the main thread sends keyboard input, while a reader thread receives peer
/// messages.
fn run_chat(stream: TcpStream) {
    let reader_stream = stream.try_clone().expect("Could not clone the TCP stream");

    let reader_thread = thread::spawn(move || {
        let reader = BufReader::new(reader_stream);

        // `lines` waits until a complete newline-terminated message arrives.
        for message in reader.lines() {
            match message {
                Ok(message) => println!("\nPeer: {message}\nYou: "),
                Err(error) => {
                    eprintln!("\nCould not read from peer: {error}");
                    break;
                }
            }
        }

        println!("\nThe other program disconnected.");
    });

    println!("Type a message and press Enter. Type /quit to leave.");
    print!("You: ");
    io::stdout().flush().expect("Could not flush stdout");

    let stdin = io::stdin();
    let mut writer = stream;

    for line in stdin.lock().lines() {
        match line {
            Ok(message) => {
                if message == "/quit" {
                    break;
                }

                if let Err(error) = writeln!(writer, "{message}") {
                    eprintln!("Could not send message: {error}");
                    break;
                }

                if let Err(error) = writer.flush() {
                    eprintln!("Could not flush message: {error}");
                    break;
                }

                print!("You: ");
                io::stdout().flush().expect("Could not flush stdout");
            }
            Err(error) => {
                eprintln!("Could not read keyboard input: {error}");
                break;
            }
        }
    }

    // Closing both halves tells the peer that this user is finished.
    let _ = writer.shutdown(Shutdown::Both);
    let _ = reader_thread.join();
    println!("Chat ended.");
}

fn print_usage(program: &str) {
    println!(
        "Usage:\n  {program} server [bind_address:port]\n  {program} client <server_ip_or_host> [port]\n\nExamples:\n  cargo run -- server\n  cargo run -- client 127.0.0.1\n  cargo run -- client 192.168.1.25 64000\n\nType /quit to end a chat."
    );
}
