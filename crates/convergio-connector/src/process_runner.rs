//! Sandboxed connector runner (separate process).

use crate::backoff::{BackoffPolicy, BackoffState};
use crate::connector::{Connector, DiscoverItem, DiscoverRequest, Health, PullPage, PullRequest};
use crate::error::{ConnectorError, FailureKind};
use crate::protocol::{Op, Request, Response};
use crate::rate_limit::RateLimiter;
use crate::types::{SchemaHash, Watermark};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::timeout;

/// Process runner configuration.
#[derive(Debug, Clone)]
pub struct ProcessConnectorSpec {
    /// Executable path.
    pub command: PathBuf,
    /// Arguments.
    pub args: Vec<String>,
    /// Environment variables to pass to the connector.
    pub env: BTreeMap<String, String>,
    /// Inherited environment keys to preserve (everything else is cleared).
    pub inherit_env: Vec<String>,
    /// Per-call timeout.
    pub timeout: Duration,
    /// Optional max calls per second.
    pub max_calls_per_sec: Option<f64>,
    /// Maximum retries for retryable failures.
    pub max_retries: u32,
    /// Backoff policy for retries.
    pub backoff: BackoffPolicy,
}

impl Default for ProcessConnectorSpec {
    fn default() -> Self {
        Self {
            command: PathBuf::from("/usr/bin/false"),
            args: Vec::new(),
            env: BTreeMap::new(),
            inherit_env: vec!["PATH".to_string()],
            timeout: Duration::from_secs(10),
            max_calls_per_sec: None,
            max_retries: 3,
            backoff: BackoffPolicy::default(),
        }
    }
}

#[derive(Debug)]
struct ProcessConnectorInner {
    spec: ProcessConnectorSpec,
    _child: Child,
    stdin: tokio::process::ChildStdin,
    stdout: tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    limiter: Option<RateLimiter>,
    backoff: BackoffState,
}

/// A connector implemented as an external process.
#[derive(Debug)]
pub struct ProcessConnector {
    inner: Mutex<ProcessConnectorInner>,
}

impl ProcessConnector {
    /// Spawn the connector process.
    pub async fn spawn(spec: ProcessConnectorSpec) -> Result<Self, ConnectorError> {
        let mut cmd = Command::new(&spec.command);
        cmd.args(&spec.args);
        cmd.kill_on_drop(true);
        cmd.env_clear();
        for k in &spec.inherit_env {
            if let Ok(v) = std::env::var(k) {
                cmd.env(k, v);
            }
        }
        for (k, v) in &spec.env {
            cmd.env(k, v);
        }
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::null());

        let mut child = cmd.spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ConnectorError::protocol("connector stdin not piped"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ConnectorError::protocol("connector stdout not piped"))?;
        let stdout = BufReader::new(stdout).lines();

        Ok(Self {
            inner: Mutex::new(ProcessConnectorInner {
                limiter: spec.max_calls_per_sec.and_then(RateLimiter::per_second),
                backoff: BackoffState::new(spec.backoff),
                spec,
                _child: child,
                stdin,
                stdout,
            }),
        })
    }
}

impl ProcessConnectorInner {
    async fn call(&mut self, op: Op, params: Value) -> Result<Value, ConnectorError> {
        if let Some(l) = &mut self.limiter {
            l.acquire().await;
        }

        let id = request_id();
        let req = Request {
            id: id.clone(),
            op,
            params,
        };
        let line = serde_json::to_vec(&req)?;
        self.stdin.write_all(&line).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;

        let secs = self.spec.timeout.as_secs();
        let read = timeout(self.spec.timeout, self.stdout.next_line()).await;
        let line_opt = match read {
            Ok(r) => r?,
            Err(_) => return Err(ConnectorError::timeout(secs, "connector response")),
        };
        let Some(line) = line_opt else {
            return Err(ConnectorError::protocol("connector closed stdout"));
        };
        let resp: Response = serde_json::from_str(&line)?;
        if resp.id != id {
            return Err(ConnectorError::protocol(format!(
                "mismatched response id: expected {id}, got {}",
                resp.id
            )));
        }
        resp.into_result()
    }

    async fn call_with_retry(&mut self, op: Op, params: Value) -> Result<Value, ConnectorError> {
        for attempt in 0..=self.spec.max_retries {
            match self.call(op.clone(), params.clone()).await {
                Ok(v) => {
                    self.backoff.reset();
                    return Ok(v);
                }
                Err(ConnectorError::ConnectorFailed {
                    kind: FailureKind::Retryable,
                    message,
                }) if attempt < self.spec.max_retries => {
                    tracing::warn!(attempt, error = %message, "retryable connector failure");
                    let delay = self.backoff.next_delay();
                    tokio::time::sleep(delay).await;
                }
                Err(e) => return Err(e),
            }
        }
        Err(ConnectorError::protocol("unreachable retry loop"))
    }
}

#[async_trait]
impl Connector for ProcessConnector {
    type Record = Value;

    async fn discover(&self, req: DiscoverRequest) -> Result<Vec<DiscoverItem>, ConnectorError> {
        let mut inner = self.inner.lock().await;
        let v = inner
            .call_with_retry(Op::Discover, serde_json::to_value(req)?)
            .await?;
        Ok(serde_json::from_value(v)?)
    }

    async fn pull(&self, req: PullRequest) -> Result<PullPage<Self::Record>, ConnectorError> {
        let mut inner = self.inner.lock().await;
        let v = inner
            .call_with_retry(Op::Pull, serde_json::to_value(req)?)
            .await?;
        Ok(serde_json::from_value(v)?)
    }

    async fn watermark(&self) -> Result<Option<Watermark>, ConnectorError> {
        let mut inner = self.inner.lock().await;
        let v = inner.call_with_retry(Op::Watermark, Value::Null).await?;
        Ok(serde_json::from_value(v)?)
    }

    async fn schema_hash(&self) -> Result<SchemaHash, ConnectorError> {
        let mut inner = self.inner.lock().await;
        let v = inner.call_with_retry(Op::SchemaHash, Value::Null).await?;
        Ok(serde_json::from_value(v)?)
    }

    async fn health(&self) -> Result<Health, ConnectorError> {
        let mut inner = self.inner.lock().await;
        let v = inner.call_with_retry(Op::Health, Value::Null).await?;
        Ok(serde_json::from_value(v)?)
    }
}

fn request_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_nanos();
    format!("req-{n}")
}
