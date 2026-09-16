use crate::{
    maths::Bool32T,
    tex::{Tex, TexT},
    util::{SHLight, SphericalHarmonics},
};
use std::ptr::NonNull;

/// This determines where lighting data comes from! The default is [`LightingSource::Manual`], where the application
/// provides all lighting via the [`Lighting`] functions. Devices that can estimate lighting from the user's
/// surroundings also have the [`LightingSource::World`] option.
/// <https://stereokit.net/Pages/StereoKit/LightingSource.html>
///
/// see also [`Lighting`]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum LightingSource {
    /// Lighting values are set manually by the application. Use the [`Lighting`] functions to configure the scene
    /// lighting.
    Manual = 0,
    /// Lighting data is pulled from the world via the device's light estimation capabilities. StereoKit will overwrite
    /// any data in [`Lighting::ambient`], [`Lighting::main_light`], and [`Lighting::reflection`] when using this
    /// source. You can check [`Lighting::is_source_available`] to see if this is supported before requesting it.
    World = 1,
}

/// This determines what form scene lighting takes: all of it can fold into the ambient probe, or the dominant
/// directional light can be separated out from it. This shapes lighting derived from an environment: both the world
/// source's estimates, and what [`Lighting::set_environment`] derives from its cubemap. Changing the mode re-delivers
/// the scene's most recent full lighting in the new shape, replacing [`Lighting::ambient`] and
/// [`Lighting::main_light`].
/// <https://stereokit.net/Pages/StereoKit/LightingMode.html>
///
/// see also [`Lighting`]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum LightingMode {
    /// All light folds into the [`Lighting::ambient`] probe. This is the default. [`Lighting::main_light`] still
    /// reports the dominant directional light, but as information only: its energy remains inside
    /// [`Lighting::ambient`], so it suits things like shadow direction, not additional shading.
    Ambient = 0,
    /// The dominant directional light is separated out into [`Lighting::main_light`], and [`Lighting::ambient`]
    /// carries only the remainder. This is for applications that render that light themselves, such as for shadow
    /// casting, since otherwise its energy is counted twice.
    MainLight = 1,
}

unsafe extern "C" {
    pub fn lighting_source_available(source: LightingSource) -> Bool32T;
    pub fn lighting_request_source(source: LightingSource);
    pub fn lighting_get_source() -> LightingSource;
    pub fn lighting_source_pending() -> Bool32T;
    pub fn lighting_set_mode(mode: LightingMode);
    pub fn lighting_get_mode() -> LightingMode;
    pub fn lighting_set_main_light(light: *const SHLight);
    pub fn lighting_get_main_light() -> SHLight;
    pub fn lighting_set_environment(sky_cubemap: TexT, out_reflection: *mut TexT);
    pub fn lighting_set_ambient(ambient_lighting: *const SphericalHarmonics);
    pub fn lighting_get_ambient() -> SphericalHarmonics;
    pub fn lighting_set_reflection(ibl_cubemap: TexT);
    pub fn lighting_get_reflection() -> TexT;
}

