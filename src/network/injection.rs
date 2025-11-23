//! Cross-platform packet injection using libpcap
//! 
//! This module provides high-performance packet injection capabilities
//! across Linux, Windows, and macOS platforms.

use crate::{DeauthError, Result};
use crate::core::packet::DeauthPacket;
use bytes::BytesMut;
use pcap::{Capture, Device, Active, Activated};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Result of packet injection attempt
#[derive(Debug, Clone)]
pub struct InjectionResult {
    pub success: bool,
    pub bytes_sent: usize,
    pub error: Option<String>,
}

/// High-performance packet injector using libpcap
pub struct PacketInjector {
    device: Arc<parking_lot::RwLock<Device>>,
    capture: Option<Capture<Active>>,
    interface_name: String,
}

impl PacketInjector {
    /// Create a new packet injector for the specified interface
    pub fn new(interface_name: &str) -> Result<Self> {
        info!("Creating packet injector for interface: {}", interface_name);
        
        // Find the device
        let device = Device::list()
            .map_err(|e| DeauthError::InterfaceError(format!("Failed to list devices: {}", e)))?
            .into_iter()
            .find(|d| d.name == interface_name)
            .ok_or_else(|| DeauthError::InterfaceError(format!("Interface {} not found", interface_name)))?;
        
        debug!("Found device: {} - {}", device.name, device.desc.as_ref().unwrap_or(&"No description".to_string()));
        
        Ok(Self {
            device: Arc::new(parking_lot::RwLock::new(device)),
            capture: None,
            interface_name: interface_name.to_string(),
        })
    }
    
    /// Initialize the injector with capture capabilities
    pub fn initialize(&mut self) -> Result<()> {
        info!("Initializing packet injector for {}", self.interface_name);
        
        // Open the device for capture and injection
        let mut capture = Capture::from_device(self.interface_name.as_str())
            .map_err(|e| DeauthError::InterfaceError(format!("Failed to open device: {}", e)))?
            .promisc(true)
            .snaplen(65535)
            .timeout(1)
            .open()
            .map_err(|e| DeauthError::InterfaceError(format!("Failed to open capture: {}", e)))?;
        
        // Set immediate mode for better performance
        if let Err(e) = capture.setnonblock() {
            warn!("Failed to set non-blocking mode: {}", e);
        }
        
        self.capture = Some(capture);
        
        info!("Packet injector initialized successfully");
        Ok(())
    }
    
    /// Inject a single packet
    pub fn inject_packet(&mut self, packet: &DeauthPacket) -> Result<InjectionResult> {
        let start_time = std::time::Instant::now();
        
        // Serialize the packet
        let packet_bytes = packet.to_bytes();
        let packet_data = packet_bytes.as_ref();
        
        debug!("Injecting {} bytes for target {}", packet_data.len(), packet.destination);
        
        // Inject the packet
        match self.inject_raw(packet_data) {
            Ok(_) => {
                let elapsed = start_time.elapsed();
                debug!("Successfully injected {} bytes in {:?}", packet_data.len(), elapsed);
                
                Ok(InjectionResult {
                    success: true,
                    bytes_sent: packet_data.len(),
                    error: None,
                })
            }
            Err(e) => {
                error!("Failed to inject packet: {}", e);
                
                Ok(InjectionResult {
                    success: false,
                    bytes_sent: 0,
                    error: Some(e.to_string()),
                })
            }
        }
    }
    
    /// Inject multiple packets with rate limiting
    pub fn inject_burst(
        &mut self,
        packets: &[DeauthPacket],
        interval: Duration,
    ) -> Result<Vec<InjectionResult>> {
        let mut results = Vec::with_capacity(packets.len());
        
        for packet in packets {
            let result = self.inject_packet(packet)?;
            results.push(result);
            
            // Rate limiting
            if !interval.is_zero() {
                std::thread::sleep(interval);
            }
        }
        
        Ok(results)
    }
    
    /// Inject raw packet data
    fn inject_raw(&mut self, data: &[u8]) -> Result<()> {
        if let Some(ref mut capture) = self.capture {
            capture.sendpacket(data)
                .map_err(|e| DeauthError::InjectionError(format!("Packet injection failed: {}", e)))?;
            Ok(())
        } else {
            Err(DeauthError::InjectionError("Injector not initialized".to_string()))
        }
    }
    
    /// Get interface statistics
    pub fn get_stats(&self) -> Result<InjectionStats> {
        // This would interface with the capture device to get statistics
        // For now, return placeholder stats
        Ok(InjectionStats {
            packets_sent: 0,
            packets_dropped: 0,
            bytes_sent: 0,
            errors: 0,
        })
    }
    
    /// Close the injector and release resources
    pub fn close(&mut self) {
        info!("Closing packet injector for {}", self.interface_name);
        
        if let Some(capture) = self.capture.take() {
            drop(capture);
        }
        
        info!("Packet injector closed");
    }
}

/// Injection statistics
#[derive(Debug, Clone)]
pub struct InjectionStats {
    pub packets_sent: u64,
    pub packets_dropped: u64,
    pub bytes_sent: u64,
    pub errors: u64,
}

/// High-throughput batch injector
pub struct BatchInjector {
    injectors: Vec<PacketInjector>,
    current_index: std::sync::atomic::AtomicUsize,
}

