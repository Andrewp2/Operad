//! Renderer-neutral chart, sparkline, and dense grid-map geometry helpers.
//!
//! These helpers turn app-owned numeric snapshots into stable paint geometry.
//! Operad does not own chart semantics, wafer-map meaning, process metrics, or
//! DAW/editor data; it only provides predictable coordinate mapping and hit
//! geometry that backends and tests can share.

use crate::{
    AccessibilityMeta, AccessibilityRole, AccessibilitySummary, PaintPath, UiPoint, UiRect, UiSize,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChartRange {
    pub min: f32,
    pub max: f32,
}

impl ChartRange {
    pub fn new(min: f32, max: f32) -> Self {
        if !min.is_finite() || !max.is_finite() {
            return Self::default();
        }

        let (min, max) = if min <= max { (min, max) } else { (max, min) };
        if (max - min).abs() <= f32::EPSILON {
            let padding = min.abs().max(1.0) * 0.5;
            return Self {
                min: min - padding,
                max: max + padding,
            };
        }

        Self { min, max }
    }

    pub fn from_values(values: &[f32]) -> Self {
        let mut range: Option<(f32, f32)> = None;
        for value in values.iter().copied().filter(|value| value.is_finite()) {
            range = Some(match range {
                Some((min, max)) => (min.min(value), max.max(value)),
                None => (value, value),
            });
        }

        range
            .map(|(min, max)| Self::new(min, max))
            .unwrap_or_default()
    }

    pub fn span(self) -> f32 {
        (self.max - self.min).max(f32::EPSILON)
    }

    pub fn normalized(self, value: f32) -> f32 {
        if value.is_finite() {
            (value - self.min) / self.span()
        } else {
            0.0
        }
    }

    pub fn normalized_clamped(self, value: f32) -> f32 {
        self.normalized(value).clamp(0.0, 1.0)
    }
}

impl Default for ChartRange {
    fn default() -> Self {
        Self { min: 0.0, max: 1.0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChartSample {
    pub x: f32,
    pub y: f32,
}

impl ChartSample {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChartDataSummary {
    pub sample_count: usize,
    pub finite_sample_count: usize,
    pub x_min: Option<f32>,
    pub x_max: Option<f32>,
    pub y_min: Option<f32>,
    pub y_max: Option<f32>,
    pub first_sample: Option<ChartSample>,
    pub last_sample: Option<ChartSample>,
    pub min_y_sample: Option<ChartSample>,
    pub max_y_sample: Option<ChartSample>,
}

impl ChartDataSummary {
    pub fn from_samples(samples: impl IntoIterator<Item = ChartSample>) -> Self {
        let mut summary = Self {
            sample_count: 0,
            finite_sample_count: 0,
            x_min: None,
            x_max: None,
            y_min: None,
            y_max: None,
            first_sample: None,
            last_sample: None,
            min_y_sample: None,
            max_y_sample: None,
        };

        for sample in samples {
            summary.sample_count += 1;
            if !sample.is_finite() {
                continue;
            }

            summary.finite_sample_count += 1;
            summary.first_sample.get_or_insert(sample);
            summary.last_sample = Some(sample);
            summary.x_min = Some(summary.x_min.map_or(sample.x, |value| value.min(sample.x)));
            summary.x_max = Some(summary.x_max.map_or(sample.x, |value| value.max(sample.x)));
            summary.y_min = Some(summary.y_min.map_or(sample.y, |value| value.min(sample.y)));
            summary.y_max = Some(summary.y_max.map_or(sample.y, |value| value.max(sample.y)));
            if summary
                .min_y_sample
                .is_none_or(|current| sample.y < current.y)
            {
                summary.min_y_sample = Some(sample);
            }
            if summary
                .max_y_sample
                .is_none_or(|current| sample.y > current.y)
            {
                summary.max_y_sample = Some(sample);
            }
        }

        summary
    }

    pub fn from_values(values: &[f32]) -> Self {
        Self::from_samples(SparklineGeometry::samples(values))
    }

    pub fn y_range(&self) -> Option<ChartRange> {
        Some(ChartRange::new(self.y_min?, self.y_max?))
    }

    pub fn value_text(&self) -> String {
        if self.finite_sample_count == 0 {
            return format!("0 of {} samples", self.sample_count);
        }

        format!(
            "{} of {} samples; y {} to {}",
            self.finite_sample_count,
            self.sample_count,
            format_chart_number(self.y_min.unwrap_or_default()),
            format_chart_number(self.y_max.unwrap_or_default())
        )
    }

    pub fn accessibility_summary(&self, title: impl Into<String>) -> AccessibilitySummary {
        let mut summary = AccessibilitySummary::new(title).item(
            "Samples",
            format!(
                "{} of {} finite",
                self.finite_sample_count, self.sample_count
            ),
        );
        if let (Some(min), Some(max)) = (self.x_min, self.x_max) {
            summary = summary.item(
                "X range",
                format!(
                    "{} to {}",
                    format_chart_number(min),
                    format_chart_number(max)
                ),
            );
        }
        if let (Some(min), Some(max)) = (self.y_min, self.y_max) {
            summary = summary.item(
                "Y range",
                format!(
                    "{} to {}",
                    format_chart_number(min),
                    format_chart_number(max)
                ),
            );
        }
        if let Some(sample) = self.first_sample {
            summary = summary.item("First sample", format_sample(sample));
        }
        if let Some(sample) = self.last_sample {
            summary = summary.item("Latest sample", format_sample(sample));
        }
        if let Some(sample) = self.min_y_sample {
            summary = summary.item("Minimum", format_sample(sample));
        }
        if let Some(sample) = self.max_y_sample {
            summary = summary.item("Maximum", format_sample(sample));
        }
        summary
    }

    pub fn accessibility_meta(&self, label: impl Into<String>) -> AccessibilityMeta {
        let label = label.into();
        AccessibilityMeta::new(AccessibilityRole::Image)
            .label(label.clone())
            .value(self.value_text())
            .summary(self.accessibility_summary(label))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChartViewport {
    pub rect: UiRect,
    pub x_range: ChartRange,
    pub y_range: ChartRange,
}

impl ChartViewport {
    pub const fn new(rect: UiRect, x_range: ChartRange, y_range: ChartRange) -> Self {
        Self {
            rect,
            x_range,
            y_range,
        }
    }

    pub fn map_x(self, value: f32) -> f32 {
        self.rect.x + self.x_range.normalized(value) * self.rect.width
    }

    pub fn map_y(self, value: f32) -> f32 {
        self.rect.y + self.rect.height - self.y_range.normalized(value) * self.rect.height
    }

    pub fn map_sample(self, sample: ChartSample) -> UiPoint {
        UiPoint::new(self.map_x(sample.x), self.map_y(sample.y))
    }

    pub fn contains_sample(self, sample: ChartSample) -> bool {
        sample.is_finite()
            && self.x_range.normalized(sample.x) >= 0.0
            && self.x_range.normalized(sample.x) <= 1.0
            && self.y_range.normalized(sample.y) >= 0.0
            && self.y_range.normalized(sample.y) <= 1.0
    }

    pub fn line_path(self, samples: impl IntoIterator<Item = ChartSample>) -> PaintPath {
        let mut path = PaintPath::new();
        let mut has_point = false;
        for sample in samples {
            if !sample.is_finite() {
                continue;
            }
            let point = self.map_sample(sample);
            path = if has_point {
                path.line_to(point)
            } else {
                has_point = true;
                path.move_to(point)
            };
        }
        path
    }

    pub fn filled_area_path(
        self,
        samples: impl IntoIterator<Item = ChartSample>,
        baseline: f32,
    ) -> PaintPath {
        let points: Vec<UiPoint> = samples
            .into_iter()
            .filter(|sample| sample.is_finite())
            .map(|sample| self.map_sample(sample))
            .collect();
        if points.is_empty() {
            return PaintPath::new();
        }

        let baseline = if baseline.is_finite() {
            baseline
        } else {
            self.y_range.min
        };
        let baseline_y = self.map_y(baseline);
        let first = points[0];
        let last = points[points.len() - 1];
        let mut path = PaintPath::new().move_to(UiPoint::new(first.x, baseline_y));
        for point in points {
            path = path.line_to(point);
        }
        path.line_to(UiPoint::new(last.x, baseline_y)).close()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SparklineGeometry {
    pub rect: UiRect,
    pub y_range: ChartRange,
}

impl SparklineGeometry {
    pub const fn new(rect: UiRect, y_range: ChartRange) -> Self {
        Self { rect, y_range }
    }

    pub fn auto(rect: UiRect, values: &[f32]) -> Self {
        Self {
            rect,
            y_range: ChartRange::from_values(values),
        }
    }

    pub fn viewport(self, len: usize) -> ChartViewport {
        let x_max = len.saturating_sub(1).max(1) as f32;
        ChartViewport::new(self.rect, ChartRange::new(0.0, x_max), self.y_range)
    }

    pub fn samples(values: &[f32]) -> impl Iterator<Item = ChartSample> + '_ {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| ChartSample::new(index as f32, *value))
    }

    pub fn line_path(self, values: &[f32]) -> PaintPath {
        self.viewport(values.len()).line_path(Self::samples(values))
    }

    pub fn filled_area_path(self, values: &[f32], baseline: f32) -> PaintPath {
        self.viewport(values.len())
            .filled_area_path(Self::samples(values), baseline)
    }

    pub fn data_summary(values: &[f32]) -> ChartDataSummary {
        ChartDataSummary::from_values(values)
    }

    pub fn accessibility_meta(label: impl Into<String>, values: &[f32]) -> AccessibilityMeta {
        Self::data_summary(values).accessibility_meta(label)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridCell {
    pub column: usize,
    pub row: usize,
}

impl GridCell {
    pub const fn new(column: usize, row: usize) -> Self {
        Self { column, row }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridCellRange {
    pub start_column: usize,
    pub end_column: usize,
    pub start_row: usize,
    pub end_row: usize,
}

impl GridCellRange {
    pub const fn new(
        start_column: usize,
        end_column: usize,
        start_row: usize,
        end_row: usize,
    ) -> Self {
        Self {
            start_column,
            end_column,
            start_row,
            end_row,
        }
    }

    pub fn columns(self) -> usize {
        self.end_column.saturating_sub(self.start_column)
    }

    pub fn rows(self) -> usize {
        self.end_row.saturating_sub(self.start_row)
    }

    pub fn len(self) -> usize {
        self.columns() * self.rows()
    }

    pub fn is_empty(self) -> bool {
        self.columns() == 0 || self.rows() == 0
    }

    pub fn contains(self, cell: GridCell) -> bool {
        cell.column >= self.start_column
            && cell.column < self.end_column
            && cell.row >= self.start_row
            && cell.row < self.end_row
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridMapGeometry {
    pub rect: UiRect,
    pub columns: usize,
    pub rows: usize,
    pub gap: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridMapSummary {
    pub columns: usize,
    pub rows: usize,
    pub total_cells: usize,
    pub visible_range: Option<GridCellRange>,
    pub active_cell: Option<GridCell>,
    pub selected_count: usize,
}

impl GridMapSummary {
    pub fn new(
        geometry: GridMapGeometry,
        visible_clip: Option<UiRect>,
        active_cell: Option<GridCell>,
        selected_count: usize,
    ) -> Self {
        let total_cells = geometry.columns.saturating_mul(geometry.rows);
        let visible_range = visible_clip.and_then(|clip| geometry.visible_cell_range(clip));
        let active_cell =
            active_cell.filter(|cell| cell.column < geometry.columns && cell.row < geometry.rows);
        Self {
            columns: geometry.columns,
            rows: geometry.rows,
            total_cells,
            visible_range,
            active_cell,
            selected_count: selected_count.min(total_cells),
        }
    }

    pub fn value_text(&self) -> String {
        let mut parts = vec![format!(
            "{} cells, {} columns by {} rows",
            self.total_cells, self.columns, self.rows
        )];
        if let Some(visible) = self.visible_range {
            parts.push(format!("{} visible", visible.len()));
        }
        if self.selected_count > 0 {
            parts.push(format!("{} selected", self.selected_count));
        }
        if let Some(active) = self.active_cell {
            parts.push(format!("active {}", format_grid_cell(active)));
        }
        parts.join("; ")
    }

    pub fn accessibility_summary(&self, title: impl Into<String>) -> AccessibilitySummary {
        let mut summary = AccessibilitySummary::new(title)
            .item("Cells", self.total_cells.to_string())
            .item("Columns", self.columns.to_string())
            .item("Rows", self.rows.to_string());
        if let Some(visible) = self.visible_range {
            summary = summary
                .item("Visible cells", visible.len().to_string())
                .item(
                    "Visible columns",
                    format_index_range(visible.start_column, visible.end_column),
                )
                .item(
                    "Visible rows",
                    format_index_range(visible.start_row, visible.end_row),
                );
        }
        if self.selected_count > 0 {
            summary = summary.item("Selected", self.selected_count.to_string());
        }
        if let Some(active) = self.active_cell {
            summary = summary.item("Active cell", format_grid_cell(active));
        }
        summary
    }

    pub fn accessibility_meta(&self, label: impl Into<String>) -> AccessibilityMeta {
        let label = label.into();
        AccessibilityMeta::new(AccessibilityRole::Grid)
            .label(label.clone())
            .value(self.value_text())
            .summary(self.accessibility_summary(label))
    }
}

impl GridMapGeometry {
    pub const fn new(rect: UiRect, columns: usize, rows: usize) -> Self {
        Self {
            rect,
            columns,
            rows,
            gap: 0.0,
        }
    }

    pub fn gap(mut self, gap: f32) -> Self {
        if gap.is_finite() {
            self.gap = gap.max(0.0);
        }
        self
    }

    pub fn cell_size(self) -> UiSize {
        if self.columns == 0 || self.rows == 0 {
            return UiSize::ZERO;
        }

        let x_gaps = self.gap * self.columns.saturating_sub(1) as f32;
        let y_gaps = self.gap * self.rows.saturating_sub(1) as f32;
        UiSize::new(
            (self.rect.width - x_gaps).max(0.0) / self.columns as f32,
            (self.rect.height - y_gaps).max(0.0) / self.rows as f32,
        )
    }

    pub fn cell_rect(self, cell: GridCell) -> Option<UiRect> {
        if cell.column >= self.columns || cell.row >= self.rows {
            return None;
        }

        let size = self.cell_size();
        if size.width <= f32::EPSILON || size.height <= f32::EPSILON {
            return None;
        }

        Some(UiRect::new(
            self.rect.x + cell.column as f32 * (size.width + self.gap),
            self.rect.y + cell.row as f32 * (size.height + self.gap),
            size.width,
            size.height,
        ))
    }

    pub fn hit_cell(self, point: UiPoint) -> Option<GridCell> {
        if self.columns == 0
            || self.rows == 0
            || point.x < self.rect.x
            || point.x >= self.rect.right()
            || point.y < self.rect.y
            || point.y >= self.rect.bottom()
        {
            return None;
        }

        let size = self.cell_size();
        if size.width <= f32::EPSILON || size.height <= f32::EPSILON {
            return None;
        }

        let column = ((point.x - self.rect.x) / (size.width + self.gap)).floor() as usize;
        let row = ((point.y - self.rect.y) / (size.height + self.gap)).floor() as usize;
        let cell = GridCell::new(column, row);
        self.cell_rect(cell)
            .filter(|rect| rect.contains_point(point))
            .map(|_| cell)
    }

    pub fn visible_cell_range(self, clip: UiRect) -> Option<GridCellRange> {
        let size = self.cell_size();
        if self.columns == 0
            || self.rows == 0
            || size.width <= f32::EPSILON
            || size.height <= f32::EPSILON
        {
            return None;
        }

        let (mut start_column, mut end_column) = visible_axis_indices(
            self.rect.x,
            size.width,
            self.gap,
            self.columns,
            clip.x,
            clip.right(),
        )?;
        let (mut start_row, mut end_row) = visible_axis_indices(
            self.rect.y,
            size.height,
            self.gap,
            self.rows,
            clip.y,
            clip.bottom(),
        )?;

        let column_intersects = |column: usize, start_row: usize, end_row: usize| {
            (start_row..end_row).any(|row| {
                self.cell_rect(GridCell::new(column, row))
                    .is_some_and(|rect| rect.intersects(clip))
            })
        };
        let row_intersects = |row: usize, start_column: usize, end_column: usize| {
            (start_column..end_column).any(|column| {
                self.cell_rect(GridCell::new(column, row))
                    .is_some_and(|rect| rect.intersects(clip))
            })
        };

        while start_column < end_column && !column_intersects(start_column, start_row, end_row) {
            start_column += 1;
        }
        while start_row < end_row && !row_intersects(start_row, start_column, end_column) {
            start_row += 1;
        }
        while end_column > start_column && !column_intersects(end_column - 1, start_row, end_row) {
            end_column -= 1;
        }
        while end_row > start_row && !row_intersects(end_row - 1, start_column, end_column) {
            end_row -= 1;
        }

        let range = GridCellRange::new(start_column, end_column, start_row, end_row);
        (!range.is_empty()).then_some(range)
    }

    pub fn visible_cells(self, clip: UiRect) -> Vec<GridCell> {
        let Some(range) = self.visible_cell_range(clip) else {
            return Vec::new();
        };

        let mut cells = Vec::with_capacity(range.len());
        for row in range.start_row..range.end_row {
            for column in range.start_column..range.end_column {
                let cell = GridCell::new(column, row);
                if self
                    .cell_rect(cell)
                    .is_some_and(|rect| rect.intersects(clip))
                {
                    cells.push(cell);
                }
            }
        }
        cells
    }

    pub fn data_summary(
        self,
        visible_clip: Option<UiRect>,
        active_cell: Option<GridCell>,
        selected_count: usize,
    ) -> GridMapSummary {
        GridMapSummary::new(self, visible_clip, active_cell, selected_count)
    }

    pub fn accessibility_meta(
        self,
        label: impl Into<String>,
        visible_clip: Option<UiRect>,
        active_cell: Option<GridCell>,
        selected_count: usize,
    ) -> AccessibilityMeta {
        self.data_summary(visible_clip, active_cell, selected_count)
            .accessibility_meta(label)
    }
}

fn format_sample(sample: ChartSample) -> String {
    format!(
        "x {}, y {}",
        format_chart_number(sample.x),
        format_chart_number(sample.y)
    )
}

fn format_chart_number(value: f32) -> String {
    if value.fract().abs() <= 0.0001 {
        format!("{value:.0}")
    } else {
        format!("{value:.3}")
    }
}

fn format_grid_cell(cell: GridCell) -> String {
    format!("column {}, row {}", cell.column, cell.row)
}

fn format_index_range(start: usize, end: usize) -> String {
    if start >= end {
        return "none".to_string();
    }
    if end == start + 1 {
        start.to_string()
    } else {
        format!("{start} to {}", end - 1)
    }
}

fn visible_axis_indices(
    origin: f32,
    cell_extent: f32,
    gap: f32,
    count: usize,
    clip_start: f32,
    clip_end: f32,
) -> Option<(usize, usize)> {
    if count == 0 || clip_end <= clip_start || clip_end <= origin {
        return None;
    }

    let total_extent = count as f32 * cell_extent + count.saturating_sub(1) as f32 * gap;
    if clip_start >= origin + total_extent {
        return None;
    }

    let pitch = cell_extent + gap;
    let start = ((clip_start - origin) / pitch).floor().max(0.0) as usize;
    let end = ((clip_end - origin) / pitch).floor().max(0.0) as usize + 1;
    Some((start.min(count), end.min(count)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PathVerb;

    #[test]
    fn chart_viewport_maps_line_and_area_paths() {
        let viewport = ChartViewport::new(
            UiRect::new(10.0, 20.0, 100.0, 50.0),
            ChartRange::new(0.0, 2.0),
            ChartRange::new(0.0, 10.0),
        );
        let samples = [
            ChartSample::new(0.0, 0.0),
            ChartSample::new(1.0, 10.0),
            ChartSample::new(2.0, 5.0),
        ];

        let line = viewport.line_path(samples);
        assert_eq!(line.verbs.len(), 3);
        assert_eq!(line.verbs[0], PathVerb::MoveTo(UiPoint::new(10.0, 70.0)));
        assert_eq!(line.verbs[1], PathVerb::LineTo(UiPoint::new(60.0, 20.0)));

        let area = viewport.filled_area_path(samples, 0.0);
        assert_eq!(
            area.verbs.first(),
            Some(&PathVerb::MoveTo(UiPoint::new(10.0, 70.0)))
        );
        assert_eq!(area.verbs.last(), Some(&PathVerb::Close));
    }

    #[test]
    fn sparkline_geometry_auto_ranges_values_and_skips_invalid_samples() {
        let values = [2.0, 4.0, f32::NAN, 6.0];
        let geometry = SparklineGeometry::auto(UiRect::new(0.0, 0.0, 90.0, 30.0), &values);
        let path = geometry.line_path(&values);

        assert_eq!(geometry.y_range, ChartRange::new(2.0, 6.0));
        assert_eq!(path.verbs.len(), 3);
        assert_eq!(path.verbs[0], PathVerb::MoveTo(UiPoint::new(0.0, 30.0)));
        assert_eq!(path.verbs[2], PathVerb::LineTo(UiPoint::new(90.0, 0.0)));
    }

    #[test]
    fn chart_data_summary_exports_accessibility_metadata() {
        let samples = [
            ChartSample::new(0.0, 2.0),
            ChartSample::new(1.0, f32::NAN),
            ChartSample::new(2.0, 8.0),
            ChartSample::new(3.0, 4.5),
        ];
        let summary = ChartDataSummary::from_samples(samples);

        assert_eq!(summary.sample_count, 4);
        assert_eq!(summary.finite_sample_count, 3);
        assert_eq!(summary.x_min, Some(0.0));
        assert_eq!(summary.x_max, Some(3.0));
        assert_eq!(summary.y_min, Some(2.0));
        assert_eq!(summary.y_max, Some(8.0));
        assert_eq!(summary.min_y_sample, Some(ChartSample::new(0.0, 2.0)));
        assert_eq!(summary.max_y_sample, Some(ChartSample::new(2.0, 8.0)));
        assert_eq!(summary.value_text(), "3 of 4 samples; y 2 to 8");

        let accessibility = summary.accessibility_meta("Yield trend");
        assert_eq!(accessibility.role, crate::AccessibilityRole::Image);
        assert_eq!(accessibility.label.as_deref(), Some("Yield trend"));
        assert_eq!(
            accessibility.value.as_deref(),
            Some("3 of 4 samples; y 2 to 8")
        );
        let screen_reader_text = accessibility.summary.unwrap().screen_reader_text();
        assert!(screen_reader_text.contains("Samples: 3 of 4 finite"));
        assert!(screen_reader_text.contains("Latest sample: x 3, y 4.500"));

        let sparkline_accessibility =
            SparklineGeometry::accessibility_meta("CPU sparkline", &[1.0, 2.0, 3.0]);
        assert_eq!(
            sparkline_accessibility.value.as_deref(),
            Some("3 of 3 samples; y 1 to 3")
        );
    }

    #[test]
    fn grid_map_geometry_maps_cells_hit_testing_and_visible_cells() {
        let geometry = GridMapGeometry::new(UiRect::new(0.0, 0.0, 100.0, 50.0), 4, 2).gap(2.0);
        let cell = GridCell::new(1, 1);

        assert_eq!(
            geometry.cell_rect(cell),
            Some(UiRect::new(25.5, 26.0, 23.5, 24.0))
        );
        assert_eq!(geometry.hit_cell(UiPoint::new(26.0, 27.0)), Some(cell));
        assert_eq!(geometry.hit_cell(UiPoint::new(50.0, 25.0)), None);

        assert_eq!(
            geometry.visible_cell_range(UiRect::new(0.0, 0.0, 52.0, 25.0)),
            Some(GridCellRange::new(0, 3, 0, 1))
        );
        let visible = geometry.visible_cells(UiRect::new(0.0, 0.0, 52.0, 25.0));
        assert_eq!(
            visible,
            vec![
                GridCell::new(0, 0),
                GridCell::new(1, 0),
                GridCell::new(2, 0)
            ]
        );
        assert_eq!(
            geometry.visible_cell_range(UiRect::new(23.5, 24.0, 2.0, 2.0)),
            None
        );
    }

    #[test]
    fn grid_map_summary_exports_accessibility_metadata() {
        let geometry = GridMapGeometry::new(UiRect::new(0.0, 0.0, 100.0, 50.0), 4, 2).gap(2.0);
        let summary = geometry.data_summary(
            Some(UiRect::new(0.0, 0.0, 52.0, 25.0)),
            Some(GridCell::new(2, 0)),
            3,
        );

        assert_eq!(summary.total_cells, 8);
        assert_eq!(summary.visible_range, Some(GridCellRange::new(0, 3, 0, 1)));
        assert_eq!(summary.active_cell, Some(GridCell::new(2, 0)));
        assert_eq!(
            summary.value_text(),
            "8 cells, 4 columns by 2 rows; 3 visible; 3 selected; active column 2, row 0"
        );

        let accessibility = summary.accessibility_meta("Wafer map");
        assert_eq!(accessibility.role, crate::AccessibilityRole::Grid);
        assert_eq!(accessibility.label.as_deref(), Some("Wafer map"));
        assert_eq!(
            accessibility.value.as_deref(),
            Some("8 cells, 4 columns by 2 rows; 3 visible; 3 selected; active column 2, row 0")
        );
        let screen_reader_text = accessibility.summary.unwrap().screen_reader_text();
        assert!(screen_reader_text.contains("Visible columns: 0 to 2"));
        assert!(screen_reader_text.contains("Active cell: column 2, row 0"));

        let clipped = geometry.accessibility_meta(
            "Empty viewport",
            Some(UiRect::new(500.0, 500.0, 10.0, 10.0)),
            Some(GridCell::new(9, 9)),
            99,
        );
        assert_eq!(
            clipped.value.as_deref(),
            Some("8 cells, 4 columns by 2 rows; 8 selected")
        );
    }
}
