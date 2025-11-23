pub struct CliFormatter;

impl CliFormatter {
    pub fn print_section_header(title: &str) {
        println!("\n{}", ansi_color("cyan", title, true));
        println!("{}", "─".repeat(title.len()).dimmed());
    }

    pub fn print_field(label: &str, value: &str, color: Option<&str>) {
        let colored_value = match color {
            Some(c) => ansi_color(c, value, false),
            None => value.to_string(),
        };
        println!("  {:<12} {}", format!("{}:", label).dimmed(), colored_value);
    }

    pub fn print_field_bold(label: &str, value: &str, color: Option<&str>) {
        let colored_value = match color {
            Some(c) => ansi_color(c, value, true),
            None => bold(value),
        };
        println!("  {:<12} {}", format!("{}:", label).dimmed(), colored_value);
    }

    pub fn print_status(status: &str, is_active: bool) {
        let (symbol, color) = if is_active {
            ("●", "green")
        } else {
            ("○", "red")
        };
        println!(
            "  {:<12} {} {}",
            "Status:".dimmed(),
            ansi_color(color, symbol, false),
            ansi_color(color, status, true)
        );
    }

    pub fn print_summary(title: &str, total: &str) {
        println!("\n{}", ansi_color("white", title, true));
        println!("  {}", ansi_color("green", total, true));
    }

    pub fn print_project_entry(name: &str, duration: &str) {
        println!(
            "  {:<25} {}",
            ansi_color("yellow", &truncate_string(name, 25), true),
            ansi_color("green", duration, true)
        );
    }

    pub fn print_context_entry(context: &str, duration: &str) {
        let color = get_context_color(context);
        println!(
            "    {:<20} {}",
            ansi_color(color, &format!("├─ {}", context), false),
            ansi_color("green", duration, false)
        );
    }

    pub fn print_session_entry(
        session_id: Option<i64>,
        project: &str,
        duration: &str,
        status: &str,
        timestamp: &str,
    ) {
        let status_symbol = if status == "active" { "●" } else { "○" };
        let status_color = if status == "active" { "green" } else { "gray" };

        println!(
            "  {} {:<20} {:<15} {}",
            ansi_color(status_color, status_symbol, false),
            ansi_color("yellow", &truncate_string(project, 20), false),
            ansi_color("green", duration, false),
            timestamp.dimmed()
        );
    }

    pub fn print_empty_state(message: &str) {
        println!("\n  {}", message.dimmed());
    }

    pub fn print_error(message: &str) {
        println!(
            "  {} {}",
            ansi_color("red", "✗", true),
            ansi_color("red", message, false)
        );
    }

    pub fn print_success(message: &str) {
        println!(
            "  {} {}",
            ansi_color("green", "✓", true),
            ansi_color("green", message, false)
        );
    }

    pub fn print_warning(message: &str) {
        println!(
            "  {} {}",
            ansi_color("yellow", "⚠", true),
            ansi_color("yellow", message, false)
        );
    }

    pub fn print_info(message: &str) {
        println!("  {} {}", ansi_color("cyan", "ℹ", true), message);
    }
}

// Helper functions
pub fn ansi_color(color: &str, text: &str, bold: bool) -> String {
    let color_code = match color {
        "red" => "31",
        "green" => "32",
        "yellow" => "33",
        "blue" => "34",
        "magenta" => "35",
        "cyan" => "36",
        "white" => "37",
        "gray" => "90",
        _ => "37", // default to white
    };

    if bold {
        format!("\x1b[1;{}m{}\x1b[0m", color_code, text)
    } else {
        format!("\x1b[{}m{}\x1b[0m", color_code, text)
    }
}

fn bold(text: &str) -> String {
    format!("\x1b[1m{}\x1b[0m", text)
}

pub trait StringFormat {
    fn dimmed(&self) -> String;
}

impl StringFormat for str {
    fn dimmed(&self) -> String {
        format!("\x1b[2m{}\x1b[0m", self)
    }
}

fn get_context_color(context: &str) -> &str {
    match context {
        "terminal" => "cyan",
        "ide" => "magenta",
        "linked" => "yellow",
        "manual" => "blue",
        _ => "white",
    }
}

pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len.saturating_sub(1)])
    }
}

pub fn format_duration_clean(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}
