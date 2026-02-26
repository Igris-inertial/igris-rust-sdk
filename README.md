# Igris Inertial Rust SDK

Rust client for [Igris Inertial](https://igris-inertial.com) — an agent execution platform that runs AI workflows inside a Rust runtime with deterministic containment, Ed25519-signed execution receipts, and multi-provider inference routing.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
igris-inertial = "0.1"
tokio = { version = "1", features = ["full"] }
```

## Quick Start

```rust
use igris_inertial::{IgrisClient, InferRequest, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = IgrisClient::builder("https://api.igris-inertial.com")
        .api_key("your-api-key")
        .build()?;

    let response = client.infer(&InferRequest {
        model: "gpt-4".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: "Hello!".to_string(),
            content_parts: None,
        }],
        ..Default::default()
    }).await?;

    println!("{}", response.choices[0].message.content);
    Ok(())
}
```

## Features

- **Async-first** — Built on tokio and reqwest
- **Strong typing** — Serde-derived (de)serialization for all types
- **Deterministic agent execution** — Agents run inside the Igris Runtime with enforced CPU, memory, and I/O containment bounds
- **Cryptographic execution receipts** — Ed25519-signed resource-accounting data with hash-chained tamper evidence; opt-in verification with `verify_receipt`
- **Multi-provider inference routing** — Route across OpenAI, Anthropic, Google, and more with SLO-based fallback
- **BYOK vault** — Securely store and rotate your own provider API keys
- **Fleet management** — Register and monitor agent fleets
- **Usage tracking** — Monitor costs and token usage

## API

```rust
// Inference
client.infer(&request).await?;
client.chat_completion(&request).await?;
client.list_models().await?;
client.health().await?;

// Providers
client.providers().register(&config).await?;
client.providers().list().await?;
client.providers().test(&config).await?;
client.providers().health("id").await?;
client.providers().delete("id").await?;

// Vault
client.vault().store(&request).await?;
client.vault().list().await?;
client.vault().rotate("openai").await?;
client.vault().delete("openai").await?;

// Fleet
client.fleet().register(&config).await?;
client.fleet().agents().await?;
client.fleet().health().await?;

// Usage & Audit
client.usage().current().await?;
client.usage().history().await?;
client.audit().list().await?;
```

## Execution Receipt Verification (v2.2.0+)

When Overture is backed by a Runtime instance, inference responses include an
`execution_receipt` with signed resource-accounting data. Verification is
opt-in.

```rust
use igris_inertial::{verify_receipt, IgrisClient};
use ed25519_dalek::VerifyingKey;

let resp = client.infer(req).await?;

if let Some(receipt) = &resp.execution_receipt {
    let pub_key_bytes: [u8; 32] = hex::decode("...") // IGRIS_RUNTIME_PUBLIC_KEY
        .unwrap().try_into().unwrap();
    let vk = VerifyingKey::from_bytes(&pub_key_bytes)?;

    verify_receipt(receipt, &vk)?;
    println!("cpu={}ms violation={}", receipt.cpu_time_ms, receipt.violation_occurred);
}
```

`verify_receipt` is never called automatically. Responses from servers that do
not emit receipts decode normally — `execution_receipt` will be `None`.

## Changelog

### 2.2.0
- Added `ExecutionReceipt` struct to response types
- Added `execution_receipt` optional field to `InferResponse`
- Added `verify_receipt(receipt, &VerifyingKey) -> Result<()>` helper (ed25519-dalek + sha2)

## License

MIT
