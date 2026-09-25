pub trait TranslationModelLease: Send {
    fn server_url(&self) -> &str;
}
