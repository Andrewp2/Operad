use std::error::Error;

use operad::platform::PixelSize;
use operad::{
    layout, process_document_frame, root_style, AccessibilityMeta, AccessibilityRole,
    ApproxTextMeasurer, ColorRgba, HostDocumentFrameOutput, HostDocumentFrameRequest,
    HostFrameOutput, HostInteractionState, InputBehavior, RenderTarget, StrokeStyle, TextStyle,
    UiDocument, UiNode, UiSize, UiVisual,
};

#[cfg(all(feature = "accesskit-winit", feature = "native-window"))]
use operad::{AccessKitTreeOptions, AccessKitWinitAdapter};

#[cfg(feature = "wgpu")]
use operad::{EmptyResourceResolver, RendererAdapter, WgpuRenderer};

#[cfg(all(feature = "wgpu", feature = "native-window"))]
use {
    operad::WgpuSurfaceRenderer,
    std::{sync::Arc, time::Duration},
    winit::{
        application::ApplicationHandler,
        dpi::PhysicalSize,
        event::WindowEvent,
        event_loop::{ActiveEventLoop, EventLoop},
        window::{Window, WindowId},
    },
};

fn main() -> Result<(), Box<dyn Error>> {
    #[cfg(all(feature = "wgpu", feature = "native-window"))]
    if std::env::var_os("OPERAD_RUN_WGPU_EXAMPLE_WINDOW").is_some() {
        return run_windowed_wgpu_example();
    }

    let viewport = UiSize::new(640.0, 360.0);
    let frame = sample_frame(viewport, RenderTarget::offscreen(PixelSize::new(640, 360)))?;

    println!(
        "native_wgpu_host: {} paint items, {} accessibility nodes",
        frame.render_request.paint.items.len(),
        frame.accessibility_tree.nodes.len()
    );

    #[cfg(feature = "wgpu")]
    if std::env::var_os("OPERAD_RUN_WGPU_EXAMPLE").is_some() {
        let mut renderer = WgpuRenderer::new();
        let output = renderer.render_frame(frame.render_request, &EmptyResourceResolver)?;
        println!(
            "native_wgpu_host: rendered {} items into {:?}",
            output.painted_items, output.target
        );
    }

    Ok(())
}

fn sample_frame(
    viewport: UiSize,
    target: RenderTarget,
) -> Result<HostDocumentFrameOutput, Box<dyn Error>> {
    let mut document = build_document();
    let mut measurer = ApproxTextMeasurer;
    let host_output = HostFrameOutput::new(HostInteractionState::default());
    Ok(process_document_frame(
        &mut document,
        &mut measurer,
        HostDocumentFrameRequest::new(viewport, target, host_output),
    )?)
}

#[cfg(all(feature = "wgpu", feature = "native-window"))]
fn run_windowed_wgpu_example() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = NativeWindowApp::new(window_frame_limit()?);
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.error {
        Err(error.into())
    } else {
        if app.presented_frames > 0 {
            println!(
                "native_wgpu_host: native surface presented {} frame(s), p95 render {:?}",
                app.presented_frames,
                percentile_duration(&app.render_samples, 95.0).unwrap_or_default()
            );
        }
        Ok(())
    }
}

#[cfg(all(feature = "wgpu", feature = "native-window"))]
struct NativeWindowApp {
    window: Option<Arc<Window>>,
    window_id: Option<WindowId>,
    renderer: Option<WgpuSurfaceRenderer<'static>>,
    #[cfg(feature = "accesskit-winit")]
    accesskit: Option<AccessKitWinitAdapter>,
    error: Option<String>,
    frame_limit: Option<usize>,
    presented_frames: usize,
    render_samples: Vec<Duration>,
}

#[cfg(all(feature = "wgpu", feature = "native-window"))]
impl NativeWindowApp {
    fn new(frame_limit: Option<usize>) -> Self {
        Self {
            window: None,
            window_id: None,
            renderer: None,
            #[cfg(feature = "accesskit-winit")]
            accesskit: None,
            error: None,
            frame_limit,
            presented_frames: 0,
            render_samples: Vec::new(),
        }
    }

