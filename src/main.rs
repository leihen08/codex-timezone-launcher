#![windows_subsystem = "windows"]

fn main() {
    if chatgpt_timezone_launcher::store_launch::run_resumer_if_requested() {
        return;
    }
    if let Err(message) = chatgpt_timezone_launcher::ui::run() {
        chatgpt_timezone_launcher::ui::show_fatal(&message);
    }
}
