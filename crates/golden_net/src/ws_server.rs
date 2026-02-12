use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use golden_core::Engine;
use golden_core::edits::{Edit, EditOrigin, Propagation};
use golden_schema::ui::messages::{
    EventBatch, GetSnapshot, MessageEnvelope, SetParam, Snapshot, Subscribe,
};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use crate::snapshot::build_snapshot;

#[derive(Clone, Debug)]
pub struct WsServerConfig {
    pub addr: SocketAddr,
}

pub async fn start_ws_server(
    engine: Arc<Mutex<Engine>>,
    config: WsServerConfig,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(config.addr).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        let engine = Arc::clone(&engine);
        tokio::spawn(async move {
            if let Err(err) = handle_connection(engine, stream).await {
                eprintln!("ws error: {err}");
            }
        });
    }
}

async fn handle_connection(
    engine: Arc<Mutex<Engine>>,
    stream: tokio::net::TcpStream,
) -> anyhow::Result<()> {
    let ws = tokio_tungstenite::accept_async(stream).await?;
    let (mut ws_write, mut ws_read) = ws.split();
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<String>();

    let writer = tokio::spawn(async move {
        while let Some(text) = out_rx.recv().await {
            if ws_write.send(Message::Text(text)).await.is_err() {
                break;
            }
        }
    });

    let snapshot = build_snapshot(&engine.lock().unwrap());
    send_snapshot(&out_tx, snapshot)?;

    let mut subscription_task: Option<tokio::task::JoinHandle<()>> = None;

    while let Some(msg) = ws_read.next().await {
        let msg = msg?;
        if !msg.is_text() {
            continue;
        }
        let text = msg.into_text()?;
        if let Ok(envelope) = serde_json::from_str::<MessageEnvelope<serde_json::Value>>(&text) {
            let payload = envelope.payload;
            match envelope.msg.as_str() {
                "GetSnapshot" => {
                    let _ = serde_json::from_value::<GetSnapshot>(payload);
                    let snapshot = build_snapshot(&engine.lock().unwrap());
                    send_snapshot(&out_tx, snapshot)?;
                }
                "SetParam" => {
                    if let Ok(set_param) = serde_json::from_value::<SetParam>(payload) {
                        let snapshot = {
                            let mut engine = engine.lock().unwrap();
                            let propagation = match set_param.propagation {
                                golden_schema::ui::messages::Propagation::Immediate => {
                                    Propagation::Immediate
                                }
                                golden_schema::ui::messages::Propagation::EndOfTick => {
                                    Propagation::EndOfTick
                                }
                                golden_schema::ui::messages::Propagation::NextTick => {
                                    Propagation::NextTick
                                }
                            };
                            engine.enqueue_edit(
                                Edit::SetParam {
                                    node: set_param.param_node_id,
                                    value: set_param.value,
                                },
                                propagation,
                                EditOrigin::Network,
                            );
                            engine.tick();
                            build_snapshot(&engine)
                        };
                        send_snapshot(&out_tx, snapshot)?;
                    }
                }
                "Subscribe" => {
                    if let Ok(subscribe) = serde_json::from_value::<Subscribe>(payload) {
                        if let Some(task) = subscription_task.take() {
                            task.abort();
                        }
                        let engine = Arc::clone(&engine);
                        let out_tx = out_tx.clone();
                        subscription_task = Some(tokio::spawn(async move {
                            let mut last_time = subscribe.from;
                            let mut interval =
                                tokio::time::interval(std::time::Duration::from_millis(16));
                            loop {
                                interval.tick().await;
                                let events = {
                                    let engine = engine.lock().unwrap();
                                    engine.events_since(last_time)
                                };
                                if let Some(last) = events.last() {
                                    last_time = last.time;
                                }
                                if events.is_empty() {
                                    continue;
                                }
                                let batch = EventBatch { events };
                                if send_event_batch(&out_tx, batch).is_err() {
                                    break;
                                }
                            }
                        }));
                    }
                }
                _ => {}
            }
        }
    }

    if let Some(task) = subscription_task {
        task.abort();
    }
    let _ = writer.await;

    Ok(())
}

fn send_snapshot(tx: &mpsc::UnboundedSender<String>, snapshot: Snapshot) -> anyhow::Result<()> {
    let envelope = MessageEnvelope {
        msg: "Snapshot".to_string(),
        req_id: None,
        payload: snapshot,
    };
    let text = serde_json::to_string(&envelope)?;
    tx.send(text)
        .map_err(|_| anyhow::anyhow!("ws send failed"))?;
    Ok(())
}

fn send_event_batch(tx: &mpsc::UnboundedSender<String>, batch: EventBatch) -> anyhow::Result<()> {
    let envelope = MessageEnvelope {
        msg: "EventBatch".to_string(),
        req_id: None,
        payload: batch,
    };
    let text = serde_json::to_string(&envelope)?;
    tx.send(text)
        .map_err(|_| anyhow::anyhow!("ws send failed"))?;
    Ok(())
}
