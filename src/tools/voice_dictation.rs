//! Voice dictation: streaming speech to text with [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx).
//!
//! [`VoiceDictation`] captures the microphone with StereoKit's [`Microphone`] at 16 kHz, and feeds it chunk by chunk to
//! a streaming transducer model (zipformer, …) running on a worker thread. While the user speaks it publishes
//! provisional text ([`VoiceDictation::partial`]); at every endpoint (a pause) it queues the *final* text
//! ([`VoiceDictation::take_final`]), ready to be inserted into a [`Ui::input`] field with [`insert_at_caret`].
//!
//! The microphone is started before the model is loaded: [`VoiceDictation::is_starting`] tells the UI the session is
//! warming up, [`VoiceDictation::level`] gives a live input level (for a meter or an animated indicator), and the
//! samples read meanwhile are queued for the recognizer instead of being lost.
//!
//! This module is enabled by the `voice-dictation` cargo feature, which brings the
//! [`sherpa-onnx`](https://crates.io/crates/sherpa-onnx) dependency in:
//!
//! ```toml
//! stereokit-rust = { version = "0.4", features = ["voice-dictation"] }
//! ```
//!
//! # Model
//!
//! A streaming (online) transducer model is needed: an `encoder`, a `decoder`, a `joiner` and a `tokens.txt`.
//! [`DictationModel::from_dir`] finds them in a model directory, and prefers the `int8` variants when both exist.
//!
//! English, ~20M parameters (a good start on a Quest):
//!
//! ```text
//! wget https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-streaming-zipformer-en-20M-2023-02-17.tar.bz2
//! tar xvf sherpa-onnx-streaming-zipformer-en-20M-2023-02-17.tar.bz2
//! ```
//!
//! French: `sherpa-onnx-streaming-zipformer-fr-2023-04-14` (small), or `sherpa-onnx-streaming-zipformer-fr-kroko` from
//! <https://huggingface.co/kouhxp/sherpa-onnx-streaming-zipformer-fr-kroko> (155 MB encoder, but far better). See the
//! [sherpa-onnx pre-trained models](https://k2-fsa.github.io/sherpa/onnx/pretrained_models/index.html) for the others.
//!
//! # Android
//!
//! On Android the sherpa-onnx crate links its C API as a shared library: its build script downloads the
//! `sherpa-onnx-v<version>-android.tar.bz2` archive, and this crate's build copies `libsherpa-onnx-c-api.so` and
//! `libonnxruntime.so` to `target/runtime_libs/<abi>/`, which the gradle template packages in the APK (next to
//! `libopenxr_loader.so`). The model files are not in the APK: push them where the app can read them, and give their
//! directory to [`DictationModel::from_dir`]. `SHERPA_ONNX_LIB_DIR` can point the sherpa-onnx build script to a
//! directory holding its `.so` files instead of downloading them.
//!
//! # Window
//!
//! The module also ships [`VoicePanel`], a ready-to-use stepper window: model folder picker, record/stop, live level,
//! provisional text and a scrollable log of the committed sentences. It pops up in front of the user, at 60% scale,
//! closes itself with its close button, and can be resized / zoomed with its scale handle. The `voice1` demo only
//! registers it.
//!
//! # Example
//!
//! ```ignore
//! use stereokit_rust::tools::voice_dictation::{DictationModel, VoiceDictation, insert_at_caret};
//!
//! let model = DictationModel::from_dir("sherpa-onnx-streaming-zipformer-en-20M-2023-02-17").unwrap();
//! let mut dictation = VoiceDictation::new(model);
//! dictation.start(); // ask for `PermissionType::Microphone` first on Android
//!
//! // then, once per frame, at the same `Ui` level as the input field:
//! dictation.update();
//! Ui::input("field", &mut text).edit();
//! while let Some(final_text) = dictation.take_final() {
//!     insert_at_caret("field", &mut text, &final_text);
//! }
//! ```
//!
//! See [`VoicePanel`] for the ready-to-use window, and the `voice1` demo which registers it.

use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

use sherpa_onnx::{OnlineRecognizer, OnlineRecognizerConfig};

use crate::{
    prelude::*,
    sound::{Microphone, Sound, SoundSampleRate},
    ui::Ui,
};

/// Sample rate of the microphone capture, in Hz: sherpa-onnx streaming models expect 16 kHz.
pub const DICTATION_SAMPLE_RATE: i32 = 16_000;

/// Samples read from the microphone per [`VoiceDictation::update`] (0.25 s at 16 kHz).
const SAMPLE_BUFFER: usize = DICTATION_SAMPLE_RATE as usize / 4;

/// Depth of the directory walk of [`DictationModel::from_dir`].
const MODEL_DIR_DEPTH: usize = 3;

