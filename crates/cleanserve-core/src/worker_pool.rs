//! Dynamic Worker Pool
//! 
//! Manages PHP worker processes with automatic scaling based on load.
//! Part of P3.2: Dynamic Worker Pool

use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// Worker state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WorkerState {
    Idle,
    Busy,
    Starting,
    Stopped,
}

/// A single PHP worker
pub struct Worker {
    id: usize,
    pid: Option<u32>,
    state: WorkerState,
    last_used: Instant,
    start_time: Instant,
}

impl Worker {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            pid: None,
            state: WorkerState::Stopped,
            last_used: Instant::now(),
            start_time: Instant::now(),
        }
    }
    
    pub fn is_available(&self) -> bool {
        self.state == WorkerState::Idle
    }
    
    pub fn mark_busy(&mut self) {
        self.state = WorkerState::Busy;
    }
    
    pub fn mark_idle(&mut self) {
        self.state = WorkerState::Idle;
        self.last_used = Instant::now();
    }
}

/// Worker pool configuration
#[derive(Debug, Clone)]
pub struct WorkerPoolConfig {
    pub min_workers: usize,
    pub max_workers: usize,
    pub idle_timeout_secs: u64,
    pub max_requests_per_worker: usize,
    pub worker_startup_timeout_ms: u64,
}

impl Default for WorkerPoolConfig {
    fn default() -> Self {
        Self {
            min_workers: 2,
            max_workers: 10,
            idle_timeout_secs: 60,
            max_requests_per_worker: 500,
            worker_startup_timeout_ms: 5000,
        }
    }
}

/// Dynamic worker pool
pub struct WorkerPool {
    config: WorkerPoolConfig,
    workers: RwLock<Vec<Worker>>,
    available_count: AtomicUsize,
    busy_count: AtomicUsize,
    total_requests: AtomicUsize,
    php_path: String,
    root: String,
    port: u16,
}

impl WorkerPool {
    pub fn new(php_path: String, root: String, port: u16, config: WorkerPoolConfig) -> Self {
        Self {
            config,
            workers: RwLock::new(Vec::new()),
            available_count: AtomicUsize::new(0),
            busy_count: AtomicUsize::new(0),
            total_requests: AtomicUsize::new(0),
            php_path,
            root,
            port,
        }
    }
    
    /// Initialize the pool with minimum workers
    pub async fn start(&self) -> anyhow::Result<()> {
        let mut workers = self.workers.write().await;
        
        for i in 0..self.config.min_workers {
            let mut worker = Worker::new(i);
            worker.state = WorkerState::Starting;
            workers.push(worker);
        }
        
        // Start all workers
        for worker in workers.iter_mut() {
            self.start_worker(worker).await?;
        }
        
        info!(
            "Worker pool started with {} workers (min={}, max={})",
            self.config.min_workers,
            self.config.min_workers,
            self.config.max_workers
        );
        
        Ok(())
    }
    
