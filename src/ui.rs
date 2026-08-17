use crate::app::{ActiveTab, App, SortBy};
use chrono::Local;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{
        Axis, Block, BorderType, Borders, Cell, Chart, Clear, Dataset, GraphType, Paragraph, Row,
        Table, Tabs, Wrap,
    },
};

pub fn render(app: &mut App, frame: &mut Frame) {
    let size = frame.area();

    // Base layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header & Tabs
            Constraint::Min(10),   // Main Content
            Constraint::Length(1), // Status & Keymap Footer
        ])
        .split(size);

    render_header(app, frame, chunks[0]);

    if app.show_process_detail {
        render_process_detail(app, frame, chunks[1]);
    } else {
        match app.active_tab {
            ActiveTab::Overview => render_overview(app, frame, chunks[1]),
            ActiveTab::Processes => render_processes(app, frame, chunks[1]),
            ActiveTab::Ports => render_ports(app, frame, chunks[1]),
            ActiveTab::StorageNetwork => render_storage_network(app, frame, chunks[1]),
            ActiveTab::Help => render_help(app, frame, chunks[1]),
        }
    }

    render_footer(app, frame, chunks[2]);

    // Modal popup if kill confirm is open
    if app.show_kill_confirm {
        render_kill_modal(app, frame, size);
    }
}

fn render_header(app: &App, frame: &mut Frame, area: Rect) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(18), // Title / Brand
            Constraint::Min(45),    // Navigation Tabs
            Constraint::Length(34), // Time & Quick info
        ])
        .split(area);

    // Title
    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(Color::Rgb(180, 180, 180)));
    let title_text = Paragraph::new(Line::from(vec![
        Span::styled(
            " devtop ",
            Style::default().fg(Color::Rgb(180, 180, 180)).bold(),
        ),
        Span::styled("v0.1.0", Style::default().fg(Color::DarkGray)),
    ]))
    .block(title_block);
    frame.render_widget(title_text, header_chunks[0]);

    // Tabs
    let tab_titles = vec![
        Line::from(vec![Span::raw("1: "), Span::raw("Overview")]),
        Line::from(vec![Span::raw("2: "), Span::raw("Processes")]),
        Line::from(vec![Span::raw("3: "), Span::raw("Ports")]),
        Line::from(vec![Span::raw("4: "), Span::raw("Storage & Net")]),
        Line::from(vec![Span::raw("5: "), Span::raw("Help")]),
    ];
    let active_index = app.active_tab as usize;
    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Plain)
                .border_style(Style::default().fg(Color::Rgb(100, 120, 160))),
        )
        .select(active_index)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Rgb(180, 180, 180))
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
        Span::styled(
            format!(" {} ", now),
            Style::default().fg(Color::Rgb(200, 190, 150)),
        ),
        Span::styled(
            format!("up {}:{:02} ", hours, mins),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            format!("rate: {}ms ", app.refresh_rate_ms),
            Style::default().fg(Color::Rgb(130, 170, 210)),
        ),
    ]))
    .alignment(Alignment::Right)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(Color::Rgb(100, 120, 160))),
    );
    frame.render_widget(right_info, header_chunks[2]);
}

