use crate::errors::{Error, Result};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::Sha256;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const REQUEST_LIMIT: usize = 1024 * 1024;
pub const RESPONSE_LIMIT: usize = 64 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hello {
    pub nonce: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Challenge {
    pub nonce: String,
    pub proof: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Signed<T> {
    pub value: T,
    pub proof: String,
}

pub async fn read<T: DeserializeOwned>(
    reader: &mut (impl AsyncRead + Unpin),
    limit: usize,
) -> Result<T> {
    let length = reader.read_u32().await? as usize;
    if length == 0 || length > limit {
        return Err(Error::new(
            "managed_protocol",
            "Gateway frame exceeds the allowed size",
        ));
    }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes).await?;
    serde_json::from_slice(&bytes)
        .map_err(|_| Error::new("managed_protocol", "Invalid gateway message"))
}

pub async fn write<T: Serialize>(
    writer: &mut (impl AsyncWrite + Unpin),
    value: &T,
    limit: usize,
) -> Result<()> {
    let bytes = serde_json::to_vec(value)
        .map_err(|_| Error::new("managed_protocol", "Cannot encode gateway message"))?;
    if bytes.len() > limit {
        return Err(Error::new(
            "output_limit",
            "Gateway message exceeds the allowed size",
        ));
    }
    writer.write_u32(bytes.len() as u32).await?;
    writer.write_all(&bytes).await?;
    writer.flush().await?;
    Ok(())
}

pub fn proof<T: Serialize>(
    key: &str,
    domain: &str,
    client_nonce: &str,
    server_nonce: &str,
    value: &T,
) -> Result<String> {
    let bytes = mac(key, domain, client_nonce, server_nonce, value)?
        .finalize()
        .into_bytes();
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

pub fn verify<T: Serialize>(
    key: &str,
    domain: &str,
    client_nonce: &str,
    server_nonce: &str,
    value: &T,
    proof: &str,
) -> Result<()> {
    if proof.len() != 64 || !proof.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(auth_error());
    }
    let bytes: Vec<u8> = proof
        .as_bytes()
        .chunks_exact(2)
        .map(|chunk| {
            let digit = |c: u8| {
                if c.is_ascii_digit() {
                    c - b'0'
                } else {
                    c.to_ascii_lowercase() - b'a' + 10
                }
            };
            digit(chunk[0]) * 16 + digit(chunk[1])
        })
        .collect();
    mac(key, domain, client_nonce, server_nonce, value)?
        .verify_slice(&bytes)
        .map_err(|_| auth_error())
}

fn mac<T: Serialize>(
    key: &str,
    domain: &str,
    client_nonce: &str,
    server_nonce: &str,
    value: &T,
) -> Result<Hmac<Sha256>> {
    if key.len() != 64 || client_nonce.len() != 64 || server_nonce.len() != 64 {
        return Err(auth_error());
    }
    let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes()).map_err(|_| auth_error())?;
    mac.update(b"andromeda.gateway.v1\0");
    mac.update(domain.as_bytes());
    mac.update(&[0]);
    mac.update(client_nonce.as_bytes());
    mac.update(&[0]);
    mac.update(server_nonce.as_bytes());
    mac.update(&[0]);
    mac.update(
        &serde_json::to_vec(value)
            .map_err(|_| Error::new("managed_protocol", "Invalid signed request"))?,
    );
    Ok(mac)
}
pub fn auth_error() -> Error {
    Error::new(
        "gateway_auth_failed",
        "Cannot authenticate the local Andromeda desktop gateway",
    )
}
