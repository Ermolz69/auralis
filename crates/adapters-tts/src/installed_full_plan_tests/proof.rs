use super::*;

#[cfg(windows)]
pub(super) fn exited(pid: u32, creation: u64) -> TestResult<bool> {
    use std::os::windows::io::{FromRawHandle, OwnedHandle};
    use windows::{
        Win32::{
            Foundation::{ERROR_INVALID_PARAMETER, FILETIME, WAIT_OBJECT_0},
            System::Threading::{
                GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
                PROCESS_SYNCHRONIZE, WaitForSingleObject,
            },
        },
        core::HRESULT,
    };
    let handle = match unsafe {
        OpenProcess(
            PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
            false,
            pid,
        )
    } {
        Ok(handle) => handle,
        Err(error) if error.code() == HRESULT::from_win32(ERROR_INVALID_PARAMETER.0) => {
            return Ok(true);
        }
        Err(error) => return Err(Box::new(error)),
    };
    let _owned = unsafe { OwnedHandle::from_raw_handle(handle.0) };
    let (mut created, mut end, mut kernel, mut user) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    unsafe { GetProcessTimes(handle, &mut created, &mut end, &mut kernel, &mut user) }?;
    let current = (u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime);
    // A reused PID proves that the original identity ended; never control the new process.
    Ok(current != creation || unsafe { WaitForSingleObject(handle, 0) } == WAIT_OBJECT_0)
}

#[cfg(not(windows))]
pub(super) fn exited(_: u32, _: u64) -> TestResult<bool> {
    Err("metadata process proof requires Windows".into())
}

pub fn audit_reports(root: &Path) -> TestResult<Vec<Value>> {
    let mut entries: Vec<_> = std::fs::read_dir(root)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    if entries.len() != 6 {
        return Err("expected one audited CONTRACT and two TTS processes per engine".into());
    }
    let mut reports = Vec::new();
    let mut adapters = 0;
    for entry in entries {
        let mut report: Value = serde_json::from_slice(&std::fs::read(entry.path())?)?;
        if report["contract_version"] != "0.2.0"
            || report["tts_version"] != "0.3.0"
            || report["model_file_opens"] != 0
            || report["engine_calls"] != 0
            || !report["violations"]
                .as_array()
                .ok_or("audit violations missing")?
                .is_empty()
        {
            return Err(
                "installed metadata control violated its frozen model-free boundary".into(),
            );
        }
        if report["profile_active_before_exit"] != true
            || !report["framework_modules_loaded"]
                .as_array()
                .ok_or("framework audit missing")?
                .is_empty()
        {
            return Err("model-free profile guard was inactive or a framework loaded".into());
        }
        let pid = u32::try_from(report["pid"].as_u64().ok_or("audit PID missing")?)?;
        let created = report["creation_time"]
            .as_u64()
            .ok_or("audit creation time missing")?;
        if !exited(pid, created)? {
            return Err("audited metadata worker exit unconfirmed".into());
        }
        report["exit_confirmed"] = json!(true);
        if !report["thin_adapter_imports"]
            .as_array()
            .ok_or("adapter import record missing")?
            .is_empty()
        {
            adapters += 1;
            chunk_preview(root, &report)?;
        }
        reports.push(report);
    }
    if adapters != 4 {
        return Err(
            "thin adapter imports were not recorded for all four TTS metadata calls".into(),
        );
    }
    Ok(reports)
}

fn chunk_preview(root: &Path, audit: &Value) -> TestResult<()> {
    use std::collections::HashMap;
    let previews = audit["chunk_previews"]
        .as_array()
        .ok_or("chunk preview audit missing")?;
    ensure(
        previews.len() == 1,
        "each TTS metadata call must prepare one full plan",
    )?;
    let preview = &previews[0];
    let engine = preview["engine"]
        .as_str()
        .filter(|engine| matches!(*engine, "voxcpm2" | "espeech"))
        .ok_or("chunk engine invalid")?;
    let draft: Value = serde_json::from_slice(&std::fs::read(
        root.parent()
            .ok_or("managed root missing")?
            .join(engine)
            .join("frozen-draft.json"),
    )?)?;
    let utterances = draft["utterances"]
        .as_array()
        .ok_or("draft utterances missing")?;
    let expected: Vec<_> = utterances
        .iter()
        .map(|utterance| utterance["utterance_id"].clone())
        .collect();
    ensure(
        preview["expected_utterance_ids"] == json!(expected),
        "prepared job changed complete utterance order",
    )?;
    let cast: HashMap<_, _> = draft["cast"]
        .as_array()
        .ok_or("draft cast missing")?
        .iter()
        .map(|role| {
            (
                role["role_id"].as_str().unwrap_or_default(),
                &role["voice_id"],
            )
        })
        .collect();
    let segments = preview["segments"]
        .as_array()
        .ok_or("prepared chunk list missing")?;
    ensure(
        segments.len() == COUNT && utterances.len() == COUNT,
        "prepared chunk/utterance coverage differs",
    )?;
    for (segment, utterance) in segments.iter().zip(utterances) {
        let id = utterance["utterance_id"]
            .as_str()
            .ok_or("utterance ID missing")?;
        ensure(
            segment["utterance_id"] == utterance["utterance_id"]
                && segment["chunk_id"] == format!("{id}-chunk-0001")
                && segment["cue_id"] == utterance["cue_id"]
                && segment["text"] == utterance["text"]
                && segment["role_id"] == utterance["role_id"]
                && segment["reference_id"] == utterance["reference_id"]
                && segment["start_ms"] == utterance["start_ms"]
                && segment["end_ms"] == utterance["end_ms"],
            "prepared technical chunk lost or changed reviewed utterance lineage",
        )?;
        ensure(
            cast.get(utterance["role_id"].as_str().ok_or("role missing")?)
                .is_some_and(|voice| **voice == segment["voice_id"]),
            "prepared chunk changed assigned voice",
        )?;
    }
    Ok(())
}
