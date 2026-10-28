use crate::ipc::UserCommand;
use crate::protocol::{Buds4ProState, EqualizerPreset, NoiseControlMode, PlacementState};
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke, Vec2};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};

const EQ_BAND_LABELS: [&str; 9] = [
    "63Hz", "125Hz", "250Hz", "500Hz", "1kHz", "2kHz", "4kHz", "8kHz", "16kHz",
];

pub struct BudsGuiApp {
    state: Arc<RwLock<Buds4ProState>>,
    cmd_tx: mpsc::UnboundedSender<UserCommand>,
    show_gui_flag: Arc<AtomicBool>,
    quit_flag: Arc<AtomicBool>,
    snapshot: Buds4ProState,
    local_bands: [i8; 9],
    bands_dirty: bool,
    last_sync: Instant,
}

impl BudsGuiApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        state: Arc<RwLock<Buds4ProState>>,
        cmd_tx: mpsc::UnboundedSender<UserCommand>,
        show_gui_flag: Arc<AtomicBool>,
        quit_flag: Arc<AtomicBool>,
    ) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_rgb(15, 17, 23);
        visuals.window_fill = Color32::from_rgb(22, 25, 34);
        visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(26, 30, 42);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(35, 40, 56);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(52, 92, 210);
        visuals.widgets.active.bg_fill = Color32::from_rgb(66, 115, 245);
        visuals.selection.bg_fill = Color32::from_rgb(58, 108, 240);
        cc.egui_ctx.set_visuals(visuals);

        let snap = state.blocking_read().clone();
        let local_bands = snap.custom_eq_bands;

        Self {
            state,
            cmd_tx,
            show_gui_flag,
            quit_flag,
            snapshot: snap,
            local_bands,
            bands_dirty: false,
            last_sync: Instant::now(),
        }
    }

    fn send(&self, cmd: UserCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    fn render_battery_card(
        ui: &mut egui::Ui,
        title: &str,
        pct: u8,
        charging: bool,
        placement: Option<PlacementState>,
        temp_c: Option<f32>,
        volt_v: Option<f32>,
    ) {
        let frame = egui::Frame::NONE
            .fill(Color32::from_rgb(24, 28, 39))
            .corner_radius(CornerRadius::same(12))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(42, 48, 66)))
            .inner_margin(egui::Margin::same(14));

        frame.show(ui, |ui| {
            ui.set_min_width(175.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(title)
                            .size(15.0)
                            .strong()
                            .color(Color32::from_rgb(220, 226, 240)),
                    );
                    if charging {
                        ui.label(
                            RichText::new("⚡ Carregando")
                                .size(12.0)
                                .color(Color32::from_rgb(80, 220, 130)),
                        );
                    }
                });

                ui.add_space(6.0);

                if pct <= 100 && pct > 0 {
                    let bar_color = if pct > 50 {
                        Color32::from_rgb(56, 205, 120)
                    } else if pct > 20 {
                        Color32::from_rgb(240, 180, 50)
                    } else {
                        Color32::from_rgb(235, 75, 75)
                    };

                    ui.label(
                        RichText::new(format!("{pct}%"))
                            .size(28.0)
                            .strong()
                            .color(bar_color),
                    );
                    ui.add(
                        egui::ProgressBar::new((pct as f32) / 100.0)
                            .fill(bar_color)
                            .desired_height(8.0),
                    );
                } else {
                    ui.label(
                        RichText::new("--%")
                            .size(28.0)
                            .strong()
                            .color(Color32::from_rgb(110, 118, 135)),
                    );
                    ui.add(
                        egui::ProgressBar::new(0.0)
                            .fill(Color32::from_rgb(55, 60, 75))
                            .desired_height(8.0),
                    );
                }

                ui.add_space(6.0);
                if let Some(pl) = placement {
                    ui.label(
                        RichText::new(pl.label_pt())
                            .size(12.5)
                            .color(Color32::from_rgb(155, 165, 190)),
                    );
                } else {
                    ui.label(
                        RichText::new(if pct <= 100 && pct > 0 {
                            "Estojo aberto"
                        } else {
                            "Estojo fechado"
                        })
                        .size(12.5)
                        .color(Color32::from_rgb(155, 165, 190)),
                    );
                }

                if let (Some(t), Some(v)) = (temp_c, volt_v) {
                    if t > 0.0 {
                        let label = if v > 0.0 {
                            format!("🌡 {t:.1} °C  •  {v:.2} V")
                        } else {
                            format!("🌡 {t:.1} °C")
                        };
                        ui.label(
                            RichText::new(label)
                                .size(11.5)
                                .color(Color32::from_rgb(120, 130, 155)),
                        );
                    }
                }
            });
        });
    }
}

