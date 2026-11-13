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
//! [`VoicePanel`] takes the folder of a model from `SHERPA_ONNX_MODEL_DIR`, or from its model folder picker. When
//! neither gave one, it looks for a [`DEFAULT_MODEL_FOLDER`] (`sherpa_models`) folder in the user's `Documents`, which
//! is where pushing the model files is the easiest - `adb push <model dir> /storage/self/primary/Documents/sherpa_models`
//! on Android.
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
//! hides itself with its close button (the stepper stays registered, its batch windows stay on screen), and can be
//! resized / zoomed with its scale handle. The `voice1` demo only registers it. Its texts (title, buttons, statuses)
//! follow the locale selected with [`set_locale`](crate::tools::locales::set_locale), through the `voice_dictation`
//! keys of the `locales/*.toml` catalogues, like the texts of the file browser.
//!
//! The window delivers the committed texts in two ways ([`DictationMode`]):
//!
//! * **Live** - each committed text goes straight to the log of the window, as it comes.
//! * **Batch** - each committed text opens a *batch* window instead, and the windows stack in *series* beside the
//!   panel, so a text can be reread, kept apart, and sent once it reads right. A series is headed by a title bar
//!   (`#N`) that moves the whole series, with `Send` (append the series to the log) and `Clear` (drop it) on its
//!   right end. Every window fits its text - drawn smaller than the panel's, so more of it fits, the panel's own texts
//!   using the same size - and closes itself with its own button, standing beside it on the right; grabbing any of them
//!   tears it off into a series of its own, which follows the hand: dropping it over another series inserts it there,
//!   where it was dropped. A whole series merges the same way: dragged by its header over another series, it hands its
//!   windows over when let go, where it was dropped. The series a window lands in - the one it opens when it lands on
//!   none - becomes the *current* one: the texts the dictation commits next are added to it. While a window or a series
//!   is dragged, the header of the series it would join tints red, which says the two can be merged by letting go. The
//!   grab aura is off for the series, so the background of a window turns blue while the hand is on it.
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

// Texts of the `VoicePanel` window, translated with the same `locales/*.toml` catalogues as the file browser (see
// `tools::locales::set_locale`). Only `VoicePanel` is translated, and it is not compiled without the event loop.
#[cfg(not(feature = "no-event-loop"))]
use rust_i18n::t;

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

#[cfg(all(not(feature = "no-event-loop"), target_os = "android"))]
use crate::permission::{Permission, PermissionState, PermissionType};
#[cfg(not(feature = "no-event-loop"))]
use crate::{
    font::Font,
    framework::Appearence,
    maths::{Pose, Vec2, Vec3},
    sprite::Sprite,
    system::{Align, Hierarchy, Text, TextFit, TextStyle},
    tools::file_browser_b::ALL_BROWSE_LOCATIONS_ROOT,
    ui::{UiBtnLayout, UiMove, UiPad, UiScroll, UiWin},
    util::{Color128, Time, named_colors},
};
#[cfg(not(feature = "no-event-loop"))]
use crate::{
    tools::{
        file_browser_b::{FILE_BROWSER_B_SELECT_DIR, FILE_BROWSER_ID_PREFIX, FileBrowserB},
        os_api::BrowseLocation,
    },
    util::PickerMode,
};

/// Height of the provisional text area, in window units (before scaling): about three lines of the label style.
#[cfg(not(feature = "no-event-loop"))]
const PARTIAL_AREA_HEIGHT: f32 = 0.075;

/// Height of the log area of the committed sentences, in window units (before scaling): about ten lines of the
/// smaller style of the texts of the window.
#[cfg(not(feature = "no-event-loop"))]
const LOG_AREA_HEIGHT: f32 = 0.35;

/// Characters of the provisional text kept for display. Past that, only the tail of the utterance is shown, with a
/// leading ellipsis: the newest words are the ones worth reading.
#[cfg(not(feature = "no-event-loop"))]
const PARTIAL_MAX_CHARS: usize = 110;

/// Width of a batch window, in window units (before scaling). Its height follows its text, so the series reflows.
#[cfg(not(feature = "no-event-loop"))]
const BATCH_WINDOW_WIDTH: f32 = 0.44;

/// Height given to a batch window the UI has not measured yet, in window units (before scaling), so a brand new
/// series stacks its windows without waiting a frame.
#[cfg(not(feature = "no-event-loop"))]
const BATCH_ESTIMATE_HEIGHT: f32 = 0.16;

/// Vertical gap between the windows of the same batch series, in window units (before scaling).
#[cfg(not(feature = "no-event-loop"))]
const BATCH_GAP: f32 = 0.012;

/// Size of the square close button of a batch window, in window units (before scaling).
#[cfg(not(feature = "no-event-loop"))]
const BATCH_CLOSE_SIZE: f32 = 0.03;

/// Size of the `Send` and `Clear` buttons that sit on the header of a series, in window units (before scaling).
#[cfg(not(feature = "no-event-loop"))]
const BATCH_ACTION_SIZE: Vec2 = Vec2::new(0.08, 0.03);

/// Gap between a batch window and the buttons that stand beside it, in window units (before scaling): they are far
/// enough from the window's face to be pinched on their own.
#[cfg(not(feature = "no-event-loop"))]
const BATCH_CLOSE_GAP: f32 = 0.04;

/// How far the buttons of a series stand in front of its windows, in window units (before scaling). Just enough for the
/// grab volume of a window not to catch a pinch meant for a button: a button standing out further shifts with the eyes
/// in a stereo view, which reads as a parallax offset.
#[cfg(not(feature = "no-event-loop"))]
const BATCH_CLOSE_FORWARD: f32 = 0.002;

/// Height of the text of a batch window, relative to the text of the window: 40% smaller.
#[cfg(not(feature = "no-event-loop"))]
const BATCH_TEXT_SCALE: f32 = 0.6;

/// Id of the [`VoicePanel`] stepper: add it with `StepperAction::add(VOICE_PANEL_ID, panel)`, so the events of the
/// panel ([`SHOW_PARAM_WINDOW`], [`DICTATION_STATE`]) have a single, known target to be sent to.
#[cfg(not(feature = "no-event-loop"))]
pub const VOICE_PANEL_ID: &str = "Tool_VoicePanel";

/// Event key that shows (`"true"`) or hides (`"false"`) the parameter window of a [`VoicePanel`]; any other value
/// toggles it. The series of batch windows are drawn in both cases, and a running session keeps delivering to them.
#[cfg(not(feature = "no-event-loop"))]
pub const SHOW_PARAM_WINDOW: &str = "Tool_ShowParamWindow";

/// Event key a [`VoicePanel`] sends to share its recording state with the app: `"true"` while the session records (or
/// warms up, the record button then being a *stop* one), `"false"` when it is idle. The value is sent to the stepper of
/// [`VoicePanel::state_event`] each time it changes, so the app can show that state elsewhere — the *dictation* button
/// of a [`ControlPanel`](crate::tools::control_panel::ControlPanel) shows the same icon pair.
#[cfg(not(feature = "no-event-loop"))]
pub const DICTATION_STATE: &str = "Tool_DictationState";

/// Folder looked up in the user's Documents when no model was given: `Documents/sherpa_models`. Pushing the model
/// files there is all it takes for a [`VoicePanel`] to find them at start.
#[cfg(not(feature = "no-event-loop"))]
pub const DEFAULT_MODEL_FOLDER: &str = "sherpa_models";

/// Tint of the background of a window of a series the user is on. The grab aura is off for the series, so the
/// background says which window is focused: it multiplies the color of the window's body. It shifts it towards blue
/// without brightening it much, so the white text of the window stays readable while the hand is on it.
#[cfg(not(feature = "no-event-loop"))]
const BATCH_FOCUS_TINT: Color128 = Color128::new(0.9, 1.2, 2.6, 1.0);

/// Tint of the header of a series that would take the window the user is dragging: letting go merges the two. Like
/// [`BATCH_FOCUS_TINT`], it multiplies the color of the background, and it shifts it towards red without brightening it
/// much, so the number and the buttons of the header stay readable.
#[cfg(not(feature = "no-event-loop"))]
const BATCH_MERGE_TINT: Color128 = Color128::new(2.6, 0.9, 0.9, 1.0);

/// Distance under which two poses are the same, in meters: the round trip of the pose of a window nobody grabbed is
/// not perfectly exact.
#[cfg(not(feature = "no-event-loop"))]
const BATCH_MOVE_EPSILON: f32 = 0.001;

/// How deep in front of or behind a series a window can be dropped to be inserted into it, in meters.
#[cfg(not(feature = "no-event-loop"))]
const BATCH_DROP_DEPTH: f32 = 0.08;

/// Extra room around a series in which a dropped window still inserts into it, in meters.
#[cfg(not(feature = "no-event-loop"))]
const BATCH_DROP_MARGIN: f32 = 0.02;

/// Where the committed texts of a dictation go, in [`VoicePanel`].
#[cfg(not(feature = "no-event-loop"))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DictationMode {
    /// Each committed text goes straight to the log of the window, as it comes.
    #[default]
    Live,
    /// Each committed text opens a *batch* window instead, so the text can be reread, corrected by chunks, and sent
    /// or dropped once it is right.
    Batch,
}

