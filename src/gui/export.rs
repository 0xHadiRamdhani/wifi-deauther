//! PCAP export functionality

use crate::{DeauthError, Result};
use chrono::{DateTime, Utc};
use pcap::{Capture, Savefile};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tracing::{debug, info};

/// PCAP file exporter
pub struct PcapExporter {
    filename: String,
    start_time: DateTime<Utc>,
}

impl PcapExporter {
    /// Create a new PCAP exporter
    pub fn new(filename: String) -> Self {
        Self {
            filename,
            start_time: Utc::now(),
        }
    }
    
    /// Export captured packets to PCAP file
    pub fn export_packets(&self, packets: &[CapturedPacket]) -> Result<()> {
        info!("Exporting {} packets to {}", packets.len(), self.filename);
        
        let file = File::create(&self.filename)
            .map_err(|e| DeauthError::IoError(e))?;
        
        let mut savefile = Savefile::new(file)
            .map_err(|e| DeauthError::InterfaceError(format!("Failed to create savefile: {}", e)))?;
        
        for packet in packets {
            savefile.write(&packet.data, packet.timestamp);
        }
        
        savefile.flush()
            .map_err(|e| DeauthError::InterfaceError(format!("Failed to flush savefile: {}", e)))?;
        
        info!("Successfully exported {} packets to {}", packets.len(), self.filename);
        Ok(())
    }
    
    /// Export metadata to JSON file
    pub fn export_metadata(&self, metadata: &ExportMetadata) -> Result<()> {
        let json_filename = format!("{}.json", self.filename.trim_end_matches(".pcap"));
        
        info!("Exporting metadata to {}", json_filename);
        
        let json = serde_json::to_string_pretty(metadata)
            .map_err(|e| DeauthError::ConfigError(format!("Failed to serialize metadata: {}", e)))?;
        
        let mut file = File::create(&json_filename)
            .map_err(|e| DeauthError::IoError(e))?;
        
        file.write_all(json.as_bytes())
            .map_err(|e| DeauthError::IoError(e))?;
        
        info!("Successfully exported metadata to {}", json_filename);
        Ok(())
    }
}

/// Captured packet data
#[derive(Debug, Clone)]
pub struct CapturedPacket {
    pub timestamp: std::time::SystemTime,
    pub data: Vec<u8>,
    pub original_length: usize,
}

/// Export metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportMetadata {
    pub export_time: DateTime<Utc>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub packet_count: usize,
    pub total_bytes: usize,
    pub interface: String,
    pub channel: Option<u8>,
    pub filter: Option<String>,
    pub description: String,
}

impl ExportMetadata {
    /// Create new export metadata
    pub fn new(
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        packet_count: usize,
        total_bytes: usize,
        interface: String,
        channel: Option<u8>,
        filter: Option<String>,
        description: String,
    ) -> Self {
        Self {
            export_time: Utc::now(),
            start_time,
            end_time,
            packet_count,
            total_bytes,
            interface,
            channel,
            filter,
            description,
        }
    }
}

/// Export configuration
#[derive(Debug, Clone)]
pub struct ExportConfig {
    pub filename: String,
    pub include_metadata: bool,
    pub compress: bool,
    pub max_packets: Option<usize>,
    pub max_size: Option<usize>,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            filename: format!("capture_{}.pcap", Utc::now().format("%Y%m%d_%H%M%S")),
            include_metadata: true,
            compress: false,
            max_packets: None,
            max_size: None,
        }
    }
}

/// PCAP export manager
pub struct ExportManager {
    config: ExportConfig,
    packets: Vec<CapturedPacket>,
    start_time: DateTime<Utc>,
    total_bytes: usize,
}

impl ExportManager {
    /// Create a new export manager
    pub fn new(config: ExportConfig) -> Self {
        Self {
            config,
            packets: Vec::new(),
            start_time: Utc::now(),
            total_bytes: 0,
        }
    }
    
    /// Add a packet to the export buffer
    pub fn add_packet(&mut self, packet: CapturedPacket) {
        self.total_bytes += packet.data.len();
        self.packets.push(packet);
        
        // Check size limits
        if let Some(max_packets) = self.config.max_packets {
            if self.packets.len() > max_packets {
                self.packets.remove(0);
            }
        }
        
        if let Some(max_size) = self.config.max_size {
            if self.total_bytes > max_size {
                // Remove oldest packets until under limit
                while self.total_bytes > max_size && !self.packets.is_empty() {
                    if let Some(removed) = self.packets.remove(0) {
                        self.total_bytes -= removed.data.len();
                    }
                }
            }
        }
    }
    
    /// Export all buffered packets
    pub fn export(&self) -> Result<()> {
        let exporter = PcapExporter::new(self.config.filename.clone());
        
        // Export packets
        exporter.export_packets(&self.packets)?;
        
        // Export metadata if requested
        if self.config.include_metadata {
            let metadata = ExportMetadata::new(
                self.start_time,
                Utc::now(),
                self.packets.len(),
                self.total_bytes,
                "wlan0".to_string(), // TODO: Get actual interface
                Some(6), // TODO: Get actual channel
                None, // TODO: Get actual filter
                "Wi-Fi Deauther capture".to_string(),
            );
            
            exporter.export_metadata(&metadata)?;
        }
        
        Ok(())
    }
    
    /// Clear buffered packets
    pub fn clear(&mut self) {
        self.packets.clear();
        self.total_bytes = 0;
        self.start_time = Utc::now();
    }
    
    /// Get current packet count
    pub fn packet_count(&self) -> usize {
        self.packets.len()
    }
    
    /// Get total bytes
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_export_metadata() {
        let metadata = ExportMetadata::new(
            Utc::now(),
            Utc::now(),
            100,
            10000,
            "wlan0".to_string(),
            Some(6),
            None,
            "Test capture".to_string(),
        );
        
        assert_eq!(metadata.packet_count, 100);
        assert_eq!(metadata.total_bytes, 10000);
        assert_eq!(metadata.interface, "wlan0");
        assert_eq!(metadata.channel, Some(6));
    }
    
    #[test]
    fn test_export_manager() {
        let config = ExportConfig::default();
        let mut manager = ExportManager::new(config);
        
        let packet = CapturedPacket {
            timestamp: std::time::SystemTime::now(),
            data: vec![0x01, 0x02, 0x03, 0x04],
            original_length: 4,
        };
        
        manager.add_packet(packet);
        
        assert_eq!(manager.packet_count(), 1);
        assert_eq!(manager.total_bytes(), 4);
    }
}