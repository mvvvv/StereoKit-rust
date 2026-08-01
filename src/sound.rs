use crate::{
    StereoKitError,
    maths::{Bool32T, Pose, Vec3},
    system::{AssetState, IAsset},
};
use std::{
    ffi::{CStr, CString, c_char, c_void},
    path::Path,
    ptr::{NonNull, null},
};

/// A perceptual description of the acoustic space sounds play in - an environment rather than a literal room, so it
/// covers halls through forests. Spatial sounds feed a shared reverb whose level stays constant with distance, so the
/// direct-to-reverb balance naturally carries how far away a sound is. A wet of 0 disables the system entirely at zero
/// cost, and a zeroed struct is the off state. Language bindings provide preset values for common spaces as starting
/// points.
/// <https://stereokit.net/Pages/StereoKit/AudioEnvironment.html>
///
/// see also [`Audio`]
#[repr(C)]
pub struct AudioEnvironment {
    /// Reverb level, 0-1. 0 turns environmental acoustics off completely, and is the default.
    pub wet: f32,
    /// Decay time in seconds - how long the tail takes to fall 60dB at mid frequencies. Rooms are ~0.4s, cathedrals a
    /// few seconds. Clamped to 0.05-10.
    pub decay: f32,
    /// 0-1, extra high frequency decay. Soft or leafy spaces are high, tiled rooms are low.
    pub damp: f32,
    /// Size of the space in meters, clamped to 2-40. Drives the spacing of the echoes that build the tail. Changing
    /// this restarts the tail, where the other fields all glide smoothly.
    pub size: f32,
    /// 0-1, how quickly discrete echoes blur into a dense wash. Scattered spaces like forests are high, bare rooms
    /// lower.
    pub scatter: f32,
    /// 0-1, level of the distinct early reflections off the space's surfaces - the first bounces that glue a sound to
    /// the room. The ground bounce keeps a minimum presence; walls and ceiling scale fully with this, so outdoor
    /// spaces sit near 0.
    pub reflect: f32,
}
impl Default for AudioEnvironment {
    fn default() -> Self {
        Self::OFF
    }
}
impl AudioEnvironment {
    /// No environmental acoustics at all, sounds play dry. This is the default, and costs nothing - the right choice
    /// for AR, where synthetic reverb would fight the real room's acoustics.
    pub const OFF: Self = AudioEnvironment { wet: 0.0, decay: 0.4, damp: 0.55, size: 7.0, scatter: 0.6, reflect: 0.55 };

    /// A small furnished room: a short, balanced tail.
    pub const ROOM: Self =
        AudioEnvironment { wet: 0.17, decay: 0.4, damp: 0.55, size: 7.0, scatter: 0.6, reflect: 0.55 };

    /// A large hall: a long, bright, spacious tail.
    pub const HALL: Self =
        AudioEnvironment { wet: 0.22, decay: 1.4, damp: 0.45, size: 16.0, scatter: 0.7, reflect: 0.55 };

    /// A cavern: a very long, dense tail with hard
    /// surfaces.
    pub const CAVE: Self = AudioEnvironment { wet: 0.3, decay: 2.6, damp: 0.2, size: 22.0, scatter: 0.8, reflect: 0.7 };

    /// A forest: no walls, just a short dark scatter off trunks and foliage - quiet, but unmistakably outdoors-with-
    /// presence.
    pub const FOREST: Self =
        AudioEnvironment { wet: 0.11, decay: 0.5, damp: 0.9, size: 12.0, scatter: 0.9, reflect: 0.12 };

    /// An open field: nearly dry, the faintest hint of ground scatter. Openness itself is the cue.
    pub const FIELD: Self =
        AudioEnvironment { wet: 0.05, decay: 0.25, damp: 0.9, size: 8.0, scatter: 0.7, reflect: 0.06 };
}

/// Global audio system controls: the master volume, per-bus category volumes, listener overrides, and an
/// output meter for checking your mix.
/// <https://stereokit.net/Pages/StereoKit/Audio.html>
pub struct Audio;

unsafe extern "C" {
    pub fn audio_set_volume(volume: f32);
    pub fn audio_get_volume() -> f32;
    pub fn audio_set_bus_volume(bus: SoundBus, volume: f32);
    pub fn audio_get_bus_volume(bus: SoundBus) -> f32;
    pub fn audio_set_listener(opt_pose: *const std::ffi::c_void);
    pub fn audio_get_output_decibels() -> f32;
    pub fn audio_set_env(environment: AudioEnvironment);
    pub fn audio_get_env() -> AudioEnvironment;
}

impl Audio {
    /// The master volume, a 0-1 trim over everything StereoKit plays. This is an app level control - the user's system
    /// volume sits below it.
    /// <https://stereokit.net/Pages/StereoKit/Audio/Volume.html>
    ///
    /// see also [`audio_set_volume`] [`Audio::get_volume`], [`Audio::bus_volume`] and [`Audio::get_bus_volume`]
    pub fn volume(value: f32) {
        unsafe {
            audio_set_volume(value);
        }
    }

    /// Sets a bus category's 0-1 volume trim. Every sound playing on that bus is affected, handy for sfx/music/ui
    /// sliders in a settings menu, or ducking a whole category.
    /// <https://stereokit.net/Pages/StereoKit/Audio/SetBusVolume.html>
    /// `bus` - The bus to adjust.
    /// `volume` - 0-1 volume trim for the bus.
    ///
    /// see also [`audio_set_bus_volume`] [`Audio::volume`] [`Audio::get_bus_volume`]
    pub fn bus_volume(bus: SoundBus, volume: f32) {
        unsafe {
            audio_set_bus_volume(bus, volume);
        }
    }

    /// The acoustic environment that spatial sounds play in! This drives a shared reverb and early reflections that
    /// carry a sense of space and absolute distance. The default is fully off (wet 0), which costs nothing and never
    /// fights the real room's acoustics - the right resting state for AR. Assign a preset like AudioEnvironment.Hall -
    /// Off returns to dry, zero cost playback - or build custom values, perhaps starting from a preset.
    /// <https://stereokit.net/Pages/StereoKit/Audio/Environment.html>
    ///
    /// see also [`audio_set_env`] [`Audio::get_environment`]
    pub fn environment(value: AudioEnvironment) {
        unsafe {
            audio_set_env(value);
        }
    }

    /// Normally the audio listener follows the user's head. Set this to hear the scene from somewhere else - a third
    /// person camera, or a remote avatar - and set it to null to give the ears back to the head.
    /// <https://stereokit.net/Pages/StereoKit/Audio/ListenerOverride.html>
    ///
    /// see also [`audio_set_listener`]
    pub fn listener_override(value: Option<Pose>) {
        if let Some(pose) = value {
            unsafe {
                audio_set_listener(&pose as *const Pose as *const std::ffi::c_void);
            }
        } else {
            unsafe {
                audio_set_listener(std::ptr::null());
            }
        }
    }

    /// The master volume, a 0-1 trim over everything StereoKit plays. This is an app level control - the user's system
    /// volume sits below it.
    /// <https://stereokit.net/Pages/StereoKit/Audio/Volume.html>
    ///
    /// see also [`audio_get_volume`] [`Audio::volume`] [`Audio::get_bus_volume`]
    pub fn get_volume() -> f32 {
        unsafe { audio_get_volume() }
    }

    /// Gets a bus category's current 0-1 volume trim.
    /// <https://stereokit.net/Pages/StereoKit/Audio/GetBusVolume.html>
    /// `bus` - The bus to inspect.
    ///
    /// Returns the bus's 0-1 volume trim.
    /// see also [`audio_get_bus_volume`] [`Audio::get_volume`] [`Audio::bus_volume`]
    pub fn get_bus_volume(bus: SoundBus) -> f32 {
        unsafe { audio_get_bus_volume(bus) }
    }

    /// RMS level of the last mixed audio block in dBFS, -120 when silent. Useful for level meters, and for checking
    /// where your content sits relative to the limiter at 0.
    /// <https://stereokit.net/Pages/StereoKit/Audio/OutputDecibels.html>
    ///
    /// see also [`audio_get_output_decibels`]
    pub fn get_output_decibels() -> f32 {
        unsafe { audio_get_output_decibels() }
    }

    /// The acoustic environment that spatial sounds play in! This drives a shared reverb and early reflections that
    /// carry a sense of space and absolute distance. The default is fully off (wet 0), which costs nothing and never
    /// fights the real room's acoustics - the right resting state for AR. Assign a preset like AudioEnvironment.Hall -
    /// Off returns to dry, zero cost playback - or build custom values, perhaps starting from a preset.
    /// <https://stereokit.net/Pages/StereoKit/Audio/Environment.html>
    ///
    /// see also [`audio_get_env`] [`Audio::environment`]
    pub fn get_environment() -> AudioEnvironment {
        unsafe { audio_get_env() }
    }
}

/// This class provides access to the hardware’s microphone, and stores it in a Sound stream. Start and Stop recording,
/// and check the Sound property for the results! Remember to ensure your application has microphone permissions enabled!
/// <https://stereokit.net/Pages/StereoKit/Microphone.html>
///
/// see also: [`Sound`]
/// /// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec3, Matrix}, mesh::Mesh, material::Material,
///                      sound::Microphone, util::named_colors};
///
/// let sphere = Mesh::generate_cube(Vec3::ONE * 0.5, None);
/// let material = Material::pbr().tex_file_copy("textures/micro.jpeg", true, None)
///                    .expect("sound.jpeg should be there");
/// let position = Vec3::new( 0.0, 0.0, 0.5);
/// let transform = Matrix::t(position);
///
/// let micros = Microphone::get_devices();
///
/// if micros.len() > 0 {
///     let first_in_list = micros[0].clone();
///     if Microphone::start(Some(first_in_list), None) {
///         assert!(Microphone::is_recording());
///     } else {
///         assert!(!Microphone::is_recording());
///     }
/// }
///
/// filename_scr = "screenshots/microphone.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     sphere.draw(&material, transform, Some(named_colors::LIGHT_BLUE.into()), None  );
///     if iter == 1990 && Microphone::is_recording() {
///         let micro_sound = Microphone::sound().expect("Microphone should be recording");
///         let mut read_samples: Vec<f32> = vec![0.0; 48000];
///         let recorded_data = micro_sound.read_samples(read_samples.as_mut_slice(), None);
///         Microphone::stop();
///         # assert!(recorded_data < 10000000);  // meaningless but useful ...
///     }
/// );
/// # sk::Sk::shutdown();
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/microphone.jpeg" alt="screenshot" width="200">
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct Microphone {
    sound: Sound,
}

