use stereokit_rust::{framework::Screen, prelude::*, system::Input, tex::Tex, util::Time};

/// Demo that displays a screen cycling through JPEG textures from assets/textures
#[derive(IStepper)]
pub struct Screen1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    screen: Screen,
    textures: Vec<Tex>,
    current_texture_index: usize,
    last_switch_time: f32,
    switch_interval: f32, // seconds between automatic switches
}

unsafe impl Send for Screen1 {}

impl Default for Screen1 {
    fn default() -> Self {
        // Load all JPEG textures from assets/textures
        let texture_paths = vec![
            "textures/exit.jpeg",
            "textures/log_viewer.jpeg",
            "textures/micro.jpeg",
            "textures/open_gltf.jpeg",
            "textures/screenshot.jpeg",
            "textures/sound.jpeg",
        ];

        let mut textures = Vec::new();
        for path in texture_paths {
            if let Ok(tex) = Tex::from_file(path, true, None) {
                textures.push(tex);
            }
        }

        // Create screen with first texture if available
        let initial_texture = textures.first().map(|t| t.clone_ref()).unwrap_or_else(Tex::default);

        let mut screen = Screen::new("slideshow", initial_texture);
        screen.screen_orientation([0.0, 180.0, 0.0]).screen_size([1.920, 1.080]);

        // Load second texture if needed
        if textures.len() > 1 {
            screen.set_texture(1, Some(textures[1].clone_ref()));
        }

        Self {
            id: "Screen1".to_string(),
            sk_info: None,
            screen,
            textures,
            current_texture_index: 0,
            last_switch_time: 0.0,
            switch_interval: 3.0, // Change image every 3 seconds
        }
    }
}

impl Screen1 {
    // Called by derive macro during IStepper::initialize
    fn start(&mut self) -> bool {
        true
    }

    // Called by derive macro during IStepper::step
    fn draw(&mut self, token: &MainThreadToken) {
        let current_time = Time::get_totalf();

        // Auto-advance textures
        if current_time - self.last_switch_time > self.switch_interval {
            self.next_texture();
            self.last_switch_time = current_time;
        }

        // Update screen
        self.screen.draw(token);

        // Check for touches
        for index in 0..Input::pointer_count(None) {
            if let Some(touched) = self.screen.touched(token, index) {
                Log::info(format!("Screen touched at: {:?}x{:?}", touched.0, touched.1));
            }
        }
    }

    // Called by derive macro for event handling
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    fn next_texture(&mut self) {
        if self.textures.is_empty() {
            return;
        }

        self.current_texture_index = (self.current_texture_index + 1) % self.textures.len();
        self.update_screen_texture();
    }

    fn update_screen_texture(&mut self) {
        if self.current_texture_index < self.textures.len() {
            let tex_slot = self.current_texture_index % 2; // Alternate between slots 0 and 1
            self.screen.set_texture(tex_slot, Some(self.textures[self.current_texture_index].clone_ref()));
            self.screen.set_tex_curr(tex_slot);
        }
    }
}

#[allow(dead_code)]
fn id() -> StepperId {
    "Screen1".to_string()
}
