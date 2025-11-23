//! Core deauthentication engine components
//! 
//! This module contains the main engine, packet handling, buffer management,
//! and metrics collection systems.

pub mod engine;
pub mod packet;
pub mod buffer;
pub mod metrics;

pub use engine::DeauthEngine;
pub use packet::{DeauthPacket, MacAddress};
pub use buffer::PacketBuffer;
pub use metrics::{Metrics, MetricsCollector};