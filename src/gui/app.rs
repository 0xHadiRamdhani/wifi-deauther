//! Main GUI application using Slint
//! 
//! This module implements the main application logic that bridges the
//! Slint UI with the core deauthentication engine.

use crate::{core::{DeauthEngine, EngineConfig, Metrics}, network::{InterfaceManager, NetworkInterface}, Result};
use slint::{Model, ModelRc, SharedString, VecModel, Weak};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

slint::include_modules!();

/// Main GUI application
pub struct DeauthApp {
    ui: MainWindow,
    engine: Arc<DeauthEngine>,
    interface_manager: Arc<InterfaceManager>,
    metrics_receiver: broadcast::Receiver<crate::core::engine::MetricsUpdate>,
}

impl DeauthApp {
    /// Create a new GUI application
    pub async fn new() -> Result<Self> {
        info!("Initializing GUI application");
        
        // Create the UI
        let ui = MainWindow::new().map_err(|e| crate::DeauthError::InterfaceError(format!("Failed to create UI: {}", e)))?;
        
        // Create engine with default config
        let config = EngineConfig::default();
        let mut engine = DeauthEngine::new(config)?;
        engine.start()?;
        let engine = Arc::new(engine);
        
        // Create interface manager
        let interface_manager = Arc::new(InterfaceManager::new()?);
        
        // Subscribe to metrics updates
        let metrics_receiver = engine.subscribe_metrics();
        
        let mut app = Self {
            ui,
            engine,
            interface_manager,
            metrics_receiver,
        };
        
        // Setup UI callbacks
        app.setup_callbacks()?;
        
        // Start metrics update task
        app.start_metrics_task();
        
        info!("GUI application initialized successfully");
        Ok(app)
    }
    
    /// Setup UI callbacks
    fn setup_callbacks(&mut self) -> Result<()> {
        let ui_handle = self.ui.as_weak();
        let engine = Arc::clone(&self.engine);
        let interface_manager = Arc::clone(&self.interface_manager);
        
        // Scan button callback
        let scan_handle = ui_handle.clone();
        self.ui.on_scan_clicked(move || {
            let ui = scan_handle.unwrap();
            let engine = Arc::clone(&engine);
            let interface_manager = Arc::clone(&interface_manager);
            
            tokio::spawn(async move {
                info!("Scan button clicked");
                ui.set_is_scanning(true);
                
                match perform_scan(&interface_manager).await {
                    Ok(targets) => {
                        update_target_list(&ui, targets);
                        info!("Scan completed successfully");
                    }
                    Err(e) => {
                        error!("Scan failed: {}", e);
                        // TODO: Show error dialog
                    }
                }
                
                ui.set_is_scanning(false);
            });
        });
        
        // Attack button callback
        let attack_handle = ui_handle.clone();
        let engine_clone = Arc::clone(&self.engine);
        self.ui.on_attack_clicked(move || {
            let ui = attack_handle.unwrap();
            let engine = Arc::clone(&engine_clone);
            
            tokio::spawn(async move {
                info!("Attack button clicked");
                ui.set_is_attacking(true);
                
                match perform_attack(&ui, &engine).await {
                    Ok(_) => {
                        info!("Attack started successfully");
                    }
                    Err(e) => {
                        error!("Attack failed: {}", e);
                        ui.set_is_attacking(false);
                        // TODO: Show error dialog
                    }
                }
            });
        });
        
        // Stop button callback
        let stop_handle = ui_handle.clone();
        let engine_clone = Arc::clone(&self.engine);
        self.ui.on_stop_clicked(move || {
            let ui = stop_handle.unwrap();
            let engine = Arc::clone(&engine_clone);
            
            tokio::spawn(async move {
                info!("Stop button clicked");
                
                match engine.stop_injection().await {
                    Ok(_) => {
                        info!("Attack stopped successfully");
                        ui.set_is_attacking(false);
                    }
                    Err(e) => {
                        error!("Failed to stop attack: {}", e);
                        // TODO: Show error dialog
                    }
                }
            });
        });
        
        // Export callback
        let export_handle = ui_handle.clone();
        self.ui.on_export_clicked(move || {
            let ui = export_handle.unwrap();
            
            tokio::spawn(async move {
                info!("Export button clicked");
                
                match perform_export(&ui).await {
                    Ok(_) => {
                        info!("Export completed successfully");
                    }
                    Err(e) => {
                        error!("Export failed: {}", e);
                        // TODO: Show error dialog
                    }
                }
            });
        });
        
        // Interface change callback
        let interface_handle = ui_handle.clone();
        self.ui.on_interface_changed(move |interface| {
            let ui = interface_handle.unwrap();
            
            tokio::spawn(async move {
                info!("Interface changed to: {}", interface);
                ui.set_selected_interface(interface.as_str());
                // TODO: Update channel list based on interface
            });
        });
        
        // Channel change callback
        let channel_handle = ui_handle.clone();
        self.ui.on_channel_changed(move |channel| {
            let ui = channel_handle.unwrap();
            
            tokio::spawn(async move {
                info!("Channel changed to: {}", channel);
                ui.set_selected_channel(channel);
                // TODO: Update interface channel
            });
        });
        
        // Target selection callback
        let target_handle = ui_handle.clone();
        self.ui.on_target_selected(move |index| {
            let ui = target_handle.unwrap();
            
            tokio::spawn(async move {
                info!("Target selected: {}", index);
                // TODO: Handle target selection
            });
        });
        
        Ok(())
    }
    
