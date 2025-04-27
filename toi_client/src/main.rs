use ctrlc::set_handler;
use futures::stream::TryStreamExt;
use models::client::TokenUsage;
use pico_args::Arguments;
use rustyline::{
    Cmd, ConditionalEventHandler, DefaultEditor, Event, EventContext, EventHandler, KeyEvent,
    error::ReadlineError,
};
use std::io::{self, Write};
use std::{collections::VecDeque, thread};
use toi::{GenerationRequest, Message, MessageRole};
use tokio::{
    io::AsyncBufReadExt,
    sync::mpsc::{Receiver, Sender},
};
use tokio_util::io::StreamReader;

mod models;

use models::{
    client::GenerationResponseChunk,
    repl::{ServerRequest, ServerResponse, UserRequest},
};

/// Loop for interacting with the server. Waits for a new message request,
/// and, when one is received, attempts to stream the response in chunks
/// until it finishes or an interrupt signal is caught.
async fn client(url: String, mut rx: Receiver<ServerRequest>, tx: Sender<ServerResponse>) {
    let client = reqwest::Client::new();

    loop {
        if let Some(ServerRequest::Start(request)) = rx.recv().await {
            let response = client
                .post(&url)
                .json(&request)
                .send()
                .await
                .map_err(|err| format!("{err:?}"));
            match response {
                Err(err) => {
                    let message = ServerResponse::Error(err);
                    tx.send(message)
                        .await
                        .expect("server response channel full");
                }
                Ok(response) if response.status() == 200 => {
                    let stream = response
                        .bytes_stream()
                        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err));
                    let reader = StreamReader::new(stream);
                    let mut lines = reader.lines();
                    loop {
                        tokio::select! {
                            result = lines.next_line() => {
                                match result {
                                    Ok(Some(line)) => {
                                        if let Some(data) = line.strip_prefix("data: ") {
                                            match data {
                                                "[DONE]" => {
                                                    let message = ServerResponse::Done;
                                                    tx.send(message).await.expect("server response channel full");
                                                    break
                                                }
                                                "\n" | "" => {}
                                                data => {
                                                    let response = serde_json::from_str::<GenerationResponseChunk>(data);
                                                    match response {
                                                        Ok(chunk) => {
                                                            let message = ServerResponse::Chunk(chunk);
                                                            tx.send(message).await.expect("server response channel full");
                                                        }
                                                        Err(err) => {
                                                            let message = ServerResponse::Error(err.to_string());
                                                            tx.send(message).await.expect("server response channel full");
                                                            break
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    // This shouldn't get hit in a streaming response because streaming responses
                                    // end with the '[DONE]' string before returning no lines.
                                    Ok(None) => unreachable!("streaming response didn't end on [DONE]"),
                                    Err(err) => {
                                        let message = ServerResponse::Error(err.to_string());
                                        tx.send(message).await.expect("server response channel full");
                                        break
                                    }
                                }
                            }
                            Some(ServerRequest::Cancel) = rx.recv() => {
                                let message = ServerResponse::Done;
                                tx.send(message).await.expect("server response channel full");
                                break
                            }
                        }
                    }
                }
                Ok(response) => {
                    let text = match response.error_for_status() {
                        Ok(response) => {
                            let repr = format!("{response:?}");
                            let content = response
                                .text()
                                .await
                                .unwrap_or_else(|err| format!("{repr} with error {err:?}"));
                            format!("{repr} with content {content}")
                        }
                        Err(err) => format!("{err:?}"),
                    };
                    let message = ServerResponse::Error(text);
                    tx.send(message)
                        .await
                        .expect("server response channel full");
                }
            }
        }
    }
}

struct InterruptEventHandler;

impl ConditionalEventHandler for InterruptEventHandler {
    fn handle(
        &self,
        _: &Event,
        _: rustyline::RepeatCount,
        _: bool,
        ctx: &EventContext,
    ) -> Option<rustyline::Cmd> {
        if ctx.line().is_empty() {
            Some(Cmd::EndOfFile)
        } else {
            Some(Cmd::Interrupt)
        }
    }
}

/// User REPL loop. The user can enter chat messages or clear their input
/// using this loop. If a message is sent, a response is streamed from
/// the server and this REPL is inactive until the response finishes or
/// the stream is interrupted through the other CTRL+C handler.
fn repl(mut rx: Receiver<()>, tx: Sender<UserRequest>) -> Result<(), ReadlineError> {
    let mut rl = DefaultEditor::new()?;
    let interrupt_event_handler = Box::new(InterruptEventHandler);
    rl.bind_sequence(
        KeyEvent::ctrl('c'),
        EventHandler::Conditional(interrupt_event_handler),
    );

    while rx.blocking_recv().is_some() {
        loop {
            match rl.readline(">> ") {
                Ok(input) => {
                    let message = UserRequest::Prompt(input);
                    tx.blocking_send(message)
                        .expect("user request channel full");
                    break;
                }
                Err(ReadlineError::Interrupted) => {
                    println!("^C");
                }
                Err(ReadlineError::Eof) => {
                    std::process::exit(0);
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// For catching user interrupts during a response stream. If a response
/// stream is not active, then the interrupt handler from the REPL takes
/// precedence and this doesn't catch the signal.
fn ctrlc_handler(tx: Sender<UserRequest>) -> Result<(), ctrlc::Error> {
    set_handler(move || {
        let message = UserRequest::Cancel;
        tx.blocking_send(message)
            .expect("server response interrupt channel full");
    })?;

    thread::park();

    Ok(())
}

/// History is used for maintaining a context limit. Context limit is
/// set as a CLI option.
struct History {
    limit: u32,
    size: u32,
    buffer: Vec<String>,
    message_history: VecDeque<Message>,
    usage_history: VecDeque<TokenUsage>,
}

impl History {
    pub fn len(&self) -> usize {
        self.message_history.len()
    }

    pub fn new(limit: u32) -> Self {
        Self {
            limit,
            size: 0,
            buffer: vec![],
            message_history: VecDeque::new(),
            usage_history: VecDeque::new(),
        }
    }

    pub fn pop_back(&mut self) {
        self.message_history.pop_back();
    }

    pub fn push_assistant_and_token_usage(&mut self, usage: TokenUsage) {
        let message = Message {
            role: MessageRole::Assistant,
            content: self.buffer.join(""),
        };
        let total_usage = usage.prompt_tokens + usage.completion_tokens;
        self.size = self
            .size
            .checked_add_signed(total_usage)
            .expect("overflow from adding token usage");
        self.message_history.push_back(message);
        self.usage_history.push_back(usage);
        self.buffer.clear();

        while self.size > self.limit {
            if let Some(usage) = self.usage_history.pop_front() {
                let total_usage = usage.prompt_tokens + usage.completion_tokens;
                self.size = self
                    .size
                    .checked_add_signed(-total_usage)
                    .expect("overflow from subbing token usage");
                self.message_history = self.message_history.split_off(2);
            }
        }
    }

    pub fn push_assistant_chunk(&mut self, content: String) {
        self.buffer.push(content);
    }

    pub fn push_user(&mut self, content: String) -> GenerationRequest {
        let message = Message {
            role: MessageRole::User,
            content,
        };
        self.message_history.push_back(message);
        GenerationRequest::new(self.message_history.clone().into())
    }
}

const HELP: &str = "\
Chat with a personal assistant

USAGE:
  toi_client [OPTIONS]

OPTIONS:
  --url     Server chat URL     [default: 127.0.0.1:6969/chat]
  --limit   Chat context limit  [default: 8000]

FLAGS:
  -h, --help    Print help information
";

struct Args {
    url: String,
    context_limit: u32,
}

/// Minimal REPL
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut pargs = Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        println!("{}", HELP);
        std::process::exit(0);
    }

    let args = Args {
        url: pargs
            .value_from_str("--url")
            .unwrap_or("127.0.0.1:6969/chat".into()),
        context_limit: pargs.value_from_str("--limit").unwrap_or(8000),
    };
    let Args { url, context_limit } = args;

    // Channels for all the IPC going on.
    let (start_repl_sender, start_repl_receiver) = tokio::sync::mpsc::channel(1);
    let (user_request_sender, mut user_request_receiver): (
        Sender<UserRequest>,
        Receiver<UserRequest>,
    ) = tokio::sync::mpsc::channel(1);
    let ctrlc_user_request_sender = user_request_sender.clone();
    let (server_request_sender, server_request_receiver): (
        Sender<ServerRequest>,
        Receiver<ServerRequest>,
    ) = tokio::sync::mpsc::channel(1);
    let (server_response_sender, mut server_response_receiver): (
        Sender<ServerResponse>,
        Receiver<ServerResponse>,
    ) = tokio::sync::mpsc::channel(1);

    // Begin background processes.
    thread::spawn(|| repl(start_repl_receiver, user_request_sender));
    tokio::spawn(client(url, server_request_receiver, server_response_sender));
    thread::spawn(|| ctrlc_handler(ctrlc_user_request_sender));

    // Kick-off the user prompt.
    start_repl_sender.send(()).await?;

    // Main loop.
    let mut history = History::new(context_limit);
    loop {
        tokio::select! {
            Some(user_request) = user_request_receiver.recv() => {
                let server_request = match user_request {
                    UserRequest::Prompt(input) => {
                        let request = history.push_user(input);
                        ServerRequest::Start(request)
                    }
                    UserRequest::Cancel => ServerRequest::Cancel
                };
                server_request_sender.send(server_request).await?;
            }
            Some(server_response) = server_response_receiver.recv() => {
                match server_response {
                    ServerResponse::Chunk(chunk) => {
                        if let Some(choice) = chunk.choices.first() {
                            history.push_assistant_chunk(choice.delta.content.clone());
                            print!("{}", choice.delta.content);
                            io::stdout().flush()?;
                        }
                        if let Some(usage) = chunk.usage {
                            history.push_assistant_and_token_usage(usage);
                        }
                    }
                    ServerResponse::Done => {
                        // Edge case where the assistance can finish their response,
                        // but the user cancelled the request just prior. If there's
                        // an odd number of messages, then we know this edge case
                        // didn't occur, and that the user's message can be ignored
                        // from the history. Otherwise, keep the latest message.
                        if history.len() % 2 == 1 {
                            history.pop_back();
                        }
                        println!();
                        start_repl_sender.send(()).await?
                    },
                    ServerResponse::Error(err) => {
                        // Edge case where the assistant can finish their response,
                        // but the done signal doesn't come through just yet. If there's
                        // an odd number of messages, then we know this edge case
                        // occurred, and we don't want to pop the assistant's
                        // message.
                        if history.len() % 2 == 1 {
                            history.pop_back();
                        }
                        println!("Error: {err}");
                        start_repl_sender.send(()).await?;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::History;
    use super::models::client::TokenUsage;

    #[test]
    fn pruning_history() {
        let mut history = History::new(10);

        // Add a user message and verify that nothing can be pruned yet since
        // there are no token usage metrics.
        history.push_user("Hello! What's your name?".to_string());
        assert_eq!(history.len(), 1);

        // Simulate an assistant response in chunks. Verify the history is still
        // the same length before and after attempting to prune yet again.
        for s in ["I", " have", " no", " name"].into_iter() {
            history.push_assistant_chunk(s.to_string());
        }
        assert_eq!(history.len(), 1);

        // Simulate the assistant given a token usage response, signaling the
        // end of the response, but with still not enough tokens to warrant
        // pruning.
        history.push_assistant_and_token_usage(TokenUsage {
            prompt_tokens: 5,
            completion_tokens: 4,
        });
        assert_eq!(history.len(), 2);
        assert_eq!(history.size, 9);

        // Finally, push one more user and assistant interaction that results
        // in pruning of the original exchange.
        history.push_user("oh...".to_string());
        history.push_assistant_chunk(":(".to_string());
        history.push_assistant_and_token_usage(TokenUsage {
            prompt_tokens: 2,
            completion_tokens: 1,
        });
        assert_eq!(history.len(), 2);
        assert_eq!(history.size, 3);
    }
}
