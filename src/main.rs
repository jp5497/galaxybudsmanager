mod bluetooth;
mod gui;
mod ipc;
mod protocol;
mod tray;

use anyhow::Result;
use bluetooth::{send_or_execute_oneshot, start_background_manager};
use clap::{Parser, Subcommand, ValueEnum};
use gui::BudsGuiApp;
use ipc::UserCommand;
use protocol::{Buds4ProState, EqualizerPreset, NoiseControlMode};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tray::spawn_system_tray;

#[derive(Parser)]
#[command(
    name = "buds4pro",
    version = "0.2.0",
    about = "Gerenciador completo em Rust para Samsung Galaxy Buds 4 Pro (CLI, System Tray e GUI)"
)]
struct Cli {
    /// Exibir saída em formato JSON (ideal para Waybar, Polybar e scripts)
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Abre a interface gráfica completa + ícone na bandeja do sistema (padrão)
    Gui,
    /// Inicia apenas o daemon em segundo plano + ícone na bandeja do sistema (System Tray)
    Daemon,
    /// Mostra o status completo dos fones (bateria, sensores, modo de ruído, equalizador)
    Status,
    /// Define ou alterna o modo de Controle de Ruído (off, anc, ambient, adaptive, cycle)
    Noise {
        #[arg(value_enum)]
        mode: CliNoiseMode,
    },
    /// Define o nível de volume do Som Ambiente (0 a 4)
    AmbientVolume {
        #[arg(value_parser = clap::value_parser!(u8).range(0..=4))]
        level: u8,
    },
    /// Define o nível do Cancelamento de Ruído ANC (0 a 4)
    AncLevel {
        #[arg(value_parser = clap::value_parser!(u8).range(0..=4))]
        level: u8,
    },
    /// Altera o preset do Equalizador
    Eq {
        #[arg(value_enum)]
        preset: CliEqPreset,
    },
    /// Define o Equalizador Personalizado de 9 bandas (-10 a +10 dB)
    CustomEq {
        #[arg(num_args = 9, allow_hyphen_values = true)]
        bands: Vec<i8>,
    },
    /// Ativa ou desativa a Detecção de Conversas (Voice Detect)
    VoiceDetect {
        #[arg(value_enum)]
        state: OnOff,
        /// Tempo de retorno em segundos (5, 10 ou 15)
        #[arg(short, long)]
        timeout: Option<u8>,
    },
    /// Bloqueia ou desbloqueia os controles de toque na haste dos fones
    Touchpad {
        #[arg(value_enum)]
        state: LockState,
    },
    /// Ativa ou desativa o Modo Jogo (baixa latência)
    GameMode {
        #[arg(value_enum)]
        state: OnOff,
    },
    /// Toca ou para o alerta sonoro de "Encontrar Meus Fones"
    Find {
        #[arg(value_enum)]
        action: StartStop,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum CliNoiseMode {
    Off,
    Anc,
    Ambient,
    Adaptive,
    Cycle,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum CliEqPreset {
    Off,
    Bass,
    Soft,
    Dynamic,
    Clear,
    Treble,
    Custom,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum OnOff {
    On,
    Off,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum LockState {
    Lock,
    Unlock,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum StartStop {
    Start,
    Stop,
}

fn print_status(s: &Buds4ProState, as_json: bool) {
    if as_json {
        if let Ok(j) = serde_json::to_string_pretty(s) {
            println!("{j}");
        }
        return;
    }

    println!("╭──────────────────────────────────────────────────────────╮");
    println!(
        "│  🎧 {:<35} {:>16} │",
        s.device_name,
        if s.connected {
            "● CONECTADO"
        } else {
            "○ DESCONECTADO"
        }
    );
    println!("├──────────────────────────────────────────────────────────┤");
    println!(
        "│  MAC: {:<20} Cor: {:<24} │",
        if s.mac_address.is_empty() {
            "N/D"
        } else {
            &s.mac_address
        },
        s.color_name
    );
    println!(
        "│  Firmware: {:<15} Hardware: {:<19} │",
        if s.firmware_version.is_empty() {
            "SM-R640"
        } else {
            &s.firmware_version
        },
        if s.hardware_version.is_empty() {
            "N/D"
        } else {
            &s.hardware_version
        }
    );
    println!("├──────────────────────────────────────────────────────────┤");
    println!(
        "│  🔋 Fone Esquerdo (L): {:>3}% {} ({:<14}) │",
        s.battery_left,
        if s.is_left_charging { "⚡" } else { "  " },
        s.placement_left.label_pt()
    );
    println!(
        "│  🔋 Fone Direito  (R): {:>3}% {} ({:<14}) │",
        s.battery_right,
        if s.is_right_charging { "⚡" } else { "  " },
        s.placement_right.label_pt()
    );
    let case_str = if s.battery_case <= 100 && s.battery_case > 0 {
        format!("{:>3}%", s.battery_case)
    } else {
        "N/D ".to_string()
    };
    println!(
        "│  🔋 Estojo     (Case): {} {}                           │",
        case_str,
        if s.is_case_charging { "⚡" } else { "  " }
    );
    if s.temp_left_c > 0.0 || s.temp_right_c > 0.0 {
        println!(
            "│  🌡️  Temperatura     : {:.1}°C (Sensores internos)         │",
            s.temp_right_c.max(s.temp_left_c)
        );
    }
    println!("├──────────────────────────────────────────────────────────┤");
    println!(
        "│  🔇 Controle de Ruído: {:<33} │",
        s.noise_control.label_pt()
    );
    println!(
        "│  🎚️  Vol. Ambiente   : {:<5} Nível ANC: {:<16} │",
        format!("{}/4", s.ambient_volume),
        format!("{}/4", s.anc_level)
    );
    println!(
        "│  🎵 Equalizador      : {:<33} │",
        s.equalizer.label_pt()
    );
    println!(
        "│  🗣️  Detectar Voz     : {:<33} │",
        if s.voice_detect_enabled {
            format!("Ativado ({}s)", s.voice_detect_timeout_sec)
        } else {
            "Desativado".to_string()
        }
    );
    println!(
        "│  👆 Touchpad         : {:<33} │",
        if s.touchpad_locked {
            "Bloqueado"
        } else {
            "Desbloqueado"
        }
    );
    println!(
        "│  🎮 Modo Jogo        : {:<33} │",
        if s.game_mode { "Ativado" } else { "Desativado" }
    );
    println!("╰──────────────────────────────────────────────────────────╯");
}

fn run_gui_mode() -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let mgr = rt.block_on(async {
        let m = start_background_manager().await?;
        spawn_system_tray(m.state.clone(), m.cmd_tx.clone(), true);
        Ok::<_, anyhow::Error>(m)
    })?;

    let icon_data = std::sync::Arc::new(eframe::egui::IconData {
        rgba: include_bytes!("../assets/window_icon_64.rgba").to_vec(),
        width: 64,
        height: 64,
    });

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([760.0, 740.0])
            .with_min_inner_size([640.0, 620.0])
            .with_app_id("buds4pro")
            .with_icon(icon_data)
            .with_title("Galaxy Buds4 Pro — Gerenciador Linux"),
        ..Default::default()
    };

    let state = mgr.state.clone();
    let cmd_tx = mgr.cmd_tx.clone();
    let show_gui_flag = mgr.show_gui_flag.clone();
    let quit_flag = mgr.quit_flag.clone();

    eframe::run_native(
        "buds4pro",
        native_options,
        Box::new(move |cc| {
            Ok(Box::new(BudsGuiApp::new(
                cc,
                state,
                cmd_tx,
                show_gui_flag,
                quit_flag,
            )))
        }),
    )
    .map_err(|e| anyhow::anyhow!("Erro ao iniciar GUI: {e}"))?;

    let _ = std::fs::remove_file(ipc::IPC_SOCKET_PATH);
    Ok(())
}

async fn run_daemon_mode() -> Result<()> {
    let mgr = start_background_manager().await?;
    spawn_system_tray(mgr.state.clone(), mgr.cmd_tx.clone(), false);
    println!(
        "🚀 Daemon Galaxy Buds4 Pro iniciado! Ícone disponível na bandeja do sistema (System Tray)."
    );
    println!("   Socket IPC ativo em: {}", ipc::IPC_SOCKET_PATH);

    while !mgr.quit_flag.load(Ordering::SeqCst) {
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    let _ = std::fs::remove_file(ipc::IPC_SOCKET_PATH);
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Gui) {
        Commands::Gui => run_gui_mode(),
        Commands::Daemon => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            rt.block_on(run_daemon_mode())
        }
        cmd => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            rt.block_on(async {
                let state = match cmd {
                    Commands::Status => send_or_execute_oneshot(UserCommand::GetStatus).await?,
                    Commands::Noise { mode } => {
                        let user_cmd = match mode {
                            CliNoiseMode::Off => UserCommand::SetNoiseControl(NoiseControlMode::Off),
                            CliNoiseMode::Anc => UserCommand::SetNoiseControl(NoiseControlMode::Anc),
                            CliNoiseMode::Ambient => {
                                UserCommand::SetNoiseControl(NoiseControlMode::Ambient)
                            }
                            CliNoiseMode::Adaptive => {
                                UserCommand::SetNoiseControl(NoiseControlMode::Adaptive)
                            }
                            CliNoiseMode::Cycle => UserCommand::CycleNoiseControl,
                        };
                        send_or_execute_oneshot(user_cmd).await?
                    }
                    Commands::AmbientVolume { level } => {
                        send_or_execute_oneshot(UserCommand::SetAmbientVolume(level)).await?
                    }
                    Commands::AncLevel { level } => {
                        send_or_execute_oneshot(UserCommand::SetAncLevel(level)).await?
                    }
                    Commands::Eq { preset } => {
                        let p = match preset {
                            CliEqPreset::Off => EqualizerPreset::Off,
                            CliEqPreset::Bass => EqualizerPreset::BassBoost,
                            CliEqPreset::Soft => EqualizerPreset::Soft,
                            CliEqPreset::Dynamic => EqualizerPreset::Dynamic,
                            CliEqPreset::Clear => EqualizerPreset::Clear,
                            CliEqPreset::Treble => EqualizerPreset::TrebleBoost,
                            CliEqPreset::Custom => EqualizerPreset::Custom,
                        };
                        send_or_execute_oneshot(UserCommand::SetEqualizer(p)).await?
                    }
                    Commands::CustomEq { bands } => {
                        let mut arr = [0i8; 9];
                        for (i, &b) in bands.iter().take(9).enumerate() {
                            arr[i] = b.clamp(-10, 10);
                        }
                        send_or_execute_oneshot(UserCommand::SetCustomEqualizer(arr)).await?
                    }
                    Commands::VoiceDetect { state, timeout } => {
                        let enabled = state == OnOff::On;
                        let mut s =
                            send_or_execute_oneshot(UserCommand::SetVoiceDetect(enabled)).await?;
                        if let Some(t) = timeout {
                            s = send_or_execute_oneshot(UserCommand::SetVoiceDetectDuration(t))
                                .await?;
                        }
                        s
                    }
                    Commands::Touchpad { state } => {
                        let lock = state == LockState::Lock;
                        send_or_execute_oneshot(UserCommand::SetTouchpadLock(lock)).await?
                    }
                    Commands::GameMode { state } => {
                        let enabled = state == OnOff::On;
                        send_or_execute_oneshot(UserCommand::SetGameMode(enabled)).await?
                    }
                    Commands::Find { action } => {
                        let start = action == StartStop::Start;
                        send_or_execute_oneshot(UserCommand::SetFindMyBuds(start)).await?
                    }
                    Commands::Gui | Commands::Daemon => unreachable!(),
                };

                print_status(&state, cli.json);
                Ok(())
            })
        }
    }
}
