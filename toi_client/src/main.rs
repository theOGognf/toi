use ctrlc::set_handler;
use futures::stream::TryStreamExt;
use models::client::TokenUsage;
use rustyline::error::ReadlineError;
use rustyline::{
    Cmd, ConditionalEventHandler, DefaultEditor, Event, EventContext, EventHandler, KeyEvent,
};
use std::collections::VecDeque;
use std::thread;
use toi::{Message, MessageRole};
use tokio::io::AsyncBufReadExt;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_util::io::StreamReader;

mod models;

use models::{
    client::GenerationResponseChunk,
    repl::{ServerRequest, ServerResponse, UserRequest},
};

async fn client(url: &str, mut rx: Receiver<ServerRequest>, tx: Sender<ServerResponse>) {
    let client = reqwest::Client::new();

    loop {
        if let Some(ServerRequest::Start(messages)) = rx.recv().await {
            let response = client
                .post(url)
                .json(&messages)
                .send()
                .await
                .map_err(|err| err.to_string());
            match response {
                Err(err) => {
                    let message = ServerResponse::Error(err);
                    tx.send(message)
                        .await
                        .expect("server response channel full");
                }
                Ok(response) => {
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
                                            let message = match response {
                                                Ok(chunk) => ServerResponse::Chunk(chunk),
                                                Err(err) => ServerResponse::Error(err.to_string())
                                            };
                                            tx.send(message).await.expect("server response channel full");
                                            break
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
        if ctx.line().len() > 0 {
            Some(Cmd::Interrupt)
        } else {
            Some(Cmd::EndOfFile)
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

    while let Some(_) = rx.blocking_recv() {
        match rl.readline(">> ") {
            Ok(input) => {
                let message = UserRequest::Prompt(input);
                tx.blocking_send(message)
                    .expect("user request channel full");
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

/// Minimal REPL
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "";
    let context_limit = 16000;
    let mut context_size = 0;
    let mut message_buffer: Vec<String> = vec![];
    let mut message_history: VecDeque<Message> = VecDeque::new();
    let mut usage_history: VecDeque<TokenUsage> = VecDeque::new();

    // Channels for all the IPC going on.
    let (start_repl_sender, start_repl_receiver) = tokio::sync::mpsc::channel(2);
    let (user_request_sender, mut user_request_receiver): (
        Sender<UserRequest>,
        Receiver<UserRequest>,
    ) = tokio::sync::mpsc::channel(2);
    let ctrlc_user_request_sender = user_request_sender.clone();
    let (server_request_sender, server_request_receiver): (
        Sender<ServerRequest>,
        Receiver<ServerRequest>,
    ) = tokio::sync::mpsc::channel(2);
    let (server_response_sender, mut server_response_receiver): (
        Sender<ServerResponse>,
        Receiver<ServerResponse>,
    ) = tokio::sync::mpsc::channel(2);

    // Begin background processes.
    thread::spawn(|| repl(start_repl_receiver, user_request_sender));
    tokio::spawn(client(url, server_request_receiver, server_response_sender));
    thread::spawn(|| ctrlc_handler(ctrlc_user_request_sender));

    // Kick-off the user prompt.
    start_repl_sender.send(()).await?;

    // Main loop.
    loop {
        tokio::select! {
            Some(user_request) = user_request_receiver.recv() => {
                let server_request = match user_request {
                    UserRequest::Prompt(input) => {
                        let message = Message{role: MessageRole::User, content: input};
                        message_history.push_back(message);
                        ServerRequest::Start(message_history.clone().into())
                    }
                    UserRequest::Cancel => ServerRequest::Cancel
                };
                server_request_sender.send(server_request).await?;
            }
            Some(server_response) = server_response_receiver.recv() => {
                match server_response {
                    ServerResponse::Chunk(chunk) => {
                        let content = chunk.content;
                        message_buffer.push(content.clone());
                        print!("{content}");
                        if let Some(usage) = chunk.usage {
                            let message = Message{role: MessageRole::Assistant, content: message_buffer.join("")};
                            message_history.push_back(message);
                            usage_history.push_back(usage);
                            message_buffer.clear();
                        }
                    }
                    ServerResponse::Done => start_repl_sender.send(()).await?,
                    ServerResponse::Error(err) => {
                        message_history.pop_back();
                        let content = format!("The following error occurred when receiving a response: {err}");
                        println!("{content}");
                        start_repl_sender.send(()).await?;
                    }
                }
            }
        }

        // Shorten message history to fit context limit.
        while context_size > context_limit {
            if let Some(u) = usage_history.pop_front() {
                context_size -= u.prompt_tokens;
                context_size -= u.completion_tokens;
                message_history = message_history.split_off(2);
            }
        }
    }
}
