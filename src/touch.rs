use crate::Pt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TouchPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

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

#[derive(Debug, Clone, Copy)]
pub struct TouchInfo {
    pub id: u64,
    pub position: (Pt, Pt),
    pub phase: TouchPhase,
}