/// One committed text, shown in its own batch window.
#[cfg(not(feature = "no-event-loop"))]
#[derive(Debug, Clone, PartialEq)]
struct BatchChunk {
    /// Unique id of the chunk: it names the window, and follows the chunk from series to series.
    id: u64,
    /// The text to show.
    text: String,
    /// Height of the window, in meters, as the UI gave it back on the last frame. The window follows its text, and the
    /// series stacks the windows with this height.
    height: f32,
    /// The user grabbed the window of this chunk, and has not let it go yet.
    dragging: bool,
    /// Number of the series this window was torn off, for the duration of that drag: dropping it back on that series
    /// would undo the gesture, so it is not a target until the window is let go.
    from: Option<u64>,
}

/// Batch windows standing in the same series, under a header that moves them all.
#[cfg(not(feature = "no-event-loop"))]
#[derive(Debug, Clone, PartialEq)]
struct BatchGroup {
    /// Number of the series: it names its header, and is unique for the whole session.
    series: u64,
    /// Pose of the bottom centre of the header: the header stands above the first window, and dragging it moves the
    /// whole series.
    pose: Pose,
    /// The chunks, from the top window to the bottom one.
    chunks: Vec<BatchChunk>,
    /// The user moves this series by its header, and has not let it go yet: dropping it over another series merges the
    /// two.
    dragging: bool,
}

/// What the user asked of the batch windows, applied once the frame is drawn.
#[cfg(not(feature = "no-event-loop"))]
#[derive(Debug, Clone, Copy, PartialEq)]
enum BatchIntent {
    /// Close the window of the chunk.
    Close(u64),
    /// Add the whole series holding the chunk to the log, and drop it.
    Send(u64),
    /// Drop the whole series holding the chunk, without sending it.
    Clear(u64),
    /// Take the chunk out of the series numbered `from`: it opens a series of its own, at that pose.
    Detach { id: u64, from: u64, pose: Pose },
    /// Put the chunk back among the series, at the pose it was dropped at.
    Drop { id: u64, pose: Pose },
    /// Merge the series numbered `series` into the one it was let go over, when there is one: its windows take the
    /// place of the drop, and the series they join becomes the current one.
    Merge(u64),
}

/// Height of a series, in meters, from the bottom of its header to the bottom of its last window: the gap under the
/// header, its windows, and the gaps between them.
#[cfg(not(feature = "no-event-loop"))]
fn batch_group_height(chunks: &[BatchChunk], gap: f32) -> f32 {
    chunks.iter().map(|chunk| chunk.height).sum::<f32>() + chunks.len() as f32 * gap
}

/// Pose of the window of the chunk at `index` of a series standing at `pose`: the windows stack downwards, in the frame
/// of the series, each one under the height the UI gave it, the first one a `gap` under the header. `gap` is the scaled
/// gap between two windows.
#[cfg(not(feature = "no-event-loop"))]
fn batch_chunk_pose(pose: Pose, chunks: &[BatchChunk], gap: f32, index: usize) -> Pose {
    let offset = (index as f32 + 1.0) * gap + chunks[..index].iter().map(|chunk| chunk.height).sum::<f32>();
    Pose { position: pose.position - (pose.orientation * Vec3::UP) * offset, orientation: pose.orientation }
}

/// Did the window move (or turn) since the pose we gave it?
#[cfg(not(feature = "no-event-loop"))]
fn batch_pose_moved(expected: Pose, actual: Pose) -> bool {
    // A quaternion and its opposite describe the same rotation, and the UI does not promise which one it gives back:
    // the axes are compared, not the components.
    let turned = |axis: Vec3| (expected.orientation * axis - actual.orientation * axis).length() > BATCH_MOVE_EPSILON;
    (actual.position - expected.position).length() > BATCH_MOVE_EPSILON || turned(Vec3::UP) || turned(Vec3::RIGHT)
}

/// Append `chunk` at the bottom of the current series - the last one in the list - or open a new series at `anchor`
/// when there is none. Returns `true` when a series was opened. [`batch_make_current`] chooses the current series.
#[cfg(not(feature = "no-event-loop"))]
fn batch_push(groups: &mut Vec<BatchGroup>, anchor: Pose, series: u64, chunk: BatchChunk) -> bool {
    match groups.last_mut() {
        Some(group) => {
            group.chunks.push(chunk);
            false
        }
        None => {
            groups.push(BatchGroup { series, pose: anchor, chunks: vec![chunk], dragging: false });
            true
        }
    }
}

/// Make the series numbered `series` the current one, the one the dictation adds its next texts to: it goes last in
/// the list, which is where [`batch_push`] appends. An unknown series changes nothing.
#[cfg(not(feature = "no-event-loop"))]
fn batch_make_current(groups: &mut Vec<BatchGroup>, series: u64) {
    if let Some(index) = groups.iter().position(|group| group.series == series) {
        let group = groups.remove(index);
        groups.push(group);
    }
}

/// Merge the series numbered `series` into the one numbered `target`, under the `slot` first windows of it: the windows
/// of the merged series keep their order, and take the place of the drop. The target becomes the current series.
/// Returns `false` when one of the two is unknown, or when they are the same.
#[cfg(not(feature = "no-event-loop"))]
fn batch_merge(groups: &mut Vec<BatchGroup>, series: u64, target: u64, slot: usize) -> bool {
    if series == target {
        return false;
    }
    let Some(index) = groups.iter().position(|group| group.series == series) else {
        return false;
    };
    if !groups.iter().any(|group| group.series == target) {
        return false;
    }
    // The merged series leaves the list first: the target may stand after it.
    let merged = groups.remove(index);
    let Some(index) = groups.iter().position(|group| group.series == target) else {
        return false;
    };
    let slot = slot.min(groups[index].chunks.len());
    groups[index].chunks.splice(slot..slot, merged.chunks);
    batch_make_current(groups, target);
    true
}

/// The point of a series that decides where its windows land when the series itself is dropped: the middle of its
/// header bar, which is what the user holds. `line` is the height of that bar.
#[cfg(not(feature = "no-event-loop"))]
fn batch_header_point(group: &BatchGroup, line: f32) -> Vec3 {
    group.pose.position + group.pose.orientation * Vec3::UP * (line / 2.0)
}

/// Remove the chunk `id` from its series, and the series itself when it was its last chunk.
#[cfg(not(feature = "no-event-loop"))]
fn batch_remove(groups: &mut Vec<BatchGroup>, id: u64) -> Option<BatchChunk> {
    for group_index in 0..groups.len() {
        if let Some(chunk_index) = groups[group_index].chunks.iter().position(|chunk| chunk.id == id) {
            let chunk = groups[group_index].chunks.remove(chunk_index);
            if groups[group_index].chunks.is_empty() {
                groups.remove(group_index);
            }
            return Some(chunk);
        }
    }
    None
}

/// Remove the whole series holding the chunk `id`, and give it back.
#[cfg(not(feature = "no-event-loop"))]
fn batch_take_group(groups: &mut Vec<BatchGroup>, id: u64) -> Option<BatchGroup> {
    let group_index = groups.iter().position(|group| group.chunks.iter().any(|chunk| chunk.id == id))?;
    Some(groups.remove(group_index))
}

/// Where a window dropped at `point` (the centre of that window, in world space) inserts itself: the series it lands
/// on, and the place it takes in it. `None` when it lands on no series. `width` is the scaled width of a window, `line`
/// the height of the title bar of a series, and `gap` the scaled gap between two windows; the heights of the windows and
/// of the body of the header are the ones the UI measured. The series numbered in `skip` are left out: the one a window
/// was just torn off, and the one it stands in meanwhile, which goes away with it.
#[cfg(not(feature = "no-event-loop"))]
fn batch_drop_slot(
    groups: &[BatchGroup],
    point: Vec3,
    width: f32,
    line: f32,
    gap: f32,
    skip: &[u64],
) -> Option<(usize, usize)> {
    for (group_index, group) in groups.iter().enumerate() {
        if skip.contains(&group.series) {
            continue;
        }
        let in_series = point - group.pose.position;
        let across = Vec3::dot(in_series, group.pose.orientation * Vec3::RIGHT);
        let down = Vec3::dot(in_series, group.pose.orientation * Vec3::UP);
        let depth = Vec3::dot(in_series, group.pose.orientation * Vec3::FORWARD);
        let height = batch_group_height(&group.chunks, gap);
        if across.abs() <= width / 2.0 + BATCH_DROP_MARGIN
            && depth.abs() <= BATCH_DROP_DEPTH
            && down <= line + BATCH_DROP_MARGIN
            && down >= -height - BATCH_DROP_MARGIN
        {
            // The place it takes: the number of windows whose middle stands above the drop point.
            let mut slot = 0;
            let mut top = gap;
            for chunk in &group.chunks {
                if down >= -(top + chunk.height / 2.0) {
                    break;
                }
                top += chunk.height + gap;
                slot += 1;
            }
            return Some((group_index, slot));
        }
    }
    None
}