unsafe extern "C" {
    pub fn mic_get_stream() -> SoundT;
    pub fn mic_is_recording() -> Bool32T;
    pub fn mic_device_count() -> i32;
    pub fn mic_device_name(index: i32) -> *const c_char;
    pub fn mic_start(device_name: *const c_char, sample_rate: SoundSampleRate) -> Bool32T;
    pub fn mic_stop();
}

impl Microphone {
    /// This is the sound stream of the Microphone when it is recording. This Asset is created the first time it is
    /// accessed via this property, or during Start, and will persist. It is re-used for the Microphone stream if you
    /// start/stop/switch devices.
    /// <https://stereokit.net/Pages/StereoKit/Microphone/Sound.html>
    ///
    /// see also [mic_get_stream]
    pub fn sound() -> Result<Sound, StereoKitError> {
        Ok(Sound(
            NonNull::new(unsafe { mic_get_stream() })
                .ok_or(StereoKitError::SoundCreate("microphone stream".to_string()))?,
        ))
    }

    /// Is the microphone currently recording?
    /// <https://stereokit.net/Pages/StereoKit/Microphone/IsRecording.html>
    ///
    /// see also [`mic_is_recording`]
    pub fn is_recording() -> bool {
        unsafe { mic_is_recording() != 0 }
    }

    /// Constructs a list of valid Microphone devices attached to the system. These names can be passed into Start to
    /// select a specific device to record from. It’s recommended to cache this list if you’re using it frequently, as
    /// this list is constructed each time you call it.
    ///
    /// It’s good to note that a user might occasionally plug or unplug microphone devices from their system, so this
    /// list may occasionally change.
    /// <https://stereokit.net/Pages/StereoKit/Microphone/GetDevices.html>
    ///
    /// see also [`mic_device_count`] [`mic_device_name`]
    pub fn get_devices() -> Vec<String> {
        let mut devices = Vec::new();
        for iter in 0..unsafe { mic_device_count() } {
            let device_name = unsafe { CStr::from_ptr(mic_device_name(iter)) }.to_str().unwrap_or_default().to_string();
            devices.push(device_name);
        }
        devices
    }

    /// This begins recording audio from the Microphone! Audio is stored in Microphone.Sound as a stream of audio. If
    /// the Microphone is already recording with a different device, it will stop the previous recording and start again
    /// with the new device.
    ///
    /// If null is provided as the device, then they system’s default input device will be used. Some systems may not
    /// provide access to devices other than the system’s default.
    /// <https://stereokit.net/Pages/StereoKit/Microphone/Start.html>
    /// * `device_name` - The name of the microphone device to use, as seen in the GetDevices list. None will use the
    ///   system’s default device preference.
    ///
    /// see also [`mic_start`] [`Microphone::get_devices`] [`Microphone::stop`]
    pub fn start(device_name: Option<String>, sample_rate: Option<SoundSampleRate>) -> bool {
        let sample_rate = sample_rate.unwrap_or_default();
        if let Some(device_name) = device_name
            && !device_name.is_empty()
        {
            let cstr = CString::new(device_name).unwrap_or_default();
            return unsafe { mic_start(cstr.as_ptr() as *const c_char, sample_rate) != 0 };
        }
        // Here we call for a null_mut device_name
        unsafe { mic_start(std::ptr::null_mut() as *const c_char, sample_rate) != 0 }
    }

    /// Stops recording audio from the microphone.
    /// <https://stereokit.net/Pages/StereoKit/Microphone/Stop.html>
    ///
    /// see also [`mic_stop`] [`Microphone::start`]
    pub fn stop() {
        unsafe { mic_stop() }
    }
}

/// A category a playing sound belongs to. Each bus is just a volume control that affects every sound tagged with it,
/// handy for separate sfx/music/ui volume sliders, or ducking categories wholesale.
/// <https://stereokit.net/Pages/StereoKit/SoundBus.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum SoundBus {
    /// General sound effects, the default bus.
    #[default]
    Sfx = 0,
    /// Background music and ambience.
    Music = 1,
    /// Interface feedback sounds. StereoKit's own UI sounds use this bus.
    Ui = 2,
    /// Dialogue, voice-over, and voice comms.
    Voice = 3,
}

/// The channel format of a Sound's data. Only mono sounds spatialize - playing a non-mono sound ignores its position
/// entirely.
/// <https://stereokit.net/Pages/StereoKit/SoundChannels.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum SoundChannels {
    /// One channel. Spatializes as a point or shaped source, the default and by far the most common format for game
    /// audio.
    #[default]
    Mono = 0,
    /// Two interleaved channels, played back head-locked and untouched. Music, and pre-rendered binaural content.
    Stereo = 1,
    /// Four interleaved first order (1) ambisonic channels in the ambiX convention (ACN order W,Y,Z,X with SN3D
    /// normalization). The sound field stays world-fixed, counter-rotating against the head - the head-tracked
    /// generalization of a binaural render. Great for recorded or simulated environmental beds.
    Ambisonic1 = 2,
}

/// Common audio sample rates, in Hz, for sound streams and microphone capture. The enum value _is_ the rate in Hz, so
/// you can cast any integer rate to this type - these are just the well-supported ones, tagged with where each is
/// typically used. StereoKit mixes everything at 48kHz and resamples to and from other rates as needed, so any positive
/// rate works, but a rate a device captures or plays natively avoids an extra resample.
/// <https://stereokit.net/Pages/StereoKit/SoundSampleRate.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum SoundSampleRate {
    /// Use StereoKit's native mix rate, 48kHz. No resampling in the mixer, and the best default unless you have a
    /// specific reason otherwise.
    #[default]
    Default = 0,
    /// 8kHz narrowband telephony, classic Bluetooth headset (HFP/SCO) quality. Tiny data rate, intelligible speech
    /// only.
    Telephony = 8000,
    /// 16kHz wideband speech - the rate that speech-to-text, wake-word, and VoIP pipelines typically expect. A good
    /// low-bandwidth choice for voice.
    Speech = 16000,
    /// 32kHz, seen in some broadcast audio and Bluetooth wideband (mSBC).
    Broadcast = 32000,
    /// 44.1kHz, the CD-audio standard and a common consumer device default.
    Cd = 44100,
    /// 48kHz, the AV/pro standard and StereoKit's native mix rate. The modern default for most capture hardware.
    Standard = 48000,
    /// 96kHz high-resolution pro audio. Rare for a microphone, and resampled down to 48kHz for mixing anyway.
    Studio = 96000,
    /// 192kHz, the extreme end of pro audio interfaces. Almost never a real microphone rate, and heavily oversampled
    /// for StereoKit's purposes.
    Ultra = 192000,
}

bitflags::bitflags! {
    /// Option flags for playing a sound, see [`SoundPlay`].
    /// <https://stereokit.net/Pages/StereoKit/SoundFlags.html>
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    #[repr(C)]
    pub struct SoundFlags: u32 {
        /// No special behavior, the default.
        const None              = 0;
        /// The sound restarts from the beginning when it reaches the end of its data, and plays until stopped. Live
        /// streams ignore this, they already wait for data forever.
        const Loop              = 1 << 0;
        /// Skip spatialization entirely: no distance attenuation, panning, or filtering. The sound follows the head,
        /// good for music, UI, or pre-rendered binaural content.
        const HeadLocked        = 1 << 1;
        /// Delay the sound's onset by its distance from the listener divided by the speed of sound (343m/s), computed
        /// once when playback starts. Great for thunder, explosions, and other far away events.
        const PropagationDelay  = 1 << 2;
    }
}
impl Default for SoundFlags {
    fn default() -> Self {
        SoundFlags::None
    }
}

/// Extra parameters for playing a sound with [`Sound::play_with`], this is the raw native layout - the public
/// API is [`SoundPlay`].
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SoundPlayT {
    /// A 0-1 volume trim on top of the sound's decibel loudness. 0 is treated as the default full trim of 1. For real
    /// silence, use a tiny value. Values above 1 amplify, negatives clamp to 0.
    pub volume: f32,
    /// Playback rate multiplier, clamped to 0.25-4. 1 is normal speed, 2 is twice as fast and an octave up. 0 is
    /// treated as 1.
    pub pitch: f32,
    /// Apparent size of the source, 0-1. 0 is a point in space, 1 fills the whole sound field evenly. Great for wind,
    /// rivers, and rumble - but keep transients like impacts at 0, width smears their attack.
    pub spread: f32,
    /// Seconds before the sound actually starts playing, sample accurate.
    /// [`SoundFlags::PropagationDelay`] adds distance/343m/s on top of this.
    pub delay: f32,
    /// Low-pass filter cutoff override in Hz for this voice. 0 uses the automatic distance/direction model.
    pub cutoff: f32,
    /// The volume category this sound belongs to, [`SoundBus::Sfx`] when zeroed.
    pub bus: SoundBus,
    /// See [`SoundFlags`]!
    pub flags: SoundFlags,
    /// Optional emitter shape points, or null for a point source.
    pub shape_points: *const Vec3,
    /// Number of points in `shape_points`.
    pub shape_point_count: i32,
    /// Radius of the shape's sphere or polyline tube, in meters.
    pub shape_radius: f32,
}

/// Optional settings for [`Sound::play_with`]! The default struct plays a plain point source: full volume
/// trim, normal pitch, no delay, on the Sfx bus.
/// <https://stereokit.net/Pages/StereoKit/SoundPlay.html>
#[derive(Debug, Clone, PartialEq)]
pub struct SoundPlay {
    /// A 0-1 volume trim on top of the Sound's Decibels loudness. 0 is treated as the default full trim of 1, use a
    /// tiny value for real silence. Values above 1 amplify, negatives clamp to 0.
    pub volume: f32,
    /// Playback rate multiplier, clamped to 0.25-4. 1 is normal speed, 2 is twice as fast and an octave up. 0 is
    /// treated as 1.
    pub pitch: f32,
    /// Apparent size of the source, 0-1. 0 is a point in space, 1 fills the whole sound field evenly. Great for wind,
    /// rivers and rumble, but keep transients like impacts at 0 - width smears their attack.
    pub spread: f32,
    /// Seconds before the sound actually starts playing, sample accurate. [`SoundFlags::PropagationDelay`] adds
    /// distance/343m/s on top of this.
    pub delay: f32,
    /// Low-pass filter cutoff override in Hz for this voice. 0 uses the automatic distance model.
    pub cutoff: f32,
    /// The volume category this sound belongs to, [`SoundBus::Sfx`] when zeroed.
    pub bus: SoundBus,
    /// See [`SoundFlags`]!
    pub flags: SoundFlags,
    /// Optional emitter shape: 1 point is a sphere, 2+ a rounded polyline. The emitter follows the listener along the
    /// shape - position becomes the closest point, and apparent size grows as the shape fills more of the view, going
    /// fully diffuse inside it. Points are copied at play, max 32. Empty means a point source at the play position.
    pub shape: Vec<Vec3>,
    /// Radius of the shape's sphere or polyline tube, in meters.
    pub shape_radius: f32,
}

