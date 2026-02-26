//! Igris Inertial Rust SDK — AI inference gateway client.

pub mod btree;
pub mod client;
pub mod containment;
pub mod errors;
pub mod fleet;
pub mod providers;
pub mod receipt;
pub mod runtime;
pub mod types;
pub mod models;
pub mod usage;
pub mod vault;

pub use btree::{
    action_node, condition_node, selector_node, sequence_node, BTreeDeployResult, BTreeRunOptions,
    BTreeRunResult, BTreeValidateResult, BehaviorTree,
};
pub use client::IgrisClient;
pub use containment::{Bounds, ViolationKind, ViolationRecord};
pub use errors::IgrisError;
pub use models::ModelManager;
pub use receipt::verify_receipt;
pub use runtime::{Runtime, RuntimeBuilder, RuntimeConfig};
pub use types::*;
