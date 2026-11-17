//! Shared per-player drafts for the details panel and floating details windows.
use crate::{
    db::queries,
    models::{AppWin, steamid::SteamID},
};
use eframe::egui::{self, Ui};

#[derive(Clone, Default)]
struct NoteEditor {
    text: String,
    saved_text: String,
    error: Option<String>,
}

pub(super) fn show(app: &AppWin, ui: &mut Ui, steamid: SteamID) {
    ui.heading("Notes");
    let key = egui::Id::unique(("player_note_draft", steamid.to_u64()));
    let existing = ui.ctx().data(|data| data.get_temp::<NoteEditor>(key));
    let mut editor = match existing {
        Some(editor) => editor,
        None => {
            let result = (|| -> Result<String, Box<dyn std::error::Error>> {
                let mut conn = app.db.get()?;
                Ok(queries::get_player_note(
                    &mut conn,
                    steamid.to_u64() as i64,
                )?)
            })();
            match result {
                Ok(text) => NoteEditor {
                    saved_text: text.clone(),
                    text,
                    error: None,
                },
                Err(error) => {
                    ui.colored_label(
                        ui.visuals().error_fg_color,
                        format!("Could not load notes: {error}"),
                    );
                    return;
                }
            }
        }
    };
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
        ui.button("Retry saving notes").clicked()
    } else {
        false
    };
    if (changed || retry) && editor.text != editor.saved_text {
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let mut conn = app.db.get()?;
            queries::save_player_note(&mut conn, steamid.to_u64() as i64, &editor.text)?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                editor.saved_text = editor.text.clone();
                editor.error = None;
            }
            Err(error) => editor.error = Some(error.to_string()),
        }
    }
    if editor.text == editor.saved_text {
        editor.error = None;
    }
    ui.ctx().data_mut(|data| data.insert_temp(key, editor));
}