impl Default for SoundPlay {
    /// A zero-initialized struct is a valid default state: full volume trim, normal pitch, no delay, on the Sfx bus.
    fn default() -> Self {
        Self {
            volume: 0.0,
            pitch: 0.0,
            spread: 0.0,
            delay: 0.0,
            cutoff: 0.0,
            bus: SoundBus::Sfx,
            flags: SoundFlags::None,
            shape: Vec::new(),
            shape_radius: 0.0,
        }
    }
}

impl SoundPlay {
    /// Convert to the native FFI struct. Borrows the shape points from `self`.
    fn to_native(&self) -> SoundPlayT {
        SoundPlayT {
            volume: self.volume,
            pitch: self.pitch,
            spread: self.spread,
            delay: self.delay,
            cutoff: self.cutoff,
            bus: self.bus,
            flags: self.flags,
            shape_points: if self.shape.is_empty() { null() } else { self.shape.as_ptr() },
            shape_point_count: self.shape.len() as i32,
            shape_radius: self.shape_radius,
        }
    }
}

/// This class represents a sound effect! Excellent for blips and bloops and little clips that you might play around
/// your scene. Right now, this supports .wav, .mp3, and procedurally generated noises!
/// <https://stereokit.net/Pages/StereoKit/Sound.html>
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec3, Quat, Matrix}, mesh::Mesh, material::Material,  
///                      sound::Sound, util::named_colors};
///
/// let mesh = Mesh::generate_cube(Vec3::ONE * 1.6, None);
/// let material = Material::unlit().tex_file_copy("textures/sound.jpeg", true, None)
///                    .expect("sound.jpeg should be there");
/// let mut position = Vec3::new(-0.5, 0.0, 0.5);
/// let rotation = Quat::from_angles(45.0, 45.0, 45.0);
/// let mut transform = Matrix::IDENTITY;
///
/// let mut plane_sound = Sound::from_file("sounds/plane_engine.mp3")
///                          .expect("plane_engine.mp3 should be there");
/// plane_sound.id("sound_plane").decibels(70.0);
///
/// let mut plane_sound_inst = plane_sound.play(position, Some(1.0));
///
/// number_of_steps = 450;
/// filename_scr = "screenshots/sound.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     transform.update_t_r(&position, &rotation);
///     mesh.draw(&material, transform, Some(named_colors::CYAN.into()), None);
///     if iter == 0 {
///         assert!(plane_sound_inst.is_playing());
///         position = Vec3::new(0.0, 0.0, -1.0);
///         plane_sound_inst
///             .position(position)
///             .volume(0.5);
///     } else if iter == 10 {
///         assert!(plane_sound_inst.is_playing());
///         assert_eq!(plane_sound_inst.get_position(), Vec3::new(0.0, 0.0, -1.0));
///         assert_eq!(plane_sound_inst.get_volume(), 0.5);
///         plane_sound_inst.stop();
///         assert!(!plane_sound_inst.is_playing());
///    } else if iter == 449 {
///         assert!(!plane_sound_inst.is_playing());
///    }
/// );
/// # sk::Sk::shutdown();
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sound.jpeg" alt="screenshot" width="200">
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct Sound(pub NonNull<_SoundT>);

impl Drop for Sound {
    fn drop(&mut self) {
        unsafe { sound_release(self.0.as_ptr()) };
    }
}
impl AsRef<Sound> for Sound {
    fn as_ref(&self) -> &Sound {
        self
    }
}

/// StereoKit internal type.
#[repr(C)]
#[derive(Debug)]
pub struct _SoundT {
    _unused: [u8; 0],
}

/// StereoKit ffi type.
pub type SoundT = *mut _SoundT;

unsafe impl Send for Sound {}
unsafe impl Sync for Sound {}

unsafe extern "C" {
    pub fn sound_find(id: *const c_char) -> SoundT;
    pub fn sound_set_id(sound: SoundT, id: *const c_char);
    pub fn sound_get_id(sound: SoundT) -> *const c_char;
    pub fn sound_create(filename_utf8: *const c_char) -> SoundT;
    pub fn sound_create_mem(id: *const c_char, in_arr_data: *const c_void, data_size: usize) -> SoundT;
    pub fn sound_create_stream(buffer_duration: f32, channels: SoundChannels, sample_rate: SoundSampleRate) -> SoundT;
    pub fn sound_create_samples(
        in_arr_samples_at_48000s: *const f32,
        sample_count: u64,
        channels: SoundChannels,
    ) -> SoundT;
    pub fn sound_get_channels(sound: SoundT) -> SoundChannels;
    pub fn sound_asset_state(sound: SoundT) -> AssetState;
    pub fn sound_generate(
        audio_generator: Option<unsafe extern "C" fn(out_arr_samples: *mut f32, frame_start: u64, frame_count: u64)>,
        duration: f32,
        channels: SoundChannels,
    ) -> SoundT;
    pub fn sound_write_samples(sound: SoundT, in_arr_samples: *const f32, sample_count: u64);
    pub fn sound_read_samples(sound: SoundT, out_arr_samples: *mut f32, sample_count: u64) -> u64;
    pub fn sound_unread_samples(sound: SoundT) -> u64;
    pub fn sound_total_samples(sound: SoundT) -> u64;
    pub fn sound_cursor_samples(sound: SoundT) -> u64;
    pub fn sound_get_decibels(sound: SoundT) -> f32;
    pub fn sound_set_decibels(sound: SoundT, decibels: f32);
    pub fn sound_play(sound: SoundT, at: Vec3, opt_settings: *const SoundPlayT) -> SoundInst;
    pub fn sound_duration(sound: SoundT) -> f32;
    pub fn sound_addref(sound: SoundT);
    pub fn sound_release(sound: SoundT);
}

impl IAsset for Sound {
    // fn id(&mut self, id: impl AsRef<str>) {
    //     self.id(id);
    // }

    fn get_id(&self) -> &str {
        self.get_id()
    }

    fn as_asset(&self) -> crate::system::AssetT {
        self.0.as_ptr() as crate::system::AssetT
    }
}

// Default is click
impl Default for Sound {
    fn default() -> Self {
        Sound::click()
    }
}

impl Sound {
    /// Create a sound used for streaming audio in or out! This is useful for things like reading from a microphone
    /// stream, or playing audio from a source streaming over the network, or even procedural sounds that are generated on the fly!
    /// Use stream sounds with the WriteSamples and ReadSamples functions.
    /// <https://stereokit.net/Pages/StereoKit/Sound/CreateStream.html>
    /// * `stream_buffer_duration` - How much audio time should this stream be able to hold without writing back over
    ///   itself?
    ///
    /// see also [`sound_create_stream`] [`Sound::from_samples`] [`Sound::create_stream_with`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// let mut stream_sound = Sound::create_stream(0.5).
    ///                            expect("A sound stream should be created");
    /// assert!(stream_sound.get_id().starts_with("auto/sound_"));
    /// stream_sound.id("sound_stream");
    ///
    /// let mut samples: Vec<f32> = vec![0.0; 48000];
    /// for i in 0..48000 {
    ///     samples[i] = (i as f32 / 48000.0).sin();
    /// }
    /// stream_sound.write_samples(samples.as_slice(), Some(48000));
    /// assert_eq!(stream_sound.get_duration(), 0.5);
    ///
    /// let stream_sound_inst = stream_sound.play([0.0, 0.0, -0.5], Some(0.5));
    ///
    /// number_of_steps = 300;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == 10 {
    ///         assert!(stream_sound_inst.is_playing());
    ///     } else if iter == 20 {
    ///         assert!(stream_sound_inst.is_playing());
    ///         stream_sound_inst.stop();
    ///         assert!(!stream_sound_inst.is_playing());
    ///     } else if iter == 299   {
    ///         assert!(!stream_sound_inst.is_playing());
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn create_stream(stream_buffer_duration: f32) -> Result<Sound, StereoKitError> {
        Self::create_stream_with(stream_buffer_duration, SoundChannels::Mono, SoundSampleRate::Default)
    }

    /// Create a stream sound with an explicit channel format and sample rate! A 16,000hz mono stream suits speech
    /// pipelines, while a stereo stream can carry pre-rendered music. Written samples are interleaved for
    /// multi-channel formats, and playback resamples to the mixer's 48,000hz automatically.
    /// <https://stereokit.net/Pages/StereoKit/Sound/CreateStream.html>
    /// * `stream_buffer_duration` - How much audio time should this stream be able to hold without writing back over
    ///   itself?
    /// * `channels` - The stream's channel format.
    /// * `sample_rate` - Capture/playback rate. [`SoundSampleRate`] names the common rates with notes - Default uses
    ///   the mixer's native 48,000, Speech (16,000) suits speech pipelines. The enum value is the rate in Hz, so cast
    ///   any integer rate to it for something off this list; playback resamples to 48,000 automatically.
    ///
    /// see also [`sound_create_stream`] [`Sound::create_stream`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::{Sound, SoundChannels, SoundSampleRate};
    ///
    /// // A stereo stream at the native mix rate for pre-rendered music
    /// let stereo_stream = Sound::create_stream_with(1.0, SoundChannels::Stereo, SoundSampleRate::Default)
    ///     .expect("A stereo stream should be created");
    /// assert!(stereo_stream.get_id().starts_with("auto/sound_"));
    ///
    /// // A 16kHz mono stream well-suited for a speech-to-text pipeline
    /// let speech_stream = Sound::create_stream_with(0.5, SoundChannels::Mono, SoundSampleRate::Speech)
    ///     .expect("A speech stream should be created");
    /// assert!(speech_stream.get_id().starts_with("auto/sound_"));
    /// # sk::Sk::shutdown();
    /// ```
    pub fn create_stream_with(
        stream_buffer_duration: f32,
        channels: SoundChannels,
        sample_rate: SoundSampleRate,
    ) -> Result<Sound, StereoKitError> {
        Ok(Sound(
            NonNull::new(unsafe { sound_create_stream(stream_buffer_duration, channels, sample_rate) })
                .ok_or(StereoKitError::SoundCreate("create_stream failed".into()))?,
        ))
    }

