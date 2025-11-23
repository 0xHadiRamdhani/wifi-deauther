//! Channel hopping and management for Wi-Fi interfaces

use crate::{DeauthError, Result};
use std::collections::HashMap;
use tracing::{debug, info};

/// Wi-Fi channel information
#[derive(Debug, Clone)]
pub struct ChannelInfo {
    pub number: u8,
    pub frequency: u32,
    pub band: WiFiBand,
    pub width: ChannelWidth,
    pub supported: bool,
}

/// Wi-Fi frequency band
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WiFiBand {
    TwoPointFourGHz,
    FiveGHz,
    SixGHz,
}

/// Channel width
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelWidth {
    TwentyMHz,
    FortyMHz,
    EightyMHz,
    OneSixtyMHz,
}

/// Channel hopper for automatic channel scanning
pub struct ChannelHopper {
    channels: Vec<ChannelInfo>,
    current_index: usize,
    dwell_time: std::time::Duration,
}

impl ChannelHopper {
    /// Create a new channel hopper
    pub fn new(band: WiFiBand, dwell_time: std::time::Duration) -> Self {
        let channels = get_channels_for_band(band);
        
        Self {
            channels,
            current_index: 0,
            dwell_time,
        }
    }
    
    /// Get next channel
    pub fn next_channel(&mut self) -> Option<&ChannelInfo> {
        if self.channels.is_empty() {
            return None;
        }
        
        let channel = &self.channels[self.current_index];
        self.current_index = (self.current_index + 1) % self.channels.len();
        
        debug!("Switching to channel {} ({} GHz)", channel.number, channel.frequency as f32 / 1000.0);
        
        Some(channel)
    }
    
    /// Get current channel
    pub fn current_channel(&self) -> Option<&ChannelInfo> {
        self.channels.get(self.current_index)
    }
    
    /// Set dwell time
    pub fn set_dwell_time(&mut self, dwell_time: std::time::Duration) {
        self.dwell_time = dwell_time;
    }
    
    /// Get dwell time
    pub fn dwell_time(&self) -> std::time::Duration {
        self.dwell_time
    }
    
    /// Get all channels
    pub fn channels(&self) -> &[ChannelInfo] {
        &self.channels
    }
}

/// Get channels for a specific band
fn get_channels_for_band(band: WiFiBand) -> Vec<ChannelInfo> {
    match band {
        WiFiBand::TwoPointFourGHz => get_2_4ghz_channels(),
        WiFiBand::FiveGHz => get_5ghz_channels(),
        WiFiBand::SixGHz => get_6ghz_channels(),
    }
}

/// 2.4 GHz channels (1-14)
fn get_2_4ghz_channels() -> Vec<ChannelInfo> {
    let mut channels = Vec::new();
    
    for channel in 1..=14 {
        let frequency = 2412 + (channel - 1) * 5;
        let supported = channel <= 11; // Most countries support 1-11
        
        channels.push(ChannelInfo {
            number: channel,
            frequency,
            band: WiFiBand::TwoPointFourGHz,
            width: ChannelWidth::TwentyMHz,
            supported,
        });
    }
    
    channels
}

/// 5 GHz channels
fn get_5ghz_channels() -> Vec<ChannelInfo> {
    let mut channels = Vec::new();
    
    // Common 5 GHz channels
    let channel_numbers = [36, 40, 44, 48, 52, 56, 60, 64, 100, 104, 108, 112, 116, 120, 124, 128, 132, 136, 140, 144, 149, 153, 157, 161, 165];
    
    for &channel in &channel_numbers {
        let frequency = 5000 + channel * 5;
        
        channels.push(ChannelInfo {
            number: channel,
            frequency,
            band: WiFiBand::FiveGHz,
            width: ChannelWidth::TwentyMHz,
            supported: true,
        });
    }
    
    channels
}

/// 6 GHz channels
fn get_6ghz_channels() -> Vec<ChannelInfo> {
    let mut channels = Vec::new();
    
    // Common 6 GHz channels (Wi-Fi 6E)
    for channel in 1..=233 {
        if channel % 4 == 1 { // Only PSC (Preferred Scanning Channels)
            let frequency = 5945 + channel * 5;
            
            channels.push(ChannelInfo {
                number: channel,
                frequency,
                band: WiFiBand::SixGHz,
                width: ChannelWidth::TwentyMHz,
                supported: true,
            });
        }
    }
    
    channels
}

/// Channel overlap checker
pub fn check_channel_overlap(channel1: u8, channel2: u8, width1: ChannelWidth, width2: ChannelWidth) -> bool {
    let width1_mhz = match width1 {
        ChannelWidth::TwentyMHz => 20,
        ChannelWidth::FortyMHz => 40,
        ChannelWidth::EightyMHz => 80,
        ChannelWidth::OneSixtyMHz => 160,
    };
    
    let width2_mhz = match width2 {
        ChannelWidth::TwentyMHz => 20,
        ChannelWidth::FortyMHz => 40,
        ChannelWidth::EightyMHz => 80,
        ChannelWidth::OneSixtyMHz => 160,
    };
    
    let freq1 = get_channel_frequency(channel1);
    let freq2 = get_channel_frequency(channel2);
    
    if freq1 == 0 || freq2 == 0 {
        return false;
    }
    
    let start1 = freq1 - width1_mhz / 2;
    let end1 = freq1 + width1_mhz / 2;
    let start2 = freq2 - width2_mhz / 2;
    let end2 = freq2 + width2_mhz / 2;
    
    // Check for overlap
    start1 < end2 && end1 > start2
}

/// Get channel frequency in MHz
fn get_channel_frequency(channel: u8) -> u32 {
    if channel >= 1 && channel <= 14 {
        2412 + (channel - 1) * 5
    } else if channel >= 36 && channel <= 165 {
        5000 + channel * 5
    } else if channel >= 1 && channel <= 233 {
        5945 + channel * 5
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_channel_hopper() {
        let mut hopper = ChannelHopper::new(WiFiBand::TwoPointFourGHz, std::time::Duration::from_secs(1));
        
        assert_eq!(hopper.channels().len(), 14);
        
        let first_channel = hopper.next_channel().unwrap();
        assert_eq!(first_channel.number, 1);
        
        let second_channel = hopper.next_channel().unwrap();
        assert_eq!(second_channel.number, 2);
    }
    
    #[test]
    fn test_channel_overlap() {
        // Channel 1 and 6 should not overlap (20 MHz)
        assert!(!check_channel_overlap(1, 6, ChannelWidth::TwentyMHz, ChannelWidth::TwentyMHz));
        
        // Channel 1 and 2 should overlap (40 MHz)
        assert!(check_channel_overlap(1, 2, ChannelWidth::FortyMHz, ChannelWidth::TwentyMHz));
    }
    
    #[test]
    fn test_channel_frequency() {
        assert_eq!(get_channel_frequency(1), 2412);
        assert_eq!(get_channel_frequency(6), 2437);
        assert_eq!(get_channel_frequency(36), 5180);
        assert_eq!(get_channel_frequency(149), 5745);
    }
}