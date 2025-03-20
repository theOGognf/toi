use std::fmt;

pub const CHAT_RESPONSE_KIND_SYSTEM_PROMPT_INTRO: &'static str = "You are a \
chat assistant that helps preprocess a user's message. Given an OpenAPI spec \
and a chat history, your job is to classify what kind of response is best. ";

pub const CHAT_RESPONSE_KIND_SYSTEM_PROMPT_OUTRO: &'static str = "Only respond \
with the number of the response that fits best and nothing else.";

pub enum ChatResponseKind {
    Unfulfillable,
    FollowUp,
    Answer,
    AnswerWithDraftHttpRequests,
    AnswerWithHttpRequests,
}

impl fmt::Display for ChatResponseKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        let repr = match self {
            Self::Unfulfillable => {
                "0. Unfulfillable: the user's message cannot accurately \
                be responded to by an answer or fulfilled by HTTP request(s). \
                It's best to notify the user."
            }
            Self::FollowUp => {
                "1. Follow-up: the user's message cannot be fulfilled by \
                an answer or HTTP request(s). It's best to follow-up to \
                seek clarification."
            }
            Self::Answer => {
                "2. Answer: the user's message is clear and can be answered \
                directly without HTTP request(s). It's best to concisely \
                answer."
            }
            Self::AnswerWithDraftHttpRequests => {
                "3. Answer with draft HTTP request(s): the user's message \
                indicates they want an action to be performed with HTTP \
                request(s), but the HTTP request(s) could benefit from \
                user clarifications and/or updates. It's best to show a \
                draft of the HTTP request(s) to the user and seek their \
                input and confirmation."
            }
            Self::AnswerWithHttpRequests => {
                "4. Answer with HTTP request(s): the user's message is \
                clear and can be accurately fulfilled with HTTP request(s). \
                It's best to make the HTTP request(s) and summarize those \
                requests and their respective responses to the user."
            }
        };
        write!(f, "{repr}")
    }
}

impl From<u8> for ChatResponseKind {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::FollowUp,
            2 => Self::Answer,
            3 => Self::AnswerWithDraftHttpRequests,
            4 => Self::AnswerWithHttpRequests,
            _ => Self::Unfulfillable,
        }
    }
}