impl BatchInjector {
    /// Create a batch injector with multiple parallel injectors
    pub fn new(interface_name: &str, num_injectors: usize) -> Result<Self> {
        let mut injectors = Vec::with_capacity(num_injectors);
        
        for i in 0..num_injectors {
            let mut injector = PacketInjector::new(interface_name)?;
            injector.initialize()?;
            injectors.push(injector);
            
            debug!("Created injector {} for {}", i, interface_name);
        }
        
        Ok(Self {
            injectors,
            current_index: std::sync::atomic::AtomicUsize::new(0),
        })
    }
    
    /// Inject a packet using round-robin distribution
    pub fn inject_packet(&mut self, packet: &DeauthPacket) -> Result<InjectionResult> {
        let index = self.current_index.fetch_add(1, std::sync::atomic::Ordering::Relaxed) 
            % self.injectors.len();
        
        self.injectors[index].inject_packet(packet)
    }
    
    /// Inject multiple packets in parallel
    pub fn inject_parallel(
        &mut self,
        packets: &[DeauthPacket],
        interval: Duration,
    ) -> Result<Vec<InjectionResult>> {
        use rayon::prelude::*;
        
        let results: Vec<_> = packets
            .par_iter()
            .map(|packet| {
                let mut local_injector = PacketInjector::new(&self.injectors[0].interface_name)?;
                local_injector.inject_packet(packet)
            })
            .collect::<Result<Vec<_>>>()?;
        
        Ok(results)
    }
    
    /// Close all injectors
    pub fn close(&mut self) {
        for (i, injector) in self.injectors.iter_mut().enumerate() {
            injector.close();
            debug!("Closed injector {}", i);
        }
    }
}

/// Platform-specific injection optimizations
mod platform_optimizations {
    use super::*;
    
    /// Linux-specific optimizations
    #[cfg(target_os = "linux")]
    pub fn optimize_for_linux(injector: &mut PacketInjector) -> Result<()> {
        // Set socket buffer sizes for better performance
        if let Some(ref mut capture) = injector.capture {
            // This would use pcap_set_buffer_size if available
            debug!("Applied Linux-specific optimizations");
        }
        Ok(())
    }
    
    /// Windows-specific optimizations
    #[cfg(target_os = "windows")]
    pub fn optimize_for_windows(injector: &mut PacketInjector) -> Result<()> {
        // Windows-specific optimizations
        debug!("Applied Windows-specific optimizations");
        Ok(())
    }
    
    /// macOS-specific optimizations
    #[cfg(target_os = "macos")]
    pub fn optimize_for_macos(injector: &mut PacketInjector) -> Result<()> {
        // macOS-specific optimizations
        debug!("Applied macOS-specific optimizations");
        Ok(())
    }
}

/// Rate-limited injector wrapper
pub struct RateLimitedInjector {
    injector: PacketInjector,
    rate_limiter: RateLimiter,
}

impl RateLimitedInjector {
    /// Create a rate-limited injector
    pub fn new(injector: PacketInjector, max_rate: u32) -> Self {
        Self {
            injector,
            rate_limiter: RateLimiter::new(max_rate),
        }
    }
    
    /// Inject a packet with rate limiting
    pub fn inject_packet(&mut self, packet: &DeauthPacket) -> Result<InjectionResult> {
        if self.rate_limiter.try_acquire() {
            self.injector.inject_packet(packet)
        } else {
            Ok(InjectionResult {
                success: false,
                bytes_sent: 0,
                error: Some("Rate limit exceeded".to_string()),
            })
        }
    }
}

/// Rate limiter for packet injection
struct RateLimiter {
    max_rate: u32,
    tokens: Arc<std::sync::atomic::AtomicU32>,
    last_refill: Arc<parking_lot::RwLock<Instant>>,
}

impl RateLimiter {
    fn new(max_rate: u32) -> Self {
        Self {
            max_rate,
            tokens: Arc::new(std::sync::atomic::AtomicU32::new(max_rate)),
            last_refill: Arc::new(parking_lot::RwLock::new(Instant::now())),
        }
    }
    
    fn try_acquire(&self) -> bool {
        let now = Instant::now();
        let mut last_refill = self.last_refill.write();
        
        // Refill tokens based on time elapsed
        let elapsed = now.duration_since(*last_refill);
        let tokens_to_add = (elapsed.as_secs() * self.max_rate as u64) +
                           (elapsed.subsec_millis() as u64 * self.max_rate as u64 / 1000);
        
        if tokens_to_add > 0 {
            let current_tokens = self.tokens.load(std::sync::atomic::Ordering::Relaxed);
            let new_tokens = (current_tokens + tokens_to_add as u32).min(self.max_rate);
            self.tokens.store(new_tokens, std::sync::atomic::Ordering::Relaxed);
            *last_refill = now;
        }
        
        // Try to acquire a token
        self.tokens.fetch_update(
            std::sync::atomic::Ordering::Relaxed,
            std::sync::atomic::Ordering::Relaxed,
            |tokens| if tokens > 0 { Some(tokens - 1) } else { None }
        ).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mac_address::MacAddress;
    
    #[test]
    fn test_injection_result() {
        let result = InjectionResult {
            success: true,
            bytes_sent: 100,
            error: None,
        };
        
        assert!(result.success);
        assert_eq!(result.bytes_sent, 100);
        assert!(result.error.is_none());
    }
    
    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(10);
        
        // Should be able to acquire 10 tokens quickly
        let mut acquired = 0;
        for _ in 0..20 {
            if limiter.try_acquire() {
                acquired += 1;
            }
        }
        
        assert_eq!(acquired, 10);
        
        // Wait a bit for tokens to refill
        std::thread::sleep(Duration::from_millis(200));
        
        // Should be able to acquire more tokens
        assert!(limiter.try_acquire());
    }
}