    /// Start metrics update task
    fn start_metrics_task(&mut self) {
        let ui_handle = self.ui.as_weak();
        let mut receiver = self.metrics_receiver.resubscribe();
        
        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(update) => {
                        if let Some(ui) = ui_handle.upgrade() {
                            update_ui_metrics(&ui, &update.metrics);
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("Metrics update lagged by {} messages", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        debug!("Metrics channel closed");
                        break;
                    }
                }
            }
        });
    }
    
    /// Run the GUI application
    pub async fn run(self) -> Result<()> {
        info!("Running GUI application");
        
        // Show the UI
        self.ui.run().map_err(|e| crate::DeauthError::InterfaceError(format!("UI error: {}", e)))?;
        
        info!("GUI application stopped");
        Ok(())
    }
}

/// Perform network scan
async fn perform_scan(interface_manager: &Arc<InterfaceManager>) -> Result<Vec<Target>> {
    info!("Performing network scan");
    
    // Get Wi-Fi interfaces
    let interfaces = interface_manager.get_wifi_interfaces();
    if interfaces.is_empty() {
        return Err(crate::DeauthError::InterfaceError("No Wi-Fi interfaces found".to_string()));
    }
    
    // TODO: Implement actual network scanning
    // For now, return mock targets
    let mock_targets = vec![
        Target {
            mac: SharedString::from("AA:BB:CC:DD:EE:FF"),
            ssid: SharedString::from("TestNetwork"),
            channel: 6,
            signal: -45,
            packets: 0,
            status: SharedString::from("Discovered"),
        },
        Target {
            mac: SharedString::from("11:22:33:44:55:66"),
            ssid: SharedString::from("AnotherAP"),
            channel: 1,
            signal: -62,
            packets: 0,
            status: SharedString::from("Discovered"),
        },
    ];
    
    Ok(mock_targets)
}

/// Perform deauthentication attack
async fn perform_attack(ui: &MainWindow, engine: &Arc<DeauthEngine>) -> Result<()> {
    info!("Starting deauthentication attack");
    
    // Get selected targets
    let targets = ui.get_targets();
    if targets.row_count() == 0 {
        return Err(crate::DeauthError::ConfigError("No targets selected".to_string()));
    }
    
    // TODO: Get actual MAC addresses from selected targets
    let target_mac = mac_address::MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    let ap_mac = mac_address::MacAddress::new([0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
    
    // Start injection
    engine.inject_deauth(
        target_mac,
        ap_mac,
        crate::core::packet::reason_codes::UNSPECIFIED,
        100, // packet count
        Duration::from_millis(100), // interval
    ).await?;
    
    Ok(())
}

/// Perform PCAP export
async fn perform_export(ui: &MainWindow) -> Result<()> {
    info!("Exporting PCAP data");
    
    // TODO: Implement actual PCAP export
    // This would involve collecting captured packets and writing to a pcap file
    
    Ok(())
}

/// Update target list in UI
fn update_target_list(ui: &MainWindow, targets: Vec<Target>) {
    let model = Rc::new(VecModel::from(targets));
    ui.set_targets(ModelRc::from(model));
}

/// Update UI metrics
fn update_ui_metrics(ui: &MainWindow, metrics: &Metrics) {
    let ui_metrics = Metrics {
        packets_per_second: metrics.packets_per_second as i32,
        success_rate: metrics.success_rate,
        active_targets: metrics.active_targets as i32,
        channel_utilization: metrics.channel_utilization,
        bytes_transmitted: metrics.bytes_transmitted as i32,
    };
    
    ui.set_metrics(ui_metrics);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_app_creation() {
        // This test would need a headless environment or mock UI
        // For now, just test that the module compiles
        assert!(true);
    }
}