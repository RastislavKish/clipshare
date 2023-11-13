use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;

use anyhow::Context;
use arboard::Clipboard;
use clap::{Args, Parser, Subcommand};
use enigo::{Enigo, KeyboardControllable};
use global_hotkey::{
    GlobalHotKeyManager, GlobalHotKeyEvent, HotKeyState,
    };
use lazy_static::lazy_static;
use notify_rust::Notification;
use winit::event_loop::{ControlFlow, EventLoopBuilder};

mod configuration;
mod core;
mod cryptography;

use crate::configuration::Config;
use crate::core::{Clipshare, SharedClipboard, SharedClipboardContent};

lazy_static! {
    static ref CLIPBOARD: Mutex<Clipboard>=Mutex::new(Clipboard::new().unwrap());
    }

#[derive(Parser)]
#[command(author, version, about, long_about=None)]
//#[command(propagate_version=true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    }

#[derive(Subcommand)]
enum Commands {
    /// Launches the Clipbshare daemon
    Daemon(DaemonArgs),
    /// Copies content to the shared clipboard
    Copy(CopyArgs),
    /// Pastes content from the shared clipboard
    Paste(PasteArgs),
    }

#[derive(Args)]
struct DaemonArgs {

    }

#[derive(Args)]
struct CopyArgs {
    /// Sets the sync mode
    #[arg(short, long)]
    sync_mode: bool,
    /// Sets the clipboard to use
    #[arg(short, long)]
    clipboard: Option<String>,
    }

#[derive(Args)]
struct PasteArgs {
    /// Sets the sync mode
    #[arg(short, long)]
    sync_mode: bool,
    /// Sets the clipboard to use
    #[arg(short, long)]
    clipboard: Option<String>,
    }

fn main() -> Result<(), anyhow::Error> {
    let cli=Cli::parse();
    let config=Config::load()?;

    match &cli.command {
        Commands::Daemon(args) => daemon_command(args, &config),
        Commands::Copy(args) => copy_command(args, &config),
        Commands::Paste(args) => paste_command(args, &config),
        }
    .unwrap_or_else(|e| notify_err(e, true));

    Ok(())
    }

fn daemon_command(_args: &DaemonArgs, config: &Config) -> Result<(), anyhow::Error> {
    let event_loop=EventLoopBuilder::new().build()?;

    let manager=GlobalHotKeyManager::new().context("Unable to optain the global hotkey manager")?;

    let mut copy_hotkeys: HashMap<u32, Rc<SharedClipboard>>=HashMap::new();
    let mut paste_hotkeys: HashMap<u32, Rc<SharedClipboard>>=HashMap::new();
    let mut sync_copy_hotkeys: HashMap<u32, Rc<SharedClipboard>>=HashMap::new();
    let mut sync_paste_hotkeys: HashMap<u32, Rc<SharedClipboard>>=HashMap::new();

    for (name, configuration) in config.clipboards() {

        let clipshare=Clipshare::new(configuration.host());
        let shared_clipboard=Rc::new(SharedClipboard::new(name, clipshare, configuration.password()));

        if !configuration.copy_hotkey().is_empty() {
            if let Ok(copy_hotkey)=configuration.copy_hotkey().parse() {
                match manager.register(copy_hotkey) {
                    Ok(_) => { copy_hotkeys.insert(copy_hotkey.id(), shared_clipboard.clone()); },
                    Err(e) => notify(&format!("Unable to register the copy hotkey of {name} clipboard. {e}"), true),
                    };
                }
            else {
                notify(&format!("Unable to parse copy hotkey of {name} clipboard."), true);
                }
            }
        if !configuration.paste_hotkey().is_empty() {
            if let Ok(paste_hotkey)=configuration.paste_hotkey().parse() {
                match manager.register(paste_hotkey) {
                    Ok(_) => { paste_hotkeys.insert(paste_hotkey.id(), shared_clipboard.clone()); },
                    Err(e) => notify(&format!("Unable to register the paste hotkey of {name} clipboard. {e}"), true),
                    };
                }
            else {
                notify(&format!("Unable to parse paste hotkey of {name} clipboard."), true);
                }
            }
        if !configuration.sync_copy_hotkey().is_empty() {
            if let Ok(sync_copy_hotkey)=configuration.sync_copy_hotkey().parse() {
                match manager.register(sync_copy_hotkey) {
                    Ok(_) => { sync_copy_hotkeys.insert(sync_copy_hotkey.id(), shared_clipboard.clone()); },
                    Err(e) => notify(&format!("Unable to register the sync copy hotkey of {name} clipboard. {e}"), true),
                    };
                }
            else {
                notify(&format!("Unable to parse sync copy hotkey of {name} clipboard."), true);
                }
            }
        if !configuration.sync_paste_hotkey().is_empty() {
            if let Ok(sync_paste_hotkey)=configuration.sync_paste_hotkey().parse() {
                match manager.register(sync_paste_hotkey) {
                    Ok(_) => { sync_paste_hotkeys.insert(sync_paste_hotkey.id(), shared_clipboard.clone()); },
                    Err(e) => notify(&format!("Unable to register the sync paste hotkey of {name} clipboard. {e}"), true),
                    };
                }
            else {
                notify(&format!("Unable to parse sync paste hotkey of {name} clipboard."), true);
                }
            }

        }

    let global_hotkey_channel=GlobalHotKeyEvent::receiver();

    event_loop.run(move |_event, event_loop| {
        event_loop.set_control_flow(ControlFlow::Poll);

        if let Ok(event)=global_hotkey_channel.try_recv() {
            if event.state()!=HotKeyState::Released {
                return;
                }

            if copy_hotkeys.contains_key(&event.id()) {
                copy(copy_hotkeys[&event.id()].clone())
                .unwrap_or_else(|e| notify_err(e, true));
                }
            else if paste_hotkeys.contains_key(&event.id()) {
                paste(paste_hotkeys[&event.id()].clone())
                .unwrap_or_else(|e| notify_err(e, true));
                }
            else if sync_copy_hotkeys.contains_key(&event.id()) {
                sync_copy(sync_copy_hotkeys[&event.id()].clone())
                .unwrap_or_else(|e| notify_err(e, true));
                }
            else if sync_paste_hotkeys.contains_key(&event.id()) {
                sync_paste(sync_paste_hotkeys[&event.id()].clone())
                .unwrap_or_else(|e| notify_err(e, true));
                }
            }
        })?;

    Ok(())
    }
