# Wi-Fi Deauther Project Plan

## Project Structure
```
wifi-deauther/
├── Cargo.toml                 # Main project configuration
├── build.rs                   # Build script for musl compilation
├── src/
│   ├── main.rs               # Entry point and CLI
│   ├── lib.rs                # Library root
│   ├── core/
│   │   ├── mod.rs            # Core module exports
│   │   ├── engine.rs         # Main deauth engine
│   │   ├── packet.rs         # 802.11 frame handling
│   │   ├── buffer.rs         # Zero-copy buffer management
│   │   └── metrics.rs        # Performance metrics
│   ├── network/
│   │   ├── mod.rs            # Network module exports
│   │   ├── interface.rs      # Network interface management
│   │   ├── injection.rs      # Packet injection logic
│   │   ├── capture.rs        # Packet capture logic
│   │   └── channel.rs        # Channel hopping
│   ├── gui/
│   │   ├── mod.rs            # GUI module exports
│   │   ├── app.rs            # Main GUI application
│   │   ├── targets.rs        # Target management UI
│   │   ├── charts.rs         # Real-time charts
│   │   └── export.rs         # Export functionality
│   └── platform/
│       ├── mod.rs            # Platform-specific code
│       ├── linux.rs          # Linux implementation
│       ├── windows.rs        # Windows implementation
│       └── macos.rs          # macOS implementation
├── ui/
│   └── app.slint             # Slint UI definition
├── .cargo/
│   └── config.toml           # Cargo configuration for musl
└── README.md                 # Documentation
```

## Cargo.toml Configuration
```toml
[package]
name = "wifi-deauther"
version = "0.1.0"
edition = "2021"
authors = ["Wi-Fi Deauther Team"]
description = "High-performance Wi-Fi deauther with modern GUI"
license = "MIT"

[dependencies]
# Async runtime
tokio = { version = "1.0", features = ["full"] }

# GUI framework
slint = "1.0"

# Networking
pcap = "1.0"
pnet = "0.34"
wifi-frames = "0.3"

# Performance
crossbeam = "0.8"
parking_lot = "0.12"
bytes = "1.5"

# Data structures
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Charts
plotters = "0.3"

# Time
chrono = "0.4"

[build-dependencies]
slint-build = "1.0"

[profile.release]
lto = true
codegen-units = 1
strip = true
opt-level = "z"
panic = "abort"

[target.'cfg(target_os = "linux")'.dependencies]
nix = "0.27"

[target.'cfg(target_os = "windows")'.dependencies]
winapi = { version = "0.3", features = ["winsock2"] }

[target.'cfg(target_os = "macos")'.dependencies]
core-foundation = "0.9"
```

## Core Implementation Strategy

### 1. Zero-Copy Buffer System
```rust
use bytes::{Bytes, BytesMut};
use crossbeam::queue::ArrayQueue;

pub struct PacketBuffer {
    pool: ArrayQueue<BytesMut>,
    size: usize,
}

impl PacketBuffer {
    pub fn new(pool_size: usize, buffer_size: usize) -> Self {
        // Pre-allocate buffer pool
    }
    
    pub fn acquire(&self) -> Option<BytesMut> {
        // Get buffer from pool
    }
    
    pub fn release(&self, buffer: BytesMut) {
        // Return buffer to pool
    }
}
```

### 2. Async Packet Engine
```rust
use tokio::sync::{mpsc, broadcast};
use std::sync::Arc;

pub struct PacketEngine {
    injector_tx: mpsc::Sender<InjectionRequest>,
    metrics_rx: broadcast::Receiver<MetricsUpdate>,
    worker_pool: Arc<ThreadPool>,
}

impl PacketEngine {
    pub async fn inject_deauth(&self, target: MacAddress, ap: MacAddress) -> Result<()> {
        // Async deauth injection
    }
    
    pub async fn start_monitoring(&self, interface: String) -> Result<()> {
        // Start packet monitoring
    }
}
```

### 3. Parallel Injection System
```rust
use crossbeam::queue::SegQueue;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct InjectionWorker {
    request_queue: Arc<SegQueue<InjectionRequest>>,
    packet_count: Arc<AtomicU64>,
    rate_limiter: RateLimiter,
}

impl InjectionWorker {
    pub fn run(&self) {
        // High-performance packet injection loop
        while let Some(request) = self.request_queue.pop() {
            self.inject_packet(request);
            self.packet_count.fetch_add(1, Ordering::Relaxed);
        }
    }
}
```

### 4. Slint GUI Structure
```slint
// ui/app.slint
import { Button, ListView, Chart } from "std-widgets.slint";

export component MainWindow inherits Window {
    title: "Wi-Fi Deauther";
    preferred-width: 800px;
    preferred-height: 600px;
    
    property<[Target]> targets;
    property<Metrics> live-metrics;
    
    VerticalLayout {
        HeaderBar {}
        HorizontalLayout {
            TargetList { targets: root.targets; }
            MetricsPanel { metrics: root.live-metrics; }
        }
        ControlPanel {}
    }
}
```

## Performance Targets
- **Packet Injection Rate**: 1000+ packets/second
- **Memory Usage**: <50MB for GUI + engine
- **CPU Usage**: <10% on modern hardware
- **Binary Size**: <3MB stripped
- **Startup Time**: <500ms

## Cross-Platform Implementation
- **Linux**: Raw sockets, netlink, monitor mode
- **Windows**: Npcap/WinPcap, Windows API
- **macOS**: BPF, ioctl interfaces

## Security Features
- Interface capability checking
- Rate limiting (max 100 packets/second per target)
- Clear logging and audit trail
- Educational use disclaimer

## Build Configuration
```toml
# .cargo/config.toml
[target.x86_64-unknown-linux-musl]
linker = "x86_64-linux-musl-gcc"

[build]
target = "x86_64-unknown-linux-musl"
```

## Testing Strategy
1. Unit tests for core components
2. Integration tests for packet injection
3. GUI automation tests
4. Cross-platform compatibility tests
5. Performance benchmarks

## Deployment
- Single binary distribution
- Static linking with musl
- Stripped symbols
- Compressed with UPX (optional)