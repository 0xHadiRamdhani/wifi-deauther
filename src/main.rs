//! Wi-Fi Deauther - High-performance wireless network testing tool
//! 
//! A modern, cross-platform Wi-Fi deauther featuring:
//! - Parallel packet injection with async I/O
//! - Zero-copy buffer management
//! - Real-time metrics and visualization
//! - Lightweight Slint GUI
//! - Cross-platform support (Linux, Windows, macOS)

use wifi_deauther::{DeauthApp, Result};
use tracing::{info, error};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Wi-Fi Deauther v{}", env!("CARGO_PKG_VERSION"));

    // Check platform compatibility
    if let Err(e) = check_platform_compatibility() {
        error!("Platform compatibility check failed: {}", e);
        return Err(e);
    }

    // Initialize and run the GUI application
    match DeauthApp::new().await {
        Ok(app) => {
            info!("GUI application initialized successfully");
            app.run().await?;
        }
        Err(e) => {
            error!("Failed to initialize GUI application: {}", e);
            return Err(e);
        }
    }

    info!("Wi-Fi Deauther shutdown complete");
    Ok(())
}

fn check_platform_compatibility() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        
        // Check if running as root (required for packet injection)
        if unsafe { libc::geteuid() } != 0 {
            return Err(wifi_deauther::DeauthError::PermissionError(
                "This application requires root privileges for packet injection. Please run with sudo.".to_string()
            ));
        }
        
        // Check for required kernel modules
        let output = Command::new("lsmod")
            .output()
            .map_err(|e| wifi_deauther::DeauthError::PlatformError(format!("Failed to check kernel modules: {}", e)))?;
        
        let modules = String::from_utf8_lossy(&output.stdout);
        if !modules.contains("mac80211") && !modules.contains("cfg80211") {
            tracing::warn!("Wireless kernel modules not loaded. Some features may be limited.");
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        // Check for Npcap/WinPcap
        use std::path::Path;
        if !Path::new("C:\\Windows\\System32\\Npcap.dll").exists() && 
           !Path::new("C:\\Windows\\System32\\wpcap.dll").exists() {
            return Err(wifi_deauther::DeauthError::PlatformError(
                "Npcap or WinPcap not found. Please install Npcap from https://npcap.com/".to_string()
            ));
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        // Check for BPF permissions
        use std::fs;
        if let Ok(entries) = fs::read_dir("/dev") {
            let bpf_devices: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_string_lossy().starts_with("bpf"))
                .collect();
            
            if bpf_devices.is_empty() {
                return Err(wifi_deauther::DeauthError::PlatformError(
                    "No BPF devices found. You may need to load the BPF kernel extension.".to_string()
                ));
            }
        }
    }
    
    Ok(())
}