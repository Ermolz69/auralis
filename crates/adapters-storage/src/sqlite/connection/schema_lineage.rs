use super::non_final_schema_error;
use ports::error::PortError;

pub(super) struct SchemaLineage {
    pub library: bool,
    pub media: bool,
    pub speech_marker: i64,
    pub audio_marker: i64,
}

pub(super) fn inspect(marker: i64, tables: &[String]) -> Result<SchemaLineage, PortError> {
    let exists = |name: &str| tables.iter().any(|table| table == name);
    let library = exists("workspace_library");
    let media_count = [
        "media_project_receipts",
        "media_copy_operations",
        "media_run_sources",
    ]
    .into_iter()
    .filter(|name| exists(name))
    .count();
    let audio_count = [
        "audio_assembly_intents",
        "audio_assembly_attempts",
        "audio_assembly_publications",
        "audio_assembly_discarded_artifacts",
        "audio_assembly_artifacts",
    ]
    .into_iter()
    .filter(|name| exists(name))
    .count();
    let media = media_count == 3;
    let audio = audio_count == 5;
    let valid = matches!(media_count, 0 | 3)
        && matches!(audio_count, 0 | 5)
        && match marker {
            1..=17 => !library && !media && !audio,
            18 => !media && !audio,
            19 => (audio && !library && !media) || (!audio && library && media),
            20 => audio && !library && !media,
            21 => audio && library,
            22 => audio && library && media,
            _ => false,
        };
    if !valid {
        return Err(non_final_schema_error("schema lineage"));
    }
    let legacy_library = library && marker <= 19;
    Ok(SchemaLineage {
        library,
        media,
        speech_marker: if legacy_library { 17 } else { marker },
        audio_marker: if legacy_library { 18 } else { marker },
    })
}

#[cfg(test)]
#[path = "schema_lineage/tests.rs"]
mod tests;
