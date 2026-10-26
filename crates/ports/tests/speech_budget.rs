use ports::speech_run::SpeechExecutionBudget;

#[test]
fn finite_speech_budgets_keep_the_legacy_default_and_round_trip_as_seconds()
-> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(SpeechExecutionBudget::default().seconds(), 2700);
    for seconds in [1, 2700, 7200, 172800] {
        let budget = SpeechExecutionBudget::try_from(seconds)?;
        assert_eq!(budget.duration().as_secs(), seconds);
        let json = serde_json::to_string(&budget)?;
        assert_eq!(json, seconds.to_string());
        assert_eq!(
            serde_json::from_str::<SpeechExecutionBudget>(&json)?,
            budget
        );
    }
    for json in [
        "0",
        "172801",
        "-1",
        "1.5",
        "null",
        "\"172800\"",
        "18446744073709551616",
    ] {
        assert!(
            serde_json::from_str::<SpeechExecutionBudget>(json).is_err(),
            "{json}"
        );
    }
    assert!(SpeechExecutionBudget::try_from(u64::MAX).is_err());
    Ok(())
}
