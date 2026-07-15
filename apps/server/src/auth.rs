use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

const TOKEN_VERSION: &str = "v1";
const SHA256_BLOCK_SIZE: usize = 64;
const MIN_SECRET_BYTES: usize = 32;

#[derive(Debug, Clone)]
pub struct AuthService {
    required: bool,
    secret: Option<Vec<u8>>,
    clock_skew_seconds: u64,
    max_token_lifetime_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthError {
    TokenRequired,
    ServiceNotConfigured,
    InvalidToken,
    Expired,
    LifetimeTooLong,
}

impl AuthError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::TokenRequired => "auth.token_required",
            Self::ServiceNotConfigured => "auth.not_configured",
            Self::InvalidToken => "auth.invalid_token",
            Self::Expired => "auth.token_expired",
            Self::LifetimeTooLong => "auth.token_lifetime",
        }
    }

    pub const fn message(self) -> &'static str {
        match self {
            Self::TokenRequired => "Authentication token is required",
            Self::ServiceNotConfigured => "Server authentication is not configured",
            Self::InvalidToken => "Authentication token is invalid",
            Self::Expired => "Authentication token has expired",
            Self::LifetimeTooLong => "Authentication token lifetime exceeds the server policy",
        }
    }
}

impl AuthService {
    pub fn from_environment() -> Result<Self> {
        let required = env_bool("HONKNET_AUTH_REQUIRED", false);
        let secret = std::env::var("HONKNET_AUTH_SECRET")
            .ok()
            .map(|value| value.into_bytes());
        let clock_skew_seconds = env_u64("HONKNET_AUTH_CLOCK_SKEW_SECONDS", 30);
        let max_token_lifetime_seconds =
            env_u64("HONKNET_AUTH_MAX_TOKEN_LIFETIME_SECONDS", 2_592_000);

        if required {
            let configured = secret
                .as_ref()
                .context("HONKNET_AUTH_SECRET is required when authentication is enabled")?;
            if configured.len() < MIN_SECRET_BYTES {
                bail!(
                    "HONKNET_AUTH_SECRET must contain at least {MIN_SECRET_BYTES} bytes when authentication is enabled"
                );
            }
        }

        Ok(Self {
            required,
            secret,
            clock_skew_seconds,
            max_token_lifetime_seconds,
        })
    }

    pub const fn required(&self) -> bool {
        self.required
    }

    pub fn validate(&self, identity: &str, token: Option<&str>) -> Result<(), AuthError> {
        let Some(token) = token.filter(|value| !value.trim().is_empty()) else {
            return if self.required {
                Err(AuthError::TokenRequired)
            } else {
                Ok(())
            };
        };
        let Some(secret) = self.secret.as_deref() else {
            return Err(AuthError::ServiceNotConfigured);
        };

        let mut parts = token.split(':');
        let version = parts.next().ok_or(AuthError::InvalidToken)?;
        let expires = parts
            .next()
            .ok_or(AuthError::InvalidToken)?
            .parse::<u64>()
            .map_err(|_| AuthError::InvalidToken)?;
        let signature = parts.next().ok_or(AuthError::InvalidToken)?;
        if parts.next().is_some() || version != TOKEN_VERSION || signature.len() != 64 {
            return Err(AuthError::InvalidToken);
        }

        let now = unix_timestamp().map_err(|_| AuthError::InvalidToken)?;
        if expires.saturating_add(self.clock_skew_seconds) < now {
            return Err(AuthError::Expired);
        }
        if expires > now.saturating_add(self.max_token_lifetime_seconds) {
            return Err(AuthError::LifetimeTooLong);
        }

        let payload = token_payload(identity, expires);
        let expected = hex_encode(&hmac_sha256(secret, payload.as_bytes()));
        if !constant_time_eq(expected.as_bytes(), signature.as_bytes()) {
            return Err(AuthError::InvalidToken);
        }
        Ok(())
    }

    #[cfg(test)]
    fn issue_for_test(&self, identity: &str, expires: u64) -> String {
        let secret = self.secret.as_deref().expect("test auth secret");
        let payload = token_payload(identity, expires);
        let signature = hex_encode(&hmac_sha256(secret, payload.as_bytes()));
        format!("{TOKEN_VERSION}:{expires}:{signature}")
    }
}

fn token_payload(identity: &str, expires: u64) -> String {
    format!("{TOKEN_VERSION}\n{identity}\n{expires}")
}

fn hmac_sha256(secret: &[u8], message: &[u8]) -> [u8; 32] {
    let mut key = [0_u8; SHA256_BLOCK_SIZE];
    if secret.len() > SHA256_BLOCK_SIZE {
        let digest = Sha256::digest(secret);
        key[..digest.len()].copy_from_slice(&digest);
    } else {
        key[..secret.len()].copy_from_slice(secret);
    }

    let mut inner_pad = [0x36_u8; SHA256_BLOCK_SIZE];
    let mut outer_pad = [0x5c_u8; SHA256_BLOCK_SIZE];
    for index in 0..SHA256_BLOCK_SIZE {
        inner_pad[index] ^= key[index];
        outer_pad[index] ^= key[index];
    }

    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(message);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner_hash);
    outer.finalize().into()
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0_u8;
    for (&left_byte, &right_byte) in left.iter().zip(right) {
        difference |= left_byte ^ right_byte;
    }
    difference == 0
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn unix_timestamp() -> Result<u64, std::time::SystemTimeError> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

fn env_bool(name: &str, fallback: bool) -> bool {
    std::env::var(name)
        .ok()
        .and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(fallback)
}

fn env_u64(name: &str, fallback: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{AuthError, AuthService};

    fn service(required: bool) -> AuthService {
        AuthService {
            required,
            secret: Some(b"this-is-a-test-secret-with-at-least-32-bytes".to_vec()),
            clock_skew_seconds: 0,
            max_token_lifetime_seconds: 3_600,
        }
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_secs()
    }

    #[test]
    fn validates_signed_identity_tokens() {
        let service = service(true);
        let token = service.issue_for_test("user-1", now() + 60);
        assert_eq!(service.validate("user-1", Some(&token)), Ok(()));
        assert_eq!(
            service.validate("user-2", Some(&token)),
            Err(AuthError::InvalidToken)
        );
    }

    #[test]
    fn guest_mode_accepts_missing_token() {
        assert_eq!(service(false).validate("guest-test", None), Ok(()));
        assert_eq!(
            service(true).validate("guest-test", None),
            Err(AuthError::TokenRequired)
        );
    }
}
