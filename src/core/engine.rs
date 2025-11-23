//! High-performance deauthentication engine with parallel packet injection
//! 
//! This module implements the core deauthentication engine featuring:
//! - Parallel packet injection with thread pool
//! - Async I/O for non-blocking operations
//! - Rate limiting and flow control
//! - Real-time metrics collection

use super::{buffer::PacketBuffer, metrics::MetricsCollector, packet::DeauthPacket};
use crate::{DeauthError, Result};
use bytes::BytesMut;
use crossbeam::queue::SegQueue;
use mac_address::MacAddress;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, oneshot};
use tracing::{debug, error, info, warn};

/// Injection request for the worker pool
#[derive(Debug, Clone)]
pub struct InjectionRequest {
    pub target: MacAddress,
    pub access_point: MacAddress,
    pub reason_code: u16,
    pub count: u32,
    pub interval: Duration,
}

/// Engine configuration
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Number of worker threads
    pub worker_threads: usize,
    
    /// Maximum injection rate per worker (packets/second)
    pub max_rate_per_worker: u32,
    
    /// Buffer pool size
    pub buffer_pool_size: usize,
    
    /// Buffer size in bytes
    pub buffer_size: usize,
    
    /// Metrics window size
    pub metrics_window: usize,
    
    /// Enable rate limiting
    pub rate_limiting: bool,
    
    /// Maximum concurrent targets
    pub max_targets: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            worker_threads: 4,
            max_rate_per_worker: 1000,
            buffer_pool_size: 100,
            buffer_size: 2048,
            metrics_window: 100,
            rate_limiting: true,
            max_targets: 50,
        }
    }
}

/// High-performance deauthentication engine
pub struct DeauthEngine {
    /// Configuration
    config: EngineConfig,
    
    /// Buffer pool for zero-copy operations
    buffer_pool: Arc<PacketBuffer>,
    
    /// Metrics collector
    metrics_collector: Arc<MetricsCollector>,
    
    /// Injection request queue
    request_queue: Arc<SegQueue<InjectionRequest>>,
    
    /// Worker thread handles
    workers: Vec<thread::JoinHandle<()>>,
    
    /// Engine control
    running: Arc<AtomicBool>,
    
    /// Channel for async communication
    control_tx: mpsc::Sender<EngineCommand>,
    control_rx: Arc<RwLock<mpsc::Receiver<EngineCommand>>>,
    
    /// Metrics broadcast channel
    metrics_tx: broadcast::Sender<MetricsUpdate>,
}

/// Engine control commands
#[derive(Debug)]
enum EngineCommand {
    StartInjection(InjectionRequest),
    StopInjection,
    Shutdown,
    GetMetrics(oneshot::Sender<MetricsUpdate>),
}

/// Metrics update message
#[derive(Debug, Clone)]
pub struct MetricsUpdate {
    pub timestamp: Instant,
    pub metrics: super::metrics::Metrics,
}

impl DeauthEngine {
    /// Create a new deauthentication engine
    pub fn new(config: EngineConfig) -> Result<Self> {
        let buffer_pool = Arc::new(PacketBuffer::new(
            config.buffer_pool_size,
            config.buffer_size,
        ));
        
        let metrics_collector = Arc::new(MetricsCollector::new(config.metrics_window));
        let request_queue = Arc::new(SegQueue::new());
        let running = Arc::new(AtomicBool::new(true));
        
        let (control_tx, control_rx) = mpsc::channel(100);
        let (metrics_tx, _) = broadcast::channel(10);
        
        Ok(Self {
            config,
            buffer_pool,
            metrics_collector,
            request_queue: Arc::clone(&request_queue),
            workers: Vec::new(),
            running,
            control_tx,
            control_rx: Arc::new(RwLock::new(control_rx)),
            metrics_tx,
        })
    }
    
    /// Start the engine and worker threads
    pub fn start(&mut self) -> Result<()> {
        info!("Starting deauthentication engine with {} workers", self.config.worker_threads);
        
        for worker_id in 0..self.config.worker_threads {
            let worker = self.spawn_worker(worker_id)?;
            self.workers.push(worker);
        }
        
        // Start metrics collection task
        self.start_metrics_task();
        
        info!("Deauthentication engine started successfully");
        Ok(())
    }
    
    /// Spawn a worker thread
    fn spawn_worker(&self, worker_id: usize) -> Result<thread::JoinHandle<()>> {
        let request_queue = Arc::clone(&self.request_queue);
        let buffer_pool = Arc::clone(&self.buffer_pool);
        let metrics_collector = Arc::clone(&self.metrics_collector);
        let running = Arc::clone(&self.running);
        let max_rate = self.config.max_rate_per_worker;
        
        let handle = thread::spawn(move || {
            info!("Worker {} started", worker_id);
            
            let mut last_injection = Instant::now();
            let min_interval = Duration::from_micros(1_000_000 / max_rate as u64);
            
            while running.load(Ordering::Relaxed) {
                if let Some(request) = request_queue.pop() {
                    let start_time = Instant::now();
                    
                    // Rate limiting
                    if start_time.duration_since(last_injection) < min_interval {
                        thread::sleep(min_interval - start_time.duration_since(last_injection));
                    }
                    
                    // Process the injection request
                    match process_injection_request(&request, &buffer_pool) {
                        Ok(bytes_sent) => {
                            let latency = start_time.elapsed();
                            metrics_collector.record_injection(bytes_sent, true, latency);
                            debug!("Worker {}: Injected {} bytes to {} in {:?}", 
                                   worker_id, bytes_sent, request.target, latency);
                        }
                        Err(e) => {
                            let latency = start_time.elapsed();
                            metrics_collector.record_injection(0, false, latency);
                            warn!("Worker {}: Injection failed: {}", worker_id, e);
                        }
                    }
                    
                    last_injection = Instant::now();
                } else {
                    // No work available, yield CPU
                    thread::yield_now();
                }
            }
            
            info!("Worker {} stopped", worker_id);
        });
        
        Ok(handle)
    }
    
