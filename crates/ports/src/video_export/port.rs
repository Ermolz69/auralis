use super::{StagedVideoExport, VideoExportRequest, VideoInspection, VideoSourceInput};
use crate::{PortError, cancellation::CancellationToken};
use async_trait::async_trait;

#[async_trait]
pub trait VideoExportPort: Send + Sync {
    async fn inspect(
        &self,
        source: VideoSourceInput,
        cancel: CancellationToken,
    ) -> Result<VideoInspection, PortError>;
    async fn export(
        &self,
        request: VideoExportRequest,
        cancel: CancellationToken,
    ) -> Result<StagedVideoExport, PortError>;
}
