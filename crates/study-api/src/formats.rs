//! import-export.feature: spreadsheets, JSON, Markdown checklists and Moodle questions
//! (R174 to R176).

use study_store::portable::Snapshot;

use crate::transfer::ExportFormat;
use crate::{ApiError, ApiResult};

/// A file in one of the plain formats, for `snapshot`.
pub fn export(_snapshot: &Snapshot, format: ExportFormat) -> ApiResult<Vec<u8>> {
    Err(ApiError::new(format!("exporting as {format:?} isn't ready yet")))
}
