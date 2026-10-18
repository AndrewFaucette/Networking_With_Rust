# Networking With Rust

A Client-Server chat application. The application is run in two terminals, one configured to be the client, and the other to be the server.

## Instructions for Build and Use

Steps to build and/or run the software:

1. Run `cargo build` in the `chat` directory to build the code (or you can just find and run the executable in the `target` folder).
2. Navigate to the `chat` directory in two seperate terminals.
3. run `cargo run -- server` in one terminal and `cargo run -- client 127.0.0.1` in the other terminal.

Instructions for using the software:

1. You can send messages in either terminal and they will be recieved by the other.

## Development Environment

To recreate the development environment, you need the following software and/or libraries with the specified versions:

* Rust v1.98.1
* Cargo v1.98.1

## Useful Websites to Learn More

I found these websites useful in developing this software:

* [Beginner's Rust Tutorial: Networking for First Time Students](https://youtu.be/p9wU1uBuhQ8?si=idvTm53Sc1J5Q8b6)
* [Rust for Network Programming](https://dev.to/godofgeeks/rust-for-network-programming-1en5)

## Future Work

The following items I plan to fix, improve, and/or add to this project in the future:

* [ ] Add functionality for more than two people in the chat
* [ ] Create a version that is strictly peer-to-peer instead of client-server
* [ ] Automatically configure the first run terminal as the client to avoid user configuration at runtime
