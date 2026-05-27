use operad::{
    layout, root_style, AccessibilityMeta, AccessibilityRole, ColorRgba, CornerRadii, PaintRect,
    PaintText, ScenePrimitive, StrokeStyle, TextHorizontalAlign, TextOverflow, TextStyle,
    TextVerticalAlign, UiDocument, UiNode, UiPoint, UiRect, UiSize, UiVisual,
};

pub const GAME_UI_VIEWPORT: UiSize = UiSize::new(1280.0, 720.0);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameUiState {
    pub health: f32,
    pub shield: f32,
    pub stamina: f32,
    pub squad_a: f32,
    pub squad_b: f32,
    pub squad_c: f32,
    pub objective: f32,
    pub overheat: f32,
    pub charge: f32,
    pub ammo: u32,
    pub ammo_capacity: u32,
    pub wave: u32,
    pub time_seconds: f32,
    pub damage_flash: f32,
}

impl Default for GameUiState {
    fn default() -> Self {
        Self {
            health: 0.74,
            shield: 0.58,
            stamina: 0.82,
            squad_a: 0.91,
            squad_b: 0.63,
            squad_c: 0.37,
            objective: 0.66,
            overheat: 0.31,
            charge: 0.48,
            ammo: 21,
            ammo_capacity: 36,
            wave: 9,
            time_seconds: 312.0,
            damage_flash: 0.22,
        }
    }
}

impl GameUiState {
    pub fn for_frame(frame: usize) -> Self {
        let phase = frame as f32 * 0.071;
        Self {
            health: wave01(phase * 0.31, 0.67, 0.92),
            shield: wave01(phase * 0.47 + 1.4, 0.34, 0.78),
            stamina: wave01(phase * 0.79 + 0.8, 0.5, 0.98),
            squad_a: wave01(phase * 0.39 + 0.2, 0.74, 1.0),
            squad_b: wave01(phase * 0.52 + 2.1, 0.42, 0.87),
            squad_c: wave01(phase * 0.61 + 4.6, 0.18, 0.68),
            objective: ((frame % 180) as f32 / 179.0).clamp(0.0, 1.0),
            overheat: wave01(phase * 0.92 + 1.1, 0.08, 0.74),
            charge: wave01(phase * 0.68 + 2.8, 0.0, 1.0),
            ammo: 8 + ((frame * 7) % 28) as u32,
            ammo_capacity: 36,
            wave: 9 + ((frame / 48) % 4) as u32,
            time_seconds: 312.0 + frame as f32 * 0.15,
            damage_flash: wave01(phase * 1.3 + 0.5, 0.0, 0.36),
        }
    }
}

pub fn game_ui_document(viewport: UiSize, state: GameUiState) -> UiDocument {
    let viewport = UiSize::new(viewport.width.max(1.0), viewport.height.max(1.0));
    let mut document = UiDocument::new(root_style(viewport.width, viewport.height));
    let root = document.root();
    document
        .node_mut(root)
        .set_visual(UiVisual::panel(ColorRgba::TRANSPARENT, None, 0.0));

    let overlay = document.add_child(
        root,
        UiNode::container("game.overlay", layout::absolute_fill())
            .with_visual(UiVisual::TRANSPARENT)
            .with_accessibility(AccessibilityMeta::new(AccessibilityRole::Group).label("Game HUD")),
    );

    let safe = 24.0;
    add_scene(
        &mut document,
        overlay,
        "game.objective",
        centered(viewport.width, 390.0),
        safe,
        390.0,
        64.0,
        objective_scene(&state),
    );
    add_scene(
        &mut document,
        overlay,
        "game.squad",
        safe,
        safe,
        326.0,
        118.0,
        squad_scene(&state),
    );
    add_scene(
        &mut document,
        overlay,
        "game.radar",
        right(viewport.width, safe, 170.0),
        safe,
        170.0,
        170.0,
        radar_scene(&state),
    );
    add_scene(
        &mut document,
        overlay,
        "game.vitals",
        safe,
        bottom(viewport.height, safe, 152.0),
        362.0,
        152.0,
        vitals_scene(&state),
    );
    add_scene(
        &mut document,
        overlay,
        "game.abilities",
        centered(viewport.width, 432.0),
        bottom(viewport.height, safe, 86.0),
        432.0,
        86.0,
        ability_scene(&state),
    );
    add_scene(
        &mut document,
        overlay,
        "game.weapon",
        right(viewport.width, safe, 342.0),
        bottom(viewport.height, safe, 146.0),
        342.0,
        146.0,
        weapon_scene(&state),
    );
    add_scene(
        &mut document,
        overlay,
        "game.reticle",
        centered(viewport.width, 96.0),
        centered(viewport.height, 96.0),
        96.0,
        96.0,
        reticle_scene(&state),
    );

    if state.damage_flash > 0.02 {
        add_scene(
            &mut document,
            overlay,
            "game.damage",
            0.0,
            0.0,
            viewport.width,
            viewport.height,
            damage_scene(viewport, state.damage_flash),
        );
    }

    document
}

