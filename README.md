![Demo][0]

<div align="center">
    <i>A personal assistant server and client.</i>
</div>

# toi

An extensible personal assistant HTTP server with a simple REPL chat client.

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

# Motivation

In addition to wanting to learn some of the dependencies I used in this project,
I've been thinking about making a self-hosted personal assistant that I could 
use and easily extend myself for a while now. Recently, there's been a flurry of
AI tool usage articles, followed by the announcement of the Model Context 
Protocol (MCP), and now MCP servers are popping-up everywhere. Eventually, I
couldn't resist the intrusive thought of *"well, you could just build type-safe
tools using plain ol' HTTP endpoints, OpenAPI schemas, and JSON Schemas"*. And
so that's what this is.

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
