use auralis_translation::{ProgressSink, RunControl, RunId, RunProgress};
use std::error::Error;

pub struct UninterruptedRun;

impl ProgressSink for UninterruptedRun {
    fn report(&mut self, _: RunProgress) {}
}
impl RunControl for UninterruptedRun {
    fn pause_requested(&self, _: RunId) -> Result<bool, Box<dyn Error>> {
        Ok(false)
    }
}
