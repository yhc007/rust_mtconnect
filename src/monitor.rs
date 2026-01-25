use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use hyper::body::Bytes;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use http_body_util::{BodyExt, Full};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use chrono::Utc;

#[derive(Debug, Clone)]
struct DeviceInfo {
    name: String,
    uuid: String,
    availability: String,
    execution: String,
    controller_mode: String,
    emergency_stop: String,
    program_name: String,
    subprogram_name: String,
    part_count: String,
    spindle_load: String,
    spindle_override: String,
    feed_override: String,
    rapid_override: String,
    feedrate_actual: String,
    x_axis_load: String,
    z_axis_load: String,
}

impl Default for DeviceInfo {
    fn default() -> Self {
        DeviceInfo {
            name: "Unknown".to_string(),
            uuid: "N/A".to_string(),
            availability: "UNAVAILABLE".to_string(),
            execution: "N/A".to_string(),
            controller_mode: "N/A".to_string(),
            emergency_stop: "N/A".to_string(),
            program_name: "N/A".to_string(),
            subprogram_name: "N/A".to_string(),
            part_count: "N/A".to_string(),
            spindle_load: "N/A".to_string(),
            spindle_override: "N/A".to_string(),
            feed_override: "N/A".to_string(),
            rapid_override: "N/A".to_string(),
            feedrate_actual: "N/A".to_string(),
            x_axis_load: "N/A".to_string(),
            z_axis_load: "N/A".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct AgentStatus {
    url: String,
    name: String,
    status: ConnectionStatus,
    last_update: String,
    devices: Vec<DeviceInfo>,
}

#[derive(Debug, Clone, PartialEq)]
enum ConnectionStatus {
    Connected,
    Failed,
    Checking,
}

struct App {
    agents: Arc<RwLock<Vec<AgentStatus>>>,
    selected_agent: usize,
}

impl App {
    fn new() -> Self {
        let remote_agents = vec![
            ("http://192.168.10.5:5000", "HCN6800"),
            ("http://192.168.10.5:5001", "QT350-1"),
            ("http://192.168.10.5:5002", "QT350-2"),
            ("http://192.168.10.5:5003", "QT350-3"),
            ("http://192.168.10.5:5004", "QT350-4"),
            ("http://192.168.10.5:5005", "MNT600-1"),
            ("http://192.168.10.5:5006", "MNT600S-1"),
            ("http://192.168.10.5:5007", "MNT600-2"),
            ("http://192.168.10.5:5008", "MNT600S-2"),
        ];

        let agents = remote_agents
            .iter()
            .map(|(url, name)| AgentStatus {
                url: url.to_string(),
                name: name.to_string(),
                status: ConnectionStatus::Checking,
                last_update: "Never".to_string(),
                devices: Vec::new(),
            })
            .collect();

        App {
            agents: Arc::new(RwLock::new(agents)),
            selected_agent: 0,
        }
    }

    fn next_agent(&mut self) {
        self.selected_agent = (self.selected_agent + 1) % 9;
    }

    fn previous_agent(&mut self) {
        if self.selected_agent == 0 {
            self.selected_agent = 8;
        } else {
            self.selected_agent -= 1;
        }
    }
}

// Kept for potential future use
#[allow(dead_code)]
fn extract_between<'a>(text: &'a str, start_tag: &str, end_tag: &str) -> Option<&'a str> {
    let start = text.find(start_tag)?;
    let end = text[start..].find(end_tag)?;
    Some(&text[start + start_tag.len()..start + end])
}

fn extract_attribute<'a>(text: &'a str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr);
    let start = text.find(&pattern)?;
    let start_pos = start + pattern.len();
    let end = text[start_pos..].find("\"")?;
    Some(text[start_pos..start_pos + end].to_string())
}

// Extract value by finding tag with specific name attribute
fn extract_value_by_name(xml: &str, tag_name: &str, name_attr: &str) -> String {
    let search_pattern = format!("name=\"{}\"", name_attr);

    if let Some(name_pos) = xml.find(&search_pattern) {
        // Search backward to find the opening tag
        if let Some(tag_start) = xml[..name_pos].rfind(&format!("<{} ", tag_name)) {
            // Find the closing tag
            let closing_tag = format!("</{}>", tag_name);
            if let Some(tag_end_pos) = xml[tag_start..].find(&closing_tag) {
                let full_tag = &xml[tag_start..tag_start + tag_end_pos + closing_tag.len()];

                // Find content between > and </
                if let Some(content_start) = full_tag.find('>') {
                    if let Some(content_end) = full_tag.find(&closing_tag) {
                        let content = &full_tag[content_start + 1..content_end];
                        return content.trim().to_string();
                    }
                }
            } else {
                // Self-closing tag (e.g., <Program ... />)
                // Extract from the tag itself if it has content
                if let Some(_tag_end) = xml[tag_start..].find("/>") {
                    // Self-closing tag with no content
                    return String::new();
                }
            }
        }
    }

    "N/A".to_string()
}

