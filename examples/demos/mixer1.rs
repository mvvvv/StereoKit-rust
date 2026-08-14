// SPDX-License-Identifier: MIT
//! # Mixer1 - An ultra-simplified VR audio mixer
//!
//! Stripped-down version of [`Mixer1`]: only the essentials.
//!
//! - **Record a track**: a single REC/STOP button. The microphone is "warmed up" in a thread (to avoid freezing the
//!   main loop) and then samples are captured directly. On stop, the voice becomes a new track.
//! - **Adjust volume**: a master volume slider (`Audio::volume`, truly global) + a per-track volume slider.
//! - **Tune each track**: all [`SoundPlay`] parameters are editable per track — pitch, spread, cutoff, delay, bus and
//!   flags (Loop / HeadLocked / PropagationDelay).
//! - **Launch all tracks at once**: a PLAY ALL / STOP ALL button starts or stops all tracks simultaneously.
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

use stereokit_rust::{
    maths::{Pose, Quat, Vec2, Vec3},
    prelude::*,
    sound::{
        Audio, AudioEnvironment, Microphone, Sound, SoundBus, SoundChannels, SoundFlags, SoundInst, SoundPlay,
        SoundSampleRate,
    },
    sprite::Sprite,
    ui::{Ui, UiBtnLayout, UiMove, UiPad},
    util::{
        Color128,
        named_colors::{CYAN, GREEN, RED},
    },
};

/// Capture sample rate (fixed, simplified: 48 kHz native).
const MIC_SAMPLE_RATE: SoundSampleRate = SoundSampleRate::Standard;

/// Frequency in Hz of a [`SoundSampleRate`] (for display only).
fn sample_rate_hz(rate: SoundSampleRate) -> u32 {
    match rate {
        SoundSampleRate::Telephony => 8_000,
        SoundSampleRate::Speech => 16_000,
        SoundSampleRate::Broadcast => 32_000,
        SoundSampleRate::Cd => 44_100,
        SoundSampleRate::Standard | SoundSampleRate::Default => 48_000,
        SoundSampleRate::Studio => 96_000,
        SoundSampleRate::Ultra => 192_000,
    }
}
/// StereoKit's 4 audio buses, in `SoundBus` enum order.
const SOUND_BUSES: [SoundBus; 4] = [SoundBus::Sfx, SoundBus::Music, SoundBus::Ui, SoundBus::Voice];
const BUS_NAMES: [&str; 4] = ["Sfx", "Music", "UI", "Voice"];

/// Acoustic environment presets cyclable via a button.
const ENV_PRESETS: [(&str, AudioEnvironment); 6] = [
    ("Off", AudioEnvironment::OFF),
    ("Room", AudioEnvironment::ROOM),
    ("Hall", AudioEnvironment::HALL),
    ("Cave", AudioEnvironment::CAVE),
    ("Forest", AudioEnvironment::FOREST),
    ("Field", AudioEnvironment::FIELD),
];

/// Returns the index of a [`SoundBus`] in [`SOUND_BUSES`] (to cycle a track's bus via a button). Falls back to `0`
/// (Sfx) if not found.
fn bus_index(bus: SoundBus) -> usize {
    SOUND_BUSES.iter().position(|&b| b == bus).unwrap_or(0)
}

/// An audio track: points to a [`Sound`], holds its volume and playback state.
/// This is the minimal version of the `Track` from [`Mixer1`].
struct Track {
    id: u32,
    name: String,
    sound: Option<Sound>,
    inst: Option<SoundInst>,
    /// Track volume (0..1). Multiplied with the master volume at playback.
    volume: f32,
    /// Does the user want the track to play?
    playing: bool,
    /// Clip duration in seconds (display).
    duration: f32,
    /// Total number of samples (mono frames) written to the stream. Used to
    /// detect the end of playback (ring buffers stay `is_playing == true` even
    /// after the data has run out).
    sample_count: u64,
    /// Position / orientation of the track window. Also serves as the 3D sound
    /// emission position (spatial audio): moving the window moves the sound
    /// source.
    pose: Pose,