fn render_overview(app: &App, frame: &mut Frame, area: Rect) {
    let main_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // CPU & Memory Braille Charts
            Constraint::Min(8),     // Quick Top Processes & Storage Preview
        ])
        .split(area);

    let cpu_harvest = app.core.get_cpu();

    // CPU & Memory Panels (Braille Time-Series Charts)
    let metrics_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_rows[0]);

    // --- CPU Chart & Core Breakdown ---
    let cpu_percent = cpu_harvest.global_usage;
    let cpu_points = history_to_points(&app.core.cpu_tracker.global_history);

    let cpu_color = if cpu_percent > 85.0 {
        Color::Red
    } else if cpu_percent > 60.0 {
        Color::Rgb(200, 190, 150)
    } else {
        Color::Rgb(130, 170, 210)
    };

    let cpu_chart_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(68), // Braille Line Chart
            Constraint::Percentage(32), // Per-Core CPU List
        ])
        .split(metrics_cols[0]);

    let (load1, load5, load15) = cpu_harvest.load_avg;
    let cpu_title = format!(
        " [ CPU: {:.1}%  Load: {:.2} {:.2} {:.2} ] ",
        cpu_percent, load1, load5, load15
    );

    let cpu_dataset = vec![
        Dataset::default()
            .name(format!("AVG {:.1}%", cpu_percent))
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(cpu_color))
            .data(&cpu_points),
    ];

    let cpu_chart = Chart::new(cpu_dataset)
        .block(
            Block::default()
                .title(cpu_title)
                .title_style(Style::default().fg(cpu_color).bold())
                .borders(Borders::ALL)
                .border_type(BorderType::Plain)
                .border_style(Style::default().fg(Color::Rgb(130, 170, 210))),
        )
        .x_axis(
            Axis::default()
                .bounds([0.0, 59.0])
                .labels(vec![Line::from("60s"), Line::from("30s"), Line::from("0s")])
                .style(Style::default().fg(Color::DarkGray)),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, 100.0])
                .labels(vec![
                    Line::from("0%"),
                    Line::from("50%"),
                    Line::from("100%"),
                ])
                .style(Style::default().fg(Color::DarkGray)),
        );
    frame.render_widget(cpu_chart, cpu_chart_chunks[0]);

    // CPU Per-Core List Box
    let mut core_lines = Vec::new();
    let cores = &cpu_harvest.per_core_usage;
    let num_cores = cores.len();

    for i in (0..num_cores).step_by(2) {
        let c1_val = cores.get(i).copied().unwrap_or(0.0);
        let c1_color = if c1_val > 80.0 {
            Color::Red
        } else if c1_val > 50.0 {
            Color::Rgb(200, 190, 150)
        } else {
            Color::Rgb(130, 170, 210)
        };

        let mut spans = vec![
            Span::styled(
                format!("C{:<2}:", i + 1),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("{:>3.0}% ", c1_val),
                Style::default().fg(c1_color).bold(),
            ),
        ];

        if let Some(&c2_val) = cores.get(i + 1) {
            let c2_color = if c2_val > 80.0 {
                Color::Red
            } else if c2_val > 50.0 {
                Color::Rgb(200, 190, 150)
            } else {
                Color::Rgb(130, 170, 210)
            };
            spans.push(Span::styled(
                format!(" C{:<2}:", i + 2),
                Style::default().fg(Color::DarkGray),
            ));
            spans.push(Span::styled(
                format!("{:>3.0}%", c2_val),
                Style::default().fg(c2_color).bold(),
            ));
        }

        core_lines.push(Line::from(spans));
    }

    let core_block = Block::default()
        .title(" [ Cores ] ")
        .title_style(Style::default().fg(Color::Rgb(180, 180, 180)).bold())
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(Color::DarkGray));
    frame.render_widget(
        Paragraph::new(core_lines).block(core_block),
        cpu_chart_chunks[1],
    );

    // --- Memory Multi-Series Braille Chart (byte-scale Y-axis like bottom) ---
    let mem = app.core.get_memory();
    let ram_points = history_to_points(&app.core.memory_tracker.ram_history);
    let swap_points = history_to_points(&app.core.memory_tracker.swap_history);
    let cache_points = history_to_points(&app.core.memory_tracker.cache_history);

    // Pick Y-axis unit based on total_bytes — same logic as bottom's get_binary_unit_and_denominator
    let total_bytes = app.core.memory_tracker.total_bytes;
    let (unit, denom) = binary_unit_and_denom(total_bytes);
    let total_gib = total_bytes as f64 / denom;
    // Scale Y bounds to total RAM in chosen unit
    let y_max = total_gib * 1.05; // 5% headroom

    // Label format matches bottom exactly: "RAM: 61%   9.8GiB/15.9GiB"
    let ram_label = format!(
        "RAM:{:3.0}%   {:.1}{}/{}{}",
        mem.used_percent,
        mem.used_bytes as f64 / denom,
        unit,
        total_gib,
        unit
    );

    let swap_total_bytes = app.core.memory_tracker.swap_total_bytes;
    let (swap_unit, swap_denom) = binary_unit_and_denom(swap_total_bytes.max(1));
    let swap_label = if swap_total_bytes > 0 {
        format!(
            "SWP:{:3.0}%   {:.1}{}/{}{}",
            mem.swap_used_percent,
            mem.swap_used_bytes as f64 / swap_denom,
            swap_unit,
            swap_total_bytes as f64 / swap_denom,
            swap_unit
        )
    } else {
        "SWP:  N/A".to_string()
    };
    let cache_label = format!(
        "CACHE:{:3.0}%   {:.1}{}",
        mem.cache_percent,
        mem.cache_bytes as f64 / denom,
        unit
    );

    // Scale swap/cache points to same unit as RAM for chart Y consistency
    let swap_points_scaled: Vec<(f64, f64)> =
        swap_points.iter().map(|(x, y)| (*x, y / denom)).collect();
    let cache_points_scaled: Vec<(f64, f64)> =
        cache_points.iter().map(|(x, y)| (*x, y / denom)).collect();
    let ram_points_scaled: Vec<(f64, f64)> =
        ram_points.iter().map(|(x, y)| (*x, y / denom)).collect();

    let mem_datasets = vec![
        Dataset::default()
            .name(ram_label)
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Rgb(160, 160, 160)))
            .data(&ram_points_scaled),
        Dataset::default()
            .name(swap_label)
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Rgb(200, 190, 150)))
            .data(&swap_points_scaled),
        Dataset::default()
            .name(cache_label)
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Rgb(180, 180, 180)))
            .data(&cache_points_scaled),
    ];

    let mem_title = format!(
        " [ Memory   {:.1}{}/{}{} ] ",
        mem.used_bytes as f64 / denom,
        unit,
        total_gib,
        unit
    );

    let mem_chart = Chart::new(mem_datasets)
        .block(
            Block::default()
                .title(mem_title)
                .title_style(Style::default().fg(Color::Rgb(160, 160, 160)).bold())
                .borders(Borders::ALL)
                .border_type(BorderType::Plain)
                .border_style(Style::default().fg(Color::Rgb(160, 160, 160))),
        )
        .x_axis(
            Axis::default()
                .bounds([0.0, 59.0])
                .labels(vec![Line::from("60s"), Line::from("30s"), Line::from("0s")])
                .style(Style::default().fg(Color::DarkGray)),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, y_max])
                .labels(vec![
                    Line::from(format!("0{}", unit)),
                    Line::from(format!("{:.1}{}", y_max / 2.0, unit)),
                    Line::from(format!("{:.1}{}", total_gib, unit)),
                ])
                .style(Style::default().fg(Color::DarkGray)),
        );
    frame.render_widget(mem_chart, metrics_cols[1]);

    // Quick Process Preview
    let bottom_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(main_rows[1]);

    let procs = app.filtered_sorted_processes();
    let top_procs = procs.iter().take(6);
    let rows = top_procs.map(|p| {
        let port_badge = if !p.ports.is_empty() {
            format!(":{}", p.ports[0])
        } else {
            "-".to_string()
        };

        Row::new(vec![
            Cell::from(p.pid.to_string()).style(Style::default().fg(Color::Rgb(180, 180, 180))),
            Cell::from(p.user.clone()).style(Style::default().fg(Color::DarkGray)),
            Cell::from(port_badge).style(Style::default().fg(Color::Rgb(200, 190, 150))),
            Cell::from(p.name.clone()).style(Style::default().bold()),
            Cell::from(format!("{:.1}%", p.cpu_usage)).style(Style::default().fg(
                if p.cpu_usage > 50.0 {
                    Color::Red
                } else if p.cpu_usage > 10.0 {
                    Color::Rgb(200, 190, 150)
                } else {
                    Color::Rgb(130, 170, 210)
                },
            )),
            Cell::from(format_bytes(p.memory_bytes)),
        ])
    });

    let proc_table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Min(15),
            Constraint::Length(9),
            Constraint::Length(11),
        ],
    )
    .header(
        Row::new(vec!["PID", "User", "Port", "Name", "CPU %", "Memory"])
            .style(Style::default().fg(Color::Rgb(200, 190, 150)).bold())
            .bottom_margin(1),
    )
    .block(
        Block::default()
            .title(" [ Top Active Processes (Press '2' for Manager) ] ")
            .title_style(Style::default().fg(Color::Rgb(200, 190, 150)).bold())
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(proc_table, bottom_cols[0]);

    // Network & Disk Quick Overview
    let net = app.core.get_network();
    let rx_str = format_speed(net.current_rx_speed);
    let tx_str = format_speed(net.current_tx_speed);
    let net_box = Block::default()
        .title(" [ Network Activity & Storage ] ")
        .title_style(Style::default().fg(Color::Rgb(180, 180, 180)).bold())
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(Color::DarkGray));

    let mut net_lines = vec![
        Line::from(vec![
            Span::styled(
                " Download (RX): ",
                Style::default().fg(Color::Rgb(130, 170, 210)).bold(),
            ),
            Span::raw(rx_str),
            Span::styled(
                "   Upload (TX): ",
                Style::default().fg(Color::Rgb(150, 170, 190)).bold(),
            ),
            Span::raw(tx_str),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " Mounted Disks: ",
            Style::default().fg(Color::Rgb(200, 190, 150)).bold(),
        )),
    ];

    for disk in app.core.get_disks().iter().take(3) {
        let total_str = format_bytes(disk.total_bytes);
        let used_str = format_bytes(disk.used_bytes);
        net_lines.push(Line::from(vec![
            Span::styled(format!("  {} ", disk.mount_point), Style::default().bold()),
            Span::raw(format!(
                "({}/{} - {:.0}% used)",
                used_str, total_str, disk.used_percent
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

    let search_style = if app.search_mode {
        Style::default()
            .fg(Color::Rgb(200, 190, 150))
            .bg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let search_content = if app.search_mode {
        format!(
            " Filter: {}_  (Press Enter/Esc to finish)",
            app.search_query
        )
    } else if !app.search_query.is_empty() {
        format!(
            " Filter: '{}'  [Press '/' to edit, Esc to clear]  | Mode: {}",
            app.search_query,
            if app.tree_mode { "Tree" } else { "List" }
        )
    } else {
        format!(
            " Press '/' to search | [t] Tree Mode: {} | [Enter] Inspect Detail | [k/x] Kill",
            if app.tree_mode { "ON" } else { "OFF" }
        )
    };

    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(if app.search_mode {
            Style::default().fg(Color::Rgb(200, 190, 150))
        } else {
            Style::default().fg(Color::DarkGray)
        });
    let search_p = Paragraph::new(search_content)
        .style(search_style)
        .block(search_block);
    frame.render_widget(search_p, chunks[0]);

    if app.tree_mode {
        render_tree_table(app, frame, chunks[1]);
    } else {
        render_flat_table(app, frame, chunks[1]);
    }
}

fn render_flat_table(app: &mut App, frame: &mut Frame, area: Rect) {
    let procs = app.filtered_sorted_processes();
    let total_count = procs.len();

    if total_count > 0 && app.selected_process >= total_count {
        app.selected_process = total_count - 1;
    }

    let cpu_header = match app.sort_by {
        SortBy::Cpu => {
            if app.sort_ascending {
                "CPU % ▲"
            } else {
                "CPU % ▼"
            }
        }
        _ => "CPU %",
    };
    let mem_header = match app.sort_by {
        SortBy::Memory => {
            if app.sort_ascending {
                "MEM % ▲"
            } else {
                "MEM % ▼"
            }
        }
        _ => "MEM %",
    };
    let pid_header = match app.sort_by {
        SortBy::Pid => {
            if app.sort_ascending {
                "PID ▲"
            } else {
                "PID ▼"
            }
        }
        _ => "PID",
    };
    let name_header = match app.sort_by {
        SortBy::Name => {
            if app.sort_ascending {
                "Name ▲"
            } else {
                "Name ▼"
            }
        }
        _ => "Name",
    };
    let user_header = match app.sort_by {
        SortBy::User => {
            if app.sort_ascending {
                "User ▲"
            } else {
                "User ▼"
            }
        }
        _ => "User",
    };

    let header_row = Row::new(vec![
        Cell::from(pid_header),
        Cell::from("PPID"),
        Cell::from(user_header),
        Cell::from("Port"),
        Cell::from(name_header),
        Cell::from(cpu_header),
        Cell::from("Memory"),
        Cell::from(mem_header),
        Cell::from("Status"),
        Cell::from("Command"),
    ])
    .style(Style::default().fg(Color::Rgb(180, 180, 180)).bold())
    .bottom_margin(1);

    let table_height = area.height.saturating_sub(4) as usize;
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
            let row_style = if is_selected {
                Style::default()
                    .bg(Color::Rgb(45, 45, 55))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let cpu_color = if p.cpu_usage > 50.0 {
                Color::Rgb(210, 100, 100)
            } else if p.cpu_usage > 15.0 {
                Color::Rgb(200, 190, 150)
            } else {
                Color::Rgb(130, 170, 210)
            };

            let prefix = if is_selected { "▶ " } else { "  " };
            let ppid_str = p
                .parent_pid
                .map(|pid| pid.to_string())
                .unwrap_or_else(|| "-".to_string());
            let port_str = if !p.ports.is_empty() {
                format!(":{}", p.ports[0])
            } else {
                "-".to_string()
            };

            Row::new(vec![
                Cell::from(format!("{}{}", prefix, p.pid)),
                Cell::from(ppid_str).style(Style::default().fg(Color::DarkGray)),
                Cell::from(p.user.clone()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(port_str).style(Style::default().fg(Color::Rgb(200, 190, 150))),
                Cell::from(p.name.clone()),
                Cell::from(format!("{:.1}%", p.cpu_usage)).style(Style::default().fg(cpu_color)),
                Cell::from(format_bytes(p.memory_bytes)),
                Cell::from(format!("{:.1}%", p.memory_percent)),
                Cell::from(p.status.clone()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(p.cmd.clone()),
            ])
            .style(row_style)
        })
        .collect();

    let table_title = format!(
        " [ Process List ({} processes) | Sort: {:?} | [t] Tree Mode | [Enter] Detail ] ",
        total_count, app.sort_by
    );

    let table = Table::new(
        rows,
        [
            Constraint::Length(9),  // PID
            Constraint::Length(8),  // PPID
            Constraint::Length(10), // User
            Constraint::Length(8),  // Port
            Constraint::Length(18), // Name
            Constraint::Length(9),  // CPU
            Constraint::Length(11), // Memory
            Constraint::Length(9),  // MEM %
            Constraint::Length(11), // Status
            Constraint::Min(20),    // Command
        ],
    )
    .header(header_row)
    .block(
        Block::default()
            .title(table_title)
            .title_style(Style::default().fg(Color::Rgb(200, 190, 150)).bold())
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(Color::Rgb(200, 190, 150))),
    );

    frame.render_widget(table, area);
}

fn render_tree_table(app: &mut App, frame: &mut Frame, area: Rect) {
    let tree_items = app.tree_processes();
    let total_count = tree_items.len();

    if total_count > 0 && app.selected_process >= total_count {
        app.selected_process = total_count - 1;
    }

    let header_row = Row::new(vec![
        Cell::from("PID"),
        Cell::from("Process Tree"),
        Cell::from("User"),
        Cell::from("Port"),
        Cell::from("CPU %"),
        Cell::from("Memory"),
        Cell::from("MEM %"),
        Cell::from("Status"),
    ])
    .style(Style::default().fg(Color::Rgb(180, 180, 180)).bold())
    .bottom_margin(1);

    let table_height = area.height.saturating_sub(4) as usize;
    let selected_index = app.selected_process;

    let start_idx = if selected_index >= app.scroll_offset + table_height {
        selected_index - table_height + 1
    } else if selected_index < app.scroll_offset {
        selected_index
    } else {
        app.scroll_offset
    };
    app.scroll_offset = start_idx;

    let rows: Vec<Row> = tree_items
        .iter()
        .enumerate()
        .skip(start_idx)
        .take(table_height.max(1))
        .map(|(idx, item)| {
            let p = &item.process;
            let is_selected = idx == selected_index;
            let row_style = if is_selected {
                Style::default()
                    .bg(Color::Rgb(45, 45, 55))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let cpu_color = if p.cpu_usage > 50.0 {
                Color::Rgb(210, 100, 100)
            } else if p.cpu_usage > 15.0 {
                Color::Rgb(200, 190, 150)
            } else {
                Color::Rgb(130, 170, 210)
            };

            let prefix = if is_selected { "▶ " } else { "  " };
            let tree_name = format!("{}{}", item.prefix, p.name);
            let port_str = if !p.ports.is_empty() {
                format!(":{}", p.ports[0])
            } else {
                "-".to_string()
            };

            Row::new(vec![
                Cell::from(format!("{}{}", prefix, p.pid)),
                Cell::from(tree_name),
                Cell::from(p.user.clone()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(port_str).style(Style::default().fg(Color::Rgb(200, 190, 150))),
                Cell::from(format!("{:.1}%", p.cpu_usage)).style(Style::default().fg(cpu_color)),
                Cell::from(format_bytes(p.memory_bytes)),
                Cell::from(format!("{:.1}%", p.memory_percent)),
                Cell::from(p.status.clone()).style(Style::default().fg(Color::DarkGray)),
            ])
            .style(row_style)
        })
        .collect();

    let table_title = format!(
        " [ Process Tree ({} nodes) | [t] Flat Table Mode | [Enter] Detail ] ",
        total_count
    );

    let table = Table::new(
        rows,
        [
            Constraint::Length(9),  // PID
            Constraint::Min(26),    // Process Tree
            Constraint::Length(10), // User
            Constraint::Length(8),  // Port
            Constraint::Length(9),  // CPU
            Constraint::Length(11), // Memory
            Constraint::Length(9),  // MEM %
            Constraint::Length(11), // Status
        ],
    )
    .header(header_row)
    .block(
        Block::default()
            .title(table_title)
            .title_style(Style::default().fg(Color::Rgb(200, 190, 150)).bold())
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(Color::Rgb(200, 190, 150))),
    );

    frame.render_widget(table, area);
}

fn render_ports(app: &mut App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search / Filter
            Constraint::Min(5),    // Ports Table
        ])
        .split(area);

    let search_style = if app.search_mode {
        Style::default()
            .fg(Color::Rgb(200, 190, 150))
            .bg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let search_content = if app.search_mode {
        format!(
            " Filter Ports: {}_  (Press Enter/Esc to finish)",
            app.search_query
        )
    } else if !app.search_query.is_empty() {
        format!(
            " Filter: '{}'  [Press '/' to edit, Esc to clear] | [k] Terminate Port Process",
            app.search_query
        )
    } else {
        " Press '/' to search ports | [Enter] Inspect Process Detail | [k/x] Kill Process on Port"
            .to_string()
    };

    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(if app.search_mode {
            Style::default().fg(Color::Rgb(200, 190, 150))
        } else {
            Style::default().fg(Color::DarkGray)
        });
    let search_p = Paragraph::new(search_content)
        .style(search_style)
        .block(search_block);
    frame.render_widget(search_p, chunks[0]);

    let ports = app.filtered_ports();
    let total_count = ports.len();

    if total_count > 0 && app.selected_port >= total_count {
        app.selected_port = total_count - 1;
    }

    let header_row = Row::new(vec![
        Cell::from("Port"),
        Cell::from("Proto"),
        Cell::from("Bind Address"),
        Cell::from("PID"),
        Cell::from("Process Name"),
        Cell::from("User"),
        Cell::from("Project / Working Dir (CWD)"),
    ])
    .style(Style::default().fg(Color::Rgb(180, 180, 180)).bold())
    .bottom_margin(1);

    let table_height = chunks[1].height.saturating_sub(4) as usize;
    let selected_index = app.selected_port;

    let start_idx = if selected_index >= app.port_scroll_offset + table_height {
        selected_index - table_height + 1
    } else if selected_index < app.port_scroll_offset {
        selected_index
    } else {
        app.port_scroll_offset
    };
    app.port_scroll_offset = start_idx;

    let rows: Vec<Row> = ports
        .iter()
        .enumerate()
        .skip(start_idx)
        .take(table_height.max(1))
        .map(|(idx, port)| {
            let is_selected = idx == selected_index;
            let row_style = if is_selected {
                Style::default()
                    .bg(Color::Rgb(45, 45, 55))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let prefix = if is_selected { "▶ " } else { "  " };
            let pid_str = port
                .pid
                .map(|p| p.to_string())
                .unwrap_or_else(|| "-".to_string());
            let name_str = port.process_name.clone().unwrap_or_else(|| "-".to_string());
            let user_str = port.user.clone().unwrap_or_else(|| "-".to_string());
            let cwd_str = port.cwd.clone().unwrap_or_else(|| "-".to_string());

            Row::new(vec![
                Cell::from(format!("{}{}", prefix, port.port))
                    .style(Style::default().fg(Color::Rgb(200, 190, 150)).bold()),
                Cell::from(port.protocol.clone())
                    .style(Style::default().fg(Color::Rgb(150, 170, 190))),
                Cell::from(port.ip.clone()),
                Cell::from(pid_str).style(Style::default().fg(Color::Rgb(180, 180, 180))),
                Cell::from(name_str).style(Style::default().bold()),
                Cell::from(user_str).style(Style::default().fg(Color::DarkGray)),
                Cell::from(cwd_str),
            ])
            .style(row_style)
        })
        .collect();

    let table_title = format!(
        " [ Listening Sockets & Ports ({} active) | [k] Kill Port | [Enter] Detail ] ",
        total_count
    );

    let table = Table::new(
        rows,
        [
            Constraint::Length(10), // Port
            Constraint::Length(8),  // Proto
            Constraint::Length(16), // IP
            Constraint::Length(9),  // PID
            Constraint::Length(18), // Process Name
            Constraint::Length(12), // User
            Constraint::Min(24),    // CWD
        ],
    )
    .header(header_row)
    .block(
        Block::default()
            .title(table_title)
            .title_style(Style::default().fg(Color::Rgb(200, 190, 150)).bold())
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(Color::Rgb(200, 190, 150))),
    );

    frame.render_widget(table, chunks[1]);
}

fn render_process_detail(app: &App, frame: &mut Frame, area: Rect) {
    let pid = match app.selected_detail_pid {
        Some(p) => p,
        None => {
            let msg = Paragraph::new("No process selected").alignment(Alignment::Center);
            frame.render_widget(msg, area);
            return;
        }
    };

    let proc = match app.core.get_process_by_pid(pid) {
        Some(p) => p,
        None => {
            let msg = Paragraph::new(format!("Process PID {} has terminated.", pid))
                .alignment(Alignment::Center);
            frame.render_widget(msg, area);
            return;
        }
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header badge
            Constraint::Length(9), // Details Grid
            Constraint::Min(6),    // Hierarchy & Cmd
            Constraint::Length(3), // Action bar
        ])
        .split(area);

    // 1. Process Top Banner
    let banner_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(Color::Rgb(180, 180, 180)));
    let banner_text = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" Process: {} ", proc.name),
            Style::default().fg(Color::Rgb(180, 180, 180)).bold(),
        ),
        Span::styled(
            format!("(PID: {}) ", proc.pid),
            Style::default().fg(Color::Rgb(200, 190, 150)).bold(),
        ),
        Span::styled(
            format!("Status: {} ", proc.status),
            Style::default().fg(Color::Rgb(130, 170, 210)),
        ),
        Span::styled(
            format!("Uptime: {} ", format_duration(proc.run_time_secs)),
            Style::default().fg(Color::DarkGray),
        ),
    ]))
    .block(banner_block);
    frame.render_widget(banner_text, chunks[0]);

    // 2. Metrics Grid
    let grid_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    let res_block = Block::default()
        .title(" [ Resource Usage ] ")
        .title_style(Style::default().fg(Color::Rgb(130, 170, 210)).bold())
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(Color::Rgb(130, 170, 210)));

    let res_lines = vec![
        Line::from(vec![
            Span::styled(" CPU Usage: ", Style::default().bold()),
            Span::styled(
                format!("{:.2}%", proc.cpu_usage),
                Style::default().fg(if proc.cpu_usage > 50.0 {
                    Color::Red
                } else if proc.cpu_usage > 15.0 {
                    Color::Rgb(200, 190, 150)
                } else {
                    Color::Rgb(130, 170, 210)
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Resident Memory (RSS): ", Style::default().bold()),
            Span::raw(format!(
                "{} ({:.1}% of system)",
                format_bytes(proc.memory_bytes),
                proc.memory_percent
            )),
        ]),
        Line::from(vec![
            Span::styled(" Virtual Memory (VIRT): ", Style::default().bold()),
            Span::raw(format_bytes(proc.virtual_memory_bytes)),
        ]),
        Line::from(vec![
            Span::styled(" Run Duration: ", Style::default().bold()),
            Span::raw(format_duration(proc.run_time_secs)),
        ]),
    ];
    frame.render_widget(Paragraph::new(res_lines).block(res_block), grid_cols[0]);

    let sys_block = Block::default()
        .title(" [ Process Context ] ")
        .title_style(Style::default().fg(Color::Rgb(160, 160, 160)).bold())
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(Color::Rgb(160, 160, 160)));

    let ppid_text = match proc.parent_pid {
        Some(ppid) => {
            let pname = app
                .core
                .get_process_by_pid(ppid)
                .map(|p| format!(" ({})", p.name))
                .unwrap_or_default();
            format!("{}{}", ppid, pname)
        }
        None => "None (Root)".to_string(),
    };

    let ports_text = if proc.ports.is_empty() {
        "None".to_string()
    } else {
        proc.ports
            .iter()
            .map(|p| format!(":{}", p))
            .collect::<Vec<_>>()
            .join(", ")
    };

    let sys_lines = vec![
        Line::from(vec![
            Span::styled(" User / Owner: ", Style::default().bold()),
            Span::raw(&proc.user),
            Span::styled("   Listening Ports: ", Style::default().bold()),
            Span::styled(ports_text, Style::default().fg(Color::Rgb(200, 190, 150))),
        ]),
        Line::from(vec![
            Span::styled(" Parent Process (PPID): ", Style::default().bold()),
            Span::raw(ppid_text),
        ]),
        Line::from(vec![
            Span::styled(" Executable Path: ", Style::default().bold()),
            Span::raw(if proc.exe.is_empty() { "-" } else { &proc.exe }),
        ]),
        Line::from(vec![
            Span::styled(" Working Dir (CWD): ", Style::default().bold()),
            Span::raw(if proc.cwd.is_empty() { "-" } else { &proc.cwd }),
        ]),
    ];
    frame.render_widget(Paragraph::new(sys_lines).block(sys_block), grid_cols[1]);

    // 3. Hierarchy & Command
    let lower_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[2]);

    let children_block = Block::default()
        .title(format!(
            " [ Children Subprocesses ({}) ] ",
            proc.children.len()
        ))
        .title_style(Style::default().fg(Color::Rgb(200, 190, 150)).bold())
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(Color::Rgb(200, 190, 150)));

    let children_lines: Vec<Line> = if proc.children.is_empty() {
        vec![Line::from(Span::styled(
            " No child processes found.",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        proc.children
            .iter()
            .take(6)
            .map(|cpid| {
                let cname = app
                    .core
                    .get_process_by_pid(*cpid)
                    .map(|c| c.name)
                    .unwrap_or_else(|| "unknown".to_string());
                Line::from(vec![
                    Span::styled("  ├─ ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("PID {}", cpid),
                        Style::default().fg(Color::Rgb(180, 180, 180)),
                    ),
                    Span::raw(format!(": {}", cname)),
                ])
            })
            .collect()
    };
    frame.render_widget(
        Paragraph::new(children_lines).block(children_block),
        lower_cols[0],
    );

    let cmd_block = Block::default()
        .title(" [ Full Command Line ] ")
        .title_style(Style::default().fg(Color::Rgb(180, 180, 180)).bold())
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(Color::Rgb(180, 180, 180)));
    let cmd_p = Paragraph::new(proc.cmd.clone())
        .wrap(Wrap { trim: true })
        .block(cmd_block);
    frame.render_widget(cmd_p, lower_cols[1]);

    // 4. Action bar
    let action_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(Color::DarkGray));
    let action_line = Paragraph::new(Line::from(vec![
        Span::styled(" [k / x] ", Style::default().fg(Color::Red).bold()),
        Span::raw("Kill Process  |  "),
        Span::styled(
            " [Esc / Enter] ",
            Style::default().fg(Color::Rgb(200, 190, 150)).bold(),
        ),
        Span::raw("Back to Process List  |  "),
        Span::styled(
            " [q] ",
            Style::default().fg(Color::Rgb(200, 190, 150)).bold(),
        ),
        Span::raw("Quit"),
    ]))
    .alignment(Alignment::Center)
    .block(action_block);
    frame.render_widget(action_line, chunks[3]);
}

