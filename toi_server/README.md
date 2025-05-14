# toi_server

A proof-of-concept for a personal assistant server with type-safe tool search
and tool usage via HTTP API endpoints.

# Requirements

The server requires the following supporting services:

- A [Postgres database][0] with [pgvector][1]
- An [OpenAI-compliant embedding API][2]
- An [OpenAI-compliant chat completions API][3]
- A [vLLM reranking API][4]

The server binary also has some native dependencies, so the Docker image
is the easiest way to get started.

# Configuration

At least two environment variables are required for configuration:

- `DATABASE_URL`: required by Diesel for connecting to the backing database
- `TOI_CONFIG_PATH`: path to the server configuration file

The actual server configuration file at the path defined by `TOI_CONFIG_PATH`
should have [HTTP client options][5] for the embedding, generation, and
reranking APIs. It also supports environment variable interpolation for some
values, so you can put something like this to keep API secrets safe:

```json
{
    "server": {
        "bind_addr": "0.0.0.0:6969",
        "user_agent": "${USER_AGENT}"
    },
    "embedding": {
        "base_url": "http://embedding:8000"
    },
    "generation": {
        "base_url": "http://generation:8000",
        "headers": {
            "api_key": "${MY_API_KEY}"
        }
    },
    "reranking": {
        "base_url": "http://reranking:8000"
    }
}
```

If you decide to use different models from the ones provided by the project's
Docker Compose file, then be sure to tune/set the embedding distance and
reranking similarity threshold values referenced by the [configuration struct][6].

# Notable dependencies

- [axum][7] for HTTP endpoint definitions
- [Diesel][8] for type-safe database interactions
- [pgvector-rust][9] for pgvector Rust support
- [schemars][10] for JSON Schema generation
- [serde][11] and [serde_json][12] for the serialization/deserialization stuff
- [tokio][13] for async stuff
- [Utoipa][14] for OpenAPI docs generation

# How it works

Generally, the flow of a user's request goes as follows:

- A user makes a request to the `/chat` endpoint
- An embedding API is used for vector search to find server endpoint descriptions
similar to the user's most recent message/query
- The vector search results are filtered and reranked using a reranking API
- If the top endpoint result matches the user's query within a threshold,
its JSON Schema is used to make an HTTP request for that endpoint using
a generation API
- The generated HTTP request is added as an assistant message to the local 
context
- The generated HTTP request is sent to the top endpoint
- The HTTP response is added as a user message to the local context
- A generation API is used to stream a summarization of the response
back to the user

# Motivation

In addition to wanting to learn some of the dependencies I used in this project,
I've been thinking about making a self-hosted personal assistant that I could 
use and easily extend myself for a while now. Recently, there's been a flurry of
AI tool usage articles, followed by the announcement of the Model Context 
Protocol (MCP), and now MCP servers are popping-up everywhere. Eventually, I
couldn't resist the intrusive thought of *"well, you could just build type-safe
tools using plain ol' HTTP endpoints, OpenAPI schemas, and JSON Schemas"*.

And so that's what this is.

# Related artifacts

- [A library dependency][15]
- [A client for interacting with the server][16]

[0]: https://github.com/postgres/postgres
[1]: https://github.com/pgvector/pgvector
[2]: https://platform.openai.com/docs/api-reference/embeddings
[3]: https://platform.openai.com/docs/api-reference/chat/create
[4]: https://docs.vllm.ai/en/latest/serving/openai_compatible_server.html#re-rank-api
[5]: https://github.com/theOGognf/toi/blob/4bb2d008de56e4fcd8be1af51e819028e41cbddb/toi_server/src/models/client.rs#L137
[6]: https://github.com/theOGognf/toi/blob/4bb2d008de56e4fcd8be1af51e819028e41cbddb/toi_server/src/models/config.rs#L21
[7]: https://crates.io/crates/axum
[8]: https://crates.io/crates/diesel
[9]: https://crates.io/crates/pgvector
[10]: https://crates.io/crates/schemars
[11]: https://crates.io/crates/serde
[12]: https://crates.io/crates/serde_json
[13]: https://crates.io/crates/tokio
[14]: https://crates.io/crates/utoipa
[15]: https://github.com/theOGognf/toi/tree/main/toi
[16]: https://github.com/theOGognf/toi/tree/main/toi_client
