// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use zonendpoint_lib::scanner::ScanStatus;

fn scan_exit_code(status: &ScanStatus) -> i32 {
    if matches!(status, ScanStatus::Clean) {
        0
    } else {
        1
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 && args[1] == "--scan" {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("create Tokio runtime");
        let result = runtime.block_on(zonendpoint_lib::scan_cli(args[2].clone().into()));
        println!("{:?}: {}", result.status, result.path);
        std::process::exit(scan_exit_code(&result.status));
    }
    zonendpoint_lib::run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_exit_success_requires_clean_for_all_typed_outcomes() {
        for (status, expected) in [
            (ScanStatus::Clean, 0),
            (ScanStatus::Infected, 1),
            (ScanStatus::Unknown, 1),
            (ScanStatus::Error, 1),
            (ScanStatus::Skipped, 1),
            (ScanStatus::Cancelled, 1),
        ] {
            assert_eq!(scan_exit_code(&status), expected, "{status:?}");
        }
    }
}
