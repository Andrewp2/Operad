#[cfg(not(feature = "widgets"))]
fn main() {
    println!("operad_showcase: rebuild without `--no-default-features` to include widgets");
}

#[cfg(all(feature = "widgets", not(feature = "native-window")))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "wgpu")]
    if let Some(path) = std::env::var_os("OPERAD_SHOWCASE_WGPU_SCREENSHOT") {
        return render_wgpu_showcase_screenshot(path.into());
    }

    validate_headless_showcase()
}

#[cfg(all(feature = "widgets", feature = "native-window"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = std::env::var_os("OPERAD_SHOWCASE_WGPU_SCREENSHOT") {
        return render_wgpu_showcase_screenshot(path.into());
    }

    if std::env::var_os("OPERAD_SHOWCASE_HEADLESS").is_some() {
        validate_headless_showcase()
    } else {
        run_windowed_showcase()
    }
}

#[cfg(feature = "widgets")]
fn validate_headless_showcase() -> Result<(), Box<dyn std::error::Error>> {
    let viewport = operad::UiSize::new(1280.0, 800.0);
    let state = showcase::ShowcaseState::default();
    let mut document = showcase::build_document(viewport, &state);
    document.compute_layout(viewport, &mut operad::ApproxTextMeasurer)?;

    println!(
        "operad_showcase: validated {} paint items, {} accessibility nodes",
        document.paint_list().items.len(),
        document.accessibility_tree().len()
    );

    Ok(())
}

#[cfg(all(feature = "widgets", feature = "wgpu"))]
fn render_wgpu_showcase_screenshot(
    path: std::path::PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    use operad::RendererAdapter;

    let viewport = screenshot_viewport()?;
    let pixel_size = operad::platform::PixelSize::new(
        viewport.width.round() as u32,
        viewport.height.round() as u32,
    );
    let state = screenshot_state();
    let mut document = showcase::build_document(viewport, &state);
    document.compute_layout(viewport, &mut operad::ApproxTextMeasurer)?;

    let request = operad::RenderFrameRequest::new(
        operad::RenderTarget::snapshot(pixel_size),
        viewport,
        document.paint_list(),
    );
    let mut renderer = operad::WgpuRenderer::default();
    let output = renderer.render_frame(request, &operad::EmptyResourceResolver)?;
    let image = output
        .snapshot
        .ok_or("WGPU snapshot target did not produce an image")?;
    write_png_rgba8(&path, &image)?;

    println!(
        "operad_showcase: wrote WGPU screenshot {:?}, {}x{}, {} paint items",
        path, image.size.width, image.size.height, output.painted_items
    );
    Ok(())
}

#[cfg(all(feature = "widgets", feature = "wgpu"))]
fn screenshot_viewport() -> Result<operad::UiSize, Box<dyn std::error::Error>> {
    let Some(value) = std::env::var_os("OPERAD_SHOWCASE_VIEWPORT") else {
        return Ok(operad::UiSize::new(1280.0, 800.0));
    };
    let value = value
        .into_string()
        .map_err(|_| "OPERAD_SHOWCASE_VIEWPORT must be valid UTF-8")?;
    let (width, height) = value
        .split_once('x')
        .or_else(|| value.split_once('X'))
        .ok_or("OPERAD_SHOWCASE_VIEWPORT must be WIDTHxHEIGHT")?;
    let width = width
        .parse::<u32>()
        .map_err(|_| "OPERAD_SHOWCASE_VIEWPORT width must be an integer")?;
    let height = height
        .parse::<u32>()
        .map_err(|_| "OPERAD_SHOWCASE_VIEWPORT height must be an integer")?;
    if width == 0 || height == 0 {
        return Err("OPERAD_SHOWCASE_VIEWPORT dimensions must be nonzero".into());
    }
    Ok(operad::UiSize::new(width as f32, height as f32))
}

#[cfg(all(feature = "widgets", feature = "wgpu"))]
fn screenshot_state() -> showcase::ShowcaseState {
    let mut state = showcase::ShowcaseState::default();
    match std::env::var("OPERAD_SHOWCASE_STATE").as_deref() {
        Ok("menu") => {
            state.active_nav = Some(0);
            state.menu_open = true;
        }
        Ok("palette") => state.palette_open = true,
        Ok("actions") => state.active_tab = 1,
        Ok("a11y") => state.active_tab = 2,
        _ => {}
    }
    state
}

