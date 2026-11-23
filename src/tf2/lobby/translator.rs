use crate::models::app_settings::{TranslationMode, TranslationModel};
use std::{collections::HashMap, fs, fs::File};

use anyhow::{Context, Result, ensure};
use candle_core::{DType, Device, Tensor};
use candle_nn::{VarBuilder, ops::log_softmax};
use candle_transformers::models::marian::{Config, MTModel};
use hf_hub::HFClientSync;
use sentencepiece_rs::SentencePieceProcessor;
use unicode_segmentation::UnicodeSegmentation;

const BEAM_WIDTH: usize = 4;

struct Beam {
    model: MTModel,
    tokens: Vec<u32>,
    log_probability: f32,
}

struct DecoderContext {
    encoded: Tensor,
    pad_id: Tensor,
    masked_score: Tensor,
}

impl DecoderContext {
    fn scores(&self, model: &mut MTModel, token: u32, position: usize) -> Result<Tensor> {
        let input = Tensor::new(&[[token]], self.encoded.device())?;
        let scores = model
            .decode(&input, &self.encoded, position)?
            .squeeze(0)?
            .squeeze(0)?;
        Ok(scores.scatter(&self.pad_id, &self.masked_score, 0)?)
    }
}

pub(super) struct Translator {
    model: MTModel,
    config: Config,
    source: SentencePieceProcessor,
    target: SentencePieceProcessor,
    vocab: HashMap<String, u32>,
    pieces: Vec<String>,
}

impl Translator {
    pub(super) fn load(model: TranslationModel) -> Result<Self> {
        // Pin the SafeTensors conversions of both upstream OPUS-MT checkpoints.
        let (owner, name, revision) = match model {
            TranslationModel::Standard => (
                "Helsinki-NLP",
                "opus-mt-mul-en",
                "f5c88837d58e937af4fa5a77015027305e4226f5",
            ),
            TranslationModel::Large => (
                "aoiandroid",
                "opus-mt-mul-en-big",
                "bd44b6741d00072d2fcf28e4c1fe6b42e3622607",
            ),
        };
        let client = HFClientSync::new()?;
        let repo = client.model(owner, name);
        let download = |filename: &str| {
            repo.download_file()
                .filename(filename)
                .revision(revision)
                .send()
                .with_context(|| format!("Could not download {filename} for {owner}/{name}"))
        };

        let config: Config = serde_json::from_reader(File::open(download("config.json")?)?)?;
        let vocab: HashMap<String, u32> =
            serde_json::from_reader(File::open(download("vocab.json")?)?)?;
        let mut pieces = vec![String::new(); config.vocab_size];
        for (piece, &id) in &vocab {
            let entry = pieces
                .get_mut(id as usize)
                .context("Invalid token ID in vocab.json")?;
            *entry = piece.clone();
        }

        let source = SentencePieceProcessor::open(download("source.spm")?)?;
        let target = SentencePieceProcessor::open(download("target.spm")?)?;
        let weights = VarBuilder::from_buffered_safetensors(
            fs::read(download("model.safetensors")?)?,
            DType::F32,
            &Device::Cpu,
        )?;
        let model = MTModel::new(&config, weights)?;
        Ok(Self {
            model,
            config,
            source,
            target,
            vocab,
            pieces,
        })
    }

    pub(super) fn translate(&mut self, text: &str, mode: TranslationMode) -> Result<String> {
        // Translate each sentence separately to reduce omissions on multi-sentence inputs.
        let mut sentences = text
            .split_sentence_bounds()
            .map(str::trim)
            .filter(|sentence| !sentence.is_empty())
            .peekable();
        let Some(first) = sentences.next() else {
            return Ok(String::new());
        };

        if sentences.peek().is_none() {
            return self.translate_ids(&self.encode(text)?, mode);
        }

        let mut translations = Vec::new();
        for sentence in std::iter::once(first).chain(sentences) {
            let ids = self.encode(sentence)?;
            translations.push(self.translate_ids(&ids, mode)?);
        }
        Ok(translations.join(" "))
    }