    // ---- SoundPlay parameters (editable per track) -----------------------
    /// Playback rate multiplier (0.25-4). 1 = normal speed. Applied live via
    /// [`SoundInst::pitch`].
    pitch: f32,
    /// Apparent size of the source (0-1). 0 = point source. Applied live via
    /// [`SoundInst::spread`].
    spread: f32,
    /// Low-pass filter cutoff in Hz. 0 = automatic distance model. Applied live
    /// via [`SoundInst::set_cutoff`].
    cutoff: f32,
    /// Delay in seconds before playback starts. Applied only when playback
    /// (re)starts, via [`SoundPlay::delay`].
    delay: f32,
    /// Audio bus of the track. Applied when playback (re)starts, via
    /// [`SoundPlay::bus`].
    bus: SoundBus,
    /// Loop the track? Streams ignore the native `Loop` flag, so we handle it
    /// manually (restart at the end of the clip).
    loop_track: bool,
    /// Head-locked sound (ignores the 3D position). On restart.
    head_locked: bool,
    /// Adds a propagation delay (distance / 343 m·s⁻¹) at start.
    propagation_delay: bool,
}

/// The main stepper for the simplified VR mixer.
#[derive(IStepper)]
pub struct Mixer1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    shutdown_completed: bool,

    // ---- Recording ---------------------------------------------------------
    /// `true` while a REC session is in progress (effective capture).
    recording: bool,
    /// Is the mic device started ("warm")?
    mic_warm: bool,
    /// Handle of the thread that runs `Microphone::start` off the main loop
    /// (device initialization can block for several seconds).
    mic_thread: Option<JoinHandle<()>>,
    /// Flag set by `mic_thread` when `Microphone::start` has completed.
    mic_started: Arc<AtomicBool>,
    /// Samples accumulated during recording (mono, 48 kHz).
    rec_buffer: Vec<f32>,
    /// The sound stream returned by the mic (for capture).
    mic_stream: Option<Sound>,

    // ---- Tracks & console --------------------------------------------------
    /// All tracks in the mixer.
    tracks: Vec<Track>,
    /// Counter to give each track a unique identifier.
    next_track_id: u32,
    /// Master mix volume (0..1). Applied via `Audio::volume`.
    master_volume: f32,
    /// Audio bus volumes (Sfx, Music, Ui, Voice). Applied via
    /// `Audio::bus_volume`.
    bus_volumes: [f32; 4],
    /// Index of the current acoustic environment preset (0=Off, 1=Room,
    /// 2=Hall, 3=Cave, 4=Forest, 5=Field).
    env_preset: usize,
    /// Pose of the main console in front of the user.
    console_pose: Pose,

    // ---- Sprites -----------------------------------------------------------
    /// "Idle playback" icon (right arrow) for PLAY buttons and Selection buttons.
    sprite_play: Sprite,
    /// "Active" icon (toggle on): track playing or REC active.
    sprite_toggle_on: Sprite,
    /// "Inactive" icon (toggle off) for the REC button at rest.
    sprite_rec_off: Sprite,
}

unsafe impl Send for Mixer1 {}

impl Default for Mixer1 {
    fn default() -> Self {
        Self {
            id: "Mixer1".to_string(),
            sk_info: None,
            shutdown_completed: false,

            recording: false,
            mic_warm: false,
            mic_thread: None,
            mic_started: Arc::new(AtomicBool::new(false)),
            rec_buffer: Vec::new(),
            mic_stream: None,

            tracks: Vec::new(),
            next_track_id: 1,
            master_volume: 1.0,
            bus_volumes: [1.0; 4],
            env_preset: 0,
            // Window placed at eye level, facing the user.
            console_pose: Pose::new(Vec3::new(0.0, 1.5, -0.6), Some(Quat::look_dir(Vec3::Z))),

            sprite_play: Sprite::arrow_right(),
            sprite_toggle_on: Sprite::toggle_on(),
            sprite_rec_off: Sprite::toggle_off(),
        }
    }
}

