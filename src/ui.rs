use crate::app::{ActiveTab, App, SortBy};
use chrono::Local;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, Gauge, Paragraph, Row, Sparkline, Table, Tabs,
    },
    Frame,
};

pub fn render(app: &mut App, frame: &mut Frame) {
    let size = frame.area();

    // Base background layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header & Tabs
            Constraint::Min(10),   // Main Content
            Constraint::Length(2), // Status & Keymap Footer
        ])
        .split(size);

    render_header(app, frame, chunks[0]);

    match app.active_tab {
        ActiveTab::Overview => render_overview(app, frame, chunks[1]),
        ActiveTab::Processes => render_processes(app, frame, chunks[1]),
        ActiveTab::StorageNetwork => render_storage_network(app, frame, chunks[1]),
        ActiveTab::Help => render_help(app, frame, chunks[1]),
    }

    render_footer(app, frame, chunks[2]);

    // Popup modal if kill confirm is active
    if app.show_kill_confirm {
        render_kill_modal(app, frame, size);
    }
}

fn render_header(app: &App, frame: &mut Frame, area: Rect) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(22), // Title / Brand
            Constraint::Min(35),    // Navigation Tabs
            Constraint::Length(35), // Time & Quick info
        ])
        .split(area);

    // Title
    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));
    let title_text = Paragraph::new(Line::from(vec![
        Span::styled(" ⚡ devtop ", Style::default().fg(Color::Cyan).bold()),
        Span::styled("v0.1.0", Style::default().fg(Color::DarkGray)),
    ]))
    .block(title_block);
    frame.render_widget(title_text, header_chunks[0]);

    // Tabs
    let tab_titles = vec![
        Line::from(vec![Span::raw("1: "), Span::raw("Overview")]),
        Line::from(vec![Span::raw("2: "), Span::raw("Processes")]),
        Line::from(vec![Span::raw("3: "), Span::raw("Storage & Net")]),
        Line::from(vec![Span::raw("4: "), Span::raw("Help")]),
    ];
    let active_index = app.active_tab as usize;
    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .select(active_index)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
    frame.render_widget(tabs, header_chunks[1]);

    // System Quick Clock & Host
    let now = Local::now().format("%H:%M:%S").to_string();
    let uptime_secs = sysinfo::System::uptime();
    let hours = uptime_secs / 3600;
    let mins = (uptime_secs % 3600) / 60;
    let right_info = Paragraph::new(Line::from(vec![
        Span::styled(format!(" ⏱ {} ", now), Style::default().fg(Color::Yellow)),
        Span::styled(
            format!("up {}:{:02} ", hours, mins),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            format!("⚡{}ms ", app.refresh_rate_ms),
            Style::default().fg(Color::Green),
        ),
    ]))
    .alignment(Alignment::Right)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Blue)),
    );
    frame.render_widget(right_info, header_chunks[2]);
}