fn render_storage_network(app: &App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // Storage Details
            Constraint::Percentage(50), // Network Braille Time-Series Chart
        ])
        .split(area);

    // 1. Storage Disks
    let disks = app.core.get_disks();
    let disk_rows: Vec<Row> = disks
        .iter()
        .map(|disk| {
            let total_str = format_bytes(disk.total_bytes);
            let free_str = format_bytes(disk.available_bytes);
            let used_str = format_bytes(disk.used_bytes);

            let color = if disk.used_percent > 90.0 {
                Color::Red
            } else if disk.used_percent > 75.0 {
                Color::Rgb(200, 190, 150)
            } else {
                Color::Rgb(130, 170, 210)
            };

            Row::new(vec![
                Cell::from(disk.name.clone()).style(Style::default().bold()),
                Cell::from(disk.mount_point.clone()),
                Cell::from(disk.file_system.clone()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(total_str),
                Cell::from(used_str),
                Cell::from(free_str),
                Cell::from(format!("{:.1}%", disk.used_percent))
                    .style(Style::default().fg(color).bold()),
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
        .style(Style::default().fg(Color::Rgb(180, 180, 180)).bold())
        .bottom_margin(1),
    )
    .block(
        Block::default()
            .title(" [ Mounted Filesystems & Disks ] ")
            .title_style(Style::default().fg(Color::Rgb(180, 180, 180)).bold())
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(Color::Rgb(180, 180, 180))),
    );
    frame.render_widget(storage_table, chunks[0]);

    // 2. Network Activity Braille Chart
    let net = app.core.get_network();
    let rx_points = history_to_points(&app.core.network_tracker.rx_history);
    let tx_points = history_to_points(&app.core.network_tracker.tx_history);

    let max_rx = app
        .core
        .network_tracker
        .rx_history
        .iter()
        .copied()
        .fold(0.0f64, f64::max);
    let max_tx = app
        .core
        .network_tracker
        .tx_history
        .iter()
        .copied()
        .fold(0.0f64, f64::max);
    let max_speed_kb = (max_rx.max(max_tx) * 1.2).max(100.0);

    let rx_label = format!(
        "RX: {} (Total: {})",
        format_speed(net.current_rx_speed),
        format_bytes(net.total_rx_bytes)
    );
    let tx_label = format!(
        "TX: {} (Total: {})",
        format_speed(net.current_tx_speed),
        format_bytes(net.total_tx_bytes)
    );

    let net_datasets = vec![
        Dataset::default()
            .name(rx_label)
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Rgb(130, 170, 210)))
            .data(&rx_points),
        Dataset::default()
            .name(tx_label)
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Rgb(150, 170, 190)))
            .data(&tx_points),
    ];

    let net_chart = Chart::new(net_datasets)
        .block(
            Block::default()
                .title(" [ Live Network Traffic (RX / TX) ] ")
                .title_style(Style::default().fg(Color::Rgb(180, 180, 180)).bold())
                .borders(Borders::ALL)
                .border_type(BorderType::Plain)
                .border_style(Style::default().fg(Color::Rgb(100, 120, 160))),
        )
        .x_axis(
            Axis::default()
                .bounds([0.0, 59.0])
                .labels(vec![Line::from("60s"), Line::from("30s"), Line::from("0s")])
                .style(Style::default().fg(Color::DarkGray)),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, max_speed_kb])
                .labels(vec![
                    Line::from("0 KB/s"),
                    Line::from(format!("{:.0} KB/s", max_speed_kb / 2.0)),
                    Line::from(format!("{:.0} KB/s", max_speed_kb)),
                ])
                .style(Style::default().fg(Color::DarkGray)),
        );

    frame.render_widget(net_chart, chunks[1]);
}