    fn init_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let window = Arc::new(
            event_loop.create_window(
                Window::default_attributes()
                    .with_title("Operad native WGPU host")
                    .with_inner_size(PhysicalSize::new(640, 360))
                    .with_visible(false),
            )?,
        );
        let size = nonzero_window_size(window.inner_size());

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance.create_surface(window.clone())?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
        }))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("native-wgpu-host-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            }))?;
        let surface_config = surface
            .get_default_config(&adapter, size.width, size.height)
            .ok_or("adapter does not support the native window surface")?;

        self.window_id = Some(window.id());
        self.renderer = Some(WgpuSurfaceRenderer::new(
            surface,
            device,
            queue,
            surface_config,
        )?);
        #[cfg(feature = "accesskit-winit")]
        {
            let viewport = UiSize::new(size.width as f32, size.height as f32);
            let frame = sample_frame(viewport, RenderTarget::window("native-wgpu-host", viewport))?;
            self.accesskit = Some(AccessKitWinitAdapter::new(
                event_loop,
                &window,
                frame.accessibility_tree,
                None,
                AccessKitTreeOptions::default(),
            ));
        }
        window.set_visible(true);
        self.window = Some(window);
        Ok(())
    }

    fn render(&mut self) -> Result<bool, Box<dyn Error>> {
        let Some(window) = self.window.as_ref() else {
            return Ok(false);
        };
        let Some(renderer) = self.renderer.as_mut() else {
            return Ok(false);
        };

        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(false);
        }
        let viewport = UiSize::new(size.width as f32, size.height as f32);
        let frame = sample_frame(viewport, RenderTarget::window("native-wgpu-host", viewport))?;
        #[cfg(feature = "accesskit-winit")]
        if let Some(accesskit) = self.accesskit.as_mut() {
            accesskit.publish_tree(frame.accessibility_tree.clone(), None);
        }
        let output = renderer.render_frame(frame.render_request, &EmptyResourceResolver)?;
        if output.snapshot.is_some() {
            return Err("native surface smoke must present without snapshot readback".into());
        }
        if let Some(duration) = output.timings.duration("render") {
            self.render_samples.push(duration);
        }
        self.presented_frames += 1;
        Ok(self
            .frame_limit
            .is_some_and(|frame_limit| self.presented_frames >= frame_limit))
    }

    fn fail_and_exit(&mut self, event_loop: &ActiveEventLoop, error: impl ToString) {
        self.error = Some(error.to_string());
        event_loop.exit();
    }
}

#[cfg(all(feature = "wgpu", feature = "native-window"))]
impl ApplicationHandler for NativeWindowApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        if let Err(error) = self.init_window(event_loop) {
            self.fail_and_exit(event_loop, error);
            return;
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if Some(window_id) != self.window_id {
            return;
        }

        #[cfg(feature = "accesskit-winit")]
        if let (Some(window), Some(accesskit)) = (self.window.as_ref(), self.accesskit.as_mut()) {
            accesskit.process_event(window, &event);
        }

        match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::RedrawRequested => match self.render() {
                Ok(true) => event_loop.exit(),
                Ok(false) => {
                    if self.frame_limit.is_some() {
                        if let Some(window) = self.window.as_ref() {
                            window.request_redraw();
                        }
                    }
                }
                Err(error) => self.fail_and_exit(event_loop, error),
            },
            _ => {}
        }
    }
}

#[cfg(all(feature = "wgpu", feature = "native-window"))]
fn nonzero_window_size(size: PhysicalSize<u32>) -> PhysicalSize<u32> {
    PhysicalSize::new(size.width.max(1), size.height.max(1))
}

#[cfg(all(feature = "wgpu", feature = "native-window"))]
fn window_frame_limit() -> Result<Option<usize>, Box<dyn Error>> {
    let Some(value) = std::env::var_os("OPERAD_WGPU_EXAMPLE_WINDOW_FRAMES") else {
        return Ok(None);
    };
    let frames = value
        .to_string_lossy()
        .parse::<usize>()
        .map_err(|error| format!("invalid OPERAD_WGPU_EXAMPLE_WINDOW_FRAMES: {error}"))?;
    if frames == 0 {
        return Err("OPERAD_WGPU_EXAMPLE_WINDOW_FRAMES must be greater than zero".into());
    }
    Ok(Some(frames))
}