fn render_overview(app: &App, frame: &mut Frame, area: Rect) {
    let main_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // System Info Bar
            Constraint::Length(9), // CPU & Memory Visualizers
            Constraint::Min(8),    // Quick Top Processes & Storage Preview
        ])
        .split(area);

    // 1. System Info Box
    let sys_box = Block::default()
        .title(" 💻 System Information ")
        .title_style(Style::default().fg(Color::Cyan).bold())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let info_text = vec![
        Line::from(vec![
            Span::styled(" Hostname: ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(&app.stats.hostname),
            Span::styled("  OS: ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(format!("{} {}", app.stats.os_name, app.stats.os_version)),
            Span::styled("  Kernel: ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(&app.stats.kernel_version),
        ]),
        Line::from(vec![
            Span::styled(" CPU Model: ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(&app.stats.cpu_model),
            Span::styled("  Cores: ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(format!("{}", app.stats.core_count)),
            Span::styled("  Total Tasks: ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(format!("{}", app.stats.sys.processes().len())),
        ]),
    ];
    frame.render_widget(Paragraph::new(info_text).block(sys_box), main_rows[0]);

    // 2. CPU & Memory Panels
    let metrics_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_rows[1]);

    // CPU Box
    let cpu_percent = app.stats.sys.global_cpu_usage() as u16;
    let cpu_block = Block::default()
        .title(format!(" ⚙ CPU Usage ({}%) ", cpu_percent))
        .title_style(Style::default().fg(Color::LightGreen).bold())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Green));

    let cpu_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(3)])
        .margin(1)
        .split(metrics_cols[0]);

    let cpu_color = if cpu_percent > 85 {
        Color::Red
    } else if cpu_percent > 60 {
        Color::Yellow
    } else {
        Color::Green
    };

    let cpu_gauge = Gauge::default()
        .gauge_style(Style::default().fg(cpu_color).bg(Color::DarkGray))
        .percent(cpu_percent.min(100))
        .label(format!("{}%", cpu_percent));
    frame.render_widget(cpu_block, metrics_cols[0]);
    frame.render_widget(cpu_gauge, cpu_layout[0]);

    let cpu_history_slice: Vec<u64> = app.stats.cpu_history.iter().copied().collect();
    let cpu_sparkline = Sparkline::default()
        .data(&cpu_history_slice)
        .style(Style::default().fg(cpu_color))
        .max(100);
    frame.render_widget(cpu_sparkline, cpu_layout[1]);

    // Memory Box
    let total_ram_gb = app.stats.sys.total_memory() as f64 / 1_073_741_824.0;
    let used_ram_gb = app.stats.sys.used_memory() as f64 / 1_073_741_824.0;
    let ram_percent = if total_ram_gb > 0.0 {
        ((used_ram_gb / total_ram_gb) * 100.0) as u16
    } else {
        0
    };

    let mem_block = Block::default()
        .title(format!(
            " 🧠 Memory: {:.2} GB / {:.2} GB ({}%) ",
            used_ram_gb,
            total_ram_gb,
            ram_percent
        ))
        .title_style(Style::default().fg(Color::Magenta).bold())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Magenta));

    let mem_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(3)])
        .margin(1)
        .split(metrics_cols[1]);

    let mem_color = if ram_percent > 85 {
        Color::Red
    } else if ram_percent > 65 {
        Color::Yellow
    } else {
        Color::Magenta
    };

    let mem_gauge = Gauge::default()
        .gauge_style(Style::default().fg(mem_color).bg(Color::DarkGray))
        .percent(ram_percent.min(100))
        .label(format!("{:.1}GB ({})%", used_ram_gb, ram_percent));
    frame.render_widget(mem_block, metrics_cols[1]);
    frame.render_widget(mem_gauge, mem_layout[0]);

    let mem_history_slice: Vec<u64> = app.stats.memory_history.iter().copied().collect();
    let mem_sparkline = Sparkline::default()
        .data(&mem_history_slice)
        .style(Style::default().fg(mem_color))
        .max(100);
    frame.render_widget(mem_sparkline, mem_layout[1]);

    // 3. Quick Process Preview
    let bottom_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(main_rows[2]);

    // Top CPU processes table preview
    let procs = app.filtered_sorted_processes();
    let top_procs = procs.iter().take(6);
    let rows = top_procs.map(|p| {
        Row::new(vec![
            Cell::from(p.pid.to_string()).style(Style::default().fg(Color::Cyan)),
            Cell::from(p.name.clone()).style(Style::default().bold()),
            Cell::from(format!("{:.1}%", p.cpu_usage))
                .style(Style::default().fg(if p.cpu_usage > 50.0 {
                    Color::Red
                } else if p.cpu_usage > 10.0 {
                    Color::Yellow
                } else {
                    Color::Green
                })),
            Cell::from(format!("{:.1} MB", p.memory_bytes as f64 / 1_048_576.0)),
        ])
    });

    let proc_table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Min(15),
            Constraint::Length(10),
            Constraint::Length(12),
        ],
    )
    .header(
        Row::new(vec!["PID", "Name", "CPU %", "Memory"])
            .style(Style::default().fg(Color::Yellow).bold())
            .bottom_margin(1),
    )
    .block(
        Block::default()
            .title(" 🔥 Top Active Processes (Press '2' for full manager) ")
            .title_style(Style::default().fg(Color::Yellow).bold())
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(proc_table, bottom_cols[0]);

    // Network & Disk Quick Overview
    let rx_kb = app.stats.current_rx_speed / 1024;
    let tx_kb = app.stats.current_tx_speed / 1024;
    let net_box = Block::default()
        .title(" 🌐 Network Activity & Storage ")
        .title_style(Style::default().fg(Color::Cyan).bold())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let mut net_lines = vec![
        Line::from(vec![
            Span::styled(" ▼ Download: ", Style::default().fg(Color::Green).bold()),
            Span::raw(format!("{} KB/s", rx_kb)),
            Span::styled("   ▲ Upload: ", Style::default().fg(Color::LightBlue).bold()),
            Span::raw(format!("{} KB/s", tx_kb)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " Mounted Disks: ",
            Style::default().fg(Color::Yellow).bold(),
        )),
    ];

    for disk in app.stats.get_disks().iter().take(3) {
        let total_gb = disk.total_space as f64 / 1_073_741_824.0;
        let free_gb = disk.available_space as f64 / 1_073_741_824.0;
        let used_pct = if total_gb > 0.0 {
            ((total_gb - free_gb) / total_gb) * 100.0
        } else {
            0.0
        };
        net_lines.push(Line::from(vec![
            Span::styled(format!("  {} ", disk.mount_point), Style::default().bold()),
            Span::raw(format!(
                "({:.1}/{:.1} GB - {:.0}% used)",
                total_gb - free_gb,
                total_gb,
                used_pct
            )),
        ]));
    }

    frame.render_widget(Paragraph::new(net_lines).block(net_box), bottom_cols[1]);
}

fn render_processes(app: &mut App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search & Controls Filter Bar
            Constraint::Min(5),    // Process Table
        ])
        .split(area);

    // Search bar / controls
    let search_style = if app.search_mode {
        Style::default().fg(Color::Yellow).bg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let search_content = if app.search_mode {
        format!(" 🔍 Filter (type to search, Enter/Esc to finish): {}█", app.search_query)
    } else if !app.search_query.is_empty() {
        format!(" 🔍 Filter active: '{}' (Press '/' to edit, Esc to clear)", app.search_query)
    } else {
        " 🔍 Press '/' to filter processes by name, PID, or command".to_string()
    };

    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if app.search_mode {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });
    let search_p = Paragraph::new(search_content).style(search_style).block(search_block);
    frame.render_widget(search_p, chunks[0]);

    // Process list
    let procs = app.filtered_sorted_processes();
    let total_count = procs.len();

    // Ensure selected_process is within range
    if total_count > 0 && app.selected_process >= total_count {
        app.selected_process = total_count - 1;
    }

    // Sort headers with arrows
    let cpu_header = match app.sort_by {
        SortBy::Cpu => if app.sort_ascending { "CPU % ▲" } else { "CPU % ▼" },
        _ => "CPU %",
    };
    let mem_header = match app.sort_by {
        SortBy::Memory => if app.sort_ascending { "MEM % ▲" } else { "MEM % ▼" },
        _ => "MEM %",
    };
    let pid_header = match app.sort_by {
        SortBy::Pid => if app.sort_ascending { "PID ▲" } else { "PID ▼" },
        _ => "PID",
    };
    let name_header = match app.sort_by {
        SortBy::Name => if app.sort_ascending { "Name ▲" } else { "Name ▼" },
        _ => "Name",
    };

    let header_row = Row::new(vec![
        Cell::from(pid_header),
        Cell::from(name_header),
        Cell::from(cpu_header),
        Cell::from("Memory"),
        Cell::from(mem_header),
        Cell::from("Status"),
        Cell::from("Command"),
    ])
    .style(Style::default().fg(Color::Cyan).bold())
    .bottom_margin(1);

    // Calculate viewport window for table scrolling
    let table_height = chunks[1].height.saturating_sub(4) as usize;
    let selected_index = app.selected_process;

    let start_idx = if selected_index >= app.scroll_offset + table_height {
        selected_index - table_height + 1
    } else if selected_index < app.scroll_offset {
        selected_index
    } else {
        app.scroll_offset
    };
    app.scroll_offset = start_idx;

    let rows: Vec<Row> = procs
        .iter()
        .enumerate()
        .skip(start_idx)
        .take(table_height.max(1))
        .map(|(idx, p)| {
            let is_selected = idx == selected_index;
            let mem_mb = p.memory_bytes as f64 / 1_048_576.0;

            let row_style = if is_selected {
                Style::default()
                    .bg(Color::Rgb(30, 60, 100))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let cpu_color = if p.cpu_usage > 50.0 {
                Color::LightRed
            } else if p.cpu_usage > 15.0 {
                Color::Yellow
            } else {
                Color::Green
            };

            let prefix = if is_selected { "▶ " } else { "  " };

            Row::new(vec![
                Cell::from(format!("{}{}", prefix, p.pid)),
                Cell::from(p.name.clone()),
                Cell::from(format!("{:.1}%", p.cpu_usage)).style(Style::default().fg(cpu_color)),
                Cell::from(format!("{:.1} MB", mem_mb)),
                Cell::from(format!("{:.1}%", p.memory_percent)),
                Cell::from(p.status.clone()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(p.cmd.clone()),
            ])
            .style(row_style)
        })
        .collect();

    let table_title = format!(
        " 📋 Process Manager ({} processes) | [s] Cycle Sort [c/m/p/n] Quick Sort | [k] Kill | [↑/↓] Select ",
        total_count
    );

    let table = Table::new(
        rows,
        [
            Constraint::Length(10), // PID
            Constraint::Length(22), // Name
            Constraint::Length(10), // CPU
            Constraint::Length(12), // Memory
            Constraint::Length(10), // MEM %
            Constraint::Length(12), // Status
            Constraint::Min(20),    // Command
        ],
    )
    .header(header_row)
    .block(
        Block::default()
            .title(table_title)
            .title_style(Style::default().fg(Color::Yellow).bold())
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    frame.render_widget(table, chunks[1]);
}

fn render_storage_network(app: &App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // Storage Details
            Constraint::Percentage(50), // Network Interfaces
        ])
        .split(area);

    // 1. Storage Disks
    let disks = app.stats.get_disks();
    let disk_rows: Vec<Row> = disks
        .iter()
        .map(|disk| {
            let total_gb = disk.total_space as f64 / 1_073_741_824.0;
            let free_gb = disk.available_space as f64 / 1_073_741_824.0;
            let used_gb = total_gb - free_gb;
            let used_pct = if total_gb > 0.0 {
                (used_gb / total_gb) * 100.0
            } else {
                0.0
            };

            let color = if used_pct > 90.0 {
                Color::Red
            } else if used_pct > 75.0 {
                Color::Yellow
            } else {
                Color::Green
            };

            Row::new(vec![
                Cell::from(disk.name.clone()).style(Style::default().bold()),
                Cell::from(disk.mount_point.clone()),
                Cell::from(disk.file_system.clone()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(format!("{:.2} GB", total_gb)),
                Cell::from(format!("{:.2} GB", used_gb)),
                Cell::from(format!("{:.2} GB", free_gb)),
                Cell::from(format!("{:.1}%", used_pct)).style(Style::default().fg(color).bold()),
            ])
        })
        .collect();

    let storage_table = Table::new(
        disk_rows,
        [
            Constraint::Length(16),
            Constraint::Length(16),
            Constraint::Length(12),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(10),
        ],
    )
    .header(
        Row::new(vec![
            "Drive/FS",
            "Mount Point",
            "Type",
            "Total Space",
            "Used Space",
            "Free Space",
            "Usage %",
        ])
        .style(Style::default().fg(Color::Cyan).bold())
        .bottom_margin(1),
    )
    .block(
        Block::default()
            .title(" 💾 Mounted Filesystems & Disks ")
            .title_style(Style::default().fg(Color::Cyan).bold())
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(storage_table, chunks[0]);

    // 2. Network Activity & Sparklines
    let net_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    // Download box
    let rx_slice: Vec<u64> = app.stats.net_rx_history.iter().copied().collect();
    let rx_kb = app.stats.current_rx_speed / 1024;
    let rx_block = Block::default()
        .title(format!(" 📥 Live Download Speed: {} KB/s ", rx_kb))
        .title_style(Style::default().fg(Color::Green).bold())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Green));
    let rx_inner = rx_block.inner(net_chunks[0]);
    frame.render_widget(rx_block, net_chunks[0]);
    let rx_sparkline = Sparkline::default()
        .data(&rx_slice)
        .style(Style::default().fg(Color::LightGreen))
        .bar_set(symbols::bar::NINE_LEVELS);
    frame.render_widget(rx_sparkline, rx_inner);

    // Upload box
    let tx_slice: Vec<u64> = app.stats.net_tx_history.iter().copied().collect();
    let tx_kb = app.stats.current_tx_speed / 1024;
    let tx_block = Block::default()
        .title(format!(" 📤 Live Upload Speed: {} KB/s ", tx_kb))
        .title_style(Style::default().fg(Color::Blue).bold())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Blue));
    let tx_inner = tx_block.inner(net_chunks[1]);
    frame.render_widget(tx_block, net_chunks[1]);
    let tx_sparkline = Sparkline::default()
        .data(&tx_slice)
        .style(Style::default().fg(Color::LightBlue))
        .bar_set(symbols::bar::NINE_LEVELS);
    frame.render_widget(tx_sparkline, tx_inner);
}