impl Mixer1 {
    /// Called once at stepper initialization. Returns `false` to cancel. Here we have nothing special to prepare.
    fn start(&mut self) -> bool {
        true
    }

    /// Stops the mic when the stepper closes.
    fn close(&mut self, triggering: bool) -> bool {
        if triggering {
            // Wait for the mic start thread to finish if it is still running.
            if let Some(handle) = self.mic_thread.take() {
                let _ = handle.join();
            }
            if self.mic_warm {
                Microphone::stop();
                self.mic_warm = false;
            }
            self.shutdown_completed = true;
        }
        self.shutdown_completed
    }

    /// Drains the mic's ring buffer every frame. If we are recording, samples go into `rec_buffer`; otherwise they are
    /// simply consumed (discarded) to empty the ring buffer. Leading zero samples are stripped on the fly: they
    /// represent the warm-up latency before the first real sound.
    fn drain_mic(&mut self) {
        if !self.mic_warm {
            return;
        }
        let Some(mic) = &self.mic_stream else { return };
        let unread = mic.get_unread_samples() as usize;
        if unread == 0 {
            return;
        }
        let mut chunk = vec![0.0f32; unread];
        let read = mic.read_samples(&mut chunk, Some(unread as u64)) as usize;
        if self.recording {
            self.rec_buffer.extend_from_slice(&chunk[..read]);
            // Strip leading zero samples (warm-up silence).
            let zeros = self.rec_buffer.iter().take_while(|&&s| s == 0.0).count();
            if zeros > 0 {
                self.rec_buffer.drain(0..zeros);
            }
        }
    }

    /// Starts recording. `Microphone::start` can block the main loop during device initialization, so we run it in a
    /// dedicated thread. Capture becomes effective as soon as the mic is warm.
    fn start_recording(&mut self) {
        let started_flag = self.mic_started.clone();
        started_flag.store(false, Ordering::SeqCst);

        // Start the mic in a thread to avoid freezing the main loop.
        self.mic_thread = Some(thread::spawn(move || {
            let ok = Microphone::start(None, Some(MIC_SAMPLE_RATE));
            started_flag.store(ok, Ordering::SeqCst);
        }));

        self.recording = true;
        self.rec_buffer.clear();
    }

    /// Stops capture and turns the recorded voice into a new track.
    fn stop_recording(&mut self) {
        // One last drain so we don't lose the final samples.
        self.drain_mic();
        // Wait for the mic start thread to finish if it is still running.
        if let Some(handle) = self.mic_thread.take() {
            let _ = handle.join();
        }
        if self.mic_warm {
            Microphone::stop();
            self.mic_warm = false;
        }
        self.mic_stream = None;
        self.mic_started.store(false, Ordering::SeqCst);
        self.recording = false;

        // Strip leading zero samples (safety, `drain_mic` already does it).
        let leading_zeros = self.rec_buffer.iter().take_while(|&&s| s == 0.0).count();
        if leading_zeros > 0 {
            self.rec_buffer.drain(0..leading_zeros);
        }

        if self.rec_buffer.is_empty() {
            return;
        }
        let capture_hz = sample_rate_hz(MIC_SAMPLE_RATE) as f32;
        let sample_count = self.rec_buffer.len() as u64;
        let duration = sample_count as f32 / capture_hz;
        // Create a stream at the native capture rate. The ring buffer must be
        // much larger than the data so playback starts at the beginning.
        let buffer_duration = duration * 2.0 + 0.5;
        let sound = Sound::create_stream_with(buffer_duration, SoundChannels::Mono, MIC_SAMPLE_RATE)
            .inspect(|s| {
                s.write_samples(&self.rec_buffer, None);
            })
            .ok();
        self.rec_buffer.clear();

        if let Some(sound) = sound {
            self.add_track(format!("Voice {}", self.tracks.len() + 1), sound, duration, sample_count);
            Log::info(format!("Mixer1: track recorded ({:.1}s @ {} Hz)", duration, capture_hz));
        }
    }

