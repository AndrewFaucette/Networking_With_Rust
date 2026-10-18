# Networking With Rust

This project is a simple two-person chat application written in Rust. It uses
the standard library's `TcpListener` and `TcpStream` types, so it does not need
external crates.

## How the program is structured

The same executable can run in two modes:

- **Server mode** binds to a local address, listens for a TCP connection, and
  accepts one client.
- **Client mode** connects to the server's IP address and port.
- **Chat mode** is shared by both sides after they connect. The TCP stream is
  cloned so one thread can receive messages while the main thread reads and
  sends keyboard input.

Messages are sent as UTF-8 text with a newline at the end. The newline gives
the receiver a simple message boundary, which is important because TCP is a
continuous byte stream rather than a message-by-message protocol.

## Build

Install Rust from [rustup.rs](https://rustup.rs/), then build from the `chat`
directory:

```text
cd CSE310/Networking_With_Rust/chat
cargo build
```

## Run two programs on one computer

Open two terminals in the `chat` directory. Start the server in the first:

```text
cargo run -- server
```

Start the client in the second:

```text
cargo run -- client 127.0.0.1
```

Type a message and press Enter. Both programs can send messages at the same
time. Type `/quit` to close the connection.

## Run across two computers

1. On the host computer, run `cargo run -- server`.
2. Find the host computer's local IP address, such as `192.168.1.25`.
3. On the other computer, run `cargo run -- client 192.168.1.25`.
4. Allow TCP port `64000` through the host computer's firewall if prompted.

The two computers should be on the same network unless the port is deliberately
made reachable through a router or other network configuration.

You can use a different port as well:

```text
cargo run -- server 0.0.0.0:65000
cargo run -- client 192.168.1.25 65000
```

## Useful Rust concepts demonstrated

- `TcpListener::bind` and `TcpStream::connect` for TCP networking
- `TcpStream::try_clone` for independent reading and writing
- `std::thread::spawn` for receiving while the user types
- `BufReader::lines` for newline-delimited messages
- `Result` handling for connection, input, and network errors

## Future work

- [ ] Support multiple clients in a shared chat room
- [ ] Add usernames and timestamps
- [ ] Add a cleaner terminal display for incoming messages
- [ ] Add encrypted communication with TLS
