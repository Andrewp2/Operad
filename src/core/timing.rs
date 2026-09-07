//! Frame measurements independent of diagnostics and test harnesses.

use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameTimingSection {
    pub name: String,
    pub duration: Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FrameTimingSectionSummary {
    pub name: String,
    pub sample_count: usize,
    pub total: Duration,
    pub average: Duration,
    pub max: Duration,
    pub total_fraction: f64,
}

impl FrameTimingSectionSummary {
    pub(crate) fn new(
        name: impl Into<String>,
        sample_count: usize,
        total: Duration,
        max: Duration,
        all_sections_total: Duration,
    ) -> Self {
        let average = if sample_count == 0 {
            Duration::ZERO
        } else {
            Duration::from_secs_f64(total.as_secs_f64() / sample_count as f64)
        };
        let total_fraction = if all_sections_total.is_zero() {
            0.0
        } else {
            total.as_secs_f64() / all_sections_total.as_secs_f64()
        };
        Self {
            name: name.into(),
            sample_count,
            total,
            average,
            max,
            total_fraction,
        }
    }

    pub fn percent_of_total(&self) -> f64 {
        self.total_fraction * 100.0
    }

    pub fn diagnostic_summary(&self) -> String {
        format!(
            "{}: samples={}, total={:?}, average={:?}, max={:?}, share={:.1}%",
            self.name,
            self.sample_count,
            self.total,
            self.average,
            self.max,
            self.percent_of_total()
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrameTiming {
    pub sections: Vec<FrameTimingSection>,
}

impl FrameTiming {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn section(mut self, name: impl Into<String>, duration: Duration) -> Self {
        self.sections.push(FrameTimingSection {
            name: name.into(),
            duration,
        });
        self
    }

    pub fn total(&self) -> Duration {
        self.sections.iter().map(|section| section.duration).sum()
    }

    pub fn duration(&self, name: &str) -> Option<Duration> {
        self.sections
            .iter()
            .find(|section| section.name == name)
            .map(|section| section.duration)
    }

    pub fn section_fraction(&self, name: &str) -> Option<f64> {
        let total = self.total();
        if total.is_zero() {
            return self.duration(name).map(|_| 0.0);
        }
        self.duration(name)
            .map(|duration| duration.as_secs_f64() / total.as_secs_f64())
    }

    pub fn dominant_section(&self) -> Option<FrameTimingSectionSummary> {
        let total = self.total();
        self.sections
            .iter()
            .max_by_key(|section| section.duration)
            .map(|section| {
                FrameTimingSectionSummary::new(
                    section.name.clone(),
                    1,
                    section.duration,
                    section.duration,
                    total,
                )
            })
    }

    pub fn within_budget(&self, budget: Duration) -> bool {
        self.total() <= budget
    }
}
