//! Basic usage example for Igris Inertial Rust SDK.

use igris_inertial::{IgrisClient, InferRequest, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = IgrisClient::builder("https://api.igris-inertial.com")
        .api_key("your-api-key")
        .build()?;

    // Simple inference
    let request = InferRequest {
        model: "gpt-4".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: "Hello, world!".to_string(),
            content_parts: None,
        }],
        stream: None,
        max_tokens: Some(100),
        temperature: None,
        top_p: None,
        stop: None,
        policy: None,
        metadata: None,
    };

    let response = client.infer(&request).await?;
    println!("Response: {}", response.choices[0].message.content);

    if let Some(usage) = &response.usage {
        println!("Tokens: {}", usage.total_tokens);
    }

    // Health check
    let health = client.health().await?;
    println!("Gateway status: {}", health.status);

    // List models
    let models = client.list_models().await?;
    println!("Models: {:?}", models.models);

    Ok(())
}
