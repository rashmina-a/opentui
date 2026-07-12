use crate::chat::{ChatState, Message};
use crate::config::ProviderType;
use crate::dev_mode::{format_metrics_line, Metrics};
use crate::ui::colors::*;
use crate::ui::{format_timestamp, sanitize_text};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

/// Maximum width for chat bubbles (percentage of available width)
const BUBBLE_MAX_WIDTH_PCT: u16 = 80;

/// Render the main chat screen
pub fn render_chat_screen(
    frame: &mut Frame,
    chat: &ChatState,
    current_provider: &ProviderType,
    current_model: &str,
    dev_mode: bool,
    last_metrics: &Option<Metrics>,
    is_streaming: bool,
) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),    // Top bar
            Constraint::Min(1),       // Messages area
            Constraint::Length(if dev_mode { 3 } else { 0 }), // Dev mode bar
            Constraint::Length(4),    // Input area
        ])
        .split(area);

    // ── Top bar ──
    render_top_bar(frame, chunks[0], current_provider, current_model, dev_mode);

    // ── Messages area ──
    render_messages(frame, chunks[1], chat, is_streaming);

    // ── Developer mode bar ──
    if dev_mode {
        render_dev_mode_bar(frame, chunks[2], last_metrics);
    }

    // ── Input area ──
    render_input_area(frame, chunks[3], chat, is_streaming);
}

fn render_top_bar(
    frame: &mut Frame,
    area: Rect,
    provider: &ProviderType,
    model: &str,
    dev_mode: bool,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(SAPPHIRE))
        .style(Style::default().bg(BASE));

    let title = format!("  🧠 OpenTUI  ");
    let provider_info = format!(
        " {} › {} {}",
        provider.display_name(),
        model,
        if dev_mode { " ⎆ DEV" } else { "" }
    );
    let keybind_hint = "  ⌨ /help  ";

    let inner = block.inner(area);
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(title.len() as u16 + 2),
            Constraint::Min(1),
            Constraint::Length(provider_info.len() as u16 + 2),
            Constraint::Length(keybind_hint.len() as u16 + 2),
        ])
        .split(inner);

    // App title
    let title_paragraph = Paragraph::new(Line::from(vec![
        Span::styled("🧠 ", Style::default().fg(TEAL)),
        Span::styled("OpenTUI", Style::default().fg(MAUVE).add_modifier(Modifier::BOLD)),
    ]))
    .style(Style::default().bg(BASE));

    frame.render_widget(block, area);
    frame.render_widget(title_paragraph, chunks[0]);

    // Provider info
    let provider_style = if dev_mode {
        Style::default().fg(YELLOW).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(GREEN)
    };

    let provider_paragraph = Paragraph::new(Line::from(vec![
        Span::styled("🔌 ", Style::default().fg(PEACH)),
        Span::styled(provider.display_name().to_string(), provider_style),
        Span::styled(" / ", Style::default().fg(OVERLAY_1)),
        Span::styled(model.to_string(), Style::default().fg(TEXT)),
        if dev_mode {
            Span::styled(" ⚙ DEV", Style::default().fg(YELLOW).add_modifier(Modifier::BOLD))
        } else {
            Span::raw("")
        },
    ]))
    .style(Style::default().bg(BASE));

    frame.render_widget(provider_paragraph, chunks[2]);

    // Keybind hint
    let hint_paragraph = Paragraph::new(Line::from(vec![
        Span::styled("⌨ ", Style::default().fg(OVERLAY_1)),
        Span::styled("Ctrl+S", Style::default().fg(SUBTEXT_0)),
        Span::styled(" settings  ", Style::default().fg(OVERLAY_1)),
        Span::styled("Ctrl+Q", Style::default().fg(SUBTEXT_0)),
        Span::styled(" quit", Style::default().fg(OVERLAY_1)),
    ]))
    .style(Style::default().bg(BASE))
    .alignment(Alignment::Right);

    frame.render_widget(hint_paragraph, chunks[3]);
}

