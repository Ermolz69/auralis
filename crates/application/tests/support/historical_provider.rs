use auralis_translation::{
    ProviderError, ProviderResponse, TargetSegment, TranslationBatch, TranslationProvider,
};

pub struct HistoricalProvider;

impl TranslationProvider for HistoricalProvider {
    fn translate(&self, batch: &TranslationBatch) -> Result<ProviderResponse, ProviderError> {
        Ok(ProviderResponse {
            schema_version: batch.schema_version(),
            translations: batch
                .targets()
                .iter()
                .map(|segment| TargetSegment {
                    id: segment.id(),
                    lines: vec![format!("Исходный перевод {}.", segment.id())],
                })
                .collect(),
        })
    }
}
