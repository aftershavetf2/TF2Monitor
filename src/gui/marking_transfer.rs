use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
};

use eframe::egui;

use crate::models::{
    AppWin,
    player_markings::{MarkingCommand, SaveResult, TransferOperation},
};

use super::theme;

pub struct Transfer {
    importing: bool,
    path: String,
    replace: bool,
    pending: Option<Receiver<SaveResult>>,
    result: Option<SaveResult>,
}

impl Transfer {
    pub fn new(importing: bool) -> Self {
        Self {
            importing,
            path: if importing {
                String::new()
            } else {
                "player-markings-backup.json".into()
            },
            replace: false,
            pending: None,
            result: None,
        }
    }
}

pub(super) fn show(app: &mut AppWin, ctx: &egui::Context) {
    let Some(mut transfer) = app.marking_transfer.take() else {
        return;
    };
    if let Some(pending) = &transfer.pending {
        match pending.try_recv() {
            Ok(result) => {
                transfer.result = Some(result);
                transfer.pending = None;
            }
            Err(TryRecvError::Disconnected) => {
                transfer.result = Some(Err("Player markings worker is unavailable.".into()));
                transfer.pending = None;
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    let title = if transfer.importing {
        "Import Player Markings"
    } else {
        "Export Player Markings"
    };
    let response = egui::Modal::new(egui::Id::unique("player_marking_transfer")).show(ctx, |ui| {
        theme::section_heading(ui, title);
        let pending = transfer.pending.is_some();
        let complete = matches!(transfer.result, Some(Ok(())));
        ui.add_enabled_ui(!pending && !complete, |ui| {
            ui.label("JSON File Path");
            ui.add(
                egui::TextEdit::singleline(&mut transfer.path)
                    .desired_width(370.0)
                    .hint_text("Path to a JSON file"),
            );
            if transfer.importing {
                ui.label("Merge adds flags and keeps existing notes on conflicts.");
                ui.checkbox(
                    &mut transfer.replace,
                    "Replace All Existing Markings and Notes",
                );
                if transfer.replace {
                    ui.label(
                        "Players missing from the import will lose their saved flags and notes.",
                    );
                }
            } else {
                ui.label("The export includes your personal notes. Choose a new file path.");
            }
        });

        if let Some(result) = &transfer.result {
            match result {
                Ok(()) => {
                    ui.label(if transfer.importing {
                        "Player markings and notes imported."
                    } else {
                        "Player markings and notes exported."
                    });
                }
                Err(error) => {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
            }
        }

        ui.horizontal(|ui| {
            let label = match (transfer.importing, transfer.replace) {
                (true, true) => "Replace All",
                (true, false) => "Import",
                (false, _) => "Export",
            };
            if !complete
                && ui
                    .add_enabled(
                        !pending && !transfer.path.trim().is_empty(),
                        theme::secondary_button(label),
                    )
                    .clicked()
            {
                let (reply, receiver) = mpsc::channel();
                let operation = if transfer.importing {
                    TransferOperation::Import {
                        replace: transfer.replace,
                    }
                } else {
                    TransferOperation::Export
                };
                match app.send_marking_command(MarkingCommand::Transfer {
                    path: PathBuf::from(transfer.path.trim()),
                    operation,
                    reply,
                }) {
                    Ok(()) => {
                        transfer.result = None;
                        transfer.pending = Some(receiver);
                    }
                    Err(error) => transfer.result = Some(Err(error)),
                }
            }
            ui.add_enabled(
                !pending && transfer.pending.is_none(),
                theme::secondary_button(if complete { "Close" } else { "Cancel" }),
            )
            .clicked()
        })
        .inner
    });

    if transfer.pending.is_some() || (!response.inner && !response.should_close()) {
        app.marking_transfer = Some(transfer);
    }
}