    /// Adds a track to the list. The window is placed on an arc in front of the user; this position also serves as the
    /// 3D sound emission point.
    fn add_track(&mut self, name: String, sound: Sound, duration: f32, sample_count: u64) {
        let id = self.next_track_id;
        self.next_track_id += 1;
        // Windows are spread along an arc in front of the user, at slightly
        // different heights => a real 3D layout.
        let t = self.tracks.len() as f32;
        let angle: f32 = -0.7 + t * 0.28;
        let pos = Vec3::new(angle.sin() * 0.85, 1.15 + (t * 0.6).sin() * 0.1, -0.85 + angle.cos() * 0.15);
        let pose = Pose::new(pos, Some(Quat::look_dir(Vec3::Z)));
        self.tracks.push(Track {
            id,
            name,
            sound: Some(sound),
            inst: None,
            volume: 1.0,
            playing: false,
            duration,
            sample_count,
            pose,
            // Default SoundPlay parameters: normal speed, point source, no
            // filter, no delay, Voice bus, no loop.
            pitch: 1.0,
            spread: 0.0,
            cutoff: 0.0,
            delay: 0.0,
            bus: SoundBus::Voice,
            loop_track: false,
            head_locked: false,
            propagation_delay: false,
        });
    }

    /// Syncs the playing sound state (`inst`) with the track's intent. Effective volume = track volume × master
    /// volume. The sound is emitted from the 3D position of the track window (spatial audio). The "live" parameters
    /// (pitch, spread, cutoff) are pushed to the instance every frame; the startup parameters (bus, delay, flags) are
    /// applied via [`SoundPlay`] when playback starts.
    fn update_track_audio(track: &mut Track, master: f32) {
        let effective = track.volume * master;
        let want_play = track.playing && track.sound.is_some();

        match (want_play, &mut track.inst) {
            (true, _inst @ None) => {
                // The track should play but has no active voice: start it from its window's 3D position with all the
                // track's `SoundPlay` settings.
                let mut flags = SoundFlags::None;
                if track.loop_track {
                    flags |= SoundFlags::Loop;
                }
                if track.head_locked {
                    flags |= SoundFlags::HeadLocked;
                }
                if track.propagation_delay {
                    flags |= SoundFlags::PropagationDelay;
                }
                let settings = SoundPlay {
                    volume: effective,
                    pitch: track.pitch,
                    spread: track.spread,
                    delay: track.delay,
                    cutoff: track.cutoff,
                    bus: track.bus,
                    flags,
                    ..Default::default()
                };
                let sound = track.sound.as_ref().unwrap();
                track.inst = Some(sound.play_with(track.pose.position, &settings));
            }
            (true, Some(inst)) => {
                // Live parameters: volume, 3D position, pitch, spread, cutoff.
                inst.volume(effective);
                inst.position(track.pose.position);
                inst.pitch(track.pitch);
                inst.spread(track.spread);
                inst.set_cutoff(track.cutoff);
                // Ring buffers stay `is_playing == true` even after the data has
                // run out: we detect the end via the cursor.
                if track.sample_count > 0 && inst.get_cursor() >= track.sample_count {
                    if track.loop_track {
                        // Streams ignore the native `Loop` flag: manually restart playback from the beginning.
                        inst.stop();
                        track.inst = None;
                        // `playing` stays `true`: the voice will restart next frame via the startup branch above.
                    } else {
                        // End of clip: stop.
                        inst.stop();
                        track.inst = None;
                        track.playing = false;
                    }
                }
            }
            (false, Some(inst)) => {
                // The user cut the track: stop the voice.
                inst.stop();
                track.inst = None;
            }
            (false, None) => {}
        }
    }

