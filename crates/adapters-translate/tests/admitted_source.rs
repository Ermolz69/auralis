use adapters_translate::FormatInspector;
use auralis_translation::SourceHash;
use ports::translation_inspection::{TranslationInspectorPort, TranslationSourceFormat};
use serde_json::{Value, json};
use std::{error::Error, io::Write, path::Path};

fn hash(bytes: &[u8]) -> String {
    SourceHash::digest(bytes).to_string()
}

#[tokio::test]
#[ignore = "explicit real-source specification and new evidence output required; no model loading"]
async fn frozen_real_source_is_inspected_without_modifying_its_bytes() -> Result<(), Box<dyn Error>>
{
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let spec_path = std::env::var("AURALIS_REVOICE_SOURCE_SPEC")?;
    let spec_bytes = std::fs::read(spec_path)?;
    let spec: Value = serde_json::from_slice(&spec_bytes)?;
    let source = root.join(spec["source_path"].as_str().ok_or("source path missing")?);
    let output = root.join(spec["output"].as_str().ok_or("output missing")?);
    if output.exists() {
        return Err("Retained inspection output already exists".into());
    }
    if std::fs::metadata(&source)?.len()
        > spec["max_source_bytes"]
            .as_u64()
            .ok_or("missing byte ceiling")?
    {
        return Err("Source exceeds frozen byte ceiling".into());
    }
    let original = std::fs::read(&source)?;
    if hash(&original)
        != spec["source_sha256"]
            .as_str()
            .ok_or("missing source hash")?
        || original.len() as u64
            != spec["source_bytes"]
                .as_u64()
                .ok_or("missing source bytes")?
    {
        return Err("Source identity differs before inspection".into());
    }
    let clock = std::time::Instant::now();
    let format = match spec["format"].as_str() {
        Some("srt") => TranslationSourceFormat::Srt,
        Some("vtt") => TranslationSourceFormat::Vtt,
        _ => return Err("Unsupported declared source format".into()),
    };
    let inspected = FormatInspector.inspect(&source, format).await;
    let mut receipt = json!({"version":1,"case_id":spec["case_id"],"source_group":spec["source_group"],"source_url":spec["source_url"],"source_sha256":hash(&original),"source_bytes":original.len(),"spec_sha256":hash(&spec_bytes),"test_source_sha256":hash(include_bytes!("admitted_source.rs")),"scope":spec["scope"],"status":"failed"});
    let outcome = (|| -> Result<(), Box<dyn Error>> {
        let parsed = inspected?;
        if parsed.segments.len() as u64
            != spec["expected_cues"]
                .as_u64()
                .ok_or("missing expected cue count")?
            || parsed.source_sha256 != hash(&original)
        {
            return Err("Strict parser coverage or source identity differs".into());
        }
        let mut text_bytes = 0;
        let mut mapping = Vec::new();
        for (index, cue) in parsed.segments.iter().enumerate() {
            if cue.id as usize != index + 1
                || cue.ordinal as usize != index
                || cue.start_ms >= cue.end_ms
            {
                return Err("Cue identity/order/window failed".into());
            }
            let text = cue.source_lines.join("\n");
            text_bytes += text.len();
            mapping.push((
                cue.id,
                cue.ordinal,
                cue.start_ms,
                cue.end_ms,
                hash(text.as_bytes()),
            ));
        }
        if std::fs::read(&source)? != original {
            return Err("Original source changed".into());
        }
        receipt["parsed"] = json!({"cues":parsed.segments.len(),"parser_version":parsed.parser_version,"text_utf8_bytes":text_bytes,"ordered_mapping_sha256":hash(&serde_json::to_vec(&mapping)?),"first_start_ms":parsed.segments.first().map(|cue|cue.start_ms),"last_end_ms":parsed.segments.last().map(|cue|cue.end_ms),"original_unchanged":true});
        receipt["status"] = json!("strict_source_inspected");
        Ok(())
    })();
    if let Err(ref error) = outcome {
        receipt["error"] = json!(error.to_string());
    }
    receipt["elapsed_ms"] = json!(clock.elapsed().as_millis());
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    file.write_all(serde_json::to_string_pretty(&receipt)?.as_bytes())?;
    println!("{receipt}");
    outcome
}