/// The series the user would merge something into if it were let go now: `None` when it would land on no series.
///
/// Two things can be held: a *window*, which would join the series under it (see [`batch_drop_slot`]), or a *series*,
/// dragged by its header, whose windows would join the series under that bar. It is what
/// [`VoicePanel::drop_chunk`] and [`VoicePanel::merge_group`] will find, read a frame ahead so that series can say it -
/// its header tints red, which tells the user the two can be merged. `width` is the scaled width of a window, `line`
/// the height of the title bar of a series, and `gap` the scaled gap between two windows.
#[cfg(not(feature = "no-event-loop"))]
fn batch_drop_target(groups: &[BatchGroup], width: f32, line: f32, gap: f32) -> Option<u64> {
    // A series held by its header: the bar under the hand is the point that decides where its windows go.
    if let Some(group) = groups.iter().find(|group| group.dragging) {
        return batch_drop_slot(groups, batch_header_point(group, line), width, line, gap, &[group.series])
            .map(|(index, _)| groups[index].series);
    }
    // The window the user holds, and the series it stands in until it is dropped: that series goes away with the
    // window, so it is not a landing place, just as in `drop_chunk`.
    let group = groups.iter().find(|group| group.chunks.iter().any(|chunk| chunk.dragging))?;
    let index = group.chunks.iter().position(|chunk| chunk.dragging)?;
    let chunk = &group.chunks[index];
    let pose = batch_chunk_pose(group.pose, &group.chunks, gap, index);
    // A window is grabbed and dropped anywhere on its body: its middle is the point that decides where it lands.
    let middle = pose.position - (pose.orientation * Vec3::UP) * (chunk.height / 2.0);
    // The series the window was torn off is not a target for this drag either. Both numbers come the same way as in
    // `drop_chunk`, a duplicate being harmless.
    let skip = [chunk.from.unwrap_or(group.series), group.series];
    batch_drop_slot(groups, middle, width, line, gap, &skip).map(|(index, _)| groups[index].series)
}

/// What the status line of the parameter window says. The texts are looked up when the window is drawn, so a locale
/// change (see [`set_locale`](crate::tools::locales::set_locale)) applies to the next frame, like the other texts of
/// the window.
#[cfg(not(feature = "no-event-loop"))]
#[derive(Debug, Clone, PartialEq)]
enum ModelStatus {
    /// No model folder chosen yet.
    NoModel,
    /// The folder of the streaming model in use.
    Loaded(String),
    /// A folder that does not hold a complete streaming model.
    Invalid(String),
}

#[cfg(not(feature = "no-event-loop"))]
impl ModelStatus {
    /// The text of the status line, in the current locale.
    fn text(&self) -> std::borrow::Cow<'static, str> {
        match self {
            ModelStatus::NoModel => t!("voice_dictation.status_no_model"),
            ModelStatus::Loaded(dir) => std::borrow::Cow::Owned(dir.clone()),
            ModelStatus::Invalid(dir) => t!("voice_dictation.status_no_streaming_model", dir = dir.clone()),
        }
    }
}

