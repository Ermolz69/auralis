use super::{conflict, hash, invalid};
use crate::error::ApplicationError;
use ports::{speech_run::SpeechRunIntent, speech_workflow::SpeechReview};
use serde_json::{Value, json};

pub(super) fn stored_plan(intent: &SpeechRunIntent) -> Result<Value, ApplicationError> {
    if hash(intent.plan_json.as_bytes()) != intent.plan_sha256 {
        return Err(conflict("Saved speech plan bytes changed; prepare a new run").into());
    }
    let plan: Value =
        serde_json::from_str(&intent.plan_json).map_err(|error| invalid(error.to_string()))?;
    if plan["version"] != 1
        || plan["plan_id"] != intent.run_id.to_string()
        || plan["project_id"] != intent.project_id.to_string()
        || plan["profile"]["profile_id"] != intent.profile_id
        || plan["profile"]["profile_sha256"] != intent.profile_sha256
        || plan["utterances"].as_array().map(Vec::len) != Some(intent.expected_utterances as usize)
    {
        return Err(conflict("Speech plan identity or ordered coverage changed").into());
    }
    Ok(plan)
}

pub(super) fn composed_plan(
    intent: &SpeechRunIntent,
    draft: &Value,
) -> Result<(), ApplicationError> {
    let mut plan = stored_plan(intent)?;
    plan.as_object_mut()
        .ok_or_else(|| invalid("Composed speech plan is not an object"))?
        .remove("plan_digest");
    if &plan != draft {
        return Err(conflict(
            "Composed speech plan changed the reviewed text, timing or voice selections",
        )
        .into());
    }
    Ok(())
}

pub(super) fn reviewed_plan(
    intent: &SpeechRunIntent,
    plan: &Value,
    review: &SpeechReview,
) -> Result<(), ApplicationError> {
    let source = &review.source;
    let continuous = source.kind == "text";
    let expected_count = if continuous { 1 } else { source.cues.len() };
    if review.project_id != intent.project_id
        || plan["review"]["decision_id"] != review.decision_id.to_string()
        || plan["review"]["status"] != "approved"
        || plan["source"]["source_sha256"] != source.sha256
        || plan["source"]["text_revision"] != source.revision
        || plan["source"]["reviewed_text"]
            != if continuous {
                json!(source.text)
            } else {
                Value::Null
            }
        || plan["mode"]
            != if continuous {
                "continuous"
            } else {
                "subtitle_utterances"
            }
        || intent.expected_utterances as usize != expected_count
    {
        return Err(conflict("Speech plan differs from its exact approved source version").into());
    }
    let utterances = plan["utterances"]
        .as_array()
        .ok_or_else(|| invalid("Speech plan has no ordered utterances"))?;
    for (index, utterance) in utterances.iter().enumerate() {
        let matches = if continuous {
            utterance["text"] == source.text
                && utterance["cue_id"].is_null()
                && utterance["start_ms"].is_null()
                && utterance["end_ms"].is_null()
                && utterance["text_range"] == json!({"start":0,"end":source.text.chars().count()})
        } else {
            let cue = &source.cues[index];
            utterance["cue_id"] == cue.id
                && utterance["text"] == cue.text
                && utterance["start_ms"] == cue.start_ms
                && utterance["end_ms"] == cue.end_ms
        };
        if !matches {
            return Err(conflict(&format!(
                "Speech utterance {} differs from the approved text, order or timing; review a new run",
                index + 1,
            )).into());
        }
    }
    Ok(())
}
