![Demo][0]

<div align="center">
    <i>A personal assistant server and client.</i>
</div>

# toi

This is a proof-of-concept for an extensible personal assistant.

See [`toi_client/README.md`][1] and [`toi_server/README.md`][2] for more info
on the client and server, respectively. See [`CONTRIBUTING.md`] for how to add
new features to the personal assistant.

# Requirements

I developed and tested this project using a single NVIDIA RTX 2080 and WSL. 
As such, this project and the default models provided in the Docker Compose
files are intended to run on a commercially available GPU with at least 8GB
of VRAM. That isn't to say this project will not work natively on Windows,
with CPUs, or even with GPUs with less VRAM; I simply have not tested it with 
those variations.

# Quickstart

1. Run the server using the provided Docker Compose file:

   ```bash
   docker compose up -d
   ```

   You can configure runtime environment variables using a local `.env` file.
   As an example, you can change the build target and log level with an `.env`
   file with the following contents:


   ```bash
   RELEASE=true
   RUST_LOG=info,tower_http=trace
   ```

2. Install the client binary:

   ```bash
   cargo install toi_client
   ```

3. Start an interactive REPL session using the client binary:

   ```bash
   toi_client
   ```

# Testing

Build and test the server using the provided test Docker Compose file:

```bash
docker compose -f docker-compose.test.yaml up -d --build
```

# Non-goals

This project is largely a learning exercise and a proof of concept. As such,
the following (and probably other things) are out of scope:

- Support for multiple users or tenants
- Additional tool calling -like endpoints similar to the `/chat` endpoint
- UIs beyond the provided REPL

[0]: assets/demo.gif?raw=true
[1]: https://github.com/theOGognf/toi/tree/main/toi_client
[2]: https://github.com/theOGognf/toi/tree/main/toi_server
