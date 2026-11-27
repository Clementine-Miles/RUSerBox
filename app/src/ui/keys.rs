//! Private keys: import (pasted or picked), generate Ed25519, show the public
//! half, remember a passphrase, delete.

use egui::{RichText, Ui};
use sbm_ssh::keys::ssh_key::private::{Ed25519Keypair, KeypairData};
use sbm_ssh::keys::ssh_key::{LineEnding, PrivateKey as SshPrivateKey};
use sbm_store::{KvStore, PrivateKey};

use super::widgets::{card, field, wide_button};
use super::{App, Pick, Platform};
use crate::i18n::t;

#[derive(Default)]
pub struct State {
    adding: bool,
    name: String,
    text: String,
    passphrase: String,
    error: Option<String>,
    shown: Option<String>,
}

/// The public key line of a private key text, when it opens.
pub fn public_line(text: &str, passphrase: Option<&str>) -> Result<String, String> {
    let k = sbm_ssh::keys::decode_secret_key(text, passphrase).map_err(|e| e.to_string())?;
    k.public_key().to_openssh().map_err(|e| e.to_string())
}

/// A new Ed25519 key in OpenSSH format.
pub fn generate_ed25519(comment: &str) -> Result<String, String> {
    let mut seed = [0u8; 32];
    getrandom_fill(&mut seed);
    let pair = Ed25519Keypair::from_seed(&seed);
    seed.fill(0);
    let key = SshPrivateKey::new(KeypairData::from(pair), comment).map_err(|e| e.to_string())?;
    Ok(key.to_openssh(LineEnding::LF).map_err(|e| e.to_string())?.to_string())
}

fn getrandom_fill(buf: &mut [u8]) {
    // The store's key generator draws from the same OS source.
    let k = sbm_store::new_key();
    buf.copy_from_slice(&k[..buf.len()]);
}

fn save(app: &mut App, name: &str, text: &str, passphrase: &str) -> Result<(), String> {
    let pass = Some(passphrase).filter(|p| !p.is_empty());
    public_line(text, pass).map_err(|e| format!("{}: {e}", t("invalidPrivateKey", "The key could not be read")))?;
    let k = PrivateKey { id: sbm_store::new_id(), name: name.trim().to_owned(), key: text.trim().to_owned() + "\n", comment: None };
    if k.name.is_empty() {
        return Err(t("nameRequired", "A name is required"));
    }
    let store = app.store();
    store.put_private_key(&k).map_err(|e| e.to_string())?;
    if let Some(p) = pass {
        store.kv(KvStore::KeyPassphrase).set(&k.id, p).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// A key file picked through the platform.
pub fn import(app: &mut App, name: &str, data: &[u8]) {
    let text = String::from_utf8_lossy(data).into_owned();
    app.keys.adding = true;
    app.keys.name = name.trim_end_matches(".pem").trim_end_matches(".key").to_owned();
    app.keys.text = text;
    app.keys.error = None;
}

pub fn show(app: &mut App, ui: &mut Ui, platform: &dyn Platform) {
    let p = app.palette;
    ui.horizontal(|ui| {
        ui.heading(t("privateKey", "Private Keys"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(RichText::new(format!("\u{f067} {}", t("add", "Add"))).color(p.yellow)).clicked() {
                app.keys = State { adding: true, ..Default::default() };
            }
        });
    });
    if app.keys.adding {
        let mut done = false;
        card(ui, &p, "", |ui| {
            field(ui, &t("name", "Name"), &mut app.keys.name, false);
            ui.label(t("privateKey", "Private Key"));
            ui.add(egui::TextEdit::multiline(&mut app.keys.text).code_editor().desired_rows(6).desired_width(f32::INFINITY));
            field(ui, &t("pwd", "Passphrase"), &mut app.keys.passphrase, true);
            ui.horizontal_wrapped(|ui| {
                if ui.button(t("import", "Import file")).clicked() {
                    platform.pick_file(Pick::ImportKey);
                }
                if ui.button(t("paste", "Paste")).clicked()
                    && let Some(c) = platform.clipboard()
                {
                    app.keys.text = c;
                }
                if ui.button(t("generate", "Generate Ed25519")).clicked() {
                    match generate_ed25519(&app.keys.name) {
                        Ok(k) => app.keys.text = k,
                        Err(e) => app.keys.error = Some(e),
                    }
                }
            });
            if let Some(e) = &app.keys.error {
                ui.colored_label(p.red, e);
            }
            ui.horizontal(|ui| {
                if ui.button(t("cancel", "Cancel")).clicked() {
                    done = true;
                }
                if ui.button(t("save", "Save")).clicked() {
                    let (n, k, pw) = (app.keys.name.clone(), app.keys.text.clone(), app.keys.passphrase.clone());
                    match save(app, &n, &k, &pw) {
                        Ok(()) => done = true,
                        Err(e) => app.keys.error = Some(e),
                    }
                }
            });
        });
        if done {
            app.keys = State::default();
        }
        return;
    }
    let store = app.store();
    let keys = store.private_keys().unwrap_or_default();
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        for k in keys {
            card(ui, &p, "", |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&k.name).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new(t("delete", "Delete")).color(p.red)).clicked() {
                            let _ = store.delete_private_key(&k.id);
                            let _ = store.kv(KvStore::KeyPassphrase).remove(&k.id);
                        }
                        if ui.button(t("publicKey", "Public key")).clicked() {
                            app.keys.shown = Some(k.id.clone());
                        }
                    });
                });
                if app.keys.shown.as_deref() == Some(&k.id) {
                    let pass = store.kv(KvStore::KeyPassphrase).get::<String>(&k.id);
                    match public_line(&k.key, pass.as_deref()) {
                        Ok(line) => {
                            ui.add(egui::Label::new(RichText::new(&line).monospace().size(11.0)).wrap());
                            if wide_button(ui, t("copy", "Copy")).clicked() {
                                platform.set_clipboard(&line);
                                app.toast(t("copied", "Copied"));
                            }
                        }
                        Err(e) => {
                            ui.colored_label(p.red, e);
                        }
                    }
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_keys_open_and_have_a_public_line() {
        let k = generate_ed25519("test@ruserbox").unwrap();
        assert!(k.starts_with("-----BEGIN OPENSSH PRIVATE KEY-----"));
        let line = public_line(&k, None).unwrap();
        assert!(line.starts_with("ssh-ed25519 "), "{line}");
        assert_ne!(generate_ed25519("x").unwrap(), generate_ed25519("x").unwrap());
    }
}