fn render_help(_app: &App, frame: &mut Frame, area: Rect) {
    let help_block = Block::default()
        .title(" 📖 devtop Documentation & Keybindings ")
        .title_style(Style::default().fg(Color::Yellow).bold())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow));

    let help_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(" ⚡ devtop - Modern Developer Terminal System Monitor", Style::default().fg(Color::Cyan).bold()),
        ]),
        Line::from(" Built with Rust, Ratatui, Crossterm, and Sysinfo."),
        Line::from(""),
        Line::from(Span::styled(" NAVIGATION & TABS", Style::default().fg(Color::Green).bold())),
        Line::from("   [Tab] or [Right]      Switch to next tab"),
        Line::from("   [BackTab] or [Left]   Switch to previous tab"),
        Line::from("   [1]                   Jump to Overview tab"),
        Line::from("   [2]                   Jump to Processes tab"),
        Line::from("   [3]                   Jump to Storage & Network tab"),
        Line::from("   [4] or [?]            Jump to Help tab"),
        Line::from(""),
        Line::from(Span::styled(" PROCESS MANAGEMENT", Style::default().fg(Color::Green).bold())),
        Line::from("   [↑ / ↓] or [j / k]    Select process in table"),
        Line::from("   [s]                   Cycle sort field (CPU -> Memory -> PID -> Name)"),
        Line::from("   [c]                   Sort by CPU %"),
        Line::from("   [m]                   Sort by Memory"),
        Line::from("   [p]                   Sort by PID"),
        Line::from("   [n]                   Sort by Process Name"),
        Line::from("   [/]                   Filter/Search processes"),
        Line::from("   [x] or [k]            Kill selected process (with confirmation dialog)"),
        Line::from(""),
        Line::from(Span::styled(" GENERAL CONTROLS", Style::default().fg(Color::Green).bold())),
        Line::from("   [+]                   Increase refresh interval (slower)"),
        Line::from("   [-]                   Decrease refresh interval (faster)"),
        Line::from("   [q] or [Esc] or [Ctrl+C] Quit devtop"),
        Line::from(""),
    ];

    frame.render_widget(Paragraph::new(help_text).block(help_block), area);
}

