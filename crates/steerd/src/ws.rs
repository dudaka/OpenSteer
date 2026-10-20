//! One session per client over the WebSocket; every frame is MessagePack.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use tokio::sync::broadcast;

use crate::contract::{Call, HostFrame, Reply, Request};

pub async fn handler(
    ws: WebSocketUpgrade,
    State(frames): State<broadcast::Sender<HostFrame>>,
) -> Response {
    let frames = frames.subscribe();
    ws.on_upgrade(|socket| serve(socket, frames))
}

/// Answers each request and forwards each simulation frame until the client goes.
async fn serve(mut socket: WebSocket, mut frames: broadcast::Receiver<HostFrame>) {
    tracing::info!(event = "client.connect");
    loop {
        let frame = tokio::select! {
            msg = socket.recv() => match msg {
                Some(Ok(Message::Binary(bytes))) => match respond(&bytes) {
                    Some(frame) => frame,
                    None => continue,
                },
                Some(Ok(_)) => continue,
                Some(Err(_)) | None => break,
            },
            frame = frames.recv() => match frame {
                Ok(frame) => frame,
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    tracing::warn!(event = "client.lagged", skipped);
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            },
        };
        let bytes = rmp_serde::to_vec_named(&frame).expect("HostFrame serializes");
        if socket.send(Message::Binary(bytes.into())).await.is_err() {
            break;
        }
    }
    tracing::info!(event = "client.disconnect");
}

/// The reply frame for one request frame; `None` for a frame that is not a request.
fn respond(bytes: &[u8]) -> Option<HostFrame> {
    let req: Request = match rmp_serde::from_slice(bytes) {
        Ok(req) => req,
        Err(e) => {
            tracing::warn!(event = "client.bad_frame", error = %e);
            return None;
        }
    };
    Some(HostFrame::Res {
        id: req.id,
        result: Some(answer(req.call)),
        error: None,
    })
}

fn answer(call: Call) -> Reply {
    match call {
        Call::DemoList => Reply::Demos(steer_demos::catalog()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_list_round_trip() {
        let bytes = rmp_serde::to_vec_named(&Request {
            id: 7,
            call: Call::DemoList,
        })
        .unwrap();
        assert_eq!(
            respond(&bytes),
            Some(HostFrame::Res {
                id: 7,
                result: Some(Reply::Demos(steer_demos::catalog())),
                error: None,
            })
        );
    }

    #[test]
    fn bad_frame_is_ignored() {
        assert_eq!(respond(&[0xc1]), None);
    }
}
