/// The values a pop-out held when it opened, so Cancel can write them back
/// into the live row widgets.
pub(crate) struct PopoutSnapshot {
    pub(crate) name: String,
    pub(crate) modifier: String,
    pub(crate) keys: String,
    pub(crate) fields: Vec<(String, String)>,
    pub(crate) invalid_provider: Option<String>,
}