#[cfg(all(feature = "widgets", feature = "wgpu"))]
fn write_png_rgba8(
    path: &std::path::Path,
    image: &operad::RenderedImage,
) -> Result<(), Box<dyn std::error::Error>> {
    if image.format != operad::ResourceFormat::Rgba8 {
        return Err(format!("expected RGBA8 screenshot, got {:?}", image.format).into());
    }
    let width = image.size.width;
    let height = image.size.height;
    let expected_len = width as usize * height as usize * 4;
    if image.pixels.len() != expected_len {
        return Err(format!(
            "expected {expected_len} screenshot bytes, got {}",
            image.pixels.len()
        )
        .into());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut scanlines = Vec::with_capacity((width as usize * 4 + 1) * height as usize);
    for row in image.pixels.chunks_exact(width as usize * 4) {
        scanlines.push(0);
        scanlines.extend_from_slice(row);
    }

    let mut png = Vec::new();
    png.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    push_png_chunk(&mut png, b"IHDR", &ihdr);

    let idat = zlib_store(&scanlines);
    push_png_chunk(&mut png, b"IDAT", &idat);
    push_png_chunk(&mut png, b"IEND", &[]);

    std::fs::write(path, png)?;
    Ok(())
}

#[cfg(all(feature = "widgets", feature = "wgpu"))]
fn zlib_store(input: &[u8]) -> Vec<u8> {
    let mut output = vec![0x78, 0x01];
    let mut remaining = input;
    while !remaining.is_empty() {
        let block_len = remaining.len().min(u16::MAX as usize);
        let final_block = block_len == remaining.len();
        output.push(u8::from(final_block));
        let len = block_len as u16;
        output.extend_from_slice(&len.to_le_bytes());
        output.extend_from_slice(&(!len).to_le_bytes());
        output.extend_from_slice(&remaining[..block_len]);
        remaining = &remaining[block_len..];
    }
    output.extend_from_slice(&adler32(input).to_be_bytes());
    output
}

#[cfg(all(feature = "widgets", feature = "wgpu"))]
fn push_png_chunk(output: &mut Vec<u8>, name: &[u8; 4], data: &[u8]) {
    output.extend_from_slice(&(data.len() as u32).to_be_bytes());
    output.extend_from_slice(name);
    output.extend_from_slice(data);

    let mut crc_input = Vec::with_capacity(name.len() + data.len());
    crc_input.extend_from_slice(name);
    crc_input.extend_from_slice(data);
    output.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

#[cfg(all(feature = "widgets", feature = "wgpu"))]
fn crc32(input: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for byte in input {
        crc ^= *byte as u32;
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

#[cfg(all(feature = "widgets", feature = "wgpu"))]
fn adler32(input: &[u8]) -> u32 {
    const MOD: u32 = 65_521;
    let mut a = 1_u32;
    let mut b = 0_u32;
    for byte in input {
        a = (a + *byte as u32) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
}

#[cfg(all(feature = "widgets", feature = "native-window"))]
fn run_windowed_showcase() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut app = ShowcaseWindowApp::new(showcase_window_frame_limit()?);
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.error {
        Err(error.into())
    } else {
        Ok(())
    }
}

#[cfg(all(feature = "widgets", feature = "native-window"))]
struct ShowcaseWindowApp {
    window: Option<std::sync::Arc<winit::window::Window>>,
    window_id: Option<winit::window::WindowId>,
    renderer: Option<operad::WgpuSurfaceRenderer<'static>>,
    state: showcase::ShowcaseState,
    cursor: Option<operad::UiPoint>,
    modifiers: winit::keyboard::ModifiersState,
    error: Option<String>,
    frame_limit: Option<usize>,
    presented_frames: usize,
}

#[cfg(all(feature = "widgets", feature = "native-window"))]
impl ShowcaseWindowApp {
    fn new(frame_limit: Option<usize>) -> Self {
        Self {
            window: None,
            window_id: None,
            renderer: None,
            state: showcase::ShowcaseState::default(),
            cursor: None,
            modifiers: winit::keyboard::ModifiersState::empty(),
            error: None,
            frame_limit,
            presented_frames: 0,
        }
    }

    fn init_window(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let window = std::sync::Arc::new(
            event_loop.create_window(
                winit::window::Window::default_attributes()
                    .with_title("Operad Showcase")
                    .with_inner_size(winit::dpi::PhysicalSize::new(1280, 800))
                    .with_min_inner_size(winit::dpi::PhysicalSize::new(820, 560))
                    .with_visible(true),
            )?,
        );
        let size = nonzero_window_size(window.inner_size());

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone())?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
        }))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("operad-showcase-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            }))?;
        let surface_config = surface
            .get_default_config(&adapter, size.width, size.height)
            .ok_or("adapter does not support the native window surface")?;

        self.window_id = Some(window.id());
        self.renderer = Some(operad::WgpuSurfaceRenderer::new(
            surface,
            device,
            queue,
            surface_config,
        )?);
        self.window = Some(window);
        Ok(())
    }

    fn request_redraw(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn viewport(&self) -> Option<operad::UiSize> {
        let size = self.window.as_ref()?.inner_size();
        if size.width == 0 || size.height == 0 {
            None
        } else {
            Some(operad::UiSize::new(size.width as f32, size.height as f32))
        }
    }

    fn render(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        use operad::RendererAdapter;

        let Some(viewport) = self.viewport() else {
            return Ok(false);
        };
        let Some(renderer) = self.renderer.as_mut() else {
            return Ok(false);
        };

        let mut document = showcase::build_document(viewport, &self.state);
        let mut measurer = operad::ApproxTextMeasurer;
        let frame = operad::process_document_frame(
            &mut document,
            &mut measurer,
            operad::HostDocumentFrameRequest::new(
                viewport,
                operad::RenderTarget::window("operad-showcase", viewport),
                operad::HostFrameOutput::new(operad::HostInteractionState::default()),
            ),
        )?;
        let output = renderer.render_frame(frame.render_request, &operad::EmptyResourceResolver)?;
        if output.snapshot.is_some() {
            return Err("windowed showcase must present without snapshot readback".into());
        }
        self.presented_frames += 1;
        Ok(self
            .frame_limit
            .is_some_and(|frame_limit| self.presented_frames >= frame_limit))
    }

    fn pointer_down(&mut self) {
        let (Some(point), Some(viewport)) = (self.cursor, self.viewport()) else {
            return;
        };
        let layout = showcase::ShowcaseLayout::new(viewport);
        if self.state.pointer_down(point, &layout) {
            self.request_redraw();
        }
    }

    fn pointer_up(&mut self) {
        let (Some(point), Some(viewport)) = (self.cursor, self.viewport()) else {
            return;
        };
        let layout = showcase::ShowcaseLayout::new(viewport);
        if self.state.pointer_up(point, &layout) {
            self.request_redraw();
        }
    }

    fn pointer_moved(&mut self, point: operad::UiPoint) {
        self.cursor = Some(point);
        let Some(viewport) = self.viewport() else {
            return;
        };
        let layout = showcase::ShowcaseLayout::new(viewport);
        if self.state.pointer_moved(point, &layout) {
            self.request_redraw();
        }
    }

    fn key_pressed(&mut self, event: &winit::event::KeyEvent) {
        use winit::keyboard::{KeyCode, PhysicalKey};

        if event.state != winit::event::ElementState::Pressed {
            return;
        }
        let mut changed = false;
        match event.physical_key {
            PhysicalKey::Code(KeyCode::Escape) => {
                changed = self.state.dismiss_popups() || self.state.text_field_focused;
                self.state.text_field_focused = false;
            }
            PhysicalKey::Code(KeyCode::Backspace) if self.state.text_field_focused => {
                changed = self.state.backspace_text_field();
            }
            PhysicalKey::Code(KeyCode::KeyP)
                if self.modifiers.control_key() && self.modifiers.shift_key() =>
            {
                self.state.palette_open = true;
                self.state.menu_open = false;
                self.state.last_action = "Command palette opened".to_string();
                changed = true;
            }
            _ => {}
        }
        if self.state.text_field_focused && !self.modifiers.control_key() {
            if let Some(text) = event.text.as_ref() {
                changed |= self.state.insert_text_field(text.as_str());
            }
        }
        if changed {
            self.request_redraw();
        }
    }

    fn fail_and_exit(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        error: impl ToString,
    ) {
        self.error = Some(error.to_string());
        event_loop.exit();
    }
}

#[cfg(all(feature = "widgets", feature = "native-window"))]
impl winit::application::ApplicationHandler for ShowcaseWindowApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        if let Err(error) = self.init_window(event_loop) {
            self.fail_and_exit(event_loop, error);
            return;
        }
        self.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if Some(window_id) != self.window_id {
            return;
        }

        match event {
            winit::event::WindowEvent::CloseRequested | winit::event::WindowEvent::Destroyed => {
                event_loop.exit();
            }
            winit::event::WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    self.request_redraw();
                }
            }
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                self.pointer_moved(operad::UiPoint::new(position.x as f32, position.y as f32));
            }
            winit::event::WindowEvent::MouseInput { state, button, .. }
                if button == winit::event::MouseButton::Left =>
            {
                match state {
                    winit::event::ElementState::Pressed => self.pointer_down(),
                    winit::event::ElementState::Released => self.pointer_up(),
                }
            }
            winit::event::WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            winit::event::WindowEvent::KeyboardInput { event, .. } => {
                self.key_pressed(&event);
            }
            winit::event::WindowEvent::RedrawRequested => match self.render() {
                Ok(true) => event_loop.exit(),
                Ok(false) => {
                    if self.frame_limit.is_some() {
                        self.request_redraw();
                    }
                }
                Err(error) => self.fail_and_exit(event_loop, error),
            },
            _ => {}
        }
    }
}

#[cfg(all(feature = "widgets", feature = "native-window"))]
fn nonzero_window_size(size: winit::dpi::PhysicalSize<u32>) -> winit::dpi::PhysicalSize<u32> {
    winit::dpi::PhysicalSize::new(size.width.max(1), size.height.max(1))
}

#[cfg(all(feature = "widgets", feature = "native-window"))]
fn showcase_window_frame_limit() -> Result<Option<usize>, Box<dyn std::error::Error>> {
    let Some(value) = std::env::var_os("OPERAD_SHOWCASE_WINDOW_FRAMES") else {
        return Ok(None);
    };
    let frames = value
        .to_string_lossy()
        .parse::<usize>()
        .map_err(|error| format!("invalid OPERAD_SHOWCASE_WINDOW_FRAMES: {error}"))?;
    if frames == 0 {
        return Err("OPERAD_SHOWCASE_WINDOW_FRAMES must be greater than zero".into());
    }
    Ok(Some(frames))
}

#[cfg(feature = "widgets")]
mod showcase {
    use operad::widgets::*;
    use operad::*;

    const NATURAL_WIDTH: f32 = 1280.0;
    const NATURAL_HEIGHT: f32 = 780.0;
    const NAV_LABELS: [&str; 4] = ["File", "Edit", "View", "Run"];
    const WORKSPACE_LABELS: [&str; 4] = ["Sessions", "Editor", "Resources", "Diagnostics"];
    const PATH_LABELS: [&str; 3] = ["workspaces", "demo", "ui.rs"];

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ShowcaseHotspot {
        Nav(usize),
        Command,
        Workspace(usize),
        ColorPicker(usize),
        Path(usize),
        SelectableText,
        MenuItem(usize),
        PaletteItem(usize),
        PrimaryButton,
        SecondaryButton,
        Slider,
        TextField,
        Tab(usize),
    }

    #[cfg_attr(not(feature = "native-window"), allow(dead_code))]
    #[derive(Debug, Clone)]
    pub struct ShowcaseState {
        pub active_tab: usize,
        pub active_nav: Option<usize>,
        pub active_workspace: usize,
        pub active_path: usize,
        pub show_grid: bool,
        pub menu_open: bool,
        pub palette_open: bool,
        pub slider_value: f32,
        pub selected_hue: usize,
        pub selectable_text_selected: bool,
        pub text_field_focused: bool,
        pub text_field_value: String,
        pub dragging_slider: bool,
        pub last_action: String,
        hovered: Option<ShowcaseHotspot>,
    }

    impl Default for ShowcaseState {
        fn default() -> Self {
            Self {
                active_tab: 0,
                active_nav: None,
                active_workspace: 1,
                active_path: 2,
                show_grid: true,
                menu_open: false,
                palette_open: false,
                slider_value: 0.62,
                selected_hue: 2,
                selectable_text_selected: true,
                text_field_focused: false,
                text_field_value: "Editable value".to_string(),
                dragging_slider: false,
                last_action: "Ready".to_string(),
                hovered: None,
            }
        }
    }

    #[cfg_attr(not(feature = "native-window"), allow(dead_code))]
    impl ShowcaseState {
        pub fn pointer_down(&mut self, point: UiPoint, layout: &ShowcaseLayout) -> bool {
            self.hovered = layout.hotspot_at(point, self);
            if matches!(self.hovered, Some(ShowcaseHotspot::Slider)) {
                self.dragging_slider = true;
                self.update_slider(point, layout);
                return true;
            }
            self.hovered.is_some()
        }

