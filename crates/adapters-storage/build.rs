fn main() {
    let _ = std::fs::rename(
        "migrations/0006_outbox_aggregate.sql",
        "migrations/0011_outbox_aggregate.sql",
    );
}
