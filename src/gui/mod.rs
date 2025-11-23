//! Slint-based GUI application
//! 
//! This module provides the user interface using Slint framework,
//! featuring real-time metrics, target management, and visualization.

pub mod app;
pub mod targets;
pub mod charts;
pub mod export;

pub use app::DeauthApp;