//! ANSI codes and helper functions for text formatting
use chrono::Local;
// Color codes
pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const BLUE: &str = "\x1b[34m";
pub const MAGENTA: &str = "\x1b[35m";
pub const CYAN: &str = "\x1b[36m";
// Text styles
pub const BOLD: &str = "\x1b[1m";
pub const UNDERLINE: &str = "\x1b[4m";
pub const ITALIC: &str = "\x1b[3m";
// Reset code
pub const RESET: &str = "\x1b[0m";
/// Returns the current time in a formatted manner.
pub fn get_time() -> String {
    Local::now().format("%H:%M:%S").to_string()
}
