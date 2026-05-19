use thiserror::Error;
use uuid::Uuid;

use crate::types::{Finding, FindingSource, SensitiveType};

#[derive(Debug, Error)]
pub enum ManualFindingError {
    #[error("selection is empty")]
    Empty,
    #[error("selection range is invalid")]
    InvalidRange,
}

pub fn create_manual_finding(
    text: &str,
    start: usize,
    end: usize,
    r#type: SensitiveType,
) -> Result<Finding, ManualFindingError> {
    if start >= end {
        return Err(ManualFindingError::Empty);
    }
    let Some(slice) = text.get(start..end) else {
        return Err(ManualFindingError::InvalidRange);
    };
    Ok(Finding {
        id: Uuid::new_v4().to_string(),
        r#type,
        start,
        end,
        text: slice.to_string(),
        source: FindingSource::Manual,
        confidence: Some(1.0),
        needs_review: Some(false),
    })
}
