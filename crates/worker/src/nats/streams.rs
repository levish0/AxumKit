// Stream names, publish subjects, durable consumer names, dead-letter routing and
// the idempotent stream creation are the contract shared with the API server, so
// they live in `job_queue` (the server runs `initialize_all_streams` at startup
// too, so a fresh NATS works regardless of which binary boots first).
pub use job_queue::streams::initialize_all_streams;
pub use job_queue::subjects::*;