fn render_help(_app: &App, frame: &mut Frame, area: Rect) {
    let help_block = Block::default()
        .title(" [ devtop Documentation & Keybindings ] ")
        .title_style(Style::default().fg(Color::Rgb(200, 190, 150)).bold())
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(Color::Rgb(200, 190, 150)));

    let help_text = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            " devtop - Terminal Control Center for Developers",
            Style::default().fg(Color::Rgb(180, 180, 180)).bold(),
        )]),
        Line::from(
            " High-performance developer system monitor with process tree & listening port detection.",
        ),
        Line::from(""),
        Line::from(Span::styled(
            " NAVIGATION & TABS",
            Style::default().fg(Color::Rgb(130, 170, 210)).bold(),
        )),
        Line::from("   [Tab] or [Right]      Switch to next tab"),
        Line::from("   [BackTab] or [Left]   Switch to previous tab"),
        Line::from("   [1]                   Overview (System metrics, load averages, charts)"),
        Line::from("   [2]                   Processes (Process manager & tree hierarchy)"),
        Line::from("   [3]                   Ports (Listening sockets & project port mappings)"),
        Line::from("   [4]                   Storage & Network (Mounted disks & live I/O)"),
        Line::from("   [5] or [?]            Help (Documentation & hotkeys)"),
        Line::from(""),
        Line::from(Span::styled(
            " PROCESS & PORT MANAGEMENT",
            Style::default().fg(Color::Rgb(130, 170, 210)).bold(),
        )),
        Line::from("   [↑ / ↓] or [j / k]    Select row in process / port table"),
        Line::from("   [PageUp / PageDown]   Scroll by page"),
        Line::from("   [Enter]               Drill-down to inspect Process Detail view"),
        Line::from("   [t]                   Toggle Process Tree Mode vs Flat Table Mode"),
        Line::from(
            "   [s]                   Cycle sort field (CPU -> Memory -> PID -> Name -> User)",
        ),
        Line::from("   [c/m/p/n/u]           Sort directly by CPU, Memory, PID, Name, User"),
        Line::from("   [/]                   Filter/Search processes and ports"),
        Line::from("   [x] or [k]            Terminate selected process / port (SIGKILL)"),
        Line::from(""),
        Line::from(Span::styled(
            " GENERAL CONTROLS",
            Style::default().fg(Color::Rgb(130, 170, 210)).bold(),
        )),
        Line::from("   [+] or [=]            Increase refresh interval (slower)"),
        Line::from("   [-] or [_]            Decrease refresh interval (faster)"),
        Line::from("   [q] or [Ctrl+C]       Quit devtop"),
        Line::from(""),
    ];

    frame.render_widget(Paragraph::new(help_text).block(help_block), area);
}

