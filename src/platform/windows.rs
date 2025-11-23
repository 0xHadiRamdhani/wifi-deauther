//! Windows platform implementation

use super::{Platform, PlatformCapabilities};
use crate::Result;

pub struct WindowsPlatform;

impl WindowsPlatform {
    pub fn new() -> Self {
        Self
    }
}

impl Platform for WindowsPlatform {
    fn name(&self) -> &str {
        "Windows"
    }
    
    fn is_supported(&self) -> bool {
        true
    }
    
    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities {
            raw_socket_support: true,
            monitor_mode_support: false,
            bpf_support: false,
            netlink_support: false,
            winpcap_support: true,
        }
    }
}