/// A ready-to-use dictation window: model folder picker, record/stop, live level, provisional text and a
/// scrollable log of the committed sentences.
///
/// It appears in front of the user ([`Ui::popup_pose`]), can be moved, resized and zoomed with its scale handle — the
/// icon of the tool (`icons/voice_dictation.png`) marks its knob — and hides itself with its close button
/// ([`Sprite::close`]) without removing the stepper: the series of batch windows stay on screen, and the
/// [`SHOW_PARAM_WINDOW`] event shows the window again.
///
/// ### Live and batch modes
/// [`VoicePanel::mode`] chooses where the committed texts go:
/// * [`DictationMode::Live`] - each committed text is appended to the log of the window, as it comes, and the log is
///   brought to its end so the new line is always the one on screen.
/// * [`DictationMode::Batch`] - each committed text opens a *batch* window instead, stacked in *series*: a series is
///   headed by a title bar of its own (numbered `#N`) that moves it as a whole, with **Send** (which appends the
///   series to the log) and **Clear** (which drops it) on its right end. Every window fits its text - drawn 40% smaller
///   than the text of the panel, which uses the same size - with its own text area and its close button outside, on its
///   right. Grabbing any window tears it off: it opens a series of its own, follows the hand, and dropping it over
///   another series inserts it there, where it was dropped (the series it comes from is not a target for that drag, so
///   the tear holds). A whole series merges the same way: its header dragged over another series hands its windows over
///   when let go, where it was dropped. The series a window lands in - the one it opens when it lands on none - becomes
///   the *current* one: the dictation adds its next texts there, and so does the series the user moves by its header.
///   While a window or a series is dragged, the header of the series it would join turns red, which says that letting go
///   merges the two. As the grab aura is off for the series, the background of a window turns blue while the hand is on
///   it.
///
/// A series is placed beside the window when it opens, and stays there when the mode goes back to
/// [`DictationMode::Live`]: the pending texts are never lost.
///
/// The parameter window itself can be hidden ([`VoicePanel::show_param_window`], or the [`SHOW_PARAM_WINDOW`] event):
/// the series stay on screen, and a running session keeps delivering to them.
///
/// ### Fields that can be changed before initialization:
/// * `appearence` - The look of the window (text styles, size, scale handle...), at 60% scale by default. See
///   [`Appearence`] and [`Appearence::set_ui_scale`].
/// * `mode` - Where the committed texts go, [`DictationMode::Live`] by default.
/// * `show_param_window` - Whether the parameter window is drawn, `true` by default. The series of batch windows are
///   drawn either way, and a running session keeps delivering to them.
/// * `state_event` - An optional `(stepper id, event key)` event to send the recording state to, `None` by default.
///   Give [`VOICE_PANEL_ID`] as the target and [`DICTATION_STATE`] as the key: the value is `"true"` while the session
///   records (or warms up), `"false"` when it is idle.
/// * `window_pose` - Where the window stands. Default is [`Pose::IDENTITY`], replaced by [`Ui::popup_pose`] at start.
///
/// ### Events this stepper is listening to:
/// * [`SHOW_PARAM_WINDOW`] - Shows the parameter window (`"true"`) or hides it (`"false"`); any other value toggles
///   it. The batch series stay on screen, and a running session keeps feeding them. The window hides itself with its
///   close button, without removing the stepper, and this event shows it again.
///
/// ### Events this stepper sends:
/// * [`DICTATION_STATE`] - The recording state of the session, sent on the stepper of [`VoicePanel::state_event`] each
///   time it changes: `"true"` while the session records (or warms up), `"false"` when it is idle. Send it on
///   [`VOICE_PANEL_ID`] (the panel's own id), which is where the
///   [`ControlPanel`](crate::tools::control_panel::ControlPanel) of this crate listens to show the same *record* /
///   *stop* icon on its dictation button.
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::tools::voice_dictation::{DictationMode, VOICE_PANEL_ID, VoicePanel};
///
/// let mut panel = VoicePanel::default();
/// panel.show_param_window = true;
/// panel.mode = DictationMode::Batch;
/// // Texts queued before the window is on screen open their series once it has a place.
/// panel.add_batch_text("Hello, this is the first recognized text.");
/// panel.add_batch_text("And a second one, a bit longer, to see the window fit its own text.");
/// sk.send_event(StepperAction::add(VOICE_PANEL_ID, panel));
///
/// filename_scr = "screenshots/voice_panel.jpeg";
/// width_scr = 400; height_scr = 400; fov_scr = 60.0;
/// from_scr = maths::Vec3::new(0.17, 0.0, 0.30); at_scr = maths::Vec3::new(0.17, 0.0, -0.45);
/// test_screenshot!( // !!!! Get a proper main loop !!!!
/// );
/// # sk::Sk::shutdown();
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/voice_panel.jpeg" alt="screenshot" width="200">
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
    /// Where the committed texts go: straight to the log, or into batch windows. See [`DictationMode`].
    pub mode: DictationMode,
    /// Draw the parameter window (model folder, record controls, mode, texts)? Default is `true`.
    ///
    /// The series of batch windows are drawn **whether it is shown or not**: hiding the parameters leaves the user's
    /// texts on screen, and a running session keeps delivering to them. Set it directly, or with the
    /// [`SHOW_PARAM_WINDOW`] event.
    pub show_param_window: bool,
    /// Optional event sent to share the recording state, as `(target stepper id, event key)` — give [`VOICE_PANEL_ID`]
    /// as the target (the panel's own id) and [`DICTATION_STATE`] as the key: the value is `"true"` while the session
    /// records (or warms up, the record button then being a *stop* one), `"false"` when it is idle, and it is sent each
    /// time the state changes. The app can then show that state elsewhere: the
    /// [`ControlPanel`](crate::tools::control_panel::ControlPanel) of this crate listens to it and carries the same icon
    /// pair on its **Dictation** button. Default is `None`.
    pub state_event: Option<(StepperId, String)>,

    /// The dictation session, `None` until a model is loaded.
    dictation: Option<VoiceDictation>,
    /// The recording state last shared with the app through [`VoicePanel::state_event`], so the event is only sent
    /// when it changes.
    shared_recording: bool,
    /// The committed sentences of the dictation, one per CRLF.
    dictation_log: String,
    /// Scroll state of `dictation_log`, driven by the scrollbar of its text area.
    log_scroll: Vec2,
    /// Whether new text arrived in `dictation_log` since the last draw. The log is then brought to its end, which
    /// needs the width of the text area: it is only known at draw time.
    log_follow_end: bool,
    /// The series of batch windows. The last one is the *current* series - the one the dictation adds the texts it
    /// commits to, and the one a new series stands beside.
    batch_groups: Vec<BatchGroup>,
    /// Texts given to [`VoicePanel::add_batch_text`] before the window started, waiting for a series to open at its
    /// place.
    batch_pending: Vec<String>,
    /// Id given to the next batch chunk.
    batch_next_id: u64,
    /// Number given to the next series of batch windows, which names its header.
    batch_next_series: u64,
    /// Text style of the batch windows, [40% smaller](BATCH_TEXT_SCALE) than the text of the window. The text style of
    /// the UI is not scaled by the appearence, so this one is not either.
    batch_style: TextStyle,
    /// Height of `batch_style`, in meters: the text of the UI, 40% smaller.
    batch_text_height: f32,
    /// What the status line of the parameter window says. The texts are looked up when the window is drawn, so a
    /// locale change applies to the next frame, like the other texts of the window.
    status: ModelStatus,
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
        let font = Font::default();
        let mut appearence = Appearence::new(&font, 0.024);
        appearence.window_size = Vec2::new(0.62, 0.0);
        appearence.min_window_size = Vec2::new(0.50, 0.0);
        appearence.scale_handle_default_offset = Vec3::new(0.33, 0.04, 0.006);
        appearence.handle_sprite = Sprite::from_file("icons/voice_dictation.png", None, None).ok();
        appearence.set_ui_scale(0.6);
        // The text of the batch windows is drawn smaller than the one of the window. The text style of the UI is not
        // scaled by the appearence, so neither is this one.
        let batch_text_height = Ui::get_text_style().get_layout_height() * BATCH_TEXT_SCALE;
        Self {
            id: VOICE_PANEL_ID.to_string(),
            sk_info: None,
            shutdown_completed: false,
            appearence,
            window_pose: Pose::IDENTITY,
            model_dir: None,
            mode: DictationMode::Live,
            show_param_window: false,
            state_event: None,
            dictation: None,
            shared_recording: false,
            dictation_log: String::new(),
            log_scroll: Vec2::ZERO,
            log_follow_end: false,
            batch_groups: Vec::new(),
            batch_pending: Vec::new(),
            batch_next_id: 1,
            batch_next_series: 1,
            batch_style: TextStyle::from_font(&font, batch_text_height, named_colors::WHITE),
            batch_text_height,
            status: ModelStatus::NoModel,
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

        // The texts given before the window was on screen can open their series now that it has a place.
        for text in std::mem::take(&mut self.batch_pending) {
            self.push_batch(text);
        }

        #[cfg(target_os = "android")]
        if Permission::get_state(PermissionType::Microphone) == PermissionState::Capable {
            Permission::request(&[PermissionType::Microphone]);
        }

        // An environment variable lets an automated run skip the file browser.
        if let Some(dir) = std::env::var_os("SHERPA_ONNX_MODEL_DIR") {
            self.load_model(Path::new(&dir));
        }

        // No model given: `Documents/sherpa_models` is the conventional place to push one to, so it is tried too.
        if self.model_dir.is_none()
            && let Some(documents) = BrowseLocation::Documents.get_path(&self.sk_info)
        {
            let dir = documents.join(DEFAULT_MODEL_FOLDER);
            if dir.is_dir() {
                self.load_model(&dir);
            }
        }
        true
    }

    /// Called by the derive macro during `IStepper::step` to handle events: the folder chosen in the file browser, and
    /// the visibility of the parameter window.
    fn check_event(&mut self, id: &StepperId, key: &str, value: &str) {
        if id != &self.id {
            return;
        }
        if key.eq(SHOW_PARAM_WINDOW) {
            // "true" or "false" sets it, anything else toggles it.
            self.show_param_window = value.parse().unwrap_or(!self.show_param_window);
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
        // The texts of the window and of the batch windows share the smaller style. The text style of the UI is not
        // scaled by the appearence, so neither is this one.
        self.batch_style.layout_height(self.batch_text_height);

        // The session runs whether the parameter window is on screen or not: the texts it commits are the work of the
        // user, and the series that show them are always drawn.
        self.update_session();

        if self.show_param_window {
            self.draw_param_window();
        }

        // The series live outside the window of the panel, and outlive it.
        self.draw_batch();

        Ui::settings(previous_settings);

        if self.show_param_window {
            // Grab-able knob beside the window: resizes / zooms it, exactly like on the file browser.
            self.appearence.scale_handle(&self.window_pose, "voice_panel_scale_handle");
        }
    }

    /// Move the session forward, and give each committed text to the log or to a batch window.
    ///
    /// It runs even when the parameter window is hidden, so a session started before hiding it keeps delivering its
    /// texts to the series, which stay on screen. The recording state is also shared through
    /// [`VoicePanel::state_event`] there, so the app shows it even while the window is hidden.
    fn update_session(&mut self) {
        if let Some(dictation) = &mut self.dictation {
            dictation.update();
        }
        let mut finals = Vec::new();
        if let Some(dictation) = &mut self.dictation {
            while let Some(sentence) = dictation.take_final() {
                finals.push(sentence);
            }
        }
        for sentence in finals {
            match self.mode {
                DictationMode::Live => self.push_log(&sentence),
                DictationMode::Batch => self.push_batch(sentence),
            }
        }

        // The app may show the recording state elsewhere (the dictation button of its control panel, for instance):
        // it is shared each time it changes. Warming up counts as recording, since the button then is a *stop* one.
        let recording = self
            .dictation
            .as_ref()
            .map(|dictation| dictation.is_starting() || dictation.is_recording())
            .unwrap_or(false);
        if recording != self.shared_recording {
            self.shared_recording = recording;
            if let Some((target, key)) = &self.state_event {
                let value = if recording { "true" } else { "false" };
                SkInfo::send_event(&self.sk_info, StepperAction::event(target.as_str(), key.as_str(), value));
            }
        }
    }

    /// Draw the parameter window: model folder, record controls, mode, provisional text and log.
    fn draw_param_window(&mut self) {
        Ui::window(t!("voice_dictation.title"))
            .pose(&mut self.window_pose)
            .size(self.appearence.scaled_window_size())
            .begin();

        // Header row: the close button, the model folder picker and the status.
        if Ui::button("close")
            .image(&self.close_sprite)
            .size(Vec2::new(self.appearence.scale(0.03), self.appearence.scale(0.03)))
            .image_layout(UiBtnLayout::CenterNoText)
            .press()
        {
            // The close button only hides the window, it does not remove the stepper: the series of batch windows stay
            // on screen, and a running session keeps delivering to them. The `SHOW_PARAM_WINDOW` event shows it again.
            self.show_param_window = false;
        }
        Ui::same_line();

        if Ui::button(t!("voice_dictation.model_folder"))
            .size(Vec2::new(self.appearence.scale(0.17), self.appearence.scale(0.03)))
            .press()
        {
            self.spawn_model_browser();
        }
        Ui::same_line();
        Ui::text(self.status.text()).fit(TextFit::Exact).draw();

        // The recording controls take a line of their own: the header line is already full once the status is on it.
        let mut after_controls = false;
        if let Some(dictation) = &mut self.dictation {
            let running = dictation.is_running();
            let starting = dictation.is_starting();
            let recording = dictation.is_recording();
            // While the session warms up, the button cancels it; once recording, it stops it. The recipe is the one
            // of the REC button of the mixer demo: the built-in toggle icons, the label, and a tint that is green
            // when the session is idle and red while it records. The panel around the button gives it some relief.
            let label = if starting {
                t!("voice_dictation.starting")
            } else if recording {
                t!("voice_dictation.stop")
            } else {
                t!("voice_dictation.record")
            };
            let icon = if starting || recording { Sprite::toggle_on() } else { Sprite::toggle_off() };
            Ui::panel_begin(Some(UiPad::Inside));
            if starting || recording {
                Ui::push_tint(Color128::from(named_colors::RED).to_gamma());
            } else {
                Ui::push_tint(Color128::from(named_colors::GREEN).to_gamma());
            }
            if Ui::button(label)
                .size(Vec2::new(self.appearence.scale(0.13), self.appearence.scale(0.03)))
                .image(&icon)
                .image_layout(UiBtnLayout::Left)
                .press()
            {
                if running {
                    dictation.stop();
                } else {
                    dictation.start();
                }
            }
            Ui::pop_tint();
            Ui::same_line();
            if Ui::button(t!("voice_dictation.clear_log"))
                .size(Vec2::new(self.appearence.scale(0.07), self.appearence.scale(0.03)))
                .press()
            {
                self.dictation_log.clear();
                self.log_scroll = Vec2::ZERO;
                self.log_follow_end = false;
            }
            Ui::panel_end();

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
            after_controls = true;
        }

        // Where the committed texts go: left of the recording controls, in a frame of its own so the pair reads as one
        // setting. The radios carry their sprites, so the active one shows.
        if after_controls {
            Ui::same_line();
        }
        Ui::panel_begin(Some(UiPad::Inside));
        if Ui::radio(t!("voice_dictation.mode_live"), self.mode == DictationMode::Live)
            .images(Sprite::radio_off(), Sprite::radio_on())
            .size(Vec2::new(self.appearence.scale(0.09), self.appearence.scale(0.03)))
            .press()
        {
            self.mode = DictationMode::Live;
        }
        Ui::same_line();
        if Ui::radio(t!("voice_dictation.mode_batch"), self.mode == DictationMode::Batch)
            .images(Sprite::radio_off(), Sprite::radio_on())
            .size(Vec2::new(self.appearence.scale(0.10), self.appearence.scale(0.03)))
            .press()
        {
            self.mode = DictationMode::Batch;
        }
        Ui::panel_end();

        // Provisional text of the current utterance, in its own frame like the log below. It keeps the text style's own
        // size: `TextFit::Exact` must be avoided here, since it scales the whole utterance to fit a single line, so a
        // partial grows smaller as it grows longer (long partials, as the streaming models of the Android devices
        // produce, end up unreadable). `Wrap` keeps every word at the style size, `Clip` stops it from spilling out of
        // its box, and only the tail of a long utterance is shown: the newest words are the ones worth reading.
        // The texts of the window use the smaller style of the batch windows, so more of them fits.
        let partial = self.dictation.as_ref().map(|dictation| dictation.partial()).unwrap_or_default();
        Ui::panel_begin(Some(UiPad::Outside));
        Ui::push_text_style(self.batch_style);
        Ui::text(partial_tail(partial, PARTIAL_MAX_CHARS))
            .size(Vec2::new(0.0, self.appearence.scale(PARTIAL_AREA_HEIGHT)))
            .fit(TextFit::Wrap | TextFit::Clip)
            .draw();
        Ui::pop_text_style();
        Ui::panel_end();

        Ui::hseparator();

        // `TextFit::Exact` would scale the whole log into the box, leaving nothing to scroll, so the log uses `Wrap`:
        // long lines wrap, and the vertical scrollbar appears as soon as the lines overflow. The panel gives the
        // committed text its own frame, so it stands out from the rest of the window, and it is drawn in the smaller
        // style of the batch windows, so more lines fit.
        Ui::panel_begin(Some(UiPad::Outside));
        Ui::push_text_style(self.batch_style);
        // A new text brings the log to its end: the scroll is the offset of the text in meters, so it is set to what
        // the log has below its visible area. The width of the text area is only known here, so the arrival of a text
        // only raises `log_follow_end`. The UI lays the text out in the width minus a line height, which the
        // scrollbar takes, so the measure does the same; the log simply stays at its top while it fits.
        if self.log_follow_end {
            self.log_follow_end = false;
            let area_height = self.appearence.scale(LOG_AREA_HEIGHT);
            let wrap_width = (Ui::get_layout_remaining().x - Ui::get_line_height()).max(0.0);
            let text_height = Text::size_layout(&self.dictation_log, Some(self.batch_style), Some(wrap_width)).y;
            self.log_scroll = Vec2::new(0.0, (text_height - area_height).max(0.0));
        }
        Ui::text(&self.dictation_log)
            .scroll(&mut self.log_scroll, UiScroll::Vertical)
            .size(Vec2::new(0.0, self.appearence.scale(LOG_AREA_HEIGHT)))
            .fit(TextFit::Wrap)
            .draw();
        Ui::pop_text_style();
        Ui::panel_end();

        if let Some(error) = self.dictation.as_ref().and_then(|dictation| dictation.error()) {
            Ui::next_line();
            Ui::push_text_style(self.appearence.small_style);
            Ui::label(error).use_padding(true).draw();
            Ui::pop_text_style();
        }

        Ui::window_end();
    }

    /// Called by the derive macro during `IStepper::shutdown`.
    fn close(&mut self, _shutting_down: bool) -> bool {
        // Dropping the dictation stops the microphone.
        self.dictation = None;
        self.shutdown_completed = true;
        self.shutdown_completed
    }

    /// Load the streaming model found in `dir`, replacing the current one. The running session is stopped first, and
    /// the status line explains what is missing when `dir` does not hold a complete model.
    fn load_model(&mut self, dir: &Path) {
        match DictationModel::from_dir(dir) {
            Some(model) => {
                Log::info(format!("VoicePanel: model found in '{}'", dir.display()));
                self.status = ModelStatus::Loaded(dir.display().to_string());
                self.model_dir = Some(dir.to_path_buf());
                match self.dictation.as_mut() {
                    // Stops the running session and keeps the tool.
                    Some(dictation) => dictation.set_model(model),
                    None => self.dictation = Some(VoiceDictation::new(model)),
                }
            }
            None => {
                self.status = ModelStatus::Invalid(dir.display().to_string());
                Log::warn(format!("VoicePanel: {}", self.status.text()));
            }
        }
    }

    /// Launch a [`FileBrowserB`] stepper in directory mode, opened on the folder of the model in use (or the platform
    /// documents folder when there is none). Since its caller id is this stepper id, the selection comes back as a
    /// [`FILE_BROWSER_B_SELECT_DIR`] event in [`VoicePanel::check_event`].
    fn spawn_model_browser(&mut self) {
        self.browser_counter = (self.browser_counter + 1) % 1000;
        let mut file_browser = FileBrowserB::default();
        file_browser.picker_mode = PickerMode::SelectDirectory;
        // The caller id MUST match this stepper id so we receive the selection back.
        file_browser.caller = self.id.clone();
        // The folder of the model in use is where the user was last time, so the browser opens there.
        file_browser.dir = self
            .model_dir
            .clone()
            .or_else(|| BrowseLocation::Documents.get_path(&self.sk_info))
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("/"));
        file_browser.root_dir = PathBuf::from(ALL_BROWSE_LOCATIONS_ROOT);
        // Shorter than the browser default, with the floor of the resize handle lowered as well so that the window is
        // not lifted back to the default height at start.
        file_browser.appearence.window_size = Vec2 { x: 0.50, y: 0.42 };
        file_browser.appearence.min_window_size.y = 0.30;
        file_browser.appearence.handle_sprite = Sprite::from_file("icons/zoom.png", None, None).ok();
        file_browser.appearence.set_ui_scale(0.6);
        SkInfo::send_event(
            &self.sk_info,
            StepperAction::add(format!("{}_{}", FILE_BROWSER_ID_PREFIX, self.browser_counter), file_browser),
        );
    }

    /// Scaled width of a batch window: its height follows its text.
    fn batch_width(&self) -> f32 {
        self.appearence.scale(BATCH_WINDOW_WIDTH)
    }

    /// Scaled distance between the right edge of a batch window and the left edge of the next series: the close button
    /// of the windows stands in between.
    fn batch_side_room(&self) -> f32 {
        self.appearence.scale(BATCH_CLOSE_GAP + BATCH_CLOSE_SIZE + BATCH_GAP)
    }

    /// Where a new series of batch windows opens: beside the window of the panel, on its left, in the frame of that
    /// window, so the series faces the user like the panel does.
    fn batch_anchor(&self) -> Pose {
        let offset = self.appearence.scaled_window_size().x / 2.0
            + self.batch_width() / 2.0
            + self.batch_side_room()
            + self.appearence.scale(0.02);
        Pose {
            position: self.window_pose.position - (self.window_pose.orientation * Vec3::RIGHT) * offset,
            orientation: self.window_pose.orientation,
        }
    }

    /// Where a new series opens: beside the current series (the last one in the list), on its left, in the frame of
    /// that series, so the series line up, with the room of the close buttons in between. The very first one opens
    /// beside the panel.
    fn batch_next_anchor(&self) -> Pose {
        let Some(last) = self.batch_groups.last() else {
            return self.batch_anchor();
        };
        let offset = self.batch_width() + self.batch_side_room();
        Pose {
            position: last.pose.position - (last.pose.orientation * Vec3::RIGHT) * offset,
            orientation: last.pose.orientation,
        }
    }

    /// Add `text` as a new batch window, at the bottom of the current series - or in a new series, when there is no
    /// series yet. The series the user moves, or the one a dragged window lands in, is the current one, see
    /// `VoicePanel::draw_batch`.
    ///
    /// It is what the dictation does with each committed text in [`DictationMode::Batch`]; calling it directly lets an
    /// application hand over texts of its own. The windows are drawn in both modes, so a text can be queued before the
    /// user switches to [`DictationMode::Batch`]. Texts added before the window started are kept, and placed once it is
    /// on screen.
    pub fn add_batch_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        if self.sk_info.is_none() {
            // Not started yet: the window has no place on screen, so the text waits for the series to open.
            self.batch_pending.push(text);
        } else {
            self.push_batch(text);
        }
    }

    /// Append `text` to the log of the committed sentences, as a new line, and bring the log to its end at the next
    /// draw - a new line is what the user is waiting to read.
    fn push_log(&mut self, text: &str) {
        if !self.dictation_log.is_empty() {
            self.dictation_log.push_str("\r\n");
        }
        self.dictation_log.push_str(text);
        self.log_follow_end = true;
    }

    /// Add `text` as a new batch window, at the bottom of the current series - or in a new series, when there is no
    /// series yet.
    fn push_batch(&mut self, text: String) {
        let chunk = BatchChunk {
            id: self.batch_next_id,
            text,
            // The UI measures the height of the window on its first frame: until then, the series uses this estimate.
            height: self.appearence.scale(BATCH_ESTIMATE_HEIGHT),
            dragging: false,
            from: None,
        };
        self.batch_next_id += 1;
        let anchor = self.batch_next_anchor();
        if batch_push(&mut self.batch_groups, anchor, self.batch_next_series, chunk) {
            self.batch_next_series += 1;
        }
    }

    /// Draw the series of batch windows, and apply what the user asks of them.
    ///
    /// Each series is drawn as its header bar, which stands alone so grabbing it moves the whole series, then one window
    /// per chunk. Grabbing any window tears it off: it opens a series of its own that follows the hand, and joins the
    /// series it is dropped on, unless that is the series it comes from. A series of a single window is moved by its
    /// window, header included.
    ///
    /// A series the user moves by its header becomes the *current* one: the texts the dictation commits next are added
    /// to it, at the bottom, and a series opened later stands beside it. So does the series a dragged window lands in,
    /// or the one a torn-off window opens: a detachment hands the dictation over to the new batch it makes.
    ///
    /// A window held by the user is a drop waiting to happen: the header of the series it would join tints red, so the
    /// user sees where to let go to merge the two (see [`batch_drop_target`]). The same holds for a series dragged by
    /// its header: let go over another series and the two merge, the windows of the moved one taking the place of the
    /// drop.
    fn draw_batch(&mut self) {
        if self.batch_groups.is_empty() {
            return;
        }
        // Drawing needs `&mut self` for the sprites and the text styles: the series leave the tool meanwhile.
        let mut groups = std::mem::take(&mut self.batch_groups);
        let gap = self.appearence.scale(BATCH_GAP);
        // A fixed width wraps the text, an empty height lets the UI fit the window to that text.
        let size = Vec2::new(self.batch_width(), 0.0);
        let mut intents = Vec::new();
        // The series the user moved this frame: it becomes the current one, where the dictation adds the next texts.
        let mut moved_series = None;
        // What letting go would do with what the user is dragging - a window or a whole series: the series it would
        // join tints its header red, so the user sees the two can be merged. The pose of what is held is only known
        // from the previous frame here, and a frame is nothing at the speed of a hand.
        let merge_target = batch_drop_target(&groups, self.batch_width(), Ui::get_line_height(), gap);

        // The grab aura draws a glowing volume around every window: on a stack of batch windows it hides the text and
        // the buttons, so it is turned off for them. They stay grabbable by their body.
        Ui::push_grab_aura(false);
        // Every id of the series hangs under this one, so their short titles ("1", "2"…) can not collide with the
        // windows of the application.
        Ui::push_id("voice_dictation_batch");

        for group in groups.iter_mut() {
            // The header of the series is a window of its own, so its grab volume is exactly that bar: dragging it moves
            // the whole series. It shows the number of the series behind a `#`, and its buttons sit on the bar, laid out
            // by hand to keep the number room.
            let pose_before = group.pose;
            Ui::window(format!("#{}", group.series))
                .pose(&mut group.pose)
                .size(size)
                .window_type(UiWin::Head)
                .move_type(UiMove::FaceUser)
                .begin();
            let focused = Ui::get_last_element_focused().is_active();
            // The grab handle of the header is the last element the UI made: it is active while the user moves the
            // series, and stops when the hand lets go.
            let held = Ui::get_last_element_active().is_active();
            // The header says what a drop would do with the series: red when it would take what the user is dragging -
            // letting go merges the two - and blue while the hand is on the header itself.
            let tint = if merge_target == Some(group.series) {
                Some(BATCH_MERGE_TINT)
            } else if focused {
                Some(BATCH_FOCUS_TINT)
            } else {
                None
            };
            end_batch_window(tint);
            // The pose the UI gives back tells whether the user moves the series by its header: it then becomes the
            // current one, where the dictation adds the next texts, and it is held for a drop that merges it into
            // another series.
            if batch_pose_moved(pose_before, group.pose) {
                moved_series = Some(group.series);
                group.dragging = true;
            } else if group.dragging && !held {
                // Let go: the series joins the one it was dropped on, if it was dropped on one.
                group.dragging = false;
                intents.push(BatchIntent::Merge(group.series));
            }

            let action = self.appearence.scale_size(BATCH_ACTION_SIZE);
            let margin = self.appearence.scale(0.012);
            let forward = -self.appearence.scale(BATCH_CLOSE_FORWARD);
            // The buttons are centred on the height of the bar, at the right end of it, and stand in front of the
            // header so a pinch on them is not caught by its grab volume. A UI element hangs under its top left corner:
            // the top of the button is half a button above the middle of the bar.
            let y = (Ui::get_line_height() + action.y) / 2.0;
            let clear_x = size.x / 2.0 - margin - action.x;
            // The buttons keep their readable text, and the series id keeps them apart from the other series.
            Ui::push_id(format!("series_{}", group.series));
            Hierarchy::push(group.pose, None);
            if Ui::button(t!("voice_dictation.send"))
                .at(Vec3::new(clear_x - margin - action.x, y, forward), action)
                .press()
                && let Some(chunk) = group.chunks.first()
            {
                intents.push(BatchIntent::Send(chunk.id));
            }
            if Ui::button(t!("voice_dictation.clear")).at(Vec3::new(clear_x, y, forward), action).press()
                && let Some(chunk) = group.chunks.first()
            {
                intents.push(BatchIntent::Clear(chunk.id));
            }
            Hierarchy::pop();
            Ui::pop_id();

            let count = group.chunks.len();
            for index in 0..count {
                let alone = count == 1;
                let expected = batch_chunk_pose(group.pose, &group.chunks, gap, index);
                let mut window_pose = expected;
                let id = group.chunks[index].id;
                let dragging = group.chunks[index].dragging;

                Ui::window(format!("batch_{id}"))
                    .pose(&mut window_pose)
                    .size(size)
                    .window_type(UiWin::Body)
                    .move_type(UiMove::FaceUser)
                    .begin();
                // The window handle the UI has just made is the last element: it is active while the window is held,
                // and focused while the user is on it. The grab aura is off for the series, so its background says so.
                let held = Ui::get_last_element_active().is_active();
                let focused = Ui::get_last_element_focused().is_active();

                // The text of the chunk: it wraps at the width of the window, and its height is its own, so the window
                // fits it exactly.
                Ui::push_text_style(self.batch_style);
                Ui::text(&group.chunks[index].text).size(Vec2::ZERO).fit(TextFit::Wrap).draw();
                Ui::pop_text_style();
                // A window takes the focus tint only: the red of a merge is for the header of the series that would
                // take it, see `batch_drop_target`.
                end_batch_window(focused.then_some(BATCH_FOCUS_TINT));

                // The bounds the UI gives back measure the window: the series stacks the next one under it, so a
                // window grows and shrinks with its text.
                let measured = Ui::get_layout_last().dimensions.y;
                if measured > 0.0 {
                    group.chunks[index].height = measured;
                }

                // The close button of the window stands beside it and in front of it, in the frame of the window: the
                // window can not catch a pinch meant for it, and it faces the user like the window does.
                let button = self.appearence.scale(BATCH_CLOSE_SIZE);
                Hierarchy::push(window_pose, None);
                if Ui::button(format!("close_{id}"))
                    .image(&self.close_sprite)
                    .at(
                        Vec3::new(
                            size.x / 2.0 + self.appearence.scale(BATCH_CLOSE_GAP),
                            0.0,
                            -self.appearence.scale(BATCH_CLOSE_FORWARD),
                        ),
                        Vec2::new(button, button),
                    )
                    .image_layout(UiBtnLayout::CenterNoText)
                    .press()
                {
                    intents.push(BatchIntent::Close(id));
                }
                Hierarchy::pop();

                // The pose the UI gives back tells what the user did with the window.
                let moved = batch_pose_moved(expected, window_pose);
                if alone {
                    // A series of a single window: dragging the window moves the series itself, header included, and
                    // it may land on another series when the user lets it go.
                    if held {
                        if moved {
                            // The header keeps its place above the window that was dragged.
                            let up = window_pose.orientation * Vec3::UP;
                            group.pose = Pose {
                                position: window_pose.position + up * gap,
                                orientation: window_pose.orientation,
                            };
                        }
                        group.chunks[index].dragging = dragging || moved;
                    } else if dragging {
                        group.chunks[index].dragging = false;
                        intents.push(BatchIntent::Drop { id, pose: window_pose });
                    }
                } else if dragging {
                    // Let go after being torn off: it may land back in a series.
                    if !held {
                        group.chunks[index].dragging = false;
                        intents.push(BatchIntent::Drop { id, pose: window_pose });
                    }
                } else if held && moved {
                    // Grabbed and moved: this window leaves the series and opens one of its own, under the hand.
                    group.chunks[index].dragging = true;
                    intents.push(BatchIntent::Detach { id, from: group.series, pose: window_pose });
                }
            }
        }

        // A series the user moved by its header becomes the current one: the dictation adds its next texts there. It
        // goes last in the list, where `batch_push` appends.
        if let Some(series) = moved_series {
            batch_make_current(&mut groups, series);
        }

        self.batch_groups = groups;
        Ui::pop_id();
        Ui::pop_grab_aura();
        for intent in intents {
            self.apply_batch_intent(intent);
        }
    }

    /// Apply one of the things the user asked of the batch windows.
    fn apply_batch_intent(&mut self, intent: BatchIntent) {
        match intent {
            BatchIntent::Close(id) => {
                batch_remove(&mut self.batch_groups, id);
            }
            BatchIntent::Send(id) => {
                // Sending a series hands it over to the log, as if its texts had been dictated live.
                if let Some(group) = batch_take_group(&mut self.batch_groups, id) {
                    for chunk in group.chunks {
                        let text = chunk.text.trim();
                        if !text.is_empty() {
                            self.push_log(text);
                        }
                    }
                }
            }
            BatchIntent::Clear(id) => {
                batch_take_group(&mut self.batch_groups, id);
            }
            BatchIntent::Detach { id, from, pose } => self.open_series(id, from, pose),
            BatchIntent::Drop { id, pose } => self.drop_chunk(id, pose),
            BatchIntent::Merge(series) => self.merge_group(series),
        }
    }

    /// Merge the series numbered `series` into the one its header was let go over, when there is one: the windows of
    /// the series take the place of the drop, in their own order, and the series they join becomes the current one.
    fn merge_group(&mut self, series: u64) {
        let Some(group) = self.batch_groups.iter().find(|group| group.series == series) else {
            return;
        };
        // The bar under the hand is what the user places: its middle is the point that decides where the windows go.
        let line = Ui::get_line_height();
        let point = batch_header_point(group, line);
        let gap = self.appearence.scale(BATCH_GAP);
        if let Some((index, slot)) =
            batch_drop_slot(&self.batch_groups, point, self.batch_width(), line, gap, &[series])
        {
            let target = self.batch_groups[index].series;
            batch_merge(&mut self.batch_groups, series, target, slot);
        }
    }

    /// Take the chunk `id` out of the series numbered `from`: it opens a series of its own at `pose`, last in the list
    /// so it is the current one - the dictation adds its next texts there - and follows the hand.
    fn open_series(&mut self, id: u64, from: u64, pose: Pose) {
        if let Some(mut chunk) = batch_remove(&mut self.batch_groups, id) {
            chunk.dragging = true;
            // The series it comes from is not a landing place for this drag: letting go over it would undo the tear.
            chunk.from = Some(from);
            let series = self.batch_next_series;
            self.batch_next_series += 1;
            self.batch_groups.push(BatchGroup { series, pose, chunks: vec![chunk], dragging: false });
        }
    }

    /// Put the chunk `id` back among the series: in the series it was dropped on, at the place it was dropped, or in a
    /// series of its own when it lands on none. That series becomes the current one, where the dictation adds the texts
    /// it commits next.
    fn drop_chunk(&mut self, id: u64, pose: Pose) {
        let Some(mut chunk) = batch_remove(&mut self.batch_groups, id) else {
            return;
        };
        chunk.dragging = false;
        let gap = self.appearence.scale(BATCH_GAP);
        let line = Ui::get_line_height();
        // A window is grabbed and dropped anywhere on its body: its middle is the point that decides where it lands.
        let middle = pose.position - (pose.orientation * Vec3::UP) * (chunk.height / 2.0);
        // The series the window was just torn off is not a target: the gesture has to mean something. Past this drag,
        // the window can come back to it.
        let skip = chunk.from.take();
        match batch_drop_slot(&self.batch_groups, middle, self.batch_width(), line, gap, skip.as_slice()) {
            Some((series_index, slot)) => {
                let series = self.batch_groups[series_index].series;
                self.batch_groups[series_index].chunks.insert(slot, chunk);
                // The series the user just dropped the window on is where the next texts go.
                batch_make_current(&mut self.batch_groups, series);
            }
            None => {
                // It landed on no series: it opens one of its own at the pose it was dropped at, last in the list, so
                // it is the current one, and it is the new batch that comes out of the detachment.
                let series = self.batch_next_series;
                self.batch_next_series += 1;
                self.batch_groups.push(BatchGroup { series, pose, chunks: vec![chunk], dragging: false });
            }
        }
    }
}