fn add_scene(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    name: &'static str,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    primitives: Vec<ScenePrimitive>,
) {
    document.add_child(
        parent,
        UiNode::scene(name, primitives, layout::absolute(x, y, width, height)),
    );
}

fn objective_scene(state: &GameUiState) -> Vec<ScenePrimitive> {
    let mut scene = panel(390.0, 64.0);
    scene.push(text(
        "ZONE 04",
        16.0,
        10.0,
        86.0,
        18.0,
        12.0,
        ACCENT_CYAN,
        TextHorizontalAlign::Start,
    ));
    scene.push(text(
        format!("WAVE {}", state.wave),
        288.0,
        10.0,
        78.0,
        18.0,
        12.0,
        ACCENT_AMBER,
        TextHorizontalAlign::End,
    ));
    progress_bar(
        &mut scene,
        UiRect::new(16.0, 36.0, 358.0, 12.0),
        state.objective,
        ACCENT_GREEN,
    );
    scene.push(text(
        "HOLD",
        166.0,
        14.0,
        58.0,
        18.0,
        12.0,
        TEXT_MUTED,
        TextHorizontalAlign::Center,
    ));
    scene
}

fn squad_scene(state: &GameUiState) -> Vec<ScenePrimitive> {
    let mut scene = panel(326.0, 118.0);
    scene.push(text(
        "SQUAD",
        16.0,
        12.0,
        78.0,
        20.0,
        13.0,
        TEXT_MAIN,
        TextHorizontalAlign::Start,
    ));
    scene.push(text(
        "LINK",
        236.0,
        12.0,
        56.0,
        20.0,
        12.0,
        ACCENT_GREEN,
        TextHorizontalAlign::End,
    ));
    squad_row(&mut scene, 42.0, "ACE", state.squad_a, ACCENT_GREEN);
    squad_row(&mut scene, 66.0, "KAI", state.squad_b, ACCENT_CYAN);
    squad_row(&mut scene, 90.0, "MIR", state.squad_c, ACCENT_RED);
    scene
}

fn squad_row(
    scene: &mut Vec<ScenePrimitive>,
    y: f32,
    callout: &'static str,
    value: f32,
    fill: ColorRgba,
) {
    scene.push(text(
        callout,
        16.0,
        y - 5.0,
        42.0,
        16.0,
        11.0,
        TEXT_MUTED,
        TextHorizontalAlign::Start,
    ));
    progress_bar(scene, UiRect::new(62.0, y, 210.0, 8.0), value, fill);
    scene.push(text(
        format!("{:02}", (value.clamp(0.0, 1.0) * 99.0).round() as u32),
        282.0,
        y - 5.0,
        26.0,
        16.0,
        11.0,
        TEXT_MAIN,
        TextHorizontalAlign::End,
    ));
}

fn vitals_scene(state: &GameUiState) -> Vec<ScenePrimitive> {
    let mut scene = panel(362.0, 152.0);
    scene.push(text(
        "VITALS",
        16.0,
        14.0,
        88.0,
        20.0,
        13.0,
        TEXT_MAIN,
        TextHorizontalAlign::Start,
    ));
    labeled_bar(&mut scene, 44.0, "HP", state.health, ACCENT_GREEN);
    labeled_bar(&mut scene, 76.0, "SHIELD", state.shield, ACCENT_CYAN);
    labeled_bar(&mut scene, 108.0, "STAM", state.stamina, ACCENT_AMBER);
    scene
}

