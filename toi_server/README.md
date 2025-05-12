# toi_server

A proof-of-concept personal assistant server for type-safe tool search and
usage via HTTP API endpoints.

## Configuration

At least two environment variables are required for configuration:

- `DATABASE_URL`: required by Diesel for connecting to the backing database
- `TOI_CONFIG_PATH`: path to the server configuration file

The actual server configuration file at the path defined by `TOI_CONFIG_PATH`
should have [HTTP client options][0] for the embedding, generation, and
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

If you decide to use models different from the ones provided by the project's
Docker Compose file, then be sure to tune/set the embedding distance and
reranking similarity threshold values in the [configuration file][1].

## Requirements

The server requires the following supporting services:

- A Postgres database with pgvector
- An OpenAI-compliant embedding API
- An OpenAI-compliant chat completions API
- A [vLLM reranking API][2]

The server binary also has some native dependencies, so the Docker image
is the easiest way to get started.

## How It Works

- [Postgres][3] as the backing database
- [pgvector][4] for tool and memory search
- [Diesel][5] for type-safe database interactions
- [axum][6] for HTTP endpoint definitions
- [Utoipa][7] for OpenAPI docs generation
- [schemars][8] for JSON Schema generation

## Motivation

On top of wanting to learn some of the dependencies I used in this project,
I've been thinking about making a self-hosted personal assistant that I could use
and easily extend myself for a while now. Then there've been a flurry of AI tool
usage articles, then Model Context Protocol (MCP) was announced, and then MCP servers
have been popping-up everywhere, so I finally couldn't resist the intrusive thought
of *"well, you could just build type-safe tools and JSON schemas using plain ol' HTTP
endpoints and OpenAPI schemas"*.

And so that's what this is.

[0]: https://github.com/theOGognf/toi/blob/4bb2d008de56e4fcd8be1af51e819028e41cbddb/toi_server/src/models/client.rs#L137
[1]: https://github.com/theOGognf/toi/blob/4bb2d008de56e4fcd8be1af51e819028e41cbddb/toi_server/src/models/config.rs#L21
[2]: https://docs.vllm.ai/en/latest/serving/openai_compatible_server.html#re-rank-api
[3]: https://github.com/postgres/postgres
[4]: https://github.com/pgvector/pgvector
[5]: https://crates.io/crates/diesel
[6]: https://crates.io/crates/axum
[7]: https://crates.io/crates/utoipa
[8]: https://crates.io/crates/schemars
