use crate::Pt;

/// Represents the current state of a touch interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TouchPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

#[cfg(not(target_os = "android"))]
impl TouchPhase {
    pub(crate) fn from_winit(phase: winit::event::TouchPhase) -> Self {
        match phase {
            winit::event::TouchPhase::Started => TouchPhase::Started,
            winit::event::TouchPhase::Moved => TouchPhase::Moved,
            winit::event::TouchPhase::Ended => TouchPhase::Ended,
            winit::event::TouchPhase::Cancelled => TouchPhase::Cancelled,
        }
    }
}

/// Information about a single active touch point.
#[derive(Debug, Clone, Copy)]
pub struct TouchInfo {
    /// Unique identifier for the touch point.
    pub id: u64,
    /// Logical screen position of the touch.
    pub position: (Pt, Pt),
    /// Current phase of the touch life cycle.
    pub phase: TouchPhase,
}
