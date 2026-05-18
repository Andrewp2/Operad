//! Operad-owned public layout primitives.
//!
//! The types in this module are the preferred public surface for common layout
//! construction. The legacy helper functions at the bottom of the module remain
//! available for migration and advanced callers that still need direct Taffy
//! values.

use taffy::prelude::{
    AlignContent, AlignItems, CompactLength, Dimension, Display, FlexDirection, FlexWrap,
    GridTemplateComponent, JustifyContent, LengthPercentage, LengthPercentageAuto,
    MaxTrackSizingFunction, MinTrackSizingFunction, Position, Rect, Size as TaffySize, Style,
    TrackSizingFunction,
};

use crate::{LayoutStyle, UiNodeStyle, UiPoint, UiRect, UiSize};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayoutLength {
    Points(f32),
    Percent(f32),
}

impl LayoutLength {
    pub const ZERO: Self = Self::Points(0.0);

    pub const fn points(value: f32) -> Self {
        Self::Points(value)
    }

    pub const fn percent(value: f32) -> Self {
        Self::Percent(value)
    }

    pub const fn to_taffy(self) -> LengthPercentage {
        match self {
            Self::Points(value) => LengthPercentage::length(value),
            Self::Percent(value) => LengthPercentage::percent(value),
        }
    }

    pub fn from_taffy(value: LengthPercentage) -> Option<Self> {
        let raw = value.into_raw();
        match raw.tag() {
            CompactLength::LENGTH_TAG => Some(Self::Points(raw.value())),
            CompactLength::PERCENT_TAG => Some(Self::Percent(raw.value())),
            _ => None,
        }
    }
}