fn render_messages(frame: &mut Frame, area: Rect, chat: &ChatState, is_streaming: bool) {
    let messages_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(SURFACE_2))
        .style(Style::default().bg(MANTLE));

    let inner_area = messages_block.inner(area);
    frame.render_widget(messages_block, area);

    if chat.conversation.messages.is_empty() && chat.streaming_content.is_empty() {
        // Welcome message
        let welcome = Text::from(vec![
            Line::from(vec![
                Span::styled("  🚀 ", Style::default().fg(TEAL)),
                Span::styled(
                    "Welcome to OpenTUI!",
                    Style::default().fg(MAUVE).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(
                "  Your terminal AI chat companion",
                Style::default().fg(SUBTEXT_0),
            )),
            Line::from(String::new()),
            Line::from(Span::styled(
                "  Type your message below and press Enter to send",
                Style::default().fg(OVERLAY_1),
            )),
            Line::from(Span::styled(
                "  Press Ctrl+S to open settings",
                Style::default().fg(OVERLAY_1),
            )),
        ])
        .alignment(Alignment::Center);

        let welcome_para = Paragraph::new(welcome)
            .style(Style::default().bg(MANTLE))
            .alignment(Alignment::Center);
        frame.render_widget(welcome_para, inner_area);
        return;
    }

    // Build message list
    let mut items: Vec<ListItem> = Vec::new();

    for msg in &chat.conversation.messages {
        let is_user = msg.role == "user";
        items.extend(render_message_bubble(msg, is_user, BUBBLE_MAX_WIDTH_PCT));
        items.push(ListItem::new(Line::from(Span::raw(""))));
    }

    // Show streaming content if available
    if !chat.streaming_content.is_empty() {
        let stream_bubbles = render_message_bubble_for_content(
            &chat.streaming_content,
            false,
        );
        items.extend(stream_bubbles);

        if is_streaming {
            items.push(ListItem::new(Line::from(vec![
                Span::styled(" ⚡", Style::default().fg(YELLOW)),
                Span::styled(" generating", Style::default().fg(YELLOW).add_modifier(Modifier::SLOW_BLINK)),
            ])));
        }
    }

    // Show error if any
    if let Some(ref error) = chat.error_message {
        items.push(ListItem::new(Line::from(vec![
            Span::styled(" ✗ ", Style::default().fg(RED)),
            Span::styled(error.clone(), Style::default().fg(RED)),
        ])));
    }

    let messages_list = List::new(items)
        .style(Style::default().bg(MANTLE));

    frame.render_widget(messages_list, inner_area);
}

fn render_message_bubble_for_content(content: &str, is_user: bool) -> Vec<ListItem<'_>> {
    let mut items = Vec::new();

    let role_style = if is_user {
        Style::default().fg(BLUE).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
    };

    let role_icon = if is_user { "👤" } else { "🤖" };
    let role_label = if is_user { "You" } else { "Assistant" };

    items.push(ListItem::new(Line::from(vec![
        Span::raw(" "),
        Span::styled(format!("{} {} ", role_icon, role_label), role_style),
    ])));

    let clean_content = sanitize_text(content);
    for line in clean_content.lines() {
        let content_style = if is_user {
            Style::default().fg(TEXT)
        } else {
            Style::default().fg(SUBTEXT_1)
        };
        items.push(ListItem::new(Line::from(vec![
            Span::styled("  ", content_style),
            Span::styled(line.to_string(), content_style),
        ])));
    }

    items
}