fn render_footer(app: &App, frame: &mut Frame, area: Rect) {
    let hints_line = Line::from(vec![
        Span::styled("[Tab]", Style::default().fg(Color::Rgb(200, 190, 150))),
        Span::raw(" Tabs "),
        Span::styled("[Enter]", Style::default().fg(Color::Rgb(200, 190, 150))),
        Span::raw(" Detail "),
        Span::styled("[t]", Style::default().fg(Color::Rgb(200, 190, 150))),
        Span::raw(" Tree "),
        Span::styled("[s]", Style::default().fg(Color::Rgb(200, 190, 150))),
        Span::raw(" Sort "),
        Span::styled("[/]", Style::default().fg(Color::Rgb(200, 190, 150))),
        Span::raw(" Search "),
        Span::styled("[k]", Style::default().fg(Color::Rgb(200, 190, 150))),
        Span::raw(" Kill "),
        Span::styled("[q]", Style::default().fg(Color::Rgb(200, 190, 150))),
        Span::raw(" Quit "),
    ]);
    // Pin keymap width so "[q] Quit" is never clipped; status shrinks first.
    let hints_width = (hints_line.width() as u16).min(area.width);

    let footer_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(hints_width)])
        .split(area);

    let status_text = if let Some((msg, _)) = &app.status_message {
        Line::from(vec![
            Span::styled(" > ", Style::default().fg(Color::Rgb(180, 180, 180)).bold()),
            Span::styled(msg.as_str(), Style::default().fg(Color::White)),
        ])
    } else {
        Line::from(vec![
            Span::styled(" devtop ready ", Style::default().fg(Color::DarkGray)),
            Span::styled("| Press '?' for help", Style::default().fg(Color::DarkGray)),
        ])
    };
    frame.render_widget(Paragraph::new(status_text), footer_chunks[0]);
    frame.render_widget(Paragraph::new(hints_line), footer_chunks[1]);
}

