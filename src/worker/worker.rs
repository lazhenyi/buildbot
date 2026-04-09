//! Worker implementation - runs on worker machines to execute builds

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::watch;
use tokio::time::interval;

use crate::protocol::worker_protocol::WorkerProtocol;
use crate::protocol::message::{Message, MessageType, Command};

/// Worker state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerState {
    Disconnected,
    Connecting,
    Handshake,
    Authenticated,
    Ready,
    RunningBuild,
    ShuttingDown,
}

/// Worker capabilities
#[derive(Debug, Clone, Default)]
pub struct WorkerCapabilitiesInfo {
    /// Available commands
    pub commands: Vec<String>,
    /// Worker environment
    pub environment: HashMap<String, String>,
    /// Maximum message size
    pub max_message_size: usize,
}

impl WorkerCapabilitiesInfo {
    /// Create default capabilities
    pub fn new() -> Self {
        let mut env = HashMap::new();
        env.insert("PATH".to_string(), std::env::var("PATH").unwrap_or_default());
        env.insert("HOME".to_string(), std::env::var("HOME").unwrap_or_default());

        Self {
            commands: vec![
                "shell".to_string(),
                "bash".to_string(),
                "git".to_string(),
                "mkdir".to_string(),
                "rm".to_string(),
                "cp".to_string(),
                "mv".to_string(),
                "cat".to_string(),
                "echo".to_string(),
            ],
            environment: env,
            max_message_size: 16 * 1024 * 1024, // 16MB
        }
    }
}

/// Represents a buildbot worker (runs on worker machines)
pub struct Worker {
    /// Worker name
    name: String,
    /// Worker password (for future auth use)
    _password: String,
    /// Token for master-worker authentication
    token: Option<String>,
    /// Base directory (for future build cache use)
    _basedir: PathBuf,
    /// Current state
    state: WorkerState,
    /// Protocol handler
    protocol: WorkerProtocol,
    /// Worker capabilities
    capabilities: WorkerCapabilitiesInfo,
    /// Build directory
    build_dir: PathBuf,
    /// TCP stream (kept for the command loop)
    stream: Option<TcpStream>,
    /// Shutdown signal sender for the command loop
    shutdown_tx: watch::Sender<bool>,
    /// Shutdown signal receiver (kept for API completeness)
    _shutdown_rx: watch::Receiver<bool>,
    /// Cancellation flag — set when CancelBuild is received from master
    cancelled: Arc<AtomicBool>,
}

