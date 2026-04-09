//! Worker connection management

use tokio::sync::mpsc;
use bytes::Bytes;

use crate::protocol::message::Message;
use crate::error::Result;

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,
    /// Connected but not authenticated
    Authenticating,
    /// Authenticated but not attached
    Attached,
    /// Fully operational
    Ready,
}

/// Represents a connection to a worker
pub struct WorkerConnection {
    /// Connection ID
    pub id: u64,
    /// Worker name
    pub worker_name: String,
    /// Connection state
    state: ConnectionState,
    /// Receive channel for incoming messages
    rx: mpsc::Receiver<Message>,
    /// Channel to send responses
    tx: Option<mpsc::Sender<Result<Bytes>>>,
}

impl WorkerConnection {
    /// Create a new worker connection
    pub fn new(id: u64, worker_name: String, rx: mpsc::Receiver<Message>) -> Self {
        Self {
            id,
            worker_name,
            state: ConnectionState::Disconnected,
            rx,
            tx: None,
        }
    }

    /// Set the connection state
    pub fn set_state(&mut self, state: ConnectionState) {
        self.state = state;
    }

    /// Get the connection state
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// Set the response sender
    pub fn set_response_sender(&mut self, tx: mpsc::Sender<Result<Bytes>>) {
        self.tx = Some(tx);
    }

    /// Send a message to the worker
    pub async fn send(&self, message: Message) -> Result<()> {
        if let Some(ref tx) = self.tx {
            let encoded = crate::protocol::codec::MessageCodec::new().encode(&message)?;
            tx.send(Ok(encoded)).await
                .map_err(|e| crate::error::BuildbotError::Channel(e.to_string()))?;
        }
        Ok(())
    }

    /// Receive a message from the worker
    pub async fn recv(&mut self) -> Option<Message> {
        self.rx.recv().await
    }
}