/// End a window, or the header, of a series, its background tinted with `tint`.
///
/// The callers read the tint from what the UI said right after the window began, when its grab handle was the last
/// element the UI had made ([`Ui::get_last_element_focused`]), and from [`batch_drop_target`] for a header. The tint is
/// pushed only around [`Ui::window_end`]: the content of the window is already drawn, so only its background takes the
/// highlight.
#[cfg(not(feature = "no-event-loop"))]
fn end_batch_window(tint: Option<Color128>) {
    if let Some(tint) = tint {
        Ui::push_tint(tint);
    }
    Ui::window_end();
    if tint.is_some() {
        Ui::pop_tint();
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

/// Tail of `text`, `max_chars` characters at most, with a leading ellipsis when the beginning is dropped.
///
/// The provisional text of a dictation grows as the user speaks, and its box is a few lines high: showing the newest
/// words (the end of the sentence) is more useful than showing the beginning, which the user already knows.
#[cfg(not(feature = "no-event-loop"))]
fn partial_tail(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars || max_chars < 2 {
        return text.to_string();
    }
    // The ellipsis takes one of the characters: the tail keeps `max_chars - 1` of them.
    let tail: String = text.chars().skip(count - (max_chars - 1)).collect();
    format!("…{tail}")
}

#[cfg(all(test, not(feature = "no-event-loop")))]
mod panel_tests {
    use crate::maths::Quat;

    use super::*;

    #[test]
    fn partial_tail_keeps_the_newest_words() {
        // Short enough: the text is returned untouched.
        assert_eq!(partial_tail("bonjour", 10), "bonjour");
        // Exactly the limit: still untouched, no ellipsis.
        assert_eq!(partial_tail("bonjour!!!", 10), "bonjour!!!");
        // Past the limit: the beginning is dropped, an ellipsis marks it, and the length stays within the limit.
        let text = "un deux trois quatre cinq six sept";
        let tail = partial_tail(text, 15);
        assert_eq!(tail.chars().count(), 15);
        let newest: String = tail.chars().skip(1).collect();
        assert!(tail.starts_with('…'));
        assert!(text.ends_with(&newest));
    }

    /// A batch chunk naming itself after its id, with a window 0.1 m high.
    fn chunk(id: u64) -> BatchChunk {
        BatchChunk { id, text: format!("text {id}"), height: 0.1, dragging: false, from: None }
    }

    /// A series of the chunks `chunks`, numbered `series`.
    fn group(series: u64, chunks: Vec<BatchChunk>) -> BatchGroup {
        BatchGroup { series, pose: Pose::IDENTITY, chunks, dragging: false }
    }

    /// The numbers of the series, in the order of the list: the last one is the current one.
    fn series_numbers(groups: &[BatchGroup]) -> Vec<u64> {
        groups.iter().map(|group| group.series).collect()
    }

    /// The ids of the chunks of the series `index`.
    fn ids(groups: &[BatchGroup], index: usize) -> Vec<u64> {
        groups[index].chunks.iter().map(|chunk| chunk.id).collect()
    }

    #[test]
    fn batch_push_stacks_the_chunks_in_the_current_series() {
        let mut groups = Vec::new();
        // No series yet: the first chunk opens one, where the anchor asks.
        assert!(batch_push(&mut groups, Pose::IDENTITY, 1, chunk(1)));
        assert_eq!(ids(&groups, 0), vec![1]);
        assert_eq!(groups[0].series, 1);
        // The current series takes the new chunks, so no new series is opened.
        assert!(!batch_push(&mut groups, Pose::IDENTITY, 2, chunk(2)));
        assert_eq!(groups.len(), 1);
        assert_eq!(ids(&groups, 0), vec![1, 2]);
    }

    #[test]
    fn batch_push_does_not_open_a_series_beside_a_torn_off_window() {
        // A window torn off the dictation opens the current series: the next text stacks in it, not in a series beside.
        let mut groups = vec![group(7, vec![chunk(1)])];
        assert!(!batch_push(&mut groups, Pose::IDENTITY, 8, chunk(2)));
        assert_eq!(series_numbers(&groups), vec![7]);
        assert_eq!(ids(&groups, 0), vec![1, 2]);
    }

    #[test]
    fn batch_make_current_moves_the_series_last() {
        let mut groups = vec![group(1, vec![chunk(1)]), group(2, vec![chunk(2)]), group(3, vec![chunk(3)])];
        // The user moves the first series by its header: it becomes the current one, last in the list.
        batch_make_current(&mut groups, 1);
        assert_eq!(series_numbers(&groups), vec![2, 3, 1]);
        assert_eq!(ids(&groups, 2), vec![1]);
        // The dictation appends the next text to it.
        assert!(!batch_push(&mut groups, Pose::IDENTITY, 4, chunk(4)));
        assert_eq!(ids(&groups, 2), vec![1, 4]);
        // An unknown series changes nothing.
        batch_make_current(&mut groups, 42);
        assert_eq!(series_numbers(&groups), vec![2, 3, 1]);
    }

    #[test]
    fn batch_make_current_keeps_the_windows_of_the_series_in_order() {
        // Dropping a window in a series inserts it at the place it was dropped, and that series becomes the current one,
        // with its windows untouched.
        let mut groups = vec![group(1, vec![chunk(1), chunk(2)]), group(2, vec![chunk(3)])];
        groups[0].chunks.insert(1, chunk(4));
        batch_make_current(&mut groups, 1);
        assert_eq!(series_numbers(&groups), vec![2, 1]);
        assert_eq!(ids(&groups, 1), vec![1, 4, 2]);
    }

    #[test]
    fn batch_remove_drops_the_series_it_empties() {
        let mut groups = vec![group(1, vec![chunk(1), chunk(2)]), group(2, vec![chunk(3)])];
        assert_eq!(batch_remove(&mut groups, 2).map(|chunk| chunk.id), Some(2));
        assert_eq!(groups.len(), 2);
        assert_eq!(ids(&groups, 0), vec![1]);
        assert_eq!(batch_remove(&mut groups, 3).map(|chunk| chunk.id), Some(3));
        assert_eq!(groups.len(), 1);
        assert_eq!(batch_remove(&mut groups, 42).map(|chunk| chunk.id), None);
    }

    #[test]
    fn batch_take_group_removes_the_whole_series() {
        let mut groups = vec![group(1, vec![chunk(1), chunk(2)]), group(2, vec![chunk(3)])];
        let group = batch_take_group(&mut groups, 2).expect("the series of the chunk");
        assert_eq!(group.chunks.len(), 2);
        assert_eq!(group.series, 1);
        assert_eq!(groups.len(), 1);
        assert_eq!(ids(&groups, 0), vec![3]);
        assert!(batch_take_group(&mut groups, 2).is_none());
    }

    #[test]
    fn batch_group_height_adds_the_windows_and_the_gaps() {
        assert_eq!(batch_group_height(&[], 0.01), 0.0);
        // The gap under the header is part of the height of the series: one window of 0.1 m and its gap.
        assert!((batch_group_height(&[chunk(1)], 0.01) - 0.11).abs() < 1e-6);
        assert!((batch_group_height(&[chunk(1), chunk(2)], 0.01) - 0.22).abs() < 1e-6);
    }

    #[test]
    fn batch_chunks_stack_downwards_in_the_frame_of_the_series() {
        let gap = 0.01;
        let top = Pose::new([0.0, 1.5, -0.5], None);
        let chunks = vec![chunk(1), chunk(2)];
        // The first window stands one gap under the header, the next ones under the height of the window above.
        let first = batch_chunk_pose(top, &chunks, gap, 0);
        assert!((first.position.y - (1.5 - 0.01)).abs() < 1e-5);
        let second = batch_chunk_pose(top, &chunks, gap, 1);
        assert!((second.position.y - (1.5 - 0.12)).abs() < 1e-5);
        assert_eq!(second.orientation, top.orientation);
        // The windows stack under the height the UI measured for each of them.
        let mut tall = chunks.clone();
        tall[0].height = 0.3;
        let below = batch_chunk_pose(top, &tall, gap, 1);
        assert!((below.position.y - (1.5 - 0.32)).abs() < 1e-5);
        // The stack follows the frame of the series, not the axes of the world.
        let turned = Pose::new([0.0, 0.0, 0.0], Some(Quat::from_angles(0.0, 0.0, 90.0)));
        let below = batch_chunk_pose(turned, &chunks, gap, 1);
        assert!((below.position.length() - 0.12).abs() < 1e-5);
        assert!(below.position.x.abs() > 0.1);
    }

    #[test]
    fn batch_drop_slot_finds_the_series_and_the_place() {
        let width = 0.2;
        let line = 0.03;
        let gap = 0.01;
        let groups = vec![
            BatchGroup { pose: Pose::new([0.0, 1.0, 0.0], None), ..group(1, vec![chunk(1), chunk(2)]) },
            BatchGroup { pose: Pose::new([0.5, 1.0, 0.0], None), ..group(2, vec![chunk(3)]) },
        ];
        let slot = |point: Vec3, skip: &[u64]| batch_drop_slot(&groups, point, width, line, gap, skip);
        // Dropped on the upper half of the second window of the first series: it inserts between the two.
        assert_eq!(slot(Vec3::new(0.02, 1.0 - 0.15, 0.0), &[]), Some((0, 1)));
        // Dropped on the title bar of that series: it inserts at its top.
        assert_eq!(slot(Vec3::new(0.0, 1.02, 0.0), &[]), Some((0, 0)));
        // Dropped at the bottom of that series: it inserts at the end.
        assert_eq!(slot(Vec3::new(0.0, 0.77, 0.0), &[]), Some((0, 2)));
        // Dropped on the upper half of the second series: it inserts there, at the top.
        assert_eq!(slot(Vec3::new(0.5, 0.97, 0.0), &[]), Some((1, 0)));
        // The series the window was torn off is not a target, so the tear holds.
        assert_eq!(slot(Vec3::new(0.02, 1.0 - 0.15, 0.0), &[1]), None);
        // Several series can be left out at once.
        assert_eq!(slot(Vec3::new(0.02, 1.0 - 0.15, 0.0), &[1, 2]), None);
        // Far from every series: it lands on none.
        assert_eq!(slot(Vec3::new(2.0, 1.0, 0.0), &[]), None);
    }

    #[test]
    fn batch_drop_target_names_the_series_a_dragged_window_would_join() {
        let width = 0.2;
        let line = 0.03;
        let gap = 0.01;
        // Nothing is held by the user: no series is singled out.
        assert_eq!(batch_drop_target(&[group(1, vec![chunk(1)])], width, line, gap), None);
        // A window of the second series is held right over the first one: letting go merges it there.
        let mut held = chunk(2);
        held.dragging = true;
        let groups = vec![
            BatchGroup { pose: Pose::new([0.0, 1.0, 0.0], None), ..group(1, vec![chunk(1)]) },
            BatchGroup { pose: Pose::new([0.0, 1.0, 0.0], None), ..group(2, vec![held.clone()]) },
        ];
        assert_eq!(batch_drop_target(&groups, width, line, gap), Some(1));
        // Held away from every series: it would open one of its own.
        let groups = vec![
            BatchGroup { pose: Pose::new([0.0, 1.0, 0.0], None), ..group(1, vec![chunk(1)]) },
            BatchGroup { pose: Pose::new([2.0, 1.0, 0.0], None), ..group(2, vec![held]) },
        ];
        assert_eq!(batch_drop_target(&groups, width, line, gap), None);
    }

    #[test]
    fn batch_merge_inserts_the_windows_and_makes_the_target_current() {
        let mut groups = vec![group(1, vec![chunk(1), chunk(2)]), group(2, vec![chunk(3)]), group(3, vec![chunk(4)])];
        // The series 3 is dropped on the upper half of the second window of the series 1: its window takes the place.
        assert!(batch_merge(&mut groups, 3, 1, 1));
        assert_eq!(series_numbers(&groups), vec![2, 1]);
        assert_eq!(ids(&groups, 1), vec![1, 4, 2]);
        // The series it joined is the current one: the dictation appends the next text there.
        assert!(!batch_push(&mut groups, Pose::IDENTITY, 5, chunk(5)));
        assert_eq!(ids(&groups, 1), vec![1, 4, 2, 5]);
        // A series cannot merge into itself, nor into one that does not exist.
        assert!(!batch_merge(&mut groups, 1, 1, 0));
        assert!(!batch_merge(&mut groups, 1, 42, 0));
        assert!(!batch_merge(&mut groups, 42, 1, 0));
        assert_eq!(ids(&groups, 1), vec![1, 4, 2, 5]);
    }

    #[test]
    fn batch_drop_target_also_sees_a_series_held_by_its_header() {
        let width = 0.2;
        let line = 0.03;
        let gap = 0.01;
        // The second series is held by its header, right over the first one: letting go merges the two.
        let groups = vec![
            BatchGroup { pose: Pose::new([0.0, 1.0, 0.0], None), ..group(1, vec![chunk(1)]) },
            BatchGroup { pose: Pose::new([0.0, 1.0, 0.0], None), dragging: true, ..group(2, vec![chunk(2), chunk(3)]) },
        ];
        assert_eq!(batch_drop_target(&groups, width, line, gap), Some(1));
        // Held away from every series: it would stay where it is.
        let groups = vec![
            BatchGroup { pose: Pose::new([0.0, 1.0, 0.0], None), ..group(1, vec![chunk(1)]) },
            BatchGroup { pose: Pose::new([2.0, 1.0, 0.0], None), dragging: true, ..group(2, vec![chunk(2)]) },
        ];
        assert_eq!(batch_drop_target(&groups, width, line, gap), None);
    }

    #[test]
    fn batch_pose_moved_sees_moves_and_turns() {
        let pose = Pose::new([0.0, 1.0, -0.5], Some(Quat::from_angles(0.0, 20.0, 0.0)));
        assert!(!batch_pose_moved(pose, pose));
        assert!(batch_pose_moved(pose, Pose::new([0.0, 1.02, -0.5], Some(pose.orientation))));
        assert!(batch_pose_moved(pose, Pose::new(pose.position, Some(Quat::from_angles(0.0, 40.0, 0.0)))));
    }

    /// The texts of the window come from the `voice_dictation` section of the catalogues: every locale has them, a
    /// translated locale really changes them, and the model folder placeholder is substituted.
    #[test]
    fn voice_dictation_texts_are_translated() {
        for locale in crate::tools::locales::available_locales() {
            let title = rust_i18n::t!("voice_dictation.title", locale = locale.as_str());
            assert!(!title.is_empty(), "no title in the locale {locale}");
            assert_ne!(title, "voice_dictation.title", "no title key in the locale {locale}");
        }
        assert_eq!(rust_i18n::t!("voice_dictation.title", locale = "fr"), "Dictée vocale");
        assert_eq!(rust_i18n::t!("voice_dictation.send", locale = "fr"), "Envoyer");
        assert_eq!(
            rust_i18n::t!("voice_dictation.status_no_streaming_model", locale = "fr", dir = "/tmp"),
            "Aucun modèle de flux dans '/tmp' : il faut encoder/decoder/joiner .onnx et tokens.txt."
        );
    }
}