impl Worker {
    /// Create a new worker instance
    pub fn new(name: String, password: String, token: Option<String>, basedir: PathBuf) -> Self {
        let build_dir = basedir.join("builds");
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        Self {
            name,
            _password: password,
            token,
            _basedir: basedir,
            state: WorkerState::Disconnected,
            protocol: WorkerProtocol::new(),
            capabilities: WorkerCapabilitiesInfo::new(),
            build_dir,
            stream: None,
            shutdown_tx,
            _shutdown_rx: shutdown_rx,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get the worker name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the current state
    pub fn state(&self) -> &WorkerState {
        &self.state
    }

    /// Connect to the master
    pub async fn connect(&mut self, host: &str, port: u16) -> anyhow::Result<()> {
        let addr = format!("{}:{}", host, port);
        tracing::info!("Connecting to master at {}", addr);

        self.state = WorkerState::Connecting;
        let stream = TcpStream::connect(&addr).await?;
        tracing::info!("Connected to master");

        self.stream = Some(stream);
        self.state = WorkerState::Handshake;
        Ok(())
    }

    /// Run the worker event loop with automatic reconnection on disconnect.
    ///
    /// This is the main entry point for worker-side code. It:
    /// 1. Connects to the master with exponential-backoff retry
    /// 2. Performs the protocol handshake
    /// 3. Runs the command loop
    /// 4. On disconnect, waits and reconnects (unless shutdown is signalled)
    pub async fn run(&mut self, host: &str, port: u16) -> anyhow::Result<()> {
        tracing::info!("Worker {} starting, master={}:{}", self.name, host, port);

        // Create build directory
        tokio::fs::create_dir_all(&self.build_dir).await?;

        let mut retry_delay = Duration::from_secs(1);
        let max_delay = Duration::from_secs(60);

        loop {
            match self.connect(host, port).await {
                Ok(()) => {
                    retry_delay = Duration::from_secs(1); // reset backoff on success
                    if let Err(e) = self.run_connected().await {
                        tracing::warn!("Connection lost: {}, will reconnect...", e);
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Worker {} failed to connect to {}:{} ({}), retrying in {:?}",
                        self.name,
                        host,
                        port,
                        e,
                        retry_delay
                    );
                }
            }

            // Check if shutdown was requested before sleeping
            if *self.shutdown_tx.subscribe().borrow() {
                tracing::info!("Worker {} shutting down", self.name);
                break;
            }

            tokio::time::sleep(retry_delay).await;
            retry_delay = (retry_delay * 2).min(max_delay);
        }

        tracing::info!("Worker {} run loop ended", self.name);
        Ok(())
    }

    /// Run the connected command loop (handshake + command handling).
    /// Assumes `self.stream` is Some.
    async fn run_connected(&mut self) -> anyhow::Result<()> {
        // Take the pre-established connection
        let stream = self.stream.take()
            .ok_or_else(|| anyhow::anyhow!("No connection established"))?;

        // Run handshake then command loop
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // ── Handshake ───────────────────────────────────────────────
        // Step 1: Send version request
        let version_msg = WorkerProtocol::get_version();
        let encoded = self.protocol.encode(&version_msg)?;
        writer.write_all(&encoded).await?;
        tracing::debug!("Sent version request to master");

        // Step 2: Read version response
        let msg = self.read_message(&mut reader).await?;
        if msg.msg_type != MessageType::Version {
            tracing::warn!("Expected Version, got {:?}", msg.msg_type);
            return Ok(());
        }

        // Step 3: Send worker info (includes name and token for auth)
        let mut info = serde_json::json!({
            "worker_name": self.name,
            "version": env!("CARGO_PKG_VERSION"),
            "commands": self.capabilities.commands,
            "environment": self.capabilities.environment,
            "max_message_size": self.capabilities.max_message_size,
        });
        // Attach token if configured
        if let Some(ref token) = self.token {
            info["token"] = serde_json::json!(token);
        }
        let msg = Message {
            msg_type: MessageType::WorkerInfo,
            payload: info,
        };
        let encoded = self.protocol.encode(&msg)?;
        writer.write_all(&encoded).await?;
        tracing::info!("Worker handshake complete, ready for commands");

        self.state = WorkerState::Ready;

        // ── Command loop ────────────────────────────────────────────
        let mut keepalive_interval = interval(Duration::from_secs(30));
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let mut writer = BufWriter::new(writer);

        loop {
            tokio::select! {
                // Read command from master
                msg_result = self.read_message_async(&mut reader) => {
                    match msg_result {
                        Ok(msg) => {
                            if let Some(response) = self.handle_message(msg, &mut writer).await? {
                                let encoded = self.protocol.encode(&response)?;
                                writer.write_all(&encoded).await?;
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to read message: {}", e);
                            break;
                        }
                    }
                }
                // Keepalive tick
                _ = keepalive_interval.tick() => {
                    if self.state == WorkerState::Ready || self.state == WorkerState::RunningBuild {
                        tracing::debug!("Sending keepalive");
                        let keepalive = WorkerProtocol::keepalive();
                        let encoded = self.protocol.encode(&keepalive)?;
                        writer.write_all(&encoded).await?;
                    }
                }
                // Shutdown signal
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        tracing::info!("Worker {} shutting down", self.name);
                        break;
                    }
                }
            }
        }

        tracing::info!("Worker {} command loop ended", self.name);
        Ok(())
    }

    /// Handle an incoming message from the master, return optional response
    async fn handle_message(
        &mut self,
        msg: Message,
        _writer: &mut tokio::io::BufWriter<tokio::net::tcp::OwnedWriteHalf>,
    ) -> anyhow::Result<Option<Message>> {
        match msg.msg_type {
            MessageType::StartBuild => {
                tracing::info!("Received StartBuild command");
                self.state = WorkerState::RunningBuild;
                // Reset cancellation flag for new build
                self.cancelled.store(false, Ordering::SeqCst);

                let builder_name = msg.payload.get("builder_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let build_request_id = msg.payload.get("build_request_id")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;

                // Execute a simple build: run shell commands
                let step_results = self.execute_build_steps(&builder_name).await?;

                // Send build finished
                let response = Message {
                    msg_type: MessageType::BuildFinished,
                    payload: serde_json::json!({
                        "build_request_id": build_request_id,
                        "builder_name": builder_name,
                        "results": 0, // success
                        "step_results": step_results,
                    }),
                };
                self.state = WorkerState::Ready;
                Ok(Some(response))
            }
            MessageType::BuildCommand => {
                tracing::info!("Received BuildCommand");
                let command: Command = serde_json::from_value(msg.payload.clone())?;
                let exit_code = self.execute_command(&command).await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;

                let response = Message {
                    msg_type: MessageType::StepFinished,
                    payload: serde_json::json!({
                        "exit_code": exit_code,
                        "stdout": "",
                        "stderr": "",
                    }),
                };
                Ok(Some(response))
            }
            MessageType::Shutdown => {
                tracing::info!("Received shutdown from master");
                self.state = WorkerState::ShuttingDown;
                self.shutdown_tx.send(true)?;
                Ok(None)
            }
            MessageType::CancelBuild => {
                let builder_name = msg.payload.get("builder_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                tracing::info!("Received CancelBuild for builder '{}'", builder_name);
                self.cancelled.store(true, Ordering::SeqCst);
                Ok(None)
            }
            MessageType::Ping => {
                Ok(Some(WorkerProtocol::pong()))
            }
            MessageType::Keepalive => {
                // No response needed for keepalive
                Ok(None)
            }
            MessageType::Detach => {
                tracing::info!("Received detach from master");
                self.state = WorkerState::Disconnected;
                Ok(None)
            }
            _ => {
                tracing::debug!("Unhandled message type: {:?}", msg.msg_type);
                Ok(None)
            }
        }
    }

    /// Execute build steps (shell commands for now), respecting cancellation
    async fn execute_build_steps(&mut self, builder_name: &str) -> anyhow::Result<Vec<serde_json::Value>> {
        // Check if already cancelled before starting
        if self.cancelled.load(Ordering::SeqCst) {
            tracing::info!("Build for '{}' was cancelled before starting", builder_name);
            return Err(anyhow::anyhow!("Build cancelled"));
        }

        let mut step_results = Vec::new();

        // Simple default build: just echo a message
        // In a full implementation, this would read steps from the StartBuild message
        let step = serde_json::json!({
            "step_name": "shell",
            "exit_code": 0,
            "output": format!("Build completed for builder: {}", builder_name),
        });
        step_results.push(step);

        // Check if cancelled during build
        if self.cancelled.load(Ordering::SeqCst) {
            tracing::info!("Build for '{}' was cancelled during execution", builder_name);
            return Err(anyhow::anyhow!("Build cancelled"));
        }

        Ok(step_results)
    }

    /// Execute a build command
    pub async fn execute_command(&self, command: &Command) -> Result<i32, String> {
        use std::process::Stdio;
        use tokio::process::Command;

        tracing::info!("Executing command: {:?}", command.command);

        let mut cmd = Command::new(&command.command);

        // Parse args
        if let Some(args) = command.args.get("args").and_then(|v| v.as_array()) {
            let arg_strs: Vec<&str> = args.iter()
                .filter_map(|v| v.as_str())
                .collect();
            cmd.args(arg_strs);
        }

        // Set workdir
        if let Some(workdir) = command.args.get("workdir").and_then(|v| v.as_str()) {
            cmd.current_dir(workdir);
        }

        // Merge environment
        if let Some(env) = command.args.get("environment").and_then(|v| v.as_object()) {
            for (key, value) in env {
                if let Some(val_str) = value.as_str() {
                    cmd.env(key, val_str);
                }
            }
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = cmd.output().await
            .map_err(|e| format!("Failed to execute command: {}", e))?;

        if output.status.success() {
            Ok(0)
        } else if let Some(code) = output.status.code() {
            Ok(code)
        } else {
            Ok(-1)
        }
    }

    /// Read a message from the stream (blocking)
    async fn read_message<R: AsyncReadExt + Unpin>(&self, reader: &mut R) -> anyhow::Result<Message> {
        let mut header = [0u8; 4];
        reader.read_exact(&mut header).await?;

        let len = u32::from_be_bytes(header) as usize;
        let mut data = vec![0u8; len];
        reader.read_exact(&mut data).await?;

        self.protocol.decode(&data)
            .map_err(|e| anyhow::anyhow!("Failed to decode message: {}", e))
    }

    /// Read a message asynchronously with error handling
    async fn read_message_async<R: AsyncReadExt + Unpin>(
        &self,
        reader: &mut BufReader<R>,
    ) -> anyhow::Result<Message> {
        let mut header = [0u8; 4];
        let n = reader.read(&mut header).await?;
        if n == 0 {
            return Err(anyhow::anyhow!("Connection closed by master"));
        }
        if n != 4 {
            return Err(anyhow::anyhow!("Short read on header: {} bytes", n));
        }

        let len = u32::from_be_bytes(header) as usize;
        if len > self.capabilities.max_message_size {
            return Err(anyhow::anyhow!("Message too large: {} > {}", len, self.capabilities.max_message_size));
        }
        if len == 0 {
            return Err(anyhow::anyhow!("Empty message"));
        }

        let mut data = vec![0u8; len];
        reader.read_exact(&mut data).await?;

        self.protocol.decode(&data)
            .map_err(|e| anyhow::anyhow!("Failed to decode message: {}", e))
    }

    /// Request shutdown
    pub fn request_shutdown(&mut self) {
        self.state = WorkerState::ShuttingDown;
        let _ = self.shutdown_tx.send(true);
    }
}