        pub fn pointer_moved(&mut self, point: UiPoint, layout: &ShowcaseLayout) -> bool {
            if self.dragging_slider {
                self.update_slider(point, layout);
                return true;
            }
            let hovered = layout.hotspot_at(point, self);
            if self.hovered != hovered {
                self.hovered = hovered;
                return true;
            }
            false
        }

        pub fn pointer_up(&mut self, point: UiPoint, layout: &ShowcaseLayout) -> bool {
            let hotspot = layout.hotspot_at(point, self);
            self.hovered = hotspot;
            if self.dragging_slider {
                self.dragging_slider = false;
                self.update_slider(point, layout);
                return true;
            }

            if self.palette_open {
                if matches!(hotspot, Some(ShowcaseHotspot::Command)) {
                    self.palette_open = false;
                    self.last_action = "Command palette closed".to_string();
                    return true;
                }
                if let Some(ShowcaseHotspot::PaletteItem(index)) = hotspot {
                    self.palette_open = false;
                    self.last_action = match index {
                        0 => "Rename command selected".to_string(),
                        1 => "Export command selected".to_string(),
                        _ => "Run command selected".to_string(),
                    };
                    return true;
                }
                if !layout.palette.contains_point(point) {
                    self.palette_open = false;
                    return true;
                }
                return false;
            }

            if self.menu_open {
                if matches!(hotspot, Some(ShowcaseHotspot::Nav(index)) if self.active_nav == Some(index))
                {
                    self.menu_open = false;
                    self.last_action = "Menu closed".to_string();
                    return true;
                }
                if let Some(ShowcaseHotspot::MenuItem(0)) = hotspot {
                    if self.active_nav == Some(2) {
                        self.show_grid = !self.show_grid;
                    }
                    self.menu_open = false;
                    self.last_action = if self.active_nav == Some(2) {
                        if self.show_grid {
                            "Grid shown".to_string()
                        } else {
                            "Grid hidden".to_string()
                        }
                    } else {
                        format!(
                            "{} action selected",
                            self.active_nav
                                .and_then(|index| NAV_LABELS.get(index))
                                .copied()
                                .unwrap_or("Menu")
                        )
                    };
                    return true;
                }
                if matches!(hotspot, Some(ShowcaseHotspot::MenuItem(1))) {
                    self.palette_open = true;
                    self.menu_open = false;
                    self.last_action = "Command palette opened".to_string();
                    return true;
                }
                if !layout.menu_rect(self).contains_point(point) {
                    self.menu_open = false;
                }
            }

            match hotspot {
                Some(ShowcaseHotspot::Nav(index)) => {
                    self.active_nav = Some(index);
                    self.text_field_focused = false;
                    self.palette_open = false;
                    self.menu_open = true;
                    self.last_action = format!("{} menu opened", NAV_LABELS[index]);
                    true
                }
                Some(ShowcaseHotspot::Command) => {
                    self.text_field_focused = false;
                    self.palette_open = true;
                    self.menu_open = false;
                    self.last_action = "Command palette opened".to_string();
                    true
                }
                Some(ShowcaseHotspot::Workspace(index)) => {
                    self.active_workspace = index;
                    self.text_field_focused = false;
                    self.last_action = format!("{} workspace selected", WORKSPACE_LABELS[index]);
                    true
                }
                Some(ShowcaseHotspot::Path(index)) => {
                    self.active_path = index;
                    self.text_field_focused = false;
                    self.last_action = format!("{} selected", PATH_LABELS[index]);
                    true
                }
                Some(ShowcaseHotspot::ColorPicker(index)) => {
                    self.selected_hue = index;
                    self.text_field_focused = false;
                    self.last_action = format!("Hue {} selected", index + 1);
                    true
                }
                Some(ShowcaseHotspot::Tab(index)) => {
                    self.active_tab = index;
                    self.text_field_focused = false;
                    self.last_action = format!("{} tab selected", TAB_LABELS[index]);
                    true
                }
                Some(ShowcaseHotspot::PrimaryButton) => {
                    self.text_field_focused = false;
                    self.last_action = "Text button clicked".to_string();
                    true
                }
                Some(ShowcaseHotspot::SecondaryButton) => {
                    self.text_field_focused = false;
                    self.last_action = "Icon button clicked".to_string();
                    true
                }
                Some(ShowcaseHotspot::TextField) => {
                    self.text_field_focused = true;
                    self.last_action = "Text field focused".to_string();
                    true
                }
                Some(ShowcaseHotspot::SelectableText) => {
                    self.text_field_focused = false;
                    self.selectable_text_selected = !self.selectable_text_selected;
                    self.last_action = if self.selectable_text_selected {
                        "Selectable text selected".to_string()
                    } else {
                        "Selectable text cleared".to_string()
                    };
                    true
                }
                _ => false,
            }
        }

        pub fn dismiss_popups(&mut self) -> bool {
            let changed = self.menu_open || self.palette_open;
            self.menu_open = false;
            self.palette_open = false;
            changed
        }

        pub fn insert_text_field(&mut self, value: &str) -> bool {
            if !self.text_field_focused {
                return false;
            }
            let old_len = self.text_field_value.len();
            let remaining = 28usize.saturating_sub(self.text_field_value.chars().count());
            self.text_field_value
                .extend(value.chars().filter(|ch| !ch.is_control()).take(remaining));
            let changed = self.text_field_value.len() != old_len;
            if changed {
                self.last_action = "Text field edited".to_string();
            }
            changed
        }

        pub fn backspace_text_field(&mut self) -> bool {
            if !self.text_field_focused || self.text_field_value.is_empty() {
                return false;
            }
            self.text_field_value.pop();
            self.last_action = "Text field edited".to_string();
            true
        }

        fn hovered(&self, hotspot: ShowcaseHotspot) -> bool {
            self.hovered == Some(hotspot)
        }

        fn update_slider(&mut self, point: UiPoint, layout: &ShowcaseLayout) {
            let value =
                ((point.x - layout.slider_track.x) / layout.slider_track.width).clamp(0.0, 1.0);
            self.slider_value = value;
            self.last_action = format!("Slider set to {:.0}%", value * 100.0);
        }
    }

    #[cfg_attr(not(feature = "native-window"), allow(dead_code))]
    #[derive(Debug, Clone)]
    pub struct ShowcaseLayout {
        pub scale: f32,
        pub root: UiRect,
        pub nav_buttons: [UiRect; 4],
        pub command_button: UiRect,
        pub palette: UiRect,
        pub palette_items: [UiRect; 3],
        pub workspace_rows: [UiRect; 4],
        pub primary_button: UiRect,
        pub secondary_button: UiRect,
        pub slider_track: UiRect,
        pub text_field: UiRect,
        pub tabs: [UiRect; 3],
        pub color_square: UiRect,
        pub color_sliders: [UiRect; 3],
        pub path_rows: [UiRect; 3],
        pub selectable_text: UiRect,
    }

    impl ShowcaseLayout {
        pub fn new(viewport: UiSize) -> Self {
            let available_width = (viewport.width - 24.0).max(320.0);
            let available_height = (viewport.height - 24.0).max(240.0);
            let scale = (available_width / NATURAL_WIDTH)
                .min(available_height / NATURAL_HEIGHT)
                .clamp(0.45, 2.6);
            let width = NATURAL_WIDTH * scale;
            let height = NATURAL_HEIGHT * scale;
            let origin = UiPoint::new(
                (viewport.width - width) * 0.5,
                (viewport.height - height) * 0.5,
            );
            let rect = |x: f32, y: f32, width: f32, height: f32| {
                UiRect::new(
                    origin.x + x * scale,
                    origin.y + y * scale,
                    width * scale,
                    height * scale,
                )
            };
            let nav_buttons = [
                rect(16.0, 10.0, 44.0, 34.0),
                rect(64.0, 10.0, 44.0, 34.0),
                rect(112.0, 10.0, 44.0, 34.0),
                rect(160.0, 10.0, 44.0, 34.0),
            ];
            let palette = rect(380.0, 84.0, 540.0, 286.0);
            Self {
                scale,
                root: rect(0.0, 0.0, NATURAL_WIDTH, NATURAL_HEIGHT),
                nav_buttons,
                command_button: rect(374.0, 10.0, 420.0, 36.0),
                palette,
                palette_items: [
                    rect(404.0, 194.0, 492.0, 36.0),
                    rect(404.0, 238.0, 492.0, 36.0),
                    rect(404.0, 282.0, 492.0, 36.0),
                ],
                workspace_rows: [
                    rect(40.0, 132.0, 226.0, 30.0),
                    rect(40.0, 170.0, 226.0, 30.0),
                    rect(40.0, 208.0, 226.0, 30.0),
                    rect(40.0, 246.0, 226.0, 30.0),
                ],
                primary_button: rect(982.0, 162.0, 250.0, 38.0),
                secondary_button: rect(982.0, 210.0, 250.0, 38.0),
                slider_track: rect(982.0, 282.0, 250.0, 10.0),
                text_field: rect(982.0, 326.0, 250.0, 42.0),
                tabs: [
                    rect(338.0, 626.0, 116.0, 38.0),
                    rect(458.0, 626.0, 116.0, 38.0),
                    rect(578.0, 626.0, 116.0, 38.0),
                ],
                color_square: rect(52.0, 348.0, 166.0, 136.0),
                color_sliders: [
                    rect(52.0, 494.0, 190.0, 12.0),
                    rect(52.0, 514.0, 190.0, 12.0),
                    rect(52.0, 534.0, 190.0, 12.0),
                ],
                path_rows: [
                    rect(40.0, 562.0, 214.0, 24.0),
                    rect(40.0, 594.0, 214.0, 24.0),
                    rect(40.0, 626.0, 214.0, 24.0),
                ],
                selectable_text: rect(40.0, 660.0, 214.0, 30.0),
            }
        }

