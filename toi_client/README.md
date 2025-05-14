# toi_client

A proof-of-concept for a minimal, terminal-based chat client for interacting
with the personal assistant server.

# Features

- Basic context size management based on token usage
- CTRL+C to interrupt the response stream during a response
- CTRL+C to clear the input buffer when it isn't empty
- CTRL+C to exit when the input buffer is empty

# Notable dependencies

- [ctrlc][0] for some CTRL+C signal handling
- [pico-args][1] for the CLI argument parser
- [reqwest][2] for the HTTP client
- [rustyline][3] for the REPL interface
- [serde][4] and [serde_json][5] for the serialization/deserialization stuff
- [tokio][6] for async stuff

# Related artifacts

- [A library dependency][8]
- [A server to interact with][9]

# Acknowledgements

- The streaming functionality is largely based on [this demo][7]

[0]: https://crates.io/crates/ctrlc
[1]: https://crates.io/crates/pico-args
[2]: https://crates.io/crates/reqwest
[3]: https://crates.io/crates/rustyline
[4]: https://crates.io/crates/serde
[5]: https://crates.io/crates/serde_json
[6]: https://crates.io/crates/tokio
[7]: https://github.com/a-poor/openai-stream-rust-demo
[8]: https://github.com/theOGognf/toi/tree/main/toi
[9]: https://github.com/theOGognf/toi/tree/main/toi_server
