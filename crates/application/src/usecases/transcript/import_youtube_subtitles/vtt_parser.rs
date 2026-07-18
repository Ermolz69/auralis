use domain::transcript::{Transcript, TranscriptSegment, TranscriptSegmentId};

use crate::error::ApplicationError;

pub fn parse_vtt(content: &str, language: &str) -> Result<Transcript, ApplicationError> {
    let mut segments = Vec::new();
    let mut current_start = None;
    let mut current_end = None;
    let mut current_text = String::new();
    let mut index = 0;

    let flush = |start: &mut Option<u64>,
                 end: &mut Option<u64>,
                 text: &mut String,
                 segments: &mut Vec<TranscriptSegment>,
                 index: &mut u32| {
        if let (Some(s), Some(e)) = (*start, *end) {
            let t = text.trim();
            if !t.is_empty() {
                segments.push(TranscriptSegment {
                    id: TranscriptSegmentId::new(),
                    index: *index,
                    start_ms: s,
                    end_ms: e,
                    source_text: t.to_string(),
                    translated_text: None,
                    adapted_text: None,
                    speaker: None,
                    confidence: None,
                });
                *index += 1;
            }
        }
        *start = None;
        *end = None;
        text.clear();
    };

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            flush(
                &mut current_start,
                &mut current_end,
                &mut current_text,
                &mut segments,
                &mut index,
            );
            continue;
        }

        if line == "WEBVTT" {
            continue;
        }

        if line.contains("-->") {
            // Might have been preceded by an identifier, flush it
            current_text.clear();

            let parts: Vec<&str> = line.split("-->").collect();
            if parts.len() == 2 {
                let start_str = parts[0].split_whitespace().last().unwrap_or("");
                let end_str = parts[1].split_whitespace().next().unwrap_or("");
                if let (Some(s), Some(e)) = (parse_vtt_time(start_str), parse_vtt_time(end_str)) {
                    current_start = Some(s);
                    current_end = Some(e);
                }
            }
        } else if current_start.is_some() {
            // this is text
            // remove vtt tags like <c.color>...</c> or <v Speaker>...</v>
            let clean = remove_vtt_tags(line);
            if !current_text.is_empty() {
                current_text.push(' ');
            }
            current_text.push_str(&clean);
        }
    }

    flush(
        &mut current_start,
        &mut current_end,
        &mut current_text,
        &mut segments,
        &mut index,
    );

    Ok(Transcript {
        language: language.to_string(),
        segments,
    })
}

fn parse_vtt_time(time_str: &str) -> Option<u64> {
    let parts: Vec<&str> = time_str.split('.').collect();
    if parts.len() != 2 {
        return None;
    }
    let ms: u64 = parts[1].parse().ok()?;
    let hms: Vec<&str> = parts[0].split(':').collect();

    let (h, m, s) = match hms.len() {
        3 => (
            hms[0].parse::<u64>().ok()?,
            hms[1].parse::<u64>().ok()?,
            hms[2].parse::<u64>().ok()?,
        ),
        2 => (0, hms[0].parse::<u64>().ok()?, hms[1].parse::<u64>().ok()?),
        _ => return None,
    };

    Some(h * 3600000 + m * 60000 + s * 1000 + ms)
}

fn remove_vtt_tags(text: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for c in text.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }
    result
}

pub fn select_best_subtitle_track(
    subtitles: &[domain::media::SubtitleTrack],
    preferred_languages: &[String],
    allow_auto_generated: bool,
) -> Result<domain::media::SubtitleTrack, ApplicationError> {
    let is_vtt = |t: &domain::media::SubtitleTrack| t.format.as_deref() == Some("vtt");

    let mut best_track = None;
    for lang in preferred_languages {
        if let Some(track) = subtitles
            .iter()
            .find(|t| &t.language == lang && !t.is_auto_generated && is_vtt(t))
        {
            best_track = Some(track.clone());
            break;
        }
    }

    if best_track.is_none() && allow_auto_generated {
        for lang in preferred_languages {
            if let Some(track) = subtitles
                .iter()
                .find(|t| &t.language == lang && t.is_auto_generated && is_vtt(t))
            {
                best_track = Some(track.clone());
                break;
            }
        }
    }

    if best_track.is_none() {
        best_track = subtitles
            .iter()
            .find(|t| !t.is_auto_generated && is_vtt(t))
            .cloned();
        if best_track.is_none() && allow_auto_generated {
            best_track = subtitles.iter().find(|t| is_vtt(t)).cloned();
        }
    }

    best_track.ok_or_else(|| ApplicationError::InvalidOperation {
        message: "No suitable subtitles found".to_string(),
    })
}