fn parse_device_info(xml: &str) -> Vec<DeviceInfo> {
    let mut devices = Vec::new();

    // Find all DeviceStream elements
    let mut search_pos = 0;
    while let Some(device_start) = xml[search_pos..].find("<DeviceStream") {
        let abs_start = search_pos + device_start;
        if let Some(device_end) = xml[abs_start..].find("</DeviceStream>") {
            let device_xml = &xml[abs_start..abs_start + device_end + 15];

            let mut device = DeviceInfo::default();

            // Extract device name and uuid from DeviceStream tag
            if let Some(name_end) = device_xml.find(">") {
                let header = &device_xml[..name_end];
                if let Some(name) = extract_attribute(header, "name") {
                    device.name = name;
                }
                if let Some(uuid) = extract_attribute(header, "uuid") {
                    device.uuid = uuid;
                }
            }

            // Extract data items using the new helper function
            device.availability = extract_value_by_name(device_xml, "Availability", "avail");
            device.execution = extract_value_by_name(device_xml, "Execution", "execution");
            device.controller_mode = extract_value_by_name(device_xml, "ControllerMode", "mode");
            device.emergency_stop = extract_value_by_name(device_xml, "EmergencyStop", "estop");

            // Program - find the one WITHOUT subType="x:SUB"
            device.program_name = extract_value_by_name(device_xml, "Program", "program");
            device.subprogram_name = extract_value_by_name(device_xml, "Program", "subprogram");

            device.part_count = extract_value_by_name(device_xml, "PartCount", "PartCountAct");

            // Spindle and Feed
            device.spindle_load = extract_value_by_name(device_xml, "Load", "Sload");
            device.spindle_override = extract_value_by_name(device_xml, "RotaryVelocityOverride", "Sovr");
            device.feed_override = extract_value_by_name(device_xml, "PathFeedrateOverride", "Fovr");
            device.rapid_override = extract_value_by_name(device_xml, "PathFeedrateOverride", "Frapidovr");
            device.feedrate_actual = extract_value_by_name(device_xml, "PathFeedrate", "Fact");

            // Axis Loads
            device.x_axis_load = extract_value_by_name(device_xml, "Load", "Xload");
            device.z_axis_load = extract_value_by_name(device_xml, "Load", "Zload");

            devices.push(device);
            search_pos = abs_start + device_end + 15;
        } else {
            break;
        }
    }

    devices
}

