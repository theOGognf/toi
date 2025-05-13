![DEMO][0]

<div align="center">
    <i>A personal assistant server and client.</i>
</div>

# toi

This is a proof-of-concept project focused on building an extensible personal
assistant for text-driven interactions.

The server is a RESTful API with an endpoint that translates natural language
into HTTP requests for other endpoints internal to the server. Along with using
a type-safe ORM for context and memory management, the server generates response
templates from predefined structs for additional type safety.

See [`toi_client/README.md`][1] and [`toi_server/README.md`][2] for more info.

# Requirements

I developed and tested this project using a single NVIDIA RTX 2080 and WSL. 
As such, this project and the default models provided in the Docker Compose
files are intended to run on a commercially available GPU with at least 8GB
of VRAM. That isn't to say this project will not work natively on Windows,
with CPUs, or even with GPUs with less VRAM; I simply have not tested it with 
those variations.

# Project structure

See each subdirectory's docs or `README.md`s for more specific info.

```bash
.
├── toi             # Library that the client and server use
├── toi_client      # Client binary source
└── toi_server      # Server binary source
```

# Running the server

Build and run the server and its supporting services using the provided
Docker Compose file:

```bash
docker compose up -d --build
```

You can configure runtime environment variables using a local `.env` file.
As an example, you can change the build target and log level with an `.env`
file with the following contents:

```bash
RELEASE=true
RUST_LOG=info,tower_http=trace
```

# Testing the server

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

[0]: assets/demo.png?raw=true
[1]: https://github.com/theOGognf/toi/tree/main/toi_client
[2]: https://github.com/theOGognf/toi/tree/main/toi_server