    /// Called every frame: here we check for any events.
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called every frame in the main thread: draws the entire scene.
    fn draw(&mut self, _token: &MainThreadToken) {
        // Retrieve the mic stream once the start thread has completed `Microphone::start`. This is where we "wait for
        // the first samples to arrive": until the flag is raised, the mic is warming up.
        if self.recording && self.mic_stream.is_none() && self.mic_started.load(Ordering::SeqCst) {
            self.mic_stream = Microphone::sound().ok();
            self.mic_warm = self.mic_stream.is_some();
        }

        self.drain_mic();

        // Apply StereoKit's global audio parameters.
        Audio::volume(self.master_volume);
        for (i, &bus) in SOUND_BUSES.iter().enumerate() {
            Audio::bus_volume(bus, self.bus_volumes[i]);
        }
        Audio::environment(ENV_PRESETS[self.env_preset].1);

        // --- Main console ----------------------------------------------------
        Ui::window("Mixer1")
            .pose(&mut self.console_pose)
            .size(Vec2::new(0.42, 0.0))
            .move_type(UiMove::FaceUser)
            .begin();

        // ============================================================
        // Recording (single REC/STOP button)
        // ============================================================
        Ui::panel_begin(Some(UiPad::Inside));
        if self.recording {
            Ui::push_tint(Color128::from(RED).to_gamma());
        } else {
            Ui::push_tint(Color128::from(GREEN).to_gamma());
        }
        // toggle_on icon when recording, toggle_off otherwise.
        let rec_img = if self.recording { &self.sprite_toggle_on } else { &self.sprite_rec_off };
        if Ui::button(if self.recording { "STOP" } else { "REC" })
            .image(rec_img)
            .size(Vec2::new(0.16, 0.10))
            .image_layout(UiBtnLayout::Center)
            .press()
        {
            if self.recording {
                self.stop_recording();
            } else {
                self.start_recording();
            }
        }
        Ui::pop_tint();

        Ui::same_line();
        // Status info (mic warm-up, duration, or help).
        if self.recording {
            if !self.mic_warm {
                Ui::label("Warming up mic...").draw();
            } else {
                let secs = self.rec_buffer.len() as f32 / sample_rate_hz(MIC_SAMPLE_RATE) as f32;
                Ui::label(format!("REC {:.1}s", secs)).draw();
            }
        } else {
            Ui::label("Press REC, speak, then STOP.").draw();
        }
        Ui::panel_end();

        Ui::hseparator();

        // ============================================================
        // Global playback (PLAY ALL / STOP ALL) + clear
        // ============================================================
        Ui::panel_begin(Some(UiPad::Inside));
        let any_playing = self.tracks.iter().any(|t| t.playing);
        let play_img = if any_playing { &self.sprite_toggle_on } else { &self.sprite_play };
        if Ui::button(if any_playing { "STOP ALL" } else { "PLAY ALL" })
            .image(play_img)
            .size(Vec2::new(0.16, 0.08))
            .image_layout(UiBtnLayout::Center)
            .press()
        {
            if any_playing {
                for track in &mut self.tracks {
                    if let Some(inst) = track.inst.take() {
                        inst.stop();
                    }
                    track.playing = false;
                }
            } else {
                // Start all tracks at the same time.
                for track in &mut self.tracks {
                    if let Some(inst) = track.inst.take() {
                        inst.stop();
                    }
                    track.playing = true;
                }
            }
        }
        Ui::same_line();
        if Ui::button("Clear all").size(Vec2::new(0.12, 0.08)).press() {
            for track in &mut self.tracks {
                if let Some(inst) = track.inst.take() {
                    inst.stop();
                }
            }
            self.tracks.clear();
        }

        // ============================================================
        // Master volume
        // ============================================================
        Ui::label(format!("Master volume: {:.0}%", self.master_volume * 100.0)).draw();
        Ui::same_line();
        if let Some(v) = Ui::hslider("master", &mut self.master_volume, 0.0, 10.0).step(0.01).interact() {
            self.master_volume = v;
        }

        Ui::panel_end();

        // ============================================================
        // Audio settings (bus volumes, environment, level)
        // ============================================================
        Ui::hseparator();
        Ui::panel_begin(Some(UiPad::Inside));

        // Volumes for the 4 audio buses (Sfx, Music, Ui, Voice).
        for (i, &bus) in SOUND_BUSES.iter().enumerate() {
            Ui::label(format!("{}: {:.0}%", BUS_NAMES[i], self.bus_volumes[i] * 100.0))
                .use_padding(false)
                .draw();
            Ui::same_line();
            if let Some(v) = Ui::hslider(BUS_NAMES[i], &mut self.bus_volumes[i], 0.0, 1.0).step(0.01).interact() {
                Audio::bus_volume(bus, v);
            }
            Ui::next_line();
        }

        // Cyclic button for the acoustic environment preset.
        Ui::label("Environment:").use_padding(false).draw();
        Ui::same_line();
        let (name, env) = ENV_PRESETS[self.env_preset];
        if Ui::button(name).image(&self.sprite_play).size(Vec2::new(0.12, 0.035)).press() {
            self.env_preset = (self.env_preset + 1) % ENV_PRESETS.len();
            Audio::environment(env);
        }
        Ui::next_line();

        // Output level (dBFS): read-only display.
        let db = Audio::get_output_decibels();
        Ui::label(format!("Output: {:.0} dBFS", db)).use_padding(false).draw();

        Ui::panel_end();

        Ui::window_end();

        // --- Tracks: one movable 3D window each -------------------------------
        // The window position = sound emission position (spatial audio).
        let mut to_remove: Vec<usize> = Vec::new();
        let mut to_duplicate: Vec<usize> = Vec::new();
        for (i, track) in self.tracks.iter_mut().enumerate() {
            // Sync audio (start/stop, volume, 3D position).
            Self::update_track_audio(track, self.master_volume);

            Ui::push_id_int(track.id as i32);
            Ui::window(track.name.as_str())
                .pose(&mut track.pose)
                .size(Vec2::new(0.33, 0.0))
                .move_type(UiMove::FaceUser)
                .begin();

            Ui::panel_begin(Some(UiPad::Inside));

            // Per-track PLAY/STOP button.
            let track_color: Color128 = CYAN.into();
            Ui::push_tint(track_color.to_gamma());
            let play_img = if track.playing { &self.sprite_toggle_on } else { &self.sprite_play };
            if Ui::button(if track.playing { "stop" } else { "play" })
                .image(play_img)
                .image_layout(UiBtnLayout::CenterNoText)
                .size(Vec2::new(0.04, 0.04))
                .press()
            {
                track.playing = !track.playing;
            }
            Ui::pop_tint();
            Ui::same_line();

            // Editable name + duration.
            Ui::input("name", &mut track.name).size(Vec2::new(0.17, 0.035)).edit();
            Ui::same_line();
            Ui::label(format!("{:.1}s", track.duration)).use_padding(false).draw();

            Ui::next_line();

            // Track volume slider.
            Ui::label(format!("Vol: {:.0}%", track.volume * 100.0)).use_padding(false).draw();
            Ui::same_line();
            if let Some(v) = Ui::hslider("volume", &mut track.volume, 0.0, 10.0).step(0.01).interact() {
                track.volume = v;
            }
            Ui::same_line();
            if Ui::button("Duplicate").size(Vec2::new(0.08, 0.035)).press() {
                to_duplicate.push(i);
            }
            Ui::same_line();
            if Ui::button("Delete").size(Vec2::new(0.06, 0.035)).press() {
                to_remove.push(i);
            }

            // ---- Track SoundPlay parameters --------------------------------
            // Pitch (playback rate). Live via SoundInst::pitch.
            Ui::next_line();
            Ui::label(format!("Pitch: {:.2}x", track.pitch)).use_padding(false).draw();
            Ui::same_line();
            Ui::hslider("pitch", &mut track.pitch, 0.25, 4.0).step(0.01).interact();

            // Spread (spatial size of the source). Live via SoundInst::spread.
            Ui::next_line();
            Ui::label(format!("Spread: {:.0}%", track.spread * 100.0)).use_padding(false).draw();
            Ui::same_line();
            Ui::hslider("spread", &mut track.spread, 0.0, 1.0).step(0.01).interact();

            // Low-pass filter cutoff (0 = automatic). Live via set_cutoff.
            Ui::next_line();
            let cutoff_lbl = if track.cutoff <= 0.0 { "Auto".to_string() } else { format!("{:.0} Hz", track.cutoff) };
            Ui::label(format!("Cutoff: {}", cutoff_lbl)).use_padding(false).draw();
            Ui::same_line();
            Ui::hslider("cutoff", &mut track.cutoff, 0.0, 20_000.0).step(50.0).interact();

            // Start delay (seconds). Only on (re)start.
            Ui::next_line();
            Ui::label(format!("Delay: {:.1}s", track.delay)).use_padding(false).draw();
            Ui::same_line();
            Ui::hslider("delay", &mut track.delay, 0.0, 5.0).step(0.1).interact();

            // Audio bus (cycle) + flags — on playback (re)start.
            Ui::label("Bus:").use_padding(false).draw();
            Ui::same_line();
            if Ui::button(BUS_NAMES[bus_index(track.bus)])
                .image(&self.sprite_play)
                .size(Vec2::new(0.10, 0.03))
                .press()
            {
                track.bus = SOUND_BUSES[(bus_index(track.bus) + 1) % SOUND_BUSES.len()];
            }
            Ui::same_line();
            Ui::toggle("Loop", &mut track.loop_track).size(Vec2::new(0.07, 0.03)).interact();

            Ui::toggle("Head locked", &mut track.head_locked).size(Vec2::new(0.13, 0.03)).interact();
            Ui::same_line();
            Ui::toggle("Propagation", &mut track.propagation_delay).size(Vec2::new(0.13, 0.03)).interact();

            Ui::panel_end();
            Ui::window_end();
            Ui::pop_id();
        }

        // Duplicate marked tracks (before removals so indices collected during
        // the loop remain valid for the source).
        for &i in to_duplicate.iter() {
            self.duplicate_track(i);
        }

        // Remove marked tracks (from the end backward).
        for &i in to_remove.iter().rev() {
            if let Some(inst) = self.tracks[i].inst.take() {
                inst.stop();
            }
            self.tracks.remove(i);
        }
    }

