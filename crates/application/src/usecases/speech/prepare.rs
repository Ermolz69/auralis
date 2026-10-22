use super::*;
use crate::error::ApplicationError;
use domain::project::ProjectId;
use ports::speech_run::{MAX_SPEECH_PLAN_BYTES, SpeechRunIntent};
use serde::Deserialize;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeechAssignment {
    pub scope: String,
    pub target: String,
    pub voice_id: String,
    pub reference_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrepareSpeechRequest {
    pub decision_id: Uuid,
    pub expected_source_sha256: String,
    pub profile_id: String,
    pub voice_id: String,
    pub reference_id: String,
    pub assignments: Vec<SpeechAssignment>,
}

impl SpeechUseCases {
    pub async fn prepare(
        &self,
        project: ProjectId,
        request: PrepareSpeechRequest,
    ) -> Result<SpeechRunView, ApplicationError> {
        let review = self
            .store
            .get_review(request.decision_id)
            .await?
            .ok_or_else(|| invalid("Reviewed text snapshot is missing"))?;
        if review.project_id != project || review.source.sha256 != request.expected_source_sha256 {
            return Err(conflict("Reviewed text does not belong to this input").into());
        }
        self.check_source(&review).await?;
        self.limits()?
            .source(&review.source.text, review.source.cues.len())?;
        let catalog = self
            .catalog
            .as_ref()
            .ok_or_else(|| {
                invalid(
                    self.runtime_error
                        .clone()
                        .unwrap_or_else(|| "TTS runtime unavailable".into()),
                )
            })?
            .snapshot(&project)
            .await?;
        let profile = catalog
            .profiles
            .iter()
            .find(|p| p.profile_id == request.profile_id)
            .ok_or_else(|| invalid("Unknown speech profile"))?;
        let cue_ids: HashSet<_> = review
            .source
            .cues
            .iter()
            .map(|cue| cue.id.as_str())
            .collect();
        let roles: HashSet<_> = review
            .source
            .cues
            .iter()
            .map(|cue| cue.role.as_str())
            .collect();
        let mut targets = HashMap::new();
        for a in &request.assignments {
            let exists = match a.scope.as_str() {
                "cue" => cue_ids.contains(a.target.as_str()),
                "role" => roles.contains(a.target.as_str()),
                _ => false,
            };
            if !exists
                || targets
                    .insert((a.scope.as_str(), a.target.as_str()), a)
                    .is_some()
            {
                return Err(invalid("Voice assignment has an unknown or repeated target").into());
            }
        }
        let run_id = Uuid::new_v4();
        let mut cast = Vec::new();
        let mut utterances = Vec::new();
        let mut contract_bindings = Vec::new();
        contract_bindings.push(ports::puza_contract::PuzaVoiceBinding {
            role_id: "narrator".into(),
            display_name: "Рассказчик".into(),
            voice_id: request.voice_id.clone(),
            reference_id: request.reference_id.clone(),
            engine_id: profile.engine_id.clone(),
        });
        let inputs = if review.source.kind == "text" {
            vec![(
                "text-1".into(),
                review.source.text.clone(),
                "Narrator".into(),
                None,
            )]
        } else {
            review
                .source
                .cues
                .iter()
                .map(|c| {
                    (
                        c.id.clone(),
                        c.text.clone(),
                        c.role.clone(),
                        Some((c.start_ms, c.end_ms)),
                    )
                })
                .collect()
        };
        for (q, (id, text, role, window)) in inputs.into_iter().enumerate() {
            let binding = targets
                .get(&("cue", id.as_str()))
                .or_else(|| targets.get(&("role", role.as_str())))
                .copied();
            let voice_id = binding.map_or(request.voice_id.as_str(), |a| a.voice_id.as_str());
            let reference_id =
                binding.map_or(request.reference_id.as_str(), |a| a.reference_id.as_str());
            let voice = catalog
                .voices
                .iter()
                .find(|v| v.voice_id == voice_id)
                .ok_or_else(|| invalid("Unknown assigned voice"))?;
            let reference = voice
                .references
                .iter()
                .find(|r| r.reference_id == reference_id && r.engine_id == profile.engine_id)
                .ok_or_else(|| {
                    invalid("Voice reference is incompatible with the selected engine")
                })?;
            if reference.qualification_status != "qualified" {
                return Err(invalid(format!(
                    "Reference {voice_id}/{reference_id} requires human qualification for {}",
                    profile.engine_id
                ))
                .into());
            }
            let seed_key = stable_cue_seed(&project, &id);
            let role_id = format!("role-{}", &seed_key[..20]);
            contract_bindings.push(ports::puza_contract::PuzaVoiceBinding {
                role_id: role_id.clone(),
                display_name: role.clone(),
                voice_id: voice_id.into(),
                reference_id: reference_id.into(),
                engine_id: profile.engine_id.clone(),
            });
            cast.push(json!({"role_id": role_id,"voice_id": voice_id,"manifest_sha256": voice.manifest_sha256}));
            utterances.push(json!({"utterance_id": format!("utterance-{:04}", q+1), "cue_id": window.map(|_| id.clone()),
                "text_range": if window.is_none() { json!({"start":0,"end":text.chars().count()}) } else { serde_json::Value::Null },
                "text":text, "role_id":role_id,"reference_id":reference_id,"emotion_id":reference.emotion_id,
                "start_ms":window.map(|w|w.0),"end_ms":window.map(|w|w.1),"speed":1.0,"volume":1.0,"seed_key":seed_key}));
        }
        let revision = match profile.engine_id.as_str() {
            "voxcpm2" => "32279effe8c19989596f05d353d1447f51d9e915",
            "espeech" => "f582b6e5897fe8a5835059405a8439d13bdf7684",
            _ => return Err(invalid("Unsupported speech engine").into()),
        };
        let draft = json!({"version":1,"plan_id":run_id,"project_id":project,"mode":if review.source.kind=="text" {"continuous"} else {"subtitle_utterances"},
            "source":{"source_sha256":review.source.sha256,"text_revision":review.source.revision,"reviewed_text":if review.source.kind=="text" {Some(review.source.text.clone())}else{None}},
            "review":{"decision_id":review.decision_id,"status":"approved"},
            "profile":{"profile_id":profile.profile_id,"profile_sha256":profile.profile_sha256,"engine_id":profile.engine_id,"model_revision":revision},
            "cast":cast,"utterances":utterances}).to_string();
        if draft.len() > MAX_SPEECH_PLAN_BYTES {
            return Err(invalid(format!("Speech draft exceeds {MAX_SPEECH_PLAN_BYTES} bytes; reduce the admitted source or assignments")).into());
        }
        if let Some(contract) = &self.contract {
            contract
                .configure(&project, review.source.text.clone(), contract_bindings)
                .await?;
        }
        let engine = self
            .engine
            .as_ref()
            .ok_or_else(|| invalid("TTS runtime unavailable"))?;
        let intent = engine
            .compose(project, run_id, request.profile_id, draft)
            .await?;
        self.limits()?.plan(&intent)?;
        self.store
            .register_reviewed_run(&intent, request.decision_id)
            .await?;
        self.view(intent).await
    }

    pub(super) async fn check_intent(
        &self,
        intent: &SpeechRunIntent,
    ) -> Result<(), ApplicationError> {
        self.limits()?.plan(intent)?;
        let plan: serde_json::Value =
            serde_json::from_str(&intent.plan_json).map_err(|e| invalid(e.to_string()))?;
        let decision = plan["review"]["decision_id"]
            .as_str()
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| invalid("Speech plan has no review decision"))?;
        let review = self
            .store
            .get_review(decision)
            .await?
            .ok_or_else(|| invalid("Speech review is missing"))?;
        if review.project_id != intent.project_id
            || plan["source"]["source_sha256"].as_str() != Some(&review.source.sha256)
        {
            return Err(conflict("Speech review and plan identity differ").into());
        }
        self.check_source(&review).await?;
        self.engine
            .as_ref()
            .ok_or_else(|| {
                invalid(
                    self.runtime_error
                        .clone()
                        .unwrap_or_else(|| "TTS runtime unavailable".into()),
                )
            })?
            .validate(intent)
            .await?;
        Ok(())
    }
}

fn stable_cue_seed(project: &ProjectId, cue_id: &str) -> String {
    hash(format!("auralis-speech-cue-v1\0{project}\0{cue_id}").as_bytes())
}

#[cfg(test)]
mod tests {
    use super::stable_cue_seed;
    use domain::project::ProjectId;

    #[test]
    fn cue_seed_survives_new_runs_and_changes_only_with_project_or_cue_identity() {
        let project = ProjectId::default();
        let other = ProjectId::default();
        let first = stable_cue_seed(&project, "cue-1");
        assert_eq!(first, stable_cue_seed(&project, "cue-1"));
        assert_ne!(first, stable_cue_seed(&project, "cue-2"));
        assert_ne!(first, stable_cue_seed(&other, "cue-1"));
        assert_eq!(first.len(), 64);
    }
}