        fn hotspot_at(&self, point: UiPoint, state: &ShowcaseState) -> Option<ShowcaseHotspot> {
            if state.palette_open && self.palette.contains_point(point) {
                for (index, rect) in self.palette_items.iter().enumerate() {
                    if rect.contains_point(point) {
                        return Some(ShowcaseHotspot::PaletteItem(index));
                    }
                }
                return None;
            }
            if state.menu_open && self.menu_rect(state).contains_point(point) {
                for (index, rect) in self.menu_item_rects(state).iter().enumerate() {
                    if rect.contains_point(point) {
                        return Some(ShowcaseHotspot::MenuItem(index));
                    }
                }
                return None;
            }
            for (index, rect) in self.nav_buttons.iter().enumerate() {
                if rect.contains_point(point) {
                    return Some(ShowcaseHotspot::Nav(index));
                }
            }
            if self.command_button.contains_point(point) {
                return Some(ShowcaseHotspot::Command);
            }
            for (index, rect) in self.workspace_rows.iter().enumerate() {
                if rect.contains_point(point) {
                    return Some(ShowcaseHotspot::Workspace(index));
                }
            }
            if let Some(index) = self.color_picker_at(point) {
                return Some(ShowcaseHotspot::ColorPicker(index));
            }
            for (index, rect) in self.path_rows.iter().enumerate() {
                if rect.contains_point(point) {
                    return Some(ShowcaseHotspot::Path(index));
                }
            }
            if self.selectable_text.contains_point(point) {
                return Some(ShowcaseHotspot::SelectableText);
            }
            if self.primary_button.contains_point(point) {
                return Some(ShowcaseHotspot::PrimaryButton);
            }
            if self.secondary_button.contains_point(point) {
                return Some(ShowcaseHotspot::SecondaryButton);
            }
            if self.slider_hit_rect().contains_point(point) {
                return Some(ShowcaseHotspot::Slider);
            }
            if self.text_field.contains_point(point) {
                return Some(ShowcaseHotspot::TextField);
            }
            for (index, rect) in self.tabs.iter().enumerate() {
                if rect.contains_point(point) {
                    return Some(ShowcaseHotspot::Tab(index));
                }
            }
            None
        }

        fn menu_rect(&self, state: &ShowcaseState) -> UiRect {
            let index = state
                .active_nav
                .unwrap_or(0)
                .min(self.nav_buttons.len() - 1);
            let anchor = self.nav_buttons[index];
            let width = 220.0 * self.scale;
            let x = anchor.x.min(self.root.right() - width - 12.0 * self.scale);
            UiRect::new(x, self.root.y + 50.0 * self.scale, width, 92.0 * self.scale)
        }

        fn menu_item_rects(&self, state: &ShowcaseState) -> [UiRect; 2] {
            let menu = self.menu_rect(state);
            [
                UiRect::new(
                    menu.x + 8.0 * self.scale,
                    menu.y + 10.0 * self.scale,
                    menu.width - 16.0 * self.scale,
                    32.0 * self.scale,
                ),
                UiRect::new(
                    menu.x + 8.0 * self.scale,
                    menu.y + 46.0 * self.scale,
                    menu.width - 16.0 * self.scale,
                    32.0 * self.scale,
                ),
            ]
        }

        fn slider_hit_rect(&self) -> UiRect {
            UiRect::new(
                self.slider_track.x,
                self.slider_track.y - 10.0 * self.scale,
                self.slider_track.width,
                self.slider_track.height + 20.0 * self.scale,
            )
        }

        fn color_picker_at(&self, point: UiPoint) -> Option<usize> {
            if self.color_square.contains_point(point) {
                return Some(0);
            }
            if self.color_sliders[0].contains_point(point) {
                let t = ((point.x - self.color_sliders[0].x) / self.color_sliders[0].width)
                    .clamp(0.0, 0.999);
                return Some((t * COLOR_WHEEL_SEGMENTS as f32) as usize);
            }
            if self.color_sliders[1].contains_point(point)
                || self.color_sliders[2].contains_point(point)
            {
                return Some(COLOR_WHEEL_SEGMENTS / 2);
            }
            None
        }
    }

    const TAB_LABELS: [&str; 3] = ["Preview", "Actions", "A11y"];
    const COLOR_WHEEL_SEGMENTS: usize = 12;

    enum ColorSlider {
        Hue,
        Alpha(f32),
        Checker,
    }

    pub fn build_document(viewport: UiSize, state: &ShowcaseState) -> UiDocument {
        let layout = ShowcaseLayout::new(viewport);
        let mut document = UiDocument::new(root_style(viewport.width, viewport.height));
        let root = document.root;
        document.node_mut(root).visual = UiVisual::panel(bg(), None, 0.0);

        add_background(&mut document, root, viewport, &layout);
        add_header(&mut document, root, state, &layout);
        add_left_panel(&mut document, root, state, &layout);
        add_editor(&mut document, root, state, &layout);
        add_right_panel(&mut document, root, state, &layout);
        add_status_bar(&mut document, root, state, &layout);
        if state.menu_open {
            add_nav_menu(&mut document, root, state, &layout);
        }
        if state.palette_open {
            add_command_palette(&mut document, root, state, &layout);
        }
        document
    }

    fn add_background(
        document: &mut UiDocument,
        root: UiNodeId,
        viewport: UiSize,
        layout: &ShowcaseLayout,
    ) {
        add_panel(
            document,
            root,
            "showcase.content",
            layout.root,
            ColorRgba::new(24, 31, 33, 255),
            Some(stroke(ColorRgba::new(70, 84, 86, 255))),
            0.0,
        );
        document.add_child(
            root,
            UiNode::scene(
                "showcase.background.wash",
                vec![ScenePrimitive::Rect(PaintRect::solid(
                    UiRect::new(0.0, 0.0, viewport.width, viewport.height),
                    bg(),
                ))],
                layout::absolute(0.0, 0.0, viewport.width, viewport.height),
            ),
        );
    }

    fn add_color_picker(
        document: &mut UiDocument,
        root: UiNodeId,
        state: &ShowcaseState,
        layout: &ShowcaseLayout,
    ) {
        let s = layout.scale;
        let bounds = layout.color_square;
        let local = UiRect::new(0.0, 0.0, bounds.width, bounds.height);
        let mut primitives = vec![ScenePrimitive::Rect(
            PaintRect::solid(local, surface()).stroke(stroke(line())),
        )];
        for row in 0..8 {
            for col in 0..12 {
                let saturation = col as f32 / 11.0;
                let value = 1.0 - row as f32 / 7.0;
                primitives.push(ScenePrimitive::Rect(PaintRect::solid(
                    UiRect::new(
                        col as f32 * local.width / 12.0,
                        row as f32 * local.height / 8.0,
                        local.width / 12.0 + 0.5 * s,
                        local.height / 8.0 + 0.5 * s,
                    ),
                    hsv_to_rgb(selected_hue(state), saturation, value),
                )));
            }
        }
        primitives.push(ScenePrimitive::Circle {
            center: UiPoint::new(local.width * 0.74, local.height * 0.36),
            radius: 5.0 * s,
            fill: ColorRgba::WHITE,
            stroke: Some(stroke(ColorRgba::new(24, 30, 32, 255))),
        });
        document.add_child(
            root,
            UiNode::scene("color.picker.square.scene", primitives, rect_layout(bounds)),
        );
        add_color_slider(
            document,
            root,
            layout.color_sliders[0],
            "color.hue",
            ColorSlider::Hue,
        );
        add_color_slider(
            document,
            root,
            layout.color_sliders[1],
            "color.alpha",
            ColorSlider::Alpha(selected_hue(state)),
        );
        add_color_slider(
            document,
            root,
            layout.color_sliders[2],
            "color.checker",
            ColorSlider::Checker,
        );
    }

