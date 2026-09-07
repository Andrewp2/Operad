//! Invalidation shared by documents, hosts, and renderers.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DirtyFlags {
    pub layout: bool,
    pub paint: bool,
    pub input: bool,
    pub theme: bool,
    pub text_measurement: bool,
}

impl DirtyFlags {
    pub const NONE: Self = Self {
        layout: false,
        paint: false,
        input: false,
        theme: false,
        text_measurement: false,
    };

    pub const ALL: Self = Self {
        layout: true,
        paint: true,
        input: true,
        theme: true,
        text_measurement: true,
    };

    pub const fn any(self) -> bool {
        self.layout || self.paint || self.input || self.theme || self.text_measurement
    }

    pub const fn union(self, other: Self) -> Self {
        Self {
            layout: self.layout || other.layout,
            paint: self.paint || other.paint,
            input: self.input || other.input,
            theme: self.theme || other.theme,
            text_measurement: self.text_measurement || other.text_measurement,
        }
    }

    pub fn clear(&mut self) {
        *self = Self::NONE;
    }
}
