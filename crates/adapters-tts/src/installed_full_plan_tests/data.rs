use super::*;
use ports::puza_contract::PuzaVoiceBinding;

pub fn draft(
    client: &TtsWorkerClient,
    project: &ProjectId,
    run: Uuid,
    engine: &str,
) -> TestResult<(Value, Vec<PuzaVoiceBinding>, String)> {
    let runtime = client.runtime();
    let profile_id = format!("native-plan-{engine}");
    let profile = runtime.profile_root.join(format!("{profile_id}.json"));
    let profile_sha = inputs::file_hash(&profile)?;
    let voice_id = "marina_elevenlabs_ru";
    let voice_file = runtime.voice_store.join(voice_id).join("voice.json");
    let manifest_sha = inputs::file_hash(&voice_file)?;
    let voice: Value = serde_json::from_slice(&std::fs::read(voice_file)?)?;
    let reference_id = format!("ref-{engine}-neutral");
    let reference = voice["references"]
        .as_array()
        .ok_or("voice references missing")?
        .iter()
        .find(|r| r["reference_id"] == reference_id && r["engine_id"] == engine)
        .ok_or("neutral reference missing")?;
    if reference["emotion_id"] != "neutral" || reference["qualification"]["status"] != "unverified"
    {
        return Err("control requires unchanged unverified neutral reference".into());
    }
    let permission = Uuid::new_v4().to_string();
    let binding = |role: &str| PuzaVoiceBinding {
        role_id: role.into(),
        display_name: "Рассказчик".into(),
        voice_id: voice_id.into(),
        reference_id: reference_id.clone(),
        engine_id: engine.into(),
        technical_reference_permission: Some(permission.clone()),
    };
    let mut bindings = vec![binding("narrator")];
    let mut cast = Vec::new();
    let mut utterances = Vec::new();
    let mut source = Vec::new();
    for i in 1..=COUNT {
        let cue = format!("cue-{i:04}");
        let seed = inputs::hash(format!("{project}:{cue}").as_bytes());
        let role = format!("role-{}", &seed[..20]);
        let text = format!("Техническая реплика номер {i:04}.");
        source.push(text.clone());
        bindings.push(binding(&role));
        cast.push(json!({"role_id":role,"voice_id":voice_id,"manifest_sha256":manifest_sha}));
        utterances.push(json!({"utterance_id":format!("utterance-{i:04}"),"cue_id":cue,"text_range":null,
            "text":text,"role_id":role,"reference_id":reference_id,"emotion_id":"neutral",
            "start_ms":(i-1)*1000,"end_ms":(i-1)*1000+800,"speed":1.0,"volume":1.0,"seed_key":seed}));
    }
    let source = source.join("\n");
    let revision = match engine {
        "voxcpm2" => "32279effe8c19989596f05d353d1447f51d9e915",
        "espeech" => "f582b6e5897fe8a5835059405a8439d13bdf7684",
        _ => return Err("engine unsupported".into()),
    };
    Ok((
        json!({"version":1,"plan_id":run,"project_id":project,"mode":"subtitle_utterances",
        "source":{"source_sha256":inputs::hash(source.as_bytes()),"text_revision":"metadata-control-1","reviewed_text":null},
        "review":{"decision_id":Uuid::new_v4(),"status":"approved","technical_reference_permission":permission},
        "profile":{"profile_id":profile_id,"profile_sha256":profile_sha,"engine_id":engine,"model_revision":revision},
        "cast":cast,"utterances":utterances}),
        bindings,
        source,
    ))
}
