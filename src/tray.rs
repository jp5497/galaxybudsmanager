use crate::ipc::UserCommand;
use crate::protocol::{Buds4ProState, EqualizerPreset, NoiseControlMode};
use ksni::menu::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

const TRAY_ICON_ARGB: &[u8] = include_bytes!("../assets/tray_icon_64.argb");

fn custom_tray_pixmap() -> Vec<ksni::Icon> {
    vec![ksni::Icon {
        width: 64,
        height: 64,
        data: TRAY_ICON_ARGB.to_vec(),
    }]
}

pub struct BudsTray {
    pub snapshot: Buds4ProState,
    pub cmd_tx: mpsc::UnboundedSender<UserCommand>,
    pub show_gui_flag: Arc<AtomicBool>,
    pub quit_flag: Arc<AtomicBool>,
    pub show_gui_option: bool,
}

impl ksni::Tray for BudsTray {
    fn id(&self) -> String {
        "buds4pro-rust-manager".to_string()
    }

    fn icon_name(&self) -> String {
        String::new()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        custom_tray_pixmap()
    }

    fn title(&self) -> String {
        if self.snapshot.connected {
            format!(
                "{} (L:{}% R:{}%)",
                self.snapshot.device_name, self.snapshot.battery_left, self.snapshot.battery_right
            )
        } else {
            "Galaxy Buds4 Pro (Desconectado)".to_string()
        }
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        let description = if self.snapshot.connected {
            let case_str = if self.snapshot.battery_case <= 100 {
                format!("{}%", self.snapshot.battery_case)
            } else {
                "N/D".to_string()
            };
            format!(
                "Esquerdo: {}% | Direito: {}% | Estojo: {}\nModo: {} | EQ: {}",
                self.snapshot.battery_left,
                self.snapshot.battery_right,
                case_str,
                self.snapshot.noise_control.short_pt(),
                self.snapshot.equalizer.label_pt()
            )
        } else {
            "Aguardando conexão com o Galaxy Buds4 Pro...".to_string()
        };

        ksni::ToolTip {
            icon_name: String::new(),
            icon_pixmap: custom_tray_pixmap(),
            title: self.snapshot.device_name.clone(),
            description,
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        self.show_gui_flag.store(true, Ordering::SeqCst);
        let _ = self.cmd_tx.send(UserCommand::ShowGui);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        let s = &self.snapshot;
        let mut items: Vec<ksni::MenuItem<Self>> = Vec::new();

        if self.show_gui_option {
            items.push(
                StandardItem {
                    label: "Abrir Painel Gráfico (GUI)".to_string(),
                    icon_name: "preferences-desktop-sound".to_string(),
                    activate: Box::new(|this: &mut Self| {
                        this.show_gui_flag.store(true, Ordering::SeqCst);
                        let _ = this.cmd_tx.send(UserCommand::ShowGui);
                    }),
                    ..Default::default()
                }
                .into(),
            );
            items.push(MenuItem::Separator);
        }

        let status_label = if s.connected {
            let case_txt = if s.battery_case <= 100 {
                format!("{}%", s.battery_case)
            } else {
                "--".to_string()
            };
            format!(
                "Bateria: L {}% | R {}% | Case {}",
                s.battery_left, s.battery_right, case_txt
            )
        } else {
            "Status: Desconectado".to_string()
        };

        items.push(
            StandardItem {
                label: status_label,
                enabled: false,
                icon_name: "battery-good".to_string(),
                ..Default::default()
            }
            .into(),
        );

        items.push(MenuItem::Separator);

        // Noise Control Radio Group
        let current_nc = match s.noise_control {
            NoiseControlMode::Anc => 0,
            NoiseControlMode::Ambient => 1,
            NoiseControlMode::Adaptive => 2,
            NoiseControlMode::Off => 3,
        };

        items.push(
            RadioGroup {
                selected: current_nc,
                select: Box::new(|this: &mut Self, idx| {
                    let mode = match idx {
                        0 => NoiseControlMode::Anc,
                        1 => NoiseControlMode::Ambient,
                        2 => NoiseControlMode::Adaptive,
                        _ => NoiseControlMode::Off,
                    };
                    let _ = this.cmd_tx.send(UserCommand::SetNoiseControl(mode));
                }),
                options: vec![
                    RadioItem {
                        label: "Ruído: Cancelamento Ativo (ANC)".to_string(),
                        ..Default::default()
                    },
                    RadioItem {
                        label: "Ruído: Som Ambiente".to_string(),
                        ..Default::default()
                    },
                    RadioItem {
                        label: "Ruído: Adaptativo".to_string(),
                        ..Default::default()
                    },
                    RadioItem {
                        label: "Ruído: Desligado".to_string(),
                        ..Default::default()
                    },
                ],
            }
            .into(),
        );

        items.push(MenuItem::Separator);

        // Equalizer SubMenu
        let eq_presets = EqualizerPreset::all_presets();
        let selected_eq = eq_presets
            .iter()
            .position(|&p| p == s.equalizer)
            .unwrap_or(0);

        items.push(
            SubMenu {
                label: format!("Equalizador ({})", s.equalizer.label_pt()),
                icon_name: "media-eq".to_string(),
                submenu: vec![RadioGroup {
                    selected: selected_eq,
                    select: Box::new(|this: &mut Self, idx| {
                        if let Some(&preset) = EqualizerPreset::all_presets().get(idx) {
                            let _ = this.cmd_tx.send(UserCommand::SetEqualizer(preset));
                        }
                    }),
                    options: eq_presets
                        .iter()
                        .map(|p| RadioItem {
                            label: p.label_pt().to_string(),
                            ..Default::default()
                        })
                        .collect(),
                }
                .into()],
                ..Default::default()
            }
            .into(),
        );

        // Quick Toggles
        items.push(
            CheckmarkItem {
                label: "Detectar Conversas (Voice Detect)".to_string(),
                checked: s.voice_detect_enabled,
                activate: Box::new(|this: &mut Self| {
                    let next = !this.snapshot.voice_detect_enabled;
                    let _ = this.cmd_tx.send(UserCommand::SetVoiceDetect(next));
                }),
                ..Default::default()
            }
            .into(),
        );

        items.push(
            CheckmarkItem {
                label: "Bloquear Touchpad dos Fones".to_string(),
                checked: s.touchpad_locked,
                activate: Box::new(|this: &mut Self| {
                    let next = !this.snapshot.touchpad_locked;
                    let _ = this.cmd_tx.send(UserCommand::SetTouchpadLock(next));
                }),
                ..Default::default()
            }
            .into(),
        );

        items.push(
            CheckmarkItem {
                label: "Modo Jogo (Baixa Latência)".to_string(),
                checked: s.game_mode,
                activate: Box::new(|this: &mut Self| {
                    let next = !this.snapshot.game_mode;
                    let _ = this.cmd_tx.send(UserCommand::SetGameMode(next));
                }),
                ..Default::default()
            }
            .into(),
        );

        items.push(MenuItem::Separator);

        items.push(
            StandardItem {
                label: "Sair Completamente".to_string(),
                icon_name: "application-exit".to_string(),
                activate: Box::new(|this: &mut Self| {
                    this.quit_flag.store(true, Ordering::SeqCst);
                    let _ = this.cmd_tx.send(UserCommand::QuitDaemon);
                }),
                ..Default::default()
            }
            .into(),
        );

        items
    }
}

pub fn spawn_system_tray(
    state: Arc<RwLock<Buds4ProState>>,
    cmd_tx: mpsc::UnboundedSender<UserCommand>,
    show_gui_flag: Arc<AtomicBool>,
    quit_flag: Arc<AtomicBool>,
    show_gui_option: bool,
) {
    let initial = Buds4ProState::default();
    let service = ksni::TrayService::new(BudsTray {
        snapshot: initial,
        cmd_tx,
        show_gui_flag,
        quit_flag,
        show_gui_option,
    });
    let handle = service.handle();
    service.spawn();

    tokio::spawn(async move {
        loop {
            let snap = state.read().await.clone();
            handle.update(|tray: &mut BudsTray| {
                tray.snapshot = snap;
            });
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });
}
