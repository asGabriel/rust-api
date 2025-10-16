use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum WorkerTopic {
    DebtCreated,
}

#[derive(Debug, Clone)]
pub struct WorkerMessage {
    pub topic: WorkerTopic,
    pub payload: String,
    pub metadata: Option<serde_json::Value>,
}

pub struct WorkerState {
    pub db: Pool<Postgres>,
    pub sender: mpsc::UnboundedSender<WorkerMessage>,
    receiver: Option<mpsc::UnboundedReceiver<WorkerMessage>>,
}

impl WorkerState {
    pub fn new(db: Pool<Postgres>) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        Self {
            db,
            sender,
            receiver: Some(receiver),
        }
    }

    pub fn start(mut self) -> Arc<Self> {
        println!("✅ Worker iniciado!");

        let mut receiver = self.receiver.take().expect("Receiver já foi usado");

        tokio::spawn(async move {
            loop {
                if let Some(message) = receiver.recv().await {
                    println!("📨 Mensagem recebida: {:?}", message.topic);
                    println!("📝 Payload: {}", message.payload);

                    // Processa a mensagem baseado no tópico
                    match message.topic {
                        WorkerTopic::DebtCreated => {
                            println!("🔄 Processando dívida criada");

                            if let Some(metadata) = message.metadata {
                                println!("📊 Metadados: {}", metadata);

                                // Aqui você pode:
                                // 1. Deserializar os dados da dívida
                                // 2. Enviar para o chatbot
                                // 3. Enviar email
                                // 4. Qualquer outra ação necessária

                                // Exemplo:
                                // if let Ok(debt) = serde_json::from_value::<Debt>(metadata) {
                                //     chatbot_state.send_message(
                                //         format!("Nova dívida: R$ {}", debt.amount)
                                //     ).await;
                                // }
                            }

                            println!("✅ Dívida processada com sucesso");
                        }
                    }

                    // Simula processamento
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                } else {
                    println!("❌ Canal de mensagens foi fechado");
                    break;
                }
            }
        });

        Arc::new(self)
    }

    pub fn notify(&self, topic: WorkerTopic, message: String, metadata: Option<serde_json::Value>) {
        let worker_message = WorkerMessage {
            topic,
            payload: message,
            metadata,
        };

        let _ = self.sender.send(worker_message);
    }
}
