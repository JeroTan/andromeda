//! Project registration and authority shared by desktop setup and managed MCP.
use crate::errors::{Error, Result};
mod clients;
mod opencode;
pub use clients::{Client, ConfigChange};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub root: PathBuf,
    pub name: String,
    pub serial: Option<String>,
    pub allowed_tools: Vec<String>,
    pub instruction_file: PathBuf,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize)]
struct StoredProject {
    project: Project,
    credential: String,
}

#[derive(Clone)]
pub struct Registry {
    directory: PathBuf,
}

impl Registry {
    pub(crate) fn directory(&self) -> &Path {
        &self.directory
    }
    pub(crate) fn credential(&self, id: &str) -> Result<String> {
        Ok(self.load(id)?.credential)
    }
    pub fn default_directory() -> Result<PathBuf> {
        directories::ProjectDirs::from("io", "Andromeda", "Andromeda")
            .map(|p| p.data_local_dir().to_owned())
            .ok_or_else(|| {
                Error::new(
                    "config_error",
                    "Cannot locate this user's application data directory",
                )
            })
    }

    pub fn open(directory: PathBuf) -> Result<Self> {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&directory)?;
        check_private(&directory)?;
        Ok(Self {
            directory: directory.canonicalize()?,
        })
    }

    pub fn register(&self, root: &Path) -> Result<Project> {
        let root = root
            .canonicalize()
            .map_err(|_| Error::invalid("Choose an existing project directory"))?;
        if !root.is_dir() {
            return Err(Error::invalid("Project root must be a directory"));
        }
        let existing = self.list()?;
        if let Some(project) = existing.iter().find(|p| p.root == root) {
            return Ok(project.clone());
        }
        if existing.len() >= 100 {
            return Err(Error::new(
                "project_limit",
                "At most 100 projects can be registered",
            ));
        }
        let mut allowed_tools: Vec<String> = crate::mcp::tools()
            .into_iter()
            .filter(|t| {
                t.annotations
                    .as_ref()
                    .is_some_and(|a| a.read_only_hint == Some(true))
            })
            .map(|t| t.name.into_owned())
            .collect();
        allowed_tools.push("adm_stop".into());
        let project = Project {
            id: random_hex(16)?,
            name: root
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            root,
            serial: None,
            allowed_tools,
            instruction_file: "ANDROMEDA.md".into(),
            enabled: true,
        };
        self.save(&StoredProject {
            project: project.clone(),
            credential: random_hex(32)?,
        })?;
        Ok(project)
    }

    pub fn list(&self) -> Result<Vec<Project>> {
        let mut projects = Vec::new();
        for entry in fs::read_dir(&self.directory)? {
            let path = entry?.path();
            if path.extension().is_some_and(|s| s == "json")
                && let Some(id) = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .filter(|s| valid_id(s))
            {
                projects.push(self.load(id)?.project);
            }
            if projects.len() > 100 {
                return Err(Error::new(
                    "project_limit",
                    "Project registry exceeds 100 entries",
                ));
            }
        }
        projects.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
        Ok(projects)
    }

    pub fn get(&self, id: &str) -> Result<Project> {
        Ok(self.load(id)?.project)
    }

    /// Remove a project registration without touching the project directory.
    pub fn remove(&self, id: &str) -> Result<()> {
        let path = self.path(id)?;
        self.load(id)?;
        fs::remove_file(path).map_err(config_io)
    }

    pub fn authorize(&self, id: &str, working_directory: &Path) -> Result<Project> {
        let project = self.load(id)?.project;
        if !project.enabled {
            return Err(Error::new(
                "project_disabled",
                "This project's Andromeda connection is disabled",
            ));
        }
        let root = project.root.canonicalize().map_err(|_| {
            Error::new(
                "project_missing",
                "The registered project directory is unavailable",
            )
        })?;
        let current = working_directory.canonicalize().map_err(|_| {
            Error::new(
                "project_scope_mismatch",
                "Cannot verify the MCP working directory",
            )
        })?;
        if root != project.root || !current.is_dir() || !current.starts_with(&root) {
            return Err(Error::new(
                "project_scope_mismatch",
                "This MCP configuration is registered for a different project directory",
            ));
        }
        Ok(project)
    }

    pub fn update(
        &self,
        id: &str,
        serial: Option<String>,
        mut allowed_tools: Vec<String>,
        instruction_file: PathBuf,
        enabled: bool,
    ) -> Result<Project> {
        validate_relative(&instruction_file)?;
        let known: Vec<String> = crate::mcp::tools()
            .into_iter()
            .map(|t| t.name.into_owned())
            .collect();
        if allowed_tools.iter().any(|name| !known.contains(name)) {
            return Err(Error::invalid("Policy contains an unknown Andromeda tool"));
        }
        if serial.as_ref().is_some_and(|s| {
            s.is_empty()
                || s.len() > 256
                || !s
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_.:-".contains(&c))
        }) {
            return Err(Error::invalid("Invalid Android serial"));
        }
        allowed_tools.push("adm_stop".into());
        allowed_tools.sort();
        allowed_tools.dedup();
        let mut stored = self.load(id)?;
        if stored
            .project
            .root
            .join(&instruction_file)
            .try_exists()
            .map_err(config_io)?
        {
            scoped_path(&stored.project.root, &instruction_file)?;
        }
        stored.project.serial = serial;
        stored.project.allowed_tools = allowed_tools;
        stored.project.instruction_file = instruction_file;
        stored.project.enabled = enabled;
        self.save(&stored)?;
        Ok(stored.project)
    }

    pub fn instructions(&self, id: &str) -> Result<Option<String>> {
        let project = self.load(id)?.project;
        validate_relative(&project.instruction_file)?;
        let requested = project.root.join(&project.instruction_file);
        if !requested.try_exists().map_err(config_io)? {
            return Ok(None);
        }
        let path = scoped_path(&project.root, &project.instruction_file)?;
        let bytes = read_bounded(&path, 65536)?;
        String::from_utf8(bytes).map(Some).map_err(|_| {
            Error::new(
                "invalid_instructions",
                "Project instructions must be UTF-8 text",
            )
        })
    }

    /// Save only the version reviewed in the editor; concurrent external edits win.
    pub fn save_instructions(&self, id: &str, text: &str, previous: Option<&str>) -> Result<()> {
        if text.len() > 65536 {
            return Err(Error::invalid(
                "Project instructions must fit within 64 KiB",
            ));
        }
        let project = self.get(id)?;
        if project.root.canonicalize().map_err(config_io)? != project.root {
            return Err(Error::new(
                "project_scope_mismatch",
                "The registered directory changed",
            ));
        }
        validate_relative(&project.instruction_file)?;
        let path = project.root.join(&project.instruction_file);
        if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::new(
                "project_scope_mismatch",
                "Instruction edits cannot replace a symbolic link",
            ));
        }
        let parent = path
            .parent()
            .ok_or_else(|| Error::invalid("Instruction file needs a parent"))?;
        if !parent
            .canonicalize()
            .map_err(config_io)?
            .starts_with(&project.root)
        {
            return Err(Error::new(
                "project_scope_mismatch",
                "Instruction file resolves outside this project",
            ));
        }
        if self.instructions(id)?.as_deref() != previous {
            return Err(Error::new(
                "instructions_changed",
                "The file changed after it was opened; reload before saving",
            ));
        }
        atomic_write(&path, text.as_bytes())
    }

    pub fn starter_instructions() -> &'static str {
        include_str!("starter.md")
    }

    fn path(&self, id: &str) -> Result<PathBuf> {
        if !valid_id(id) {
            return Err(Error::invalid("Invalid project ID"));
        }
        Ok(self.directory.join(format!("{id}.json")))
    }
    fn load(&self, id: &str) -> Result<StoredProject> {
        let path = self.path(id)?;
        if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::new(
                "config_error",
                "Registry entries cannot be symbolic links",
            ));
        }
        check_private(&path)?;
        let stored: StoredProject = serde_json::from_slice(&read_bounded(&path, 65536)?)
            .map_err(|e| Error::new("config_error", e.to_string()))?;
        if stored.project.id != id || stored.credential.len() != 64 {
            return Err(Error::new("config_error", "Invalid project registry entry"));
        }
        Ok(stored)
    }
    fn save(&self, stored: &StoredProject) -> Result<()> {
        let data = serde_json::to_vec_pretty(stored)
            .map_err(|e| Error::new("config_error", e.to_string()))?;
        atomic_write(&self.path(&stored.project.id)?, &data)
    }
}

fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

pub(crate) fn random_hex(bytes: usize) -> Result<String> {
    let mut random = vec![0; bytes];
    getrandom::fill(&mut random)
        .map_err(|_| Error::new("entropy_unavailable", "System randomness is unavailable"))?;
    Ok(random.iter().map(|n| format!("{n:02x}")).collect())
}

fn validate_relative(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(Error::invalid(
            "Choose a relative instruction-file path within the project",
        ));
    }
    Ok(())
}

fn scoped_path(root: &Path, relative: &Path) -> Result<PathBuf> {
    validate_relative(relative)?;
    let resolved = root.join(relative).canonicalize().map_err(config_io)?;
    if !resolved.starts_with(root) {
        return Err(Error::new(
            "project_scope_mismatch",
            "The selected file resolves outside the registered project",
        ));
    }
    Ok(resolved)
}

fn check_private(path: &Path) -> Result<()> {
    let metadata = fs::metadata(path).map_err(config_io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(Error::new(
                "insecure_permissions",
                "Andromeda application state must be accessible only to its owner",
            ));
        }
    }
    #[cfg(not(unix))]
    let _ = metadata;
    Ok(())
}

pub(crate) fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(config_io)?
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(config_io)?;
    if bytes.len() > limit {
        return Err(Error::new(
            "file_limit",
            format!("File exceeds {limit} bytes"),
        ));
    }
    Ok(bytes)
}

pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::invalid("File needs a parent directory"))?;
    let temporary = parent.join(format!(".andromeda-{}.tmp", random_hex(16)?));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| -> Result<()> {
        let mut file = options.open(&temporary).map_err(config_io)?;
        file.write_all(bytes).map_err(config_io)?;
        file.sync_all().map_err(config_io)?;
        drop(file);
        if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::new(
                "config_error",
                "Refusing to replace a symbolic link",
            ));
        }
        fs::rename(&temporary, path).map_err(config_io)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub(crate) fn config_io(error: std::io::Error) -> Error {
    Error::new("configuration_io", error.to_string())
}
