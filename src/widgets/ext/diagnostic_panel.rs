//! Shared options and node handles for diagnostic property and trace panels.
//!
//! Panels with the same layout contract use the same options type. The trace
//! being displayed determines the rows; it does not require another UI type.

use crate::{LayoutStyle, UiNodeId};
use taffy::prelude::{Dimension, Display, FlexDirection, Size as TaffySize, Style};

#[derive(Debug, Clone)]
pub struct TimelinePanelOptions {
    pub layout: LayoutStyle,
    pub label_width: f32,
    pub row_height: f32,
    pub max_frame_rows: usize,
    pub action_prefix: Option<String>,
}

impl Default for TimelinePanelOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
            label_width: 132.0,
            row_height: 24.0,
            max_frame_rows: 8,
            action_prefix: None,
        }
    }
}

impl TimelinePanelOptions {
    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct SourcePanelOptions {
    pub layout: LayoutStyle,
    pub label_width: f32,
    pub row_height: f32,
    pub max_source_rows: usize,
    pub action_prefix: Option<String>,
}

impl Default for SourcePanelOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
            label_width: 132.0,
            row_height: 24.0,
            max_source_rows: 8,
            action_prefix: None,
        }
    }
}

impl SourcePanelOptions {
    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct RecordPanelOptions {
    pub layout: LayoutStyle,
    pub label_width: f32,
    pub row_height: f32,
    pub max_record_rows: usize,
    pub action_prefix: Option<String>,
}

impl Default for RecordPanelOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
            label_width: 132.0,
            row_height: 24.0,
            max_record_rows: 8,
            action_prefix: None,
        }
    }
}

impl RecordPanelOptions {
    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct CandidatePanelOptions {
    pub layout: LayoutStyle,
    pub label_width: f32,
    pub row_height: f32,
    pub max_candidate_rows: usize,
    pub action_prefix: Option<String>,
}

impl Default for CandidatePanelOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
            label_width: 132.0,
            row_height: 24.0,
            max_candidate_rows: 8,
            action_prefix: None,
        }
    }
}

impl CandidatePanelOptions {
    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct TextRowsPanelOptions {
    pub layout: LayoutStyle,
    pub label_width: f32,
    pub row_height: f32,
    pub max_text_rows: usize,
    pub action_prefix: Option<String>,
}

impl Default for TextRowsPanelOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
            label_width: 132.0,
            row_height: 24.0,
            max_text_rows: 8,
            action_prefix: None,
        }
    }
}

impl TextRowsPanelOptions {
    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct PropertyPanelOptions {
    pub layout: LayoutStyle,
    pub label_width: f32,
    pub row_height: f32,
    pub action_prefix: Option<String>,
}

impl Default for PropertyPanelOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
            label_width: 132.0,
            row_height: 24.0,
            action_prefix: None,
        }
    }
}

impl PropertyPanelOptions {
    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct NodeRowsPanelOptions {
    pub layout: LayoutStyle,
    pub label_width: f32,
    pub row_height: f32,
    pub max_node_rows: usize,
    pub action_prefix: Option<String>,
}

impl Default for NodeRowsPanelOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
            label_width: 132.0,
            row_height: 24.0,
            max_node_rows: 8,
            action_prefix: None,
        }
    }
}

impl NodeRowsPanelOptions {
    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct NodeTimelinePanelOptions {
    pub layout: LayoutStyle,
    pub label_width: f32,
    pub row_height: f32,
    pub max_node_rows: usize,
    pub max_frame_rows: usize,
    pub action_prefix: Option<String>,
}

impl Default for NodeTimelinePanelOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
            label_width: 132.0,
            row_height: 24.0,
            max_node_rows: 5,
            max_frame_rows: 4,
            action_prefix: None,
        }
    }
}

impl NodeTimelinePanelOptions {
    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct ChangePanelOptions {
    pub layout: LayoutStyle,
    pub label_width: f32,
    pub row_height: f32,
    pub max_change_rows: usize,
    pub action_prefix: Option<String>,
}

impl Default for ChangePanelOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
            label_width: 132.0,
            row_height: 24.0,
            max_change_rows: 8,
            action_prefix: None,
        }
    }
}

impl ChangePanelOptions {
    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct IssuePanelOptions {
    pub layout: LayoutStyle,
    pub label_width: f32,
    pub row_height: f32,
    pub max_issue_rows: usize,
    pub action_prefix: Option<String>,
}

impl Default for IssuePanelOptions {
    fn default() -> Self {
        Self {
            layout: LayoutStyle::from_taffy_style(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: TaffySize {
                    width: Dimension::percent(1.0),
                    height: Dimension::auto(),
                },
                ..Default::default()
            }),
            label_width: 132.0,
            row_height: 24.0,
            max_issue_rows: 8,
            action_prefix: None,
        }
    }
}

impl IssuePanelOptions {
    pub fn with_action_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.action_prefix = Some(prefix.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticPanelNodes {
    pub root: UiNodeId,
    pub rows: UiNodeId,
}