    /// Start metrics collection background task
    fn start_metrics_task(&self) {
        let metrics_collector = Arc::clone(&self.metrics_collector);
        let metrics_tx = self.metrics_tx.clone();
        let running = Arc::clone(&self.running);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));
            
            while running.load(Ordering::Relaxed) {
                interval.tick().await;
                
                let metrics = metrics_collector.calculate_metrics();
                let update = MetricsUpdate {
                    timestamp: Instant::now(),
                    metrics,
                };
                
                if let Err(e) = metrics_tx.send(update) {
                    debug!("No metrics subscribers: {}", e);
                }
            }
        });
    }
    
    /// Submit an injection request
    pub async fn inject_deauth(
        &self,
        target: MacAddress,
        access_point: MacAddress,
        reason_code: u16,
        count: u32,
        interval: Duration,
    ) -> Result<()> {
        let request = InjectionRequest {
            target,
            access_point,
            reason_code,
            count,
            interval,
        };
        
        self.control_tx.send(EngineCommand::StartInjection(request))
            .await
            .map_err(|e| DeauthError::InjectionError(format!("Failed to submit request: {}", e)))?;
        
        Ok(())
    }
    
    /// Stop all injections
    pub async fn stop_injection(&self) -> Result<()> {
        self.control_tx.send(EngineCommand::StopInjection)
            .await
            .map_err(|e| DeauthError::InjectionError(format!("Failed to stop injection: {}", e)))?;
        
        Ok(())
    }
    
    /// Get current metrics
    pub async fn get_metrics(&self) -> Result<super::metrics::Metrics> {
        let (tx, rx) = oneshot::channel();
        
        self.control_tx.send(EngineCommand::GetMetrics(tx))
            .await
            .map_err(|e| DeauthError::InjectionError(format!("Failed to get metrics: {}", e)))?;
        
        let update = rx.await
            .map_err(|e| DeauthError::InjectionError(format!("Metrics request failed: {}", e)))?;
        
        Ok(update.metrics)
    }
    
    /// Subscribe to metrics updates
    pub fn subscribe_metrics(&self) -> broadcast::Receiver<MetricsUpdate> {
        self.metrics_tx.subscribe()
    }
    
    /// Shutdown the engine
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down deauthentication engine");
        
        self.running.store(false, Ordering::Relaxed);
        
        self.control_tx.send(EngineCommand::Shutdown)
            .await
            .map_err(|e| DeauthError::InjectionError(format!("Failed to shutdown: {}", e)))?;
        
        // Wait for workers to finish
        for worker in &self.workers {
            if let Err(e) = worker.thread().unpark() {
                error!("Failed to unpark worker: {}", e);
            }
        }
        
        info!("Deauthentication engine shutdown complete");
        Ok(())
    }
}

/// Process a single injection request
fn process_injection_request(
    request: &InjectionRequest,
    buffer_pool: &Arc<PacketBuffer>,
) -> Result<usize> {
    let mut total_bytes = 0;
    
    // Get buffer from pool
    let mut buffer = buffer_pool.acquire()
        .ok_or_else(|| DeauthError::InjectionError("Buffer pool exhausted".to_string()))?;
    
    // Create deauth packet
    let packet = DeauthPacket::new(
        request.target,
        request.access_point,
        request.access_point,
        request.reason_code,
    );
    
    // Serialize packet
    let packet_bytes = packet.to_bytes();
    let packet_size = packet_bytes.len();
    
    // Simulate packet injection (this would be replaced with actual network code)
    // For now, we'll just log and return success
    debug!("Would inject {} bytes for target {}", packet_size, request.target);
    
    total_bytes += packet_size;
    
    // Return buffer to pool
    buffer_pool.release(buffer);
    
    Ok(total_bytes)
}

/// Rate limiter for injection control
pub struct RateLimiter {
    max_rate: u32,
    tokens: Arc<AtomicU64>,
    last_refill: Arc<RwLock<Instant>>,
}

impl RateLimiter {
    pub fn new(max_rate: u32) -> Self {
        Self {
            max_rate,
            tokens: Arc::new(AtomicU64::new(max_rate as u64)),
            last_refill: Arc::new(RwLock::new(Instant::now())),
        }
    }
    
    pub fn try_acquire(&self) -> bool {
        let now = Instant::now();
        let mut last_refill = self.last_refill.write();
        
        // Refill tokens based on time elapsed
        let elapsed = now.duration_since(*last_refill);
        let tokens_to_add = (elapsed.as_secs() * self.max_rate as u64) + 
                           (elapsed.subsec_millis() as u64 * self.max_rate as u64 / 1000);
        
        if tokens_to_add > 0 {
            let current_tokens = self.tokens.load(Ordering::Relaxed);
            let new_tokens = (current_tokens + tokens_to_add).min(self.max_rate as u64);
            self.tokens.store(new_tokens, Ordering::Relaxed);
            *last_refill = now;
        }
        
        // Try to acquire a token
        self.tokens.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |tokens| {
            if tokens > 0 {
                Some(tokens - 1)
            } else {
                None
            }
        }).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[test]
    fn test_engine_creation() {
        let config = EngineConfig::default();
        let engine = DeauthEngine::new(config).expect("Should create engine");
        
        assert_eq!(engine.config.worker_threads, 4);
        assert_eq!(engine.config.max_rate_per_worker, 1000);
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