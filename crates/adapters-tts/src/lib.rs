mod catalog;
mod client;
mod execute;
mod framed;
mod integrity;
#[cfg(test)]
mod integrity_tests;
mod owned_process;
mod preflight;
mod protocol;
mod result_verification;
mod wav_verification;
mod workflow;

pub use client::{TtsRuntimeConfig, TtsWorkerClient};
pub use execute::{
    SavedCheckpoint, TtsCompleted, TtsExecutionObserver, TtsExecutionOutcome, TtsExecutionRequest,
};
pub use framed::WorkerFrameReader;
pub use preflight::{TtsCapabilities, TtsCatalog, TtsComposed, TtsValidated};
pub use protocol::{WorkerEvent, WorkerIdentity, WorkerProtocol};
pub use result_verification::verify_result_files;
