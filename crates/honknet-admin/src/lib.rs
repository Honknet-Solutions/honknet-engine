use std::collections::{HashMap, HashSet};

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Permission(pub String);

#[derive(Debug, Clone, Default)]
pub struct Principal {
    pub id: String,
    pub permissions: HashSet<Permission>,
}

impl Principal {
    pub fn can(&self, permission: &str) -> bool {
        self.permissions
            .contains(&Permission(permission.to_owned()))
            || self.permissions.contains(&Permission("*".to_owned()))
    }
}

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("unknown command: {0}")]
    Unknown(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("command failed: {0}")]
    Failed(String),
}

pub type CommandHandler<C> =
    Box<dyn Fn(&mut C, &[&str]) -> Result<String, CommandError> + Send + Sync>;

pub struct CommandRegistry<C> {
    commands: HashMap<String, RegisteredCommand<C>>,
}

struct RegisteredCommand<C> {
    permission: Option<String>,
    handler: CommandHandler<C>,
}

impl<C> Default for CommandRegistry<C> {
    fn default() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }
}

impl<C> CommandRegistry<C> {
    pub fn register(
        &mut self,
        name: impl Into<String>,
        permission: Option<String>,
        handler: CommandHandler<C>,
    ) -> bool {
        self.commands
            .insert(
                name.into(),
                RegisteredCommand {
                    permission,
                    handler,
                },
            )
            .is_none()
    }

    pub fn execute(
        &self,
        principal: &Principal,
        context: &mut C,
        command_line: &str,
    ) -> Result<String, CommandError> {
        let mut parts = command_line.split_whitespace();
        let name = parts
            .next()
            .ok_or_else(|| CommandError::Unknown(String::new()))?;
        let command = self
            .commands
            .get(name)
            .ok_or_else(|| CommandError::Unknown(name.to_owned()))?;
        if let Some(permission) = &command.permission {
            if !principal.can(permission) {
                return Err(CommandError::PermissionDenied(permission.clone()));
            }
        }
        let arguments = parts.collect::<Vec<_>>();
        (command.handler)(context, &arguments)
    }
}
