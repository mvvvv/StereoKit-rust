use crate::{
    StereoKitError,
    maths::{Bool32T, Vec3},
    system::IAsset,
};
use std::{
    ffi::{CStr, CString, c_char},
    path::Path,
    ptr::NonNull,
};

/// This class represents a sound effect! Excellent for blips and bloops and little clips that you might play around
/// your scene. Right now, this supports .wav, .mp3, and procedurally generated noises!
///
/// On HoloLens 2, sounds are automatically processed on the HPU, freeing up the CPU for more of your app’s code. To
/// simulate this same effect on your development PC, you need to enable spatial sound on your audio endpoint. To do
/// this, right click the speaker icon in your system tray, navigate to “Spatial sound”, and choose “Windows Sonic for
/// Headphones.” For more information, visit <https://docs.microsoft.com/en-us/windows/win32/coreaudio/spatial-sound>
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
/// number_of_steps = 150;
/// filename_scr = "screenshots/sound.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     transform.update_t_r(&position, &rotation);
///     mesh.draw(token, &material, transform, Some(named_colors::CYAN.into()), None);
///     if iter == 0 {
///         assert!(plane_sound_inst.is_playing());
///         position = Vec3::new(0.0, 0.0, -1.0);
///         plane_sound_inst
///             .position(position)
///             .volume(0.5);
///     } else if iter == 100 {
///         assert!(plane_sound_inst.is_playing());
///         assert_eq!(plane_sound_inst.get_position(), Vec3::new(0.0, 0.0, -1.0));
///         assert_eq!(plane_sound_inst.get_volume(), 0.5);
///         plane_sound_inst.stop();
///         assert!(!plane_sound_inst.is_playing());
///    }
/// );
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

