use restident_evil_5::on_process_attach;

pub fn main() {
    on_process_attach(0);
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