    /// Duplicates a track: clones the [`Sound`] via [`Sound::clone_ref`] (a new
    /// reference to the same audio asset) and copies all `SoundPlay` parameters
    /// from the source. The copy starts stopped and is positioned after the
    /// existing tracks on the arc.
    fn duplicate_track(&mut self, index: usize) {
        // Extract all values from the source before borrowing `self` mutably
        // (for the `push`). Copy fields are copied, the `Sound` is cloned via
        // `clone_ref`, the name is suffixed.
        let src = match self.tracks.get(index) {
            Some(t) => t,
            None => return,
        };
        let sound = match &src.sound {
            Some(s) => s.clone_ref(),
            None => return,
        };
        let name = format!("{} copy", src.name);
        let volume = src.volume;
        let duration = src.duration;
        let sample_count = src.sample_count;
        let pitch = src.pitch;
        let spread = src.spread;
        let cutoff = src.cutoff;
        let delay = src.delay;
        let bus = src.bus;
        let loop_track = src.loop_track;
        let head_locked = src.head_locked;
        let propagation_delay = src.propagation_delay;

        let id = self.next_track_id;
        self.next_track_id += 1;
        let t = self.tracks.len() as f32;
        let angle: f32 = -0.7 + t * 0.28;
        let pos = Vec3::new(angle.sin() * 0.85, 1.15 + (t * 0.6).sin() * 0.1, -0.85 + angle.cos() * 0.15);
        let pose = Pose::new(pos, Some(Quat::look_dir(Vec3::Z)));
        self.tracks.push(Track {
            id,
            name,
            sound: Some(sound),
            inst: None,
            volume,
            playing: false,
            duration,
            sample_count,
            pose,
            pitch,
            spread,
            cutoff,
            delay,
            bus,
            loop_track,
            head_locked,
            propagation_delay,
        });
    }
}
