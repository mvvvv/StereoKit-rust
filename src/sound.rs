use crate::{
    maths::{Bool32T, Vec3},
    system::IAsset,
    StereoKitError,
};

use std::{
    ffi::{CStr, CString},
    path::Path,
    ptr::NonNull,
};

/// This class represents a sound effect! Excellent for blips and bloops and little clips that you might play around
/// your scene. Right now, this supports .wav, .mp3, and procedurally generated noises!
///
/// On HoloLens 2, sounds are automatically processed on the HPU, freeing up the CPU for more of your app’s code. To
/// simulate this same effect on your development PC, you need to enable spatial sound on your audio endpoint. To do
/// this, right click the speaker icon in your system tray, navigate to “Spatial sound”, and choose “Windows Sonic for
/// Headphones.” For more information, visit https://docs.microsoft.com/en-us/windows/win32/coreaudio/spatial-sound
///<https://stereokit.net/Pages/StereoKit/Sound.html>
/// ## Examples
///
///
#[repr(C)]
#[derive(Debug)]
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
#[repr(C)]
#[derive(Debug)]
pub struct _SoundT {
    _unused: [u8; 0],
}
pub type SoundT = *mut _SoundT;

extern "C" {
    pub fn sound_find(id: *const ::std::os::raw::c_char) -> SoundT;
    pub fn sound_set_id(sound: SoundT, id: *const ::std::os::raw::c_char);
    pub fn sound_get_id(sound: SoundT) -> *const ::std::os::raw::c_char;
    pub fn sound_create(filename_utf8: *const ::std::os::raw::c_char) -> SoundT;
    pub fn sound_create_stream(buffer_duration: f32) -> SoundT;
    pub fn sound_create_samples(in_arr_samples_at_48000s: *const f32, sample_count: u64) -> SoundT;
    pub fn sound_generate(
        audio_generator: Option<unsafe extern "C" fn(sample_time: f32) -> f32>,
        duration: f32,
    ) -> SoundT;
    pub fn sound_write_samples(sound: SoundT, in_arr_samples: *const f32, sample_count: u64);
    pub fn sound_read_samples(sound: SoundT, out_arr_samples: *mut f32, sample_count: u64) -> u64;
    pub fn sound_unread_samples(sound: SoundT) -> u64;
    pub fn sound_total_samples(sound: SoundT) -> u64;
    pub fn sound_cursor_samples(sound: SoundT) -> u64;
    pub fn sound_play(sound: SoundT, at: Vec3, volume: f32) -> SoundInst;
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
}

impl Sound {
    /// Create a sound used for streaming audio in or out! This is useful for things like reading from a microphone
    /// stream, or playing audio from a source streaming over the network, or even procedural sounds that are generated on the fly!
    /// Use stream sounds with the WriteSamples and ReadSamples functions.
    /// <https://stereokit.net/Pages/StereoKit/Sound/CreateStream.html>
    ///
    /// see also [`crate::sound::sound_create_stream`]
    pub fn create_stream(stream_buffer_duration: f32) -> Result<Sound, StereoKitError> {
        Ok(Sound(
            NonNull::new(unsafe { sound_create_stream(stream_buffer_duration) })
                .ok_or(StereoKitError::SoundCreate("create_stream failed".into()))?,
        ))
    }

    /// Loads a sound effect from file! Currently, StereoKit supports .wav and .mp3 files. Audio is converted to mono.
    /// <https://stereokit.net/Pages/StereoKit/Sound/FromFile.html>
    ///
    /// see also [`crate::sound::sound_create`]
    pub fn from_file(file_utf8: impl AsRef<Path>) -> Result<Sound, StereoKitError> {
        let path_buf = file_utf8.as_ref().to_path_buf();
        let c_str = CString::new(path_buf.clone().to_str().ok_or(StereoKitError::SoundFile(path_buf.clone()))?)?;

        Ok(Sound(
            NonNull::new(unsafe { sound_create(c_str.as_ptr()) }).ok_or(StereoKitError::SoundFile(path_buf))?,
        ))
    }

    /// This function will create a sound from an array of samples. Values should range from -1 to +1, and there should
    /// be 48,000 values per second of audio.
    /// <https://stereokit.net/Pages/StereoKit/Sound/FromSamples.html>
    ///
    /// see also [`crate::sound::sound_create_samples`]
    pub fn from_samples(in_arr_samples_at_48000s: &[f32]) -> Result<Sound, StereoKitError> {
        Ok(Sound(
            NonNull::new(unsafe {
                sound_create_samples(in_arr_samples_at_48000s.as_ptr(), in_arr_samples_at_48000s.len() as u64)
            })
            .ok_or(StereoKitError::SoundCreate("from_samples failed".into()))?,
        ))
    }

