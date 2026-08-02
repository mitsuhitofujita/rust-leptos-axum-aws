//! Types exchanged between the frontend (`app`) and the API (`server`).
//!
//! This crate must stay free of platform-specific dependencies: it is compiled
//! both for `wasm32-unknown-unknown` and for the server's native target.

use serde::{Deserialize, Serialize};

/// The payload of `GET /api/greeting`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Greeting {
    pub message: String,
}

impl Greeting {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
