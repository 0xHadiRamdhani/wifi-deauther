//! Network interface management and packet injection
//! 
//! This module provides cross-platform network interface management,
//! packet injection, and channel hopping functionality.

pub mod interface;
pub mod injection;
pub mod capture;
pub mod channel;

pub use interface::{NetworkInterface, InterfaceManager};
pub use injection::{PacketInjector, InjectionResult};
pub use capture::{PacketCapture, CaptureResult};
pub use channel::{ChannelHopper, ChannelInfo};