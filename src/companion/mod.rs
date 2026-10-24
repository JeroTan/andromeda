//! Optional Android AccessibilityService accessed through the authorized ADB transport.
use crate::{
    device::Adb,
    errors::{Error, Result},
    projects::{Registry, atomic_write, random_hex, read_bounded},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{path::PathBuf, time::Duration};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Clone)]
pub struct Store {
    registry: Registry,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pairing {
    token: String,
}

impl Store {
    pub fn open(directory: PathBuf) -> Result<Self> {
        Ok(Self {
            registry: Registry::open(directory)?,
        })
    }
    fn path(&self, serial: &str) -> PathBuf {
        let hash = Sha256::digest(serial.as_bytes());
        self.registry.directory().join(format!(
            "device-{}.json",
            hash.iter().map(|b| format!("{b:02x}")).collect::<String>()
        ))
    }
    pub fn is_paired(&self, serial: &str) -> Result<bool> {
        Ok(self.load(serial)?.is_some())
    }
    pub fn forget(&self, serial: &str) -> Result<()> {
        match std::fs::remove_file(self.path(serial)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(crate::projects::config_io(error)),
        }
    }
    fn load(&self, serial: &str) -> Result<Option<Pairing>> {
        let path = self.path(serial);
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(crate::projects::config_io(error)),
        };
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(Error::new(
                "invalid_pairing",
                "Pairing state must be a regular private file",
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(Error::new(
                    "insecure_permissions",
                    "Pairing state must be accessible only to its owner",
                ));
            }
        }
        let pairing: Pairing =
            serde_json::from_slice(&read_bounded(&path, 1024)?).map_err(|_| {
                Error::new(
                    "invalid_pairing",
                    "Pair this device again; its saved credential is invalid",
                )
            })?;
        if pairing.token.len() != 64 || !pairing.token.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(Error::new("invalid_pairing", "Invalid saved pairing token"));
        }
        Ok(Some(pairing))
    }
}

#[derive(Clone)]
pub struct Companion {
    adb: Adb,
    store: Store,
    project: Option<String>,
}

impl Companion {
    pub fn new(adb: Adb, store: Store) -> Self {
        Self {
            adb,
            store,
            project: None,
        }
    }
    pub(crate) fn scoped(mut self, project: String) -> Self {
        self.project = Some(project);
        self
    }
    /// Only called by the operator's setup interface, never an MCP tool.
    pub async fn pair(&self, serial: &str) -> Result<()> {
        let token = random_hex(32)?;
        let result = self
            .request(serial, json!({"version":1,"method":"pair","token":token}))
            .await?;
        if result.get("paired") != Some(&Value::Bool(true)) {
            return Err(Error::new(
                "companion_protocol",
                "Phone did not confirm pairing",
            ));
        }
        atomic_write(
            &self.store.path(serial),
            &serde_json::to_vec(&Pairing { token })
                .map_err(|_| Error::new("invalid_pairing", "Could not encode pairing state"))?,
        )
    }
    pub async fn status(&self, serial: &str) -> Result<Value> {
        self.call(serial, "status", json!({})).await
    }
    pub async fn stop(&self, serial: &str) -> Result<Value> {
        self.call(serial, "stop", json!({})).await
    }
    pub fn is_paired(&self, serial: &str) -> Result<bool> {
        self.store.is_paired(serial)
    }
    pub async fn call(&self, serial: &str, method: &str, mut arguments: Value) -> Result<Value> {
        let pairing = self.store.load(serial)?.ok_or_else(|| {
            Error::new(
                "companion_not_paired",
                "Open pairing on the phone and pair it from Andromeda on the computer",
            )
        })?;
        let object = arguments
            .as_object_mut()
            .ok_or_else(|| Error::invalid("Companion arguments must be an object"))?;
        if let Some(project) = &self.project {
            object.insert("project_id".into(), json!(project));
        }
        object.insert("version".into(), json!(1));
        object.insert("method".into(), json!(method));
        object.insert("token".into(), json!(pairing.token));
        self.request(serial, arguments).await.map_err(|mut error| {
            if method == "gesture" || method == "type_text" {
                error.retryable = false;
            }
            error
        })
    }
    async fn request(&self, serial: &str, request: Value) -> Result<Value> {
        let bytes = serde_json::to_vec(&request)
            .map_err(|_| Error::new("companion_protocol", "Cannot encode companion request"))?;
        if bytes.len() > 1024 * 1024 {
            return Err(Error::invalid("Companion request exceeds 1 MiB"));
        }
        tokio::time::timeout(Duration::from_secs(15),async {
            let mut socket=self.adb.service(serial,"localabstract:andromeda-v1").await.map_err(|mut error|{if error.code=="adb_rejected" {error.code="companion_unavailable".into();error.message="Enable Andromeda accessibility on the selected phone; reconnect USB if the device is offline".into();}error})?;
            socket.write_u32(bytes.len() as u32).await?;socket.write_all(&bytes).await?;
            let length=socket.read_u32().await? as usize;
            if length==0 || length>4*1024*1024{return Err(Error::new("companion_protocol","Invalid companion response size"));}
            let mut response=vec![0;length];socket.read_exact(&mut response).await?;
            let response:Value=serde_json::from_slice(&response).map_err(|_|Error::new("companion_protocol","Phone returned invalid JSON"))?;
            match response.get("success").and_then(Value::as_bool) {
                Some(true)=>response.get("data").cloned().ok_or_else(||Error::new("companion_protocol","Phone omitted result data")),
                Some(false)=>Err(Error::new(response["error"]["code"].as_str().unwrap_or("companion_error"),response["error"]["message"].as_str().unwrap_or("Companion request failed"))),
                None=>Err(Error::new("companion_protocol","Phone omitted result status")),
            }
        }).await.map_err(|_|Error::new("timeout","Companion deadline exceeded; observe before retrying input"))?
    }
}