fn render_footer(app: &App, frame: &mut Frame, area: Rect) {
    let footer_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Status or Notification Message
    let status_text = if let Some((msg, _)) = &app.status_message {
        Line::from(vec![
            Span::styled(" ℹ ", Style::default().fg(Color::Cyan).bold()),
            Span::styled(msg.as_str(), Style::default().fg(Color::White)),
        ])
    } else {
        Line::from(vec![
            Span::styled(" devtop ready ", Style::default().fg(Color::DarkGray)),
            Span::styled("| Press '?' for help", Style::default().fg(Color::DarkGray)),
        ])
    };
    frame.render_widget(Paragraph::new(status_text), footer_chunks[0]);

    // Key hints
    let hints = Paragraph::new(Line::from(vec![
        Span::styled("[Tab]", Style::default().fg(Color::Yellow)),
        Span::raw(" Tabs "),
        Span::styled("[s]", Style::default().fg(Color::Yellow)),
        Span::raw(" Sort "),
        Span::styled("[/]", Style::default().fg(Color::Yellow)),
        Span::raw(" Search "),
        Span::styled("[k]", Style::default().fg(Color::Yellow)),
        Span::raw(" Kill "),
        Span::styled("[q]", Style::default().fg(Color::Yellow)),
        Span::raw(" Quit "),
    ]))
    .alignment(Alignment::Right);
    frame.render_widget(hints, footer_chunks[1]);
}

fn render_kill_modal(app: &App, frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(50, 25, area);
    frame.render_widget(Clear, popup_area);

    let default_target = (0, "Unknown".to_string());
    let (pid, name) = app.kill_target.as_ref().unwrap_or(&default_target);

    let modal_block = Block::default()
        .title(" ⚠️ Confirm Kill Process ")
        .title_style(Style::default().fg(Color::Red).bold())
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Red));

    let modal_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw(" Are you sure you want to terminate: "),
            Span::styled(format!("{} (PID: {})", name, pid), Style::default().fg(Color::Yellow).bold()),
            Span::raw("?"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [ y / Enter ] Confirm Kill  ", Style::default().bg(Color::Red).fg(Color::White).bold()),
            Span::raw("   "),
            Span::styled("  [ n / Esc ] Cancel  ", Style::default().bg(Color::DarkGray).fg(Color::White)),
        ]),
    ];

    let paragraph = Paragraph::new(modal_text)
        .alignment(Alignment::Center)
        .block(modal_block);
    frame.render_widget(paragraph, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
