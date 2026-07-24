use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub at: u64,
    pub actor: String,
    pub action: String,
    pub target: Option<String>,
    pub arguments: Value,
    pub success: bool,
}

#[derive(Debug, Error)]
pub enum AdminError {
    #[error("permission denied")]
    Denied,
    #[error("unknown command {0}")]
    Unknown(String),
    #[error("command failed: {0}")]
    Failed(String),
}

pub type CommandFn =
    Arc<dyn Fn(&CommandContext, &[String]) -> Result<Value, AdminError> + Send + Sync>;
#[derive(Clone)]
pub struct CommandContext {
    pub actor: String,
    pub permissions: HashSet<String>,
}

struct Command {
    permission: String,
    help: String,
    run: CommandFn,
}

#[derive(Default, Clone)]
pub struct AdminConsole {
    commands: Arc<RwLock<HashMap<String, Command>>>,
    audit: Arc<RwLock<Vec<AuditEntry>>>,
}

impl AdminConsole {
    pub fn register<F>(&self, name: &str, permission: &str, help: &str, f: F)
    where
        F: Fn(&CommandContext, &[String]) -> Result<Value, AdminError> + Send + Sync + 'static,
    {
        self.commands.write().insert(
            name.into(),
            Command {
                permission: permission.into(),
                help: help.into(),
                run: Arc::new(f),
            },
        );
    }
    pub fn execute(&self, c: &CommandContext, line: &str) -> Result<Value, AdminError> {
        let parts = shell_words(line);
        let name = parts
            .first()
            .ok_or_else(|| AdminError::Unknown(String::new()))?;
        let map = self.commands.read();
        let cmd = map
            .get(name)
            .ok_or_else(|| AdminError::Unknown(name.clone()))?;
        if !c.permissions.contains(&cmd.permission) && !c.permissions.contains("*") {
            self.log(c, line, false);
            return Err(AdminError::Denied);
        }
        let r = (cmd.run)(c, &parts[1..]);
        self.log(c, line, r.is_ok());
        r
    }
    pub fn autocomplete(&self, prefix: &str) -> Vec<String> {
        self.commands
            .read()
            .keys()
            .filter(|x| x.starts_with(prefix))
            .cloned()
            .collect()
    }
    pub fn help(&self) -> Vec<(String, String)> {
        self.commands
            .read()
            .iter()
            .map(|(n, c)| (n.clone(), c.help.clone()))
            .collect()
    }
    pub fn audit(&self) -> Vec<AuditEntry> {
        self.audit.read().clone()
    }
    fn log(&self, c: &CommandContext, line: &str, success: bool) {
        self.audit.write().push(AuditEntry {
            at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            actor: c.actor.clone(),
            action: line.into(),
            target: None,
            arguments: Value::Null,
            success,
        })
    }
}

fn shell_words(s: &str) -> Vec<String> {
    let mut out = vec![];
    let mut cur = String::new();
    let mut quote = false;
    for c in s.chars() {
        match c {
            '"' => quote = !quote,
            ' ' | '\t' if !quote => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur))
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur)
    }
    out
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteRequest {
    pub actor: String,
    pub token: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteResponse {
    pub ok: bool,
    pub value: Value,
    pub error: Option<String>,
}

pub async fn serve_remote(
    console: AdminConsole,
    address: &str,
    expected_token: Arc<str>,
) -> Result<(), std::io::Error> {
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::TcpListener,
    };
    let listener = TcpListener::bind(address).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        let console = console.clone();
        let expected_token = Arc::clone(&expected_token);
        tokio::spawn(async move {
            let (read, mut write) = stream.into_split();
            let mut lines = BufReader::new(read).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let response = match serde_json::from_str::<RemoteRequest>(&line) {
                    Ok(req) => {
                        if req.token.as_bytes() != expected_token.as_bytes() {
                            let response = RemoteResponse {
                                ok: false,
                                value: Value::Null,
                                error: Some("authentication failed".into()),
                            };
                            if let Ok(mut data) = serde_json::to_vec(&response) {
                                data.push(b'\n');
                                let _ = write.write_all(&data).await;
                            }
                            continue;
                        }
                        let ctx = CommandContext {
                            actor: req.actor,
                            permissions: HashSet::from(["*".to_string()]),
                        };
                        match console.execute(&ctx, &req.command) {
                            Ok(value) => RemoteResponse {
                                ok: true,
                                value,
                                error: None,
                            },
                            Err(error) => RemoteResponse {
                                ok: false,
                                value: Value::Null,
                                error: Some(error.to_string()),
                            },
                        }
                    }
                    Err(error) => RemoteResponse {
                        ok: false,
                        value: Value::Null,
                        error: Some(error.to_string()),
                    },
                };
                if let Ok(mut data) = serde_json::to_vec(&response) {
                    data.push(b'\n');
                    if write.write_all(&data).await.is_err() {
                        break;
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissions_are_enforced_and_every_attempt_is_audited() {
        let console = AdminConsole::default();
        console.register("kick", "players.kick", "Kick a player", |_context, args| {
            Ok(serde_json::json!({ "target": args.first() }))
        });
        let denied = CommandContext {
            actor: "helper".into(),
            permissions: HashSet::new(),
        };
        assert!(matches!(
            console.execute(&denied, "kick player-1"),
            Err(AdminError::Denied)
        ));
        let allowed = CommandContext {
            actor: "moderator".into(),
            permissions: HashSet::from(["players.kick".into()]),
        };
        assert!(console.execute(&allowed, "kick player-1").is_ok());
        let audit = console.audit();
        assert_eq!(audit.len(), 2);
        assert!(!audit[0].success);
        assert!(audit[1].success);
    }

    #[test]
    fn quoted_command_arguments_are_kept_together() {
        assert_eq!(
            shell_words("announce \"round ending soon\""),
            vec!["announce", "round ending soon"]
        );
    }
}
