use lavende::{LoadResult, Track};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MusicCommand {
    Play {
        req_id: String,
        guild_id: String,
        channel_id: String,
        text_channel_id: Option<String>,
        query: String,
    },
    Pause {
        guild_id: String,
    },
    Stop {
        guild_id: String,
    },
    Resume {
        guild_id: String,
    },
    Skip {
        guild_id: String,
    },
    Queue {
        req_id: String,
        guild_id: String,
    },
    Filter {
        guild_id: String,
        filter_type: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MusicResponse {
    PlayResult { req_id: String, result: LoadResult },
    QueueResult { req_id: String, tracks: Vec<Track>, current: Option<Track> },
    Error { req_id: String, message: String },
}
