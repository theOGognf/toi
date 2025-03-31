use ctrlc::set_handler;
use futures::stream::TryStreamExt;
use rustyline::error::ReadlineError;
use rustyline::{
    Cmd, ConditionalEventHandler, DefaultEditor, Event, EventContext, EventHandler, KeyEvent,
};
use std::io::Write;
use std::process::exit;
use std::thread::{self, sleep};
use tokio::io::AsyncBufReadExt;
use tokio::sync::mpsc::error::SendError;
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

fn ctrlc_handler(tx: Sender<ServerRequest>) {
    set_handler(move || {
        let message = ServerRequest::Cancel;
        tx.blocking_send(message)
            .expect("server response interrupt channel full");
    });
    thread::park();
}

/// Minimal REPL
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    repl()?;

    Ok(())
}