    fn encode(&self, text: &str) -> Result<Vec<u32>> {
        let unknown_id = *self
            .vocab
            .get("<unk>")
            .context("Missing <unk> in vocab.json")?;
        // SentencePiece segments the text. Marian's shared vocabulary provides the model IDs.
        let mut source_ids: Vec<u32> = self
            .source
            .encode(text)?
            .iter()
            .map(|piece| self.vocab.get(piece).copied().unwrap_or(unknown_id))
            .collect();
        source_ids.push(self.config.eos_token_id);
        ensure!(
            source_ids.len() <= self.config.max_position_embeddings,
            "Text is too long for OPUS-MT (maximum {} tokens including the end token)",
            self.config.max_position_embeddings
        );
        Ok(source_ids)
    }

    fn translate_ids(&mut self, source_ids: &[u32], mode: TranslationMode) -> Result<String> {
        let source = Tensor::new(source_ids, &Device::Cpu)?.unsqueeze(0)?;
        self.model.reset_kv_cache();
        let decoder = DecoderContext {
            encoded: self.model.encoder().forward(&source, 0)?,
            pad_id: Tensor::new(&[self.config.pad_token_id], &Device::Cpu)?,
            masked_score: Tensor::new(&[f32::NEG_INFINITY], &Device::Cpu)?,
        };
        let tokens = match mode {
            TranslationMode::Fast => self.decode_fast(&decoder)?,
            TranslationMode::Beam => self.decode_beam(&decoder)?,
        };
        let translation: Vec<_> = tokens
            .iter()
            .map(|&token| self.pieces[token as usize].as_str())
            .collect();
        Ok(self.target.decode(&translation)?.trim().to_string())
    }

    fn decode_fast(&mut self, decoder: &DecoderContext) -> Result<Vec<u32>> {
        let mut token = self.config.decoder_start_token_id;
        let mut translation = Vec::new();

        for position in 0..self.config.max_position_embeddings {
            token = decoder
                .scores(&mut self.model, token, position)?
                .argmax(0)?
                .to_scalar()?;
            if token == self.config.eos_token_id {
                return Ok(translation);
            }
            translation.push(token);
        }

        anyhow::bail!("Translation reached the OPUS-MT length limit")
    }

    fn decode_beam(&self, decoder: &DecoderContext) -> Result<Vec<u32>> {
        let mut beams = vec![Beam {
            model: self.model.clone(),
            tokens: vec![self.config.decoder_start_token_id],
            log_probability: 0.0,
        }];
        let mut finished = 0;
        let mut best: Option<(f32, Vec<u32>)> = None;

        for position in 0..self.config.max_position_embeddings {
            let mut candidates = Vec::new();
            for (parent, beam) in beams.iter_mut().enumerate() {
                let scores = decoder.scores(&mut beam.model, beam.tokens[position], position)?;
                let mut ranked: Vec<_> = log_softmax(&scores, 0)?
                    .to_vec1::<f32>()?
                    .into_iter()
                    .enumerate()
                    .collect();
                // Each parent can contribute one EOS and at most BEAM_WIDTH continuations.
                let count = BEAM_WIDTH + 1;
                ranked.select_nth_unstable_by(count - 1, |a, b| b.1.total_cmp(&a.1));
                candidates.extend(
                    ranked
                        .into_iter()
                        .take(count)
                        .map(|(token, score)| (parent, token as u32, beam.log_probability + score)),
                );
            }
            candidates.sort_unstable_by(|a, b| b.2.total_cmp(&a.2));

            let mut next = Vec::new();
            for (rank, (parent, token, log_probability)) in candidates.into_iter().enumerate() {
                let beam = &beams[parent];
                if token == self.config.eos_token_id {
                    if rank < BEAM_WIDTH {
                        finished += 1;
                        let score = log_probability / beam.tokens.len() as f32;
                        if best.as_ref().is_none_or(|(previous, _)| score > *previous) {
                            best = Some((score, beam.tokens[1..].to_vec()));
                        }
                    }
                } else {
                    let mut tokens = beam.tokens.clone();
                    tokens.push(token);
                    next.push(Beam {
                        model: beam.model.clone(),
                        tokens,
                        log_probability,
                    });
                    if next.len() == BEAM_WIDTH {
                        break;
                    }
                }
            }
            if finished >= BEAM_WIDTH || next.is_empty() {
                break;
            }
            beams = next;
        }

        best.map(|(_, tokens)| tokens)
            .context("Translation reached the OPUS-MT length limit")
    }
}
