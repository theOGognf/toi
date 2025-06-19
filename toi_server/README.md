# toi_server

A personal assistant server with type-safe tool search and tool usage via
HTTP API endpoints.

# Requirements

The server requires the following supporting services:

- A [Postgres database][0] with [pgvector][1]
- An [OpenAI-compliant embedding API][2]
- An [OpenAI-compliant chat completions API][3]
- A [vLLM reranking API][4]

The server binary also has some native dependencies, so the [Docker image][5]
is the easiest way to get started.

# Configuration

At least two environment variables are required for configuration:

- `DATABASE_URL`: required by Diesel for connecting to the backing database
- `TOI_CONFIG_PATH`: path to the server configuration file

The actual server configuration file at the path defined by `TOI_CONFIG_PATH`
should have [HTTP client options][6] for the embedding, generation, and
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
reranking similarity threshold values referenced by the [configuration struct][7].

# Notable dependencies

- [axum][8] for HTTP endpoint definitions
- [Diesel][9] for type-safe database interactions
- [pgvector-rust][10] for pgvector Rust support
- [schemars][11] for JSON Schema generation
- [serde][12] and [serde_json][13] for the serialization/deserialization stuff
- [tokio][14] for async stuff
- [Utoipa][15] for OpenAPI docs generation

# How it works

- A user makes a request to the `/assistant` endpoint
- The embedding API is used for vector search to find server endpoint
  descriptions similar to the user's most recent message/query
- The vector search results are filtered and reranked using the reranking API
- If the best-fit endpoint matches the user's query within a threshold,
  its JSON Schema is used to build an HTTP request using the generation API
- The generated HTTP request is added as an assistant message to the local 
  context
- The generated HTTP request is sent to the best-fit endpoint
- The HTTP response is added as a user message to the local context
- The generation API is used to stream a summarization of the response back
  to the user

# Related artifacts

- [A library dependency][16]
- [A client for interacting with the server][17]

[0]: https://github.com/postgres/postgres
[1]: https://github.com/pgvector/pgvector
[2]: https://platform.openai.com/docs/api-reference/embeddings
[3]: https://platform.openai.com/docs/api-reference/chat/create
[4]: https://docs.vllm.ai/en/latest/serving/openai_compatible_server.html#re-rank-api
[5]: https://hub.docker.com/r/ognf/toi
[6]: https://github.com/theOGognf/toi/blob/4bb2d008de56e4fcd8be1af51e819028e41cbddb/toi_server/src/models/client.rs#L137
[7]: https://github.com/theOGognf/toi/blob/4bb2d008de56e4fcd8be1af51e819028e41cbddb/toi_server/src/models/config.rs#L21
[8]: https://crates.io/crates/axum
[9]: https://crates.io/crates/diesel
[10]: https://crates.io/crates/pgvector
[11]: https://crates.io/crates/schemars
[12]: https://crates.io/crates/serde
[13]: https://crates.io/crates/serde_json
[14]: https://crates.io/crates/tokio
[15]: https://crates.io/crates/utoipa
[16]: https://github.com/theOGognf/toi/tree/main/toi
[17]: https://github.com/theOGognf/toi/tree/main/toi_client