    /// Start a single worker
    async fn start_worker(&self, worker: &mut Worker) -> anyhow::Result<()> {
        debug!("Starting worker {}", worker.id);
        
        let child = Command::new(&self.php_path)
            .args([
                "-S",
                &format!("127.0.0.1:{}", self.port + 9000 + worker.id as u16),
                "-t",
                &self.root,
                "-d",
                "cgi.fix_pathinfo=1",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        
        worker.pid = Some(child.id());
        worker.state = WorkerState::Idle;
        worker.start_time = Instant::now();
        
        self.available_count.fetch_add(1, Ordering::SeqCst);
        
        debug!("Worker {} started with PID {}", worker.id, child.id());
        Ok(())
    }
    
    /// Get an available worker
    pub async fn acquire(&self) -> Option<usize> {
        let mut workers = self.workers.write().await;
        
        // Find available worker
        if let Some(worker) = workers.iter_mut().find(|w| w.is_available()) {
            worker.mark_busy();
            self.available_count.fetch_sub(1, Ordering::SeqCst);
            self.busy_count.fetch_add(1, Ordering::SeqCst);
            return Some(worker.id);
        }
        
        // Scale up if possible
        if workers.len() < self.config.max_workers {
            let id = workers.len();
            let mut worker = Worker::new(id);
            worker.state = WorkerState::Starting;
            workers.push(worker);
            
            drop(workers);
            
            // Start the new worker
            let mut workers = self.workers.write().await;
            if let Some(w) = workers.iter_mut().find(|w| w.id == id) {
                if let Err(e) = self.start_worker(w).await {
                    error!("Failed to start worker {}: {}", id, e);
                    return None;
                }
                w.mark_busy();
                self.busy_count.fetch_add(1, Ordering::SeqCst);
                return Some(id);
            }
        }
        
        None
    }
    
    /// Release a worker back to the pool
    pub async fn release(&self, worker_id: usize) {
        let mut workers = self.workers.write().await;
        
        if let Some(worker) = workers.iter_mut().find(|w| w.id == worker_id) {
            worker.mark_idle();
            self.available_count.fetch_add(1, Ordering::SeqCst);
            self.busy_count.fetch_sub(1, Ordering::SeqCst);
            self.total_requests.fetch_add(1, Ordering::SeqCst);
        }
    }
    
    /// Stop a specific worker
    pub async fn stop_worker(&self, worker_id: usize) -> anyhow::Result<()> {
        let mut workers = self.workers.write().await;
        
        if let Some(worker) = workers.iter_mut().find(|w| w.id == worker_id) {
            if let Some(pid) = worker.pid {
                #[cfg(windows)]
                {
                    Command::new("taskkill")
                        .args(["/PID", &pid.to_string()])
                        .output()?;
                }
                #[cfg(unix)]
                {
                    Command::new("kill")
                        .arg(pid.to_string())
                        .output()?;
                }
                
                worker.state = WorkerState::Stopped;
                worker.pid = None;
                
                if worker.is_available() {
                    self.available_count.fetch_sub(1, Ordering::SeqCst);
                } else {
                    self.busy_count.fetch_sub(1, Ordering::SeqCst);
                }
            }
        }
        
        Ok(())
    }
    
    /// Scale down idle workers
    pub async fn scale_down(&self) {
        let mut workers = self.workers.write().await;
        
        while workers.len() > self.config.min_workers {
            // Find oldest idle worker
            if let Some(oldest) = workers.iter()
                .filter(|w| w.is_available())
                .min_by_key(|w| w.last_used)
                .map(|w| w.id)
            {
                if let Some(idx) = workers.iter().position(|w| w.id == oldest) {
                    let worker = &workers[idx];
                    if let Some(pid) = worker.pid {
                        #[cfg(windows)]
                        {
                            let _ = Command::new("taskkill")
                                .args(["/PID", &pid.to_string(), "/F"])
                                .output();
                        }
                        #[cfg(unix)]
                        {
                            let _ = Command::new("kill")
                                .arg(pid.to_string())
                                .output();
                        }
                    }
                    
                    self.available_count.fetch_sub(1, Ordering::SeqCst);
                    workers.remove(idx);
                    info!("Scaled down: now {} workers", workers.len());
                }
            } else {
                break;
            }
        }
    }
    
    /// Get pool statistics
    pub async fn stats(&self) -> PoolStats {
        let workers = self.workers.read().await;
        
        PoolStats {
            total_workers: workers.len(),
            available: self.available_count.load(Ordering::SeqCst),
            busy: self.busy_count.load(Ordering::SeqCst),
            total_requests: self.total_requests.load(Ordering::SeqCst),
            min_workers: self.config.min_workers,
            max_workers: self.config.max_workers,
        }
    }
    
    /// Stop all workers
    pub async fn stop(&self) {
        let mut workers = self.workers.write().await;
        
        for worker in workers.iter() {
            if let Some(pid) = worker.pid {
                #[cfg(windows)]
                {
                    let _ = Command::new("taskkill")
                        .args(["/PID", &pid.to_string(), "/F"])
                        .output();
                }
                #[cfg(unix)]
                {
                    let _ = Command::new("kill")
                        .arg(pid.to_string())
                        .output();
                }
            }
        }
        
        workers.clear();
        self.available_count.store(0, Ordering::SeqCst);
        self.busy_count.store(0, Ordering::SeqCst);
        
        info!("Worker pool stopped");
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_workers: usize,
    pub available: usize,
    pub busy: usize,
    pub total_requests: usize,
    pub min_workers: usize,
    pub max_workers: usize,
}

impl std::fmt::Display for PoolStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Workers: {}/{} (available={}, busy={}), Requests: {}",
            self.total_workers,
            self.max_workers,
            self.available,
            self.busy,
            self.total_requests
        )
    }
}
