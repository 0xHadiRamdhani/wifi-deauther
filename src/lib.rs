//! High-performance Wi-Fi deauther with parallel packet injection
//! 
//! This library provides a complete implementation of a Wi-Fi deauther
//! featuring async I/O, zero-copy buffers, and a modern Slint GUI.

#![warn(clippy::all)]
#![warn(clippy::pedantic)]

pub mod core;
pub mod network;
pub mod gui;
pub mod platform;

pub use core::{engine::DeauthEngine, metrics::Metrics};
pub use network::{interface::NetworkInterface, injection::PacketInjector};
pub use gui::app::DeauthApp;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DeauthError {
    #[error("Network interface error: {0}")]
    InterfaceError(String),
    
    #[error("Packet injection failed: {0}")]
    InjectionError(String),
    
    #[error("Permission denied: {0}")]
    PermissionError(String),
    
    #[error("Platform not supported: {0}")]
    PlatformError(String),
    
    #[error("Invalid configuration: {0}")]
    ConfigError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("PCAP error: {0}")]
    PcapError(#[from] pcap::Error),
}

pub type Result<T> = std::result::Result<T, DeauthError>;