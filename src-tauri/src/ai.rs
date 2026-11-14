use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};

const OLLAMA_URL: &str = "http://localhost:11434/api/chat";
const MODEL: &str = "gemma3:4b";

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
    images: Vec<String>,
}

#[derive(Deserialize)]
struct OllamaResponse {
    message: OllamaMessage,
}

#[derive(Deserialize)]
struct OllamaMessage {
    content: String,
}

pub async fn ask_with_screenshot(
    question: String,
    screenshot: Option<Vec<u8>>,
) -> Result<String, String> {
    let image = screenshot.map(|bytes| {
        general_purpose::STANDARD.encode(bytes)
    });

    let prompt = format!(
        "You are Context, a desktop AI assistant. \
         The user is asking you a question about what they are currently doing on their computer. \
         Use the provided screenshot as visual context when available. \
         Answer directly and concisely. \
         If something in the screenshot is unclear or unreadable, say so instead of guessing. \
         Do not claim that you clicked, typed, changed, or otherwise interacted with anything. \
         You only analyze and explain what you can see.\n\n\
         User question: {}",
        question
    );

    let message = Message {
        role: "user".to_string(),
        content: prompt,
        images: image.into_iter().collect(),
    };

    let request = OllamaRequest {
        model: MODEL.to_string(),
        messages: vec![message],
        stream: false,
    };

    let client = reqwest::Client::new();

    let response = client
        .post(OLLAMA_URL)
        .json(&request)
        .send()
        .await
        .map_err(|error| {
            format!("Failed to connect to Ollama: {}", error)
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        return Err(format!(
            "Ollama returned {}: {}",
            status, body
        ));
    }

    let result: OllamaResponse = response
        .json()
        .await
        .map_err(|error| {
            format!(
                "Failed to parse Ollama response: {}",
                error
            )
        })?;

    Ok(result.message.content)
}