impl eframe::App for BudsGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.quit_flag.load(Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        if self.show_gui_flag.swap(false, Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }

        if self.last_sync.elapsed() >= Duration::from_millis(120) {
            if let Ok(guard) = self.state.try_read() {
                self.snapshot = guard.clone();
                if !self.bands_dirty {
                    self.local_bands = self.snapshot.custom_eq_bands;
                }
            }
            self.last_sync = Instant::now();
        }

        ctx.request_repaint_after(Duration::from_millis(200));

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(4.0);

                // Header
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(&self.snapshot.device_name)
                                .size(22.0)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        let sub = if self.snapshot.connected {
                            format!(
                                "● Conectado ({})  •  Cor: {}  •  FW: {}",
                                self.snapshot.mac_address,
                                self.snapshot.color_name,
                                if self.snapshot.firmware_version.is_empty() {
                                    "SM-R640"
                                } else {
                                    &self.snapshot.firmware_version
                                }
                            )
                        } else {
                            "○ Desconectado — Aguardando Galaxy Buds 4 Pro...".to_string()
                        };
                        let sub_color = if self.snapshot.connected {
                            Color32::from_rgb(90, 220, 140)
                        } else {
                            Color32::from_rgb(230, 150, 70)
                        };
                        ui.label(RichText::new(sub).size(12.5).color(sub_color));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("↻ Atualizar Sensores").clicked() {
                            self.send(UserCommand::RefreshDebug);
                        }
                    });
                });

                ui.add_space(12.0);

                // Battery Cards Row
                ui.horizontal_wrapped(|ui| {
                    Self::render_battery_card(
                        ui,
                        "Fone Esquerdo (L)",
                        self.snapshot.battery_left,
                        self.snapshot.is_left_charging,
                        Some(self.snapshot.placement_left),
                        Some(self.snapshot.temp_left_c),
                        Some(self.snapshot.voltage_left_v),
                    );
                    ui.add_space(8.0);
                    Self::render_battery_card(
                        ui,
                        "Fone Direito (R)",
                        self.snapshot.battery_right,
                        self.snapshot.is_right_charging,
                        Some(self.snapshot.placement_right),
                        Some(self.snapshot.temp_right_c),
                        Some(self.snapshot.voltage_right_v),
                    );
                    ui.add_space(8.0);
                    Self::render_battery_card(
                        ui,
                        "Estojo (Case)",
                        self.snapshot.battery_case,
                        self.snapshot.is_case_charging,
                        None,
                        None,
                        None,
                    );
                });

                ui.add_space(14.0);

                // Noise Control Card
                let card_frame = egui::Frame::NONE
                    .fill(Color32::from_rgb(24, 28, 39))
                    .corner_radius(CornerRadius::same(12))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(42, 48, 66)))
                    .inner_margin(egui::Margin::same(16));

                card_frame.show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label(
                        RichText::new("Controle de Ruído")
                            .size(16.0)
                            .strong()
                            .color(Color32::WHITE),
                    );
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        let modes = [
                            (NoiseControlMode::Off, "Desligado"),
                            (NoiseControlMode::Anc, "ANC (Cancelamento)"),
                            (NoiseControlMode::Ambient, "Som Ambiente"),
                            (NoiseControlMode::Adaptive, "Adaptativo"),
                        ];
                        for (mode, label) in modes {
                            let selected = self.snapshot.noise_control == mode;
                            let btn = egui::Button::new(
                                RichText::new(label).size(13.5).strong().color(if selected {
                                    Color32::WHITE
                                } else {
                                    Color32::from_rgb(185, 195, 215)
                                }),
                            )
                            .min_size(Vec2::new(130.0, 36.0))
                            .fill(if selected {
                                Color32::from_rgb(58, 108, 240)
                            } else {
                                Color32::from_rgb(35, 40, 56)
                            })
                            .corner_radius(CornerRadius::same(8));

                            if ui.add(btn).clicked() {
                                self.send(UserCommand::SetNoiseControl(mode));
                            }
                        }
                    });

                    ui.add_space(10.0);

                    if self.snapshot.noise_control == NoiseControlMode::Ambient {
                        ui.horizontal(|ui| {
                            ui.label("Volume do Som Ambiente (0 - 4):");
                            let mut vol = self.snapshot.ambient_volume;
                            if ui.add(egui::Slider::new(&mut vol, 0..=4)).changed() {
                                self.send(UserCommand::SetAmbientVolume(vol));
                            }
                        });
                    }

                    if self.snapshot.noise_control == NoiseControlMode::Anc {
                        ui.horizontal(|ui| {
                            ui.label("Nível de Cancelamento de Ruído (ANC 0 - 4):");
                            let mut level = self.snapshot.anc_level.min(4);
                            if ui.add(egui::Slider::new(&mut level, 0..=4)).changed() {
                                self.send(UserCommand::SetAncLevel(level));
                            }
                        });
                    }

                    ui.add_space(4.0);
                    let mut one_ear = self.snapshot.anc_with_one_earbud;
                    if ui
                        .checkbox(
                            &mut one_ear,
                            "Permitir Controle de Ruído usando apenas um fone",
                        )
                        .changed()
                    {
                        self.send(UserCommand::SetAncWithOneEarbud(one_ear));
                    }
                });

                ui.add_space(12.0);

                // Equalizer Card
                card_frame.show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label(
                        RichText::new("Equalizador & Som")
                            .size(16.0)
                            .strong()
                            .color(Color32::WHITE),
                    );
                    ui.add_space(8.0);

                    ui.horizontal_wrapped(|ui| {
                        for &preset in EqualizerPreset::all_presets() {
                            let selected = self.snapshot.equalizer == preset;
                            if ui.selectable_label(selected, preset.label_pt()).clicked() {
                                self.bands_dirty = false;
                                self.send(UserCommand::SetEqualizer(preset));
                            }
                        }
                    });

                    if self.snapshot.equalizer == EqualizerPreset::Custom {
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new("Equalizador Personalizado de 9 Bandas (-10 a +10 dB)")
                                .size(13.0)
                                .color(Color32::from_rgb(170, 180, 205)),
                        );
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            for i in 0..9 {
                                ui.vertical(|ui| {
                                    let mut val = self.local_bands[i] as i32;
                                    let slider = egui::Slider::new(&mut val, -10..=10)
                                        .vertical()
                                        .show_value(true);
                                    if ui.add_sized([36.0, 95.0], slider).changed() {
                                        self.local_bands[i] = val as i8;
                                        self.bands_dirty = true;
                                    }
                                    ui.label(RichText::new(EQ_BAND_LABELS[i]).size(11.0));
                                });
                            }
                        });
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if ui.button("Aplicar Curva nos Fones").clicked() {
                                self.bands_dirty = false;
                                self.send(UserCommand::SetCustomEqualizer(self.local_bands));
                            }
                            if ui.button("Zerar Bandas (Flat)").clicked() {
                                self.local_bands = [0; 9];
                                self.bands_dirty = false;
                                self.send(UserCommand::SetCustomEqualizer(self.local_bands));
                            }
                        });
                    }
                });

                ui.add_space(12.0);

                // Voice Detect & Advanced Audio Features
                card_frame.show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label(
                        RichText::new("Detecção de Voz & Recursos Avançados")
                            .size(16.0)
                            .strong()
                            .color(Color32::WHITE),
                    );
                    ui.add_space(8.0);

                    let mut vd = self.snapshot.voice_detect_enabled;
                    if ui
                        .checkbox(
                            &mut vd,
                            "Detectar Conversas (baixa o volume e liga Som Ambiente ao falar)",
                        )
                        .changed()
                    {
                        self.send(UserCommand::SetVoiceDetect(vd));
                    }

                    if self.snapshot.voice_detect_enabled {
                        ui.horizontal(|ui| {
                            ui.label("Tempo de retorno após parar de falar:");
                            for sec in [5u8, 10, 15] {
                                if ui
                                    .selectable_label(
                                        self.snapshot.voice_detect_timeout_sec == sec,
                                        format!("{sec}s"),
                                    )
                                    .clicked()
                                {
                                    self.send(UserCommand::SetVoiceDetectDuration(sec));
                                }
                            }
                        });
                    }

                    ui.add_space(6.0);

                    let mut game = self.snapshot.game_mode;
                    if ui
                        .checkbox(&mut game, "Modo Jogo (Reduzir latência de áudio Bluetooth)")
                        .changed()
                    {
                        self.send(UserCommand::SetGameMode(game));
                    }

                    let mut seamless = self.snapshot.seamless_connection;
                    if ui
                        .checkbox(
                            &mut seamless,
                            "Conexão Contínua entre dispositivos (Seamless Connection)",
                        )
                        .changed()
                    {
                        self.send(UserCommand::SetSeamlessConnection(seamless));
                    }

                    let mut sidetone = self.snapshot.sidetone_enabled;
                    if ui
                        .checkbox(&mut sidetone, "Ouvir a própria voz em chamadas (Sidetone)")
                        .changed()
                    {
                        self.send(UserCommand::SetSidetone(sidetone));
                    }

                    let mut clear_call = self.snapshot.extra_clear_call;
                    if ui
                        .checkbox(&mut clear_call, "Chamadas com Som Extra Claro")
                        .changed()
                    {
                        self.send(UserCommand::SetExtraClearCall(clear_call));
                    }
                });

                ui.add_space(12.0);

                // Touchpad & Find My Buds
                card_frame.show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label(
                        RichText::new("Controles de Toque (Haste) & Encontrar Meus Fones")
                            .size(16.0)
                            .strong()
                            .color(Color32::WHITE),
                    );
                    ui.add_space(8.0);

                    let mut locked = self.snapshot.touchpad_locked;
                    if ui
                        .checkbox(&mut locked, "Bloquear todos os controles de toque / pinça")
                        .changed()
                    {
                        self.send(UserCommand::SetTouchpadLock(locked));
                    }

                    if !self.snapshot.touchpad_locked {
                        ui.horizontal_wrapped(|ui| {
                            let mut s1 = self.snapshot.single_tap_on;
                            let mut s2 = self.snapshot.double_tap_on;
                            let mut s3 = self.snapshot.triple_tap_on;
                            let mut sh = self.snapshot.touch_hold_on;
                            let c1 = ui.checkbox(&mut s1, "Pinça simples").changed();
                            let c2 = ui.checkbox(&mut s2, "Pinça dupla").changed();
                            let c3 = ui.checkbox(&mut s3, "Pinça tripla").changed();
                            let ch = ui.checkbox(&mut sh, "Pinçar e segurar").changed();
                            if c1 || c2 || c3 || ch {
                                self.send(UserCommand::SetTouchGestures {
                                    single: s1,
                                    double: s2,
                                    triple: s3,
                                    hold: sh,
                                    double_call: self.snapshot.double_tap_call_on,
                                    hold_call: self.snapshot.touch_hold_call_on,
                                });
                            }
                        });
                    }

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Encontrar Meus Fones:").strong());
                        if !self.snapshot.find_my_buds_active {
                            if ui.button("🔊 Tocar Alerta Sonoro").clicked() {
                                self.send(UserCommand::SetFindMyBuds(true));
                            }
                        } else {
                            if ui.button("⏹ Parar Alerta").clicked() {
                                self.send(UserCommand::SetFindMyBuds(false));
                            }
                            let mut ml = self.snapshot.mute_left;
                            let mut mr = self.snapshot.mute_right;
                            if ui.checkbox(&mut ml, "Silenciar Esquerdo").changed()
                                || ui.checkbox(&mut mr, "Silenciar Direito").changed()
                            {
                                self.send(UserCommand::SetMuteEarbuds {
                                    left: ml,
                                    right: mr,
                                });
                            }
                        }
                    });
                });

                ui.add_space(10.0);
                // Footer Telemetry Info
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "HW: {}  |  SKU L/R: {} / {}  |  Serial L/R: {} / {}  |  Rev Protocolo: {}",
                            if self.snapshot.hardware_version.is_empty() {
                                "N/D"
                            } else {
                                &self.snapshot.hardware_version
                            },
                            if self.snapshot.sku_left.is_empty() {
                                "SM-R640"
                            } else {
                                &self.snapshot.sku_left
                            },
                            if self.snapshot.sku_right.is_empty() {
                                "SM-R640"
                            } else {
                                &self.snapshot.sku_right
                            },
                            if self.snapshot.serial_left.is_empty() {
                                "--"
                            } else {
                                &self.snapshot.serial_left
                            },
                            if self.snapshot.serial_right.is_empty() {
                                "--"
                            } else {
                                &self.snapshot.serial_right
                            },
                            self.snapshot.revision
                        ))
                        .size(11.0)
                        .color(Color32::from_rgb(115, 125, 148)),
                    );
                });
            });
        });
    }
}
