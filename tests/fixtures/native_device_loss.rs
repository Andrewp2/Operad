//! Destroy an idle native app's real GPU device from a worker. The host must
//! wake and return a structured failure without requiring input or a timer.
use operad::errors::{ErrorKind, ErrorReport, RendererErrorKind};
use operad::native::{
    run_app_with_canvas_renderers_and_hooks, NativeWgpuCanvasRenderContext,
    NativeWgpuCanvasRenderRegistry, NativeWindowOptions,
};
use operad::renderer::CanvasRenderOutput;
use operad::runtime::RuntimeHooks;
use operad::{LayoutStyle, UiDocument, UiNode};
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};

fn main() {
    let during_render = std::env::args().nth(1).as_deref() == Some("during-render");
    let (sender, receiver) = mpsc::sync_channel::<wgpu::Device>(1);
    let worker = (!during_render).then(|| {
        std::thread::spawn(move || {
            let device = receiver.recv().unwrap();
            let command = std::io::stdin().lock().lines().next().unwrap().unwrap();
            assert_eq!(command, "lose");
            destroy_device(&device);
        })
    });
    let mut renderers = NativeWgpuCanvasRenderRegistry::new();
    renderers.register(
        "device-loss",
        move |sent: &mut bool, context: NativeWgpuCanvasRenderContext<'_>| {
            if !*sent {
                if during_render {
                    destroy_device(context.surface.device());
                } else {
                    sender.send(context.surface.device().clone()).unwrap();
                }
                *sent = true;
            }
            Ok(CanvasRenderOutput::default())
        },
    );
    let frames = Arc::new(AtomicUsize::new(0));
    let observed_frames = frames.clone();
    let result = run_app_with_canvas_renderers_and_hooks(
        NativeWindowOptions::new("Device loss probe"),
        false,
        |_, _| {},
        |_, viewport, _| {
            let mut document = UiDocument::new(LayoutStyle::size(viewport.width, viewport.height));
            document.add_child(
                document.root(),
                UiNode::canvas("device-loss", "device-loss", LayoutStyle::size(80.0, 80.0)),
            );
            document
        },
        renderers,
        RuntimeHooks::new().with_frame_observer(move |_: &bool, _| {
            let count = observed_frames.fetch_add(1, Ordering::SeqCst) + 1;
            println!("FRAME {count}");
            std::io::stdout().flush().unwrap();
        }),
    );
    if let Some(worker) = worker {
        worker.join().unwrap();
    }
    let error = result.expect_err("a lost device must stop the native runner");
    let report = error
        .source()
        .and_then(|source| source.downcast_ref::<ErrorReport>())
        .unwrap_or_else(|| panic!("missing structured error source: {error}"));
    assert_eq!(
        report.kind,
        ErrorKind::Renderer(RendererErrorKind::DeviceLost),
        "wrong device-loss classification: {error}"
    );
    println!("DEVICE_LOSS_HANDLED {}", frames.load(Ordering::SeqCst));
}

fn destroy_device(device: &wgpu::Device) {
    device.destroy();
    let _ = device.poll(wgpu::PollType::Poll);
    println!("DEVICE_DESTROYED");
    std::io::stdout().flush().unwrap();
}
