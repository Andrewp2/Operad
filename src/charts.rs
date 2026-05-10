//! Renderer-neutral chart, sparkline, and dense grid-map geometry helpers.
//!
//! These helpers turn app-owned numeric snapshots into stable paint geometry.
//! Operad does not own chart semantics, wafer-map meaning, process metrics, or
//! DAW/editor data; it only provides predictable coordinate mapping and hit
//! geometry that backends and tests can share.

use crate::{PaintPath, UiPoint, UiRect, UiSize};

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
}