#[cfg(all(feature = "wgpu", feature = "native-window"))]
fn percentile_duration(samples: &[Duration], percentile: f64) -> Option<Duration> {
    if samples.is_empty() {
        return None;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let clamped = percentile.clamp(0.0, 100.0);
    let index = ((clamped / 100.0) * (sorted.len().saturating_sub(1) as f64)).ceil() as usize;
    sorted.get(index).copied()
}

fn build_document() -> UiDocument {
    let mut document = UiDocument::new(root_style(640.0, 360.0));
    let panel = document.add_child(
        document.root,
        UiNode::container(
            "native.panel",
            layout::node_style(layout::with_margin_all(
                layout::with_size(layout::column(), layout::px(280.0), layout::px(340.0)),
                24.0,
            )),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(24, 29, 36, 255),
            Some(StrokeStyle::new(ColorRgba::new(91, 110, 132, 255), 1.0)),
            6.0,
        )),
    );

    document.add_child(
        panel,
        UiNode::text(
            "native.title",
            "Operad native WGPU host",
            TextStyle {
                font_size: 18.0,
                line_height: 24.0,
                color: ColorRgba::WHITE,
                ..TextStyle::default()
            },
            layout::size(layout::percent(1.0), layout::px(32.0)),
        ),
    );

    for (index, label) in ["Play", "Select"].into_iter().enumerate() {
        document.add_child(
            panel,
            UiNode::text(
                format!("native.button.{index}"),
                label,
                TextStyle::default(),
                layout::size(layout::percent(1.0), layout::px(34.0)),
            )
            .with_input(InputBehavior::BUTTON)
            .with_visual(UiVisual::panel(
                ColorRgba::new(42, 51, 63, 255),
                Some(StrokeStyle::new(ColorRgba::new(112, 135, 162, 255), 1.0)),
                4.0,
            ))
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Button)
                    .label(label)
                    .focusable(),
            ),
        );
    }

    let text_input = document.add_child(
        panel,
        UiNode::container(
            "native.text_input",
            layout::node_style(layout::with_size(
                layout::row(),
                layout::percent(1.0),
                layout::px(34.0),
            )),
        )
        .with_input(InputBehavior::BUTTON)
        .with_visual(UiVisual::panel(
            ColorRgba::new(18, 22, 28, 255),
            Some(StrokeStyle::new(ColorRgba::new(72, 84, 104, 255), 1.0)),
            4.0,
        ))
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::TextBox)
                .label("Filter")
                .value("filter clips")
                .focusable(),
        ),
    );
    document.add_child(
        text_input,
        UiNode::text(
            "native.text_input.value",
            "filter clips",
            TextStyle::default(),
            layout::size(layout::percent(1.0), layout::px(30.0)),
        ),
    );

    let menu = document.add_child(
        panel,
        UiNode::container(
            "native.popup.menu",
            layout::node_style(layout::with_size(
                layout::column(),
                layout::percent(1.0),
                layout::px(68.0),
            )),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(31, 38, 48, 255),
            Some(StrokeStyle::new(ColorRgba::new(112, 135, 162, 255), 1.0)),
            4.0,
        ))
        .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Menu).label("Mode menu")),
    );
    for (index, label) in ["Auto", "Manual"].into_iter().enumerate() {
        document.add_child(
            menu,
            UiNode::text(
                format!("native.popup.item.{index}"),
                label,
                TextStyle::default(),
                layout::size(layout::percent(1.0), layout::px(30.0)),
            )
            .with_input(InputBehavior::BUTTON)
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::MenuItem)
                    .label(label)
                    .focusable(),
            ),
        );
    }

    document.add_child(
        panel,
        UiNode::text(
            "native.drag.handle",
            "Drag handle",
            TextStyle::default(),
            layout::size(layout::percent(1.0), layout::px(28.0)),
        )
        .with_input(InputBehavior::BUTTON)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Slider)
                .label("Drag handle")
                .focusable(),
        ),
    );

    document.add_child(
        panel,
        UiNode::canvas(
            "native.canvas.viewport",
            "native.canvas.viewport",
            layout::size(layout::percent(1.0), layout::px(52.0)),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(18, 22, 28, 255),
            Some(StrokeStyle::new(ColorRgba::new(72, 84, 104, 255), 1.0)),
            4.0,
        ))
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Group)
                .label("Canvas viewport")
                .focusable(),
        ),
    );

    document
}
