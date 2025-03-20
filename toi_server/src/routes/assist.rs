use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{client, models, schema, state, utils};

pub fn router(openapi_spec: String, state: state::ToiState) -> OpenApiRouter {
    let chat = move |state| chat(openapi_spec, state);
    OpenApiRouter::new().routes(routes!(chat)).with_state(state)
}

#[utoipa::path(
    post,
    path = "",
    responses(
        (status = 200, description = "Successfully got a response"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
async fn chat(openapi_spec: String, State(client): State<client::Client>) -> Result {
    let chat_response_kind_system_prompt = format!(
        r#"
{}

Here is the OpenAPI spec for reference:

{}

And here are your classification options:

{}
{}
{}
{}
{}

{}
        "#,
        models::assist::CHAT_RESPONSE_KIND_SYSTEM_PROMPT_INTRO,
        openapi_spec,
        models::assist::ChatResponseKind::Unfulfillable,
        models::assist::ChatResponseKind::FollowUp,
        models::assist::ChatResponseKind::Answer,
        models::assist::ChatResponseKind::AnswerWithDraftHttpRequests,
        models::assist::ChatResponseKind::AnswerWithHttpRequests,
        models::assist::CHAT_RESPONSE_KIND_SYSTEM_PROMPT_OUTRO
    );
}
