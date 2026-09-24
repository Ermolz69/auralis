use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReadyPublication {
    pub translation_id: Uuid,
    pub result_id: Uuid,
}
