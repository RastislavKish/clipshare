use anyhow::{bail, Context};
use derive_getters::Getters;
use serde::{Serialize, Deserialize};

use crate::cryptography::{encrypt, decrypt, calculate_pseudosalted_password_hash};

/// A wrapper structure for communication with a clipshare server instance.
pub struct Clipshare {
    host: String,
    }
impl Clipshare {

    /// Creates a new instance of Clipshare
    pub fn new(host: &str) -> Clipshare {
        let host=host.to_string();

        Clipshare { host }
        }

    /// Gets the raw content of a shared clipboard.
    pub fn get_content(&self, clipboard_id: &str) -> Result<String, anyhow::Error> {
        let client=reqwest::blocking::Client::builder()
        .https_only(true)
        .build()?;

        let res=client.get(format!("{}/clipboard/{}", self.host, clipboard_id))
        .send().context("Unable to connect to the shared clipboard.")?;

        if !res.status().is_success() {
            bail!("{}", res.text()?);
            }

        let encrypted_content=res.text().context("Unable to access the body of shared clipboard get request.")?;

        Ok(encrypted_content)
        }

    /// Sets the raw content of a shared clipboard.
    pub fn set_content(&self, clipboard_id: &str, content: &str) -> Result<(), anyhow::Error> {
        let client=reqwest::blocking::Client::builder()
        .https_only(true)
        .build()?;

        let res=client.post(format!("{}/clipboard/{}", self.host, clipboard_id))
        .body(content.to_string())
        .send().context("Unable to connect to the shared clipboard")?;

        if !res.status().is_success() {
            bail!("{}", res.text()?);
            }

        Ok(())
        }
    }

/// A wrapper structure for working with shared clipboards.
/// While Clipshare represents a Clipshare server instance and its functionality, SharedClipboard is a structure that represents shared clipboards as functional units.
/// Since one Clipshare server can embrace any number of shared clipboards for the user.
/// Although the Clipshare objects are not shared among SharedClipboards even if multiple Clipshare objects refer to the same server instance, for ergonomical reasons.
/// SharedClipboard is the structure that gets to serialize/deserialize and encrypt/decrypt the content to be put into a shared clipboard. The Clipshare structure has only access to the resulting encrypted data and the clipboard id.
#[derive(Getters)]
pub struct SharedClipboard {
    name: String,
    clipshare: Clipshare,
    clipboard_id: String,
    password: String,
    }
impl SharedClipboard {

    /// Creates a new instance of SharedClipboard.
    pub fn new(name: &str, clipshare: Clipshare, password: &str) -> SharedClipboard {
        SharedClipboard {
            name: name.to_string(),
            clipshare,
            clipboard_id: calculate_pseudosalted_password_hash(password),
            password: password.to_string(),
            }
        }

    /// Gets the content of the shared clipboard.
    pub fn get_content(&self) -> Result<SharedClipboardContent, anyhow::Error> {
        let encrypted_content=self.clipshare.get_content(&self.clipboard_id)?;
        let serialized_content=decrypt(&encrypted_content, &self.password).context("Unable to decrypt the shared clipboard.")?;

        let content: SharedClipboardContent=serde_json::from_str(&serialized_content)
        .context("Unable to deserialize the shared clipboard content.")?;

        Ok(content)
        }

    /// Sets the content of the shared clipboard.
    pub fn set_content(&self, content: SharedClipboardContent) -> Result<(), anyhow::Error> {
        let serialized_content=serde_json::to_string(&content)
        .context("Unable to serialize the content for the shared clipboard")?;

        let encrypted_content=encrypt(&serialized_content, &self.password)?;
        self.clipshare.set_content(&self.clipboard_id, &encrypted_content)?;

        Ok(())
        }
    }

/// An enum representing the content of the SharedClipboard.
#[derive(Serialize, Deserialize)]
pub enum SharedClipboardContent {
    Text(String),
    }

