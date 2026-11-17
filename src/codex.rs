use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use crate::model::{Quota, normalize};

pub enum Account {
    Chatgpt(String),
    LoggedOut,
    Unsupported,
}

pub struct Codex {
    child: Child,
    stdin: ChildStdin,
    messages: Receiver<Result<Value, ()>>,
    deadline: Instant,
    next_id: u64,
}

impl Codex {
    pub fn start(executable: &Path) -> Result<Self, ()> {
        let mut child = Command::new(executable)
            .args(["app-server", "--stdio"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| ())?;
        let stdin = child.stdin.take().ok_or(())?;
        let stdout = child.stdout.take().ok_or(())?;
        let (send, messages) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = Vec::new();
                // Bound untrusted output before allocating a complete line.
                match std::io::Read::take(&mut reader, 1_048_577).read_until(b'\n', &mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) if line.len() > 1_048_576 => {
                        let _ = send.send(Err(()));
                        break;
                    }
                    Ok(_) => {
                        if send
                            .send(serde_json::from_slice(&line).map_err(|_| ()))
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
        });
        let mut connection = Self {
            child,
            stdin,
            messages,
            deadline: Instant::now() + Duration::from_secs(15),
            next_id: 1,
        };
        connection.request(
            "initialize",
            json!({"clientInfo":{"name":"herdr-quota","version":env!("CARGO_PKG_VERSION")}}),
        )?;
        connection.send(&json!({"method":"initialized"}))?;
        Ok(connection)
    }

    fn send(&mut self, message: &Value) -> Result<(), ()> {
        serde_json::to_writer(&mut self.stdin, message).map_err(|_| ())?;
        self.stdin.write_all(b"\n").map_err(|_| ())?;
        self.stdin.flush().map_err(|_| ())
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value, ()> {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({"id":id,"method":method,"params":params}))?;
        loop {
            let remaining = self
                .deadline
                .checked_duration_since(Instant::now())
                .ok_or(())?;
            let response = self.messages.recv_timeout(remaining).map_err(|_| ())??;
            if response.get("id").and_then(Value::as_u64) == Some(id) {
                // Never copy server error bodies to output or logs.
                if response.get("error").is_some() {
                    return Err(());
                }
                return response.get("result").cloned().ok_or(());
            }
            // Sparse updates cannot replace a complete snapshot; the periodic
            // read is authoritative. No model threads or turns are started.
        }
    }

    pub fn account(&mut self) -> Result<Account, ()> {
        let response = self.request("account/read", json!({"refreshToken":false}))?;
        let account = &response["account"];
        match account.get("type").and_then(Value::as_str) {
            None if account.is_null() => Ok(Account::LoggedOut),
            Some("chatgpt") => {
                let user_id = response
                    .get("userId")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty());
                let email = account
                    .get("email")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty());
                if user_id.is_none() && email.is_none() {
                    return Err(());
                }
                let metadata = json!([
                    user_id,
                    email,
                    account.get("workspaceName"),
                    account.get("planType")
                ]);
                let digest = Sha256::digest(serde_json::to_vec(&metadata).map_err(|_| ())?);
                Ok(Account::Chatgpt(format!("{digest:x}")))
            }
            Some(_) => Ok(Account::Unsupported),
            _ => Err(()),
        }
    }

    pub fn quota(&mut self) -> Result<Quota, ()> {
        let response = self.request(
            "account/rateLimits/read",
            json!({"excludeResetCreditDetails":true}),
        )?;
        normalize(&response).map_err(|_| ())
    }
}

impl Drop for Codex {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
