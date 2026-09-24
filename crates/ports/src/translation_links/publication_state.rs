#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranslationPublicationState {
    Pending,
    Ready,
    Failed,
}
