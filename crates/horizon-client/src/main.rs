//! Horizon: native Rust client for Minecraft Java Edition 26.3 gameplay.
#![forbid(unsafe_code)]
mod game;

use horizon_client::{crash, logging};

/// The mod API level this game serves, kept in the program for the launcher
/// to read without running it (`horizon_mods::api_level::MARK`). `main`
/// reads it, so the linker keeps it.
static MOD_API_LEVEL: [u8; 27] = horizon_mods::api_level::MARK;

fn main() {
    std::hint::black_box(&MOD_API_LEVEL);
    horizon_client::telemetry::mark_start();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let game_args = args.iter().take_while(|a| *a != "--dedicated");
    let help = game_args.clone().any(|a| a == "--help" || a == "-h");
    // The dedicated server logs in its own folder, as vanilla's does; the game in its home.
    let (home, side, thread) = if args.iter().any(|a| a == "--dedicated") {
        (std::path::PathBuf::from("."), "server", "Server thread")
    } else {
        (logging::game_home(), "client", "Render thread")
    };
    // iOS starts an app in `/`, which it can't write to: the game folder (options, saves, the
    // prepared resources) is the home in Documents.
    #[cfg(target_os = "ios")]
    {
        let _ = std::fs::create_dir_all(&home);
        let _ = std::env::set_current_dir(&home);
        // A phone's memory: the first start's options are small. The player can raise them.
        for (file, text) in [
            (
                "options.txt",
                "renderDistance:4\nsimulationDistance:5\nmaxFps:60\nrenderClouds:\"false\"\n\
                 particles:1\nentityShadows:false\nbiomeBlendRadius:0\nmipmapLevels:2\n",
            ),
            ("horizon-graphics.txt", "workers:2\nentityCulling:true\n"),
        ] {
            if !home.join(file).exists() {
                let _ = std::fs::write(home.join(file), text);
            }
        }
    }
    if !help {
        logging::init(&home, thread);
        // The log's first line names the build, so a bug report that carries the log says which one it was.
        log::info!("Horizon {}", crash::version());
        crash::install(home.clone(), side);
    }
    if let Err(error) = game::run() {
        log::error!("Horizon: {error}");
        if !help {
            crash::report_error(&home, side, &*error);
        }
        // Started from Explorer, the console window closes with the game:
        // keep the message on screen until it is read.
        #[cfg(windows)]
        if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
            eprintln!("Press Enter to close.");
            let _ = std::io::stdin().read_line(&mut String::new());
        }
        std::process::exit(1);
    }
    logging::flush();
}
