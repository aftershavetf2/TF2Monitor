use super::{Lobby, translator::Translator};
use crate::models::app_settings::{TranslationModel, TranslationOptions};
use std::{
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread,
    time::Instant,
};

#[derive(Default)]
pub(super) struct ChatTranslation {
    worker: Option<TranslationWorker>,
    unavailable: Option<TranslationModel>,
    next_row: usize,
}

impl ChatTranslation {
    pub(super) fn retry_worker(&mut self) {
        self.unavailable = None;
    }

    pub(super) fn reset_lobby(&mut self) {
        self.next_row = 0;
    }

    pub(super) fn update(&mut self, lobby: &mut Lobby, options: Option<TranslationOptions>) {
        self.receive_translation(lobby);
        let Some(options) = options else {
            return;
        };
        if self.unavailable == Some(options.model) {
            return;
        }

        let Some(chat) = lobby.chat.get(self.next_row) else {
            return;
        };
        let worker = match &mut self.worker {
            Some(worker) => worker,
            slot @ None => match TranslationWorker::start() {
                Ok(worker) => slot.insert(worker),
                Err(error) => {
                    log::warn!("Could not start the chat translation worker: {error}");
                    self.unavailable = Some(options.model);
                    return;
                }
            },
        };
        if worker.pending.is_some() {
            return;
        }
        let request = TranslationRequest {
            message: chat.message.clone(),
            options,
        };
        if worker.messages.send(request).is_ok() {
            worker.pending = Some(PendingTranslation {
                session_id: lobby.session_id,
                row: self.next_row,
                model: options.model,
            });
        } else {
            self.worker = None;
            self.unavailable = Some(options.model);
        }
    }

    fn receive_translation(&mut self, lobby: &mut Lobby) {
        let Some(worker) = &mut self.worker else {
            return;
        };
        let Some(pending) = worker.pending else {
            return;
        };
        match worker.translations.try_recv() {
            Ok(TranslationResult::Completed(translation)) => {
                worker.pending = None;
                if pending.session_id == lobby.session_id
                    && let Some(chat) = lobby.chat.get_mut(pending.row)
                {
                    chat.translated_message = translation;
                    self.next_row = pending.row + 1;
                }
            }
            Ok(TranslationResult::ModelUnavailable) => {
                worker.pending = None;
                self.unavailable = Some(pending.model);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.worker = None;
                self.unavailable = Some(pending.model);
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct TranslationRequest {
    message: String,
    options: TranslationOptions,
}

enum TranslationResult {
    Completed(Option<String>),
    ModelUnavailable,
}

#[derive(Clone, Copy)]
struct PendingTranslation {
    session_id: u64,
    row: usize,
    model: TranslationModel,
}

struct TranslationWorker {
    messages: Sender<TranslationRequest>,
    translations: Receiver<TranslationResult>,
    // Keep at most one message in flight, and discard results from earlier lobbies.
    pending: Option<PendingTranslation>,
}

impl TranslationWorker {
    fn start() -> std::io::Result<Self> {
        let (messages, incoming) = mpsc::channel();
        let (outgoing, translations) = mpsc::channel();
        thread::Builder::new()
            .name("chat-translation".into())
            .spawn(move || translate_messages(incoming, outgoing))?;
        Ok(Self {
            messages,
            translations,
            pending: None,
        })
    }
}

fn translate_messages(incoming: Receiver<TranslationRequest>, outgoing: Sender<TranslationResult>) {
    let mut loaded = None;

    for request in incoming {
        if outgoing
            .send(translate_request(&mut loaded, request))
            .is_err()
        {
            return;
        }
    }
}

fn translate_request(
    loaded: &mut Option<(TranslationModel, Translator)>,
    request: TranslationRequest,
) -> TranslationResult {
    let model = request.options.model;
    let mut translator = match loaded.take() {
        Some((current, translator)) if current == model => translator,
        previous => {
            // Release the previous weights before loading another model.
            drop(previous);
            match Translator::load(model) {
                Ok(translator) => translator,
                Err(error) => {
                    log::warn!("Could not load the {model:?} chat translation model: {error:#}");
                    return TranslationResult::ModelUnavailable;
                }
            }
        }
    };
    let mode = request.options.mode;
    let started = Instant::now();
    let result = translator.translate(&request.message, mode);
    let elapsed_ms = started.elapsed().as_millis();
    let translation = match result {
        Ok(translation) => {
            log::info!("Translated a chat message in {elapsed_ms} ms ({model:?}, {mode:?})");
            Some(translation)
        }
        Err(error) => {
            log::warn!(
                "Could not translate a chat message after {elapsed_ms} ms ({model:?}, {mode:?}): {error:#}"
            );
            None
        }
    };
    *loaded = Some((model, translator));
    TranslationResult::Completed(translation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::app_settings::TranslationMode;
    use crate::tf2::lobby::LobbyChat;

    #[test]
    fn translation_requests_follow_the_setting_and_discard_previous_lobby_results() {
        let (messages, incoming) = mpsc::channel();
        let (outgoing, translations) = mpsc::channel();
        let mut translation = ChatTranslation {
            worker: Some(TranslationWorker {
                messages,
                translations,
                pending: None,
            }),
            ..Default::default()
        };
        let mut lobby = Lobby {
            chat: vec![LobbyChat {
                message: "Hej".into(),
                ..Default::default()
            }],
            ..Default::default()
        };

        translation.update(&mut lobby, None);
        assert_eq!(incoming.try_recv(), Err(TryRecvError::Empty));
        let standard = TranslationOptions {
            model: TranslationModel::Standard,
            mode: TranslationMode::Fast,
        };
        let large = TranslationOptions {
            model: TranslationModel::Large,
            mode: TranslationMode::Beam,
        };
        translation.update(&mut lobby, Some(standard));
        assert_eq!(
            incoming.try_recv().unwrap(),
            TranslationRequest {
                message: "Hej".into(),
                options: standard,
            }
        );

        lobby.session_id += 1;
        lobby.chat[0].message = "Tack".into();
        translation.reset_lobby();
        outgoing
            .send(TranslationResult::Completed(Some("Hello".into())))
            .unwrap();
        translation.update(&mut lobby, None);
        assert_eq!(incoming.try_recv(), Err(TryRecvError::Empty));
        assert_eq!(lobby.chat[0].translated_message, None);

        translation.update(&mut lobby, Some(large));
        assert_eq!(
            incoming.try_recv().unwrap(),
            TranslationRequest {
                message: "Tack".into(),
                options: large,
            }
        );

        outgoing.send(TranslationResult::ModelUnavailable).unwrap();
        translation.update(&mut lobby, Some(large));
        assert_eq!(incoming.try_recv(), Err(TryRecvError::Empty));
        translation.update(&mut lobby, Some(standard));
        assert_eq!(incoming.try_recv().unwrap().options, standard);
        outgoing
            .send(TranslationResult::Completed(Some("Thanks".into())))
            .unwrap();
        translation.update(&mut lobby, Some(standard));
        assert_eq!(lobby.chat[0].translated_message.as_deref(), Some("Thanks"));
        assert_eq!(incoming.try_recv(), Err(TryRecvError::Empty));
    }
}