#[link(name = "StereoKitC")]
unsafe extern "C" {
    pub fn sound_find(id: *const c_char) -> SoundT;
    pub fn sound_set_id(sound: SoundT, id: *const c_char);
    pub fn sound_get_id(sound: SoundT) -> *const c_char;
    pub fn sound_create(filename_utf8: *const c_char) -> SoundT;
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
    pub fn sound_get_decibels(sound: SoundT) -> f32;
    pub fn sound_set_decibels(sound: SoundT, decibels: f32);
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
    /// see also [`sound_create_stream`] [`Sound::from_samples`]
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
    /// let mut stream_sound_inst = stream_sound.play([0.0, 0.0, -0.5], Some(0.5));
    ///
    /// filename_scr = "screenshots/sound_stream.jpeg";
    /// number_of_steps = 150;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == 0 {
    ///         assert!(stream_sound_inst.is_playing());
    ///     } else if iter == 150 - 2 {
    ///         assert!(stream_sound_inst.is_playing());
    ///         stream_sound_inst.stop();
    ///         assert!(!stream_sound_inst.is_playing());
    ///     }
    /// );
    /// ```
    pub fn create_stream(stream_buffer_duration: f32) -> Result<Sound, StereoKitError> {
        Ok(Sound(
            NonNull::new(unsafe { sound_create_stream(stream_buffer_duration) })
                .ok_or(StereoKitError::SoundCreate("create_stream failed".into()))?,
        ))
    }

    /// Loads a sound effect from file! Currently, StereoKit supports .wav and .mp3 files. Audio is converted to mono.
    /// <https://stereokit.net/Pages/StereoKit/Sound/FromFile.html>
    /// * `file_utf8` - Name of the audio file! Supports .wav and .mp3 files.
    ///
    /// see also [`sound_create`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, sound::Sound};
    ///
    /// let mut position = Vec3::new(-0.5, 0.0, 0.5);
    ///
    /// let mut plane_sound = Sound::from_file("sounds/no.wav")
    ///                           .expect("no.wav should be in the sounds folder");
    /// assert_eq!(plane_sound.get_id(), "sounds/no.wav");
    /// plane_sound.id("sound_plane").decibels(90.0);
    ///
    /// let mut plane_sound_inst = plane_sound.play(position, Some(1.0));
    ///
    /// number_of_steps = 150;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     //TODO: assert!(plane_sound_inst.is_playing());
    /// );
    /// ```
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
    /// * `in_arr_samples_at_48000s` - Values should range from -1 to +1, and there should be 48,000 per second of audio.
    ///
    /// see also [`sound_create_samples`] [`Sound::write_samples`]
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
    /// let mut sound_inst = sound.play([0.0, 0.0, -0.5], Some(0.5));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     assert!(sound_inst.is_playing());
    /// );
    /// ```
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
    /// * `generator` - This function takes a time value as an argument, which will range from 0-duration, and should
    ///   return a value from -1 - +1 representing the audio wave at that point in time.
    /// * `duration` - The duration of the sound in seconds.
    ///
    /// see also [`sound_generate`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// unsafe extern "C" fn generator(sample_time: f32) -> f32 {
    ///     (sample_time * 440.0 * 2.0 * std::f32::consts::PI).sin()
    /// }
    /// let mut sound = Sound::generate(generator, 1.0)
    ///                     .expect("Sound should be created from generator");
    /// assert!(sound.get_id().starts_with("auto/sound_"));
    /// sound.id("sound_generator");
    ///
    /// let mut sound_inst = sound.play([0.0, 0.0, -0.5], Some(0.5));
    ///
    /// number_of_steps = 150;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     //assert!(sound_inst.is_playing());
    /// );
    /// ```
    pub fn generate(generator: unsafe extern "C" fn(f32) -> f32, duration: f32) -> Result<Sound, StereoKitError> {
        Ok(Sound(
            NonNull::new(unsafe { sound_generate(Some(generator), duration) })
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
    /// let mut plane_sound = Sound::from_file("sounds/plane_engine.mp3")
    ///                           .expect("plane_engine.mp3 should be in the sounds folder");
    ///
    /// let same_sound =  plane_sound.clone_ref();
    ///
    /// assert_eq!(plane_sound.get_id(), same_sound.get_id());
    /// assert_eq!(plane_sound, same_sound);
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
    /// ```
    pub fn id<S: AsRef<str>>(&mut self, id: S) -> &mut Self {
        let cstr_id = CString::new(id.as_ref()).unwrap();
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
    /// see also [`sound_play`] [`SoundInst::position`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, sound::Sound};
    ///
    /// let mut position = Vec3::new(-0.5, 0.0, 0.5);
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
    /// ```
    pub fn play(&self, at: impl Into<Vec3>, volume: Option<f32>) -> SoundInst {
        let volume = volume.unwrap_or(1.0);
        unsafe { sound_play(self.0.as_ptr(), at.into(), volume) }
    }

    /// <https://stereokit.net/Pages/StereoKit/Sound/Decibels.html>
    ///
    /// see also [`sound_set_decibels`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, sound::Sound};
    ///
    /// let mut position = Vec3::new(-0.5, 0.0, 0.5);
    ///
    /// let mut plane_sound = Sound::from_file("sounds/plane_engine.mp3").
    ///                           expect("A sound should be created");
    /// plane_sound.id("sound_plane").decibels(70.0);
    ///
    /// let mut plane_sound_inst = plane_sound.play(position, Some(1.0));
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
    /// let mut stream_sound = Sound::create_stream(0.5).
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
    /// for i in 0..24000 {
    ///     assert_eq!(samples[i], read_samples[i]);
    /// }
    ///
    /// let read_count = stream_sound.read_samples(read_samples.as_mut_slice(), Some(48000));
    /// assert_eq!(read_count, 0);
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
    /// let mut stream_sound = Sound::create_stream(1.0).
    ///                            expect("A sound stream should be created");
    ///
    /// let mut samples: Vec<f32> = vec![0.0; 48000];
    /// for i in 0..48000 {
    ///     samples[i] = (i as f32 / 48000.0).sin();
    /// }
    /// stream_sound.write_samples(samples.as_slice(), Some(48000));
    ///
    /// assert_eq!(stream_sound.get_unread_samples(), 48000);
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
        unsafe { CStr::from_ptr(sound_get_id(self.0.as_ptr())) }.to_str().unwrap()
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
    /// let mut sound = Sound::from_samples(&samples)
    ///                     .expect("Sound should be created from samples");
    ///
    /// assert_eq!(sound.get_cursor_samples(), 0);
    ///
    /// let mut sound_inst = sound.play([0.0, 0.0, -0.5], Some(0.5));
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
    /// let mut sound = Sound::from_samples(&samples)
    ///                     .expect("Sound should be created from samples");
    /// assert_eq!(sound.get_duration(), 1.0);
    ///
    /// let mut sound_file = Sound::from_file("sounds/no.wav")
    ///                          .expect("Sound should be created from file");
    /// assert_eq!(sound_file.get_duration(), 1.4830834);
    /// ```
    pub fn get_duration(&self) -> f32 {
        unsafe { sound_duration(self.0.as_ptr()) }
    }

    /// <https://stereokit.net/Pages/StereoKit/Sound/Decibels.html>
    ///
    /// see also [`sound_get_decibels`]
    /// see example in [`Sound::decibels`]
    pub fn get_decibels(&self) -> f32 {
        unsafe { sound_get_decibels(self.0.as_ptr()) }
    }

    /// This will return the total number of audio samples used by the sound! StereoKit currently uses 48,000 samples
    /// per second for all audio.
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
    /// let mut sound = Sound::from_samples(&samples)
    ///                     .expect("Sound should be created from samples");
    /// assert_eq!(sound.get_total_samples(), 48000);
    ///
    /// let mut sound_file = Sound::from_file("sounds/no.wav")
    ///                          .expect("Sound should be created from file");
    /// assert_eq!(sound_file.get_duration(), 1.4830834);
    /// // 1.4830834 * 48000 = 71188
    /// assert_eq!(sound_file.get_total_samples(), 71188);
    /// ```
    pub fn get_total_samples(&self) -> u64 {
        unsafe { sound_total_samples(self.0.as_ptr()) }
    }

    /// This is the maximum number of samples in the sound that are currently available for reading via ReadSamples!
    /// ReadSamples will reduce this number by the amount of samples read. This is only really valid for Stream
    /// sounds, all other sound types will just return 0.
    /// <https://stereokit.net/Pages/StereoKit/Sound/UnreadSamples.html>
    ///
    /// see also [`sound_unread_samples`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sound::Sound;
    ///
    /// // Half of the samples won't be kept in the buffer (0.5 instead of 1.0)
    /// let mut stream_sound = Sound::create_stream(1.0).
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
    /// ```
    pub fn get_unread_samples(&self) -> u64 {
        unsafe { sound_unread_samples(self.0.as_ptr()) }
    }

    /// A default click sound that lasts for 300ms. It’s a procedurally generated sound based on a mouse press, with
    /// extra low frequencies in it.
    /// <https://stereokit.net/Pages/StereoKit/Sound/Click.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, sound::Sound};
    ///
    /// let mut click_sound = Sound::click();
    /// assert_eq!(click_sound.get_id(), "default/sound_click");
    ///
    /// let mut click_sound_inst = click_sound.play([0.0, 0.0, -0.5], Some(0.5));
    ///
    /// number_of_steps = 100;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     // TODO: assert!(grab_sound_inst.is_playing());
    /// );
    /// ```
    pub fn click() -> Self {
        let cstr_id = CString::new("default/sound_click").unwrap();
        Sound(NonNull::new(unsafe { sound_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// A default unclick sound that lasts for 300ms. It’s a procedurally generated sound based on a mouse press, with
    /// extra low frequencies in it.
    /// <https://stereokit.net/Pages/StereoKit/Sound/Unclick.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, sound::Sound};
    ///
    /// let mut unclick_sound = Sound::unclick();
    /// assert_eq!(unclick_sound.get_id(), "default/sound_unclick");
    ///
    /// let mut unclick_sound_inst = unclick_sound.play([0.0, 0.0, -0.5], Some(0.5));
    ///
    /// number_of_steps = 100;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     // TODO: assert!(grab_sound_inst.is_playing());
    /// );
    /// ```
    pub fn unclick() -> Self {
        let cstr_id = CString::new("default/sound_unclick").unwrap();
        Sound(NonNull::new(unsafe { sound_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// A default grab sound
    /// <https://stereokit.net/Pages/StereoKit/Sound.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, sound::Sound};
    ///
    /// let mut grab_sound = Sound::grab();
    /// assert_eq!(grab_sound.get_id(), "default/sound_grab");
    ///
    /// let mut grab_sound_inst = grab_sound.play([0.0, 0.0, -0.5], Some(0.5));
    ///
    /// number_of_steps = 100;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     // TODO: assert!(grab_sound_inst.is_playing());
    /// );
    /// ```
    pub fn grab() -> Self {
        let cstr_id = CString::new("default/sound_grab").unwrap();
        Sound(NonNull::new(unsafe { sound_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// A default ungrab sound
    /// <https://stereokit.net/Pages/StereoKit/Sound.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, sound::Sound};
    ///
    /// let mut ungrab_sound = Sound::ungrab();
    /// assert_eq!(ungrab_sound.get_id(), "default/sound_ungrab");
    ///
    /// let mut ungrab_sound_inst = ungrab_sound.play([0.0, 0.0, -0.5], Some(0.5));
    ///
    /// number_of_steps = 100;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     // TODO: assert!(ungrab_sound_inst.is_playing());
    /// );
    /// ```
    pub fn ungrab() -> Self {
        let cstr_id = CString::new("default/sound_ungrab").unwrap();
        Sound(NonNull::new(unsafe { sound_find(cstr_id.as_ptr()) }).unwrap())
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
/// let mut plane_sound1 = Sound::from_file("sounds/no.wav")
///                           .expect("no.wav should be there");
/// plane_sound1.id("sound_plane1").decibels(70.0);
/// let mut plane_sound_inst1 = plane_sound1.play(position1, Some(1.0));
///
/// let mut plane_sound2 = Sound::from_file("sounds/no.wav")
///                           .expect("no.wav should be there");
/// plane_sound2.id("sound_plane2").decibels(70.0);
/// let mut plane_sound_inst2 = plane_sound2.play(position2, Some(1.0));
/// plane_sound_inst2.stop();
///
/// number_of_steps = 150;
/// filename_scr = "screenshots/sound_inst.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     let transform1 = Matrix::t(position1);
///     let transform2 = Matrix::t(position2);
///     sphere.draw(token, &material, transform1, Some(named_colors::PINK.into()), None  );
///     sphere.draw(token, &material, transform2, Some(named_colors::LIGHT_GREEN.into()), None  );
///
///     if iter == 0 {
///         //TODO: assert!(plane_sound_inst1.is_playing());
///         assert!(!plane_sound_inst2.is_playing());
///         position1 = Vec3::new(-0.3, 0.0, 0.3);
///         plane_sound_inst1
///             .position(position1)
///             .volume(0.5);
///     } else if iter == 150 - 2 {
///         //TODO: assert!(plane_sound_inst1.is_playing());
///         position2 = Vec3::new(0.3, 0.0, 0.3);
///         plane_sound_inst2 = plane_sound2.play(position2, Some(1.0));
///         assert!(plane_sound_inst2.is_playing());
///    }
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sound_inst.jpeg" alt="screenshot" width="200">
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SoundInst {
    pub _id: u16,
    pub _slot: i16,
}

#[link(name = "StereoKitC")]
unsafe extern "C" {
    pub fn sound_inst_stop(sound_inst: SoundInst);
    pub fn sound_inst_is_playing(sound_inst: SoundInst) -> Bool32T;
    pub fn sound_inst_set_pos(sound_inst: SoundInst, pos: Vec3);
    pub fn sound_inst_get_pos(sound_inst: SoundInst) -> Vec3;
    pub fn sound_inst_set_volume(sound_inst: SoundInst, volume: f32);
    pub fn sound_inst_get_volume(sound_inst: SoundInst) -> f32;
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
    /// use stereokit_rust::{maths::Vec3, sound::Sound};
    ///
    /// let mut plane_sound = Sound::from_file("sounds/plane_engine.mp3").
    ///                           expect("A sound should be created");
    /// let mut plane_sound_inst = plane_sound.play([0.0, 0.0, 0.0], Some(1.0));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == 1 {
    ///         plane_sound_inst.stop();
    ///         assert!(!plane_sound_inst.is_playing());
    ///     }
    /// );
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
    /// ```
    pub fn position(&mut self, at: impl Into<Vec3>) -> &mut Self {
        unsafe { sound_inst_set_pos(*self, at.into()) }
        self
    }

    /// The volume multiplier of this Sound instance! A number between 0 and 1, where 0 is silent, and 1 is full volume.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/Volume.html>
    ///
    /// see also [`sound_inst_set_volume`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, sound::Sound, system::Assets};
    ///
    /// let mut position = Vec3::new(0.0, 0.0, 0.5);
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
    /// ```
    pub fn volume(&mut self, volume: f32) -> &mut Self {
        unsafe { sound_inst_set_volume(*self, volume) }
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

    /// The volume multiplier of this Sound instance! A number between 0 and 1, where 0 is silent, and 1 is full volume.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/Volume.html>
    ///
    /// see also [`sound_inst_get_volume`]
    /// see example in [`SoundInst::volume`]
    pub fn get_volume(&self) -> f32 {
        unsafe { sound_inst_get_volume(*self) }
    }

    /// The maximum intensity of the sound data since the last frame, as a value from 0-1. This is unaffected by its 3d
    /// position or volume settings, and is straight from the audio file's data.
    /// <https://stereokit.net/Pages/StereoKit/SoundInst/Intensity.html>
    ///
    /// see also [`sound_inst_get_intensity`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, sound::Sound};
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
    /// use stereokit_rust::{maths::Vec3, sound::Sound};
    ///
    /// let mut plane_sound = Sound::from_file("sounds/plane_engine.mp3").
    ///                           expect("A sound should be created");
    /// let mut plane_sound_inst = plane_sound.play([0.0, 0.0, 0.0], Some(1.0));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == 1 {
    ///         assert!(plane_sound_inst.is_playing());
    ///         plane_sound_inst.stop();
    ///     } else if iter > 1 {
    ///         assert!(!plane_sound_inst.is_playing());
    ///     }
    /// );
    /// ```
    pub fn is_playing(&self) -> bool {
        unsafe { sound_inst_is_playing(*self) != 0 }
    }
}