fn render_message_bubble(msg: &Message, is_user: bool, _max_width_pct: u16) -> Vec<ListItem<'_>> {
    let mut items = Vec::new();

    // Role label
    let role_style = if is_user {
        Style::default().fg(BLUE).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
    };

    let role_icon = if is_user { "👤" } else { "🤖" };
    let role_label = if is_user { "You" } else { "Assistant" };

    items.push(ListItem::new(Line::from(vec![
        Span::raw(" "),
        Span::styled(format!("{} {} ", role_icon, role_label), role_style),
        Span::styled(format_timestamp(&msg.timestamp), Style::default().fg(OVERLAY_1)),
    ])));

    // Message content with word wrapping (sanitized)
    let clean_content = sanitize_text(&msg.content);
    for line in clean_content.lines() {
        let content_style = if is_user {
            Style::default().fg(TEXT)
        } else {
            Style::default().fg(SUBTEXT_1)
        };
        items.push(ListItem::new(Line::from(vec![
            Span::styled("  ", content_style),
            Span::styled(line.to_string(), content_style),
        ])));
    }

    items
}

fn render_dev_mode_bar(frame: &mut Frame, area: Rect, last_metrics: &Option<Metrics>) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(YELLOW))
        .style(Style::default().bg(CRUST))
        .title(Line::from(vec![
            Span::styled(" ⚙ ", Style::default().fg(YELLOW)),
            Span::styled("Developer Mode", Style::default().fg(YELLOW).add_modifier(Modifier::BOLD)),
        ]))
        .title_alignment(Alignment::Left);

    let inner = block.inner(area);

    let metrics_text = if let Some(ref metrics) = last_metrics {
        format_metrics_line(metrics)
    } else {
        "Waiting for response...".to_string()
    };

    let metrics_paragraph = Paragraph::new(Line::from(vec![
        Span::styled(" ", Style::default().fg(OVERLAY_0)),
        Span::styled(metrics_text, Style::default().fg(TEAL)),
    ]))
    .style(Style::default().bg(CRUST));

    frame.render_widget(block, area);
    frame.render_widget(metrics_paragraph, inner);
}

fn render_input_area(frame: &mut Frame, area: Rect, chat: &ChatState, is_streaming: bool) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if is_streaming {
            Style::default().fg(YELLOW)
        } else if chat.input.is_empty() {
            Style::default().fg(SURFACE_2)
        } else {
            Style::default().fg(BLUE)
        })
        .style(Style::default().bg(BASE))
        .title(Line::from(vec![
            Span::styled(" 💬 ", Style::default().fg(TEAL)),
            Span::styled("Message", Style::default().fg(SUBTEXT_0)),
        ]))
        .title_alignment(Alignment::Left);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let cursor_visible = !is_streaming;

    if chat.input.is_empty() && !is_streaming {
        let placeholder = Paragraph::new(Line::from(vec![
            Span::styled(" Type your message here...", Style::default().fg(OVERLAY_1)),
        ]))
        .style(Style::default().bg(BASE));
        frame.render_widget(placeholder, inner);
    } else if chat.input.is_empty() && is_streaming {
        let streaming_indicator = Paragraph::new(Line::from(vec![
            Span::styled(" ⏳ Waiting for response...", Style::default().fg(YELLOW)),
        ]))
        .style(Style::default().bg(BASE));
        frame.render_widget(streaming_indicator, inner);
    } else {
        let input_paragraph = Paragraph::new(Line::from(vec![
            Span::styled(&chat.input, Style::default().fg(TEXT)),
        ]))
        .style(Style::default().bg(BASE));
        frame.render_widget(input_paragraph, inner);

        // Set cursor position
        if cursor_visible {
            frame.set_cursor_position((
                inner.x + (chat.cursor_position as u16).min(inner.width.saturating_sub(1)),
                inner.y,
            ));
        }
    }

    // Send hint
    if !chat.input.is_empty() && !is_streaming {
        let hint = Paragraph::new(Line::from(vec![
            Span::styled(" ⏎ Send", Style::default().fg(SUBTEXT_0)),
            Span::styled("  ⎋ Cancel", Style::default().fg(SUBTEXT_0)),
        ]))
        .style(Style::default().bg(BASE))
        .alignment(Alignment::Right);
        let hint_area = Rect::new(
            area.x + area.width.saturating_sub(18),
            area.y + area.height.saturating_sub(1),
            18,
            1,
        );
        frame.render_widget(hint, hint_area);
    }
}
