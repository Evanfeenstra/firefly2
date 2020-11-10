pub use iota_wallet_actor::{
  Message as WalletMessage, MessageType as WalletMessageType, WalletMessageHandler,
};
use riker::actors::*;
use serde::Deserialize;
use tokio::{runtime::Runtime, sync::mpsc::unbounded_channel};

use std::path::PathBuf;
use std::time::Duration;

const POLLING_INTERVAL_MS: u64 = 30_000;

pub struct WalletActor {
  wallet_message_handler: WalletMessageHandler,
  runtime: Runtime,
}

impl WalletActor {
  /// Starts the polling mechanism.
  pub fn start_polling(&self, interval_ms: u64) -> &Self {
    self
      .wallet_message_handler
      .account_manager()
      .start_polling(Duration::from_millis(interval_ms));
    self
  }
}

impl ActorFactoryArgs<PathBuf> for WalletActor {
  fn create_args(storage_path: PathBuf) -> Self {
    let actor = Self {
      wallet_message_handler: WalletMessageHandler::with_storage_path(storage_path)
        .expect("failed to initialise account manager"),
      runtime: Runtime::new().expect("failed to create tokio runtime"),
    };
    actor.start_polling(POLLING_INTERVAL_MS);
    actor
  }
}

impl Default for WalletActor {
  fn default() -> Self {
    let actor = Self {
      wallet_message_handler: Default::default(),
      runtime: Runtime::new().expect("failed to create tokio runtime"),
    };
    actor.start_polling(POLLING_INTERVAL_MS);
    actor
  }
}

impl Actor for WalletActor {
  type Msg = WalletMessage;

  fn recv(&mut self, _ctx: &Context<Self::Msg>, msg: Self::Msg, _sender: Sender) {
    let wallet_message_handler = &self.wallet_message_handler;
    self.runtime.block_on(async move {
      wallet_message_handler.handle(msg).await;
    });
  }
}

#[derive(Deserialize)]
struct DispatchMessage {
  id: usize,
  #[serde(flatten)]
  message: WalletMessageType,
}

pub(crate) async fn dispatch(
  wallet_actor: &ActorRef<WalletMessage>,
  message: String,
) -> Result<Option<String>, String> {
  let (response_tx, mut response_rx) = unbounded_channel();
  let message: DispatchMessage = serde_json::from_str(&message).map_err(|e| e.to_string())?;
  wallet_actor.tell(
    WalletMessage::new(message.id, message.message, response_tx),
    None,
  );
  let response = response_rx.recv().await;
  match response {
    Some(res) => Ok(Some(
      serde_json::to_string(&res).map_err(|e| e.to_string())?,
    )),
    None => Ok(None),
  }
}