fn copy_command(args: &CopyArgs, config: &Config) -> Result<(), anyhow::Error> {
    let clipboard_name=match &args.clipboard {
        Some(c) => c.to_string(),
        None => config.default_clipboard().to_string(),
        };

    let shared_clipboard=get_shared_clipboard(&clipboard_name, config)?;

    if !args.sync_mode {
        copy(shared_clipboard)?;
        }
    else {
        sync_copy(shared_clipboard)?;
        }

    Ok(())
    }
fn paste_command(args: &PasteArgs, config: &Config) -> Result<(), anyhow::Error> {
    let clipboard_name=match &args.clipboard {
        Some(c) => c.to_string(),
        None => config.default_clipboard().to_string(),
        };

    let shared_clipboard=get_shared_clipboard(&clipboard_name, config)?;

    if !args.sync_mode {
        paste(shared_clipboard)?;
        }
    else {
        sync_paste(shared_clipboard)?;
        }

    Ok(())
    }

/// Copyes content from environment to the shared clipboard by emulating a Ctrl+C key press.
fn copy(shared_clipboard: Rc<SharedClipboard>) -> Result<(), anyhow::Error> {
    let mut clipboard=CLIPBOARD.lock().unwrap();

    let original_system_clipboard_text=clipboard_get_text(&mut clipboard).context("Unable to read from the system clipboard")?;

    let mut enigo=Enigo::new();

    clipboard.set_text("").context("Unable to write to system clipboard")?;
    std::thread::sleep(std::time::Duration::from_millis(500));
    enigo.key_sequence_parse("{+CTRL}c{-CTRL}");
    std::thread::sleep(std::time::Duration::from_millis(100));

    let system_clipboard_text=clipboard_get_text(&mut clipboard).context("Unable to read from the system clipboard")?;

    if system_clipboard_text.is_empty() {
        clipboard.set_text(original_system_clipboard_text).context("Unable to write to system clipboard")?;
        notify("Nothing to copy", true);

        return Ok(());
        }

    let shared_clipboard_content=SharedClipboardContent::Text(system_clipboard_text);
    shared_clipboard.set_content(shared_clipboard_content)
    .context("Unable to access the shared clipboard")?;

    clipboard.set_text(original_system_clipboard_text).context("Unable to write to system clipboard")?;
    notify(&format!("Copied to {}", shared_clipboard.name()), true);

    Ok(())
    }

