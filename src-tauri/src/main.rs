// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;

fn main() {
    // Check if running in web mode
    let args: Vec<String> = env::args().collect();
    
    if args.contains(&"--web".to_string()) {
        // Web mode: start HTTP server
        let port = args
            .iter()
            .position(|arg| arg == "--port")
            .and_then(|idx| args.get(idx + 1))
            .and_then(|s| s.parse().ok())
            .unwrap_or(3001);
        
        println!("Starting OpenFoot Manager Web Server on port {}...", port);
        println!("Open http://localhost:{} in your browser", port);
        
        let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        rt.block_on(async {
            openfootmanager_lib::run_web(port)
                .await
                .expect("Failed to start web server");
        });
    } else {
        // Tauri desktop mode
        openfootmanager_lib::run();
    }
}
