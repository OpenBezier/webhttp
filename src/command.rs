use super::websocket::{OutMessage, ROOM};
use crossbeam::queue::SegQueue;
use dashmap::DashMap;
use futures::future::Future;
use futures::task::Poll;
use serde::Serialize;
use std::sync::Arc;
use std::task::Waker;
use webproto;

#[derive(Clone)]
pub struct ServerCommand<T: Send + Sync + serde::ser::Serialize> {
    pub queue: Arc<DashMap<String, Option<T>>>,
    pub wake_mgt: WakerManager,
}

impl<T: Send + Sync + serde::ser::Serialize> ServerCommand<T> {
    pub async fn send_indication(&self, client_id: String, in_data: T) -> anyhow::Result<()> {
        let addr = match ROOM.sessions.entry(client_id.clone()) {
            dashmap::mapref::entry::Entry::Occupied(entry) => entry.get().1.clone(),
            dashmap::mapref::entry::Entry::Vacant(_) => {
                return Err(anyhow::anyhow!(
                    "socket mutex is not existed: {:?}",
                    client_id
                ));
            }
        };

        let data = webproto::Indication::<T>::encode(in_data)?;
        let command = OutMessage { data: data };
        let send_status = addr.send(command).await;
        if send_status.is_err() {
            return Err(anyhow::anyhow!(
                "send socket data to client with error: {:?}",
                send_status.err()
            ));
        }
        anyhow::Ok(())
    }

    pub async fn send_answer(
        &self,
        client_id: String,
        event_id: String,
        in_data: impl Serialize,
    ) -> anyhow::Result<()> {
        let addr = match ROOM.sessions.entry(client_id.clone()) {
            dashmap::mapref::entry::Entry::Occupied(entry) => entry.get().1.clone(),
            dashmap::mapref::entry::Entry::Vacant(_) => {
                return Err(anyhow::anyhow!(
                    "socket mutex is not existed: {:?}",
                    client_id
                ));
            }
        };

        let data = webproto::ClientCommand::<T>::encode(in_data, event_id.clone())?;
        let command = OutMessage { data: data };
        let send_status = addr.send(command).await;
        if send_status.is_err() {
            return Err(anyhow::anyhow!(
                "send socket data to client with error: {:?}",
                send_status.err()
            ));
        }
        anyhow::Ok(())
    }

    pub fn new(queue: Arc<DashMap<String, Option<T>>>) -> Self {
        let wake_mgt = WakerManager::default();
        wake_mgt.start(1);
        ServerCommand {
            queue: queue,
            wake_mgt: wake_mgt,
        }
    }

    pub async fn send_command(
        &self,
        client_id: String,
        in_data: impl Serialize,
        timeout_seconds: u64,
    ) -> anyhow::Result<T> {
        let event_id = uuid::Uuid::new_v4().to_string();
        let addr = match ROOM.sessions.entry(client_id.clone()) {
            dashmap::mapref::entry::Entry::Occupied(entry) => entry.get().1.clone(),
            dashmap::mapref::entry::Entry::Vacant(_) => {
                return Err(anyhow::anyhow!(
                    "socket mutex is not existed: {:?}",
                    client_id
                ));
            }
        };

        let data = webproto::ServerCommand::<T>::encode(in_data, event_id.clone())?;
        let command = OutMessage { data: data };
        self.queue.insert(event_id.clone(), None);
        let send_status = addr.send(command).await;
        if send_status.is_err() {
            self.queue.remove(&event_id);
            return Err(anyhow::anyhow!(
                "send socket data to client with error: {:?}",
                send_status.err()
            ));
        }

        let client = ServerInternalCommand {
            event_id: event_id.clone(),
            queue: self.queue.clone(),
            wake_mgt: self.wake_mgt.clone(),
        };

        let resp =
            tokio::time::timeout(tokio::time::Duration::from_secs(timeout_seconds), client).await;
        match resp {
            Ok(resp) => return anyhow::Ok(resp),
            Err(_) => {
                return Err(anyhow::anyhow!("timeout for waiting for client response"));
            }
        }
    }
}

#[derive(Clone)]
pub struct ServerInternalCommand<T: Send + Sync + serde::ser::Serialize> {
    pub event_id: String,
    pub queue: Arc<DashMap<String, Option<T>>>,
    pub wake_mgt: WakerManager,
}

impl<T: Send + Sync + serde::ser::Serialize> Future for ServerInternalCommand<T> {
    type Output = T;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let Self {
            event_id,
            queue,
            wake_mgt,
        } = &mut *self;

        let value = queue.entry(event_id.clone());
        match value {
            dashmap::mapref::entry::Entry::Occupied(e) => {
                if e.get().is_some() {
                    let out_data = e.remove().unwrap();
                    return Poll::Ready(out_data);
                }
            }
            dashmap::mapref::entry::Entry::Vacant(_) => {}
        }

        wake_mgt.wakers.push(cx.waker().clone());
        return Poll::Pending;
    }
}

#[derive(Default, Clone)]
pub struct WakerManager {
    pub wakers: Arc<SegQueue<Waker>>,
}

impl WakerManager {
    pub fn start(&self, duration_millis: u64) {
        let copied_manager = self.clone();
        // let arbiter = actix_rt::Arbiter::current();
        tokio::spawn(async move {
            let manager = copied_manager;
            loop {
                // std::thread::sleep(dur);
                tokio::time::sleep(tokio::time::Duration::from_millis(duration_millis)).await;
                let queue_length = manager.wakers.len();
                for _i in 0..queue_length {
                    if let Some(item) = manager.wakers.pop() {
                        item.wake();
                    }
                }
            }
        });
    }
}