fn render_kill_modal(app: &App, frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(50, 25, area);
    frame.render_widget(Clear, popup_area);

    let default_target = (0, "Unknown".to_string());
    let (pid, name) = app.kill_target.as_ref().unwrap_or(&default_target);

    let modal_block = Block::default()
        .title(" [ Confirm Termination ] ")
        .title_style(Style::default().fg(Color::Red).bold())
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(Color::Red));

    let modal_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw(" Are you sure you want to terminate: "),
            Span::styled(
                format!("{} (PID: {})", name, pid),
                Style::default().fg(Color::Rgb(200, 190, 150)).bold(),
            ),
            Span::raw("?"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  [ y / Enter ] Confirm SIGKILL  ",
                Style::default().bg(Color::Red).fg(Color::White).bold(),
            ),
            Span::raw("   "),
            Span::styled(
                "  [ n / Esc ] Cancel  ",
                Style::default().bg(Color::DarkGray).fg(Color::White),
            ),
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

fn history_to_points(history: &std::collections::VecDeque<f64>) -> Vec<(f64, f64)> {
    history
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as f64, v))
        .collect()
}

fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * KIB;
    const GIB: u64 = 1024 * MIB;
    const TIB: u64 = 1024 * GIB;

    if bytes >= TIB {
        format!("{:.2} GiB", bytes as f64 / TIB as f64)
    } else if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn format_speed(bytes_per_sec: u64) -> String {
    format!("{}/s", format_bytes(bytes_per_sec))
}

fn format_duration(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    let s = secs % 60;

    if days > 0 {
        format!("{}d {:02}:{:02}:{:02}", days, hours, mins, s)
    } else {
        format!("{:02}:{:02}:{:02}", hours, mins, s)
    }
}

/// Returns the most appropriate binary unit label and its denominator for the given byte count.
/// Matches bottom's `get_binary_unit_and_denominator` exactly:
///   < 1 KiB  → "B",   1.0
///   < 1 MiB  → "KiB", 1024.0
///   < 1 GiB  → "MiB", 1024^2
///   < 1 TiB  → "GiB", 1024^3
///   otherwise → "TiB", 1024^4
fn binary_unit_and_denom(bytes: u64) -> (&'static str, f64) {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * KIB;
    const GIB: u64 = 1024 * MIB;
    const TIB: u64 = 1024 * GIB;

    match bytes {
        b if b < KIB => ("B", 1.0),
        b if b < MIB => ("KiB", KIB as f64),
        b if b < GIB => ("MiB", MIB as f64),
        b if b < TIB => ("GiB", GIB as f64),
        _ => ("TiB", TIB as f64),
    }
}