async fn fetch_agent_data(base_url: String, name: String, agents: Arc<RwLock<Vec<AgentStatus>>>) {
    let client = Client::builder(TokioExecutor::new()).build_http();

    loop {
        // Fetch current data
        let url = format!("{}/current", base_url);

        match url.parse::<hyper::Uri>() {
            Ok(uri) => {
                let req = hyper::Request::builder()
                    .uri(uri)
                    .method(hyper::Method::GET)
                    .body(Full::<Bytes>::default());

                match req {
                    Ok(request) => {
                        match tokio::time::timeout(
                            Duration::from_secs(2),
                            client.request(request)
                        ).await {
                            Ok(Ok(res)) => {
                                if res.status().is_success() {
                                    let body = res.into_body();
                                    match body.collect().await {
                                        Ok(collected) => {
                                            let bytes: Bytes = collected.to_bytes();
                                            match String::from_utf8(bytes.to_vec()) {
                                                Ok(content) => {
                                                    // Parse device information
                                                    let devices = parse_device_info(&content);

                                                    // Update agent status
                                                    let mut agents_lock = agents.write().await;
                                                    if let Some(agent) = agents_lock.iter_mut().find(|a| a.name == name) {
                                                        agent.status = ConnectionStatus::Connected;
                                                        agent.last_update = Utc::now().format("%H:%M:%S").to_string();
                                                        agent.devices = devices;
                                                    }
                                                }
                                                Err(_) => {
                                                    update_failed(&agents, &name).await;
                                                }
                                            }
                                        }
                                        Err(_) => {
                                            update_failed(&agents, &name).await;
                                        }
                                    }
                                } else {
                                    update_failed(&agents, &name).await;
                                }
                            }
                            _ => {
                                update_failed(&agents, &name).await;
                            }
                        }
                    }
                    Err(_) => {
                        update_failed(&agents, &name).await;
                    }
                }
            }
            Err(_) => {
                update_failed(&agents, &name).await;
            }
        }

        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

async fn update_failed(agents: &Arc<RwLock<Vec<AgentStatus>>>, name: &str) {
    let mut agents_lock = agents.write().await;
    if let Some(agent) = agents_lock.iter_mut().find(|a| a.name == name) {
        agent.status = ConnectionStatus::Failed;
        agent.devices.clear();
    }
}

fn get_load_color(load_str: &str) -> Color {
    if load_str == "N/A" || load_str == "UNAVAILABLE" {
        return Color::Gray;
    }

    match load_str.parse::<f32>() {
        Ok(load) => {
            if load >= 80.0 {
                Color::Red
            } else if load >= 50.0 {
                Color::Yellow
            } else {
                Color::Green
            }
        }
        Err(_) => Color::Gray,
    }
}

fn format_percentage(value: &str) -> String {
    if value == "N/A" || value == "UNAVAILABLE" || value.is_empty() {
        "N/A".to_string()
    } else {
        format!("{}%", value)
    }
}

fn format_value(value: &str) -> String {
    if value == "N/A" || value == "UNAVAILABLE" || value.is_empty() {
        "N/A".to_string()
    } else {
        value.to_string()
    }
}

fn ui<B: Backend>(f: &mut Frame, app: &App, agents: &[AgentStatus]) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(12),
            Constraint::Min(20),
        ])
        .split(f.area());

    // Title
    let title = Paragraph::new("MTConnect Agent Monitor - Device Details")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Agent list
    let items: Vec<ListItem> = agents
        .iter()
        .enumerate()
        .map(|(i, agent)| {
            let status_symbol = match agent.status {
                ConnectionStatus::Connected => "●",
                ConnectionStatus::Failed => "✗",
                ConnectionStatus::Checking => "○",
            };
            let status_color = match agent.status {
                ConnectionStatus::Connected => Color::Green,
                ConnectionStatus::Failed => Color::Red,
                ConnectionStatus::Checking => Color::Yellow,
            };

            let content = Line::from(vec![
                Span::styled(
                    format!("{} ", status_symbol),
                    Style::default().fg(status_color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("{:12} ", agent.name)),
                Span::styled(
                    format!("Devices: {:2} ", agent.devices.len()),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!("Updated: {}", agent.last_update),
                    Style::default().fg(Color::Gray),
                ),
            ]);

            let style = if i == app.selected_agent {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Agents (↑/↓ to navigate, q to quit)"));
    f.render_widget(list, chunks[1]);

    // Detail view
    if let Some(selected) = agents.get(app.selected_agent) {
        let mut detail_lines = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(Color::Yellow)),
                Span::raw(&selected.name),
            ]),
            Line::from(vec![
                Span::styled("URL: ", Style::default().fg(Color::Yellow)),
                Span::raw(&selected.url),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{:?}", selected.status),
                    Style::default().fg(match selected.status {
                        ConnectionStatus::Connected => Color::Green,
                        ConnectionStatus::Failed => Color::Red,
                        ConnectionStatus::Checking => Color::Yellow,
                    }),
                ),
            ]),
            Line::from(vec![
                Span::styled("Last Update: ", Style::default().fg(Color::Yellow)),
                Span::raw(&selected.last_update),
            ]),
            Line::from(""),
        ];

        if selected.devices.is_empty() {
            detail_lines.push(Line::from(Span::styled(
                "No device data available",
                Style::default().fg(Color::Red),
            )));
        } else {
            for (idx, device) in selected.devices.iter().enumerate() {
                detail_lines.push(Line::from(Span::styled(
                    format!("═══ Device {} ═══", idx + 1),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )));
                detail_lines.push(Line::from(vec![
                    Span::styled("Name: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&device.name),
                ]));
                detail_lines.push(Line::from(vec![
                    Span::styled("UUID: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&device.uuid),
                ]));
                detail_lines.push(Line::from(vec![
                    Span::styled("Availability: ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        &device.availability,
                        Style::default().fg(if device.availability == "AVAILABLE" {
                            Color::Green
                        } else {
                            Color::Red
                        }),
                    ),
                ]));
                detail_lines.push(Line::from(vec![
                    Span::styled("Execution: ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        &device.execution,
                        Style::default().fg(if device.execution == "ACTIVE" {
                            Color::Green
                        } else if device.execution == "READY" {
                            Color::Cyan
                        } else {
                            Color::Gray
                        }),
                    ),
                ]));
                detail_lines.push(Line::from(vec![
                    Span::styled("Controller Mode: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&device.controller_mode),
                ]));
                detail_lines.push(Line::from(vec![
                    Span::styled("Emergency Stop: ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        &device.emergency_stop,
                        Style::default().fg(if device.emergency_stop == "ARMED" {
                            Color::Green
                        } else {
                            Color::Red
                        }),
                    ),
                ]));
                detail_lines.push(Line::from(vec![
                    Span::styled("Program: ", Style::default().fg(Color::Yellow)),
                    Span::styled(format_value(&device.program_name), Style::default().fg(Color::Cyan)),
                ]));
                detail_lines.push(Line::from(vec![
                    Span::styled("Subprogram: ", Style::default().fg(Color::Yellow)),
                    Span::styled(format_value(&device.subprogram_name), Style::default().fg(Color::Cyan)),
                ]));
                detail_lines.push(Line::from(vec![
                    Span::styled("Part Count: ", Style::default().fg(Color::Yellow)),
                    Span::styled(format_value(&device.part_count), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                ]));
                detail_lines.push(Line::from(""));

                // Spindle & Feed Information
                detail_lines.push(Line::from(Span::styled(
                    "── Spindle & Feed ──",
                    Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                )));
                detail_lines.push(Line::from(vec![
                    Span::styled("Spindle Load: ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format_percentage(&device.spindle_load),
                        Style::default().fg(get_load_color(&device.spindle_load)),
                    ),
                ]));
                detail_lines.push(Line::from(vec![
                    Span::styled("Spindle Override: ", Style::default().fg(Color::Yellow)),
                    Span::styled(format_percentage(&device.spindle_override), Style::default().fg(Color::Cyan)),
                ]));
                detail_lines.push(Line::from(vec![
                    Span::styled("Feed Override: ", Style::default().fg(Color::Yellow)),
                    Span::styled(format_percentage(&device.feed_override), Style::default().fg(Color::Cyan)),
                ]));
                detail_lines.push(Line::from(vec![
                    Span::styled("Rapid Override: ", Style::default().fg(Color::Yellow)),
                    Span::styled(format_percentage(&device.rapid_override), Style::default().fg(Color::Cyan)),
                ]));
                detail_lines.push(Line::from(vec![
                    Span::styled("Feedrate (Actual): ", Style::default().fg(Color::Yellow)),
                    Span::styled(format_value(&device.feedrate_actual), Style::default().fg(Color::White)),
                ]));
                detail_lines.push(Line::from(""));

                // Axis Loads
                detail_lines.push(Line::from(Span::styled(
                    "── Axis Loads ──",
                    Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                )));
                detail_lines.push(Line::from(vec![
                    Span::styled("X-Axis Load: ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format_percentage(&device.x_axis_load),
                        Style::default().fg(get_load_color(&device.x_axis_load)),
                    ),
                ]));
                detail_lines.push(Line::from(vec![
                    Span::styled("Z-Axis Load: ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format_percentage(&device.z_axis_load),
                        Style::default().fg(get_load_color(&device.z_axis_load)),
                    ),
                ]));
                detail_lines.push(Line::from(""));
            }
        }

        let detail = Paragraph::new(detail_lines)
            .block(Block::default().borders(Borders::ALL).title("Device Details"))
            .wrap(Wrap { trim: false });
        f.render_widget(detail, chunks[2]);
    }
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> io::Result<()> {
    // Start background tasks to fetch data
    let agents_clone = Arc::clone(&app.agents);
    let agents_for_fetch = agents_clone.read().await.clone();

    for agent in agents_for_fetch {
        let agents_clone = Arc::clone(&app.agents);
        tokio::spawn(async move {
            fetch_agent_data(agent.url, agent.name, agents_clone).await;
        });
    }

    loop {
        let agents = app.agents.read().await.clone();
        terminal.draw(|f| ui::<B>(f, &app, &agents))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Down => app.next_agent(),
                    KeyCode::Up => app.previous_agent(),
                    _ => {}
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let app = App::new();
    let res = run_app(&mut terminal, app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}