    fn add_color_slider(
        document: &mut UiDocument,
        root: UiNodeId,
        bounds: UiRect,
        id: &str,
        slider: ColorSlider,
    ) {
        let s = bounds.height / 12.0;
        let local = UiRect::new(0.0, 0.0, bounds.width, bounds.height);
        let mut primitives = Vec::new();
        match slider {
            ColorSlider::Hue => {
                for index in 0..COLOR_WHEEL_SEGMENTS {
                    let x = index as f32 * local.width / COLOR_WHEEL_SEGMENTS as f32;
                    primitives.push(ScenePrimitive::Rect(PaintRect::solid(
                        UiRect::new(
                            x,
                            0.0,
                            local.width / COLOR_WHEEL_SEGMENTS as f32 + 0.5 * s,
                            local.height,
                        ),
                        color_wheel_color(index),
                    )));
                }
            }
            ColorSlider::Alpha(hue) => {
                add_checker_primitives(&mut primitives, local, 6.0 * s);
                for index in 0..16 {
                    let t = index as f32 / 15.0;
                    let x = index as f32 * local.width / 16.0;
                    let mut color = hsv_to_rgb(hue, 0.72, 0.88);
                    color.a = (t * 255.0).round() as u8;
                    primitives.push(ScenePrimitive::Rect(PaintRect::solid(
                        UiRect::new(x, 0.0, local.width / 16.0 + 0.5 * s, local.height),
                        color,
                    )));
                }
            }
            ColorSlider::Checker => add_checker_primitives(&mut primitives, local, 6.0 * s),
        }
        primitives.push(ScenePrimitive::Rect(
            PaintRect::solid(local, ColorRgba::TRANSPARENT).stroke(stroke(line())),
        ));
        document.add_child(root, UiNode::scene(id, primitives, rect_layout(bounds)));
    }

    fn add_checker_primitives(primitives: &mut Vec<ScenePrimitive>, bounds: UiRect, cell: f32) {
        let cols = (bounds.width / cell).ceil() as usize;
        let rows = (bounds.height / cell).ceil() as usize;
        for row in 0..rows {
            for col in 0..cols {
                let fill = if (row + col) % 2 == 0 {
                    ColorRgba::new(96, 106, 108, 255)
                } else {
                    ColorRgba::new(52, 61, 63, 255)
                };
                primitives.push(ScenePrimitive::Rect(PaintRect::solid(
                    UiRect::new(col as f32 * cell, row as f32 * cell, cell, cell),
                    fill,
                )));
            }
        }
    }

    fn add_header(
        document: &mut UiDocument,
        root: UiNodeId,
        state: &ShowcaseState,
        layout: &ShowcaseLayout,
    ) {
        let s = layout.scale;
        add_panel(
            document,
            root,
            "showcase.header",
            r(layout, 0.0, 0.0, NATURAL_WIDTH, 56.0),
            ColorRgba::new(38, 47, 49, 242),
            Some(stroke(ColorRgba::new(81, 95, 97, 255))),
            0.0,
        );
        for (index, label) in ["File", "Edit", "View", "Run"].into_iter().enumerate() {
            let x = 16.0 + index as f32 * 48.0;
            let active = state.active_nav == Some(index);
            add_button(
                document,
                root,
                &format!("nav.{label}"),
                label,
                r(layout, x, 10.0, 44.0, 34.0),
                active,
                state.hovered(ShowcaseHotspot::Nav(index)),
            );
        }
        let command_active = state.palette_open;
        let command_hovered = state.hovered(ShowcaseHotspot::Command);
        add_panel(
            document,
            root,
            "command.palette",
            layout.command_button,
            interactive_fill(command_active, command_hovered),
            Some(stroke(interactive_stroke(command_active, command_hovered))),
            3.0 * s,
        );
        add_text(
            document,
            root,
            "command.palette.label",
            "Command Palette",
            inset(
                layout.command_button,
                16.0 * s,
                10.0 * s,
                190.0 * s,
                18.0 * s,
            ),
            13.0 * s,
            ColorRgba::WHITE,
            FontWeight::NORMAL,
        );
        add_text(
            document,
            root,
            "command.palette.shortcut",
            "Ctrl+Shift+P",
            inset(
                layout.command_button,
                300.0 * s,
                10.0 * s,
                104.0 * s,
                18.0 * s,
            ),
            13.0 * s,
            muted(),
            FontWeight::NORMAL,
        );
    }

    fn add_left_panel(
        document: &mut UiDocument,
        root: UiNodeId,
        state: &ShowcaseState,
        layout: &ShowcaseLayout,
    ) {
        let s = layout.scale;
        add_panel(
            document,
            root,
            "left.panel",
            r(layout, 24.0, 78.0, 260.0, 620.0),
            panel(),
            Some(stroke(line())),
            2.0 * s,
        );
        add_text(
            document,
            root,
            "left.title",
            "Workspace",
            r(layout, 44.0, 100.0, 180.0, 22.0),
            15.0 * s,
            text(),
            FontWeight::BOLD,
        );
        for (index, label) in ["Sessions", "Editor", "Resources", "Diagnostics"]
            .into_iter()
            .enumerate()
        {
            let y = 132.0 + index as f32 * 38.0;
            let active = state.active_workspace == index;
            let hovered = state.hovered(ShowcaseHotspot::Workspace(index));
            add_panel(
                document,
                root,
                &format!("left.row.{index}"),
                r(layout, 40.0, y, 226.0, 30.0),
                interactive_fill(active, hovered),
                Some(stroke(interactive_stroke(active, hovered))),
                2.0 * s,
            );
            add_text(
                document,
                root,
                &format!("left.row.{index}.text"),
                label,
                r(layout, 54.0, y + 7.0, 160.0, 18.0),
                13.0 * s,
                if active { ColorRgba::WHITE } else { text() },
                FontWeight::NORMAL,
            );
        }

        add_text(
            document,
            root,
            "picker.title",
            "Picker primitives",
            r(layout, 40.0, 316.0, 180.0, 20.0),
            13.0 * s,
            text(),
            FontWeight::BOLD,
        );
        add_color_picker(document, root, state, layout);
        for (index, label) in ["workspaces", "demo", "ui.rs"].into_iter().enumerate() {
            let y = 562.0 + index as f32 * 32.0;
            let active = state.active_path == index;
            let hovered = state.hovered(ShowcaseHotspot::Path(index));
            add_panel(
                document,
                root,
                &format!("path.{index}"),
                r(layout, 40.0, y, 214.0, 24.0),
                interactive_fill(active, hovered),
                Some(stroke(interactive_stroke(active, hovered))),
                2.0 * s,
            );
            add_text(
                document,
                root,
                &format!("path.{index}.text"),
                label,
                r(layout, 52.0, y + 5.0, 160.0, 16.0),
                12.0 * s,
                if active { ColorRgba::WHITE } else { muted() },
                FontWeight::NORMAL,
            );
        }
        if state.selectable_text_selected {
            add_panel(
                document,
                root,
                "copy.policy.selection",
                layout.selectable_text,
                ColorRgba::new(84, 118, 126, 220),
                Some(stroke(accent())),
                2.0 * s,
            );
        } else {
            add_panel(
                document,
                root,
                "copy.policy.idle",
                layout.selectable_text,
                if state.hovered(ShowcaseHotspot::SelectableText) {
                    hover()
                } else {
                    surface()
                },
                Some(stroke(line())),
                2.0 * s,
            );
        }
        add_text(
            document,
            root,
            "copy.policy",
            "Selected text: latency 11ms",
            inset(
                layout.selectable_text,
                12.0 * s,
                8.0 * s,
                186.0 * s,
                18.0 * s,
            ),
            12.0 * s,
            ColorRgba::WHITE,
            FontWeight::NORMAL,
        );
    }

    fn add_editor(
        document: &mut UiDocument,
        root: UiNodeId,
        state: &ShowcaseState,
        layout: &ShowcaseLayout,
    ) {
        let s = layout.scale;
        add_panel(
            document,
            root,
            "editor.panel",
            r(layout, 304.0, 78.0, 640.0, 620.0),
            panel(),
            Some(stroke(line())),
            2.0 * s,
        );
        add_text(
            document,
            root,
            "editor.title",
            "Session timeline",
            r(layout, 328.0, 102.0, 180.0, 22.0),
            16.0 * s,
            text(),
            FontWeight::BOLD,
        );
        add_timeline_scene(document, root, state, layout);
        add_resource_row(document, root, layout);
        add_tabs(document, root, state, layout);
    }