    /// Loads a sound from file! StereoKit supports .wav and .mp3 files. Mono sounds spatialize, stereo plays
    /// head-locked, and 4 channel files load as first order ambisonics: world-fixed sound fields that counter-rotate
    /// against the user's head, ideal for environmental beds like rain, wind, or crowds. Bare 4 channel content is
    /// read as ambiX (ACN order, SN3D - the YouTube 360 convention), FuMa-tagged .amb files are converted on load, and
    /// other surround layouts downmix to stereo. Check Channels for what a file loaded as. Decoding happens
    /// asynchronously, but playing right away is fine - a Play before the decode finishes catches up to real time once
    /// it lands, as if it had started on schedule.
    ///
    /// <https://stereokit.net/Pages/StereoKit/Sound/FromFile.html>
    /// * `file_utf8` - Name of the audio file! Supports .wav and .mp3 files.
    ///
    /// see also [`sound_create`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, sound::Sound};
    ///
    /// let position = Vec3::new(-0.5, 0.0, 0.5);
    ///
    /// let mut plane_sound = Sound::from_file("sounds/plane_engine.mp3")
    ///                           .expect("no.wav should be in the sounds folder");
    /// assert_eq!(plane_sound.get_id(), "sounds/plane_engine.mp3");
    /// plane_sound.id("sound_plane").decibels(90.0);
    ///
    /// let plane_sound_inst = plane_sound.play(position, Some(1.0));
    ///
    /// # if cfg!(not(feature = "test-xr-mode")) {
    /// number_of_steps = 450;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == 449 {
    ///         assert!(plane_sound_inst.is_playing());
    ///     }
    /// );
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn from_file(file_utf8: impl AsRef<Path>) -> Result<Sound, StereoKitError> {
        let path_buf = file_utf8.as_ref().to_path_buf();
        let c_str = CString::new(path_buf.clone().to_str().ok_or(StereoKitError::SoundFile(path_buf.clone()))?)?;

        Ok(Sound(
            NonNull::new(unsafe { sound_create(c_str.as_ptr()) }).ok_or(StereoKitError::SoundFile(path_buf))?,
        ))
    }

