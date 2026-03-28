//! btv-log server binary.
//!
//! Usage:
//!   cargo run --manifest-path paper2/Cargo.toml --bin btv-log -- --port 8080
//!
//! Or with a custom Ed25519 key (hex-encoded 32-byte seed via env var):
//!   BTV_LOG_KEY=<hex-seed> cargo run --bin btv-log

use btv_transparency::log_server::{build_router, LogState, SharedLog};
use ed25519_dalek::SigningKey;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("BTV_LOG_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let signing_key = signing_key_from_env();
    let vk = signing_key.verifying_key();

    let log: SharedLog = Arc::new(Mutex::new(LogState::new(signing_key)));
    let app = build_router(log);

    let addr = format!("0.0.0.0:{port}");
    println!("btv-log listening on {addr}");
    println!("Verifying key (pin this in LogClient::new):");
    println!("  {}", encode_hex(vk.as_bytes()));

    println!(
        "sizeof(DeliveryToken)   = {} bytes",
        std::mem::size_of::<btv_transparency::DeliveryToken>()
    );
    println!(
        "sizeof(InclusionReceipt) = {} bytes",
        std::mem::size_of::<btv_transparency::InclusionReceipt>()
    );

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn signing_key_from_env() -> SigningKey {
    if let Ok(hex) = std::env::var("BTV_LOG_KEY") {
        let bytes: Vec<u8> = (0..hex.len())
            .step_by(2)
            .filter_map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
            .collect();
        if bytes.len() == 32 {
            let arr: [u8; 32] = bytes.try_into().unwrap();
            return SigningKey::from_bytes(&arr);
        }
        eprintln!("Warning: BTV_LOG_KEY invalid, generating random key");
    }
    // Generate a random key (ephemeral — for testing only)
    use rand::rngs::OsRng;
    SigningKey::generate(&mut OsRng)
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
