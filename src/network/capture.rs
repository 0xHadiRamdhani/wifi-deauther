//! Packet capture functionality for monitoring and analysis

use crate::{DeauthError, Result};
use pcap::{Capture, Device};
use std::sync::Arc;
use tracing::{debug, info};

/// Packet capture result
#[derive(Debug, Clone)]
pub struct CaptureResult {
    pub timestamp: std::time::SystemTime,
    pub data: Vec<u8>,
    pub length: usize,
}

/// High-performance packet capture
pub struct PacketCapture {
    capture: Arc<std::sync::Mutex<Capture<pcap::Active>>>,
    interface_name: String,
}

impl PacketCapture {
    /// Create a new packet capture instance
    pub fn new(interface_name: &str) -> Result<Self> {
        info!("Creating packet capture for interface: {}", interface_name);
        
        let device = Device::list()
            .map_err(|e| DeauthError::InterfaceError(format!("Failed to list devices: {}", e)))?
            .into_iter()
            .find(|d| d.name == interface_name)
            .ok_or_else(|| DeauthError::InterfaceError(format!("Interface {} not found", interface_name)))?;
        
        let capture = Capture::from_device(device)
            .map_err(|e| DeauthError::InterfaceError(format!("Failed to create capture: {}", e)))?
            .promisc(true)
            .snaplen(65535)
            .timeout(100)
            .open()
            .map_err(|e| DeauthError::InterfaceError(format!("Failed to open capture: {}", e)))?;
        
        Ok(Self {
            capture: Arc::new(std::sync::Mutex::new(capture)),
            interface_name: interface_name.to_string(),
        })
    }
    
    /// Capture a single packet
    pub fn capture_packet(&self) -> Result<Option<CaptureResult>> {
        let mut capture = self.capture.lock().unwrap();
        
        match capture.next_packet() {
            Ok(packet) => {
                let result = CaptureResult {
                    timestamp: std::time::SystemTime::now(),
                    data: packet.data.to_vec(),
                    length: packet.data.len(),
                };
                
                debug!("Captured packet: {} bytes", result.length);
                Ok(Some(result))
            }
            Err(pcap::Error::TimeoutExpired) => {
                Ok(None)
            }
            Err(e) => {
                Err(DeauthError::InterfaceError(format!("Capture error: {}", e)))
            }
        }
    }
    
    /// Start continuous capture
    pub fn start_capture<F>(&self, mut handler: F) -> Result<()>
    where
        F: FnMut(CaptureResult) -> bool,
    {
        info!("Starting continuous packet capture");
        
        loop {
            match self.capture_packet() {
                Ok(Some(result)) => {
                    if !handler(result) {
                        break;
                    }
                }
                Ok(None) => {
                    // Timeout, continue
                    continue;
                }
                Err(e) => {
                    error!("Capture error: {}", e);
                    break;
                }
            }
        }
        
        info!("Packet capture stopped");
        Ok(())
    }
    
    /// Get capture statistics
    pub fn get_stats(&self) -> Result<CaptureStats> {
        // This would interface with the capture device
        Ok(CaptureStats {
            packets_captured: 0,
            packets_dropped: 0,
            bytes_captured: 0,
        })
    }
    
    /// Stop capture and release resources
    pub fn stop(&self) {
        info!("Stopping packet capture for {}", self.interface_name);
        // Resources will be released when the struct is dropped
    }
}

/// Capture statistics
#[derive(Debug, Clone)]
pub struct CaptureStats {
    pub packets_captured: u64,
    pub packets_dropped: u64,
    pub bytes_captured: u64,
}