use std::error::Error;

use operad::platform::PixelSize;
use operad::{
    layout, process_document_frame, root_style, ApproxTextMeasurer, ColorRgba,
    EmptyResourceResolver, HostDocumentFrameRequest, HostFrameOutput, HostInteractionState,
    InputBehavior, RenderTarget, StrokeStyle, TextStyle, UiDocument, UiNode, UiSize, UiVisual,
};

#[cfg(feature = "wgpu")]
use operad::{RendererAdapter, WgpuRenderer};

fn main() -> Result<(), Box<dyn Error>> {
    let viewport = UiSize::new(640.0, 360.0);
    let mut document = build_document();
    let mut measurer = ApproxTextMeasurer;
    let host_output = HostFrameOutput::new(HostInteractionState::default());
    let frame = process_document_frame(
        &mut document,
        &mut measurer,
        HostDocumentFrameRequest::new(
            viewport,
            RenderTarget::offscreen(PixelSize::new(640, 360)),
            host_output,
        ),
    )?;

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

fn build_document() -> UiDocument {
    let mut document = UiDocument::new(root_style(640.0, 360.0));
    let panel = document.add_child(
        document.root,
        UiNode::container(
            "native.panel",
            layout::node_style(layout::with_margin_all(
                layout::with_size(layout::column(), layout::px(280.0), layout::px(180.0)),
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

    for (index, label) in ["Play", "Select", "Drag"].into_iter().enumerate() {
        document.add_child(
            panel,
            UiNode::text(
                format!("native.button.{index}"),
                label,
                TextStyle::default(),
                layout::size(layout::percent(1.0), layout::px(36.0)),
            )
            .with_input(InputBehavior::BUTTON)
            .with_visual(UiVisual::panel(
                ColorRgba::new(42, 51, 63, 255),
                Some(StrokeStyle::new(ColorRgba::new(112, 135, 162, 255), 1.0)),
                4.0,
            )),
        );
    }

    document
}
