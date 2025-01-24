use crate::agentic::symbol::ui_event::UIEventWithID;
use axum::response::sse::Event;
use futures::{stream, Stream, StreamExt};
use serde_json::json;
use std::panic::{catch_unwind, AssertUnwindSafe};
use tracing::error;

// Helper function to create a safe stream that handles panics and errors
pub fn create_safe_stream(
    init_data: serde_json::Value,
    receiver: tokio::sync::mpsc::UnboundedReceiver<UIEventWithID>,
    session_id: String,
) -> impl Stream<Item = Result<Event, Box<dyn std::error::Error + Send + Sync>>> {
    let init_stream = {
        stream::once(async move {
            Ok(Event::default()
                .json_data(init_data)
                .expect("failed to serialize initialization object"))
        })
    };

    let event_stream = {
        let session_id = session_id.clone();
        tokio_stream::wrappers::UnboundedReceiverStream::new(receiver).map(move |event| {
            let session_id = session_id.clone();
            let result = catch_unwind(AssertUnwindSafe(move || Event::default().json_data(event)));

            match result {
                Ok(Ok(event)) => Ok(event),
                Ok(Err(e)) => {
                    error!("Failed to serialize event: {}", e);
                    Ok(Event::default()
                        .json_data(UIEventWithID::error(
                            session_id.clone(),
                            format!("Failed to process event: {}", e),
                        ))
                        .expect("failed to serialize error event"))
                }
                Err(panic) => {
                    error!("Panic in stream processing: {:?}", panic);
                    Ok(Event::default()
                        .json_data(UIEventWithID::error(
                            session_id,
                            "Internal server error: stream processing failed".to_owned(),
                        ))
                        .expect("failed to serialize error event"))
                }
            }
        })
    };

    let done_stream = stream::once(async move {
        Ok(Event::default()
            .json_data(json!({
                "done": "[CODESTORY_DONE]".to_owned(),
                "session_id": session_id,
            }))
            .expect("failed to serialize done object"))
    });

    init_stream.chain(event_stream).chain(done_stream)
}
