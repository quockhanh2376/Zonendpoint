// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 && args[1] == "--scan" {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("create Tokio runtime");
        let result = runtime.block_on(zonendpoint_lib::scan_cli(args[2].clone().into()));
        println!("{:?}: {}", result.status, result.path);
        if matches!(
            result.status,
            zonendpoint_lib::scanner::ScanStatus::Error
                | zonendpoint_lib::scanner::ScanStatus::Unknown
        ) {
            std::process::exit(1);
        }
        return;
    }
    zonendpoint_lib::run()
}
