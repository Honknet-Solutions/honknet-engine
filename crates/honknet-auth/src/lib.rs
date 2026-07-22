use hmac::{Hmac, Mac};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use uuid::Uuid;
type H = Hmac<Sha256>;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub subject: String,
    pub session: String,
    pub roles: Vec<String>,
    pub issued: u64,
    pub expires: u64,
    pub nonce: String,
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("invalid token")]
    Invalid,
    #[error("expired token")]
    Expired,
    #[error("revoked token")]
    Revoked,
    #[error("banned: {0}")]
    Banned(String),
    #[error("OIDC: {0}")]
    Oidc(String),
}

#[derive(Clone)]
pub struct TokenIssuer {
    secret: Arc<Vec<u8>>,
    revoked: Arc<RwLock<HashSet<String>>>,
}

impl TokenIssuer {
    pub fn new(secret: impl AsRef<[u8]>) -> Self {
        Self {
            secret: Arc::new(secret.as_ref().to_vec()),
            revoked: Default::default(),
        }
    }
    pub fn issue(&self, subject: &str, roles: Vec<String>, ttl: u64) -> String {
        let now = now();
        let c = Claims {
            subject: subject.into(),
            session: Uuid::new_v4().to_string(),
            roles,
            issued: now,
            expires: now + ttl,
            nonce: Uuid::new_v4().to_string(),
        };
        let body = serde_json::to_vec(&c).unwrap_or_default();
        let mut mac = H::new_from_slice(&self.secret).expect("HMAC accepts arbitrary key length");
        mac.update(&body);
        format!(
            "{}.{}",
            hex::encode(body),
            hex::encode(mac.finalize().into_bytes())
        )
    }
    pub fn verify(&self, t: &str) -> Result<Claims, AuthError> {
        let (a, b) = t.split_once('.').ok_or(AuthError::Invalid)?;
        let body = hex::decode(a).map_err(|_| AuthError::Invalid)?;
        let sig = hex::decode(b).map_err(|_| AuthError::Invalid)?;
        let mut mac = H::new_from_slice(&self.secret).map_err(|_| AuthError::Invalid)?;
        mac.update(&body);
        mac.verify_slice(&sig).map_err(|_| AuthError::Invalid)?;
        let c: Claims = serde_json::from_slice(&body).map_err(|_| AuthError::Invalid)?;
        if c.expires < now() {
            return Err(AuthError::Expired);
        }
        if self.revoked.read().contains(&c.session) {
            return Err(AuthError::Revoked);
        }
        Ok(c)
    }
    pub fn revoke(&self, session: &str) {
        self.revoked.write().insert(session.into());
    }
}

#[derive(Debug, Clone)]
pub struct Ban {
    pub subject: String,
    pub reason: String,
    pub expires: Option<u64>,
}

#[derive(Default, Clone)]
pub struct AccessControl {
    bans: Arc<RwLock<HashMap<String, Ban>>>,
    sessions: Arc<RwLock<HashMap<String, String>>>,
}

impl AccessControl {
    pub fn ban(&self, b: Ban) {
        self.bans.write().insert(b.subject.clone(), b);
    }
    pub fn check(&self, subject: &str) -> Result<(), AuthError> {
        if let Some(b) = self.bans.read().get(subject) {
            if b.expires.is_none_or(|x| x > now()) {
                return Err(AuthError::Banned(b.reason.clone()));
            }
        }
        Ok(())
    }
    pub fn claim_session(&self, subject: &str, session: &str) -> Option<String> {
        self.sessions.write().insert(subject.into(), session.into())
    }
    pub fn release_session(&self, subject: &str, session: &str) {
        let mut s = self.sessions.write();
        if s.get(subject).is_some_and(|x| x == session) {
            s.remove(subject);
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct OidcDiscovery {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub jwks_uri: String,
}

pub fn discover_oidc(issuer: &str) -> Result<OidcDiscovery, AuthError> {
    let discovery_url = format!(
        "{}/.well-known/openid-configuration",
        issuer.trim_end_matches('/'),
    );

    reqwest::blocking::get(discovery_url)
        .map_err(|error| AuthError::Oidc(error.to_string()))?
        .error_for_status()
        .map_err(|error| AuthError::Oidc(error.to_string()))?
        .json()
        .map_err(|error| AuthError::Oidc(error.to_string()))
}

pub fn guest_identity() -> String {
    format!("guest-{}", Uuid::new_v4())
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