/// What [`VoiceDictation`] publishes while it runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DictationEvent {
    /// The microphone is recording. The recognizer may still be loading its model: the samples wait in a queue until
    /// it is ready.
    Started,
    /// Provisional text of the current utterance: it may still change (or be dropped).
    Partial(String),
    /// Committed text of an utterance: an endpoint was detected, or the session stopped.
    Final(String),
    /// The session stopped, normally (`None`) or because of an error.
    Stopped(Option<String>),
}

/// Paths of the files of a streaming transducer model, and the recognizer options that go with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DictationModel {
    /// The `encoder*.onnx` file.
    pub encoder: PathBuf,
    /// The `decoder*.onnx` file.
    pub decoder: PathBuf,
    /// The `joiner*.onnx` file.
    pub joiner: PathBuf,
    /// The `tokens.txt` file.
    pub tokens: PathBuf,
    /// Optional sherpa-onnx `model_type` (e.g. `"zipformer2"`). `None` lets sherpa-onnx read it from the model.
    pub model_type: Option<String>,
    /// Threads used by the recognizer. Default is 1: dictation is light, and an XR device has few cores to spare.
    pub num_threads: i32,
}

impl DictationModel {
    /// A model from the paths of its four files. `model_type` is `None` and `num_threads` is 1.
    pub fn new(
        encoder: impl Into<PathBuf>,
        decoder: impl Into<PathBuf>,
        joiner: impl Into<PathBuf>,
        tokens: impl Into<PathBuf>,
    ) -> Self {
        Self {
            encoder: encoder.into(),
            decoder: decoder.into(),
            joiner: joiner.into(),
            tokens: tokens.into(),
            model_type: None,
            num_threads: 1,
        }
    }

    /// Find a streaming transducer model in `dir` (and its sub-directories, up to 3 levels down).
    ///
    /// The `encoder`, `decoder` and `joiner` files are the `.onnx` files whose name contains it, the `int8` variants
    /// being preferred when both exist. `tokens` is the `tokens.txt` file. Returns `None` (with a log line naming the
    /// missing file) when the directory does not hold a complete model.
    pub fn from_dir(dir: impl AsRef<Path>) -> Option<Self> {
        let dir = dir.as_ref();
        if !dir.is_dir() {
            Log::warn(format!("VoiceDictation: model directory '{}' does not exist", dir.display()));
            return None;
        }
        let mut files = Vec::new();
        collect_files(dir, MODEL_DIR_DEPTH, &mut files);

        let Some(encoder) = pick_file(&files, "encoder", ".onnx", true) else {
            Log::err(format!("VoiceDictation: no encoder .onnx file in '{}'", dir.display()));
            return None;
        };
        let Some(decoder) = pick_file(&files, "decoder", ".onnx", true) else {
            Log::err(format!("VoiceDictation: no decoder .onnx file in '{}'", dir.display()));
            return None;
        };
        let Some(joiner) = pick_file(&files, "joiner", ".onnx", true) else {
            Log::err(format!("VoiceDictation: no joiner .onnx file in '{}'", dir.display()));
            return None;
        };
        let Some(tokens) = pick_file(&files, "tokens", ".txt", false) else {
            Log::err(format!("VoiceDictation: no tokens.txt file in '{}'", dir.display()));
            return None;
        };
        Some(Self { encoder, decoder, joiner, tokens, model_type: None, num_threads: 1 })
    }

    /// Check that the four files exist. The error names the first missing file.
    pub fn validate(&self) -> Result<(), String> {
        for (kind, path) in [
            ("encoder", &self.encoder),
            ("decoder", &self.decoder),
            ("joiner", &self.joiner),
            ("tokens", &self.tokens),
        ] {
            if !path.is_file() {
                return Err(format!("{kind} file '{}' does not exist", path.display()));
            }
        }
        Ok(())
    }

    /// A single line naming the model files, for logs.
    pub fn describe(&self) -> String {
        format!(
            "encoder: '{}', decoder: '{}', joiner: '{}', tokens: '{}'",
            self.encoder.display(),
            self.decoder.display(),
            self.joiner.display(),
            self.tokens.display()
        )
    }
}

/// Walk `dir` and collect the files, `depth` directories levels down at most.
fn collect_files(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, depth - 1, out);
        } else if path.is_file() {
            out.push(path);
        }
    }
}