/// Pastes content from environment to the shared clipboard by emulating a Ctrl+C key press.
fn paste(shared_clipboard: Rc<SharedClipboard>) -> Result<(), anyhow::Error> {
    let mut clipboard=CLIPBOARD.lock().unwrap();

    let original_system_clipboard_text=clipboard_get_text(&mut clipboard).context("Unable to read from the system clipboard")?;

    let shared_clipboard_content=shared_clipboard.get_content()?;

    match shared_clipboard_content {
        SharedClipboardContent::Text(text) => {
            clipboard.set_text(&text).context("Unable to write to system clipboard")?;

            let mut enigo=Enigo::new();
            enigo.key_sequence_parse("{+CTRL}v{-CTRL}");
            std::thread::sleep(std::time::Duration::from_millis(500));

            notify(&format!("Pasted from {}", shared_clipboard.name()), true);
            },
        }

    clipboard.set_text(original_system_clipboard_text).context("Unable to write to system clipboard")?;

    Ok(())
    }

/// Copyes content from the system clipboard to the shared clipboard.
/// Note: sync refers to system and shared clipboard synchronization, not to  be confused with programming paradigm.
fn sync_copy(shared_clipboard: Rc<SharedClipboard>) -> Result<(), anyhow::Error> {
    let mut clipboard=CLIPBOARD.lock().unwrap();

    let content=clipboard_get_text(&mut clipboard).context("Unable to read from the system clipboard")?;

    if content.is_empty() {
        notify("Nothing to copy", true);
        return Ok(());
        }

    let shared_clipboard_content=SharedClipboardContent::Text(content);
    shared_clipboard.set_content(shared_clipboard_content)
    .context("Unable to access the shared clipboard")?;

    notify(&format!("Sync-copied to {}", shared_clipboard.name()), true);

    Ok(())
    }

/// Pastes content from the shared clipboard to the system clipboard.
/// Note: sync refers to system and shared clipboard synchronization, not to  be confused with programming paradigm.
fn sync_paste(shared_clipboard: Rc<SharedClipboard>) -> Result<(), anyhow::Error> {
    let mut clipboard=CLIPBOARD.lock().unwrap();

    let shared_clipboard_content=shared_clipboard.get_content().context("Unable to access the shared clipboard")?;

    match shared_clipboard_content {
        SharedClipboardContent::Text(text) => {
            clipboard.set_text(&text).context("Unable to write to the system clipboard")?;
            notify(&format!("Sync-pasted from {}", shared_clipboard.name()), true);
            },
        };

    Ok(())
    }

/// A helper method returning empty string when the system clipboard is empty, instead of throwing an error
fn clipboard_get_text(clipboard: &mut Clipboard) -> Result<String, arboard::Error> {
    match clipboard.get_text() {
        Ok(text) => Ok(text),
        Err(e) => {
            if let arboard::Error::ContentNotAvailable=e {
                return Ok(String::new());
                }

            Err(e)
            }
        }
    }

/// a wrapper for getting SharedClipboard instance
fn get_shared_clipboard(clipboard_name: &str, config: &Config) -> Result<Rc<SharedClipboard>, anyhow::Error> {
    if !config.clipboards().contains_key(clipboard_name) {
        anyhow::bail!("Unable to find clipboard {clipboard_name}");
        }

    let clipboard_configuration=&config.clipboards()[clipboard_name];
    let clipshare=Clipshare::new(clipboard_configuration.host());
    let shared_clipboard=Rc::new(SharedClipboard::new(&clipboard_name, clipshare, clipboard_configuration.password()));

    Ok(shared_clipboard)
    }

/// Throws a system notification or prints to the console
fn notify(text: &str, system_notification: bool) {
    if system_notification {
        Notification::new()
        .body(text)
        .show().unwrap();
        }
    else {
        println!("{text}");
        }
    }

/// Throws a system notification or eprints to the console
fn notify_err(error: anyhow::Error, system_notification: bool) {
    if system_notification {
        Notification::new()
        .body(&format!("{error}"))
        .show().unwrap();
        }
    else {
        eprintln!("{error}");
        }
    }

