// Stuff from chatgpt.sh
// TEMPERATURE=${TEMPERATURE:-0.7}
// MAX_TOKENS=${MAX_TOKENS:-1024}
// MODEL=${MODEL:-gpt-3.5-turbo}
// SIZE=${SIZE:-512x512}
// CONTEXT=${CONTEXT:-false}
// MULTI_LINE_PROMPT=${MULTI_LINE_PROMPT:-false}

// curl https://api.openai.com/v1/chat/completions \
// 	-sS \
// 	-H 'Content-Type: application/json' \
// 	-H "Authorization: Bearer $OPENAI_KEY" \
// 	-d '{
//         "model": "'"$MODEL"'",
//         "messages": [
//             {"role": "system", "content": "'"$escaped_system_prompt"'"},
//             '"$message"'
//             ],
//         "max_tokens": '$MAX_TOKENS',
//         "temperature": '$TEMPERATURE'
//         }'

use std::{
    env::{self, VarError},
    io::Write,
};

use eventsource_stream::Eventsource;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc;

// Until we define our error-type
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Req(#[from] reqwest::Error),
    #[error("channel closed")]
    Send(#[from] mpsc::error::SendError<Output>),
    #[error("Missing OPENAI_KEY environment variable")]
    ApiKey(#[from] VarError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Msg {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GptReq {
    model: String,
    messages: Vec<Msg>,
    stream: bool,
    // max_tokens: usize,
    // temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Choice {
    index: i64,
    message: Msg,
    finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Usage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GptRes {
    id: String,
    object: String,
    created: i64,
    model: String,
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Chunk {
    id: String,
    object: String,
    created: i64,
    model: String,
    system_fingerprint: Option<String>,
    choices: Vec<ChunkChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChunkChoice {
    index: usize,
    delta: DeltaMsg,
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeltaMsg {
    content: Option<String>,
    role: Option<String>,
}

pub struct GptClient {
    client: Client,
    messages: Vec<Msg>,
}

#[test]
fn test_chunk() {
    let msg = r#"{"id":"chatcmpl-8UdjQUhf7LF0Pw7YFvm2If9QVLiHo","object":"chat.completion.chunk","created":1702313260,"model":"gpt-3.5-turbo-0613","system_fingerprint":null,"choices":[{"index":0,"delta":{"content":"As"},"finish_reason":null}]}"#;
    let parsed: std::result::Result<Chunk, _> = serde_json::from_str(&msg);
    assert!(parsed.is_ok(), "Error: {}", parsed.unwrap_err());
}

#[derive(Debug)]
pub enum Output {
    Data(String),
    End,
}

impl GptClient {
    pub fn new() -> Self {
        GptClient {
            client: reqwest::Client::new(),
            messages: Vec::new(),
        }
    }

    pub async fn make_request(&mut self, input: String) -> Result<Vec<String>> {
        self.messages.push(Msg {
            role: "user".to_string(),
            content: input,
        });

        let model = env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4-0125-preview".to_string());

        let rq = GptReq {
            model,
            messages: self.messages.clone(),
            stream: true,
        };

        let openai_key = env::var("OPENAI_KEY")?;

        let mut response_stream = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(openai_key)
            .json(&rq)
            .send()
            .await?
            .error_for_status()?
            .bytes_stream()
            .eventsource();

        let mut answer = Vec::new();
        while let Some(item) = response_stream.next().await {
            let event = item.unwrap();
            let parsed: Chunk = match serde_json::from_str(&event.data) {
                Ok(value) => value,
                Err(e) => {
                    if event.data != "[DONE]" {
                        eprintln!("{} could not be parsed: {e}", event.data);
                    }
                    continue;
                }
            };
            for word in parsed.choices.into_iter().flat_map(|c| c.delta.content) {
                print!("{}", word);
                std::io::stdout().flush().unwrap();
                answer.push(word);
            }
        }
        println!("");
        Ok(answer)
    }

    pub async fn event_stream(
        mut self,
        mut input_rx: mpsc::Receiver<String>,
        output_tx: mpsc::Sender<Output>,
    ) -> Result<()> {
        while let Some(input) = input_rx.recv().await {
            self.messages.push(Msg {
                role: "user".to_string(),
                content: input,
            });

            let model =
                env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4-0125-preview".to_string());

            let rq = GptReq {
                model,
                messages: self.messages.clone(),
                stream: true,
            };

            let openai_key = env::var("OPENAI_KEY")?;

            let mut response_stream = self
                .client
                .post("https://api.openai.com/v1/chat/completions")
                .bearer_auth(openai_key)
                .json(&rq)
                .send()
                .await?
                .error_for_status()?
                .bytes_stream()
                .eventsource();

            while let Some(item) = response_stream.next().await {
                let event = item.unwrap();
                let parsed: Chunk = match serde_json::from_str(&event.data) {
                    Ok(value) => value,
                    Err(e) => {
                        if event.data != "[DONE]" {
                            eprintln!("{} could not be parsed: {e}", event.data);
                        }
                        continue;
                    }
                };
                for word in parsed.choices.into_iter().flat_map(|c| c.delta.content) {
                    output_tx.send(Output::Data(word)).await?;
                }
            }
            // Let the outside world know, that chatgpt is done now
            output_tx.send(Output::End).await?;
        }
        Ok(())
    }
}
