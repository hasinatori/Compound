use crate::state::{AppState, TerminalSession};
use crate::types::*;
use crate::ui_error::{self, Error};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::ipc::Channel;
use tauri::State;

pub const EOF_MARKER: &str = "\u{1}[PROZESS BEENDET]";

fn kill_all(state: &State<'_, AppState>) {
    let mut terms = state.terminals.lock().unwrap();
    for (_, s) in terms.iter() {
        s.quit.store(true, Ordering::Relaxed);
        let _ = s.child.lock().unwrap().kill();
    }
    terms.clear();
}

/// Startet eine eingebettete Terminal-Sitzung (portable-pty) im gewünschten Ordner.
#[tauri::command]
pub fn terminal_open(
    state: State<'_, AppState>,
    cwd: String,
    out: Channel<String>,
) -> Result<TerminalInfo, Error> {
    let cwd_p = Path::new(&cwd);
    if !cwd_p.is_dir() {
        return Err(Error::new("folderMissing", "Ordner existiert nicht."));
    }
    kill_all(&state);

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 90,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| ui_error::io("pty", e))?;

    let mut cmd = CommandBuilder::new(&shell);
    cmd.cwd(&cwd);
    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| ui_error::io("shell", e))?;
    drop(pair.slave);

    let id = uuid::Uuid::new_v4().to_string();
    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| ui_error::io("pty-reader", e))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|e| ui_error::io("pty-writer", e))?;

    let quit = Arc::new(AtomicBool::new(false));
    {
        let mut terms = state
            .terminals
            .lock()
            .map_err(|_| ui_error::lock())?;
        terms.insert(
            id.clone(),
            TerminalSession {
                master: std::sync::Mutex::new(pair.master),
                writer: std::sync::Mutex::new(writer),
                child: std::sync::Mutex::new(child),
                quit: quit.clone(),
            },
        );
    }

    // Reader-Thread: Strömt Terminal-Ausgabe in den IPC-Channel.
    let q = quit.clone();
    std::thread::spawn(move || {
        let mut reader = reader;
        let mut buf = vec![0u8; 16 * 1024];
        loop {
            if q.load(Ordering::Relaxed) {
                break;
            }
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => {
                    if q.load(Ordering::Relaxed) {
                        break;
                    }
                    let _ = out.send(EOF_MARKER.to_string());
                    break;
                }
                Ok(n) => {
                    let s = String::from_utf8_lossy(&buf[..n]).into_owned();
                    if out.send(s).is_err() {
                        break;
                    }
                }
            }
        }
    });

    Ok(TerminalInfo { id, cwd })
}

/// Schickt Eingaben an die Terminal-Sitzung.
#[tauri::command]
pub fn terminal_write(state: State<'_, AppState>, id: String, data: String) -> Result<(), Error> {
    let terms = state
        .terminals
        .lock()
        .map_err(|_| ui_error::lock())?;
    if let Some(s) = terms.get(&id) {
        let mut w = s.writer.lock().map_err(|_| ui_error::lock())?;
        w.write_all(data.as_bytes())
            .map_err(|e| ui_error::io("terminal", e))?;
        w.flush().map_err(|e| ui_error::io("terminal", e))?;
    }
    Ok(())
}

#[tauri::command]
pub fn terminal_resize(
    state: State<'_, AppState>,
    id: String,
    cols: u32,
    rows: u32,
) -> Result<(), Error> {
    let terms = state
        .terminals
        .lock()
        .map_err(|_| ui_error::lock())?;
    if let Some(s) = terms.get(&id) {
        let master = s.master.lock().map_err(|_| ui_error::lock())?;
        let _ = master.resize(PtySize {
            rows: rows as u16,
            cols: cols as u16,
            pixel_width: 0,
            pixel_height: 0,
        });
    }
    Ok(())
}

#[tauri::command]
pub fn terminal_close(state: State<'_, AppState>, id: String) -> Result<(), Error> {
    let mut terms = state
        .terminals
        .lock()
        .map_err(|_| ui_error::lock())?;
    if let Some(s) = terms.get(&id) {
        s.quit.store(true, Ordering::Relaxed);
        let _ = s.child.lock().unwrap().kill();
    }
    terms.remove(&id);
    Ok(())
}