    fn add_timeline_scene(
        document: &mut UiDocument,
        root: UiNodeId,
        state: &ShowcaseState,
        layout: &ShowcaseLayout,
    ) {
        let s = layout.scale;
        let bounds = r(layout, 328.0, 138.0, 592.0, 330.0);
        let local = UiRect::new(0.0, 0.0, bounds.width, bounds.height);
        let mut primitives = Vec::new();
        primitives.push(ScenePrimitive::Rect(
            PaintRect::solid(local, ColorRgba::new(35, 49, 50, 255))
                .stroke(stroke(ColorRgba::new(84, 102, 104, 255))),
        ));
        if state.show_grid {
            for i in 0..13 {
                let x = i as f32 * local.width / 12.0;
                primitives.push(ScenePrimitive::Line {
                    from: UiPoint::new(x, 42.0 * s),
                    to: UiPoint::new(x, local.bottom() - 36.0 * s),
                    stroke: StrokeStyle::new(ColorRgba::new(87, 107, 109, 160), 1.0 * s),
                });
            }
            for i in 0..5 {
                let y = (70.0 + i as f32 * 46.0) * s;
                primitives.push(ScenePrimitive::Line {
                    from: UiPoint::new(22.0 * s, y),
                    to: UiPoint::new(local.right() - 22.0 * s, y),
                    stroke: StrokeStyle::new(ColorRgba::new(87, 107, 109, 160), 1.0 * s),
                });
            }
        }
        for (index, color) in [
            ColorRgba::new(102, 198, 218, 255),
            ColorRgba::new(177, 155, 220, 255),
            ColorRgba::new(132, 205, 174, 255),
        ]
        .into_iter()
        .enumerate()
        {
            let x = (48.0 + index as f32 * 160.0) * s;
            let y = (92.0 + index as f32 * 58.0) * s;
            primitives.push(ScenePrimitive::Rect(
                PaintRect::solid(UiRect::new(x, y, 130.0 * s, 34.0 * s), color)
                    .stroke(stroke(ColorRgba::new(220, 236, 238, 255))),
            ));
        }
        let automation_y = local.bottom() - 58.0 * s;
        let points = [
            UiPoint::new(44.0 * s, automation_y),
            UiPoint::new(92.0 * s, automation_y - 16.0 * s),
            UiPoint::new(214.0 * s, automation_y - 16.0 * s),
            UiPoint::new(286.0 * s, automation_y),
            UiPoint::new(404.0 * s, automation_y - 24.0 * s),
            UiPoint::new(544.0 * s, automation_y - 24.0 * s),
        ];
        for segment in points.windows(2) {
            primitives.push(ScenePrimitive::Line {
                from: segment[0],
                to: segment[1],
                stroke: StrokeStyle::new(accent_warm(), 2.0 * s),
            });
        }
        for point in points {
            primitives.push(ScenePrimitive::Circle {
                center: point,
                radius: 4.0 * s,
                fill: accent_warm(),
                stroke: Some(stroke(ColorRgba::new(45, 42, 32, 255))),
            });
        }
        document.add_child(
            root,
            UiNode::scene("editor.timeline.scene", primitives, rect_layout(bounds)),
        );
        add_text(
            document,
            root,
            "editor.scene.label",
            "Audio lanes with clip blocks and automation curve",
            r(layout, 346.0, 150.0, 330.0, 20.0),
            13.0 * s,
            text(),
            FontWeight::NORMAL,
        );
    }

    fn add_resource_row(document: &mut UiDocument, root: UiNodeId, layout: &ShowcaseLayout) {
        let s = layout.scale;
        let waveform = r(layout, 328.0, 490.0, 280.0, 106.0);
        let image = r(layout, 624.0, 490.0, 296.0, 106.0);
        add_panel(
            document,
            root,
            "waveform.panel",
            waveform,
            surface(),
            Some(stroke(line())),
            2.0 * s,
        );
        add_text(
            document,
            root,
            "waveform.title",
            "WGPU canvas waveform",
            inset(waveform, 12.0 * s, 10.0 * s, 250.0 * s, 18.0 * s),
            12.0 * s,
            text(),
            FontWeight::BOLD,
        );
        add_waveform_scene(
            document,
            root,
            inset(waveform, 12.0 * s, 38.0 * s, 256.0 * s, 48.0 * s),
            s,
        );

        add_panel(
            document,
            root,
            "image.panel",
            image,
            surface(),
            Some(stroke(line())),
            2.0 * s,
        );
        add_text(
            document,
            root,
            "image.title",
            "Procedural texture preview",
            inset(image, 12.0 * s, 10.0 * s, 250.0 * s, 18.0 * s),
            12.0 * s,
            text(),
            FontWeight::BOLD,
        );
        add_texture_preview_scene(
            document,
            root,
            inset(image, 12.0 * s, 38.0 * s, 272.0 * s, 48.0 * s),
            s,
        );
    }

