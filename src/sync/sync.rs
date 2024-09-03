use std::borrow::BorrowMut;
use std::sync::Arc;

use log::debug;
use log::info;
use opendal::Operator;
use tokio::select;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;

use super::init_operator;
use super::sync_client::SnapshotInfo;
use super::sync_client::SyncClient;
use crate::config::StorageConfig;
use crate::config::StorageParams;
use crate::database::DbCommand;
use crate::database::WalGenerationPos;
use crate::error::Result;

#[derive(Clone, Debug)]
pub enum SyncCommand {
    DbChanged(WalGenerationPos),
    Snapshot(Vec<u8>),
}

pub struct Sync {
    index: usize,
    client: SyncClient,
    db_notifier: Sender<DbCommand>,
    position: WalGenerationPos,
}

impl Sync {
    pub fn new(
        config: StorageConfig,
        index: usize,
        db_notifier: Sender<DbCommand>,
    ) -> Result<Arc<Self>> {
        Ok(Arc::new(Self {
            index,
            position: WalGenerationPos::default(),
            db_notifier,
            client: SyncClient::new(config)?,
        }))
    }

    pub fn start(s: Arc<Sync>, rx: Receiver<SyncCommand>) -> Result<JoinHandle<()>> {
        let handle = tokio::spawn(async move {
            let _ = Sync::main(s, rx).await;
        });

        Ok(handle)
    }

    pub async fn main(s: Arc<Sync>, rx: Receiver<SyncCommand>) -> Result<()> {
        let mut rx = rx;
        loop {
            select! {
                cmd = rx.recv() => if let Some(cmd) = cmd {
                    s.command(cmd).await?
                }
            }
        }
        Ok(())
    }

    async fn command(&self, cmd: SyncCommand) -> Result<()> {
        match cmd {
            SyncCommand::DbChanged(pos) => self.sync(pos).await?,
            SyncCommand::Snapshot(compressed_data) => {
                println!("recv snapshots: {}", compressed_data.len());
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    // async fn create_snapshot(&self, generation: &str) -> Result<SnapshotInfo> {}

    async fn sync(&self, pos: WalGenerationPos) -> Result<()> {
        info!("replica sync pos: {:?}\n", pos);

        if pos.offset == 0 {
            return Ok(());
        }

        // Create a new snapshot and update the current replica position if
        // the generation on the database has changed.
        let generation = pos.generation.clone();
        if generation != self.position.generation {
            let snapshots = self.client.snapshots(&generation).await?;
            if snapshots.len() == 0 {
                // Create snapshot if no snapshots exist for generation.
                self.db_notifier
                    .send(DbCommand::Snapshot(self.index))
                    .await?;
                return Ok(());
            }
        }

        Ok(())
    }
}
