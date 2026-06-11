//! Streaming helpers for `/v1/chat/completions`.
//!
//! The actual streaming implementation lives in [`super::chat`] (alongside the
//! non-streaming `handle_non_stream` and the body-builder helpers).  This
//! module is a thin re-export so callers that prefer a "streaming-only"
//! module path can write `crate::handlers::streaming::handle_stream` while
//! the real code stays grouped with the rest of the chat-completions
//! logic.

#[allow(unused_imports)]
pub(crate) use super::chat::{
    build_stream_chunk_response_body, build_stream_final_response_body, handle_stream,
};