/// Scene lighting! StereoKit's lighting is entirely environment based. An ambient lighting probe
/// ([`Lighting::ambient`]) provides soft directional light, a specular reflection cubemap ([`Lighting::reflection`])
/// provides shiny highlights and mirror surfaces, and a dominant directional light ([`Lighting::main_light`]) is
/// derived from it all for effects like shadows. [`Lighting::set_environment`] fills in all of this from a single
/// cubemap, and is the easiest place to start!
///
/// On devices that can estimate lighting from their surroundings, the world source keeps all of this matched to the
/// user's real room instead, see [`Lighting::request_source`].
/// <https://stereokit.net/Pages/StereoKit/Lighting.html>
///
/// see also: [`LightingSource`] [`LightingMode`] [`crate::render::Renderer`] [`crate::tex::SHCubemap`]
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{lighting::{Lighting, LightingSource, LightingMode},
///                      util::{named_colors, SHLight, SphericalHarmonics}, maths::Vec3};
///
/// // The default lighting source is Manual, and it is always available:
/// assert_eq!(Lighting::get_source(), LightingSource::Manual);
/// assert_eq!(Lighting::is_source_available(LightingSource::Manual), true);
/// assert_eq!(Lighting::get_mode(), LightingMode::Ambient);
///
/// // The scene's lighting pieces can be set manually:
/// let light = SHLight::new(Vec3::UP, named_colors::WHITE);
/// Lighting::main_light(light);
/// Lighting::ambient(SphericalHarmonics::from_lights(&[light]));
///
/// assert_eq!(Lighting::get_main_light(), light);
/// # sk::Sk::shutdown();
/// ```
pub struct Lighting;

impl Lighting {
    /// Sets up the whole scene's lighting from a single cubemap!
    /// This includes the [`crate::render::Renderer::skybox_tex`], a generated [`Lighting::reflection`], and once
    /// that's finished generating, [`Lighting::ambient`] and [`Lighting::main_light`] too. Raw radiance cubemaps such
    /// as the ones [`Tex::from_cubemap`](crate::tex::SHCubemap::from_cubemap) provides are perfect here, no mip chain
    /// needed.
    ///
    /// Anything you assign to those properties afterwards overrides that piece. Null resets all of it to StereoKit's
    /// defaults. Ignored when using [`LightingSource::World`] as a lighting source.
    /// <https://stereokit.net/Pages/StereoKit/Lighting/SetEnvironment.html>
    /// * `sky_cubemap` - A cubemap of the environment's radiance, such as one from
    ///   [`Tex::from_cubemap`](crate::tex::SHCubemap::from_cubemap), or None to reset to the default environment.
    ///
    /// see also [`lighting_set_environment`] [`Lighting::set_environment_with_reflection`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{lighting::Lighting, maths::Vec3, tex::{Tex, TexType, TexFormat},
    ///                      util::Color128};
    ///
    /// let sky_cubemap = Tex::gen_color(Color128::WHITE, 128, 128, TexType::Cubemap,
    ///                                  TexFormat::Rgba32Linear);
    /// Lighting::set_environment(Some(&sky_cubemap));
    ///
    /// // The reflection, and the lighting derived from it, generate
    /// // asynchronously, so let them land before the app shuts down.
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     system::Assets::block_for_priority(i32::MAX);
    ///
    ///     assert!(Lighting::get_reflection().is_some());
    ///     assert_ne!(Lighting::get_ambient().coefficients[0], Vec3::ZERO);
    ///
    ///     // None resets everything back to StereoKit's default environment:
    ///     Lighting::set_environment(None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_environment(sky_cubemap: Option<&Tex>) {
        let mut reflection: TexT = std::ptr::null_mut();
        unsafe {
            lighting_set_environment(sky_cubemap.map_or(std::ptr::null_mut(), |tex| tex.0.as_ptr()), &mut reflection);
            // The caller doesn't want the reflection, so release it right away.
            if !reflection.is_null() {
                crate::tex::tex_release(reflection);
            }
        }
    }

    /// This flavor also hands back the reflection texture it generated, for hooking a load callback
    /// ([`crate::tex::tex_on_load`]), or re-convolving later with [`Tex::gen_cubemap_reflection`].
    /// <https://stereokit.net/Pages/StereoKit/Lighting/SetEnvironment.html>
    /// * `sky_cubemap` - A cubemap of the environment's radiance, such as one from
    ///   [`SHCubemap::from_cubemap`](crate::tex::SHCubemap::from_cubemap), or None to reset to the default environment.
    ///
    /// Returns the generated reflection, or None if none was generated, such as on a None reset, or when using
    /// [`LightingSource::World`] as a lighting source.
    ///
    /// see also [`lighting_set_environment`] [`Lighting::set_environment`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{lighting::Lighting, tex::{Tex, TexType, TexFormat},
    ///                      util::Color128};
    ///
    /// let sky_cubemap = Tex::gen_color(Color128::WHITE, 128, 128, TexType::Cubemap, TexFormat::Rgba32Linear);
    /// let reflection = Lighting::set_environment_with_reflection(Some(&sky_cubemap));
    ///
    /// let reflection = reflection.expect("the reflection is handed back right away");
    /// assert_eq!(Lighting::get_reflection().as_ref(), Some(&reflection));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     system::Assets::block_for_priority(i32::MAX);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_environment_with_reflection(sky_cubemap: Option<&Tex>) -> Option<Tex> {
        let mut reflection: TexT = std::ptr::null_mut();
        unsafe {
            lighting_set_environment(sky_cubemap.map_or(std::ptr::null_mut(), |tex| tex.0.as_ptr()), &mut reflection);
        }
        NonNull::new(reflection).map(Tex)
    }

    /// Where scene lighting comes from right now? The application ([`LightingSource::Manual`], the default), or
    /// estimated live from the user's surroundings ([`LightingSource::World`]). This never changes on its own, the
    /// world source is an opt-in via [`Lighting::request_source`].
    ///
    /// A request that's still settling reads as its previous value here, see [`Lighting::is_source_pending`].
    /// <https://stereokit.net/Pages/StereoKit/Lighting/Source.html>
    ///
    /// see also [`lighting_get_source`] [`Lighting::request_source`]
    /// see example in [`Lighting`]
    pub fn get_source() -> LightingSource {
        unsafe { lighting_get_source() }
    }

    /// True while a [`Lighting::request_source`] is still settling, which is typically a permission dialog. Resolves
    /// within moments, with the outcome in [`Lighting::get_source`].
    /// <https://stereokit.net/Pages/StereoKit/Lighting/SourcePending.html>
    ///
    /// see also [`lighting_source_pending`] [`Lighting::request_source`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::lighting::Lighting;
    ///
    /// assert_eq!(Lighting::is_source_pending(), false);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn is_source_pending() -> bool {
        unsafe { lighting_source_pending() != 0 }
    }

    /// Requests a switch to a lighting source! [`LightingSource::Manual`] applies immediately, while
    /// [`LightingSource::World`] may show a permission dialog the user can decline, watch
    /// [`Lighting::is_source_pending`] for that.
    /// <https://stereokit.net/Pages/StereoKit/Lighting/RequestSource.html>
    /// * `source` - The lighting source to switch to.
    ///
    /// see also [`lighting_request_source`] [`Lighting::get_source`] [`Lighting::is_source_available`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::lighting::{Lighting, LightingSource};
    ///
    /// // Switching back to Manual is immediate:
    /// Lighting::request_source(LightingSource::Manual);
    /// assert_eq!(Lighting::get_source(), LightingSource::Manual);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn request_source(source: LightingSource) {
        unsafe { lighting_request_source(source) }
    }

    /// How environment lighting is delivered! [`LightingMode::Ambient`] folds all light into the [`Lighting::ambient`]
    /// SH, while [`LightingMode::MainLight`] splits the dominant directional light out into [`Lighting::main_light`],
    /// leaving [`Lighting::ambient`] the remainder.
    ///
    /// NOTE: StereoKit builtin shaders do not yet account for directional light, you will need your own shaders for
    /// [`LightingMode::MainLight`] mode to work. Changing [`Lighting::mode`] will ALSO change your _current_
    /// [`Lighting::ambient`] SH, adding or subtracting the main light's energy.
    /// <https://stereokit.net/Pages/StereoKit/Lighting/Mode.html>
    /// * `mode` - How environment lighting should be delivered.
    ///
    /// see also [`lighting_set_mode`] [`Lighting::get_mode`] [`Lighting::main_light`]
    /// see example in [`Lighting`]
    pub fn mode(mode: LightingMode) {
        unsafe { lighting_set_mode(mode) }
    }

    /// How environment lighting is delivered, see [`Lighting::mode`].
    /// <https://stereokit.net/Pages/StereoKit/Lighting/Mode.html>
    ///
    /// see also [`lighting_get_mode`] [`Lighting::mode`]
    /// see example in [`Lighting`]
    pub fn get_mode() -> LightingMode {
        unsafe { lighting_get_mode() }
    }

    /// The scene's dominant directional light, ideal as a shadow direction! Environment lighting keeps this derived,
    /// and the direction is always normalized. Check [`Lighting::mode`] before shading with it, since in
    /// [`LightingMode::Ambient`] mode this light's energy is _also_ inside the [`Lighting::ambient`] probe.
    ///
    /// A black color means no light. Assignments follow the same rules as [`Lighting::ambient`]: applied as-is until
    /// the next environment lighting replaces them. Built-in shaders don't consume this, it's for your own shaders and
    /// effects.
    /// <https://stereokit.net/Pages/StereoKit/Lighting/MainLight.html>
    /// * `light` - The main light to assign.
    ///
    /// see also [`lighting_set_main_light`] [`Lighting::get_main_light`]
    /// see example in [`Lighting`]
    pub fn main_light(light: impl Into<SHLight>) {
        let light = light.into();
        unsafe { lighting_set_main_light(&light) }
    }

    /// The scene's dominant directional light, see [`Lighting::main_light`].
    /// <https://stereokit.net/Pages/StereoKit/Lighting/MainLight.html>
    ///
    /// see also [`lighting_get_main_light`] [`Lighting::main_light`]
    /// see example in [`Lighting`]
    pub fn get_main_light() -> SHLight {
        unsafe { lighting_get_main_light() }
    }

    /// The scene's ambient light probe, as spherical harmonics! This is soft omnidirectional light, the color and
    /// intensity arriving from each direction rather than a discrete source. Build one with
    /// [`SphericalHarmonics::from_lights`], or let [`Lighting::set_environment`] derive it from a cubemap.
    ///
    /// Assignments apply exactly as provided, and win over a [`Lighting::set_environment`] that's still loading, even
    /// once it finishes. Later environment lighting replaces them: a new [`Lighting::set_environment`], or world
    /// source estimates, where assignments are ignored entirely. A [`Lighting::mode`] change re-shapes the newest
    /// lighting, assigned or not.
    /// <https://stereokit.net/Pages/StereoKit/Lighting/Ambient.html>
    /// * `ambient_lighting` - The ambient light probe to assign.
    ///
    /// see also [`lighting_set_ambient`] [`Lighting::get_ambient`] [`SphericalHarmonics::from_lights`]
    /// see example in [`Lighting`]
    pub fn ambient(ambient_lighting: impl Into<SphericalHarmonics>) {
        let ambient_lighting = ambient_lighting.into();
        unsafe { lighting_set_ambient(&ambient_lighting) }
    }

    /// The scene's ambient light probe, as spherical harmonics, see [`Lighting::ambient`].
    /// <https://stereokit.net/Pages/StereoKit/Lighting/Ambient.html>
    ///
    /// see also [`lighting_get_ambient`] [`Lighting::ambient`]
    /// see example in [`Lighting`]
    pub fn get_ambient() -> SphericalHarmonics {
        unsafe { lighting_get_ambient() }
    }

    /// The specular reflection cubemap used by PBR shading: GGX convolved radiance, one roughness level per mip.
    /// Generate one with [`Tex::gen_cubemap_reflection`], and None restores the built-in default.
    ///
    /// A cubemap without a convolved mip chain still binds, but reads mirror-sharp at every roughness. In the world
    /// source this is fed by estimation, and assignments are ignored.
    /// <https://stereokit.net/Pages/StereoKit/Lighting/Reflection.html>
    /// * `reflection` - The reflection cubemap to assign, or None to restore the built-in default.
    ///
    /// see also [`lighting_set_reflection`] [`Lighting::get_reflection`] [`crate::tex::Tex::gen_cubemap_reflection`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{lighting::Lighting, tex::{Tex, TexType, TexFormat},
    ///                      util::Color128};
    ///
    /// let sky_cubemap = Tex::gen_color(Color128::WHITE, 128, 128, TexType::Cubemap, TexFormat::Rgba32Linear);
    /// let reflection = Tex::gen_cubemap_reflection(&sky_cubemap, None, 64)
    ///                     .expect("reflection should be generated");
    /// Lighting::reflection(Some(&reflection));
    ///
    /// assert_eq!(Lighting::get_reflection(), Some(reflection.clone_ref()));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     system::Assets::block_for_priority(i32::MAX);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn reflection(reflection: Option<&Tex>) {
        unsafe { lighting_set_reflection(reflection.map_or(std::ptr::null_mut(), |tex| tex.0.as_ptr())) }
    }

    /// The specular reflection cubemap used by PBR shading, see [`Lighting::reflection`].
    /// <https://stereokit.net/Pages/StereoKit/Lighting/Reflection.html>
    ///
    /// Returns None when no custom reflection is set, such as with the built-in default environment.
    ///
    /// see also [`lighting_get_reflection`] [`Lighting::reflection`]
    /// see example in [`Lighting::reflection`]
    pub fn get_reflection() -> Option<Tex> {
        let reflection = unsafe { lighting_get_reflection() };
        NonNull::new(reflection).map(Tex)
    }

    /// Check if a lighting source is supported on this device, without switching to it! Manual is always available,
    /// while world needs light estimation support from the XR runtime, and may still need a permission the user can
    /// decline at [`Lighting::request_source`] time.
    /// <https://stereokit.net/Pages/StereoKit/Lighting/SourceAvailable.html>
    /// * `source` - The lighting source to check on.
    ///
    /// Returns true if the source can be used on this device.
    ///
    /// see also [`lighting_source_available`] [`Lighting::request_source`]
    /// see example in [`Lighting`]
    pub fn is_source_available(source: LightingSource) -> bool {
        unsafe { lighting_source_available(source) != 0 }
    }
}
