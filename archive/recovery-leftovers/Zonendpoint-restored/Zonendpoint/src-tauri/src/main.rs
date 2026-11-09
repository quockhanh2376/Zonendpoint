// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 && args[1] == "--scan" {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("create Tokio runtime");
        match runtime.block_on(zonendpoint_lib::scanner::scan_file(&args[2])) {
            Ok(result) => println!("{}: {}", result.verdict, result.path),
            Err(error) => { eprintln!("Scan failed: {error}"); std::process::exit(1); }
        }
        return;
    }
    zonendpoint_lib::run()
}