    /// Loads a sound from a file's data in memory! Same format support and async decode behavior as
    /// [`Sound::from_file`]. The data is copied, so the array is yours again as soon as this returns.
    /// <https://stereokit.net/Pages/StereoKit/Sound/FromMemory.html>
    /// * `data` - The complete contents of an audio file.
    /// * `id` - A unique identifier for this sound - loading the same id again returns the already loaded sound.
    ///
    /// see also [`sound_create_mem`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// // Embed the file bytes at compile time and load from memory.
    /// let data = include_bytes!("../assets/sounds/no.wav");
    /// let sound = Sound::from_memory(data, "sound_from_memory")
    ///     .expect("Sound should be created from memory");
    /// assert_eq!(sound.get_id(), "sound_from_memory");
    ///
    /// // Loading the same id again returns the already loaded sound.
    /// let same = Sound::find("sound_from_memory").expect("sound should be found");
    /// assert_eq!(sound, same);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn from_memory(data: &[u8], id: impl AsRef<str>) -> Result<Sound, StereoKitError> {
        let cstr_id = CString::new(id.as_ref())?;
        Ok(Sound(
            NonNull::new(unsafe { sound_create_mem(cstr_id.as_ptr(), data.as_ptr() as *const c_void, data.len()) })
                .ok_or(StereoKitError::SoundCreate("from_memory failed".into()))?,
        ))
    }

    /// This function will create a sound from an array of samples. Values should range from -1 to +1, and there should
    /// be 48,000 values per second of audio.
    /// <https://stereokit.net/Pages/StereoKit/Sound/FromSamples.html>
    /// * `in_arr_samples_at_48000s` - Values should range from -1 to +1, and there should be 48,000 per second of audio.
    ///
    /// see also [`sound_create_samples`] [`Sound::write_samples`] [`Sound::from_samples_with`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// let mut samples: Vec<f32> = vec![0.0; 48000];
    /// for i in 0..48000 {
    ///     samples[i] = (i as f32 / 48000.0).sin();
    /// }
    /// let mut sound = Sound::from_samples(&samples)
    ///                     .expect("Sound should be created from samples");
    /// assert!(sound.get_id().starts_with("auto/sound_"));
    /// sound.id("sound_samples");
    ///
    /// let sound_inst = sound.play([0.0, 0.0, -0.5], Some(0.5));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     assert!(sound_inst.is_playing());
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn from_samples(in_arr_samples_at_48000s: &[f32]) -> Result<Sound, StereoKitError> {
        Self::from_samples_with(in_arr_samples_at_48000s, SoundChannels::Mono)
    }

    /// Create a sound from an array of samples with an explicit channel format! Multi-channel data is interleaved -
    /// stereo alternates left/right, and Ambisonic1 packs W,Y,Z,X per frame in the ambiX convention. 48,000 frames per
    /// second of audio.
    /// <https://stereokit.net/Pages/StereoKit/Sound/FromSamples.html>
    /// * `in_arr_samples_at_48000s` - Interleaved samples from -1 to +1, 48,000 frames per second.
    /// * `channels` - How the samples are laid out.
    ///
    /// see also [`sound_create_samples`] [`Sound::from_samples`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::{Sound, SoundChannels};
    ///
    /// // 1 second of stereo audio: 48000 frames, 2 interleaved samples each.
    /// let mut samples: Vec<f32> = vec![0.0; 48000 * 2];
    /// for i in 0..48000 {
    ///     let t = i as f32 / 48000.0;
    ///     samples[i * 2]     = (t * 440.0 * 2.0 * std::f32::consts::PI).sin(); // left
    ///     samples[i * 2 + 1] = (t * 440.0 * 2.0 * std::f32::consts::PI).sin(); // right
    /// }
    /// let sound = Sound::from_samples_with(&samples, SoundChannels::Stereo)
    ///     .expect("Stereo sound should be created");
    /// assert_eq!(sound.get_channels(), SoundChannels::Stereo);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn from_samples_with(
        in_arr_samples_at_48000s: &[f32],
        channels: SoundChannels,
    ) -> Result<Sound, StereoKitError> {
        Ok(Sound(
            NonNull::new(unsafe {
                sound_create_samples(in_arr_samples_at_48000s.as_ptr(), in_arr_samples_at_48000s.len() as u64, channels)
            })
            .ok_or(StereoKitError::SoundCreate("from_samples failed".into()))?,
        ))
    }

    /// This function will generate a sound from a function you provide! The function is called once for each buffer of
    /// samples in the duration. As an example, it may be called 48,000 times for each second of duration.
    /// <https://stereokit.net/Pages/StereoKit/Sound/Generate.html>
    /// * `generator` - This function takes a pointer to a buffer of samples, the index of the buffer's first frame, and
    ///   the number of frames to fill. It should fill the buffer completely with values from -1 to +1 representing the
    ///   audio wave.
    /// * `duration` - The duration of the sound in seconds.
    ///
    /// see also [`sound_generate`] [`Sound::generate_with`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// unsafe extern "C" fn generator(out_samples: *mut f32, frame_start: u64, frame_count: u64) {
    ///     let buf = unsafe{std::slice::from_raw_parts_mut(out_samples, frame_count as usize)};
    ///     for i in 0..frame_count as usize {
    ///         let t = (frame_start as usize + i) as f32 / 48000.0;
    ///         buf[i] = (t * 440.0 * 2.0 * std::f32::consts::PI).sin();
    ///     }
    /// }
    /// let mut sound = Sound::generate(generator, 1.0)
    ///                           .expect("Sound should be created from generator");
    /// assert!(sound.get_id().starts_with("auto/sound_"));
    /// sound.id("sound_generator");
    ///
    /// let sound_inst = sound.play([0.0, 0.0, -0.5], Some(0.5));
    ///
    /// # if cfg!(not(feature = "test-xr-mode")) {
    /// number_of_steps = 150;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     assert!(sound_inst.is_playing());
    /// );
    /// # }sk::Sk::shutdown();
    /// ```
    pub fn generate(
        generator: unsafe extern "C" fn(*mut f32, u64, u64),
        duration: f32,
    ) -> Result<Sound, StereoKitError> {
        Self::generate_with(generator, duration, SoundChannels::Mono)
    }

    /// This function generates a sound by asking your function to fill whole buffers of samples! This is far faster
    /// than a per-sample callback, one interop call instead of one per sample.
    ///
    /// With a channel format, the buffer holds frames-x-channels interleaved samples: stereo alternates left/right,
    /// and Ambisonic1 packs W,Y,Z,X per frame in the ambiX convention - so procedural head-tracked sound fields are
    /// just a generator away.
    /// <https://stereokit.net/Pages/StereoKit/Sound/Generate.html>
    /// * `generator` - Fills the provided buffer completely with interleaved audio sample values from -1 to +1. The
    ///   second parameter is the index of the buffer's first frame, at 48,000 frames per second.
    /// * `duration` - In seconds, how long should the sound be?
    /// * `channels` - The channel format the generator fills.
    ///
    /// see also [`sound_generate`] [`Sound::generate`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::{Sound, SoundChannels};
    ///
    /// // Generate a 1.5 second stereo tone, two interleaved samples per frame.
    /// unsafe extern "C" fn stereo_generator(out: *mut f32, start: u64, count: u64) {
    ///     unsafe {
    ///         let buf = std::slice::from_raw_parts_mut(out, (count * 2) as usize);
    ///         for i in 0..count as usize {
    ///             let t = (start as usize + i) as f32 / 48000.0;
    ///             let s = (t * 440.0 * 2.0 * std::f32::consts::PI).sin();
    ///             buf[i * 2]     = s; // left
    ///             buf[i * 2 + 1] = s; // right
    ///         }
    ///     }
    /// }
    /// let sound = Sound::generate_with(stereo_generator, 1.5, SoundChannels::Stereo)
    ///                       .expect("Stereo generated sound should be created");
    /// assert_eq!(sound.get_duration(), 1.5);
    /// let sound_inst = sound.play([0.0, 0.0, -0.5], Some(10.5));
    ///
    /// # if cfg!(not(feature = "test-xr-mode")) {
    /// number_of_steps = 150;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     assert!(sound_inst.is_playing());
    /// );
    /// # }sk::Sk::shutdown();
    /// ```
    pub fn generate_with(
        generator: unsafe extern "C" fn(*mut f32, u64, u64),
        duration: f32,
        channels: SoundChannels,
    ) -> Result<Sound, StereoKitError> {
        Ok(Sound(
            NonNull::new(unsafe { sound_generate(Some(generator), duration, channels) })
                .ok_or(StereoKitError::SoundCreate("sound_generate failed".into()))?,
        ))
    }

    /// ooks for a Sound asset that’s already loaded, matching the given id!
    /// <https://stereokit.net/Pages/StereoKit/Sound/Find.html>
    /// * `id` - Which Sound are you looking for?
    ///
    /// see also [`sound_find`] [`Sound::clone_ref`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// let mut plane_sound = Sound::from_file("sounds/plane_engine.mp3")
    ///                           .expect("plane_engine.mp3 should be in the sounds folder");
    /// plane_sound.id("sound_plane").decibels(70.0);
    ///
    /// let same_sound = Sound::find("sound_plane")
    ///                             .expect("sound_plane should be found");
    /// assert_eq!(plane_sound.get_id(), same_sound.get_id());
    /// assert_eq!(plane_sound, same_sound);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn find<S: AsRef<str>>(id: S) -> Result<Sound, StereoKitError> {
        let cstr_id = CString::new(id.as_ref())?;
        Ok(Sound(
            NonNull::new(unsafe { sound_find(cstr_id.as_ptr()) })
                .ok_or(StereoKitError::SoundFind(id.as_ref().to_string(), "not found".to_owned()))?,
        ))
    }

    /// Creates a clone of the same reference. Basically, the new variable is the same asset. This is what you get by
    /// calling find() method.
    /// <https://stereokit.net/Pages/StereoKit/Sound/Find.html>
    ///
    /// see also [`sound_find`] [`Sound::find`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// let plane_sound = Sound::from_file("sounds/plane_engine.mp3")
    ///                           .expect("plane_engine.mp3 should be in the sounds folder");
    ///
    /// let same_sound =  plane_sound.clone_ref();
    ///
    /// assert_eq!(plane_sound.get_id(), same_sound.get_id());
    /// assert_eq!(plane_sound, same_sound);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn clone_ref(&self) -> Sound {
        Sound(NonNull::new(unsafe { sound_find(sound_get_id(self.0.as_ptr())) }).expect("<asset>::clone_ref failed!"))
    }

    /// Sets the unique identifier of this asset resource! This can be helpful for debugging,
    /// managing your assets, or finding them later on!
    /// <https://stereokit.net/Pages/StereoKit/Sound/Id.html>
    ///
    /// see also [`sound_set_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// // A sound from a file will have its file path as its id
    /// let mut plane_sound = Sound::from_file("sounds/plane_engine.mp3")
    ///                           .expect("plane_engine.mp3 should be in the sounds folder");
    /// assert_eq!(plane_sound.get_id(), "sounds/plane_engine.mp3");
    /// plane_sound.id("plane_sound");
    /// assert_eq!(plane_sound.get_id(), "plane_sound");
    ///
    /// // A sound other than from a file will have an auto id
    /// let mut stream_sound = Sound::create_stream(0.5).
    ///                            expect("A sound stream should be created");
    /// assert!(stream_sound.get_id().starts_with("auto/sound_"));
    /// stream_sound.id("sound_stream");
    /// # sk::Sk::shutdown();
    /// ```
    pub fn id<S: AsRef<str>>(&mut self, id: S) -> &mut Self {
        let cstr_id = CString::new(id.as_ref()).unwrap_or_default();
        unsafe { sound_set_id(self.0.as_ptr(), cstr_id.as_ptr()) };
        self
    }

    /// Plays the sound at the 3D location specified, using the volume parameter as an additional volume control option!
    /// Sound volume falls off from 3D location, and can also indicate direction and location through spatial audio
    /// cues. So make sure the position is where you want people to think it’s from! Currently, if this sound is playing
    /// somewhere else, it’ll be canceled, and moved to this location.
    /// <https://stereokit.net/Pages/StereoKit/Sound/Play.html>
    /// * `at` - World space location for the audio to play at.
    /// * `volume` - Volume modifier for the effect! 1 means full volume, and 0 means completely silent. If None will
    ///   have default value of 1.0
    ///
    /// Returns a link to the Sound's play instance, which you can use to track and modify how the sound plays after
    /// the initial conditions are set.
    /// see also [`sound_play`] [`Sound::play_with`] [`SoundInst::position`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, sound::Sound};
    ///
    /// let position = Vec3::new(-0.5, 0.0, 0.5);
    ///
    /// let mut plane_sound = Sound::from_file("sounds/plane_engine.mp3").
    ///                           expect("A sound should be created");
    /// plane_sound.id("sound_plane").decibels(70.0);
    ///
    /// let mut plane_sound_inst = plane_sound.play(position, Some(1.0));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     assert!(plane_sound_inst.is_playing());
    ///     if iter == 2 {
    ///        // Move the sound to the other side
    ///        plane_sound_inst.position(Vec3::new(0.5, 0.0, 0.5));
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn play(&self, at: impl Into<Vec3>, volume: Option<f32>) -> SoundInst {
        let volume = volume.unwrap_or(1.0);
        // A zeroed volume resolves to full trim natively, so preserve this
        // overload's documented 0 = silent with a near-silent value.
        let settings = SoundPlay { volume: if volume == 0.0 { 1e-8 } else { volume }, ..Default::default() };
        self.play_with(at, &settings)
    }

    /// Plays the sound at the 3D location specified, with extra settings! Pitch, onset delay, emitter shapes, bus
    /// routing, and behavior flags all live in [`SoundPlay`] - a default struct behaves just like the plain
    /// [`Sound::play`] call.
    /// <https://stereokit.net/Pages/StereoKit/Sound/Play.html>
    /// * `at` - World space location for the audio to play at. Ignored for non-mono sounds and head-locked plays.
    /// * `settings` - Extra playback settings, see [`SoundPlay`].
    ///
    /// Returns a link to the Sound's play instance, which you can use to track and modify how the sound plays after
    /// the initial conditions are set.
    /// see also [`sound_play`] [`SoundPlay`] [`Sound::play`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{sound::{Sound, SoundPlay, SoundFlags, SoundBus}};
    ///
    /// let sound = Sound::from_file("sounds/plane_engine.mp3")
    ///     .expect("A sound should be created");
    /// sound.decibels(70.0);
    ///
    /// // A head-locked, looping play on the music bus.
    /// let settings = SoundPlay {
    ///     flags: SoundFlags::HeadLocked | SoundFlags::Loop,
    ///     bus: SoundBus::Music,
    ///     ..Default::default()
    /// };
    /// let inst = sound.play_with([0.0, 0.0, 0.0], &settings);
    /// assert!(inst.is_playing());
    /// inst.stop();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn play_with(&self, at: impl Into<Vec3>, settings: &SoundPlay) -> SoundInst {
        let native = settings.to_native();
        unsafe { sound_play(self.0.as_ptr(), at.into(), &native) }
    }

    /// The sound's real-world loudness at 1 meter, in decibels! StereoKit measures the audio data's loudness, so the
    /// value you declare here is the loudness you get - the waveform is the *shape* of the sound, Decibels is how loud
    /// it is. Loudness then falls off physically with distance (-6dB per doubling), so louder things carry farther
    /// with no extra tuning.
    ///
    /// Some reference points: rustling leaves 20, a whisper 30, calm conversation 60, a vacuum cleaner at arm's length
    /// 75, a busy street corner 80 (the default), shouting up close 88, a rock concert 110, thunder from a nearby
    /// strike 120.
    /// <https://stereokit.net/Pages/StereoKit/Sound/Decibels.html>
    ///
    /// see also [`sound_set_decibels`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, sound::Sound};
    ///
    /// let position = Vec3::new(-0.5, 0.0, 0.5);
    ///
    /// let mut plane_sound = Sound::from_file("sounds/plane_engine.mp3").
    ///                           expect("A sound should be created");
    /// plane_sound.id("sound_plane").decibels(70.0);
    ///
    /// let plane_sound_inst = plane_sound.play(position, Some(1.0));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     assert!(plane_sound_inst.is_playing());
    ///     if iter == 1 {
    ///         // Change decibel for all instances
    ///          assert_eq!(plane_sound.get_decibels(), 70.0);
    ///         plane_sound.decibels(10.0);
    ///     } else if iter == 2 {
    ///         assert_eq!(plane_sound.get_decibels(), 10.0);
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn decibels(&self, decibels: f32) {
        unsafe { sound_set_decibels(self.0.as_ptr(), decibels) }
    }

    /// This will read samples from the sound stream, starting from the first unread sample. Check UnreadSamples for how
    /// many samples are available to read.
    /// <https://stereokit.net/Pages/StereoKit/Sound/ReadSamples.html>
    /// * `out_arr_samples` - A pre-allocated buffer to read the samples into! This function will stop reading when this
    ///   buffer is full, or when the sound runs out of unread samples.
    /// * `sample_count` - The maximum number of samples to read, this should be less than or equal to the number of
    ///   samples the sampleBuffer can contain.
    ///
    /// see also [`sound_read_samples`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// // Half of the samples won't be kept in the buffer (0.5 instead of 1.0)
    /// let stream_sound = Sound::create_stream(0.5).
    ///                            expect("A sound stream should be created");
    ///
    /// let mut samples: Vec<f32> = vec![0.0; 48000];
    /// for i in 0..48000 {
    ///     samples[i] = (i as f32 / 48000.0).sin();
    /// }
    /// stream_sound.write_samples(samples.as_slice(), Some(48000));
    ///
    /// assert_eq!(stream_sound.get_unread_samples(), 24000);
    ///
    /// let mut read_samples: Vec<f32> = vec![0.0; 48000];
    /// let read_count = stream_sound.read_samples(read_samples.as_mut_slice(), Some(48000));
    /// assert_eq!(read_count, 24000);
    ///
    /// let read_count = stream_sound.read_samples(read_samples.as_mut_slice(), Some(48000));
    /// assert_eq!(read_count, 0);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn read_samples(&self, out_arr_samples: &mut [f32], sample_count: Option<u64>) -> u64 {
        let sample_count = sample_count.unwrap_or(out_arr_samples.len() as u64);
        unsafe { sound_read_samples(self.0.as_ptr(), out_arr_samples.as_mut_ptr(), sample_count) }
    }

    /// Only works if this Sound is a stream type! This writes a number of audio samples to the sample buffer, and
    /// samples should be between -1 and +1. Streams are stored as ring buffers of a fixed size, so writing beyond the
    /// capacity of the ring buffer will overwrite the oldest samples.
    ///
    /// StereoKit uses 48,000 samples per second of audio.
    ///
    /// This variation of the method bypasses marshalling memory into C#, so it is the most optimal way to copy sound
    /// data if your source is already in native memory!
    /// <https://stereokit.net/Pages/StereoKit/Sound/WriteSamples.html>
    /// * `in_arr_samples` - An array of audio samples, where each sample is between -1 and +1.
    /// * `sample_count` - You can use this to write only a subset of the samples in the array, rather than the entire
    ///   array!
    ///
    /// see also [`sound_write_samples`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// // Half of the samples won't be kept in the buffer (0.5 instead of 1.0)
    /// let stream_sound = Sound::create_stream(1.0).
    ///                            expect("A sound stream should be created");
    ///
    /// let mut samples: Vec<f32> = vec![0.0; 48000];
    /// for i in 0..48000 {
    ///     samples[i] = (i as f32 / 48000.0).sin();
    /// }
    /// stream_sound.write_samples(samples.as_slice(), Some(48000));
    ///
    /// assert_eq!(stream_sound.get_unread_samples(), 48000);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn write_samples(&self, in_arr_samples: &[f32], sample_count: Option<u64>) {
        let sample_count = sample_count.unwrap_or(in_arr_samples.len() as u64);
        unsafe { sound_write_samples(self.0.as_ptr(), in_arr_samples.as_ptr(), sample_count) };
    }

    /// The id of this sound
    /// <https://stereokit.net/Pages/StereoKit/Sound/Id.html>
    ///
    /// see also [`sound_get_id`]
    /// see example in [`Sound::id`]
    pub fn get_id(&self) -> &str {
        unsafe { CStr::from_ptr(sound_get_id(self.0.as_ptr())) }.to_str().unwrap_or_default()
    }

    /// This is the current position of the playback cursor, measured in samples from the start of the audio data.
    /// <https://stereokit.net/Pages/StereoKit/Sound/CursorSamples.html>
    ///
    /// see also [`sound_cursor_samples`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// let mut samples: Vec<f32> = vec![0.0; 48000];
    /// for i in 0..48000 {
    ///     samples[i] = (i as f32 / 48000.0).sin();
    /// }
    /// let sound = Sound::from_samples(&samples)
    ///                     .expect("Sound should be created from samples");
    ///
    /// assert_eq!(sound.get_cursor_samples(), 0);
    ///
    /// let sound_inst = sound.play([0.0, 0.0, -0.5], Some(0.5));
    /// sound_inst.stop();
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == 1 {
    ///         assert_eq!(sound.get_total_samples(), 48000);
    ///         assert_eq!(sound.get_cursor_samples(), 0);
    ///         sound.write_samples(&samples, None);
    ///     } else if iter == 2 {
    ///        assert_eq!(sound.get_cursor_samples(), 0);
    ///     }
    /// );
    ///
    pub fn get_cursor_samples(&self) -> u64 {
        unsafe { sound_cursor_samples(self.0.as_ptr()) }
    }

    /// This will return the total length of the sound in seconds.
    /// <https://stereokit.net/Pages/StereoKit/Sound/Duration.html>
    ///
    /// see also [`sound_duration`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// let mut samples: Vec<f32> = vec![0.0; 48000];
    /// for i in 0..48000 {
    ///     samples[i] = (i as f32 / 48000.0).sin();
    /// }
    /// let sound = Sound::from_samples(&samples)
    ///                     .expect("Sound should be created from samples");
    /// assert_eq!(sound.get_duration(), 1.0);
    ///
    /// let sound_file = Sound::from_file("sounds/no.wav")
    ///                          .expect("Sound should be created from file");
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == 1 {
    ///         assert_eq!(sound_file.get_duration(), 1.4830834);
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_duration(&self) -> f32 {
        unsafe { sound_duration(self.0.as_ptr()) }
    }

    /// The sound's real-world loudness at 1 meter, in decibels! StereoKit measures the audio data's loudness, so the
    /// value you declare here is the loudness you get - the waveform is the *shape* of the sound, Decibels is how loud
    /// it is. Loudness then falls off physically with distance (-6dB per doubling), so louder things carry farther with
    /// no extra tuning.
    ///
    /// Some reference points: rustling leaves 20, a whisper 30, calm conversation 60, a vacuum cleaner at arm's length
    /// 75, a busy street corner 80 (the default), shouting up close 88, a rock concert 110, thunder from a nearby
    /// strike 120.
    /// <https://stereokit.net/Pages/StereoKit/Sound/Decibels.html>
    ///
    /// see also [`sound_get_decibels`]
    /// see example in [`Sound::decibels`]
    pub fn get_decibels(&self) -> f32 {
        unsafe { sound_get_decibels(self.0.as_ptr()) }
    }

    /// This will return the total number of audio samples used by the sound! StereoKit currently uses 48,000 samples
    /// per second for all audio. For stream sounds this is everything ever written. Against a playing
    /// [`SoundInst::get_cursor`], the difference is how much audio is queued ahead of that voice's playback.
    /// <https://stereokit.net/Pages/StereoKit/Sound/TotalSamples.html>
    ///
    /// see also [`sound_total_samples`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// let mut samples: Vec<f32> = vec![0.0; 48000];
    /// for i in 0..48000 {
    ///     samples[i] = (i as f32 / 48000.0).sin();
    /// }
    /// let sound = Sound::from_samples(&samples)
    ///                     .expect("Sound should be created from samples");
    /// assert_eq!(sound.get_total_samples(), 48000);
    ///
    /// let sound_file = Sound::from_file("sounds/no.wav")
    ///                          .expect("Sound should be created from file");
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == 1 {
    ///         assert_eq!(sound_file.get_duration(), 1.4830834);
    ///         // 1.4830834 * 48000 = 71188
    ///         assert_eq!(sound_file.get_total_samples(), 71188);
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_total_samples(&self) -> u64 {
        unsafe { sound_total_samples(self.0.as_ptr()) }
    }

    /// This is the maximum number of samples in the sound that are currently available for reading via ReadSamples!
    /// ReadSamples will reduce this number by the amount of samples read. Playback doesn't consume samples - playing
    /// voices each keep their own cursor, see [`SoundInst::get_cursor`].
    ///
    /// This is only really valid for Stream sounds, all other sound types will just return 0.
    /// <https://stereokit.net/Pages/StereoKit/Sound/UnreadSamples.html>
    ///
    /// see also [`sound_unread_samples`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// // Half of the samples won't be kept in the buffer (0.5 instead of 1.0)
    /// let stream_sound = Sound::create_stream(1.0).
    ///                            expect("A sound stream should be created");
    ///
    /// let mut samples: Vec<f32> = vec![0.0; 48000];
    /// for i in 0..48000 {
    ///     samples[i] = (i as f32 / 48000.0).sin();
    /// }
    /// stream_sound.write_samples(samples.as_slice(), Some(48000));
    ///
    /// assert_eq!(stream_sound.get_unread_samples(), 48000);
    ///
    /// let mut read_samples: Vec<f32> = vec![0.0; 48000];
    /// let read_count = stream_sound.read_samples(read_samples.as_mut_slice(), Some(48000));
    /// assert_eq!(read_count, 48000);
    /// assert_eq!(stream_sound.get_unread_samples(), 0);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_unread_samples(&self) -> u64 {
        unsafe { sound_unread_samples(self.0.as_ptr()) }
    }

    /// The channel format of this sound's data. Only Mono sounds spatialize - Stereo plays head-locked with its image
    /// intact, and Ambisonic1 is a world-fixed sound field that counter-rotates against the head.
    /// <https://stereokit.net/Pages/StereoKit/Sound/Channels.html>
    ///
    /// see also [`sound_get_channels`] [`SoundChannels`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::{Sound, SoundChannels};
    ///
    /// // A file loads as whatever channel format it contains.
    /// let sound = Sound::from_file("sounds/no.wav").expect("no.wav should load");
    /// # system::Assets::block_until(&sound, stereokit_rust::system::AssetState::Loaded);
    /// let channels = sound.get_channels();
    /// assert!(channels == SoundChannels::Mono || channels == SoundChannels::Stereo);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_channels(&self) -> SoundChannels {
        unsafe { sound_get_channels(self.0.as_ptr()) }
    }

    /// Sounds loaded from file decode asynchronously - this tells you where that's at! Playing is safe at any point: a
    /// Play while still Loading is held until the data lands, then catches up as if it had started on time. Negative
    /// states mean the load failed, and any held plays die quietly.
    /// <https://stereokit.net/Pages/StereoKit/Sound/AssetState.html>
    ///
    /// see also [`sound_asset_state`] [`AssetState`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{sound::Sound, system::AssetState};
    ///
    /// // A sound created from samples is immediately loaded.
    /// let samples = vec![0.0f32; 480];
    /// let sound = Sound::from_samples(&samples).expect("sound should be created");
    /// assert_eq!(sound.get_asset_state(), AssetState::Loaded);
    ///
    /// // A file decodes asynchronously, so block until it is ready.
    /// let file_sound = Sound::from_file("sounds/no.wav").expect("no.wav should load");
    /// system::Assets::block_until(&file_sound, AssetState::Loaded);
    /// assert_eq!(file_sound.get_asset_state(), AssetState::Loaded);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_asset_state(&self) -> AssetState {
        unsafe { sound_asset_state(self.0.as_ptr()) }
    }

    /// A default click sound that lasts for 300ms. It’s a procedurally generated sound based on a mouse press, with
    /// extra low frequencies in it.
    /// <https://stereokit.net/Pages/StereoKit/Sound/Click.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// let click_sound = Sound::click();
    /// assert_eq!(click_sound.get_id(), "default/sound_click");
    /// # system::Assets::block_for_priority(i32::MAX);
    ///
    /// let click_sound_inst = click_sound.play([0.0, 0.0, -0.5], Some(0.5));
    ///
    /// number_of_steps = 100;
    /// let mut was_played = false;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     was_played |= click_sound_inst.is_playing();
    /// );
    /// assert!(was_played);
    /// sk::Sk::shutdown();
    /// ```
    pub fn click() -> Self {
        let cstr_id = CString::new("default/sound_click").unwrap_or_default();
        Sound(NonNull::new(unsafe { sound_find(cstr_id.as_ptr()) }).expect("default/sound_click should be found!"))
    }

    /// A default unclick sound that lasts for 300ms. It’s a procedurally generated sound based on a mouse press, with
    /// extra low frequencies in it.
    /// <https://stereokit.net/Pages/StereoKit/Sound/Unclick.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// let unclick_sound = Sound::unclick();
    /// assert_eq!(unclick_sound.get_id(), "default/sound_unclick");
    /// # system::Assets::block_for_priority(i32::MAX);
    ///
    /// let unclick_sound_inst = unclick_sound.play([0.0, 0.0, -0.5], Some(0.5));
    ///
    /// number_of_steps = 100;
    /// let mut was_played = false;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     was_played |= unclick_sound_inst.is_playing();
    /// );
    /// assert!(was_played);
    /// sk::Sk::shutdown();
    /// ```
    pub fn unclick() -> Self {
        let cstr_id = CString::new("default/sound_unclick").unwrap_or_default();
        Sound(NonNull::new(unsafe { sound_find(cstr_id.as_ptr()) }).expect("default/sound_unclick should be found!"))
    }

    /// A default grab sound
    /// <https://stereokit.net/Pages/StereoKit/Sound.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// let grab_sound = Sound::grab();
    /// assert_eq!(grab_sound.get_id(), "default/sound_grab");
    /// # system::Assets::block_for_priority(i32::MAX);
    ///
    /// let grab_sound_inst = grab_sound.play([0.0, 0.0, -0.5], Some(0.5));
    ///
    /// number_of_steps = 100;
    /// let mut was_played = false;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     was_played |= grab_sound_inst.is_playing();
    /// );
    /// assert!(was_played);
    /// sk::Sk::shutdown();
    /// ```
    pub fn grab() -> Self {
        let cstr_id = CString::new("default/sound_grab").unwrap_or_default();
        Sound(NonNull::new(unsafe { sound_find(cstr_id.as_ptr()) }).expect("default/sound_grab should be found!"))
    }

    /// A default ungrab sound
    /// <https://stereokit.net/Pages/StereoKit/Sound.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// let ungrab_sound = Sound::ungrab();
    /// assert_eq!(ungrab_sound.get_id(), "default/sound_ungrab");
    /// # system::Assets::block_for_priority(i32::MAX);
    ///
    /// let ungrab_sound_inst = ungrab_sound.play([0.0, 0.0, -0.5], Some(0.5));
    ///
    /// number_of_steps = 100;
    /// let mut was_played = false;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     was_played |= ungrab_sound_inst.is_playing();
    /// );
    /// assert!(was_played);
    /// sk::Sk::shutdown();
    /// ```
    pub fn ungrab() -> Self {
        let cstr_id = CString::new("default/sound_ungrab").unwrap_or_default();
        Sound(NonNull::new(unsafe { sound_find(cstr_id.as_ptr()) }).expect("default/sound_ungrab should be found!"))
    }
}