/// Pick a file whose name contains `needle` and ends with `extension`, preferring the `int8` variant when
/// `prefer_int8` is true (then the shortest name breaks the tie).
fn pick_file(files: &[PathBuf], needle: &str, extension: &str, prefer_int8: bool) -> Option<PathBuf> {
    let mut candidates: Vec<&PathBuf> = files
        .iter()
        .filter(|path| {
            path.file_name().and_then(|name| name.to_str()).is_some_and(|name| {
                let name = name.to_ascii_lowercase();
                name.contains(needle) && name.ends_with(extension)
            })
        })
        .collect();
    candidates.sort_by_key(|path| {
        let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_ascii_lowercase();
        let int8 = if prefer_int8 && name.contains("int8") { 0 } else { 1 };
        (int8, name.len())
    });
    candidates.first().map(|path| (*path).clone())
}

/// Streaming dictation: microphone capture, recognition on a worker thread, provisional and final text.
///
/// Create it with a [`DictationModel`], call [`VoiceDictation::start`], then [`VoiceDictation::update`] once per frame
/// and read [`VoiceDictation::partial`] / [`VoiceDictation::take_final`]. [`VoiceDictation::level`] follows the
/// microphone input and [`VoiceDictation::is_starting`] tells when the session is still warming up. Dropping it stops
/// the microphone.
pub struct VoiceDictation {
    /// The model and its options.
    model: DictationModel,
    /// The recognition thread, while a session is running.
    worker: Option<Worker>,
    /// The session is starting: the worker is spawned, but the microphone and the recognizer are not ready yet.
    starting: bool,
    /// The microphone is recording (the worker said [`DictationEvent::Started`]).
    recording: bool,
    /// Smoothed level of the microphone input, from 0 (silence) to 1 (loud).
    level: f32,
    /// Provisional text of the current utterance.
    partial: String,
    /// Committed texts not taken yet.
    finals: VecDeque<String>,
    /// Last error of the worker, if any.
    error: Option<String>,
    /// The microphone stream, once the worker is recording.
    mic: Option<Sound>,
    /// Reused buffer for [`Sound::read_samples`].
    buffer: Vec<f32>,
}

/// The recognition thread and the two channels talking to it.
struct Worker {
    /// Samples waiting for the recognizer. Dropping it ends the thread, which flushes and stops the microphone.
    samples: Option<Sender<Vec<f32>>>,
    /// What the thread publishes.
    events: Receiver<DictationEvent>,
    /// The thread itself, joined by [`VoiceDictation::stop`].
    thread: Option<JoinHandle<()>>,
}

impl VoiceDictation {
    /// A dictation tool for `model`. Nothing runs until [`VoiceDictation::start`] is called.
    pub fn new(model: DictationModel) -> Self {
        Self {
            model,
            worker: None,
            starting: false,
            recording: false,
            level: 0.0,
            partial: String::new(),
            finals: VecDeque::new(),
            error: None,
            mic: None,
            buffer: vec![0.0; SAMPLE_BUFFER],
        }
    }

    /// The model in use.
    pub fn model(&self) -> &DictationModel {
        &self.model
    }

    /// Change the model. A running session is stopped first.
    pub fn set_model(&mut self, model: DictationModel) {
        self.stop();
        self.model = model;
    }

    /// Is the recognition thread running (the microphone may still be starting)?
    pub fn is_running(&self) -> bool {
        self.worker.is_some()
    }

    /// Is the microphone recording? The recognizer may still be loading its model, the samples are queued until then.
    pub fn is_recording(&self) -> bool {
        self.recording
    }

    /// Is the session starting? The worker is spawned, but the microphone is warming up (device initialization can
    /// take a moment) and the recognizer is not ready yet.
    pub fn is_starting(&self) -> bool {
        self.starting
    }

    /// Smoothed level of the microphone input, from 0 (silence) to 1 (loud), for a live meter.
    ///
    /// It is computed from the samples read by [`VoiceDictation::update`], with a fast attack and a slow release, and
    /// falls back to 0 when nothing is recorded. The scale is rough: `rms * 6`, clamped, so speech usually moves it
    /// between 0.2 and 1.
    pub fn level(&self) -> f32 {
        self.level
    }

    /// Provisional text of the current utterance: empty when nothing is being recognized.
    pub fn partial(&self) -> &str {
        &self.partial
    }

    /// Take the oldest committed text, if any.
    pub fn take_final(&mut self) -> Option<String> {
        self.finals.pop_front()
    }

