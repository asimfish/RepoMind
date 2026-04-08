fn main() {
    // Load .env file for local development (ignored if not present)
    if let Ok(content) = std::fs::read_to_string(".env") {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, val)) = line.split_once('=') {
                let key = key.trim();
                let val = val.trim().trim_matches('"');
                // Tell cargo to rerun if .env changes, and set the env var
                println!("cargo:rustc-env={}={}", key, val);
            }
        }
        println!("cargo:rerun-if-changed=.env");
    }

    tauri_build::build()
}
