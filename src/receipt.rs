//! Optional receipt verification helper for the Igris Inertial Rust SDK.
//!
//! # Example
//!
//! ```no_run
//! use igris_inertial::receipt::verify_receipt;
//! use ed25519_dalek::VerifyingKey;
//!
//! // `resp` is an `InferResponse` returned by `IgrisClient::infer`.
//! // `pub_key_bytes` is the 32-byte Ed25519 public key matching
//! // `IGRIS_RUNTIME_PUBLIC_KEY` on the Runtime.
//!
//! # async fn example(
//! #     resp: igris_inertial::InferResponse,
//! #     pub_key_bytes: &[u8; 32],
//! # ) -> Result<(), Box<dyn std::error::Error>> {
//! if let Some(receipt) = &resp.execution_receipt {
//!     let vk = VerifyingKey::from_bytes(pub_key_bytes)?;
//!     verify_receipt(receipt, &vk)?;
//!     println!("cpu={}ms violation={}", receipt.cpu_time_ms, receipt.violation_occurred);
//! }
//! # Ok(())
//! # }
//! ```

use std::collections::BTreeMap;

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use ed25519_dalek::{Signature, Signer, VerifyingKey};
use sha2::{Digest, Sha256};

use crate::errors::IgrisError;
use crate::types::ExecutionReceipt;

fn api_err(msg: impl Into<String>) -> IgrisError {
    IgrisError::Api { message: msg.into(), status_code: 0 }
}

/// Verify the Ed25519 signature of an [`ExecutionReceipt`].
///
/// The canonical payload is produced by inserting all receipt fields except
/// `signature` into a [`BTreeMap`] (alphabetical key order), serialising it
/// to compact JSON with `serde_json`, and SHA-256 hashing the result. The
/// signature is verified against that hash using the provided [`VerifyingKey`].
///
/// This mirrors the signing procedure in `igris-server/src/receipt.rs` and
/// Overture's `verifySignedJSON` in Go.
///
/// # Errors
///
/// Returns an error if:
/// - the `signature` field is not valid base64,
/// - the decoded bytes are not a valid Ed25519 signature,
/// - the signature does not verify against the canonical payload.
///
/// Returns `Ok(())` on success.
pub fn verify_receipt(
    receipt: &ExecutionReceipt,
    pub_key: &VerifyingKey,
) -> Result<(), IgrisError> {
    let sig_bytes = B64.decode(&receipt.signature)
        .map_err(|e| api_err(format!("receipt: signature base64 decode: {e}")))?;
    let signature = Signature::from_slice(&sig_bytes)
        .map_err(|e| api_err(format!("receipt: invalid signature bytes: {e}")))?;

    let canonical = canonical_payload(receipt)?;
    let hash = Sha256::digest(&canonical);

    pub_key
        .verify_strict(hash.as_slice(), &signature)
        .map_err(|_| api_err("receipt: signature verification failed"))
}

