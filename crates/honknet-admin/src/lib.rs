use parking_lot::RwLock;
use serde::{
    Deserialize,
    Serialize
};
use serde_json::Value;
use std::{
    collections::{
        HashMap,
        HashSet
    },
    sync::Arc,
    time::{
        SystemTime,
        UNIX_EPOCH
    }
};
use thiserror::Error;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub at: u64,
    pub actor: String,
    pub action: String,
    pub target: Option<String>,
    pub arguments: Value,
    pub success: bool
}

#[derive(Debug, Error)]
pub enum AdminError {
    #[error("permission denied")]
    Denied,
    #[error("unknown command {0}")]
    Unknown(String),
    #[error("command failed: {0}")]
    Failed(String)
}

pub type CommandFn = Arc<dyn Fn(&CommandContext, &[String]) -> Result<Value, AdminError> + Send + Sync>;
#[derive(Clone)]
pub struct CommandContext {
    pub actor: String,
    pub permissions: HashSet<String>
}

struct Command {
    permission: String,
    help: String,
    run: CommandFn
}

#[derive(Default, Clone)]
pub struct AdminConsole {
    commands: Arc<RwLock<HashMap<String, Command>>>,
    audit: Arc<RwLock<Vec<AuditEntry>>>
}

impl AdminConsole {
    pub fn register<F>(&self, name: &str, permission: &str, help: &str, f: F) where F: Fn(&CommandContext, &[String]) -> Result<Value,
    AdminError> + Send + Sync + 'static {
        self.commands.write().insert(name.into(), Command {
            permission: permission.into(), help: help.into(), run: Arc::new(f)
        });
    }
    pub fn execute(&self, c: &CommandContext, line: &str) -> Result<Value, AdminError> {
        let parts = shell_words(line);
        let name = parts.first().ok_or_else(||AdminError::Unknown(String::new()))?;
        let map = self.commands.read();
        let cmd = map.get(name).ok_or_else(||AdminError::Unknown(name.clone()))?;
        if !c.permissions.contains(&cmd.permission) && !c.permissions.contains("*") {
            self.log(c, line, false);
            return Err(AdminError::Denied)
        }
        let r =(cmd.run)(c, &parts[1..]);
        self.log(c, line, r.is_ok());
        r
    }
    pub fn autocomplete(&self, prefix: &str) -> Vec<String> {
        self.commands.read().keys().filter(|x| x.starts_with(prefix)).cloned().collect()
    }
    pub fn help(&self) -> Vec<(String, String)> {
        self.commands.read().iter().map(|(n, c)|(n.clone(), c.help.clone())).collect()
    }
    pub fn audit(&self) -> Vec<AuditEntry> {
        self.audit.read().clone()
    }
    fn log(&self, c: &CommandContext, line: &str, success: bool) {
        self.audit.write().push(AuditEntry {
            at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(), actor: c.actor.clone(),
            action: line.into(), target: None, arguments: Value::Null, success
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
            },
            _ => cur.push(c)
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
    pub permissions: HashSet<String>,
    pub command: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteResponse {
    pub ok: bool,
    pub value: Value,
    pub error: Option<String>
}

pub async fn serve_remote(console: AdminConsole, address: &str) -> Result<(), std::io::Error> {
    use tokio::{
        io::{
            AsyncBufReadExt,
            AsyncWriteExt,
            BufReader
        },
        net::TcpListener
    };
    let listener = TcpListener::bind(address).await?;
    loop {
        let(stream, _) = listener.accept().await?;
        let console = console.clone();
        tokio::spawn(async move {
            let(read, mut write) = stream.into_split();
            let mut lines = BufReader::new(read).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let response = match serde_json::from_str::<RemoteRequest>(&line) {
                    Ok(req) => {
                        let ctx = CommandContext {
                            actor: req.actor, permissions: req.permissions
                        };
                        match console.execute(&ctx, &req.command) {
                            Ok(value) => RemoteResponse {
                                ok: true, value, error: None
                            }, Err(error) => RemoteResponse {
                                ok: false, value: Value::Null, error: Some(error.to_string())
                            }
                        }
                    }, Err(error) => RemoteResponse {
                        ok: false, value: Value::Null, error: Some(error.to_string())
                    }
                };
                if let Ok(mut data) = serde_json::to_vec(&response) {
                    data.push(b'\n');
                    if write.write_all(&data).await.is_err() {
                        break
                    }
                }
            }
        });
    }
}
