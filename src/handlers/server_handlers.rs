use crate::game::game_session::{GameSession, MAX_PLAYERS_PER_SESSION};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::sync::{Mutex, broadcast};
use uuid::Uuid;

// MUDANÇA IMPORTANTE:
// Antes: armazenava (GameSession, Sender)
// Agora: armazenamos (Arc<Mutex<GameSession>>, Sender)
// Por quê? Para permitir múltiplas tasks acessarem a mesma sessão simultaneamente.
pub type ServerSessions = Arc<Mutex<HashMap<Uuid, (Arc<Mutex<GameSession>>, Sender<String>)>>>;

pub async fn register_player(
    sessions_arc: &ServerSessions,
    player: String,
) -> Result<(Uuid, Receiver<String>), String> {
    // agora precisamos lockar HashMap -> depois cada session individual
    let mut sessions = sessions_arc.lock().await;

    let session_id: Uuid;
    let receiver: Receiver<String>;
    let mut available_id: Option<Uuid> = None;

    // MUDANÇA: agora cada session é Arc<Mutex<_>>, então temos que lockar ela individualmente
    for (id, (session_arc, _)) in sessions.iter() {
        let session = session_arc.lock().await;
        if session.party.len() < MAX_PLAYERS_PER_SESSION {
            available_id = Some(id.clone());
            break;
        }
    }

    let result = match available_id {
        // MUDANÇA: agora criamos GameSession dentro de Arc<Mutex<_>>
        None => {
            let (sender, _) = broadcast::channel::<String>(128);
            let game_session = Arc::new(Mutex::new(GameSession::new()));
            let id = game_session.lock().await.id;

            // armazena a sessão em Arc<Mutex<_>>
            sessions.insert(id, (game_session.clone(), sender));

            let (session_arc, sender) = sessions.get_mut(&id).unwrap();

            // adiciona jogador numa session agora protegida por Mutex
            match session_arc.lock().await.add_player(player) {
                Ok(_) => {
                    session_id = id;
                    receiver = sender.subscribe();
                    Ok((session_id, receiver))
                }
                Err(err) => Err(err),
            }
        }
        // mesma lógica de antes, mas agora com session_arc.lock()
        Some(id) => {
            let (session_arc, sender) = sessions.get_mut(&id).unwrap();
            let mut session = session_arc.lock().await;

            match session.add_player(player) {
                Ok(_) => {
                    session_id = session.id;
                    receiver = sender.subscribe();
                    Ok((session_id, receiver))
                }
                Err(err) => Err(err),
            }
        }
    };

    drop(sessions);
    result
}

pub async fn remove_player(
    sessions_arc: &ServerSessions,
    username: &String,
    party_id: Option<Uuid>,
) {
    let mut sessions = sessions_arc.lock().await;

    match party_id {
        None => {
            // agora cada sessão precisa ser lockada individualmente
            for (_, (session_arc, _)) in sessions.iter_mut() {
                let mut session = session_arc.lock().await;
                if session.contains(username) {
                    session.remove_player(username);
                    break;
                }
            }
        }
        // o acesso agora é via Arc<Mutex<_>>
        Some(id) => {
            if let Some((session_arc, _)) = sessions.get_mut(&id) {
                session_arc.lock().await.remove_player(username);
            }
        }
    }

    drop(sessions);
}

// basicamente, quantos acertaram e quantos erraram, talvez a implementaçao nao é a melhor but it works
// assim, nao sei onde botar isso, entao coloquei aqui, deve ir no game_session.rs talvez
pub struct RoundResult {
    pub expected: usize, // quantos jogadores deveriam responder
    pub received: usize, // quantas respostas foram recebidas
    pub correct: usize, // quantas respostas corretas
    pub wrong: usize, // quantas respostas erradas
}

// NOVO: calcular maioria a partir de GameSession.answers
pub fn evaluate_round(session: &GameSession) -> RoundResult {
    let received = session.answers.len();
    let correct = session.answers.values().filter(|v| **v).count();
    let wrong = received - correct;

    RoundResult {
        expected: session.party.len(),
        received,
        correct,
        wrong,
    }
}