fn labeled_bar(
    scene: &mut Vec<ScenePrimitive>,
    y: f32,
    label: &'static str,
    value: f32,
    fill: ColorRgba,
) {
    scene.push(text(
        label,
        16.0,
        y - 8.0,
        72.0,
        18.0,
        12.0,
        TEXT_MUTED,
        TextHorizontalAlign::Start,
    ));
    progress_bar(scene, UiRect::new(96.0, y - 1.0, 216.0, 12.0), value, fill);
    scene.push(text(
        format!("{:03}", (value.clamp(0.0, 1.0) * 100.0).round() as u32),
        318.0,
        y - 8.0,
        28.0,
        18.0,
        12.0,
        TEXT_MAIN,
        TextHorizontalAlign::End,
    ));
}

fn ability_scene(state: &GameUiState) -> Vec<ScenePrimitive> {
    let mut scene = panel(432.0, 86.0);
    for slot in 0..5 {
        let x = 18.0 + slot as f32 * 80.0;
        let ready = (state.charge + slot as f32 * 0.19).fract();
        scene.push(rect(
            x,
            18.0,
            58.0,
            50.0,
            tint(PANEL_FILL, 1.12),
            Some(StrokeStyle::new(
                if ready > 0.62 {
                    ACCENT_CYAN
                } else {
                    PANEL_STROKE
                },
                1.0,
            )),
            6.0,
        ));
        if ready < 0.62 {
            scene.push(rect(
                x + 4.0,
                22.0 + (42.0 * ready),
                50.0,
                42.0 * (1.0 - ready),
                ColorRgba::new(6, 8, 12, 156),
                None,
                4.0,
            ));
        }
        scene.push(text(
            format!("{}", slot + 1),
            x,
            28.0,
            58.0,
            20.0,
            14.0,
            TEXT_MAIN,
            TextHorizontalAlign::Center,
        ));
        scene.push(text(
            ability_label(slot),
            x,
            48.0,
            58.0,
            14.0,
            10.0,
            TEXT_MUTED,
            TextHorizontalAlign::Center,
        ));
    }
    scene
}

fn weapon_scene(state: &GameUiState) -> Vec<ScenePrimitive> {
    let mut scene = panel(342.0, 146.0);
    scene.push(text(
        "AMMO",
        18.0,
        16.0,
        80.0,
        20.0,
        12.0,
        TEXT_MUTED,
        TextHorizontalAlign::Start,
    ));
    scene.push(text(
        format!("{:02}", state.ammo),
        18.0,
        42.0,
        122.0,
        48.0,
        38.0,
        TEXT_MAIN,
        TextHorizontalAlign::Start,
    ));
    scene.push(text(
        format!("/{:02}", state.ammo_capacity),
        126.0,
        58.0,
        70.0,
        24.0,
        16.0,
        TEXT_MUTED,
        TextHorizontalAlign::Start,
    ));
    scene.push(text(
        "PULSE",
        238.0,
        18.0,
        72.0,
        20.0,
        12.0,
        ACCENT_CYAN,
        TextHorizontalAlign::End,
    ));
    labeled_meter(&mut scene, 94.0, "HEAT", state.overheat, ACCENT_RED);
    labeled_meter(&mut scene, 116.0, "CHARGE", state.charge, ACCENT_AMBER);
    scene
}

fn radar_scene(state: &GameUiState) -> Vec<ScenePrimitive> {
    let mut scene = panel(170.0, 170.0);
    scene.push(ScenePrimitive::Circle {
        center: UiPoint::new(85.0, 85.0),
        radius: 65.0,
        fill: ColorRgba::new(8, 14, 20, 178),
        stroke: Some(StrokeStyle::new(PANEL_STROKE, 1.0)),
    });
    for radius in [22.0, 44.0] {
        scene.push(ScenePrimitive::Circle {
            center: UiPoint::new(85.0, 85.0),
            radius,
            fill: ColorRgba::TRANSPARENT,
            stroke: Some(StrokeStyle::new(ColorRgba::new(77, 203, 219, 72), 1.0)),
        });
    }
    scene.push(ScenePrimitive::Line {
        from: UiPoint::new(85.0, 20.0),
        to: UiPoint::new(85.0, 150.0),
        stroke: StrokeStyle::new(ColorRgba::new(77, 203, 219, 64), 1.0),
    });
    scene.push(ScenePrimitive::Line {
        from: UiPoint::new(20.0, 85.0),
        to: UiPoint::new(150.0, 85.0),
        stroke: StrokeStyle::new(ColorRgba::new(77, 203, 219, 64), 1.0),
    });
    for index in 0..7 {
        let angle = state.time_seconds * 0.018 + index as f32 * 0.91;
        let distance = 18.0 + ((index * 11) % 42) as f32;
        let center = UiPoint::new(85.0 + angle.cos() * distance, 85.0 + angle.sin() * distance);
        scene.push(ScenePrimitive::Circle {
            center,
            radius: if index == 2 { 4.0 } else { 2.5 },
            fill: if index == 2 { ACCENT_RED } else { ACCENT_GREEN },
            stroke: None,
        });
    }
    scene.push(text(
        format!(
            "{:02}:{:02}",
            (state.time_seconds as u32) / 60,
            (state.time_seconds as u32) % 60
        ),
        48.0,
        142.0,
        74.0,
        16.0,
        11.0,
        TEXT_MUTED,
        TextHorizontalAlign::Center,
    ));
    scene
}

