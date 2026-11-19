//! Shared per-player drafts for the details panel and floating details windows.
use crate::models::{
    AppWin,
    player_markings::{MarkingCommand, SaveResult},
    steamid::SteamID,
};
use eframe::egui::{self, Ui};
use std::sync::{
    Arc, Mutex,
    mpsc::{self, Receiver, TryRecvError},
};

#[derive(Clone, Default)]
struct NoteEditor {
    text: String,
    pending: Option<Arc<Mutex<Receiver<SaveResult>>>>,
    error: Option<String>,
}

pub(super) fn show(app: &AppWin, ui: &mut Ui, steamid: SteamID) {
    ui.heading("Notes");
    let key = egui::Id::unique(("player_note_draft", steamid.to_u64()));
    let mut editor = ui
        .ctx()
        .data(|data| data.get_temp::<NoteEditor>(key))
        .unwrap_or_default();
    if let Some(pending) = &editor.pending {
        let result = pending.lock().expect("Note reply lock poisoned").try_recv();
        match result {
            Ok(result) => {
                editor.pending = None;
                editor.error = result.err();
            }
            Err(TryRecvError::Disconnected) => {
                editor.pending = None;
                editor.error = Some("Player markings worker is unavailable.".into());
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    let markings = app.markings.snapshot();
    let saved = markings
        .get(&steamid.to_u64())
        .map_or("", |marking| marking.note.as_str());
    if editor.pending.is_none() && editor.error.is_none() && editor.text != saved {
        editor.text = saved.to_owned();
    }

    let changed = ui
        .add(
            egui::TextEdit::multiline(&mut editor.text)
                .id(ui.scope_id().with(("player_note", steamid.to_u64())))
                .hint_text("Why you flagged this player, or anything else to remember…")
                .desired_rows(5)
                .desired_width(f32::INFINITY),
        )
        .changed();
    let retry = if let Some(error) = &editor.error {
        ui.colored_label(
            ui.visuals().error_fg_color,
            format!("Notes not saved: {error}"),
        );
        ui.add(super::theme::secondary_button("Retry Saving Notes"))
            .clicked()
    } else {
        false
    };
    // Queue every edit in order, including a correction back to the previous saved text.
    if changed || retry {
        let (reply, receiver) = mpsc::channel();
        let result = app.send_marking_command(MarkingCommand::SaveNote {
            steamid,
            note: editor.text.clone(),
            reply,
        });
        match result {
            Ok(()) => {
                editor.pending = Some(Arc::new(Mutex::new(receiver)));
                editor.error = None;
            }
            Err(error) => editor.error = Some(error),
        }
    }
    ui.ctx().data_mut(|data| data.insert_temp(key, editor));
}