impl From<LayoutLength> for LengthPercentage {
    fn from(value: LayoutLength) -> Self {
        value.to_taffy()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayoutDimension {
    Auto,
    Points(f32),
    Percent(f32),
}

impl LayoutDimension {
    pub const AUTO: Self = Self::Auto;
    pub const ZERO: Self = Self::Points(0.0);

    pub const fn points(value: f32) -> Self {
        Self::Points(value)
    }

    pub const fn percent(value: f32) -> Self {
        Self::Percent(value)
    }

    pub const fn points_value(self) -> Option<f32> {
        match self {
            Self::Points(value) => Some(value),
            Self::Auto | Self::Percent(_) => None,
        }
    }

    pub const fn to_taffy(self) -> Dimension {
        match self {
            Self::Auto => Dimension::auto(),
            Self::Points(value) => Dimension::length(value),
            Self::Percent(value) => Dimension::percent(value),
        }
    }

    pub fn from_taffy(value: Dimension) -> Option<Self> {
        match value.tag() {
            CompactLength::AUTO_TAG => Some(Self::Auto),
            CompactLength::LENGTH_TAG => Some(Self::Points(value.value())),
            CompactLength::PERCENT_TAG => Some(Self::Percent(value.value())),
            _ => None,
        }
    }
}

impl From<LayoutDimension> for Dimension {
    fn from(value: LayoutDimension) -> Self {
        value.to_taffy()
    }
}

impl From<LayoutLength> for LayoutDimension {
    fn from(value: LayoutLength) -> Self {
        match value {
            LayoutLength::Points(value) => Self::Points(value),
            LayoutLength::Percent(value) => Self::Percent(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayoutInset {
    Auto,
    Points(f32),
    Percent(f32),
}

impl LayoutInset {
    pub const AUTO: Self = Self::Auto;
    pub const ZERO: Self = Self::Points(0.0);

    pub const fn points(value: f32) -> Self {
        Self::Points(value)
    }

    pub const fn percent(value: f32) -> Self {
        Self::Percent(value)
    }

    pub const fn to_taffy(self) -> LengthPercentageAuto {
        match self {
            Self::Auto => LengthPercentageAuto::auto(),
            Self::Points(value) => LengthPercentageAuto::length(value),
            Self::Percent(value) => LengthPercentageAuto::percent(value),
        }
    }

    pub fn from_taffy(value: LengthPercentageAuto) -> Option<Self> {
        let raw = value.into_raw();
        match raw.tag() {
            CompactLength::AUTO_TAG => Some(Self::Auto),
            CompactLength::LENGTH_TAG => Some(Self::Points(raw.value())),
            CompactLength::PERCENT_TAG => Some(Self::Percent(raw.value())),
            _ => None,
        }
    }
}

impl From<LayoutInset> for LengthPercentageAuto {
    fn from(value: LayoutInset) -> Self {
        value.to_taffy()
    }
}

impl From<LayoutLength> for LayoutInset {
    fn from(value: LayoutLength) -> Self {
        match value {
            LayoutLength::Points(value) => Self::Points(value),
            LayoutLength::Percent(value) => Self::Percent(value),
        }
    }
}

impl From<LayoutDimension> for LayoutInset {
    fn from(value: LayoutDimension) -> Self {
        match value {
            LayoutDimension::Auto => Self::Auto,
            LayoutDimension::Points(value) => Self::Points(value),
            LayoutDimension::Percent(value) => Self::Percent(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutSize {
    pub width: LayoutDimension,
    pub height: LayoutDimension,
}

impl LayoutSize {
    pub const AUTO: Self = Self::new(LayoutDimension::Auto, LayoutDimension::Auto);
    pub const ZERO: Self = Self::points(0.0, 0.0);
    pub const FILL: Self = Self::percent(1.0, 1.0);

    pub const fn new(width: LayoutDimension, height: LayoutDimension) -> Self {
        Self { width, height }
    }

    pub const fn points(width: f32, height: f32) -> Self {
        Self {
            width: LayoutDimension::Points(width),
            height: LayoutDimension::Points(height),
        }
    }

    pub const fn percent(width: f32, height: f32) -> Self {
        Self {
            width: LayoutDimension::Percent(width),
            height: LayoutDimension::Percent(height),
        }
    }

    pub const fn width(mut self, width: LayoutDimension) -> Self {
        self.width = width;
        self
    }

    pub const fn height(mut self, height: LayoutDimension) -> Self {
        self.height = height;
        self
    }

    pub const fn to_taffy(self) -> TaffySize<Dimension> {
        TaffySize {
            width: self.width.to_taffy(),
            height: self.height.to_taffy(),
        }
    }

    pub fn from_taffy(size: TaffySize<Dimension>) -> Option<Self> {
        Some(Self {
            width: LayoutDimension::from_taffy(size.width)?,
            height: LayoutDimension::from_taffy(size.height)?,
        })
    }
}

impl From<LayoutSize> for TaffySize<Dimension> {
    fn from(value: LayoutSize) -> Self {
        value.to_taffy()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutInsets {
    pub left: LayoutInset,
    pub right: LayoutInset,
    pub top: LayoutInset,
    pub bottom: LayoutInset,
}

impl LayoutInsets {
    pub const ZERO: Self = Self::all(LayoutInset::Points(0.0));
    pub const AUTO: Self = Self::all(LayoutInset::Auto);

    pub const fn new(
        left: LayoutInset,
        right: LayoutInset,
        top: LayoutInset,
        bottom: LayoutInset,
    ) -> Self {
        Self {
            left,
            right,
            top,
            bottom,
        }
    }

    pub const fn all(value: LayoutInset) -> Self {
        Self {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }

    pub const fn points(value: f32) -> Self {
        Self::all(LayoutInset::Points(value))
    }

    pub const fn percent(value: f32) -> Self {
        Self::all(LayoutInset::Percent(value))
    }

    pub const fn horizontal_vertical(horizontal: LayoutInset, vertical: LayoutInset) -> Self {
        Self {
            left: horizontal,
            right: horizontal,
            top: vertical,
            bottom: vertical,
        }
    }

    pub const fn left(mut self, value: LayoutInset) -> Self {
        self.left = value;
        self
    }

    pub const fn right(mut self, value: LayoutInset) -> Self {
        self.right = value;
        self
    }

    pub const fn top(mut self, value: LayoutInset) -> Self {
        self.top = value;
        self
    }

    pub const fn bottom(mut self, value: LayoutInset) -> Self {
        self.bottom = value;
        self
    }

    pub const fn to_taffy_rect(self) -> Rect<LengthPercentageAuto> {
        Rect {
            left: self.left.to_taffy(),
            right: self.right.to_taffy(),
            top: self.top.to_taffy(),
            bottom: self.bottom.to_taffy(),
        }
    }

    pub fn from_taffy(rect: Rect<LengthPercentageAuto>) -> Option<Self> {
        Some(Self {
            left: LayoutInset::from_taffy(rect.left)?,
            right: LayoutInset::from_taffy(rect.right)?,
            top: LayoutInset::from_taffy(rect.top)?,
            bottom: LayoutInset::from_taffy(rect.bottom)?,
        })
    }
}

impl From<LayoutInsets> for Rect<LengthPercentageAuto> {
    fn from(value: LayoutInsets) -> Self {
        value.to_taffy_rect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutSpacing {
    pub left: LayoutLength,
    pub right: LayoutLength,
    pub top: LayoutLength,
    pub bottom: LayoutLength,
}

impl LayoutSpacing {
    pub const ZERO: Self = Self::all(LayoutLength::Points(0.0));

    pub const fn new(
        left: LayoutLength,
        right: LayoutLength,
        top: LayoutLength,
        bottom: LayoutLength,
    ) -> Self {
        Self {
            left,
            right,
            top,
            bottom,
        }
    }

    pub const fn all(value: LayoutLength) -> Self {
        Self {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }

    pub const fn points(value: f32) -> Self {
        Self::all(LayoutLength::Points(value))
    }

    pub const fn percent(value: f32) -> Self {
        Self::all(LayoutLength::Percent(value))
    }

    pub const fn horizontal_vertical(horizontal: LayoutLength, vertical: LayoutLength) -> Self {
        Self {
            left: horizontal,
            right: horizontal,
            top: vertical,
            bottom: vertical,
        }
    }

    pub const fn left(mut self, value: LayoutLength) -> Self {
        self.left = value;
        self
    }

    pub const fn right(mut self, value: LayoutLength) -> Self {
        self.right = value;
        self
    }

    pub const fn top(mut self, value: LayoutLength) -> Self {
        self.top = value;
        self
    }

    pub const fn bottom(mut self, value: LayoutLength) -> Self {
        self.bottom = value;
        self
    }

    pub const fn to_taffy_rect(self) -> Rect<LengthPercentage> {
        Rect {
            left: self.left.to_taffy(),
            right: self.right.to_taffy(),
            top: self.top.to_taffy(),
            bottom: self.bottom.to_taffy(),
        }
    }

    pub fn from_taffy(rect: Rect<LengthPercentage>) -> Option<Self> {
        Some(Self {
            left: LayoutLength::from_taffy(rect.left)?,
            right: LayoutLength::from_taffy(rect.right)?,
            top: LayoutLength::from_taffy(rect.top)?,
            bottom: LayoutLength::from_taffy(rect.bottom)?,
        })
    }
}

impl From<LayoutSpacing> for Rect<LengthPercentage> {
    fn from(value: LayoutSpacing) -> Self {
        value.to_taffy_rect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutGap {
    pub column: LayoutLength,
    pub row: LayoutLength,
}

impl LayoutGap {
    pub const ZERO: Self = Self::all(LayoutLength::Points(0.0));

    pub const fn new(column: LayoutLength, row: LayoutLength) -> Self {
        Self { column, row }
    }

    pub const fn all(value: LayoutLength) -> Self {
        Self {
            column: value,
            row: value,
        }
    }

    pub const fn points(column: f32, row: f32) -> Self {
        Self {
            column: LayoutLength::Points(column),
            row: LayoutLength::Points(row),
        }
    }

    pub const fn to_taffy_size(self) -> TaffySize<LengthPercentage> {
        TaffySize {
            width: self.column.to_taffy(),
            height: self.row.to_taffy(),
        }
    }

    pub fn from_taffy(size: TaffySize<LengthPercentage>) -> Option<Self> {
        Some(Self {
            column: LayoutLength::from_taffy(size.width)?,
            row: LayoutLength::from_taffy(size.height)?,
        })
    }
}

impl From<LayoutGap> for TaffySize<LengthPercentage> {
    fn from(value: LayoutGap) -> Self {
        value.to_taffy_size()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutDisplay {
    Flex,
    Grid,
    None,
}

impl LayoutDisplay {
    pub const fn to_taffy(self) -> Display {
        match self {
            Self::Flex => Display::Flex,
            Self::Grid => Display::Grid,
            Self::None => Display::None,
        }
    }

    pub const fn from_taffy(value: Display) -> Option<Self> {
        match value {
            Display::Flex => Some(Self::Flex),
            Display::Grid => Some(Self::Grid),
            Display::None => Some(Self::None),
            _ => None,
        }
    }
}

impl Default for LayoutDisplay {
    fn default() -> Self {
        Self::Flex
    }
}

impl From<LayoutDisplay> for Display {
    fn from(value: LayoutDisplay) -> Self {
        value.to_taffy()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutPosition {
    Relative,
    Absolute,
}

impl LayoutPosition {
    pub const fn to_taffy(self) -> Position {
        match self {
            Self::Relative => Position::Relative,
            Self::Absolute => Position::Absolute,
        }
    }

    pub const fn from_taffy(value: Position) -> Self {
        match value {
            Position::Relative => Self::Relative,
            Position::Absolute => Self::Absolute,
        }
    }
}

impl Default for LayoutPosition {
    fn default() -> Self {
        Self::Relative
    }
}

impl From<LayoutPosition> for Position {
    fn from(value: LayoutPosition) -> Self {
        value.to_taffy()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutFlexDirection {
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

impl LayoutFlexDirection {
    pub const fn to_taffy(self) -> FlexDirection {
        match self {
            Self::Row => FlexDirection::Row,
            Self::Column => FlexDirection::Column,
            Self::RowReverse => FlexDirection::RowReverse,
            Self::ColumnReverse => FlexDirection::ColumnReverse,
        }
    }

    pub const fn from_taffy(value: FlexDirection) -> Self {
        match value {
            FlexDirection::Row => Self::Row,
            FlexDirection::Column => Self::Column,
            FlexDirection::RowReverse => Self::RowReverse,
            FlexDirection::ColumnReverse => Self::ColumnReverse,
        }
    }
}

impl Default for LayoutFlexDirection {
    fn default() -> Self {
        Self::Row
    }
}

impl From<LayoutFlexDirection> for FlexDirection {
    fn from(value: LayoutFlexDirection) -> Self {
        value.to_taffy()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutFlexWrap {
    NoWrap,
    Wrap,
    WrapReverse,
}

impl LayoutFlexWrap {
    pub const fn to_taffy(self) -> FlexWrap {
        match self {
            Self::NoWrap => FlexWrap::NoWrap,
            Self::Wrap => FlexWrap::Wrap,
            Self::WrapReverse => FlexWrap::WrapReverse,
        }
    }

    pub const fn from_taffy(value: FlexWrap) -> Self {
        match value {
            FlexWrap::NoWrap => Self::NoWrap,
            FlexWrap::Wrap => Self::Wrap,
            FlexWrap::WrapReverse => Self::WrapReverse,
        }
    }
}

impl Default for LayoutFlexWrap {
    fn default() -> Self {
        Self::NoWrap
    }
}

impl From<LayoutFlexWrap> for FlexWrap {
    fn from(value: LayoutFlexWrap) -> Self {
        value.to_taffy()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutAlignment {
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Stretch,
}

impl LayoutAlignment {
    pub const fn to_taffy(self) -> AlignItems {
        match self {
            Self::Start => AlignItems::Start,
            Self::End => AlignItems::End,
            Self::FlexStart => AlignItems::FlexStart,
            Self::FlexEnd => AlignItems::FlexEnd,
            Self::Center => AlignItems::Center,
            Self::Baseline => AlignItems::Baseline,
            Self::Stretch => AlignItems::Stretch,
        }
    }

    pub const fn from_taffy(value: AlignItems) -> Self {
        match value {
            AlignItems::Start => Self::Start,
            AlignItems::End => Self::End,
            AlignItems::FlexStart => Self::FlexStart,
            AlignItems::FlexEnd => Self::FlexEnd,
            AlignItems::Center => Self::Center,
            AlignItems::Baseline => Self::Baseline,
            AlignItems::Stretch => Self::Stretch,
        }
    }
}

impl From<LayoutAlignment> for AlignItems {
    fn from(value: LayoutAlignment) -> Self {
        value.to_taffy()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutJustifyContent {
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

impl LayoutJustifyContent {
    pub const fn to_taffy(self) -> JustifyContent {
        match self {
            Self::Start => AlignContent::Start,
            Self::End => AlignContent::End,
            Self::FlexStart => AlignContent::FlexStart,
            Self::FlexEnd => AlignContent::FlexEnd,
            Self::Center => AlignContent::Center,
            Self::Stretch => AlignContent::Stretch,
            Self::SpaceBetween => AlignContent::SpaceBetween,
            Self::SpaceAround => AlignContent::SpaceAround,
            Self::SpaceEvenly => AlignContent::SpaceEvenly,
        }
    }

    pub const fn from_taffy(value: JustifyContent) -> Self {
        match value {
            AlignContent::Start => Self::Start,
            AlignContent::End => Self::End,
            AlignContent::FlexStart => Self::FlexStart,
            AlignContent::FlexEnd => Self::FlexEnd,
            AlignContent::Center => Self::Center,
            AlignContent::Stretch => Self::Stretch,
            AlignContent::SpaceBetween => Self::SpaceBetween,
            AlignContent::SpaceEvenly => Self::SpaceEvenly,
            AlignContent::SpaceAround => Self::SpaceAround,
        }
    }
}

impl From<LayoutJustifyContent> for JustifyContent {
    fn from(value: LayoutJustifyContent) -> Self {
        value.to_taffy()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayoutGridTrack {
    Auto,
    Points(f32),
    Fraction(f32),
    MinMaxPointsFraction { min: f32, max_fraction: f32 },
}

impl LayoutGridTrack {
    pub const fn auto() -> Self {
        Self::Auto
    }

    pub const fn points(value: f32) -> Self {
        Self::Points(value)
    }

    pub const fn fraction(value: f32) -> Self {
        Self::Fraction(value)
    }

    pub const fn minmax_points_fraction(min: f32, max_fraction: f32) -> Self {
        Self::MinMaxPointsFraction { min, max_fraction }
    }

    fn to_taffy(self) -> GridTemplateComponent<String> {
        let track = match self {
            Self::Auto => TrackSizingFunction {
                min: MinTrackSizingFunction::auto(),
                max: MaxTrackSizingFunction::auto(),
            },
            Self::Points(value) => {
                let value = value.max(0.0);
                TrackSizingFunction {
                    min: MinTrackSizingFunction::length(value),
                    max: MaxTrackSizingFunction::length(value),
                }
            }
            Self::Fraction(value) => TrackSizingFunction {
                min: MinTrackSizingFunction::length(0.0),
                max: MaxTrackSizingFunction::fr(value.max(0.0)),
            },
            Self::MinMaxPointsFraction { min, max_fraction } => TrackSizingFunction {
                min: MinTrackSizingFunction::length(min.max(0.0)),
                max: MaxTrackSizingFunction::fr(max_fraction.max(0.0)),
            },
        };
        GridTemplateComponent::Single(track)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    pub display: LayoutDisplay,
    pub position: LayoutPosition,
    pub size: LayoutSize,
    pub min_size: LayoutSize,
    pub max_size: LayoutSize,
    pub inset: LayoutInsets,
    pub margin: LayoutInsets,
    pub padding: LayoutSpacing,
    pub gap: LayoutGap,
    pub flex_direction: LayoutFlexDirection,
    pub flex_wrap: LayoutFlexWrap,
    pub align_items: Option<LayoutAlignment>,
    pub justify_content: Option<LayoutJustifyContent>,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: LayoutDimension,
}

impl Layout {
    pub const DEFAULT: Self = Self {
        display: LayoutDisplay::Flex,
        position: LayoutPosition::Relative,
        size: LayoutSize::AUTO,
        min_size: LayoutSize::AUTO,
        max_size: LayoutSize::AUTO,
        inset: LayoutInsets::AUTO,
        margin: LayoutInsets::ZERO,
        padding: LayoutSpacing::ZERO,
        gap: LayoutGap::ZERO,
        flex_direction: LayoutFlexDirection::Row,
        flex_wrap: LayoutFlexWrap::NoWrap,
        align_items: None,
        justify_content: None,
        flex_grow: 0.0,
        flex_shrink: 1.0,
        flex_basis: LayoutDimension::Auto,
    };

    pub const fn new() -> Self {
        Self::DEFAULT
    }

    pub const fn row() -> Self {
        Self::new().display(LayoutDisplay::Flex)
    }

    pub const fn column() -> Self {
        Self::row().flex_direction(LayoutFlexDirection::Column)
    }

    pub const fn grid() -> Self {
        Self::new().display(LayoutDisplay::Grid)
    }

    pub const fn fixed(width: f32, height: f32) -> Self {
        Self::new().size(LayoutSize::points(width, height))
    }

    pub const fn fill() -> Self {
        Self::new().size(LayoutSize::FILL)
    }

    pub const fn display(mut self, display: LayoutDisplay) -> Self {
        self.display = display;
        self
    }

    pub const fn position(mut self, position: LayoutPosition) -> Self {
        self.position = position;
        self
    }

    pub const fn size(mut self, size: LayoutSize) -> Self {
        self.size = size;
        self
    }

    pub const fn width(mut self, width: LayoutDimension) -> Self {
        self.size = self.size.width(width);
        self
    }

    pub const fn height(mut self, height: LayoutDimension) -> Self {
        self.size = self.size.height(height);
        self
    }

    pub const fn min_size(mut self, size: LayoutSize) -> Self {
        self.min_size = size;
        self
    }

    pub const fn max_size(mut self, size: LayoutSize) -> Self {
        self.max_size = size;
        self
    }

    pub const fn inset(mut self, inset: LayoutInsets) -> Self {
        self.inset = inset;
        self
    }

    pub const fn margin(mut self, margin: LayoutInsets) -> Self {
        self.margin = margin;
        self
    }

    pub const fn padding(mut self, padding: LayoutSpacing) -> Self {
        self.padding = padding;
        self
    }

    pub const fn gap(mut self, gap: LayoutGap) -> Self {
        self.gap = gap;
        self
    }

    pub const fn flex_direction(mut self, direction: LayoutFlexDirection) -> Self {
        self.flex_direction = direction;
        self
    }

    pub const fn flex_wrap(mut self, wrap: LayoutFlexWrap) -> Self {
        self.flex_wrap = wrap;
        self
    }

    pub const fn align_items(mut self, alignment: LayoutAlignment) -> Self {
        self.align_items = Some(alignment);
        self
    }

    pub const fn justify_content(mut self, alignment: LayoutJustifyContent) -> Self {
        self.justify_content = Some(alignment);
        self
    }

    pub const fn flex(mut self, grow: f32, shrink: f32, basis: LayoutDimension) -> Self {
        self.flex_grow = grow;
        self.flex_shrink = shrink;
        self.flex_basis = basis;
        self
    }

    pub fn to_taffy_style(self) -> Style {
        Style {
            display: self.display.to_taffy(),
            position: self.position.to_taffy(),
            size: self.size.to_taffy(),
            min_size: self.min_size.to_taffy(),
            max_size: self.max_size.to_taffy(),
            inset: self.inset.to_taffy_rect(),
            margin: self.margin.to_taffy_rect(),
            padding: self.padding.to_taffy_rect(),
            gap: self.gap.to_taffy_size(),
            flex_direction: self.flex_direction.to_taffy(),
            flex_wrap: self.flex_wrap.to_taffy(),
            align_items: self.align_items.map(LayoutAlignment::to_taffy),
            justify_content: self.justify_content.map(LayoutJustifyContent::to_taffy),
            flex_grow: self.flex_grow.max(0.0),
            flex_shrink: self.flex_shrink.max(0.0),
            flex_basis: self.flex_basis.to_taffy(),
            ..Default::default()
        }
    }

    pub fn to_layout_style(self) -> LayoutStyle {
        LayoutStyle::from_taffy_style(self.to_taffy_style())
    }

    pub fn from_layout_style(style: &LayoutStyle) -> Option<Self> {
        Self::from_taffy_style(style.as_taffy_style())
    }

    pub fn from_taffy_style(style: &Style) -> Option<Self> {
        Some(Self {
            display: LayoutDisplay::from_taffy(style.display)?,
            position: LayoutPosition::from_taffy(style.position),
            size: LayoutSize::from_taffy(style.size)?,
            min_size: LayoutSize::from_taffy(style.min_size)?,
            max_size: LayoutSize::from_taffy(style.max_size)?,
            inset: LayoutInsets::from_taffy(style.inset)?,
            margin: LayoutInsets::from_taffy(style.margin)?,
            padding: LayoutSpacing::from_taffy(style.padding)?,
            gap: LayoutGap::from_taffy(style.gap)?,
            flex_direction: LayoutFlexDirection::from_taffy(style.flex_direction),
            flex_wrap: LayoutFlexWrap::from_taffy(style.flex_wrap),
            align_items: style.align_items.map(LayoutAlignment::from_taffy),
            justify_content: style.justify_content.map(LayoutJustifyContent::from_taffy),
            flex_grow: style.flex_grow,
            flex_shrink: style.flex_shrink,
            flex_basis: LayoutDimension::from_taffy(style.flex_basis)?,
        })
    }
}

impl Default for Layout {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Layout> for Style {
    fn from(value: Layout) -> Self {
        value.to_taffy_style()
    }
}

impl From<Layout> for LayoutStyle {
    fn from(value: Layout) -> Self {
        value.to_layout_style()
    }
}

pub fn from_layout(layout: Layout) -> LayoutStyle {
    layout.to_layout_style()
}

pub fn owned_size(width: LayoutDimension, height: LayoutDimension) -> LayoutStyle {
    Layout::new()
        .size(LayoutSize::new(width, height))
        .to_layout_style()
}

pub fn px(value: f32) -> Dimension {
    Dimension::length(value)
}

pub fn percent(value: f32) -> Dimension {
    Dimension::percent(value)
}

pub fn auto() -> Dimension {
    Dimension::auto()
}

pub fn spacing(value: f32) -> LengthPercentage {
    LengthPercentage::length(value)
}

pub fn fixed(width: f32, height: f32) -> LayoutStyle {
    Layout::fixed(width, height).to_layout_style()
}

pub fn absolute(x: f32, y: f32, width: f32, height: f32) -> LayoutStyle {
    with_absolute_position(fixed(width, height), x, y)
}

pub fn absolute_fill() -> LayoutStyle {
    Layout::fill()
        .position(LayoutPosition::Absolute)
        .inset(LayoutInsets::points(0.0))
        .to_layout_style()
}

pub fn size(width: Dimension, height: Dimension) -> LayoutStyle {
    LayoutStyle::from_taffy_style(Style {
        size: TaffySize { width, height },
        ..Default::default()
    })
}

pub fn row() -> LayoutStyle {
    Layout::row().to_layout_style()
}

pub fn column() -> LayoutStyle {
    Layout::column().to_layout_style()
}

pub fn centered_row() -> LayoutStyle {
    with_centered_children(row())
}

pub fn wrapping_row() -> LayoutStyle {
    Layout::row()
        .flex_wrap(LayoutFlexWrap::Wrap)
        .to_layout_style()
}

pub fn grid() -> LayoutStyle {
    Layout::grid().to_layout_style()
}

pub fn with_grid_template_columns(
    mut style: LayoutStyle,
    columns: impl IntoIterator<Item = LayoutGridTrack>,
) -> LayoutStyle {
    style.as_taffy_style_mut().grid_template_columns =
        columns.into_iter().map(LayoutGridTrack::to_taffy).collect();
    style
}

pub fn with_grid_template_rows(
    mut style: LayoutStyle,
    rows: impl IntoIterator<Item = LayoutGridTrack>,
) -> LayoutStyle {
    style.as_taffy_style_mut().grid_template_rows =
        rows.into_iter().map(LayoutGridTrack::to_taffy).collect();
    style
}

pub fn centered_column() -> LayoutStyle {
    with_centered_children(column())
}

pub fn fill() -> LayoutStyle {
    Layout::fill().to_layout_style()
}

pub fn flex_item(grow: f32, shrink: f32, basis: Dimension) -> LayoutStyle {
    with_flex(LayoutStyle::default(), grow, shrink, basis)
}

pub fn with_size(mut style: LayoutStyle, width: Dimension, height: Dimension) -> LayoutStyle {
    style.as_taffy_style_mut().size = TaffySize { width, height };
    style
}

pub fn with_min_size(mut style: LayoutStyle, width: Dimension, height: Dimension) -> LayoutStyle {
    style.as_taffy_style_mut().min_size = TaffySize { width, height };
    style
}

pub fn with_max_size(mut style: LayoutStyle, width: Dimension, height: Dimension) -> LayoutStyle {
    style.as_taffy_style_mut().max_size = TaffySize { width, height };
    style
}

pub fn with_absolute_position(mut style: LayoutStyle, x: f32, y: f32) -> LayoutStyle {
    let taffy_style = style.as_taffy_style_mut();
    taffy_style.position = Position::Absolute;
    taffy_style.inset.left = LengthPercentageAuto::length(x);
    taffy_style.inset.top = LengthPercentageAuto::length(y);
    taffy_style.inset.right = LengthPercentageAuto::auto();
    taffy_style.inset.bottom = LengthPercentageAuto::auto();
    style
}

pub fn with_flex(mut style: LayoutStyle, grow: f32, shrink: f32, basis: Dimension) -> LayoutStyle {
    let taffy_style = style.as_taffy_style_mut();
    taffy_style.flex_grow = grow.max(0.0);
    taffy_style.flex_shrink = shrink.max(0.0);
    taffy_style.flex_basis = basis;
    style
}

pub fn with_flex_wrap(mut style: LayoutStyle, wrap: LayoutFlexWrap) -> LayoutStyle {
    style.as_taffy_style_mut().flex_wrap = wrap.to_taffy();
    style
}

pub fn with_centered_children(mut style: LayoutStyle) -> LayoutStyle {
    let taffy_style = style.as_taffy_style_mut();
    taffy_style.align_items = Some(AlignItems::Center);
    taffy_style.justify_content = Some(JustifyContent::Center);
    style
}

pub fn with_flex_start_children(mut style: LayoutStyle) -> LayoutStyle {
    style.as_taffy_style_mut().align_items = Some(AlignItems::FlexStart);
    style
}

pub fn with_gap(mut style: LayoutStyle, column_gap: f32, row_gap: f32) -> LayoutStyle {
    style.as_taffy_style_mut().gap = TaffySize {
        width: spacing(column_gap.max(0.0)),
        height: spacing(row_gap.max(0.0)),
    };
    style
}

pub fn with_gap_all(style: LayoutStyle, value: f32) -> LayoutStyle {
    with_gap(style, value, value)
}

pub fn with_margin_all(mut style: LayoutStyle, value: f32) -> LayoutStyle {
    style.as_taffy_style_mut().margin = Rect::length(value);
    style
}

pub fn with_margin_left(mut style: LayoutStyle, value: f32) -> LayoutStyle {
    style.as_taffy_style_mut().margin.left = LengthPercentageAuto::length(value);
    style
}

pub fn with_margin_right(mut style: LayoutStyle, value: f32) -> LayoutStyle {
    style.as_taffy_style_mut().margin.right = LengthPercentageAuto::length(value);
    style
}

pub fn with_margin_top(mut style: LayoutStyle, value: f32) -> LayoutStyle {
    style.as_taffy_style_mut().margin.top = LengthPercentageAuto::length(value);
    style
}

pub fn with_margin_bottom(mut style: LayoutStyle, value: f32) -> LayoutStyle {
    style.as_taffy_style_mut().margin.bottom = LengthPercentageAuto::length(value);
    style
}

pub fn with_padding_all(mut style: LayoutStyle, value: f32) -> LayoutStyle {
    style.as_taffy_style_mut().padding = Rect::length(value);
    style
}

pub fn with_auto_horizontal_margin(mut style: LayoutStyle) -> LayoutStyle {
    let taffy_style = style.as_taffy_style_mut();
    taffy_style.margin.left = LengthPercentageAuto::auto();
    taffy_style.margin.right = LengthPercentageAuto::auto();
    style
}

pub fn node_style(layout: impl Into<LayoutStyle>) -> UiNodeStyle {
    UiNodeStyle::new(layout)
}

pub fn clipped_node_style(layout: impl Into<LayoutStyle>) -> UiNodeStyle {
    UiNodeStyle::clipped(layout)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContainedFlowLayout {
    pub bounds: UiRect,
    pub gap: f32,
    pub cascade_offset: f32,
    cursor: UiPoint,
    row_height: f32,
    placed: usize,
    overflow_index: usize,
}

impl ContainedFlowLayout {
    pub fn new(bounds: UiRect) -> Self {
        let bounds = finite_rect(bounds);
        Self {
            bounds,
            gap: 0.0,
            cascade_offset: 0.0,
            cursor: UiPoint::new(bounds.x, bounds.y),
            row_height: 0.0,
            placed: 0,
            overflow_index: 0,
        }
    }

    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = finite_or(gap, 0.0).max(0.0);
        self
    }

    pub fn with_cascade_offset(mut self, offset: f32) -> Self {
        self.cascade_offset = finite_or(offset, 0.0).max(0.0);
        self
    }

    pub fn next_rect(&mut self, preferred_size: UiSize, min_size: UiSize) -> UiRect {
        let size = contain_size(preferred_size, min_size, bounds_size(self.bounds));
        if self.cursor.x > self.bounds.x && self.cursor.x + size.width > self.bounds.right() {
            self.cursor.x = self.bounds.x;
            self.cursor.y += self.row_height + self.gap;
            self.row_height = 0.0;
        }

        let rect = if self.cursor.y + size.height > self.bounds.bottom() && self.placed > 0 {
            let offset = self.cascade_offset * (self.overflow_index % 8) as f32;
            self.overflow_index += 1;
            contain_rect(
                UiRect::new(
                    self.bounds.x + offset,
                    self.bounds.y + offset,
                    size.width,
                    size.height,
                ),
                self.bounds,
                min_size,
            )
        } else {
            let rect = contain_rect(
                UiRect::new(self.cursor.x, self.cursor.y, size.width, size.height),
                self.bounds,
                min_size,
            );
            self.cursor.x = rect.right() + self.gap;
            self.row_height = self.row_height.max(rect.height);
            rect
        };

        self.placed += 1;
        rect
    }
}

pub fn inset_rect(rect: UiRect, margin: f32) -> UiRect {
    let rect = finite_rect(rect);
    let margin = finite_or(margin, 0.0).max(0.0);
    let x_margin = margin.min(rect.width * 0.5);
    let y_margin = margin.min(rect.height * 0.5);
    UiRect::new(
        rect.x + x_margin,
        rect.y + y_margin,
        (rect.width - x_margin * 2.0).max(0.0),
        (rect.height - y_margin * 2.0).max(0.0),
    )
}

pub fn rect_overflow_amount(rect: UiRect, bounds: UiRect) -> f32 {
    let rect = finite_rect(rect);
    let bounds = finite_rect(bounds);
    (bounds.x - rect.x).max(0.0)
        + (rect.right() - bounds.right()).max(0.0)
        + (bounds.y - rect.y).max(0.0)
        + (rect.bottom() - bounds.bottom()).max(0.0)
}

pub fn contain_size(preferred_size: UiSize, min_size: UiSize, bounds: UiSize) -> UiSize {
    let available = finite_size(bounds);
    let min = UiSize::new(
        finite_or(min_size.width, 0.0).max(0.0).min(available.width),
        finite_or(min_size.height, 0.0)
            .max(0.0)
            .min(available.height),
    );
    UiSize::new(
        finite_or(preferred_size.width, min.width)
            .max(min.width)
            .min(available.width),
        finite_or(preferred_size.height, min.height)
            .max(min.height)
            .min(available.height),
    )
}

pub fn contain_rect(rect: UiRect, bounds: UiRect, min_size: UiSize) -> UiRect {
    let bounds = finite_rect(bounds);
    let rect = finite_rect(rect);
    let size = contain_size(
        UiSize::new(rect.width, rect.height),
        min_size,
        bounds_size(bounds),
    );
    let (x, width) = contain_axis(rect.x, size.width, bounds.x, bounds.right());
    let (y, height) = contain_axis(rect.y, size.height, bounds.y, bounds.bottom());
    UiRect::new(x, y, width, height)
}

pub fn contain_rect_from_origin(rect: UiRect, bounds: UiRect, min_size: UiSize) -> UiRect {
    let bounds = finite_rect(bounds);
    let rect = finite_rect(rect);
    let min_size = contain_size(min_size, UiSize::ZERO, bounds_size(bounds));
    let x = contain_origin_for_min(rect.x, bounds.x, bounds.right(), min_size.width);
    let y = contain_origin_for_min(rect.y, bounds.y, bounds.bottom(), min_size.height);
    let max_width = (bounds.right() - x).max(min_size.width);
    let max_height = (bounds.bottom() - y).max(min_size.height);
    UiRect::new(
        x,
        y,
        finite_or(rect.width, min_size.width).clamp(min_size.width, max_width),
        finite_or(rect.height, min_size.height).clamp(min_size.height, max_height),
    )
}

fn bounds_size(bounds: UiRect) -> UiSize {
    UiSize::new(bounds.width.max(0.0), bounds.height.max(0.0))
}

fn contain_axis(start: f32, size: f32, min: f32, max: f32) -> (f32, f32) {
    let available = (max - min).max(0.0);
    let size = finite_or(size, 0.0).max(0.0).min(available);
    let max_start = max - size;
    let start = if max_start <= min {
        min
    } else {
        finite_or(start, min).clamp(min, max_start)
    };
    (start, size)
}

fn contain_origin_for_min(start: f32, min: f32, max: f32, min_size: f32) -> f32 {
    let max_start = (max - min_size).max(min);
    finite_or(start, min).clamp(min, max_start)
}

fn finite_rect(rect: UiRect) -> UiRect {
    UiRect::new(
        finite_or(rect.x, 0.0),
        finite_or(rect.y, 0.0),
        finite_or(rect.width, 0.0).max(0.0),
        finite_or(rect.height, 0.0).max(0.0),
    )
}

fn finite_size(size: UiSize) -> UiSize {
    UiSize::new(
        finite_or(size.width, 0.0).max(0.0),
        finite_or(size.height, 0.0).max(0.0),
    )
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use taffy::prelude::{
        AlignContent, AlignItems, Dimension, Display, LengthPercentage, LengthPercentageAuto,
        Position,
    };

    use super::*;

    #[test]
    fn owned_layout_converts_to_taffy_style() {
        let layout = Layout::new()
            .position(LayoutPosition::Absolute)
            .size(LayoutSize::new(
                LayoutDimension::points(320.0),
                LayoutDimension::percent(0.5),
            ))
            .min_size(LayoutSize::points(120.0, 32.0))
            .max_size(LayoutSize::new(
                LayoutDimension::percent(1.0),
                LayoutDimension::Auto,
            ))
            .inset(
                LayoutInsets::horizontal_vertical(
                    LayoutInset::points(12.0),
                    LayoutInset::points(20.0),
                )
                .right(LayoutInset::Auto),
            )
            .margin(LayoutInsets::points(4.0).left(LayoutInset::Auto))
            .padding(LayoutSpacing::horizontal_vertical(
                LayoutLength::points(8.0),
                LayoutLength::points(6.0),
            ))
            .gap(LayoutGap::points(10.0, 14.0))
            .flex_direction(LayoutFlexDirection::RowReverse)
            .flex_wrap(LayoutFlexWrap::Wrap)
            .align_items(LayoutAlignment::Center)
            .justify_content(LayoutJustifyContent::SpaceBetween)
            .flex(2.0, 0.5, LayoutDimension::points(64.0));

        let style = layout.to_layout_style();
        let taffy = style.as_taffy_style();

        assert_eq!(taffy.position, Position::Absolute);
        assert_eq!(taffy.size.width, Dimension::length(320.0));
        assert_eq!(taffy.size.height, Dimension::percent(0.5));
        assert_eq!(taffy.min_size.width, Dimension::length(120.0));
        assert_eq!(taffy.max_size.width, Dimension::percent(1.0));
        assert_eq!(taffy.max_size.height, Dimension::auto());
        assert_eq!(taffy.inset.left, LengthPercentageAuto::length(12.0));
        assert_eq!(taffy.inset.right, LengthPercentageAuto::auto());
        assert_eq!(taffy.margin.left, LengthPercentageAuto::auto());
        assert_eq!(taffy.padding.left, LengthPercentage::length(8.0));
        assert_eq!(taffy.padding.top, LengthPercentage::length(6.0));
        assert_eq!(taffy.gap.width, LengthPercentage::length(10.0));
        assert_eq!(taffy.gap.height, LengthPercentage::length(14.0));
        assert_eq!(taffy.flex_direction, FlexDirection::RowReverse);
        assert_eq!(taffy.flex_wrap, FlexWrap::Wrap);
        assert_eq!(taffy.align_items, Some(AlignItems::Center));
        assert_eq!(taffy.justify_content, Some(AlignContent::SpaceBetween));
        assert_eq!(taffy.flex_grow, 2.0);
        assert_eq!(taffy.flex_shrink, 0.5);
        assert_eq!(taffy.flex_basis, Dimension::length(64.0));

        let recovered = Layout::from_layout_style(&style).expect("common layout should recover");
        assert_eq!(recovered.size, layout.size);
        assert_eq!(recovered.inset, layout.inset);
        assert_eq!(recovered.padding, layout.padding);
        assert_eq!(recovered.flex_direction, layout.flex_direction);
        assert_eq!(recovered.flex_wrap, layout.flex_wrap);
    }

    #[test]
    fn owned_layout_values_round_trip_from_taffy_basics() {
        assert_eq!(
            LayoutDimension::from_taffy(Dimension::percent(0.25)),
            Some(LayoutDimension::Percent(0.25))
        );
        assert_eq!(
            LayoutInset::from_taffy(LengthPercentageAuto::auto()),
            Some(LayoutInset::Auto)
        );
        assert_eq!(
            LayoutLength::from_taffy(LengthPercentage::length(6.0)),
            Some(LayoutLength::Points(6.0))
        );
    }

    #[test]
    fn grid_layout_sets_display_and_tracks() {
        let style = with_grid_template_columns(
            Layout::grid()
                .size(LayoutSize::points(320.0, 120.0))
                .gap(LayoutGap::points(8.0, 12.0))
                .to_layout_style(),
            [
                LayoutGridTrack::points(120.0),
                LayoutGridTrack::minmax_points_fraction(80.0, 1.0),
                LayoutGridTrack::fraction(2.0),
            ],
        );
        let taffy = style.as_taffy_style();

        assert_eq!(taffy.display, Display::Grid);
        assert_eq!(taffy.grid_template_columns.len(), 3);
        assert_eq!(taffy.gap.width, LengthPercentage::length(8.0));
        assert_eq!(
            LayoutDisplay::from_taffy(Display::Grid),
            Some(LayoutDisplay::Grid)
        );
    }

    #[test]
    fn containment_shifts_and_shrinks_rects_to_bounds() {
        let bounds = UiRect::new(10.0, 20.0, 100.0, 80.0);

        assert_eq!(
            contain_rect(
                UiRect::new(90.0, 30.0, 40.0, 20.0),
                bounds,
                UiSize::new(20.0, 10.0),
            ),
            UiRect::new(70.0, 30.0, 40.0, 20.0)
        );
        assert_eq!(
            contain_rect(
                UiRect::new(-40.0, 0.0, 180.0, 120.0),
                bounds,
                UiSize::new(20.0, 10.0),
            ),
            bounds
        );
        assert_eq!(
            inset_rect(bounds, 15.0),
            UiRect::new(25.0, 35.0, 70.0, 50.0)
        );
        assert_eq!(rect_overflow_amount(bounds, bounds), 0.0);
    }

    #[test]
    fn origin_preserving_containment_shrinks_trailing_edges() {
        let bounds = UiRect::new(12.0, 12.0, 376.0, 276.0);

        assert_eq!(
            contain_rect_from_origin(
                UiRect::new(100.0, 40.0, 600.0, 120.0),
                bounds,
                UiSize::new(160.0, 96.0),
            ),
            UiRect::new(100.0, 40.0, 288.0, 120.0)
        );
        assert_eq!(
            contain_rect_from_origin(
                UiRect::new(380.0, 280.0, 160.0, 120.0),
                bounds,
                UiSize::new(160.0, 96.0),
            ),
            UiRect::new(228.0, 192.0, 160.0, 96.0)
        );
    }

    #[test]
    fn contained_flow_wraps_and_cascades_inside_bounds() {
        let mut flow = ContainedFlowLayout::new(UiRect::new(0.0, 0.0, 220.0, 120.0))
            .with_gap(10.0)
            .with_cascade_offset(12.0);

        let first = flow.next_rect(UiSize::new(100.0, 40.0), UiSize::new(40.0, 20.0));
        let second = flow.next_rect(UiSize::new(100.0, 40.0), UiSize::new(40.0, 20.0));
        let third = flow.next_rect(UiSize::new(100.0, 40.0), UiSize::new(40.0, 20.0));
        let fourth = flow.next_rect(UiSize::new(200.0, 90.0), UiSize::new(40.0, 20.0));

        assert_eq!(first, UiRect::new(0.0, 0.0, 100.0, 40.0));
        assert_eq!(second, UiRect::new(110.0, 0.0, 100.0, 40.0));
        assert_eq!(third, UiRect::new(0.0, 50.0, 100.0, 40.0));
        assert!(UiRect::new(0.0, 0.0, 220.0, 120.0).contains_rect(fourth));
    }
}
