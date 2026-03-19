//! Signal Handling Module
//! 
//! Handles Unix signals for graceful shutdown and reload.
//! Essential for containerized/PID 1 environments.

use tokio::sync::mpsc;
use tracing::{info, warn};

/// Signal types handled by CleanServe
#[derive(Debug, Clone)]
pub enum Signal {
    Shutdown,
    Interrupt,
    Reload,
    ChildExited(u32),
}

#[derive(Debug, Clone)]
pub struct ShutdownConfig {
    pub shutdown_timeout_secs: u64,
    pub wait_for_connections: bool,
    pub force_kill: bool,
}

impl Default for ShutdownConfig {
    fn default() -> Self {
        Self {
            shutdown_timeout_secs: 30,
            wait_for_connections: true,
            force_kill: true,
        }
    }
}

pub struct SignalHandler {
    shutdown_tx: mpsc::Sender<Signal>,
}

impl SignalHandler {
    pub fn new() -> (Self, mpsc::Receiver<Signal>) {
        let (tx, rx) = mpsc::channel(100);
        (Self { shutdown_tx: tx }, rx)
    }
    
    pub fn transmitter(&self) -> mpsc::Sender<Signal> {
        self.shutdown_tx.clone()
    }
    
    pub async fn run(&self, mut rx: mpsc::Receiver<Signal>) {
        while let Some(signal) = rx.recv().await {
            match signal {
                Signal::Shutdown => {
                    info!("Received shutdown signal");
                    break;
                }
                Signal::Interrupt => {
                    info!("Received interrupt signal");
                    break;
                }
                Signal::Reload => {
                    info!("Received reload signal");
                }
                Signal::ChildExited(pid) => {
                    warn!("Child process {} exited", pid);
                }
            }
        }
    }
}

pub struct GracefulShutdown {
    config: ShutdownConfig,
    connections_active: std::sync::atomic::AtomicUsize,
    shutdown_initiated: std::sync::atomic::AtomicBool,
}

impl GracefulShutdown {
    pub fn new(config: ShutdownConfig) -> Self {
        Self {
            config,
            connections_active: std::sync::atomic::AtomicUsize::new(0),
            shutdown_initiated: std::sync::atomic::AtomicBool::new(false),
        }
    }
    
    pub fn connection_started(&self) {
        self.connections_active.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
    
    pub fn connection_ended(&self) {
        self.connections_active.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
    
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown_initiated.load(std::sync::atomic::Ordering::SeqCst)
    }
    
    pub async fn initiate(&self) {
        self.shutdown_initiated.store(true, std::sync::atomic::Ordering::SeqCst);
        
        if self.config.wait_for_connections {
            let timeout = std::time::Duration::from_secs(self.config.shutdown_timeout_secs);
            let start = std::time::Instant::now();
            
            while self.connections_active.load(std::sync::atomic::Ordering::SeqCst) > 0 {
                if start.elapsed() > timeout {
                    warn!("Shutdown timeout reached");
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
        
        info!("Graceful shutdown complete");
    }
}

#[cfg(unix)]
pub async fn setup_unix_signals() -> mpsc::Receiver<Signal> {
    use tokio::signal::unix::{signal, SignalKind};
    
    let (tx, rx) = mpsc::channel(100);
    
    let tx_term = tx.clone();
    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        sigterm.recv().await;
        let _ = tx_term.send(Signal::Shutdown).await;
    });
    
    let tx_int = tx.clone();
    tokio::spawn(async move {
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        sigint.recv().await;
        let _ = tx_int.send(Signal::Interrupt).await;
    });
    
    info!("Signal handlers registered");
    rx
}

#[cfg(windows)]
pub async fn setup_unix_signals() -> mpsc::Receiver<Signal> {
    use tokio::signal::ctrl_c;
    
    let (tx, rx) = mpsc::channel(100);
    
    let tx_ctrl = tx.clone();
    tokio::spawn(async move {
        ctrl_c().await.ok();
        let _ = tx_ctrl.send(Signal::Interrupt).await;
    });
    
    info!("Windows signal handlers registered");
    rx
}

pub fn is_pid_one() -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::getpid() == 1 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}

#[cfg(unix)]
pub async fn run_as_pid_one() {
    if !is_pid_one() {
        warn!("Not running as PID 1");
        return;
    }
    
    info!("Running as PID 1 (container environment)");
}

#[cfg(not(unix))]
pub async fn run_as_pid_one() {
    warn!("run_as_pid_one() is only supported on Unix");
}