fn reticle_scene(state: &GameUiState) -> Vec<ScenePrimitive> {
    let pulse = 1.0 + state.charge * 4.0;
    vec![
        ScenePrimitive::Circle {
            center: UiPoint::new(48.0, 48.0),
            radius: 6.0 + pulse * 0.12,
            fill: ColorRgba::TRANSPARENT,
            stroke: Some(StrokeStyle::new(ACCENT_CYAN, 1.0)),
        },
        ScenePrimitive::Line {
            from: UiPoint::new(48.0, 18.0),
            to: UiPoint::new(48.0, 34.0 - pulse),
            stroke: StrokeStyle::new(ACCENT_CYAN, 1.0),
        },
        ScenePrimitive::Line {
            from: UiPoint::new(48.0, 62.0 + pulse),
            to: UiPoint::new(48.0, 78.0),
            stroke: StrokeStyle::new(ACCENT_CYAN, 1.0),
        },
        ScenePrimitive::Line {
            from: UiPoint::new(18.0, 48.0),
            to: UiPoint::new(34.0 - pulse, 48.0),
            stroke: StrokeStyle::new(ACCENT_CYAN, 1.0),
        },
        ScenePrimitive::Line {
            from: UiPoint::new(62.0 + pulse, 48.0),
            to: UiPoint::new(78.0, 48.0),
            stroke: StrokeStyle::new(ACCENT_CYAN, 1.0),
        },
    ]
}

fn damage_scene(viewport: UiSize, amount: f32) -> Vec<ScenePrimitive> {
    let alpha = (amount.clamp(0.0, 1.0) * 118.0).round() as u8;
    let red = ColorRgba::new(245, 72, 72, alpha);
    vec![
        rect(0.0, 0.0, viewport.width, 18.0, red, None, 0.0),
        rect(
            0.0,
            viewport.height - 18.0,
            viewport.width,
            18.0,
            red,
            None,
            0.0,
        ),
        rect(0.0, 0.0, 18.0, viewport.height, red, None, 0.0),
        rect(
            viewport.width - 18.0,
            0.0,
            18.0,
            viewport.height,
            red,
            None,
            0.0,
        ),
    ]
}

fn labeled_meter(
    scene: &mut Vec<ScenePrimitive>,
    y: f32,
    label: &'static str,
    value: f32,
    fill: ColorRgba,
) {
    scene.push(text(
        label,
        18.0,
        y - 7.0,
        64.0,
        16.0,
        10.0,
        TEXT_MUTED,
        TextHorizontalAlign::Start,
    ));
    progress_bar(scene, UiRect::new(92.0, y, 216.0, 7.0), value, fill);
}

fn progress_bar(scene: &mut Vec<ScenePrimitive>, bounds: UiRect, value: f32, fill: ColorRgba) {
    scene.push(ScenePrimitive::Rect(
        PaintRect::solid(bounds, ColorRgba::new(9, 14, 20, 198))
            .stroke(StrokeStyle::new(PANEL_STROKE, 1.0))
            .corner_radii(CornerRadii::uniform(4.0)),
    ));
    let fill_width = (bounds.width * value.clamp(0.0, 1.0)).max(0.0);
    scene.push(rect(
        bounds.x + 2.0,
        bounds.y + 2.0,
        (fill_width - 4.0).max(0.0),
        (bounds.height - 4.0).max(0.0),
        fill,
        None,
        3.0,
    ));
    for tick in 1..4 {
        let x = bounds.x + bounds.width * tick as f32 * 0.25;
        scene.push(rect(
            x,
            bounds.y + 2.0,
            1.0,
            (bounds.height - 4.0).max(0.0),
            ColorRgba::new(222, 238, 240, 72),
            None,
            0.0,
        ));
    }
}

