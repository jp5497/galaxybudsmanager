use crate::ipc::UserCommand;
use crate::protocol::{Buds4ProState, EqualizerPreset, NoiseControlMode, PlacementState};
use eframe::egui::{
    self, Align, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Id, Layout, Margin,
    Pos2, Rect, Response, RichText, Sense, Stroke, Ui, UiBuilder, Vec2,
};
use std::f32::consts::PI;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};

// --- Color Palette (One UI 8 / Modern Obsidian Dark) ---
const BG_BASE: Color32 = Color32::from_rgb(11, 14, 20);
const BG_CARD: Color32 = Color32::from_rgb(19, 24, 34);
const BG_SUBTLE: Color32 = Color32::from_rgb(15, 19, 28);
const BORDER_SUBTLE: Color32 = Color32::from_rgb(33, 42, 58);
const ACCENT_BLUE: Color32 = Color32::from_rgb(59, 130, 246);
const ACCENT_CYAN: Color32 = Color32::from_rgb(56, 189, 248);
const ACCENT_TINT: Color32 = Color32::from_rgb(21, 36, 66);
const STATUS_GREEN: Color32 = Color32::from_rgb(16, 185, 129);
const STATUS_AMBER: Color32 = Color32::from_rgb(245, 158, 11);
const STATUS_RED: Color32 = Color32::from_rgb(239, 68, 68);
const TEXT_PRIMARY: Color32 = Color32::from_rgb(244, 247, 252);
const TEXT_SECONDARY: Color32 = Color32::from_rgb(148, 163, 184);
const TEXT_MUTED: Color32 = Color32::from_rgb(94, 109, 130);