    /// The last error reported by the worker, if any. Cleared by the next successful [`VoiceDictation::start`].
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

impl VoiceDictation {
    /// Start the recognition thread, which starts the microphone at 16 kHz ([`SoundSampleRate::Speech`]).
    ///
    /// Returns `false` (and logs the reason) when the model files are missing or the thread can not be spawned. On
    /// Android, ask for `PermissionType::Microphone` first. Calling it on a running session does nothing.
    pub fn start(&mut self) -> bool {
        if self.worker.is_some() {
            return true;
        }
        if let Err(error) = self.model.validate() {
            Log::err(format!("VoiceDictation: {error}"));
            self.error = Some(error);
            return false;
        }
        let (samples_tx, samples_rx) = mpsc::channel::<Vec<f32>>();
        let (events_tx, events_rx) = mpsc::channel::<DictationEvent>();
        let model = self.model.clone();
        let spawned = thread::Builder::new()
            .name("voice_dictation".to_owned())
            .spawn(move || run_worker(model, samples_rx, events_tx));
        match spawned {
            Ok(thread) => {
                Log::info(format!("VoiceDictation: listening with {}", self.model.describe()));
                self.worker = Some(Worker { samples: Some(samples_tx), events: events_rx, thread: Some(thread) });
                self.starting = true;
                self.partial.clear();
                self.level = 0.0;
                self.error = None;
                true
            }
            Err(error) => {
                let error = format!("could not start the recognition thread: {error}");
                Log::err(format!("VoiceDictation: {error}"));
                self.error = Some(error);
                false
            }
        }
    }

    /// Stop the recognition thread and the microphone. The worker flushes the last utterance first, so the texts it
    /// produced are still available through [`VoiceDictation::take_final`].
    pub fn stop(&mut self) {
        let Some(mut worker) = self.worker.take() else {
            return;
        };
        // Dropping the samples channel ends the worker loop: it flushes, stops the microphone, and returns.
        worker.samples = None;
        if let Some(thread) = worker.thread.take() {
            let _ = thread.join();
        }
        while let Ok(event) = worker.events.try_recv() {
            self.apply_event(event);
        }
        self.starting = false;
        self.recording = false;
        self.level = 0.0;
        self.partial.clear();
        self.mic = None;
    }

    /// Move the session forward: collect what the recognizer published, then feed it the new microphone samples.
    ///
    /// Call it once per frame, before drawing the [`Ui::input`] the text goes into.
    pub fn update(&mut self) {
        if self.worker.is_none() {
            return;
        }

        // 1. Collect what the worker published.
        let mut events = Vec::new();
        if let Some(worker) = &self.worker {
            while let Ok(event) = worker.events.try_recv() {
                events.push(event);
            }
        }
        let mut finished = false;
        for event in events {
            if matches!(event, DictationEvent::Stopped(_)) {
                finished = true;
            }
            self.apply_event(event);
        }
        if finished {
            self.finish_worker();
        }

        // 2. Feed the new microphone samples to the worker, and follow their level.
        if !self.recording {
            // The microphone is warming up, or the session stopped: the meter falls back to silence.
            self.level *= 0.85;
            return;
        }
        let Some(samples) = self.worker.as_ref().and_then(|worker| worker.samples.clone()) else {
            return;
        };
        if self.mic.is_none() {
            self.mic = Microphone::sound().ok();
        }
        let Some(mic) = &self.mic else {
            return;
        };
        loop {
            let read = mic.read_samples(&mut self.buffer, None) as usize;
            if read == 0 {
                break;
            }
            // Root mean square of the chunk, scaled to 0..=1: speech usually lands between 0.2 and 1. Fast attack, slow
            // release, so the meter reacts at once and fades instead of flickering.
            let sum: f32 = self.buffer[..read].iter().map(|sample| sample * sample).sum();
            let target = ((sum / read as f32).sqrt() * 6.0).clamp(0.0, 1.0);
            self.level = if target > self.level { target } else { self.level * 0.8 + target * 0.2 };
            if samples.send(self.buffer[..read].to_vec()).is_err() {
                break;
            }
            if read < SAMPLE_BUFFER {
                break;
            }
        }
    }

    /// Apply one event to the tool state.
    fn apply_event(&mut self, event: DictationEvent) {
        match event {
            DictationEvent::Started => {
                self.starting = false;
                self.recording = true;
                self.level = 0.0;
                self.partial.clear();
            }
            DictationEvent::Partial(text) => self.partial = text,
            DictationEvent::Final(text) => {
                let text = text.trim();
                if !text.is_empty() {
                    self.finals.push_back(text.to_owned());
                }
                self.partial.clear();
            }
            DictationEvent::Stopped(error) => {
                self.starting = false;
                self.recording = false;
                self.level = 0.0;
                self.partial.clear();
                if let Some(error) = error {
                    Log::err(format!("VoiceDictation: {error}"));
                    self.error = Some(error);
                }
            }
        }
    }

