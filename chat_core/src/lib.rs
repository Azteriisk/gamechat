use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserStatus {
    Online,
    Idle,
    DoNotDisturb,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub status: UserStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RoomType {
    Direct,
    Group,
    Public,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub topic: Option<String>,
    pub room_type: RoomType,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    Text,
    Image,
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub sender: String,
    pub content: String,
    pub schema: MessageType,
    pub timestamp: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_serialization() {
        let user = User {
            id: "user123".to_string(),
            display_name: "Rustacean".to_string(),
            avatar_url: None,
            status: UserStatus::Online,
        };

        let json = serde_json::to_string(&user).unwrap();
        let deserialized: User = serde_json::from_str(&json).unwrap();

        assert_eq!(user.id, deserialized.id);
        assert_eq!(user.status, deserialized.status);
    }

    #[test]
    fn test_message_serialization() {
        let message = Message {
            id: "msg1".to_string(),
            sender: "user123".to_string(),
            content: "Hello World".to_string(),
            schema: MessageType::Text,
            timestamp: 1678888888,
        };

        let json = serde_json::to_string(&message).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();

        assert_eq!(message.content, deserialized.content);
        assert_eq!(message.schema, deserialized.schema);
    }
}
