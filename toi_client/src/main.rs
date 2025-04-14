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
use toi::{GenerationRequest, Message, MessageRole, detailed_reqwest_error};
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

async fn client(url: String, mut rx: Receiver<ServerRequest>, tx: Sender<ServerResponse>) {
    let client = reqwest::Client::new();

    loop {
        if let Some(ServerRequest::Start(request)) = rx.recv().await {
            let response = client
                .post(&url)
                .json(&request)
                .send()
                .await
                .map_err(detailed_reqwest_error);
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
                            Ok(Some(line)) = lines.next_line() => {
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
                                                },
                                                Err(err) => {
                                                    let message = ServerResponse::Error(err.to_string());
                                                    tx.send(message).await.expect("server response channel full");
                                                    break
                                                }
                                            }

                                        },
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
                    let text = response.text().await.unwrap_or_else(detailed_reqwest_error);
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

fn ctrlc_handler(tx: Sender<UserRequest>) -> Result<(), ctrlc::Error> {
    set_handler(move || {
        let message = UserRequest::Cancel;
        tx.blocking_send(message)
            .expect("server response interrupt channel full");
    })?;

    thread::park();

    Ok(())
}

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

    pub fn prune(&mut self) {
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

    pub fn push_assistant(&mut self, usage: TokenUsage) {
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
    }

    pub fn push_buffer(&mut self, content: String) {
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
                        if let Some(content) = chunk.content.first() {
                            history.push_buffer(content.clone());
                            print!("{content}");
                            io::stdout().flush()?;
                        }
                        if let Some(usage) = chunk.usage {
                            history.push_assistant(usage);
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

        // Shorten message history to fit context limit.
        history.prune();
    }
}