/// This represents a play instance of a Sound! You can get one when you call Sound::play(). This allows you to do things
/// like cancel a piece of audio early, or change the volume and position of it as it’s playing.
/// <https://stereokit.net/Pages/StereoKit/SoundInst.html>
///
/// see also: [`Sound`]
/// /// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec3, Matrix}, mesh::Mesh, material::Material,
///                      sound::Sound, util::named_colors};
///
/// let sphere = Mesh::generate_sphere(0.5, None);
/// let material = Material::pbr().tex_file_copy("textures/sound.jpeg", true, None)
///                    .expect("sound.jpeg should be there");
/// let mut position1 = Vec3::new(-0.5, 0.0, 0.5);
/// let mut position2 = Vec3::new( 0.5, 0.0, 0.5);
///
/// let mut plane_sound = Sound::from_file("sounds/plane_engine.mp3")
///                           .expect("no.wav should be there");
/// plane_sound.id("sound_plane").decibels(70.0);
/// let mut plane_sound_inst1 = plane_sound.play(position1, Some(1.0));
/// let mut plane_sound_inst2 = plane_sound.play(position2, Some(1.0));
///
/// # if cfg!(not(feature = "test-xr-mode")) {
/// number_of_steps = 150;
/// filename_scr = "screenshots/sound_inst.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     let transform1 = Matrix::t(position1);
///     let transform2 = Matrix::t(position2);
///     sphere.draw(&material, transform1, Some(named_colors::PINK.into()), None  );
///     sphere.draw(&material, transform2, Some(named_colors::LIGHT_GREEN.into()), None  );
///
///     if iter == 0 {
///         assert!(plane_sound_inst1.is_playing());
///         assert!(plane_sound_inst2.is_playing());
///         position1 = Vec3::new(-0.3, 0.0, 0.3);
///         plane_sound_inst1
///             .position(position1)
///             .volume(0.5);
///     } else if iter == 50 {
///         assert!(plane_sound_inst1.is_playing());
///         plane_sound_inst1.stop();
///         assert!(!plane_sound_inst1.is_playing());
///         position2 = Vec3::new(0.3, 0.0, 0.3);
///         plane_sound_inst2 = plane_sound.play(position2, Some(1.0));
///         assert!(plane_sound_inst2.is_playing());
///     } else if iter == number_of_steps {
///         assert!(!plane_sound_inst1.is_playing());
///         assert!(plane_sound_inst2.is_playing());
///         plane_sound_inst1.stop();
///         plane_sound_inst2.stop();
///         assert!(!plane_sound_inst2.is_playing()); // delay
///     }
/// );
/// # } sk::Sk::shutdown();
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sound_inst.jpeg" alt="screenshot" width="200">
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SoundInst {
    pub _id: u16,
    pub _slot: i16,
}