    /// This function will generate a sound from a function you provide! The function is called once for each sample in
    /// the duration. As an example, it may be called 48,000 times for each second of duration.
    /// <https://stereokit.net/Pages/StereoKit/Sound/Generate.html>
    ///
    /// see also [`crate::sound::sound_generate`]
    pub fn generate(generator: unsafe extern "C" fn(f32) -> f32, duration: f32) -> Result<Sound, StereoKitError> {
        Ok(Sound(
            NonNull::new(unsafe { sound_generate(Some(generator), duration) })
                .ok_or(StereoKitError::SoundCreate("sound_generate failed".into()))?,
        ))
    }

    /// ooks for a Sound asset that’s already loaded, matching the given id!
    /// <https://stereokit.net/Pages/StereoKit/Sound/Find.html>
    ///
    /// see also [`crate::sound::sound_find`]
    pub fn find<S: AsRef<str>>(id: S) -> Result<Sound, StereoKitError> {
        let cstr_id = CString::new(id.as_ref())?;
        Ok(Sound(
            NonNull::new(unsafe { sound_find(cstr_id.as_ptr()) })
                .ok_or(StereoKitError::SoundFind(id.as_ref().to_string()))?,
        ))
    }

    /// A default click sound that lasts for 300ms. It’s a procedurally generated sound based on a mouse press, with
    /// extra low frequencies in it.
    /// <https://stereokit.net/Pages/StereoKit/Sound/Click.html>
    pub fn click() -> Self {
        let cstr_id = CString::new("default/sound_click").unwrap();
        Sound(NonNull::new(unsafe { sound_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// A default unclick sound that lasts for 300ms. It’s a procedurally generated sound based on a mouse press, with
    /// extra low frequencies in it.
    /// <https://stereokit.net/Pages/StereoKit/Sound/Unclick.html>
    pub fn unclick() -> Self {
        let cstr_id = CString::new("default/sound_unclick").unwrap();
        Sound(NonNull::new(unsafe { sound_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// A default grab sound
    /// <https://stereokit.net/Pages/StereoKit/Sound.html>
    pub fn grab() -> Self {
        let cstr_id = CString::new("default/sound_grab").unwrap();
        Sound(NonNull::new(unsafe { sound_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// A default ungrab sound
    /// <https://stereokit.net/Pages/StereoKit/Sound.html>
    pub fn ungrab() -> Self {
        let cstr_id = CString::new("default/sound_ungrab").unwrap();
        Sound(NonNull::new(unsafe { sound_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// Plays the sound at the 3D location specified, using the volume parameter as an additional volume control option!
    /// Sound volume falls off from 3D location, and can also indicate direction and location through spatial audio
    /// cues. So make sure the position is where you want people to think it’s from! Currently, if this sound is playing
    /// somewhere else, it’ll be canceled, and moved to this location.
    /// <https://stereokit.net/Pages/StereoKit/Sound/Play.html>
    /// * volume - if None will have default value of 1.0
    ///
    /// see also [`stereokit::StereoKitDraw::sound_play`]
    pub fn play(&self, at: impl Into<Vec3>, volume: Option<f32>) -> SoundInst {
        let volume = volume.unwrap_or(1.0);
        unsafe { sound_play(self.0.as_ptr(), at.into(), volume) }
    }

    /// This will read samples from the sound stream, starting from the first unread sample. Check UnreadSamples for how
    /// many samples are available to read.
    /// <https://stereokit.net/Pages/StereoKit/Sound/ReadSamples.html>
    ///
    /// see also [`stereokit::StereoKitDraw::sound_read_samples`]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn read_samples(&self, out_arr_samples: *mut f32, sample_count: u64) -> u64 {
        unsafe { sound_read_samples(self.0.as_ptr(), out_arr_samples, sample_count) }
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
    ///
    /// see also [`stereokit::StereoKitDraw::sound_write_samples`]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn write_samples(&self, in_arr_samples: *const f32, sample_count: u64) {
        unsafe { sound_write_samples(self.0.as_ptr(), in_arr_samples, sample_count) };
    }

    /// sets the unique identifier of this asset resource! This can be helpful for debugging,
    /// managing your assets, or finding them later on!
    ///<https://stereokit.net/Pages/StereoKit/Sound/Id.html>
    ///
    /// see also [`crate::sound::sound_set_id`]
    pub fn id<S: AsRef<str>>(&mut self, id: S) -> &mut Self {
        let cstr_id = CString::new(id.as_ref()).unwrap();
        unsafe { sound_set_id(self.0.as_ptr(), cstr_id.as_ptr()) };
        self
    }

    /// The id of this sound
    /// <https://stereokit.net/Pages/StereoKit/Sound/Id.html>
    ///
    /// see also [`crate::sound::sound_get_id`]
    pub fn get_id(&self) -> &str {
        unsafe { CStr::from_ptr(sound_get_id(self.0.as_ptr())) }.to_str().unwrap()
    }

    /// This is the current position of the playback cursor, measured in samples from the start of the audio data.
    /// <https://stereokit.net/Pages/StereoKit/Sound/CursorSamples.html>
    ///
    /// see also [`crate::sound::sound_cursor_samples`]
    pub fn get_cursor_samples(&self) -> u64 {
        unsafe { sound_cursor_samples(self.0.as_ptr()) }
    }

    /// This will return the total length of the sound in seconds.
    /// <https://stereokit.net/Pages/StereoKit/Sound/Duration.html>
    ///
    /// see also [`crate::sound::sound_duration`]
    pub fn get_duration(&self) -> f32 {
        unsafe { sound_duration(self.0.as_ptr()) }
    }

    /// This will return the total number of audio samples used by the sound! StereoKit currently uses 48,000 samples
    /// per second for all audio.
    /// <https://stereokit.net/Pages/StereoKit/Sound/TotalSamples.html>
    ///
    /// see also [`crate::sound::sound_total_samples`]
    pub fn get_total_samples(&self) -> u64 {
        unsafe { sound_total_samples(self.0.as_ptr()) }
    }

    /// This is the maximum number of samples in the sound that are currently available for reading via ReadSamples!
    /// ReadSamples will reduce this number by the amount of samples read. This is only really valid for Stream
    /// sounds, all other sound types will just return 0.
    /// <https://stereokit.net/Pages/StereoKit/Sound/UnreadSamples.html>
    ///
    /// see also [`crate::sound::sound_unread_samples`]
    pub fn get_unread_samples(&self) -> u64 {
        unsafe { sound_unread_samples(self.0.as_ptr()) }
    }
}

extern "C" {
    pub fn sound_inst_stop(sound_inst: SoundInst);
    pub fn sound_inst_is_playing(sound_inst: SoundInst) -> Bool32T;
    pub fn sound_inst_set_pos(sound_inst: SoundInst, pos: Vec3);
    pub fn sound_inst_get_pos(sound_inst: SoundInst) -> Vec3;
    pub fn sound_inst_set_volume(sound_inst: SoundInst, volume: f32);
    pub fn sound_inst_get_volume(sound_inst: SoundInst) -> f32;
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SoundInst {
    pub _id: u16,
    pub _slot: i16,
}

impl SoundInst {
    /// This stops the sound early if it’s still playing. consume the SoundInst as it will not be playable again.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/Stop.html>
    ///
    /// see also [`crate::sound::sound_inst_stop`]
    pub fn stop(self) {
        unsafe { sound_inst_stop(self) }
    }

    /// The 3D position in world space this sound instance is currently playing at. If this instance is no longer
    /// valid, the position will be at zero.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/Position.html>
    ///
    /// see also [`crate::sound::sound_inst_set_pos`]
    pub fn position(&mut self, at: impl Into<Vec3>) -> &mut Self {
        unsafe { sound_inst_set_pos(*self, at.into()) }
        self
    }

    /// The volume multiplier of this Sound instance! A number between 0 and 1, where 0 is silent, and 1 is full volume.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/Volume.html>
    ///
    /// see also [`crate::sound::sound_inst_set_volume`]
    pub fn volume(&mut self, volume: f32) -> &mut Self {
        unsafe { sound_inst_set_volume(*self, volume) }
        self
    }

    /// The 3D position in world space this sound instance is currently playing at. If this instance is no longer
    /// valid, the position will be at zero.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/Position.html>
    ///
    /// see also [`crate::sound::sound_inst_get_pos`]
    pub fn get_position(&self) -> Vec3 {
        unsafe { sound_inst_get_pos(*self) }
    }

    /// The volume multiplier of this Sound instance! A number between 0 and 1, where 0 is silent, and 1 is full volume.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/Volume.html>
    ///
    /// see also [`crate::sound::sound_inst_get_volume`]
    pub fn get_volume(&self) -> f32 {
        unsafe { sound_inst_get_volume(*self) }
    }

    /// Is this Sound instance currently playing? For streaming assets, this will be true even if they don’t have any
    /// new data in them, and they’re just idling at the end of their data.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/IsPlaying.html>
    ///
    /// see also [`crate::sound::sound_inst_is_playing`]
    pub fn is_playing(&self) -> bool {
        unsafe { sound_inst_is_playing(*self) != 0 }
    }
}
