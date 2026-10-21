//! One session per client over the WebSocket; every frame is MessagePack.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use steer_core::Vec2;
use tokio::sync::{mpsc, watch};

use crate::contract::{Call, HostFrame, Reply, Request};
use crate::origin::LoopbackGuard;
use crate::sim::{catalog, Command};

/// What every session shares: the latest frame and the way to the sim.
#[derive(Clone)]
pub struct Channels {
    pub frames: watch::Receiver<HostFrame>,
    pub commands: mpsc::UnboundedSender<Command>,
}

pub async fn handler(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(channels): State<Channels>,
) -> Response {
    if !(LoopbackGuard { port: crate::PORT }).allows(&headers) {
        tracing::warn!(
            event = "client.rejected",
            host = ?headers.get("host"),
            origin = ?headers.get("origin"),
            "WebSocket from a page this host does not serve"
        );
        return StatusCode::FORBIDDEN.into_response();
    }
    ws.on_upgrade(|socket| serve(socket, channels))
}

/// Sends the current frame, then answers each request and forwards each new frame until the
/// client goes. A slow client skips to the latest frame.
async fn serve(
    mut socket: WebSocket,
    Channels {
        mut frames,
        commands,
    }: Channels,
) {
    tracing::info!(event = "client.connect");
    frames.mark_changed();
    loop {
        let frame = tokio::select! {
            msg = socket.recv() => match msg {
                Some(Ok(Message::Binary(bytes))) => match respond(&bytes, &commands) {
                    Some(frame) => frame,
                    None => continue,
                },
                Some(Ok(_)) => continue,
                Some(Err(_)) | None => break,
            },
            changed = frames.changed() => match changed {
                Ok(()) => frames.borrow_and_update().clone(),
                Err(_) => break,
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
fn respond(bytes: &[u8], commands: &mpsc::UnboundedSender<Command>) -> Option<HostFrame> {
    let req: Request = match rmp_serde::from_slice(bytes) {
        Ok(req) => req,
        Err(e) => {
            tracing::warn!(event = "client.bad_frame", error = %e);
            return None;
        }
    };
    let (result, error) = match answer(req.call, commands) {
        Ok(reply) => (Some(reply), None),
        Err(e) => (None, Some(e)),
    };
    Some(HostFrame::Res {
        id: req.id,
        result,
        error,
    })
}

fn answer(call: Call, commands: &mpsc::UnboundedSender<Command>) -> Result<Reply, String> {
    let command = match call {
        Call::DemoList => return Ok(Reply::Demos(catalog())),
        Call::DemoSelect { id } if !catalog().iter().any(|d| d.id == id) => {
            return Err(format!("no demo named {id:?}"));
        }
        Call::DemoSelect { id } => Command::Select(id),
        Call::DemoClick { x, y } => Command::Click(Vec2::new(x, y)),
        Call::DemoAction { id } => Command::Action(id),
        Call::DemoParam { id, value } => Command::Param(id, value),
        Call::SimPlay => Command::Play,
        Call::SimPause => Command::Pause,
        Call::SimStep => Command::Step,
        Call::SimReset => Command::Reset,
    };
    commands
        .send(command)
        .expect("the sim task runs for the life of the host");
    Ok(Reply::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(id: u32, call: Call) -> Vec<u8> {
        rmp_serde::to_vec_named(&Request { id, call }).expect("Request serializes")
    }

    #[test]
    fn demo_list_round_trip() {
        let (commands, _) = mpsc::unbounded_channel();
        assert_eq!(
            respond(&request(7, Call::DemoList), &commands),
            Some(HostFrame::Res {
                id: 7,
                result: Some(Reply::Demos(catalog())),
                error: None,
            })
        );
    }

    #[test]
    fn sim_calls_reach_the_sim_and_reply_ok() {
        let (commands, mut received) = mpsc::unbounded_channel();
        let reply = respond(&request(3, Call::SimPause), &commands);
        assert_eq!(
            reply,
            Some(HostFrame::Res {
                id: 3,
                result: Some(Reply::Ok),
                error: None,
            })
        );
        assert_eq!(received.try_recv(), Ok(Command::Pause));
    }

    #[test]
    fn an_unknown_demo_is_refused_before_the_sim() {
        let (commands, mut received) = mpsc::unbounded_channel();
        let reply = respond(
            &request(4, Call::DemoSelect { id: "nope".into() }),
            &commands,
        );
        assert_eq!(
            reply,
            Some(HostFrame::Res {
                id: 4,
                result: None,
                error: Some("no demo named \"nope\"".into()),
            })
        );
        assert!(received.try_recv().is_err());
    }

    #[test]
    fn demo_param_reaches_the_sim() {
        let (commands, mut received) = mpsc::unbounded_channel();
        let call = Call::DemoParam {
            id: "slowing_radius".into(),
            value: 4.5,
        };
        respond(&request(5, call), &commands);
        assert_eq!(
            received.try_recv(),
            Ok(Command::Param("slowing_radius".into(), 4.5))
        );
    }

    #[test]
    fn bad_frame_is_ignored() {
        let (commands, _) = mpsc::unbounded_channel();
        assert_eq!(respond(&[0xc1], &commands), None);
    }
}
