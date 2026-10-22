#![allow(clippy::unwrap_used)]

#[path = "job_queue/cancellation.rs"]
mod cancellation;
#[path = "job_queue/cpu_lifetime.rs"]
mod cpu_lifetime;
#[path = "job_queue/failures.rs"]
mod failures;
#[path = "job_queue/pipeline.rs"]
mod pipeline;
#[path = "job_queue/resources.rs"]
mod resources;
#[path = "job_queue/srt.rs"]
mod srt;
#[path = "support/job_queue.rs"]
mod support;
