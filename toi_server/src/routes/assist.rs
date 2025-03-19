use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use diesel::{
    ExpressionMethods,
    prelude::{QueryDsl, SelectableHelper},
};
use diesel_async::RunQueryDsl;
use pgvector::VectorExpressionMethods;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{models, schema, state, utils};

pub fn router(state: state::ToiState) -> OpenApiRouter {
    OpenApiRouter::new().routes(routes!(chat)).with_state(state)
}

#[utoipa::path(
    post,
    path = "",
    responses(
        (status = 200, description = "Successfully got assistance"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
async fn chat(State(state): State<state::ToiState>) -> Result {
    todo!(
        r#"Maybe a prompt that takes in the OpenAPI spec and asks an LLM to classify what to do:
1. Provide a direct response to the user. E.g., if their request is not clear or there's an immediate
    answer to their request.
2. Draft the HTTP requests that could perform the user's request and ask the user to provide feedback
    and/or confirmation.
3. The user request is clear and HTTP requests can be made to complete the request. Make the requests
    directly and then provide the user feedback on the response of the requests.
"#
    );
}
