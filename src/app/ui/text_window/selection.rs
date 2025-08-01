use crate::app::buffer::BufferPosition;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Selection {
    fixed_point: BufferPosition,
    moving_point: BufferPosition,
}

impl Selection {
    pub fn from_single(bufpos: &BufferPosition) -> Self {
        Selection {
            fixed_point: bufpos.clone(),
            moving_point: bufpos.clone(),
        }
    }

    pub fn from_pair(fixed: &BufferPosition, moving: &BufferPosition) -> Self {
        Selection {
            fixed_point: fixed.clone(),
            moving_point: moving.clone(),
        }
    }
}
