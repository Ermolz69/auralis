mod audio_layout;
mod catalog;
mod client;
mod execute;
mod framed;
#[cfg(test)]
mod installed_full_plan_tests;
mod integrity;
#[cfg(test)]
mod integrity_tests;
mod output_layout;
mod owned_process;
mod preflight;
mod protocol;
mod result_verification;
mod verification_control;
mod wav_verification;
mod workflow;

pub use audio_layout::verify_result_audio_layout;
pub use client::{TtsRuntimeConfig, TtsWorkerClient};
pub use execute::{
    SavedCheckpoint, TtsCompleted, TtsExecutionObserver, TtsExecutionOutcome, TtsExecutionRequest,
};
pub use framed::WorkerFrameReader;
pub use preflight::{TtsCapabilities, TtsCatalog, TtsComposed, TtsValidated};
pub use protocol::{WorkerEvent, WorkerIdentity, WorkerProtocol};
pub use result_verification::verify_result_files;
mod contract;
mod marker;
mod marker_lock;
#[cfg(test)]
mod marker_tests;
pub use marker::{MarkerClient, MarkerRuntimeConfig};
pub mod media_contract;