    /// Release the worker after it stopped by itself (an error, or an internal stop).
    fn finish_worker(&mut self) {
        if let Some(mut worker) = self.worker.take() {
            worker.samples = None;
            if let Some(thread) = worker.thread.take() {
                let _ = thread.join();
            }
        }
        self.starting = false;
        self.recording = false;
        self.level = 0.0;
        self.mic = None;
    }
}

impl Drop for VoiceDictation {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Insert `text` into the [`Ui::input`] field `id` of `field`, at the caret of the focused field, and move the caret
/// after the inserted text.
///
/// Must be called at the same `Ui` stack level as the input it refers to (typically right after its `edit()`).
/// Returns `true` when the text was inserted at the caret. When `id` is not the focused field, the text is appended at
/// the end of `field` instead, and `false` is returned.
pub fn insert_at_caret(id: impl AsRef<str>, field: &mut String, text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    let caret = if Ui::input_is_focused(id) { Ui::input_caret() } else { -1 };
    if caret < 0 {
        field.push_str(text);
        return false;
    }
    let byte = field.char_indices().nth(caret as usize).map(|(byte, _)| byte).unwrap_or(field.len());
    field.insert_str(byte, text);
    let caret = caret + text.chars().count() as i32;
    Ui::input_caret_set(caret, caret);
    true
}

/// Worker thread: own the recognizer, consume the samples, publish the texts.
fn run_worker(model: DictationModel, samples: Receiver<Vec<f32>>, events: Sender<DictationEvent>) {
    let mut config = OnlineRecognizerConfig::default();
    config.feat_config.sample_rate = DICTATION_SAMPLE_RATE;
    config.model_config.transducer.encoder = Some(model.encoder.to_string_lossy().into_owned());
    config.model_config.transducer.decoder = Some(model.decoder.to_string_lossy().into_owned());
    config.model_config.transducer.joiner = Some(model.joiner.to_string_lossy().into_owned());
    config.model_config.tokens = Some(model.tokens.to_string_lossy().into_owned());
    config.model_config.model_type = model.model_type.clone();
    config.model_config.num_threads = model.num_threads;
    config.model_config.provider = Some("cpu".to_owned());
    config.enable_endpoint = true;
    config.decoding_method = Some("greedy_search".to_owned());
    // Endpoint rules: commit an utterance after ~0.8 s of trailing silence, 1.2 s for a longer pause, and never let an
    // utterance grow past 20 s.
    config.rule1_min_trailing_silence = 0.8;
    config.rule2_min_trailing_silence = 1.2;
    config.rule3_min_utterance_length = 20.0;

    // The microphone goes first: its initialization can take a moment, and starting it before the model load lets the
    // UI show the input level right away. The samples read during the model load wait in the channel.
    if !Microphone::start(None, Some(SoundSampleRate::Speech)) {
        let error = "could not start the microphone".to_owned();
        Log::err(format!("VoiceDictation: {error}"));
        let _ = events.send(DictationEvent::Stopped(Some(error)));
        return;
    }
    let _ = events.send(DictationEvent::Started);

    let Some(recognizer) = OnlineRecognizer::create(&config) else {
        Microphone::stop();
        let error = format!("could not create the sherpa-onnx recognizer with {}", model.describe());
        Log::err(format!("VoiceDictation: {error}"));
        let _ = events.send(DictationEvent::Stopped(Some(error)));
        return;
    };

    let stream = recognizer.create_stream();
    let mut last = String::new();
    while let Ok(chunk) = samples.recv() {
        stream.accept_waveform(DICTATION_SAMPLE_RATE, &chunk);
        while recognizer.is_ready(&stream) {
            recognizer.decode(&stream);
        }
        if let Some(result) = recognizer.get_result(&stream)
            && result.text != last
        {
            last = result.text;
            let _ = events.send(DictationEvent::Partial(last.clone()));
        }
        if recognizer.is_endpoint(&stream) {
            if !last.trim().is_empty() {
                let _ = events.send(DictationEvent::Final(last.clone()));
            }
            last.clear();
            let _ = events.send(DictationEvent::Partial(String::new()));
            recognizer.reset(&stream);
        }
    }

    // The samples channel closed: flush the last utterance before releasing the microphone.
    stream.input_finished();
    while recognizer.is_ready(&stream) {
        recognizer.decode(&stream);
    }
    if let Some(result) = recognizer.get_result(&stream)
        && !result.text.trim().is_empty()
    {
        let _ = events.send(DictationEvent::Final(result.text));
    }
    Microphone::stop();
    let _ = events.send(DictationEvent::Stopped(None));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a temporary model directory holding `files` in an `exp/` sub-directory.
    fn temp_model_dir(files: &[&str]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sk_voice_dictation_{}_{}", std::process::id(), files.len()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("exp")).expect("temporary model directory");
        for file in files {
            fs::write(dir.join("exp").join(file), b"test").expect("temporary model file");
        }
        dir
    }

    #[test]
    fn model_from_dir_prefers_int8_files() {
        let dir = temp_model_dir(&[
            "encoder-epoch-1.onnx",
            "encoder-epoch-1.int8.onnx",
            "decoder-epoch-1.onnx",
            "joiner-epoch-1.onnx",
            "tokens.txt",
        ]);
        let model = DictationModel::from_dir(&dir).expect("a complete model");
        assert!(model.encoder.to_string_lossy().contains("int8"));
        assert!(model.validate().is_ok());
        assert_eq!(model.num_threads, 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_names_the_missing_file() {
        let model = DictationModel::new("encoder.onnx", "decoder.onnx", "joiner.onnx", "tokens.txt");
        let error = model.validate().expect_err("the files do not exist");
        assert!(error.starts_with("encoder file"));
    }
}

// ---------------------------------------------------------------------------
// VoicePanel: a ready-to-use dictation window
// ---------------------------------------------------------------------------

#[cfg(not(feature = "no-event-loop"))]
use crate::{
    font::Font,
    framework::Appearence,
    maths::{Pose, Vec2, Vec3},
    sprite::Sprite,
    system::{Align, TextFit},
    ui::{UiBtnLayout, UiPad, UiScroll},
    util::Time,
};
#[cfg(all(not(feature = "no-event-loop"), target_os = "android"))]
use crate::{
    permission::{Permission, PermissionState, PermissionType},
    tools::os_api::{SystemAction, system_deep_link},
};
#[cfg(not(feature = "no-event-loop"))]
use crate::{
    tools::{
        file_browser_b::{FILE_BROWSER_B_SELECT_DIR, FILE_BROWSER_ID_PREFIX, FileBrowserB},
        os_api::BrowseLocation,
    },
    util::PickerMode,
};

/// Pages opened by the download buttons, on Android only: the headset has no other browser.
#[cfg(all(not(feature = "no-event-loop"), target_os = "android"))]
const MODEL_PAGES: [(&str, &str); 3] = [
    (
        "French model (HF, 4 files)",
        "https://huggingface.co/kouhxp/sherpa-onnx-streaming-zipformer-fr-kroko",
    ),
    ("sherpa-onnx models", "https://k2-fsa.github.io/sherpa/onnx/pretrained_models/index.html"),
    ("Models archive", "https://github.com/k2-fsa/sherpa-onnx/releases/tag/asr-models"),
];

/// A ready-to-use dictation window: model folder picker, record/stop, live level, provisional text and a
/// scrollable log of the committed sentences.
///
/// It appears in front of the user ([`Ui::popup_pose`]), can be moved, resized and zoomed with its scale handle, and
/// closes itself with its close button ([`Sprite::close`]).
///
/// ### Fields that can be changed before initialization:
/// * `appearence` - The look of the window (text styles, size, scale handle...), at 60% scale by default. See
///   [`Appearence`] and [`Appearence::set_ui_scale`].
/// * `window_pose` - Where the window stands. Default is [`Pose::IDENTITY`], replaced by [`Ui::popup_pose`] at start.
///
/// ### Events this stepper is listening to:
/// None, the window closes itself with its close button.
#[cfg(not(feature = "no-event-loop"))]
#[derive(IStepper)]
pub struct VoicePanel {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    shutdown_completed: bool,

    /// The look of the window: size, scaling, text styles and scale handle, see [`Appearence`].
    pub appearence: Appearence,
    /// Where the window stands, in world space.
    pub window_pose: Pose,
    /// Directory of the loaded model, `None` until one is loaded.
    pub model_dir: Option<PathBuf>,

    /// The dictation session, `None` until a model is loaded.
    dictation: Option<VoiceDictation>,
    /// The committed sentences of the dictation, one per CRLF.
    dictation_log: String,
    /// Scroll state of `dictation_log`, driven by the scrollbar of its text area.
    log_scroll: Vec2,
    /// The model status: the chosen folder, or what is missing.
    status: String,
    /// Elapsed time in seconds, for the animated ellipsis of the microphone warm-up.
    anim_time: f32,
    /// The close button icon.
    close_sprite: Sprite,
    /// Counter in the file browser id, so several browsers can be opened one after the other.
    browser_counter: u32,
}

#[cfg(not(feature = "no-event-loop"))]
unsafe impl Send for VoicePanel {}

#[cfg(not(feature = "no-event-loop"))]
impl Default for VoicePanel {
    fn default() -> Self {
        // 60% of the base look by default, and a window the scale handle can resize and zoom.
        let mut appearence = Appearence::new(&Font::default(), 0.024);
        appearence.window_size = Vec2::new(0.62, 0.50);
        appearence.min_window_size = Vec2::new(0.40, 0.34);
        appearence.set_ui_scale(0.6);
        Self {
            id: "VoicePanel".to_string(),
            sk_info: None,
            shutdown_completed: false,
            appearence,
            window_pose: Pose::IDENTITY,
            model_dir: None,
            dictation: None,
            dictation_log: String::new(),
            log_scroll: Vec2::ZERO,
            status: "No model: choose the model folder (encoder/decoder/joiner .onnx + tokens.txt).".to_string(),
            anim_time: 0.0,
            close_sprite: Sprite::close(),
            browser_counter: 0,
        }
    }
}

#[cfg(not(feature = "no-event-loop"))]
impl VoicePanel {
    /// Called by the derive macro during `IStepper::initialize`.
    fn start(&mut self) -> bool {
        // The window pops up in front of the user.
        self.window_pose = Ui::popup_pose(Vec3::new(0.0, 0.0, 0.10));
        self.appearence.start();

        #[cfg(target_os = "android")]
        if Permission::get_state(PermissionType::Microphone) == PermissionState::Capable {
            Permission::request(&[PermissionType::Microphone]);
        }

        // An environment variable lets an automated run skip the file browser.
        if let Some(dir) = std::env::var_os("SHERPA_ONNX_MODEL_DIR") {
            self.load_model(Path::new(&dir));
        }
        true
    }

    /// Called by the derive macro during `IStepper::step` to handle events: the folder chosen in the file browser.
    fn check_event(&mut self, id: &StepperId, key: &str, value: &str) {
        if id != &self.id {
            return;
        }
        if key == FILE_BROWSER_B_SELECT_DIR
            && let Some(dir) = FileBrowserB::get_selected_paths(value).first()
        {
            self.load_model(Path::new(dir));
        }
    }

    /// Called by the derive macro during `IStepper::step`.
    fn draw(&mut self, _token: &MainThreadToken) {
        self.anim_time += Time::get_step() as f32;

        let previous_settings = Ui::get_settings();
        Ui::settings(self.appearence.get_ui_settings_scaled());

        Ui::window("Voice dictation")
            .pose(&mut self.window_pose)
            .size(self.appearence.scaled_window_size())
            .begin();

        // Header row: the close button, the model folder picker, the status, then the record controls.
        if Ui::button("close")
            .image(&self.close_sprite)
            .size(Vec2::new(self.appearence.scale(0.03), self.appearence.scale(0.03)))
            .image_layout(UiBtnLayout::CenterNoText)
            .press()
        {
            SkInfo::send_event(&self.sk_info, StepperAction::remove(self.id.clone()));
        }
        Ui::same_line();

        if Ui::button("Model folder:")
            .size(Vec2::new(self.appearence.scale(0.17), self.appearence.scale(0.03)))
            .press()
        {
            self.spawn_model_browser();
        }
        Ui::same_line();
        Ui::text(&self.status).fit(TextFit::Exact).draw();

        if let Some(dictation) = &mut self.dictation {
            dictation.update();
            let running = dictation.is_running();
            let starting = dictation.is_starting();
            let recording = dictation.is_recording();
            // While the session warms up, the button cancels it; once recording, it stops it.
            let label = if starting {
                "Starting..."
            } else if recording {
                "Stop"
            } else {
                "Record"
            };
            Ui::same_line();
            if Ui::button(label).size(Vec2::new(self.appearence.scale(0.10), self.appearence.scale(0.03))).press() {
                if running {
                    dictation.stop();
                } else {
                    dictation.start();
                }
            }
            Ui::same_line();
            if Ui::button("Clear")
                .size(Vec2::new(self.appearence.scale(0.07), self.appearence.scale(0.03)))
                .press()
            {
                self.dictation_log.clear();
                self.log_scroll = Vec2::ZERO;
            }
            Ui::same_line();
            // The dots follow the microphone level, or the waiting time while the microphone warms up. They live in a
            // fixed-size box so the controls around them never move.
            let dots = if starting {
                waiting_dots(self.anim_time)
            } else if recording {
                level_dots(dictation.level())
            } else {
                ".".to_string()
            };
            Ui::label(dots)
                .size(Vec2::new(self.appearence.scale(0.045), self.appearence.scale(0.024)))
                .use_padding(false)
                .text_align(Align::CenterLeft)
                .draw();
        }
        Ui::next_line();

        // On Android there is no other browser to download a model from.
        #[cfg(target_os = "android")]
        {
            for (index, (label, url)) in MODEL_PAGES.iter().enumerate() {
                if index > 0 {
                    Ui::same_line();
                }
                if Ui::button(*label).size(Vec2::new(self.appearence.scale(0.16), self.appearence.scale(0.03))).press()
                    && !system_deep_link(SystemAction::Browser { url: (*url).to_string() })
                {
                    Log::warn(format!("VoicePanel: could not open '{url}'"));
                }
            }
            Ui::next_line();
        }

        // Provisional text of the current utterance: several lines, scaled to the area.
        let partial = self.dictation.as_ref().map(|dictation| dictation.partial()).unwrap_or_default();
        Ui::text(partial)
            .size(Vec2::new(0.0, self.appearence.scale(0.075)))
            .fit(TextFit::Wrap | TextFit::Exact)
            .draw();
        Ui::next_line();

        // Each flush of the dictation becomes one more line of this text area, separated by CRLF.
        if let Some(dictation) = &mut self.dictation {
            while let Some(sentence) = dictation.take_final() {
                if !self.dictation_log.is_empty() {
                    self.dictation_log.push_str("\r\n");
                }
                self.dictation_log.push_str(&sentence);
            }
        }
        // `TextFit::Exact` would scale the whole log into the box, leaving nothing to scroll, so the log uses `Wrap`:
        // long lines wrap, and the vertical scrollbar appears as soon as the lines overflow. The panel gives the
        // committed text its own frame, so it stands out from the rest of the window.
        Ui::panel_begin(Some(UiPad::Inside));
        Ui::text(&self.dictation_log)
            .scroll(&mut self.log_scroll, UiScroll::Vertical)
            .size(Vec2::new(0.0, self.appearence.scale(0.15)))
            .fit(TextFit::Wrap)
            .draw();
        Ui::panel_end();

        if let Some(error) = self.dictation.as_ref().and_then(|dictation| dictation.error()) {
            Ui::next_line();
            Ui::push_text_style(self.appearence.small_style);
            Ui::label(error).use_padding(true).draw();
            Ui::pop_text_style();
        }

        Ui::window_end();
        Ui::settings(previous_settings);

        // Grab-able knob beside the window: resizes / zooms it, exactly like on the file browser.
        self.appearence.scale_handle(&self.window_pose, "voice_panel_scale_handle");
    }

    /// Called by the derive macro during `IStepper::shutdown`.
    fn close(&mut self, _shutting_down: bool) -> bool {
        // Dropping the dictation stops the microphone.
        self.dictation = None;
        self.shutdown_completed = true;
        self.shutdown_completed
    }
}

#[cfg(not(feature = "no-event-loop"))]
impl VoicePanel {
    /// Load the streaming model found in `dir`, replacing the current one. The running session is stopped first, and
    /// the status line explains what is missing when `dir` does not hold a complete model.
    fn load_model(&mut self, dir: &Path) {
        match DictationModel::from_dir(dir) {
            Some(model) => {
                Log::info(format!("VoicePanel: model found in '{}'", dir.display()));
                self.status = dir.display().to_string();
                self.model_dir = Some(dir.to_path_buf());
                match self.dictation.as_mut() {
                    // Stops the running session and keeps the tool.
                    Some(dictation) => dictation.set_model(model),
                    None => self.dictation = Some(VoiceDictation::new(model)),
                }
            }
            None => {
                self.status = format!(
                    "No streaming model in '{}': need encoder/decoder/joiner .onnx and tokens.txt.",
                    dir.display()
                );
                Log::warn(format!("VoicePanel: {}", self.status));
            }
        }
    }

    /// Launch a [`FileBrowserB`] stepper in directory mode. Since its caller id is this stepper id, the selection comes
    /// back as a [`FILE_BROWSER_B_SELECT_DIR`] event in [`VoicePanel::check_event`].
    fn spawn_model_browser(&mut self) {
        self.browser_counter = (self.browser_counter + 1) % 1000;
        let mut file_browser = FileBrowserB::default();
        file_browser.picker_mode = PickerMode::SelectDirectory;
        // The caller id MUST match this stepper id so we receive the selection back.
        file_browser.caller = self.id.clone();
        file_browser.dir = BrowseLocation::Documents
            .get_path(&self.sk_info)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("/"));
        file_browser.root_dir =
            file_browser.dir.parent().map(PathBuf::from).unwrap_or_else(|| file_browser.dir.clone());
        file_browser.appearence.window_size = Vec2 { x: 0.50, y: 0.6 };
        file_browser.appearence.handle_sprite = Sprite::from_file("icons/zoom.png", None, None).ok();
        file_browser.appearence.set_ui_scale(0.6);
        SkInfo::send_event(
            &self.sk_info,
            StepperAction::add(format!("{}_{}", FILE_BROWSER_ID_PREFIX, self.browser_counter), file_browser),
        );
    }
}

/// Dots driven by the microphone level: 1 at rest, up to 4 when the input is loud.
#[cfg(not(feature = "no-event-loop"))]
fn level_dots(level: f32) -> String {
    ".".repeat(1 + (level.clamp(0.0, 1.0) * 3.999) as usize)
}

/// Dots cycling with time, for the microphone warm-up state.
#[cfg(not(feature = "no-event-loop"))]
fn waiting_dots(time: f32) -> String {
    ".".repeat(1 + (time * 3.0) as usize % 3)
}
