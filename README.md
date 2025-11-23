# Wi-Fi Deauther

A high-performance Wi-Fi deauther built in Rust featuring parallel packet injection, async I/O, zero-copy buffers, and a modern Slint GUI. Designed for network testing and security research with real-time metrics and cross-platform support.

## Features

### High Performance
- **Parallel Packet Injection**: Multi-threaded engine with configurable worker pools
- **Zero-Copy Buffers**: Lock-free buffer pool minimizes memory allocations
- **Async I/O**: Non-blocking operations using Tokio runtime
- **Rate Limiting**: Configurable injection rates to prevent network overload

### Real-Time Metrics
- **Live Performance Monitoring**: Packets/second, success rates, latency tracking
- **Channel Utilization**: Real-time spectrum analysis
- **Target Management**: Visual target selection with signal strength indicators
- **Export Capabilities**: PCAP and JSON metadata export

### Modern GUI
- **Slint Framework**: Lightweight, responsive, cross-platform interface
- **Real-Time Charts**: Live visualization of performance metrics
- **Target Tree View**: Easy target selection and management
- **Dark/Light Theme**: Native platform integration

### Cross-Platform
- **Linux**: Raw sockets, netlink, monitor mode support
- **Windows**: WinPcap/Npcap integration
- **macOS**: BPF (Berkeley Packet Filter) support
- **Static Linking**: Single binary distribution with musl

## Quick Start

### Prerequisites

#### Linux
```bash
# Install required packages (Ubuntu/Debian)
sudo apt-get install libpcap-dev build-essential

# For musl static linking
sudo apt-get install musl-tools
```

#### Windows
- Install [Npcap](https://npcap.com/) (includes WinPcap compatibility)
- Install Visual Studio Build Tools or full Visual Studio

#### macOS
```bash
# Install Xcode Command Line Tools
xcode-select --install

# Install libpcap (usually pre-installed)
brew install libpcap
```

### Build from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/wifi-deauther.git
cd wifi-deauther

# Build with optimizations
cargo build --release

# For static linking (Linux)
cargo build --release --target x86_64-unknown-linux-musl
```

### Run

```bash
# Run with root privileges (required for packet injection)
sudo ./target/release/wifi-deauther

# Or with cargo
sudo cargo run --release
```

## Usage

### GUI Mode
1. Launch the application
2. Select your Wi-Fi interface from the dropdown
3. Click "Scan" to discover available targets
4. Select targets from the list
5. Click "Start Attack" to begin deauthentication
6. Monitor real-time metrics and charts
7. Export results as needed

### Command Line Mode (Future)
```bash
# Scan for targets
sudo wifi-deauther scan --interface wlan0

# Attack specific target
sudo wifi-deauther attack --target AA:BB:CC:DD:EE:FF --interface wlan0

# Attack with custom parameters
sudo wifi-deauther attack --target AA:BB:CC:DD:EE:FF --packets 100 --interval 100ms
```

## Architecture

### Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                        Slint GUI                            │
│   ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│   │ Target List │  │ Live Charts  │  │ Export Controls  │   │
│   └─────────────┘  └──────────────┘  └──────────────────┘   │
└─────────────────────────────┬───────────────────────────────┘
                              │
┌─────────────────────────────┴───────────────────────────────┐
│                       Deauth Engine                         │
│   ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│   │   Engine    │  │   Metrics    │  │  Buffer Pool     │   │
│   │  Controller │  │  Collector   │  │  (Zero-Copy)     │   │ 
│   └─────────────┘  └──────────────┘  └──────────────────┘   │
└─────────────────────────────┬───────────────────────────────┘
                              │
┌─────────────────────────────┴───────────────────────────────┐
│                       Network Layer                         │
│   ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│   │  Interface  │  │   Packet     │  │   Channel        │   │
│   │  Manager    │  │  Injector    │  │   Hopper         │   │
│   └─────────────┘  └──────────────┘  └──────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Performance Features

- **Zero-Copy Buffers**: Pre-allocated buffer pool eliminates allocation overhead
- **Lock-Free Queues**: Crossbeam SegQueue for high-throughput packet processing
- **Parallel Workers**: Configurable thread pool for concurrent injection
- **Async Runtime**: Tokio-based non-blocking I/O operations
- **Rate Limiting**: Prevents network congestion and detection

## Configuration

### Engine Configuration
```rust
let config = EngineConfig {
    worker_threads: 4,           // Number of injection workers
    max_rate_per_worker: 1000,   // Max packets/second per worker
    buffer_pool_size: 100,       // Buffer pool size
    buffer_size: 2048,          // Individual buffer size
    rate_limiting: true,        // Enable rate limiting
    max_targets: 50,            // Maximum concurrent targets
};
```

### Export Configuration
```rust
let export_config = ExportConfig {
    filename: "capture.pcap".to_string(),
    include_metadata: true,     // Export JSON metadata
    compress: false,             // Enable compression
    max_packets: Some(10000),  // Limit packet count
    max_size: Some(100_000_000), // Limit file size (100MB)
};
```

## Security Considerations

### Ethical Use
- **Educational Purpose**: Designed for security research and testing
- **Network Testing**: Use only on networks you own or have permission to test
- **Rate Limiting**: Built-in protections against network overload
- **Clear Logging**: All activities are logged for accountability

### Safety Features
- **Permission Checks**: Verifies root/admin privileges
- **Interface Validation**: Ensures proper interface selection
- **Rate Limiting**: Prevents excessive packet injection
- **Clear Documentation**: Usage guidelines and warnings

## Development

### Project Structure
```
wifi-deauther/
├── src/
│   ├── core/          # Core engine and packet handling
│   ├── network/       # Network interface management
│   ├── gui/           # Slint GUI implementation
│   ├── platform/      # Platform-specific code
│   └── main.rs        # Application entry point
├── ui/                # Slint UI definitions
├── Cargo.toml         # Project configuration
└── README.md          # This file
```

### Building for Different Platforms

#### Linux (Static)
```bash
cargo build --release --target x86_64-unknown-linux-musl
strip target/x86_64-unknown-linux-musl/release/wifi-deauther
```

#### Windows
```bash
cargo build --release
```

#### macOS
```bash
cargo build --release
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Disclaimer

**IMPORTANT**: This tool is intended for educational purposes and authorized security testing only. Using this tool on networks you don't own or without explicit permission may be illegal in your jurisdiction. The authors are not responsible for any misuse of this software.

## Acknowledgments

- [Rust](https://www.rust-lang.org/) - Systems programming language
- [Slint](https://slint-ui.com/) - GUI framework
- [libpcap](https://www.tcpdump.org/) - Packet capture library
- [Tokio](https://tokio.rs/) - Async runtime
- [Crossbeam](https://github.com/crossbeam-rs/crossbeam) - Lock-free data structures

## Support

For issues, questions, or contributions, please open an issue on GitHub.