const EQ_BAND_LABELS: [&str; 9] = [
    "63", "125", "250", "500", "1k", "2k", "4k", "8k", "16k",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveTab {
    SoundControl = 0,
    Equalizer = 1,
    TouchAndSystem = 2,
}

impl ActiveTab {
    fn index(self) -> usize {
        self as usize
    }
}

fn ease_out_cubic(t: f32) -> f32 {
    let clamped = t.clamp(0.0, 1.0);
    1.0 - (1.0 - clamped).powi(3)
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}

pub struct BudsGuiApp {
    state: Arc<RwLock<Buds4ProState>>,
    cmd_tx: mpsc::UnboundedSender<UserCommand>,
    show_gui_flag: Arc<AtomicBool>,
    quit_flag: Arc<AtomicBool>,
    snapshot: Buds4ProState,
    active_tab: ActiveTab,
    local_bands: [i8; 9],
    bands_dirty: bool,
    last_sync: Instant,

    // Animation state
    launch_instant: Instant,
    last_frame_instant: Instant,
    anim_batt_left: f32,
    anim_batt_right: f32,
    anim_batt_case: f32,
    anim_eq_bands: [f32; 9],
    tab_transition_instant: Instant,
    tab_slide_dir: f32,
}

impl BudsGuiApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        state: Arc<RwLock<Buds4ProState>>,
        cmd_tx: mpsc::UnboundedSender<UserCommand>,
        show_gui_flag: Arc<AtomicBool>,
        quit_flag: Arc<AtomicBool>,
    ) -> Self {
        Self::configure_fonts(&cc.egui_ctx);
        Self::configure_theme(&cc.egui_ctx);

        let snap = state.blocking_read().clone();
        let local_bands = snap.custom_eq_bands;
        let now = Instant::now();

        Self {
            state,
            cmd_tx,
            show_gui_flag,
            quit_flag,
            snapshot: snap,
            active_tab: ActiveTab::SoundControl,
            local_bands,
            bands_dirty: false,
            last_sync: now,
            launch_instant: now,
            last_frame_instant: now,
            // Start battery gauges at 0.0 so they sweep up smoothly on launch!
            anim_batt_left: 0.0,
            anim_batt_right: 0.0,
            anim_batt_case: 0.0,
            anim_eq_bands: [0.0; 9],
            tab_transition_instant: now,
            tab_slide_dir: 1.0,
        }
    }

    fn configure_fonts(ctx: &egui::Context) {
        let mut fonts = FontDefinitions::default();
        if let Ok(bytes) = std::fs::read("/usr/share/fonts/noto/NotoSans-Medium.ttf") {
            fonts
                .font_data
                .insert("NotoSans".to_owned(), FontData::from_owned(bytes).into());
            if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
                family.insert(0, "NotoSans".to_owned());
            }
        } else if let Ok(bytes) = std::fs::read("/usr/share/fonts/noto/NotoSans-Regular.ttf") {
            fonts
                .font_data
                .insert("NotoSans".to_owned(), FontData::from_owned(bytes).into());
            if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
                family.insert(0, "NotoSans".to_owned());
            }
        }
        ctx.set_fonts(fonts);
    }

    fn configure_theme(ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(10.0, 10.0);
        style.spacing.window_margin = Margin::same(18);
        style.spacing.button_padding = Vec2::new(14.0, 8.0);
        style.animation_time = 0.22;

        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = BG_BASE;
        visuals.window_fill = BG_BASE;
        visuals.override_text_color = Some(TEXT_PRIMARY);
        visuals.widgets.noninteractive.bg_fill = BG_CARD;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(28, 36, 50);
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
        visuals.widgets.inactive.corner_radius = CornerRadius::same(10);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(38, 49, 68);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT_BLUE);
        visuals.widgets.hovered.corner_radius = CornerRadius::same(10);
        visuals.widgets.active.bg_fill = ACCENT_BLUE;
        visuals.widgets.active.corner_radius = CornerRadius::same(10);
        visuals.selection.bg_fill = ACCENT_BLUE;
        style.visuals = visuals;
        ctx.set_style(style);
    }

    fn send(&self, cmd: UserCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    fn switch_tab(&mut self, next_tab: ActiveTab) {
        if self.active_tab != next_tab {
            self.tab_slide_dir = if next_tab.index() > self.active_tab.index() {
                1.0
            } else {
                -1.0
            };
            self.active_tab = next_tab;
            self.tab_transition_instant = Instant::now();
        }
    }

    /// Updates continuous animations (battery sweep-up, EQ curve morphing, tab transitions)
    /// and returns `true` if another 60fps frame is needed immediately.
    fn step_animations(&mut self) -> bool {
        let now = Instant::now();
        let dt = now
            .duration_since(self.last_frame_instant)
            .as_secs_f32()
            .clamp(0.001, 0.05);
        self.last_frame_instant = now;

        let since_launch = now.duration_since(self.launch_instant).as_secs_f32();
        let mut animating = false;

        // Staggered battery count-up animation (L starts at 0.06s, R at 0.16s, Case at 0.26s)
        let target_l = if self.snapshot.battery_left <= 100 && since_launch > 0.06 {
            self.snapshot.battery_left as f32
        } else {
            0.0
        };
        let target_r = if self.snapshot.battery_right <= 100 && since_launch > 0.16 {
            self.snapshot.battery_right as f32
        } else {
            0.0
        };
        let target_c = if self.snapshot.battery_case <= 100 && since_launch > 0.26 {
            self.snapshot.battery_case as f32
        } else {
            0.0
        };

        let speed = 5.2_f32;
        for (curr, target) in [
            (&mut self.anim_batt_left, target_l),
            (&mut self.anim_batt_right, target_r),
            (&mut self.anim_batt_case, target_c),
        ] {
            let diff = target - *curr;
            if diff.abs() > 0.15 {
                // Smooth exponential approach + minimum velocity so it reaches exact integer cleanly
                let step = diff * (speed * dt).min(1.0) + diff.signum() * 8.0 * dt;
                if step.abs() >= diff.abs() {
                    *curr = target;
                } else {
                    *curr += step;
                }
                animating = true;
            } else {
                *curr = target;
            }
        }

        // Smooth Equalizer 9-Band curve morphing
        let is_custom = self.snapshot.equalizer == EqualizerPreset::Custom;
        let target_bands = if is_custom || self.bands_dirty {
            self.local_bands.map(|v| v as f32)
        } else {
            Self::preset_curve_preview(self.snapshot.equalizer, &self.local_bands)
        };

        for i in 0..9 {
            let diff = target_bands[i] - self.anim_eq_bands[i];
            if diff.abs() > 0.05 {
                self.anim_eq_bands[i] += diff * (12.0 * dt).min(1.0);
                animating = true;
            } else {
                self.anim_eq_bands[i] = target_bands[i];
            }
        }

        if since_launch < 0.75
            || now.duration_since(self.tab_transition_instant).as_secs_f32() < 0.30
        {
            animating = true;
        }

        animating
    }

    // --- Custom Painted Widgets ---

    fn card_frame() -> egui::Frame {
        egui::Frame::NONE
            .fill(BG_CARD)
            .corner_radius(CornerRadius::same(16))
            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
            .inner_margin(Margin::same(16))
    }

    /// Draws a smooth iOS / Samsung One UI animated pill toggle switch
    fn draw_toggle(ui: &mut Ui, id_str: &str, value: &mut bool) -> Response {
        let desired_size = Vec2::new(46.0, 25.0);
        let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());
        if response.clicked() {
            *value = !*value;
            response.mark_changed();
        }

        let how_on = ui.ctx().animate_bool(Id::new(id_str), *value);
        let bg_color = lerp_color(Color32::from_rgb(36, 45, 62), ACCENT_BLUE, how_on);
        let border_color = if *value || response.hovered() {
            ACCENT_BLUE
        } else {
            Color32::from_rgb(52, 64, 86)
        };

        let radius = 0.5 * rect.height();
        ui.painter().rect(
            rect,
            CornerRadius::same(radius as u8),
            bg_color,
            Stroke::new(1.0_f32, border_color),
            egui::StrokeKind::Inside,
        );

        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = Pos2::new(circle_x, rect.center().y);
        ui.painter()
            .circle_filled(center, radius - 3.5, Color32::WHITE);

        response
    }

    /// Draws a full clickable feature card with an icon circle, title, subtitle, and animated toggle
    fn draw_feature_toggle_card(
        ui: &mut Ui,
        id_str: &str,
        icon_kind: &str,
        title: &str,
        subtitle: &str,
        mut enabled: bool,
    ) -> bool {
        let mut changed = false;
        let how_on = ui.ctx().animate_bool(Id::new((id_str, "card_bg")), enabled);
        let fill_color = lerp_color(BG_CARD, Color32::from_rgb(20, 29, 46), how_on);
        let stroke_color = lerp_color(BORDER_SUBTLE, Color32::from_rgb(45, 85, 160), how_on);

        let frame = egui::Frame::NONE
            .fill(fill_color)
            .corner_radius(CornerRadius::same(14))
            .stroke(Stroke::new(1.0_f32, stroke_color))
            .inner_margin(Margin::symmetric(16, 13));

        frame.show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                // Vector Icon Badge
                let (icon_rect, _) = ui.allocate_exact_size(Vec2::new(38.0, 38.0), Sense::hover());
                let badge_bg = lerp_color(Color32::from_rgb(26, 33, 46), ACCENT_TINT, how_on);
                let fg = lerp_color(TEXT_SECONDARY, ACCENT_CYAN, how_on);
                ui.painter()
                    .circle_filled(icon_rect.center(), 19.0, badge_bg);
                Self::paint_vector_icon(ui.painter(), icon_rect.center(), icon_kind, fg);

                ui.add_space(6.0);

                ui.vertical(|ui| {
                    ui.label(RichText::new(title).size(14.5).strong().color(TEXT_PRIMARY));
                    ui.label(RichText::new(subtitle).size(12.0).color(TEXT_SECONDARY));
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if Self::draw_toggle(ui, id_str, &mut enabled).changed() {
                        changed = true;
                    }
                });
            });
        });

        changed
    }

    /// Paints clean vector icons inside a 38x38 badge without relying on broken emoji fonts
    fn paint_vector_icon(painter: &egui::Painter, c: Pos2, kind: &str, color: Color32) {
        let stroke = Stroke::new(1.8_f32, color);
        match kind {
            "off" => {
                painter.circle_stroke(c, 8.0, stroke);
                painter.line_segment(
                    [Pos2::new(c.x - 5.5, c.y - 5.5), Pos2::new(c.x + 5.5, c.y + 5.5)],
                    stroke,
                );
            }
            "anc" => {
                painter.circle_stroke(c, 8.5, stroke);
                painter.line_segment(
                    [Pos2::new(c.x - 3.5, c.y - 3.5), Pos2::new(c.x - 3.5, c.y + 3.5)],
                    stroke,
                );
                painter.line_segment([Pos2::new(c.x, c.y - 5.5), Pos2::new(c.x, c.y + 5.5)], stroke);
                painter.line_segment(
                    [Pos2::new(c.x + 3.5, c.y - 3.5), Pos2::new(c.x + 3.5, c.y + 3.5)],
                    stroke,
                );
            }
            "ambient" => {
                painter.circle_filled(Pos2::new(c.x - 3.0, c.y), 2.5, color);
                Self::paint_arc(painter, Pos2::new(c.x - 3.0, c.y), 6.0, -0.8, 0.8, stroke);
                Self::paint_arc(painter, Pos2::new(c.x - 3.0, c.y), 10.0, -0.9, 0.9, stroke);
            }
            "adaptive" => {
                painter.line_segment([Pos2::new(c.x, c.y - 8.5), Pos2::new(c.x, c.y + 8.5)], stroke);
                painter.line_segment([Pos2::new(c.x - 8.5, c.y), Pos2::new(c.x + 8.5, c.y)], stroke);
                painter.line_segment(
                    [Pos2::new(c.x - 5.0, c.y - 5.0), Pos2::new(c.x + 5.0, c.y + 5.0)],
                    stroke,
                );
                painter.line_segment(
                    [Pos2::new(c.x + 5.0, c.y - 5.0), Pos2::new(c.x - 5.0, c.y + 5.0)],
                    stroke,
                );
            }
            "voice" => {
                for (dx, h) in [(-6.0, 3.5), (-2.0, 7.5), (2.0, 5.5), (6.0, 3.0)] {
                    painter.line_segment(
                        [Pos2::new(c.x + dx, c.y - h), Pos2::new(c.x + dx, c.y + h)],
                        Stroke::new(2.0_f32, color),
                    );
                }
            }
            "game" => {
                let pts = [
                    Pos2::new(c.x + 1.5, c.y - 8.5),
                    Pos2::new(c.x - 5.0, c.y + 1.0),
                    Pos2::new(c.x + 0.5, c.y + 1.0),
                    Pos2::new(c.x - 1.5, c.y + 8.5),
                ];
                painter.line_segment([pts[0], pts[1]], stroke);
                painter.line_segment([pts[1], pts[2]], stroke);
                painter.line_segment([pts[2], pts[3]], stroke);
            }
            "link" => {
                painter.circle_stroke(Pos2::new(c.x - 4.0, c.y), 4.5, stroke);
                painter.circle_stroke(Pos2::new(c.x + 4.0, c.y), 4.5, stroke);
            }
            "touch" => {
                painter.circle_filled(c, 2.5, color);
                painter.circle_stroke(c, 6.0, stroke);
                Self::paint_arc(painter, c, 9.5, -2.2, -0.9, stroke);
            }
            "call" => {
                let r = Rect::from_center_size(Pos2::new(c.x, c.y - 2.0), Vec2::new(6.0, 10.0));
                painter.rect_stroke(r, CornerRadius::same(3), stroke, egui::StrokeKind::Inside);
                painter.line_segment([Pos2::new(c.x, c.y + 5.0), Pos2::new(c.x, c.y + 8.5)], stroke);
                painter.line_segment(
                    [Pos2::new(c.x - 4.0, c.y + 8.5), Pos2::new(c.x + 4.0, c.y + 8.5)],
                    stroke,
                );
            }
            _ => {
                painter.circle_stroke(c, 6.5, stroke);
            }
        }
    }

    fn paint_arc(
        painter: &egui::Painter,
        center: Pos2,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        stroke: Stroke,
    ) {
        let steps = 32;
        let mut pts = Vec::with_capacity(steps + 1);
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let a = start_angle + (end_angle - start_angle) * t;
            pts.push(Pos2::new(
                center.x + radius * a.cos(),
                center.y + radius * a.sin(),
            ));
        }
        painter.add(egui::Shape::line(pts, stroke));
    }

    /// Dynamic color interpolation as the battery gauge sweeps from 0% up to the target %
    fn animated_battery_color(anim_pct: f32, is_active: bool) -> Color32 {
        if !is_active {
            return Color32::from_rgb(64, 74, 94);
        }
        if anim_pct < 25.0 {
            lerp_color(STATUS_RED, STATUS_AMBER, (anim_pct / 25.0).clamp(0.0, 1.0))
        } else if anim_pct < 55.0 {
            lerp_color(
                STATUS_AMBER,
                STATUS_GREEN,
                ((anim_pct - 25.0) / 30.0).clamp(0.0, 1.0),
            )
        } else {
            STATUS_GREEN
        }
    }

    /// Renders a modern Hero Card for Left Bud, Right Bud, or Case with an animated circular radial battery gauge
    #[allow(clippy::too_many_arguments)]
    fn render_hero_device_card(
        ui: &mut Ui,
        title: &str,
        side_badge: &str,
        raw_pct: u8,
        anim_pct: f32,
        charging: bool,
        placement: Option<PlacementState>,
        temp_c: Option<f32>,
        case_sync_live: bool,
    ) {
        let is_active = raw_pct > 0 && raw_pct <= 100;
        let display_pct = anim_pct.round().clamp(0.0, 100.0) as u8;
        let accent = Self::animated_battery_color(anim_pct, is_active);

        let frame = egui::Frame::NONE
            .fill(BG_CARD)
            .corner_radius(CornerRadius::same(16))
            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
            .inner_margin(Margin::symmetric(14, 14));

        frame.show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.vertical_centered(|ui| {
                // Header row with title + charging indicator
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(title)
                            .size(13.5)
                            .strong()
                            .color(TEXT_SECONDARY),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if charging {
                            ui.label(
                                RichText::new("⚡ Carregando")
                                    .size(11.0)
                                    .strong()
                                    .color(STATUS_GREEN),
                            );
                        } else if let Some(t) = temp_c {
                            if t > 0.0 {
                                ui.label(
                                    RichText::new(format!("{t:.1}°C"))
                                        .size(11.0)
                                        .color(TEXT_MUTED),
                                );
                            }
                        }
                    });
                });

                ui.add_space(6.0);

                // Circular Radial Battery Gauge + Earbud/Case Vector Illustration
                let (gauge_rect, _) =
                    ui.allocate_exact_size(Vec2::new(86.0, 86.0), Sense::hover());
                let center = gauge_rect.center();
                let radius = 36.0;
                let start_angle = PI * 0.75; // 135 deg
                let sweep = PI * 1.5; // 270 deg

                // Track arc
                Self::paint_arc(
                    ui.painter(),
                    center,
                    radius,
                    start_angle,
                    start_angle + sweep,
                    Stroke::new(5.5_f32, Color32::from_rgb(30, 38, 54)),
                );

                // Animated Progress Arc
                if is_active && anim_pct > 0.2 {
                    let frac = (anim_pct / 100.0).clamp(0.0, 1.0);
                    Self::paint_arc(
                        ui.painter(),
                        center,
                        radius,
                        start_angle,
                        start_angle + sweep * frac,
                        Stroke::new(5.5_f32, accent),
                    );

                    // Subtle glowing tip dot at the end of the animated arc
                    let end_angle = start_angle + sweep * frac;
                    let tip_pos = Pos2::new(
                        center.x + radius * end_angle.cos(),
                        center.y + radius * end_angle.sin(),
                    );
                    ui.painter()
                        .circle_filled(tip_pos, 3.2, Color32::WHITE);
                }

                // Inner vector illustration (Earbud with blade stem or Charging Case)
                if side_badge == "CASE" {
                    let case_rect = Rect::from_center_size(
                        Pos2::new(center.x, center.y + 1.0),
                        Vec2::new(30.0, 22.0),
                    );
                    ui.painter().rect(
                        case_rect,
                        CornerRadius::same(7),
                        Color32::from_rgb(32, 40, 56),
                        Stroke::new(1.5_f32, TEXT_SECONDARY),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().line_segment(
                        [
                            Pos2::new(case_rect.left() + 2.0, center.y - 1.0),
                            Pos2::new(case_rect.right() - 2.0, center.y - 1.0),
                        ],
                        Stroke::new(1.0_f32, TEXT_MUTED),
                    );
                    ui.painter()
                        .circle_filled(Pos2::new(center.x, center.y + 4.5), 2.0, accent);
                } else {
                    let dir = if side_badge == "L" { -1.0 } else { 1.0 };
                    let head_pos = Pos2::new(center.x + dir * 3.5, center.y - 6.5);
                    ui.painter()
                        .circle_filled(head_pos, 7.5, Color32::from_rgb(45, 56, 78));
                    ui.painter()
                        .circle_stroke(head_pos, 7.5, Stroke::new(1.5_f32, TEXT_SECONDARY));
                    let stem_top = Pos2::new(center.x - dir * 2.0, center.y - 4.0);
                    let stem_bot = Pos2::new(center.x - dir * 5.0, center.y + 12.0);
                    ui.painter().line_segment(
                        [stem_top, stem_bot],
                        Stroke::new(4.2_f32, TEXT_SECONDARY),
                    );
                }

                // Animated Percentage Readout
                let pct_text = if is_active {
                    format!("{display_pct}%")
                } else {
                    "--%".to_string()
                };
                ui.label(
                    RichText::new(pct_text)
                        .size(23.0)
                        .strong()
                        .color(if is_active { TEXT_PRIMARY } else { TEXT_MUTED }),
                );

                ui.add_space(2.0);

                // Status Pill
                let (status_str, pill_bg, pill_fg) = match placement {
                    Some(pl) => {
                        let bg = match pl {
                            PlacementState::Wearing | PlacementState::Charging => {
                                Color32::from_rgb(16, 48, 42)
                            }
                            PlacementState::InCase => Color32::from_rgb(20, 40, 66),
                            _ => Color32::from_rgb(25, 31, 44),
                        };
                        let fg = match pl {
                            PlacementState::Wearing | PlacementState::Charging => STATUS_GREEN,
                            PlacementState::InCase => ACCENT_CYAN,
                            _ => TEXT_SECONDARY,
                        };
                        (pl.label_pt(), bg, fg)
                    }
                    None => {
                        if charging {
                            ("Carregando", Color32::from_rgb(16, 48, 42), STATUS_GREEN)
                        } else if is_active && case_sync_live {
                            (
                                "Sincronizado (Aberto)",
                                Color32::from_rgb(20, 40, 66),
                                ACCENT_CYAN,
                            )
                        } else if is_active {
                            (
                                "Última leitura (Cache)",
                                Color32::from_rgb(32, 36, 48),
                                TEXT_SECONDARY,
                            )
                        } else {
                            (
                                "Sem leitura",
                                Color32::from_rgb(25, 31, 44),
                                TEXT_MUTED,
                            )
                        }
                    }
                };

                egui::Frame::NONE
                    .fill(pill_bg)
                    .corner_radius(CornerRadius::same(20))
                    .inner_margin(Margin::symmetric(10, 4))
                    .show(ui, |ui| {
                        ui.label(RichText::new(status_str).size(11.5).strong().color(pill_fg));
                    });
            });
        });
    }

    /// Renders a sleek 5-step interactive level bar (0..=4) for Ambient Volume or ANC Level
    fn render_step_bar(&self, ui: &mut Ui, label: &str, current: u8, on_select: impl Fn(u8)) {
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(label).size(13.0).strong().color(TEXT_SECONDARY));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("Nível {} de 4", current.min(4)))
                        .size(12.5)
                        .strong()
                        .color(ACCENT_CYAN),
                );
            });
        });
        ui.add_space(4.0);

        ui.columns(5, |cols| {
            for step in 0..=4u8 {
                let is_lit = step <= current;
                let is_exact = step == current;
                let anim_lit = cols[step as usize]
                    .ctx()
                    .animate_bool(Id::new((label, step)), is_lit);
                let bg = if is_exact {
                    ACCENT_BLUE
                } else {
                    lerp_color(
                        Color32::from_rgb(28, 35, 48),
                        Color32::from_rgb(38, 78, 155),
                        anim_lit,
                    )
                };
                let border = if is_exact {
                    ACCENT_CYAN
                } else {
                    BORDER_SUBTLE
                };

                let btn = egui::Button::new(
                    RichText::new(if step == 0 {
                        "Mín (0)".to_string()
                    } else if step == 4 {
                        "Máx (4)".to_string()
                    } else {
                        format!("{step}")
                    })
                    .size(12.0)
                    .strong()
                    .color(if is_lit { TEXT_PRIMARY } else { TEXT_MUTED }),
                )
                .fill(bg)
                .stroke(Stroke::new(1.0_f32, border))
                .corner_radius(CornerRadius::same(8))
                .min_size(Vec2::new(cols[step as usize].available_width(), 30.0));

                if cols[step as usize].add(btn).clicked() {
                    on_select(step);
                }
            }
        });
    }

    // --- Tab 1: Controle de Som ---

    fn render_sound_control_tab(&mut self, ui: &mut Ui) {
        Self::card_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Controle Inteligente de Ruído")
                            .size(16.0)
                            .strong()
                            .color(TEXT_PRIMARY),
                    );
                    ui.label(
                        RichText::new("Alterne entre isolamento acústico total, som ambiente ou modo adaptativo")
                            .size(12.0)
                            .color(TEXT_SECONDARY),
                    );
                });
            });

            ui.add_space(12.0);

            let modes = [
                (NoiseControlMode::Off, "off", "Desligado", "Isolamento passivo"),
                (NoiseControlMode::Anc, "anc", "ANC Ativo", "Bloqueia ruídos"),
                (NoiseControlMode::Ambient, "ambient", "Ambiente", "Ouça ao redor"),
                (NoiseControlMode::Adaptive, "adaptive", "Adaptativo", "Ajuste automático"),
            ];

            ui.columns(4, |cols| {
                for (idx, (mode, icon_kind, title, desc)) in modes.iter().enumerate() {
                    let selected = self.snapshot.noise_control == *mode;
                    let anim_sel = cols[idx]
                        .ctx()
                        .animate_bool(Id::new(("nc_mode_anim", idx)), selected);
                    let fill = lerp_color(BG_SUBTLE, ACCENT_TINT, anim_sel);
                    let border_col = lerp_color(BORDER_SUBTLE, ACCENT_BLUE, anim_sel);

                    let frame = egui::Frame::NONE
                        .fill(fill)
                        .corner_radius(CornerRadius::same(12))
                        .stroke(Stroke::new(1.0 + 0.6 * anim_sel, border_col))
                        .inner_margin(Margin::symmetric(10, 12));

                    let resp = frame
                        .show(&mut cols[idx], |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.vertical_centered(|ui| {
                                let (icon_rect, _) =
                                    ui.allocate_exact_size(Vec2::new(34.0, 34.0), Sense::hover());
                                let circle_col = lerp_color(
                                    Color32::from_rgb(26, 33, 46),
                                    ACCENT_BLUE,
                                    anim_sel,
                                );
                                ui.painter()
                                    .circle_filled(icon_rect.center(), 17.0, circle_col);
                                Self::paint_vector_icon(
                                    ui.painter(),
                                    icon_rect.center(),
                                    icon_kind,
                                    lerp_color(TEXT_SECONDARY, Color32::WHITE, anim_sel),
                                );
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new(*title)
                                        .size(13.5)
                                        .strong()
                                        .color(lerp_color(TEXT_PRIMARY, Color32::WHITE, anim_sel)),
                                );
                                ui.label(
                                    RichText::new(*desc)
                                        .size(11.0)
                                        .color(lerp_color(TEXT_MUTED, ACCENT_CYAN, anim_sel)),
                                );
                            });
                        })
                        .response
                        .interact(Sense::click());

                    if resp.clicked() {
                        self.send(UserCommand::SetNoiseControl(*mode));
                    }
                }
            });

            if self.snapshot.noise_control == NoiseControlMode::Ambient {
                self.render_step_bar(
                    ui,
                    "Volume do Som Ambiente",
                    self.snapshot.ambient_volume,
                    |vol| self.send(UserCommand::SetAmbientVolume(vol)),
                );
            } else if self.snapshot.noise_control == NoiseControlMode::Anc {
                self.render_step_bar(
                    ui,
                    "Intensidade do Cancelamento de Ruído (ANC)",
                    self.snapshot.anc_level,
                    |lvl| self.send(UserCommand::SetAncLevel(lvl)),
                );
            }
        });

        ui.add_space(10.0);

        if Self::draw_feature_toggle_card(
            ui,
            "toggle_voice_detect",
            "voice",
            "Detectar Conversas (Voice Detect)",
            "Reduz a mídia e ativa o Som Ambiente automaticamente ao ouvir sua voz",
            self.snapshot.voice_detect_enabled,
        ) {
            self.send(UserCommand::SetVoiceDetect(!self.snapshot.voice_detect_enabled));
        }

        if self.snapshot.voice_detect_enabled {
            Self::card_frame().show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Retornar ao modo anterior após silêncio de:")
                            .size(13.0)
                            .color(TEXT_SECONDARY),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        for &sec in &[15u8, 10, 5] {
                            let active = self.snapshot.voice_detect_timeout_sec == sec;
                            let btn = egui::Button::new(
                                RichText::new(format!("{sec} segundos"))
                                    .size(12.0)
                                    .strong()
                                    .color(if active { Color32::WHITE } else { TEXT_SECONDARY }),
                            )
                            .fill(if active {
                                ACCENT_BLUE
                            } else {
                                Color32::from_rgb(28, 36, 50)
                            })
                            .corner_radius(CornerRadius::same(8));
                            if ui.add(btn).clicked() {
                                self.send(UserCommand::SetVoiceDetectDuration(sec));
                            }
                        }
                    });
                });
            });
        }

        ui.add_space(6.0);

        ui.columns(2, |cols| {
            if Self::draw_feature_toggle_card(
                &mut cols[0],
                "toggle_one_ear",
                "anc",
                "Ruído com 1 Fone",
                "Permite ANC e Ambiente usando só um lado",
                self.snapshot.anc_with_one_earbud,
            ) {
                self.send(UserCommand::SetAncWithOneEarbud(
                    !self.snapshot.anc_with_one_earbud,
                ));
            }

            if Self::draw_feature_toggle_card(
                &mut cols[1],
                "toggle_game_mode",
                "game",
                "Modo Jogo",
                "Minimiza a latência de áudio Bluetooth",
                self.snapshot.game_mode,
            ) {
                self.send(UserCommand::SetGameMode(!self.snapshot.game_mode));
            }
        });

        ui.add_space(6.0);

        if Self::draw_feature_toggle_card(
            ui,
            "toggle_seamless",
            "link",
            "Conexão Contínua de Dispositivos (Seamless)",
            "Permite alternar rapidamente a conexão Bluetooth entre seus dispositivos",
            self.snapshot.seamless_connection,
        ) {
            self.send(UserCommand::SetSeamlessConnection(
                !self.snapshot.seamless_connection,
            ));
        }
    }

    // --- Tab 2: Equalizador & Áudio ---

    fn preset_curve_preview(preset: EqualizerPreset, custom: &[i8; 9]) -> [f32; 9] {
        match preset {
            EqualizerPreset::Off => [0.0; 9],
            EqualizerPreset::BassBoost => [7.0, 6.0, 4.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            EqualizerPreset::Soft => [-2.0, -1.0, 0.0, 1.0, 1.0, 0.0, -2.0, -3.0, -4.0],
            EqualizerPreset::Dynamic => [5.0, 4.0, 1.0, -1.0, 0.0, 2.0, 4.0, 5.0, 4.0],
            EqualizerPreset::Clear => [-3.0, -2.0, 0.0, 2.0, 4.0, 5.0, 4.0, 3.0, 2.0],
            EqualizerPreset::TrebleBoost => [0.0, 0.0, 0.0, 0.0, 1.0, 3.0, 5.0, 7.0, 7.0],
            EqualizerPreset::Custom => custom.map(|v| v as f32),
        }
    }

    fn render_equalizer_tab(&mut self, ui: &mut Ui) {
        Self::card_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                RichText::new("Perfil de Equalização")
                    .size(16.0)
                    .strong()
                    .color(TEXT_PRIMARY),
            );
            ui.label(
                RichText::new("Escolha uma assinatura sonora da Samsung ou ajuste as 9 bandas manualmente")
                    .size(12.0)
                    .color(TEXT_SECONDARY),
            );
            ui.add_space(10.0);

            ui.horizontal_wrapped(|ui| {
                for &preset in EqualizerPreset::all_presets() {
                    let active = self.snapshot.equalizer == preset;
                    let anim_act = ui
                        .ctx()
                        .animate_bool(Id::new(("eq_preset_btn", preset as u8)), active);
                    let btn = egui::Button::new(
                        RichText::new(preset.label_pt())
                            .size(13.0)
                            .strong()
                            .color(lerp_color(TEXT_SECONDARY, Color32::WHITE, anim_act)),
                    )
                    .fill(lerp_color(
                        Color32::from_rgb(26, 33, 46),
                        ACCENT_BLUE,
                        anim_act,
                    ))
                    .stroke(Stroke::new(
                        1.0_f32,
                        lerp_color(BORDER_SUBTLE, ACCENT_CYAN, anim_act),
                    ))
                    .corner_radius(CornerRadius::same(10));

                    if ui.add(btn).clicked() {
                        self.bands_dirty = false;
                        self.send(UserCommand::SetEqualizer(preset));
                    }
                }
            });
        });

        ui.add_space(10.0);

        Self::card_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            let is_custom = self.snapshot.equalizer == EqualizerPreset::Custom;

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(if is_custom {
                            "Estúdio Equalizador de 9 Bandas (Modo Personalizado Ativo)"
                        } else {
                            "Visualização da Curva de Frequência (Arraste qualquer banda para personalizar)"
                        })
                        .size(15.0)
                        .strong()
                        .color(TEXT_PRIMARY),
                    );
                    ui.label(
                        RichText::new("Ganho de -10 dB a +10 dB por faixa de frequência")
                            .size(12.0)
                            .color(TEXT_SECONDARY),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Aplicar Curva")
                                    .size(12.5)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(ACCENT_BLUE)
                            .corner_radius(CornerRadius::same(8)),
                        )
                        .clicked()
                    {
                        self.bands_dirty = false;
                        self.send(UserCommand::SetCustomEqualizer(self.local_bands));
                    }

                    if ui
                        .add(
                            egui::Button::new(RichText::new("Flat (0 dB)").size(12.0))
                                .corner_radius(CornerRadius::same(8)),
                        )
                        .clicked()
                    {
                        self.local_bands = [0; 9];
                        self.bands_dirty = false;
                        self.send(UserCommand::SetCustomEqualizer(self.local_bands));
                    }
                });
            });

            ui.add_space(12.0);

            let avail_w = ui.available_width();
            let graph_h = 190.0;
            let (rect, _) = ui.allocate_exact_size(Vec2::new(avail_w, graph_h), Sense::hover());

            let pad_x = 34.0;
            let pad_top = 22.0;
            let pad_bot = 28.0;
            let plot_rect = Rect::from_min_max(
                Pos2::new(rect.left() + pad_x, rect.top() + pad_top),
                Pos2::new(rect.right() - 16.0, rect.bottom() - pad_bot),
            );

            for (db_label, frac) in [("+10", 0.0_f32), ("0 dB", 0.5_f32), ("-10", 1.0_f32)] {
                let y = egui::lerp(plot_rect.top()..=plot_rect.bottom(), frac);
                ui.painter().line_segment(
                    [Pos2::new(plot_rect.left(), y), Pos2::new(plot_rect.right(), y)],
                    Stroke::new(
                        1.0_f32,
                        if (frac - 0.5).abs() < 0.01 {
                            Color32::from_rgb(48, 60, 82)
                        } else {
                            Color32::from_rgb(28, 35, 48)
                        },
                    ),
                );
                ui.painter().text(
                    Pos2::new(rect.left() + 2.0, y),
                    egui::Align2::LEFT_CENTER,
                    db_label,
                    egui::FontId::proportional(10.5),
                    TEXT_MUTED,
                );
            }

            let values = self.anim_eq_bands;

            let mut band_points = [Pos2::ZERO; 9];
            for i in 0..9 {
                let x = egui::lerp(plot_rect.left()..=plot_rect.right(), i as f32 / 8.0);
                let norm = (10.0 - values[i].clamp(-10.0, 10.0)) / 20.0;
                let y = egui::lerp(plot_rect.top()..=plot_rect.bottom(), norm);
                band_points[i] = Pos2::new(x, y);

                ui.painter().line_segment(
                    [Pos2::new(x, plot_rect.top()), Pos2::new(x, plot_rect.bottom())],
                    Stroke::new(3.0_f32, Color32::from_rgb(28, 36, 52)),
                );
                let zero_y = plot_rect.center().y;
                ui.painter().line_segment(
                    [Pos2::new(x, zero_y), Pos2::new(x, y)],
                    Stroke::new(3.5_f32, ACCENT_BLUE),
                );
            }

            ui.painter().add(egui::Shape::line(
                band_points.to_vec(),
                Stroke::new(2.5_f32, ACCENT_CYAN),
            ));

            for i in 0..9 {
                let pt = band_points[i];
                let hit_rect = Rect::from_center_size(
                    Pos2::new(pt.x, plot_rect.center().y),
                    Vec2::new(32.0, plot_rect.height() + 16.0),
                );
                let band_resp =
                    ui.interact(hit_rect, Id::new(("eq_band_drag", i)), Sense::click_and_drag());
                if band_resp.dragged() || band_resp.clicked() {
                    if let Some(mouse_pos) = band_resp.interact_pointer_pos() {
                        let clamped_y = mouse_pos.y.clamp(plot_rect.top(), plot_rect.bottom());
                        let norm = (clamped_y - plot_rect.top()) / plot_rect.height();
                        let db = (10.0 - norm * 20.0).round().clamp(-10.0, 10.0) as i8;
                        if !is_custom && !self.bands_dirty {
                            self.local_bands = Self::preset_curve_preview(
                                self.snapshot.equalizer,
                                &self.local_bands,
                            )
                            .map(|v| v as i8);
                        }
                        self.local_bands[i] = db;
                        self.bands_dirty = true;
                    }
                }
                if band_resp.drag_stopped() && self.bands_dirty {
                    self.bands_dirty = false;
                    self.send(UserCommand::SetCustomEqualizer(self.local_bands));
                }

                let hovered = band_resp.hovered() || band_resp.dragged();
                ui.painter().circle_filled(
                    pt,
                    if hovered { 7.5 } else { 6.0 },
                    if hovered { Color32::WHITE } else { ACCENT_CYAN },
                );
                ui.painter().circle_stroke(
                    pt,
                    if hovered { 7.5 } else { 6.0 },
                    Stroke::new(2.0_f32, BG_BASE),
                );

                let db_val = values[i].round() as i8;
                let db_str = if db_val > 0 {
                    format!("+{db_val}")
                } else {
                    format!("{db_val}")
                };
                ui.painter().text(
                    Pos2::new(pt.x, rect.top() + 8.0),
                    egui::Align2::CENTER_CENTER,
                    db_str,
                    egui::FontId::proportional(11.0),
                    if db_val != 0 { ACCENT_CYAN } else { TEXT_MUTED },
                );

                ui.painter().text(
                    Pos2::new(pt.x, rect.bottom() - 10.0),
                    egui::Align2::CENTER_CENTER,
                    EQ_BAND_LABELS[i],
                    egui::FontId::proportional(11.5),
                    TEXT_SECONDARY,
                );
            }
        });
    }

    // --- Tab 3: Toques, Chamadas & Sistema ---

    fn render_touch_and_system_tab(&mut self, ui: &mut Ui) {
        if Self::draw_feature_toggle_card(
            ui,
            "toggle_touch_lock",
            "touch",
            "Bloquear Todos os Toques na Haste",
            "Evita acionamentos acidentais de pinça ou deslize nos fones",
            self.snapshot.touchpad_locked,
        ) {
            self.send(UserCommand::SetTouchpadLock(!self.snapshot.touchpad_locked));
        }

        if !self.snapshot.touchpad_locked {
            ui.add_space(6.0);
            Self::card_frame().show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(
                    RichText::new("Gestos Individuais da Haste")
                        .size(14.5)
                        .strong()
                        .color(TEXT_PRIMARY),
                );
                ui.add_space(8.0);

                let mut s1 = self.snapshot.single_tap_on;
                let mut s2 = self.snapshot.double_tap_on;
                let mut s3 = self.snapshot.triple_tap_on;
                let mut sh = self.snapshot.touch_hold_on;
                let mut any_changed = false;

                ui.columns(2, |cols| {
                    cols[0].horizontal(|ui| {
                        if Self::draw_toggle(ui, "g_s1", &mut s1).changed() {
                            any_changed = true;
                        }
                        ui.label("Pinça Simples (Play / Pause)");
                    });
                    cols[1].horizontal(|ui| {
                        if Self::draw_toggle(ui, "g_s2", &mut s2).changed() {
                            any_changed = true;
                        }
                        ui.label("Pinça Dupla (Próxima faixa)");
                    });
                });
                ui.add_space(4.0);
                ui.columns(2, |cols| {
                    cols[0].horizontal(|ui| {
                        if Self::draw_toggle(ui, "g_s3", &mut s3).changed() {
                            any_changed = true;
                        }
                        ui.label("Pinça Tripla (Faixa anterior)");
                    });
                    cols[1].horizontal(|ui| {
                        if Self::draw_toggle(ui, "g_sh", &mut sh).changed() {
                            any_changed = true;
                        }
                        ui.label("Pinçar e Segurar (Controle de Ruído)");
                    });
                });

                if any_changed {
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

        ui.add_space(8.0);

        ui.columns(2, |cols| {
            if Self::draw_feature_toggle_card(
                &mut cols[0],
                "toggle_clear_call",
                "call",
                "Chamadas Extra Claras",
                "Aprimora nitidez de voz em ligações",
                self.snapshot.extra_clear_call,
            ) {
                self.send(UserCommand::SetExtraClearCall(
                    !self.snapshot.extra_clear_call,
                ));
            }

            if Self::draw_feature_toggle_card(
                &mut cols[1],
                "toggle_sidetone",
                "call",
                "Retorno de Voz (Sidetone)",
                "Ouça sua própria voz durante chamadas",
                self.snapshot.sidetone_enabled,
            ) {
                self.send(UserCommand::SetSidetone(!self.snapshot.sidetone_enabled));
            }
        });

        ui.add_space(8.0);

        Self::card_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Encontrar Meus Fones (Alerta Sonoro)")
                            .size(15.0)
                            .strong()
                            .color(TEXT_PRIMARY),
                    );
                    ui.label(
                        RichText::new("Emite um bipe progressivo nos fones para localizá-los no ambiente")
                            .size(12.0)
                            .color(TEXT_SECONDARY),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if !self.snapshot.find_my_buds_active {
                        let btn = egui::Button::new(
                            RichText::new("Emitir Alerta Sonoro")
                                .size(13.0)
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(ACCENT_BLUE)
                        .corner_radius(CornerRadius::same(10));
                        if ui.add(btn).clicked() {
                            self.send(UserCommand::SetFindMyBuds(true));
                        }
                    } else {
                        let btn = egui::Button::new(
                            RichText::new("Parar Alerta")
                                .size(13.0)
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(STATUS_RED)
                        .corner_radius(CornerRadius::same(10));
                        if ui.add(btn).clicked() {
                            self.send(UserCommand::SetFindMyBuds(false));
                        }
                    }
                });
            });

            if self.snapshot.find_my_buds_active {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let mut ml = self.snapshot.mute_left;
                    let mut mr = self.snapshot.mute_right;
                    let mut ch = false;
                    if Self::draw_toggle(ui, "mute_l", &mut ml).changed() {
                        ch = true;
                    }
                    ui.label("Silenciar Fone Esquerdo");
                    ui.add_space(16.0);
                    if Self::draw_toggle(ui, "mute_r", &mut mr).changed() {
                        ch = true;
                    }
                    ui.label("Silenciar Fone Direito");
                    if ch {
                        self.send(UserCommand::SetMuteEarbuds {
                            left: ml,
                            right: mr,
                        });
                    }
                });
            }
        });

        ui.add_space(8.0);

        Self::card_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Informações de Hardware & Telemetria")
                        .size(14.5)
                        .strong()
                        .color(TEXT_PRIMARY),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Atualizar Sensores").size(12.0))
                                .corner_radius(CornerRadius::same(8)),
                        )
                        .clicked()
                    {
                        self.send(UserCommand::RefreshDebug);
                    }
                });
            });
            ui.add_space(8.0);

            egui::Grid::new("hw_info_grid")
                .num_columns(4)
                .spacing([24.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Modelo / SKU:").color(TEXT_MUTED));
                    ui.label(
                        RichText::new(if self.snapshot.sku_left.is_empty() {
                            "SM-R640"
                        } else {
                            &self.snapshot.sku_left
                        })
                        .strong(),
                    );
                    ui.label(RichText::new("Firmware:").color(TEXT_MUTED));
                    ui.label(
                        RichText::new(if self.snapshot.firmware_version.is_empty() {
                            "SM-R640"
                        } else {
                            &self.snapshot.firmware_version
                        })
                        .strong(),
                    );
                    ui.end_row();

                    ui.label(RichText::new("Serial Esquerdo:").color(TEXT_MUTED));
                    ui.label(
                        RichText::new(if self.snapshot.serial_left.is_empty() {
                            "N/D"
                        } else {
                            &self.snapshot.serial_left
                        })
                        .strong(),
                    );
                    ui.label(RichText::new("Serial Direito:").color(TEXT_MUTED));
                    ui.label(
                        RichText::new(if self.snapshot.serial_right.is_empty() {
                            "N/D"
                        } else {
                            &self.snapshot.serial_right
                        })
                        .strong(),
                    );
                    ui.end_row();

                    ui.label(RichText::new("Endereço MAC:").color(TEXT_MUTED));
                    ui.label(RichText::new(&self.snapshot.mac_address).strong());
                    ui.label(RichText::new("Hardware / Cor:").color(TEXT_MUTED));
                    ui.label(
                        RichText::new(format!(
                            "{} • {}",
                            if self.snapshot.hardware_version.is_empty() {
                                "rev0.2"
                            } else {
                                &self.snapshot.hardware_version
                            },
                            self.snapshot.color_name
                        ))
                        .strong(),
                    );
                    ui.end_row();
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
            // Re-trigger the smooth entrance and battery sweep animation when reopening from tray!
            let now = Instant::now();
            self.launch_instant = now;
            self.anim_batt_left = 0.0;
            self.anim_batt_right = 0.0;
            self.anim_batt_case = 0.0;
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }

        if self.last_sync.elapsed() >= Duration::from_millis(90) {
            if let Ok(guard) = self.state.try_read() {
                self.snapshot = guard.clone();
                if !self.bands_dirty {
                    self.local_bands = self.snapshot.custom_eq_bands;
                }
            }
            self.last_sync = Instant::now();
        }

        let is_animating = self.step_animations();
        if is_animating {
            ctx.request_repaint_after(Duration::from_millis(16));
        } else {
            ctx.request_repaint_after(Duration::from_millis(150));
        }

        let since_launch = self.launch_instant.elapsed().as_secs_f32();
        let launch_ease = ease_out_cubic(since_launch / 0.55);

        let tab_elapsed = self.tab_transition_instant.elapsed().as_secs_f32();
        let tab_ease = ease_out_cubic(tab_elapsed / 0.26);
        let tab_offset_x = self.tab_slide_dir * 26.0 * (1.0 - tab_ease);

        egui::CentralPanel::default().show(ctx, |ui| {
            // Smooth entrance vertical offset and fade-in on startup
            ui.set_opacity(launch_ease);
            let entrance_y = (1.0 - launch_ease) * 14.0;
            if entrance_y > 0.1 {
                ui.add_space(entrance_y);
            }

            // --- Top App Header Bar ---
            ui.horizontal(|ui| {
                // Custom App Icon Badge in Header
                let (logo_rect, _) = ui.allocate_exact_size(Vec2::new(42.0, 42.0), Sense::hover());
                ui.painter().rect(
                    logo_rect,
                    CornerRadius::same(11),
                    Color32::from_rgb(17, 26, 46),
                    Stroke::new(1.2_f32, Color32::from_rgb(56, 189, 248)),
                    egui::StrokeKind::Inside,
                );
                let lc = logo_rect.center();
                ui.painter()
                    .circle_filled(Pos2::new(lc.x - 6.0, lc.y - 6.0), 4.8, TEXT_PRIMARY);
                ui.painter().line_segment(
                    [
                        Pos2::new(lc.x - 7.5, lc.y - 4.0),
                        Pos2::new(lc.x - 4.5, lc.y + 10.0),
                    ],
                    Stroke::new(3.0_f32, ACCENT_CYAN),
                );
                ui.painter()
                    .circle_filled(Pos2::new(lc.x + 6.0, lc.y - 6.0), 4.8, TEXT_PRIMARY);
                ui.painter().line_segment(
                    [
                        Pos2::new(lc.x + 7.5, lc.y - 4.0),
                        Pos2::new(lc.x + 4.5, lc.y + 10.0),
                    ],
                    Stroke::new(3.0_f32, ACCENT_CYAN),
                );

                ui.add_space(4.0);

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&self.snapshot.device_name)
                                .size(22.0)
                                .strong()
                                .color(TEXT_PRIMARY),
                        );

                        let (status_txt, dot_color, bg_pill) = if self.snapshot.connected {
                            ("Conectado", STATUS_GREEN, Color32::from_rgb(16, 46, 38))
                        } else {
                            ("Desconectado", STATUS_AMBER, Color32::from_rgb(48, 34, 18))
                        };
                        egui::Frame::NONE
                            .fill(bg_pill)
                            .corner_radius(CornerRadius::same(20))
                            .inner_margin(Margin::symmetric(10, 4))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (dot_rect, _) =
                                        ui.allocate_exact_size(Vec2::new(8.0, 8.0), Sense::hover());
                                    ui.painter().circle_filled(dot_rect.center(), 4.0, dot_color);
                                    ui.label(
                                        RichText::new(status_txt)
                                            .size(11.5)
                                            .strong()
                                            .color(dot_color),
                                    );
                                });
                            });
                    });

                    ui.label(
                        RichText::new(format!(
                            "Samsung Galaxy Buds4 Pro  •  {}  •  Firmware {}",
                            self.snapshot.color_name,
                            if self.snapshot.firmware_version.is_empty() {
                                "SM-R640"
                            } else {
                                &self.snapshot.firmware_version
                            }
                        ))
                        .size(12.0)
                        .color(TEXT_SECONDARY),
                    );
                });

                // Top-Right Navigation Tabs Pill Bar with Animated Sliding Indicator
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    egui::Frame::NONE
                        .fill(BG_CARD)
                        .corner_radius(CornerRadius::same(12))
                        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                        .inner_margin(Margin::same(4))
                        .show(ui, |ui| {
                            let tabs = [
                                (ActiveTab::TouchAndSystem, "Toques & Sistema"),
                                (ActiveTab::Equalizer, "Equalizador"),
                                (ActiveTab::SoundControl, "Controle de Som"),
                            ];
                            let mut active_rect: Option<Rect> = None;
                            let mut clicked_tab: Option<ActiveTab> = None;

                            // Reserve a background layer shape slot for the sliding blue pill
                            let pill_shape_idx = ui.painter().add(egui::Shape::Noop);

                            for (tab, label) in tabs {
                                let active = self.active_tab == tab;
                                let anim_col = ui
                                    .ctx()
                                    .animate_bool(Id::new(("tab_txt_col", tab.index())), active);
                                let btn = egui::Button::new(
                                    RichText::new(label)
                                        .size(12.5)
                                        .strong()
                                        .color(lerp_color(TEXT_SECONDARY, Color32::WHITE, anim_col)),
                                )
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE)
                                .corner_radius(CornerRadius::same(8));

                                let resp = ui.add(btn);
                                if active {
                                    active_rect = Some(resp.rect);
                                }
                                if resp.clicked() {
                                    clicked_tab = Some(tab);
                                }
                            }

                            if let Some(target_r) = active_rect {
                                let anim_left = ui.ctx().animate_value_with_time(
                                    Id::new("nav_pill_left"),
                                    target_r.left(),
                                    0.20,
                                );
                                let anim_right = ui.ctx().animate_value_with_time(
                                    Id::new("nav_pill_right"),
                                    target_r.right(),
                                    0.20,
                                );
                                let pill_rect = Rect::from_min_max(
                                    Pos2::new(anim_left, target_r.top()),
                                    Pos2::new(anim_right, target_r.bottom()),
                                );
                                ui.painter().set(
                                    pill_shape_idx,
                                    egui::Shape::rect_filled(
                                        pill_rect,
                                        CornerRadius::same(8),
                                        ACCENT_BLUE,
                                    ),
                                );
                            }

                            if let Some(next) = clicked_tab {
                                self.switch_tab(next);
                            }
                        });
                });
            });

            ui.add_space(12.0);

            // --- 3 Hero Battery & Device Status Cards ---
            let any_bud_in_open_case = matches!(
                self.snapshot.placement_left,
                PlacementState::InCase | PlacementState::Charging
            ) || matches!(
                self.snapshot.placement_right,
                PlacementState::InCase | PlacementState::Charging
            );

            ui.columns(3, |cols| {
                Self::render_hero_device_card(
                    &mut cols[0],
                    "Fone Esquerdo (L)",
                    "L",
                    self.snapshot.battery_left,
                    self.anim_batt_left,
                    self.snapshot.is_left_charging,
                    Some(self.snapshot.placement_left),
                    Some(self.snapshot.temp_left_c),
                    false,
                );
                Self::render_hero_device_card(
                    &mut cols[1],
                    "Fone Direito (R)",
                    "R",
                    self.snapshot.battery_right,
                    self.anim_batt_right,
                    self.snapshot.is_right_charging,
                    Some(self.snapshot.placement_right),
                    Some(self.snapshot.temp_right_c),
                    false,
                );
                Self::render_hero_device_card(
                    &mut cols[2],
                    "Estojo de Carga",
                    "CASE",
                    self.snapshot.battery_case,
                    self.anim_batt_case,
                    self.snapshot.is_case_charging && any_bud_in_open_case,
                    None,
                    None,
                    any_bud_in_open_case,
                );
            });

            ui.add_space(12.0);

            // --- Animated Active Tab Content Area (Slide + Cross-Fade) ---
            let avail = ui.available_rect_before_wrap();
            let shifted_rect = avail.translate(Vec2::new(tab_offset_x, 0.0));
            ui.scope_builder(UiBuilder::new().max_rect(shifted_rect), |ui| {
                ui.set_opacity(launch_ease * tab_ease);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.active_tab {
                        ActiveTab::SoundControl => self.render_sound_control_tab(ui),
                        ActiveTab::Equalizer => self.render_equalizer_tab(ui),
                        ActiveTab::TouchAndSystem => self.render_touch_and_system_tab(ui),
                    });
            });
        });
    }
}