fn panel(width: f32, height: f32) -> Vec<ScenePrimitive> {
    vec![
        rect(
            0.0,
            0.0,
            width,
            height,
            PANEL_FILL,
            Some(StrokeStyle::new(PANEL_STROKE, 1.0)),
            8.0,
        ),
        rect(
            8.0,
            8.0,
            width - 16.0,
            1.0,
            ColorRgba::new(146, 224, 227, 48),
            None,
            0.0,
        ),
    ]
}

fn rect(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    fill: ColorRgba,
    stroke: Option<StrokeStyle>,
    radius: f32,
) -> ScenePrimitive {
    let mut rect = PaintRect::solid(UiRect::new(x, y, width.max(0.0), height.max(0.0)), fill)
        .corner_radii(CornerRadii::uniform(radius));
    if let Some(stroke) = stroke {
        rect = rect.stroke(stroke);
    }
    ScenePrimitive::Rect(rect)
}

fn text(
    label: impl Into<String>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    size: f32,
    color: ColorRgba,
    align: TextHorizontalAlign,
) -> ScenePrimitive {
    ScenePrimitive::Text(
        PaintText::new(
            label,
            UiRect::new(x, y, width, height),
            TextStyle {
                font_size: size,
                line_height: height,
                color,
                ..TextStyle::default()
            },
        )
        .horizontal_align(align)
        .vertical_align(TextVerticalAlign::Center)
        .overflow(TextOverflow::Clip)
        .multiline(false),
    )
}

fn ability_label(slot: usize) -> &'static str {
    match slot {
        0 => "BLINK",
        1 => "MARK",
        2 => "WALL",
        3 => "SYNC",
        _ => "BURST",
    }
}

fn wave01(phase: f32, low: f32, high: f32) -> f32 {
    let t = phase.sin() * 0.5 + 0.5;
    low + (high - low) * t
}

fn tint(color: ColorRgba, factor: f32) -> ColorRgba {
    let channel = |value: u8| (value as f32 * factor).round().clamp(0.0, 255.0) as u8;
    ColorRgba::new(
        channel(color.r),
        channel(color.g),
        channel(color.b),
        color.a,
    )
}

fn right(viewport_width: f32, safe: f32, width: f32) -> f32 {
    (viewport_width - safe - width).max(safe)
}

fn bottom(viewport_height: f32, safe: f32, height: f32) -> f32 {
    (viewport_height - safe - height).max(safe)
}

fn centered(total: f32, size: f32) -> f32 {
    ((total - size) * 0.5).max(0.0)
}

const PANEL_FILL: ColorRgba = ColorRgba::new(8, 12, 17, 184);
const PANEL_STROKE: ColorRgba = ColorRgba::new(70, 91, 104, 210);
const TEXT_MAIN: ColorRgba = ColorRgba::new(233, 241, 244, 255);
const TEXT_MUTED: ColorRgba = ColorRgba::new(149, 169, 180, 255);
const ACCENT_CYAN: ColorRgba = ColorRgba::new(77, 203, 219, 255);
const ACCENT_GREEN: ColorRgba = ColorRgba::new(110, 224, 150, 255);
const ACCENT_AMBER: ColorRgba = ColorRgba::new(238, 194, 86, 255);
const ACCENT_RED: ColorRgba = ColorRgba::new(245, 90, 92, 255);

#[cfg(feature = "native-window")]
fn main() -> operad::native::NativeWindowResult {
    operad::native::run("Game UI", |viewport| {
        game_ui_document(viewport, GameUiState::default())
    })
}

#[cfg(not(feature = "native-window"))]
fn main() {
    let mut document = game_ui_document(GAME_UI_VIEWPORT, GameUiState::default());
    document
        .compute_layout(GAME_UI_VIEWPORT, &mut operad::ApproxTextMeasurer)
        .expect("game UI layout");
    println!(
        "game UI nodes={} paint_items={}",
        document.node_count(),
        document.paint_list().items.len()
    );
}
