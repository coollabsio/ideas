use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: Uuid,
    pub github_id: i64,
    pub login: String,
    pub avatar_url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicUser {
    pub login: String,
    pub avatar_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IdeaStatus {
    Open,
    InProgress,
    Closed,
}

impl IdeaStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "inprogress",
            Self::Closed => "closed",
        }
    }

    pub fn from_db(value: &str) -> Self {
        match value {
            "inprogress" => Self::InProgress,
            "closed" => Self::Closed,
            _ => Self::Open,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Idea {
    pub id: Uuid,
    pub title: String,
    pub body_text: String,
    pub problem: String,
    pub upvote_count: i64,
    pub comment_count: i64,
    pub viewer_has_upvoted: bool,
    pub viewer_can_edit: bool,
    pub viewer_can_delete: bool,
    pub viewer_can_close: bool,
    pub author: PublicUser,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: IdeaStatus,
    pub closed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: Uuid,
    pub idea_id: Uuid,
    pub body_text: String,
    pub upvote_count: i64,
    pub viewer_has_upvoted: bool,
    pub viewer_can_edit: bool,
    pub viewer_can_delete: bool,
    pub author: PublicUser,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub sid: String,
    pub user_id: Uuid,
    pub csrf_token: String,
    pub expires_at: i64,
    pub user: User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewIdea {
    pub title: String,
    pub body: String,
    pub problem: String,
}
