#![windows_subsystem = "windows"]

fn main() {
    if let Err(message) = chatgpt_timezone_launcher::ui::run() {
        chatgpt_timezone_launcher::ui::show_fatal(&message);
    }
}