/// Build the canonical JSON payload used for signing/verification.
///
/// All non-`None` fields of the receipt except `signature` are inserted into
/// a `BTreeMap` (which `serde_json` serialises with alphabetically sorted
/// keys). This matches the BTreeMap canonical form produced by the Runtime.
fn canonical_payload(r: &ExecutionReceipt) -> Result<Vec<u8>, IgrisError> {
    // Use serde_json::Value to build a generic map, then re-serialise via
    // BTreeMap to guarantee alphabetical key ordering.
    let raw = serde_json::to_value(r)
        .map_err(|e| api_err(format!("receipt: serialize: {e}")))?;

    let obj = raw
        .as_object()
        .ok_or_else(|| api_err("receipt: not a JSON object"))?;

    let mut btree: BTreeMap<&str, &serde_json::Value> = BTreeMap::new();
    for (k, v) in obj {
        if k != "signature" {
            btree.insert(k.as_str(), v);
        }
    }

    serde_json::to_vec(&btree)
        .map_err(|e| api_err(format!("receipt: canonical serialize: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
    use rand::rngs::OsRng;

    fn build_signed_receipt(sk: &SigningKey) -> ExecutionReceipt {
        let mut receipt = ExecutionReceipt {
            execution_id: "0194f3b2-1a2c-7000-8000-000000000001".into(),
            agent_id: Some("agent-test".into()),
            transaction_id: Some("0194f3b2-1a2c-7000-8000-000000000000".into()),
            transaction_hash: Some("sha256:aabbcc".into()),
            cpu_time_ms: 142,
            wall_time_ms: 380,
            memory_peak_mb: 48,
            fs_bytes_written: 0,
            tool_calls: 3,
            violation_occurred: false,
            timestamp_utc: Some("2026-02-21T10:00:00.000Z".into()),
            previous_hash: Some("sha256:001122".into()),
            hash: "sha256:placeholder".into(),
            signature: String::new(),
        };

        let canonical = canonical_payload(&receipt).unwrap();
        let hash = Sha256::digest(&canonical);
        let sig = sk.sign(hash.as_slice());
        receipt.signature = B64.encode(sig.to_bytes());
        receipt
    }

    #[test]
    fn test_verify_receipt_valid() {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = VerifyingKey::from(&sk);
        let receipt = build_signed_receipt(&sk);
        assert!(verify_receipt(&receipt, &vk).is_ok());
    }

    #[test]
    fn test_verify_receipt_tampered_field() {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = VerifyingKey::from(&sk);
        let mut receipt = build_signed_receipt(&sk);
        receipt.cpu_time_ms = 9999; // tamper after signing
        assert!(verify_receipt(&receipt, &vk).is_err());
    }

    #[test]
    fn test_verify_receipt_wrong_key() {
        let sk = SigningKey::generate(&mut OsRng);
        let wrong_sk = SigningKey::generate(&mut OsRng);
        let wrong_vk = VerifyingKey::from(&wrong_sk);
        let receipt = build_signed_receipt(&sk);
        assert!(verify_receipt(&receipt, &wrong_vk).is_err());
    }

    #[test]
    fn test_verify_receipt_invalid_base64_signature() {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = VerifyingKey::from(&sk);
        let mut receipt = build_signed_receipt(&sk);
        receipt.signature = "!!!not-base64!!!".into();
        assert!(verify_receipt(&receipt, &vk).is_err());
    }

    #[test]
    fn test_infer_response_without_receipt_decodes() {
        let json = r#"{
            "id":"chatcmpl-456",
            "object":"chat.completion",
            "created":1700000000,
            "model":"gpt-4",
            "choices":[{"index":0,"message":{"role":"assistant","content":"hi"},"finish_reason":"stop"}]
        }"#;
        let resp: crate::types::InferResponse = serde_json::from_str(json).unwrap();
        assert!(resp.execution_receipt.is_none());
    }

    #[test]
    fn test_infer_response_with_receipt_decodes() {
        let json = r#"{
            "id":"chatcmpl-123",
            "object":"chat.completion",
            "created":1700000000,
            "model":"gpt-4",
            "choices":[{"index":0,"message":{"role":"assistant","content":"hi"},"finish_reason":"stop"}],
            "execution_receipt":{
                "execution_id":"0194f3b2-0000-7000-8000-000000000001",
                "cpu_time_ms":142,
                "wall_time_ms":380,
                "memory_peak_mb":48,
                "fs_bytes_written":0,
                "tool_calls":3,
                "violation_occurred":false,
                "hash":"sha256:aabb",
                "signature":"dGVzdA=="
            }
        }"#;
        let resp: crate::types::InferResponse = serde_json::from_str(json).unwrap();
        let r = resp.execution_receipt.as_ref().unwrap();
        assert_eq!(r.execution_id, "0194f3b2-0000-7000-8000-000000000001");
        assert_eq!(r.cpu_time_ms, 142);
        assert!(!r.violation_occurred);
    }
}