unsafe extern "C" {
    pub fn sound_inst_stop(sound_inst: SoundInst);
    pub fn sound_inst_is_playing(sound_inst: SoundInst) -> Bool32T;
    pub fn sound_inst_set_pos(sound_inst: SoundInst, pos: Vec3);
    pub fn sound_inst_get_pos(sound_inst: SoundInst) -> Vec3;
    pub fn sound_inst_set_volume(sound_inst: SoundInst, volume_pct: f32);
    pub fn sound_inst_get_volume(sound_inst: SoundInst) -> f32;
    pub fn sound_inst_set_pitch(sound_inst: SoundInst, pitch_mult: f32);
    pub fn sound_inst_get_pitch(sound_inst: SoundInst) -> f32;
    pub fn sound_inst_set_spread(sound_inst: SoundInst, spread_pct: f32);
    pub fn sound_inst_get_spread(sound_inst: SoundInst) -> f32;
    pub fn sound_inst_set_cutoff(sound_inst: SoundInst, cutoff_hz: f32);
    pub fn sound_inst_set_paused(sound_inst: SoundInst, paused: Bool32T);
    pub fn sound_inst_get_paused(sound_inst: SoundInst) -> Bool32T;
    pub fn sound_inst_seek(sound_inst: SoundInst, sample: u64);
    pub fn sound_inst_get_cursor(sound_inst: SoundInst) -> u64;
    pub fn sound_inst_set_shape(sound_inst: SoundInst, in_arr_points: *const Vec3, point_count: i32, radius: f32);
    pub fn sound_inst_get_intensity(sound_inst: SoundInst) -> f32;
}

impl SoundInst {
    /// This stops the sound early if it’s still playing. consume the SoundInst as it will not be playable again.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/Stop.html>
    ///
    /// see also [`sound_inst_stop`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// let plane_sound = Sound::from_file("sounds/plane_engine.mp3").
    ///                           expect("A sound should be created");
    /// let plane_sound_inst = plane_sound.play([0.0, 0.0, 0.0], Some(1.0));
    ///
    /// number_of_steps = 400;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == 10 {
    ///         plane_sound_inst.stop();
    ///         assert!(!plane_sound_inst.is_playing());
    ///     } else if iter == 399 {
    ///         assert!(!plane_sound_inst.is_playing());
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn stop(self) {
        unsafe { sound_inst_stop(self) }
    }

