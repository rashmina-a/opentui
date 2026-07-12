use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// A single message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

impl Message {
    pub fn new(role: &str, content: &str) -> Self {
        Self {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: Local::now().format("%H:%M:%S").to_string(),
        }
    }

    pub fn user(content: &str) -> Self {
        Self::new("user", content)
    }

    pub fn assistant(content: &str) -> Self {
        Self::new("assistant", content)
    }

    pub fn system(content: &str) -> Self {
        Self::new("system", content)
    }
}

/// A complete conversation
#[derive(Debug, Clone)]
pub struct Conversation {
    pub messages: VecDeque<Message>,
    pub system_prompt: Option<String>,
    pub max_messages: usize,
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            messages: VecDeque::new(),
            system_prompt: None,
            max_messages: 100,
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push_back(message);
        if self.messages.len() > self.max_messages {
            self.messages.pop_front();
        }
    }

    pub fn get_messages(&self) -> Vec<Message> {
        let mut msgs = Vec::new();

        // Add system prompt as first message if present
        if let Some(system) = &self.system_prompt {
            msgs.push(Message::system(system));
        }

        msgs.extend(self.messages.iter().cloned());
        msgs
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

impl Default for Conversation {
    fn default() -> Self {
        Self::new()
    }
}

/// State of the chat interface
#[derive(Debug)]
pub struct ChatState {
    pub conversation: Conversation,
    pub input: String,
    pub cursor_position: usize,
    pub streaming_content: String,
    pub error_message: Option<String>,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            conversation: Conversation::new(),
            input: String::new(),
            cursor_position: 0,
            streaming_content: String::new(),
            error_message: None,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_position, c);
        self.cursor_position += c.len_utf8();
    }

    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 && !self.input.is_empty() {
            let prev = self.input[..self.cursor_position]
                .char_indices()
                .next_back()
                .map(|(i, _c)| (i, _c.len_utf8()))
                .unwrap_or((0, 0));
            self.input.drain(prev.0..self.cursor_position);
            self.cursor_position = prev.0;
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            let prev = self.input[..self.cursor_position]
                .char_indices()
                .next_back()
                .map(|(i, _c)| i)
                .unwrap_or(0);
            self.cursor_position = prev;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.input.len() {
            let next = self.input[self.cursor_position..]
                .char_indices()
                .nth(1)
                .map(|(i, _c)| self.cursor_position + i)
                .unwrap_or(self.input.len());
            self.cursor_position = next;
        }
    }

    pub fn clear_input(&mut self) {
        self.input.clear();
        self.cursor_position = 0;
    }

    /// Submit the current input as a user message
    pub fn submit(&mut self) -> Option<String> {
        let input = self.input.trim().to_string();
        if input.is_empty() {
            return None;
        }
        self.clear_input();
        Some(input)
    }
}
