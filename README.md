![Demo][0]

<div align="center">
    <i>A personal assistant server and client.</i>
</div>

# toi

An extensible personal assistant web server with a simple REPL chat client.

- For details on the server and client, see [`toi_server/README.md`][1] and 
  [`toi_client/README.md`][2], respectively.
- To extend the assistant, see [`CONTRIBUTING.md`][3].

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

# Non-goals

This project is largely a learning exercise and a proof of concept. As such,
the following (and probably other things) are out of scope:

- Support for multiple users or tenants
- Additional tool calling -like endpoints similar to the `/assistant` endpoint
- UIs beyond the provided REPL

[0]: assets/demo.gif?raw=true
[1]: toi_server
[2]: toi_client
[3]: CONTRIBUTING.md
