#![windows_subsystem = "windows"]

fn main() {
    if chatgpt_timezone_launcher::store_launch::run_resumer_if_requested() {
        return;
    }

    let _single_instance = match chatgpt_timezone_launcher::single_instance::acquire() {
        Ok(chatgpt_timezone_launcher::single_instance::AcquireResult::Acquired(guard)) => guard,
        Ok(chatgpt_timezone_launcher::single_instance::AcquireResult::AlreadyRunning) => {
            chatgpt_timezone_launcher::ui::focus_existing_window();
            return;
        }
        Err(message) => {
            chatgpt_timezone_launcher::ui::show_fatal(&message);
            return;
        }
    };

    if let Err(message) = chatgpt_timezone_launcher::ui::run() {
        chatgpt_timezone_launcher::ui::show_fatal(&message);
    }
}