    fn add_waveform_scene(document: &mut UiDocument, root: UiNodeId, bounds: UiRect, scale: f32) {
        let local = UiRect::new(0.0, 0.0, bounds.width, bounds.height);
        document.add_child(
            root,
            UiNode::canvas(
                "waveform.canvas",
                "showcase.waveform.wgpu",
                rect_layout(bounds),
            )
            .with_visual(UiVisual::panel(
                ColorRgba::new(34, 42, 40, 255),
                Some(stroke(ColorRgba::new(79, 94, 90, 255))),
                2.0 * scale,
            )),
        );
        let mut primitives = Vec::new();
        let center_y = local.height * 0.5;
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(0.0, center_y),
            to: UiPoint::new(local.width, center_y),
            stroke: StrokeStyle::new(ColorRgba::new(87, 102, 102, 180), 1.0 * scale),
        });
        let mut previous = UiPoint::new(0.0, center_y);
        for index in 1..48 {
            let t = index as f32 / 47.0;
            let x = local.width * t;
            let amp = ((t * 18.0).sin() * 0.55 + (t * 43.0).cos() * 0.25) * local.height * 0.38;
            let point = UiPoint::new(x, center_y + amp);
            primitives.push(ScenePrimitive::Line {
                from: previous,
                to: point,
                stroke: StrokeStyle::new(ColorRgba::new(149, 218, 178, 255), 2.0 * scale),
            });
            previous = point;
        }
        document.add_child(
            root,
            UiNode::scene("waveform.scene", primitives, rect_layout(bounds)),
        );
    }

    fn add_texture_preview_scene(
        document: &mut UiDocument,
        root: UiNodeId,
        bounds: UiRect,
        scale: f32,
    ) {
        let local = UiRect::new(0.0, 0.0, bounds.width, bounds.height);
        let mut primitives = vec![ScenePrimitive::Rect(
            PaintRect::solid(local, ColorRgba::new(31, 39, 40, 255))
                .stroke(stroke(ColorRgba::new(79, 94, 96, 255))),
        )];
        let cell = 24.0 * scale;
        for row in 0..2 {
            for col in 0..12 {
                let fill = if (row + col) % 2 == 0 {
                    ColorRgba::new(70, 99, 104, 255)
                } else {
                    ColorRgba::new(43, 59, 62, 255)
                };
                primitives.push(ScenePrimitive::Rect(PaintRect::solid(
                    UiRect::new(col as f32 * cell, row as f32 * cell, cell, cell),
                    fill,
                )));
            }
        }
        for index in 0..7 {
            let x = (18.0 + index as f32 * 38.0) * scale;
            primitives.push(ScenePrimitive::Circle {
                center: UiPoint::new(x, local.height * 0.5),
                radius: (8.0 + (index % 3) as f32 * 3.0) * scale,
                fill: color_wheel_color((index * 2) % COLOR_WHEEL_SEGMENTS),
                stroke: Some(stroke(ColorRgba::new(218, 232, 232, 210))),
            });
        }
        document.add_child(
            root,
            UiNode::scene("texture.preview.scene", primitives, rect_layout(bounds)),
        );
    }

    fn add_tabs(
        document: &mut UiDocument,
        root: UiNodeId,
        state: &ShowcaseState,
        layout: &ShowcaseLayout,
    ) {
        let s = layout.scale;
        add_panel(
            document,
            root,
            "tabs.panel",
            r(layout, 328.0, 620.0, 592.0, 52.0),
            surface(),
            Some(stroke(line())),
            2.0 * s,
        );
        for (index, rect) in layout.tabs.iter().enumerate() {
            let active = state.active_tab == index;
            let hovered = state.hovered(ShowcaseHotspot::Tab(index));
            add_panel(
                document,
                root,
                &format!("tab.{index}"),
                *rect,
                interactive_fill(active, hovered),
                Some(stroke(interactive_stroke(active, hovered))),
                2.0 * s,
            );
            add_text(
                document,
                root,
                &format!("tab.{index}.label"),
                TAB_LABELS[index],
                inset(*rect, 12.0 * s, 11.0 * s, 92.0 * s, 18.0 * s),
                13.0 * s,
                if active { ColorRgba::WHITE } else { text() },
                FontWeight::BOLD,
            );
        }
    }

    fn add_right_panel(
        document: &mut UiDocument,
        root: UiNodeId,
        state: &ShowcaseState,
        layout: &ShowcaseLayout,
    ) {
        let s = layout.scale;
        add_panel(
            document,
            root,
            "right.panel",
            r(layout, 964.0, 78.0, 292.0, 620.0),
            panel(),
            Some(stroke(line())),
            2.0 * s,
        );
        add_text(
            document,
            root,
            "right.title",
            "Interaction + Data",
            r(layout, 982.0, 102.0, 220.0, 22.0),
            16.0 * s,
            text(),
            FontWeight::BOLD,
        );
        add_text(
            document,
            root,
            "right.subtitle",
            "Click buttons, drag the slider, open menus",
            r(layout, 982.0, 132.0, 250.0, 20.0),
            12.0 * s,
            muted(),
            FontWeight::NORMAL,
        );
        add_button(
            document,
            root,
            "right.primary",
            "Text button target",
            layout.primary_button,
            false,
            state.hovered(ShowcaseHotspot::PrimaryButton),
        );
        add_button(
            document,
            root,
            "right.secondary",
            "Icon button target",
            layout.secondary_button,
            false,
            state.hovered(ShowcaseHotspot::SecondaryButton),
        );
        add_slider(document, root, state, layout);
        add_text_field(document, root, state, layout);
        add_data_table(document, root, layout);
    }

    fn add_slider(
        document: &mut UiDocument,
        root: UiNodeId,
        state: &ShowcaseState,
        layout: &ShowcaseLayout,
    ) {
        let s = layout.scale;
        add_text(
            document,
            root,
            "slider.label",
            "Gain",
            r(layout, 982.0, 258.0, 80.0, 18.0),
            12.0 * s,
            text(),
            FontWeight::BOLD,
        );
        add_panel(
            document,
            root,
            "slider.track",
            layout.slider_track,
            if state.hovered(ShowcaseHotspot::Slider) || state.dragging_slider {
                hover()
            } else {
                ColorRgba::new(45, 56, 56, 255)
            },
            Some(stroke(if state.dragging_slider {
                accent_warm()
            } else if state.hovered(ShowcaseHotspot::Slider) {
                accent()
            } else {
                line()
            })),
            5.0 * s,
        );
        add_panel(
            document,
            root,
            "slider.fill",
            UiRect::new(
                layout.slider_track.x,
                layout.slider_track.y,
                layout.slider_track.width * state.slider_value,
                layout.slider_track.height,
            ),
            accent(),
            None,
            5.0 * s,
        );
        let thumb_x = layout.slider_track.x + layout.slider_track.width * state.slider_value;
        add_panel(
            document,
            root,
            "slider.thumb",
            UiRect::new(
                thumb_x - 5.0 * s,
                layout.slider_track.y - 5.0 * s,
                10.0 * s,
                20.0 * s,
            ),
            ColorRgba::WHITE,
            Some(stroke(ColorRgba::new(37, 48, 48, 255))),
            2.0 * s,
        );
    }

    fn add_text_field(
        document: &mut UiDocument,
        root: UiNodeId,
        state: &ShowcaseState,
        layout: &ShowcaseLayout,
    ) {
        let s = layout.scale;
        add_panel(
            document,
            root,
            "text.field",
            layout.text_field,
            if state.hovered(ShowcaseHotspot::TextField) || state.text_field_focused {
                ColorRgba::new(43, 56, 58, 255)
            } else {
                ColorRgba::new(37, 47, 48, 255)
            },
            Some(stroke(if state.text_field_focused {
                accent_warm()
            } else if state.hovered(ShowcaseHotspot::TextField) {
                accent()
            } else {
                line()
            })),
            2.0 * s,
        );
        let text_font_size = 13.0 * s;
        let char_width = text_font_size * 0.55;
        let value_width =
            (state.text_field_value.chars().count() as f32 * char_width).clamp(0.0, 188.0 * s);
        add_text(
            document,
            root,
            "text.field.value",
            &state.text_field_value,
            inset(layout.text_field, 12.0 * s, 12.0 * s, 196.0 * s, 18.0 * s),
            text_font_size,
            ColorRgba::WHITE,
            FontWeight::NORMAL,
        );
        if state.text_field_focused {
            add_panel(
                document,
                root,
                "text.field.caret",
                inset(
                    layout.text_field,
                    12.0 * s + value_width,
                    10.0 * s,
                    1.5 * s,
                    22.0 * s,
                ),
                accent_warm(),
                None,
                0.0,
            );
        }
        add_text(
            document,
            root,
            "last.action",
            &state.last_action,
            r(layout, 982.0, 382.0, 250.0, 34.0),
            12.0 * s,
            accent_warm(),
            FontWeight::NORMAL,
        );
    }

    fn add_data_table(document: &mut UiDocument, root: UiNodeId, layout: &ShowcaseLayout) {
        let s = layout.scale;
        let table = r(layout, 982.0, 440.0, 250.0, 148.0);
        add_panel(
            document,
            root,
            "data.table",
            table,
            surface(),
            Some(stroke(line())),
            2.0 * s,
        );
        add_panel(
            document,
            root,
            "data.header",
            inset(table, 0.0, 0.0, table.width, 30.0 * s),
            ColorRgba::new(82, 95, 98, 255),
            None,
            0.0,
        );
        add_text(
            document,
            root,
            "data.h0",
            "Node",
            inset(table, 12.0 * s, 7.0 * s, 80.0 * s, 16.0 * s),
            12.0 * s,
            text(),
            FontWeight::BOLD,
        );
        add_text(
            document,
            root,
            "data.h1",
            "State",
            inset(table, 132.0 * s, 7.0 * s, 80.0 * s, 16.0 * s),
            12.0 * s,
            text(),
            FontWeight::BOLD,
        );
        for index in 0..4 {
            let y = table.y + (40.0 + index as f32 * 24.0) * s;
            add_text(
                document,
                root,
                &format!("data.row.{index}.node"),
                &format!("node-{index}"),
                UiRect::new(table.x + 12.0 * s, y, 90.0 * s, 16.0 * s),
                11.0 * s,
                muted(),
                FontWeight::NORMAL,
            );
            add_text(
                document,
                root,
                &format!("data.row.{index}.state"),
                if index == 1 { "selected" } else { "ready" },
                UiRect::new(table.x + 132.0 * s, y, 90.0 * s, 16.0 * s),
                11.0 * s,
                if index == 1 { accent() } else { muted() },
                FontWeight::NORMAL,
            );
        }
    }

    fn add_status_bar(
        document: &mut UiDocument,
        root: UiNodeId,
        state: &ShowcaseState,
        layout: &ShowcaseLayout,
    ) {
        let s = layout.scale;
        let rect = r(layout, 304.0, 710.0, 952.0, 46.0);
        add_panel(
            document,
            root,
            "status.bar",
            rect,
            ColorRgba::new(36, 45, 46, 255),
            Some(stroke(line())),
            2.0 * s,
        );
        add_text(
            document,
            root,
            "status.text",
            &format!(
                "Tab: {}    Grid: {}    Slider: {:.0}%    {}",
                TAB_LABELS[state.active_tab],
                if state.show_grid { "on" } else { "off" },
                state.slider_value * 100.0,
                state.last_action
            ),
            inset(rect, 16.0 * s, 14.0 * s, 880.0 * s, 18.0 * s),
            12.0 * s,
            text(),
            FontWeight::NORMAL,
        );
    }

    fn add_nav_menu(
        document: &mut UiDocument,
        root: UiNodeId,
        state: &ShowcaseState,
        layout: &ShowcaseLayout,
    ) {
        let s = layout.scale;
        let menu_rect = layout.menu_rect(state);
        add_panel(
            document,
            root,
            "nav.menu",
            menu_rect,
            ColorRgba::new(54, 63, 67, 255),
            Some(stroke(ColorRgba::new(104, 122, 128, 255))),
            2.0 * s,
        );
        let labels = menu_labels(state);
        let item_rects = layout.menu_item_rects(state);
        for (index, label) in labels.into_iter().enumerate() {
            let rect = item_rects[index];
            let hovered = state.hovered(ShowcaseHotspot::MenuItem(index));
            add_panel(
                document,
                root,
                &format!("nav.menu.item.{index}"),
                rect,
                if hovered {
                    hover()
                } else if index == 0 {
                    ColorRgba::new(66, 82, 85, 255)
                } else {
                    ColorRgba::TRANSPARENT
                },
                None,
                1.0 * s,
            );
            add_text(
                document,
                root,
                &format!("nav.menu.item.{index}.label"),
                label,
                inset(rect, 12.0 * s, 8.0 * s, 180.0 * s, 16.0 * s),
                12.0 * s,
                ColorRgba::WHITE,
                FontWeight::NORMAL,
            );
        }
    }

    fn menu_labels(state: &ShowcaseState) -> [&'static str; 2] {
        match state.active_nav.unwrap_or(0) {
            0 => ["New session", "Open command palette"],
            1 => ["Rename selection", "Open command palette"],
            2 if state.show_grid => ["Hide grid", "Open command palette"],
            2 => ["Show grid", "Open command palette"],
            3 => ["Run preview", "Open command palette"],
            _ => ["Menu action", "Open command palette"],
        }
    }

    fn add_command_palette(
        document: &mut UiDocument,
        root: UiNodeId,
        state: &ShowcaseState,
        layout: &ShowcaseLayout,
    ) {
        let s = layout.scale;
        add_panel(
            document,
            root,
            "palette",
            layout.palette,
            ColorRgba::new(50, 57, 64, 252),
            Some(stroke(ColorRgba::new(129, 149, 158, 255))),
            3.0 * s,
        );
        add_text(
            document,
            root,
            "palette.title",
            "Command Palette",
            inset(layout.palette, 22.0 * s, 18.0 * s, 220.0 * s, 22.0 * s),
            16.0 * s,
            ColorRgba::WHITE,
            FontWeight::BOLD,
        );
        add_panel(
            document,
            root,
            "palette.input",
            inset(layout.palette, 22.0 * s, 54.0 * s, 476.0 * s, 36.0 * s),
            ColorRgba::new(38, 46, 51, 255),
            Some(stroke(accent())),
            2.0 * s,
        );
        add_text(
            document,
            root,
            "palette.input.placeholder",
            "Type a command",
            inset(layout.palette, 36.0 * s, 63.0 * s, 180.0 * s, 18.0 * s),
            13.0 * s,
            muted(),
            FontWeight::NORMAL,
        );
        add_panel(
            document,
            root,
            "palette.input.caret",
            inset(layout.palette, 36.0 * s, 61.0 * s, 1.5 * s, 22.0 * s),
            accent_warm(),
            None,
            0.0,
        );
        for (index, label) in ["Rename selection", "Export snapshot", "Run preview"]
            .into_iter()
            .enumerate()
        {
            let rect = layout.palette_items[index];
            let hovered = state.hovered(ShowcaseHotspot::PaletteItem(index));
            add_panel(
                document,
                root,
                &format!("palette.item.{index}"),
                rect,
                if hovered {
                    hover()
                } else if index == 0 {
                    ColorRgba::new(66, 82, 85, 255)
                } else {
                    ColorRgba::TRANSPARENT
                },
                None,
                2.0 * s,
            );
            add_text(
                document,
                root,
                &format!("palette.item.{index}.label"),
                label,
                inset(rect, 14.0 * s, 10.0 * s, 260.0 * s, 18.0 * s),
                13.0 * s,
                ColorRgba::WHITE,
                FontWeight::NORMAL,
            );
        }
    }

    fn add_button(
        document: &mut UiDocument,
        root: UiNodeId,
        name: &str,
        label: &str,
        rect: UiRect,
        active: bool,
        hovered: bool,
    ) -> UiNodeId {
        button(
            document,
            root,
            name,
            label,
            ButtonOptions {
                layout: layout::with_absolute_position(
                    layout::with_padding_all(
                        layout::with_centered_children(layout::fixed(rect.width, rect.height)),
                        10.0 * (rect.height / 34.0).clamp(0.6, 1.4),
                    ),
                    rect.x,
                    rect.y,
                ),
                visual: UiVisual::panel(
                    interactive_fill(active, hovered),
                    Some(stroke(interactive_stroke(active, hovered))),
                    3.0,
                ),
                text_style: TextStyle {
                    font_size: (rect.height * 0.36).clamp(10.0, 15.0),
                    line_height: (rect.height * 0.52).clamp(14.0, 20.0),
                    color: ColorRgba::WHITE,
                    ..TextStyle::default()
                },
                accessibility_label: Some(label.to_string()),
                ..Default::default()
            },
        )
    }

    fn add_panel(
        document: &mut UiDocument,
        parent: UiNodeId,
        name: &str,
        rect: UiRect,
        fill: ColorRgba,
        stroke: Option<StrokeStyle>,
        radius: f32,
    ) -> UiNodeId {
        document.add_child(
            parent,
            UiNode::container(
                name,
                UiNodeStyle {
                    clip: ClipBehavior::Clip,
                    ..layout::node_style(rect_layout(rect))
                },
            )
            .with_visual(UiVisual::panel(fill, stroke, radius)),
        )
    }

    fn add_text(
        document: &mut UiDocument,
        parent: UiNodeId,
        name: &str,
        value: &str,
        rect: UiRect,
        font_size: f32,
        color: ColorRgba,
        weight: FontWeight,
    ) -> UiNodeId {
        document.add_child(
            parent,
            UiNode::text(
                name,
                value,
                TextStyle {
                    font_size,
                    line_height: font_size * 1.28,
                    color,
                    weight,
                    wrap: TextWrap::None,
                    ..TextStyle::default()
                },
                rect_layout(rect),
            ),
        )
    }

    fn rect_layout(rect: UiRect) -> LayoutStyle {
        layout::absolute(rect.x, rect.y, rect.width, rect.height)
    }

    fn r(layout: &ShowcaseLayout, x: f32, y: f32, width: f32, height: f32) -> UiRect {
        UiRect::new(
            layout.root.x + x * layout.scale,
            layout.root.y + y * layout.scale,
            width * layout.scale,
            height * layout.scale,
        )
    }

    fn inset(rect: UiRect, x: f32, y: f32, width: f32, height: f32) -> UiRect {
        UiRect::new(rect.x + x, rect.y + y, width, height)
    }

    fn stroke(color: ColorRgba) -> StrokeStyle {
        StrokeStyle::new(color, 1.0)
    }

    fn bg() -> ColorRgba {
        ColorRgba::new(18, 22, 23, 255)
    }

    fn panel() -> ColorRgba {
        ColorRgba::new(40, 49, 51, 242)
    }

    fn surface() -> ColorRgba {
        ColorRgba::new(50, 60, 63, 230)
    }

    fn selected() -> ColorRgba {
        ColorRgba::new(80, 112, 119, 255)
    }

    fn hover() -> ColorRgba {
        ColorRgba::new(64, 78, 81, 255)
    }

    fn interactive_fill(active: bool, hovered: bool) -> ColorRgba {
        if active {
            selected()
        } else if hovered {
            hover()
        } else {
            ColorRgba::new(44, 53, 56, 255)
        }
    }

    fn interactive_stroke(active: bool, hovered: bool) -> ColorRgba {
        if active {
            accent()
        } else if hovered {
            ColorRgba::new(142, 165, 168, 255)
        } else {
            line()
        }
    }

    fn line() -> ColorRgba {
        ColorRgba::new(91, 109, 112, 255)
    }

    fn text() -> ColorRgba {
        ColorRgba::new(222, 229, 225, 255)
    }

    fn muted() -> ColorRgba {
        ColorRgba::new(166, 178, 178, 255)
    }

    fn accent() -> ColorRgba {
        ColorRgba::new(105, 197, 188, 255)
    }

    fn accent_warm() -> ColorRgba {
        ColorRgba::new(240, 202, 122, 255)
    }

    fn color_wheel_color(index: usize) -> ColorRgba {
        let hue = (index % COLOR_WHEEL_SEGMENTS) as f32 / COLOR_WHEEL_SEGMENTS as f32;
        hsv_to_rgb(hue, 0.58, 0.88)
    }

    fn selected_hue(state: &ShowcaseState) -> f32 {
        (state.selected_hue % COLOR_WHEEL_SEGMENTS) as f32 / COLOR_WHEEL_SEGMENTS as f32
    }

    fn hsv_to_rgb(hue: f32, saturation: f32, value: f32) -> ColorRgba {
        let hue = hue.rem_euclid(1.0) * 6.0;
        let chroma = value * saturation;
        let x = chroma * (1.0 - (hue % 2.0 - 1.0).abs());
        let m = value - chroma;
        let (r, g, b) = if hue < 1.0 {
            (chroma, x, 0.0)
        } else if hue < 2.0 {
            (x, chroma, 0.0)
        } else if hue < 3.0 {
            (0.0, chroma, x)
        } else if hue < 4.0 {
            (0.0, x, chroma)
        } else if hue < 5.0 {
            (x, 0.0, chroma)
        } else {
            (chroma, 0.0, x)
        };
        let channel = |component: f32| ((component + m) * 255.0).round().clamp(0.0, 255.0) as u8;
        ColorRgba::new(channel(r), channel(g), channel(b), 255)
    }
}