    /// The 3D position in world space this sound instance is currently playing at. If this instance is no longer
    /// valid, the position will be at zero.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/Position.html>
    ///
    /// see also [`sound_inst_set_pos`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, sound::Sound};
    ///
    /// let mut position = Vec3::new(-2.5, 0.0, 0.5);
    ///
    /// let mut plane_sound = Sound::from_file("sounds/plane_engine.mp3").
    ///                           expect("A sound should be created");
    /// plane_sound.id("sound_plane").decibels(70.0);
    ///
    /// let mut plane_sound_inst = plane_sound.play(position, None);
    /// assert_eq!(plane_sound_inst.get_position(), position);
    ///
    /// number_of_steps = 150;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     position += Vec3::new(0.0001, 0.0, 0.0);
    ///     plane_sound_inst.position(position);
    /// );
    /// plane_sound_inst.stop();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn position(&mut self, at: impl Into<Vec3>) -> &mut Self {
        unsafe { sound_inst_set_pos(*self, at.into()) }
        self
    }

    /// The volume multiplier of this Sound instance! Typically 0-1, where 0 is silent, and 1 is full volume. Values
    /// above 1 amplify, and negatives clamp to 0.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/Volume.html>
    ///
    /// see also [`sound_inst_set_volume`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, sound::Sound, system::Assets};
    ///
    /// let position = Vec3::new(0.0, 0.0, 0.5);
    /// let mut volume = 0.0;
    ///
    /// let mut plane_sound = Sound::from_file("sounds/plane_engine.mp3").
    ///                           expect("A sound should be created");
    /// plane_sound.id("sound_plane");
    /// Assets::block_for_priority(i32::MAX);
    ///
    /// let mut plane_sound_inst = plane_sound.play(position, None);
    /// plane_sound_inst.volume(0.005);
    ///
    /// number_of_steps = 150;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     volume += 0.01;
    ///     plane_sound_inst.volume(volume);
    /// );
    /// plane_sound_inst.stop();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn volume(&mut self, volume: f32) -> &mut Self {
        unsafe { sound_inst_set_volume(*self, volume) }
        self
    }

    /// Playback rate multiplier, clamped to 0.25-4. 1 is normal speed, 2 is twice as fast and an octave up. Animatable
    /// while playing.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/Pitch.html>
    ///
    /// see also [`sound_inst_set_pitch`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// let sound = Sound::from_file("sounds/plane_engine.mp3").expect("A sound should be created");
    /// let mut inst = sound.play([0.0, 0.0, -1.0], Some(1.0));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == 0 {
    ///         assert_eq!(inst.get_pitch(), 1.0); // default
    ///         inst.pitch(1.5); // speed up, one fifth up
    ///     } else if iter == 1 {
    ///         assert_eq!(inst.get_pitch(), 1.5);
    ///     } else if iter == 2 {
    ///         inst.stop();
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn pitch(&mut self, pitch: f32) -> &mut Self {
        unsafe { sound_inst_set_pitch(*self, pitch) }
        self
    }

    /// Apparent size of the source, 0-1. 0 is a point in space, 1 fills the whole sound field. Shaped emitters compute
    /// this themselves, treating a set value as their minimum.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/Spread.html>
    ///
    /// see also [`sound_inst_set_spread`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// let sound = Sound::from_file("sounds/plane_engine.mp3").expect("A sound should be created");
    /// let mut inst = sound.play([0.0, 0.0, -1.0], Some(1.0));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == 0 {
    ///         inst.spread(0.5); // widen the source to fill more of the field
    ///     } else if iter == 1 {
    ///         assert_eq!(inst.get_spread(), 0.5);
    ///     } else if iter == 2 {
    ///         inst.stop();
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn spread(&mut self, spread: f32) -> &mut Self {
        unsafe { sound_inst_set_spread(*self, spread) }
        self
    }

    /// Overrides the voice's low-pass filter cutoff in Hz, replacing the automatic distance model. 0 hands control back
    /// to the distance model.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/SetCutoff.html>
    /// * `cutoff_hz` - Low-pass cutoff frequency in Hz, 0 for automatic.
    ///
    /// see also [`sound_inst_set_cutoff`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// let sound = Sound::from_file("sounds/plane_engine.mp3").expect("A sound should be created");
    /// let mut inst = sound.play([0.0, 0.0, -1.0], Some(1.0));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == 0 {
    ///         // Muffle the sound with a low cutoff.
    ///         inst.set_cutoff(800.0);
    ///     } else if iter == 1 {
    ///         // Hand control back to the automatic distance model.
    ///         inst.set_cutoff(0.0);
    ///     } else if iter == 2 {
    ///         inst.stop();
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_cutoff(&mut self, cutoff_hz: f32) -> &mut Self {
        unsafe { sound_inst_set_cutoff(*self, cutoff_hz) }
        self
    }

    /// Pause and resume this voice. A paused voice keeps its place and stays alive until stopped or stolen.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/Paused.html>
    ///
    /// see also [`sound_inst_set_paused`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// let sound = Sound::from_file("sounds/plane_engine.mp3").expect("A sound should be created");
    /// let mut inst = sound.play([0.0, 0.0, -1.0], Some(1.0));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == 0 {
    ///         assert!(!inst.get_paused()); // playing
    ///         inst.paused(true);           // pause it
    ///     } else if iter == 1 {
    ///         assert!(inst.get_paused());
    ///         inst.paused(false);          // resume
    ///     } else if iter == 2 {
    ///         assert!(!inst.get_paused());
    ///         inst.stop();
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn paused(&mut self, paused: bool) -> &mut Self {
        unsafe { sound_inst_set_paused(*self, paused as Bool32T) }
        self
    }

    /// Jump this voice's playback to a sample position. Only works for fully in-memory sounds! Files up to ~10 seconds
    /// decode fully into memory on load, while longer files stream, and stream playback reads forward only.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/Seek.html>
    /// * `sample` - Sample index to jump to, clamped to the sound's length.
    ///
    /// see also [`sound_inst_seek`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// // A short, fully in-memory sound.
    /// let samples = vec![0.0f32; 4800]; // 0.1s
    /// let sound = Sound::from_samples(&samples).expect("sound should be created");
    ///
    /// let mut inst = sound.play([0.0, 0.0, 0.0], Some(1.0));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == 0 {
    ///         // Jump to the middle of the sound.
    ///         inst.seek(2400);
    ///         inst.stop();
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn seek(&mut self, sample: u64) -> &mut Self {
        unsafe { sound_inst_seek(*self, sample) }
        self
    }

    /// Gives this voice a polyline emitter shape! The emitter follows the listener along the shape - position becomes
    /// the closest point, apparent size grows as the shape fills more of the view, and the sound goes fully diffuse
    /// inside it. Great for streams, wind lines, and shorelines. Points are copied, max 32.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/SetShape.html>
    /// * `points` - The polyline's points, in world space.
    /// * `radius` - Radius of the polyline's tube, in meters.
    ///
    /// see also [`sound_inst_set_shape`] [`SoundInst::set_shape_sphere`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, sound::Sound};
    ///
    /// let sound = Sound::from_file("sounds/plane_engine.mp3").expect("A sound should be created");
    /// let mut inst = sound.play([0.0, 0.0, 0.0], Some(1.0));
    ///
    /// // A wind line along two points.
    /// let points = vec![Vec3::new(-2.0, 0.0, -1.0), Vec3::new(2.0, 0.0, -1.0)];
    /// inst.set_shape(&points, 0.3);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == 10 { inst.stop(); }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_shape(&mut self, points: &[Vec3], radius: f32) -> &mut Self {
        unsafe { sound_inst_set_shape(*self, points.as_ptr(), points.len() as i32, radius) }
        self
    }

    /// Gives this voice a sphere emitter shape! The emitter follows the listener around the sphere's surface, growing to
    /// fully diffuse inside it. Great for wind volumes and rain areas.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/SetShape.html>
    /// * `center` - The sphere's center, in world space.
    /// * `radius` - The sphere's radius, in meters.
    ///
    /// see also [`sound_inst_set_shape`] [`SoundInst::set_shape`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, sound::Sound};
    ///
    /// let sound = Sound::from_file("sounds/plane_engine.mp3").expect("A sound should be created");
    /// let mut inst = sound.play([0.0, 0.0, 0.0], Some(1.0));
    ///
    /// // A spherical rain volume around the listener.
    /// inst.set_shape_sphere(Vec3::new(0.0, 1.5, 0.0), 3.0);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == 10 { inst.stop(); }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_shape_sphere(&mut self, center: Vec3, radius: f32) -> &mut Self {
        unsafe { sound_inst_set_shape(*self, &center, 1, radius) }
        self
    }

    /// The 3D position in world space this sound instance is currently playing at. If this instance is no longer
    /// valid, the position will be at zero.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/Position.html>
    ///
    /// see also [`sound_inst_get_pos`]
    /// see example in [`SoundInst::position`]
    pub fn get_position(&self) -> Vec3 {
        unsafe { sound_inst_get_pos(*self) }
    }

    /// The volume multiplier of this Sound instance! Typically 0-1, where 0 is silent, and 1 is full volume. Values
    /// above 1 amplify, and negatives clamp to 0.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/Volume.html>
    ///
    /// see also [`sound_inst_get_volume`]
    /// see example in [`SoundInst::volume`]
    pub fn get_volume(&self) -> f32 {
        unsafe { sound_inst_get_volume(*self) }
    }

    /// Playback rate multiplier, clamped to 0.25-4. 1 is normal speed, 2 is twice as fast and an octave up. Animatable
    /// while playing.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/Pitch.html>
    ///
    /// see also [`sound_inst_get_pitch`]
    /// see example in [`SoundInst::pitch`]
    pub fn get_pitch(&self) -> f32 {
        unsafe { sound_inst_get_pitch(*self) }
    }

    /// Apparent size of the source, 0-1. 0 is a point in space, 1 fills the whole sound field. Shaped emitters compute
    /// this themselves, treating a set value as their minimum.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/Spread.html>
    ///
    /// see also [`sound_inst_get_spread`]
    /// see example in [`SoundInst::spread`]
    pub fn get_spread(&self) -> f32 {
        unsafe { sound_inst_get_spread(*self) }
    }

    /// Is this voice currently paused? A paused voice keeps its place and stays alive until stopped or stolen.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/Paused.html>
    ///
    /// see also [`sound_inst_get_paused`]
    /// see example in [`SoundInst::paused`]
    pub fn get_paused(&self) -> bool {
        unsafe { sound_inst_get_paused(*self) != 0 }
    }

    /// This voice's playback position in source samples. For stream sounds this is an absolute position in the stream,
    /// so [`Sound::get_total_samples`] - Cursor is how much audio is queued ahead of this voice. Only fully in-memory
    /// sounds can Seek, streams read forward only.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/Cursor.html>
    ///
    /// see also [`sound_inst_get_cursor`]
    /// see example in [`SoundInst::seek`]
    pub fn get_cursor(&self) -> u64 {
        unsafe { sound_inst_get_cursor(*self) }
    }

    /// The maximum intensity of the sound data since the last frame, as a value from 0-1. This is unaffected by its 3d
    /// position or volume settings, and is straight from the audio file's data.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/Intensity.html>
    ///
    /// see also [`sound_inst_get_intensity`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// let mut plane_sound = Sound::from_file("sounds/plane_engine.mp3").
    ///                           expect("A sound should be created");
    /// plane_sound.id("sound_plane").decibels(70.0);
    ///
    /// let mut plane_sound_inst = plane_sound.play([0.0, 0.0, 0.0], Some(1.0));
    /// plane_sound_inst.volume(1.0);
    ///
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     assert_eq!(plane_sound_inst.get_intensity(), 0.0);
    ///     plane_sound_inst.stop();
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_intensity(&self) -> f32 {
        unsafe { sound_inst_get_intensity(*self) }
    }

    /// Is this Sound instance currently playing? For streaming assets, this will be true even if they don’t have any
    /// new data in them, and they’re just idling at the end of their data.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/IsPlaying.html>
    ///
    /// see also [`sound_inst_is_playing`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// let plane_sound = Sound::from_file("sounds/plane_engine.mp3").
    ///                           expect("A sound should be created");
    /// let plane_sound_inst = plane_sound.play([0.0, 0.0, 0.0], Some(1.0));
    ///
    /// number_of_steps = 300;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == 10 {
    ///         assert!(plane_sound_inst.is_playing());
    ///         plane_sound_inst.stop();
    ///     } else if iter == number_of_steps  {
    ///         assert!(!plane_sound_inst.is_playing());
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn is_playing(&self) -> bool {
        unsafe { sound_inst_is_playing(*self) != 0 }
    }
}
