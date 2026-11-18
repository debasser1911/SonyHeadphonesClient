use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use bluetooth_connector_rs::port::{BluetoothDevice, IBluetoothConnector, MockBluetoothConnector};
use bluetooth_wrapper_rs::port::BluetoothWrapper;
use command_serializer_rs::port::package_data_for_bt;
use constants_rs::port::{
    APP_VERSION, CommandType, DataType, HeadphoneModel, MINIMUM_VOICE_FOCUS_STEP, OptimizerState,
    S2cToggle, SOUND_POSITION_PRESET_ARRAY, device_capabilities,
};
use eframe::egui;
use exceptions_rs::port::HeadphonesError;
use headphones_rs::port::Headphones;
use timed_message_queue_rs::port::{DEFAULT_ERROR_MESSAGE_MS, TimedMessageQueue};

const CASCADIA_FONT: &[u8] = include_bytes!("../../../static/Fonts/CascadiaCode.ttf");
const _BADGE_SVG: &str = include_str!("../../../static/badge.svg");

const GUI_MAX_MESSAGES: usize = 5;
const SENS_HINTS: [&str; 3] = ["Auto", "High", "Low"];
const TIME_HINTS: [&str; 4] = ["Short", "Standard", "Long", "Off"];
const SOUND_POS_LABELS: [&str; 6] = [
    "Off",
    "Front Left",
    "Front Right",
    "Front",
    "Rear Left",
    "Rear Right",
];
const VPT_LABELS: [&str; 5] = ["Off", "Outdoor Festival", "Arena", "Concert Hall", "Club"];
const EQ_PRESETS: [(u8, &str); 10] = [
    (0x00, "Off"),
    (0x10, "Bright"),
    (0x11, "Excited"),
    (0x12, "Mellow"),
    (0x13, "Relaxed"),
    (0x14, "Vocal"),
    (0x15, "Treble Boost"),
    (0x16, "Bass Boost"),
    (0x17, "Speech"),
    (0xFF, "Custom"),
];
const EQ_BAND_LABELS: [&str; 6] = [
    "Clear Bass",
    "400 Hz",
    "1 kHz",
    "2.5 kHz",
    "6.3 kHz",
    "16 kHz",
];

const ACCENT_BLUE: egui::Color32 = egui::Color32::from_rgb(76, 148, 255);
const STATUS_GREEN: egui::Color32 = egui::Color32::from_rgb(72, 199, 142);
const STATUS_AMBER: egui::Color32 = egui::Color32::from_rgb(245, 180, 75);
const STATUS_MUTED: egui::Color32 = egui::Color32::from_rgb(148, 156, 170);
const CARD_BG: egui::Color32 = egui::Color32::from_rgb(30, 33, 40);
const CARD_STROKE: egui::Color32 = egui::Color32::from_rgb(52, 57, 68);
const ERR_BG: egui::Color32 = egui::Color32::from_rgb(60, 26, 30);
const ERR_STROKE: egui::Color32 = egui::Color32::from_rgb(175, 65, 75);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NcAsmMode {
    Off,
    NoiseCancelling,
    AmbientSound,
}

pub fn launch_gui() -> Result<(), eframe::Error> {
    let title = format!("Sony Headphones Client v{APP_VERSION}");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([580.0, 680.0])
            .with_min_inner_size([460.0, 440.0]),
        ..Default::default()
    };
    eframe::run_native(
        &title,
        options,
        Box::new(|cc| Ok(Box::new(SonyHeadphonesApp::new(cc)))),
    )
}

struct SonyHeadphonesApp {
    bt: Arc<BluetoothWrapper>,
    headphones: Arc<Mutex<Headphones>>,
    mq: TimedMessageQueue,
    discovered_devices: Vec<BluetoothDevice>,
    selected_device: Option<usize>,
    connected_device: Option<BluetoothDevice>,
    discover_rx: Option<Receiver<Result<Vec<BluetoothDevice>, HeadphonesError>>>,
    connect_rx: Option<Receiver<Result<BluetoothDevice, HeadphonesError>>>,
    refresh_rx: Option<Receiver<Result<(), HeadphonesError>>>,
    send_rx: Option<Receiver<Result<(), HeadphonesError>>>,
    ambient_sound_control: bool,
    focus_on_voice: bool,
    asm_level: i32,
    sound_position_idx: usize,
    vpt_idx: usize,
    s2c_enabled: bool,
    s2c_sens: u8,
    s2c_off_time: u8,
    s2c_voice: bool,
    mp_connect_target: usize,
    force_xm4_multipoint: bool,
    eq_preset: u8,
    eq_bands: [i8; 6],
    dsee_enabled: bool,
    simulator_mode: bool,
    cached_model: HeadphoneModel,
    cached_caps: u32,
    cached_max_steps: i32,
    cached_opt_state: OptimizerState,
    cached_devices: Vec<BluetoothDevice>,
    cached_connected_indices: (usize, usize),
    last_edit_at: Option<Instant>,
}

impl SonyHeadphonesApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_fonts_and_style(&cc.egui_ctx);

        let connector = create_platform_connector();
        let bt = Arc::new(BluetoothWrapper::new(connector));
        let mut app = Self {
            bt,
            headphones: Arc::new(Mutex::new(Headphones::default())),
            mq: TimedMessageQueue::new(GUI_MAX_MESSAGES, DEFAULT_ERROR_MESSAGE_MS),
            discovered_devices: Vec::new(),
            selected_device: None,
            connected_device: None,
            discover_rx: None,
            connect_rx: None,
            refresh_rx: None,
            send_rx: None,
            ambient_sound_control: true,
            focus_on_voice: false,
            asm_level: 0,
            sound_position_idx: 0,
            vpt_idx: 0,
            s2c_enabled: false,
            s2c_sens: 0,
            s2c_off_time: 0,
            s2c_voice: false,
            mp_connect_target: 0,
            force_xm4_multipoint: false,
            eq_preset: 0,
            eq_bands: [0; 6],
            dsee_enabled: false,
            simulator_mode: false,
            cached_model: HeadphoneModel::Generic,
            cached_caps: 0,
            cached_max_steps: 19,
            cached_opt_state: OptimizerState::Idle,
            cached_devices: Vec::new(),
            cached_connected_indices: (0, 0),
            last_edit_at: None,
        };
        app.start_discovery();
        app
    }

    fn push_error(&mut self, err: HeadphonesError, prefix: &str) {
        if err.should_disconnect() {
            self.bt.disconnect();
            self.connected_device = None;
            self.mq.add_message(format!("{prefix}{err}"));
        } else {
            self.mq.add_message(err.to_string());
        }
    }

    fn start_discovery(&mut self) {
        if self.discover_rx.is_some() {
            return;
        }
        self.selected_device = None;
        let bt = Arc::clone(&self.bt);
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(bt.get_connected_devices());
        });
        self.discover_rx = Some(rx);
    }

    fn switch_to_simulator(&mut self) {
        self.bt.disconnect();
        let mock = build_xm4_simulator_connector();
        self.bt = Arc::new(BluetoothWrapper::new(Box::new(mock)));
        self.simulator_mode = true;
        self.connected_device = None;
        self.selected_device = Some(0);
        self.start_discovery();
    }

    fn exit_simulator(&mut self) {
        self.bt.disconnect();
        self.bt = Arc::new(BluetoothWrapper::new(create_platform_connector()));
        self.simulator_mode = false;
        self.connected_device = None;
        self.selected_device = None;
        self.discovered_devices.clear();
        self.start_discovery();
    }

    fn start_connect(&mut self, dev: BluetoothDevice) {
        if self.connect_rx.is_some() {
            return;
        }
        let bt = Arc::clone(&self.bt);
        let hp = Arc::clone(&self.headphones);
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let res = (|| -> Result<BluetoothDevice, HeadphonesError> {
                bt.connect(&dev.mac)?;
                if let Ok(mut guard) = hp.lock() {
                    guard.configure_for_device(&dev.name);
                    if guard.model() == HeadphoneModel::Wh1000Xm4 {
                        let _ = guard.query_state(&bt);
                    }
                }
                Ok(dev)
            })();
            let _ = tx.send(res);
        });
        self.connect_rx = Some(rx);
    }

    fn start_refresh_state(&mut self) {
        if self.refresh_rx.is_some() || !self.bt.is_connected() {
            return;
        }
        let bt = Arc::clone(&self.bt);
        let hp = Arc::clone(&self.headphones);
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let res = hp
                .lock()
                .map_err(|_| HeadphonesError::runtime("Headphones mutex poisoned".to_string()))
                .and_then(|mut g| g.query_state(&bt));
            let _ = tx.send(res);
        });
        self.refresh_rx = Some(rx);
    }

    fn poll_async_tasks(&mut self) {
        if let Some(rx) = &self.discover_rx
            && let Ok(res) = rx.try_recv()
        {
            self.discover_rx = None;
            match res {
                Ok(list) => {
                    self.discovered_devices = list;
                    if self.selected_device.is_none() && !self.discovered_devices.is_empty() {
                        self.selected_device = Some(0);
                    }
                }
                Err(e) => self.push_error(e, ""),
            }
        }
        if let Some(rx) = &self.connect_rx
            && let Ok(res) = rx.try_recv()
        {
            self.connect_rx = None;
            match res {
                Ok(dev) => {
                    self.connected_device = Some(dev);
                    self.sync_ui_from_headphones();
                }
                Err(e) => self.push_error(e, ""),
            }
        }
        if let Some(rx) = &self.refresh_rx
            && let Ok(res) = rx.try_recv()
        {
            self.refresh_rx = None;
            match res {
                Ok(()) => self.sync_ui_from_headphones(),
                Err(e) => self.push_error(e, "Refresh failed: "),
            }
        }
        if let Some(rx) = &self.send_rx
            && let Ok(res) = rx.try_recv()
        {
            self.send_rx = None;
            if let Err(e) = res {
                self.push_error(e, "Disconnected due to: ");
            } else {
                self.refresh_cached_snapshot();
            }
        }
    }

    fn refresh_cached_snapshot(&mut self) {
        if let Ok(hp) = self.headphones.try_lock() {
            self.cached_model = hp.model();
            self.cached_caps = hp.capabilities();
            self.cached_max_steps = i32::from(hp.max_asm_steps());
            self.cached_opt_state = hp.optimizer_state.current;
            self.cached_devices = hp.saved_devices.clone();
            self.cached_connected_indices = hp.get_connected_device_indices();
        }
    }

    fn sync_ui_from_headphones(&mut self) {
        if let Ok(hp) = self.headphones.lock() {
            self.ambient_sound_control = hp.ambient_sound_control.current;
            self.focus_on_voice = hp.focus_on_voice.current;
            self.asm_level = hp.asm_level.current;
            self.sound_position_idx = SOUND_POSITION_PRESET_ARRAY
                .iter()
                .position(|&p| p == hp.surround_position.current)
                .filter(|&i| i < SOUND_POS_LABELS.len())
                .unwrap_or(0);
            let vpt = usize::try_from(hp.vpt_type.current).unwrap_or(0);
            self.vpt_idx = if vpt < VPT_LABELS.len() { vpt } else { 0 };
            self.s2c_enabled = hp.speak_to_chat.current == S2cToggle::Active;
            let opts = hp.s2c_options.current;
            self.s2c_sens = ((opts >> 16) & 0xff) as u8;
            self.s2c_voice = ((opts >> 8) & 0xff) != 0;
            self.s2c_off_time = (opts & 0xff) as u8;
            self.eq_preset = hp.eq_preset.current;
            self.eq_bands = hp.eq_bands.current;
            self.dsee_enabled = hp.dsee_enabled.current;
            self.cached_model = hp.model();
            self.cached_caps = hp.capabilities();
            self.cached_max_steps = i32::from(hp.max_asm_steps());
            self.cached_opt_state = hp.optimizer_state.current;
            self.cached_devices = hp.saved_devices.clone();
            self.cached_connected_indices = hp.get_connected_device_indices();
        }
    }

    fn draw_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.heading("Sony Headphones Client");
                ui.label(
                    egui::RichText::new(format!("v{APP_VERSION} • Unofficial open-source client"))
                        .small()
                        .color(STATUS_MUTED),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (status_text, status_color) = if self.bt.is_connected() {
                    let prefix = if self.simulator_mode {
                        "Simulating"
                    } else {
                        "Connected"
                    };
                    let name = self
                        .connected_device
                        .as_ref()
                        .map_or("Headphones", |d| d.name.as_str());
                    (
                        format!("{prefix}: {name}"),
                        if self.simulator_mode {
                            STATUS_AMBER
                        } else {
                            STATUS_GREEN
                        },
                    )
                } else if self.simulator_mode {
                    ("Simulator Ready".to_owned(), STATUS_AMBER)
                } else {
                    ("Disconnected".to_owned(), STATUS_MUTED)
                };

                egui::Frame::new()
                    .fill(status_color.linear_multiply(0.14))
                    .stroke(egui::Stroke::new(1.0, status_color.linear_multiply(0.5)))
                    .corner_radius(12.0)
                    .inner_margin(egui::Margin::symmetric(10, 4))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(status_text)
                                .small()
                                .strong()
                                .color(status_color),
                        );
                    });
            });
        });
    }

    fn draw_errors(&mut self, ui: &mut egui::Ui) {
        let msgs: Vec<String> = self.mq.active_messages().map(str::to_owned).collect();
        if msgs.is_empty() {
            return;
        }
        egui::Frame::new()
            .fill(ERR_BG)
            .stroke(egui::Stroke::new(1.0, ERR_STROKE))
            .corner_radius(8.0)
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Attention")
                            .strong()
                            .color(egui::Color32::LIGHT_RED),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Dismiss").clicked() {
                            self.mq =
                                TimedMessageQueue::new(GUI_MAX_MESSAGES, DEFAULT_ERROR_MESSAGE_MS);
                        }
                    });
                });
                for msg in msgs {
                    ui.colored_label(egui::Color32::from_rgb(255, 190, 190), msg);
                }
            });
        ui.add_space(6.0);
    }

    fn draw_device_discovery(&mut self, ui: &mut egui::Ui) {
        section_card(ui, |ui| {
            if self.bt.is_connected() {
                self.refresh_cached_snapshot();
                let (model, caps) = (self.cached_model, self.cached_caps);
                let (name, mac) = self.connected_device.as_ref().map_or_else(
                    || ("Headphones".to_owned(), String::new()),
                    |d| (d.name.clone(), d.mac.clone()),
                );

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(&name).strong().size(15.0));
                        let model_str = match model {
                            HeadphoneModel::Wh1000Xm4 => "WH-1000XM4",
                            HeadphoneModel::Wh1000Xm3 => "WH-1000XM3",
                            HeadphoneModel::Generic => "Generic Sony",
                        };
                        ui.label(
                            egui::RichText::new(format!("{model_str}  •  {mac}"))
                                .small()
                                .color(STATUS_MUTED),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Disconnect").clicked() {
                            self.selected_device = None;
                            self.connected_device = None;
                            self.bt.disconnect();
                        }
                        if self.refresh_rx.is_some() {
                            ui.spinner();
                        } else if ui
                            .button("Refresh State")
                            .on_hover_text(
                                "Re-query current settings and paired devices from headphones",
                            )
                            .clicked()
                        {
                            self.start_refresh_state();
                        }
                    });
                });

                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    capability_badge(ui, "NC / ASM", caps & device_capabilities::NC_ASM != 0);
                    capability_badge(
                        ui,
                        "Speak-to-Chat",
                        caps & device_capabilities::SPEAK_TO_CHAT != 0,
                    );
                    capability_badge(
                        ui,
                        if self.force_xm4_multipoint && caps & device_capabilities::MULTI_POINT == 0
                        {
                            "Multipoint (Forced)"
                        } else {
                            "Multipoint"
                        },
                        self.force_xm4_multipoint || caps & device_capabilities::MULTI_POINT != 0,
                    );
                    capability_badge(ui, "Optimizer", caps & device_capabilities::OPTIMIZER != 0);
                    capability_badge(ui, "Virtual Sound", caps & device_capabilities::VPT != 0);
                });
                if caps & device_capabilities::MULTI_POINT == 0 {
                    ui.add_space(4.0);
                    if ui
                        .checkbox(
                            &mut self.force_xm4_multipoint,
                            "Experimental: Force XM4 Multipoint",
                        )
                        .on_hover_text(
                            "Show Multipoint controls even if the headset omitted capability 0x38 (e.g. LDAC active or renamed device).",
                        )
                        .changed()
                        && self.force_xm4_multipoint
                    {
                        self.start_refresh_state();
                    }
                }
                return;
            }

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Devices").strong().size(15.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.simulator_mode {
                        if ui.button("Exit Simulator").clicked() {
                            self.exit_simulator();
                        }
                    } else if ui
                        .button("Simulate WH-1000XM4")
                        .on_hover_text("Try all XM4 features without physical headphones")
                        .clicked()
                    {
                        self.switch_to_simulator();
                    }
                });
            });
            ui.add_space(4.0);

            if self.discovered_devices.is_empty() {
                ui.label(
                    egui::RichText::new(
                        "No Bluetooth headphones found. Make sure your headphones are paired and connected, then click Refresh, or use Simulator Mode.",
                    )
                    .color(STATUS_MUTED),
                );
            } else {
                let mut connect_idx = None;
                for (i, dev) in self.discovered_devices.iter().enumerate() {
                    let chosen = self.selected_device == Some(i);
                    let resp = ui.selectable_label(chosen, format!("{}  ({})", dev.name, dev.mac));
                    if resp.clicked() {
                        self.selected_device = Some(i);
                    }
                    if resp.double_clicked() {
                        self.selected_device = Some(i);
                        connect_idx = Some(i);
                    }
                }
                if let Some(idx) = connect_idx
                    && let Some(dev) = self.discovered_devices.get(idx).cloned()
                {
                    self.start_connect(dev);
                }
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if self.connect_rx.is_some() {
                    ui.spinner();
                    ui.label("Connecting...");
                } else {
                    let can_connect = self.selected_device.is_some();
                    if ui
                        .add_enabled(can_connect, egui::Button::new("Connect"))
                        .clicked()
                        && let Some(idx) = self.selected_device
                        && let Some(dev) = self.discovered_devices.get(idx).cloned()
                    {
                        self.start_connect(dev);
                    }
                }
                if self.discover_rx.is_some() {
                    ui.spinner();
                    ui.label("Scanning...");
                } else if ui.button("Refresh").clicked() {
                    self.start_discovery();
                }
            });
        });
    }

    fn draw_controls(&mut self, ui: &mut egui::Ui) {
        self.refresh_cached_snapshot();
        let caps = self.cached_caps;
        let max_steps = self.cached_max_steps;
        let opt_state = self.cached_opt_state;
        let devices = self.cached_devices.clone();
        let (d1, d2) = self.cached_connected_indices;

        if caps & device_capabilities::NC_ASM != 0 {
            self.draw_nc_asm_section(ui, max_steps);
        }

        if caps & device_capabilities::SPEAK_TO_CHAT != 0 {
            self.draw_speak_to_chat_section(ui);
        }

        if caps & device_capabilities::OPTIMIZER != 0 {
            self.draw_optimizer_section(ui, opt_state);
        }

        if self.force_xm4_multipoint || caps & device_capabilities::MULTI_POINT != 0 {
            self.draw_multipoint_section(ui, &devices, d1, d2);
        }

        if caps & device_capabilities::VPT != 0 {
            self.draw_vpt_section(ui);
        }

        self.draw_eq_section(ui);
        self.draw_dsee_section(ui);

        self.flush_headphone_changes(ui);
    }

    fn draw_nc_asm_section(&mut self, ui: &mut egui::Ui, max_steps: i32) {
        section_card(ui, |ui| {
            ui.label(
                egui::RichText::new("Noise Cancelling & Ambient Sound")
                    .strong()
                    .size(15.0),
            );
            ui.add_space(4.0);

            let current_mode = if !self.ambient_sound_control {
                NcAsmMode::Off
            } else if self.asm_level == 0 {
                NcAsmMode::NoiseCancelling
            } else {
                NcAsmMode::AmbientSound
            };

            let mut next_mode = current_mode;
            ui.horizontal(|ui| {
                ui.selectable_value(&mut next_mode, NcAsmMode::Off, "Off");
                ui.selectable_value(
                    &mut next_mode,
                    NcAsmMode::NoiseCancelling,
                    "Noise Cancelling",
                );
                ui.selectable_value(&mut next_mode, NcAsmMode::AmbientSound, "Ambient Sound");
            });

            if next_mode != current_mode {
                match next_mode {
                    NcAsmMode::Off => self.ambient_sound_control = false,
                    NcAsmMode::NoiseCancelling => {
                        self.ambient_sound_control = true;
                        self.asm_level = 0;
                    }
                    NcAsmMode::AmbientSound => {
                        self.ambient_sound_control = true;
                        if self.asm_level == 0 {
                            self.asm_level = 10.min(max_steps).max(1);
                        }
                    }
                }
            }

            if self.ambient_sound_control {
                ui.add_space(6.0);
                self.asm_level = self.asm_level.clamp(0, max_steps);
                let level_label = if self.asm_level == 0 {
                    "Level 0 (Noise Cancelling)".to_owned()
                } else {
                    format!("Ambient Level {}/{max_steps}", self.asm_level)
                };
                ui.add(egui::Slider::new(&mut self.asm_level, 0..=max_steps).text(level_label));

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Presets:").small().color(STATUS_MUTED));
                    if ui.small_button("NC (0)").clicked() {
                        self.asm_level = 0;
                    }
                    if ui.small_button("Low (5)").clicked() {
                        self.asm_level = 5.min(max_steps);
                    }
                    if ui.small_button("Med (10)").clicked() {
                        self.asm_level = 10.min(max_steps);
                    }
                    if ui.small_button(format!("Max ({max_steps})")).clicked() {
                        self.asm_level = max_steps;
                    }
                });

                ui.add_space(4.0);
                let voice_avail = self.asm_level > MINIMUM_VOICE_FOCUS_STEP;
                ui.add_enabled(
                    voice_avail,
                    egui::Checkbox::new(&mut self.focus_on_voice, "Focus on Voice"),
                )
                .on_disabled_hover_text(format!(
                    "Requires Ambient Sound Level > {MINIMUM_VOICE_FOCUS_STEP}"
                ));
            }
        });
    }

    fn draw_speak_to_chat_section(&mut self, ui: &mut egui::Ui) {
        section_card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Speak-to-Chat").strong().size(15.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.checkbox(&mut self.s2c_enabled, "Enabled");
                });
            });
            ui.label(
                egui::RichText::new(
                    "Automatically pauses music and lets in ambient sound when you speak.",
                )
                .small()
                .color(STATUS_MUTED),
            );
            ui.add_space(6.0);

            ui.add_enabled_ui(self.s2c_enabled, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Sensitivity:");
                    for (idx, hint) in SENS_HINTS.iter().enumerate() {
                        ui.selectable_value(&mut self.s2c_sens, idx as u8, *hint);
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Auto close:");
                    for (idx, hint) in TIME_HINTS.iter().enumerate() {
                        ui.selectable_value(&mut self.s2c_off_time, idx as u8, *hint);
                    }
                });
                ui.checkbox(&mut self.s2c_voice, "Voice passthrough");
            });
        });
    }

    fn draw_optimizer_section(&mut self, ui: &mut egui::Ui, opt_state: OptimizerState) {
        section_card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("ANC Optimizer").strong().size(15.0));
                    ui.label(
                        egui::RichText::new(
                            "Calibrates noise cancelling to your ears and atmospheric pressure.",
                        )
                        .small()
                        .color(STATUS_MUTED),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let is_optimizing = opt_state != OptimizerState::Idle;
                    let label = if is_optimizing {
                        "Cancel Optimization"
                    } else {
                        "Start Optimizer"
                    };
                    if is_optimizing {
                        ui.spinner();
                    }
                    if ui.button(label).clicked()
                        && let Ok(mut hp) = self.headphones.lock()
                    {
                        hp.optimizer_state.desired = if is_optimizing {
                            OptimizerState::Idle
                        } else {
                            OptimizerState::Optimizing
                        };
                    }
                });
            });
        });
    }

    fn draw_multipoint_section(
        &mut self,
        ui: &mut egui::Ui,
        devices: &[BluetoothDevice],
        d1: usize,
        d2: usize,
    ) {
        section_card(ui, |ui| {
            ui.label(
                egui::RichText::new("Multipoint Connections")
                    .strong()
                    .size(15.0),
            );
            ui.add_space(4.0);

            if devices.len() <= 1 {
                ui.label(
                    egui::RichText::new("No paired devices reported yet. Try Refresh State.")
                        .small()
                        .color(STATUS_MUTED),
                );
                return;
            }

            self.draw_multipoint_slot(ui, 1, devices.get(d1));
            self.draw_multipoint_slot(ui, 2, devices.get(d2));

            let free_slot = if d1 == 0 {
                Some(1)
            } else if d2 == 0 {
                Some(2)
            } else {
                None
            };

            if let Some(slot) = free_slot {
                let has_candidates = devices
                    .iter()
                    .enumerate()
                    .any(|(i, d)| i != 0 && i != d1 && i != d2 && !d.name.is_empty());
                if has_candidates {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let selected_text = devices
                            .get(self.mp_connect_target)
                            .filter(|d| !d.name.is_empty())
                            .map_or("Select a device...", |d| d.name.as_str());
                        egui::ComboBox::from_id_salt("mp_connect_combo")
                            .selected_text(selected_text)
                            .show_ui(ui, |ui| {
                                for (i, d) in devices.iter().enumerate() {
                                    if i != 0 && i != d1 && i != d2 && !d.name.is_empty() {
                                        ui.selectable_value(
                                            &mut self.mp_connect_target,
                                            i,
                                            format!("{} ({})", d.name, d.mac),
                                        );
                                    }
                                }
                            });
                        if ui
                            .add_enabled(
                                self.mp_connect_target != 0,
                                egui::Button::new(format!("Connect to Slot {slot}")),
                            )
                            .clicked()
                            && let Ok(mut hp) = self.headphones.lock()
                        {
                            hp.set_multi_point_connection(slot, self.mp_connect_target);
                            self.mp_connect_target = 0;
                        }
                    });
                }
            }
        });
    }

    fn draw_multipoint_slot(
        &mut self,
        ui: &mut egui::Ui,
        slot: usize,
        dev: Option<&BluetoothDevice>,
    ) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("Slot {slot}:")).strong());
            if let Some(d) = dev.filter(|d| !d.name.is_empty()) {
                ui.label(&d.name);
                ui.label(
                    egui::RichText::new(format!("({})", d.mac))
                        .small()
                        .color(STATUS_MUTED),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Disconnect").clicked()
                        && let Ok(mut hp) = self.headphones.lock()
                    {
                        hp.set_multi_point_connection(slot, 0);
                    }
                });
            } else {
                ui.label(egui::RichText::new("Empty").color(STATUS_MUTED));
            }
        });
    }

    fn draw_vpt_section(&mut self, ui: &mut egui::Ui) {
        section_card(ui, |ui| {
            ui.label(
                egui::RichText::new("Virtual Sound (VPT)")
                    .strong()
                    .size(15.0),
            );
            ui.label(
                egui::RichText::new("Only one spatial effect can be active at a time.")
                    .small()
                    .color(STATUS_MUTED),
            );
            ui.add_space(4.0);

            let prev_sp = self.sound_position_idx;
            egui::ComboBox::from_label("Sound Position")
                .selected_text(SOUND_POS_LABELS[self.sound_position_idx])
                .show_ui(ui, |ui| {
                    for (i, label) in SOUND_POS_LABELS.iter().enumerate() {
                        ui.selectable_value(&mut self.sound_position_idx, i, *label);
                    }
                });
            if self.sound_position_idx != prev_sp {
                self.vpt_idx = 0;
            }

            let prev_vpt = self.vpt_idx;
            egui::ComboBox::from_label("Surround (VPT)")
                .selected_text(VPT_LABELS[self.vpt_idx])
                .show_ui(ui, |ui| {
                    for (i, label) in VPT_LABELS.iter().enumerate() {
                        ui.selectable_value(&mut self.vpt_idx, i, *label);
                    }
                });
            if self.vpt_idx != prev_vpt {
                self.sound_position_idx = 0;
            }
        });
    }

    fn draw_eq_section(&mut self, ui: &mut egui::Ui) {
        section_card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Equalizer (5-Band + Clear Bass)")
                        .strong()
                        .size(15.0),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Reset Flat").clicked() {
                        self.eq_bands = [0; 6];
                        self.eq_preset = 0xFF;
                    }
                });
            });
            ui.add_space(4.0);

            let preset_name = EQ_PRESETS
                .iter()
                .find(|(id, _)| *id == self.eq_preset)
                .map_or("Custom", |(_, name)| *name);
            egui::ComboBox::from_label("Preset")
                .selected_text(preset_name)
                .show_ui(ui, |ui| {
                    for (id, name) in EQ_PRESETS {
                        ui.selectable_value(&mut self.eq_preset, id, name);
                    }
                });

            ui.add_space(4.0);
            for (idx, label) in EQ_BAND_LABELS.iter().enumerate() {
                if ui
                    .add(egui::Slider::new(&mut self.eq_bands[idx], -10..=10).text(*label))
                    .changed()
                {
                    self.eq_preset = 0xFF;
                }
            }
        });
    }

    fn draw_dsee_section(&mut self, ui: &mut egui::Ui) {
        section_card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("DSEE Extreme").strong().size(15.0));
                    ui.label(
                        egui::RichText::new("Upscales compressed audio sources in real time.")
                            .small()
                            .color(STATUS_MUTED),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.checkbox(&mut self.dsee_enabled, "Enabled");
                });
            });
        });
    }

    fn flush_headphone_changes(&mut self, ui: &mut egui::Ui) {
        if self.send_rx.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(
                    egui::RichText::new("Applying changes...")
                        .small()
                        .color(STATUS_MUTED),
                );
            });
            return;
        }

        let Ok(mut hp) = self.headphones.try_lock() else {
            return;
        };
        hp.ambient_sound_control.desired = self.ambient_sound_control;
        hp.asm_level.desired = self.asm_level;
        hp.focus_on_voice.desired = self.focus_on_voice;
        hp.surround_position.desired = SOUND_POSITION_PRESET_ARRAY[self.sound_position_idx];
        hp.vpt_type.desired = self.vpt_idx as i32;
        hp.speak_to_chat.desired = if self.s2c_enabled {
            S2cToggle::Active
        } else {
            S2cToggle::Inactive
        };
        hp.set_s2c_options(self.s2c_sens, self.s2c_voice, self.s2c_off_time);
        hp.eq_preset.desired = self.eq_preset;
        hp.eq_bands.desired = self.eq_bands;
        hp.dsee_enabled.desired = self.dsee_enabled;

        if !hp.is_changed() {
            self.last_edit_at = None;
            return;
        }

        if ui.input(|i| i.pointer.any_down()) {
            let now = Instant::now();
            let last = *self.last_edit_at.get_or_insert(now);
            if now.duration_since(last) < Duration::from_millis(150) {
                ui.ctx().request_repaint_after(Duration::from_millis(50));
                return;
            }
            self.last_edit_at = Some(now);
        } else {
            self.last_edit_at = None;
        }

        let bt = Arc::clone(&self.bt);
        let hp_arc = Arc::clone(&self.headphones);
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let res = hp_arc
                .lock()
                .map_err(|_| HeadphonesError::runtime("Headphones mutex poisoned".to_string()))
                .and_then(|mut g| g.set_changes(&bt));
            let _ = tx.send(res);
        });
        self.send_rx = Some(rx);
    }
}

impl eframe::App for SonyHeadphonesApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_async_tasks();
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.draw_header(ui);
                ui.add_space(8.0);
                self.draw_errors(ui);
                self.draw_device_discovery(ui);
                if self.bt.is_connected() {
                    ui.add_space(4.0);
                    self.draw_controls(ui);
                }
                ui.add_space(8.0);
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Not affiliated with Sony. Use at your own risk.")
                            .small()
                            .color(STATUS_MUTED),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.hyperlink_to(
                            egui::RichText::new("GitHub").small(),
                            "https://github.com/Plutoberth/SonyHeadphonesClient",
                        );
                    });
                });
            });
        });
    }
}

fn configure_fonts_and_style(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "CascadiaCode".to_owned(),
        Arc::new(egui::FontData::from_static(CASCADIA_FONT)),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "CascadiaCode".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "CascadiaCode".to_owned());
    ctx.set_fonts(fonts);

    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.visuals = egui::Visuals::dark();
    style.visuals.selection.bg_fill = ACCENT_BLUE;
    style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
    style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
    style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);
    ctx.set_style_of(egui::Theme::Dark, style);
    ctx.set_theme(egui::Theme::Dark);
}

fn section_card(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(CARD_BG)
        .stroke(egui::Stroke::new(1.0, CARD_STROKE))
        .corner_radius(10.0)
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add_contents(ui);
        });
    ui.add_space(6.0);
}

fn capability_badge(ui: &mut egui::Ui, label: &str, supported: bool) {
    let color = if supported { ACCENT_BLUE } else { STATUS_MUTED };
    let text = if supported {
        format!("✓ {label}")
    } else {
        label.to_owned()
    };
    egui::Frame::new()
        .fill(color.linear_multiply(if supported { 0.14 } else { 0.06 }))
        .stroke(egui::Stroke::new(1.0, color.linear_multiply(0.35)))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(7, 2))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).small().color(color));
        });
}

fn create_platform_connector() -> Box<dyn IBluetoothConnector> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(win) = bluetooth_connector_rs::port::win::WindowsBluetoothConnector::new() {
            return Box::new(win);
        }
    }
    Box::new(MockBluetoothConnector::default())
}

fn push_mock_devices_packet(mock: &MockBluetoothConnector) {
    let mut payload = vec![CommandType::DevicesQueryResponse as u8, 0x01, 0x02];
    let dev1_mac = b"AA:11:22:33:44:55";
    let dev1_name = b"Work Laptop";
    payload.extend_from_slice(dev1_mac);
    payload.push(0x01);
    payload.push(dev1_name.len() as u8);
    payload.extend_from_slice(dev1_name);

    let dev2_mac = b"BB:66:77:88:99:00";
    let dev2_name = b"Phone";
    payload.extend_from_slice(dev2_mac);
    payload.push(0x00);
    payload.push(dev2_name.len() as u8);
    payload.extend_from_slice(dev2_name);

    if let Ok(pkt) = package_data_for_bt(&payload, DataType::DataMdrNo2, 0) {
        mock.push_incoming(pkt);
    }
}

fn build_xm4_simulator_connector() -> MockBluetoothConnector {
    let mock = MockBluetoothConnector::new(vec![
        BluetoothDevice {
            name: "WH-1000XM4".into(),
            mac: "11:22:33:44:55:66".into(),
        },
        BluetoothDevice {
            name: "WH-1000XM3".into(),
            mac: "AA:BB:CC:DD:EE:FF".into(),
        },
    ]);
    for i in 0..3u8 {
        if let Ok(ack) = package_data_for_bt(&[], DataType::Ack, i % 2) {
            mock.push_incoming(ack);
        }
    }
    if let Ok(cap) = package_data_for_bt(
        &[
            CommandType::CapabilityQueryResponse as u8,
            0x38,
            0x81,
            0x62,
            0xF5,
        ],
        DataType::DataMdr,
        0,
    ) {
        mock.push_incoming(cap);
    }
    push_mock_devices_packet(&mock);
    if let Ok(s2c) = package_data_for_bt(
        &[CommandType::S2cQueryResponse as u8, 0x05, 0x00, 0x00],
        DataType::DataMdr,
        0,
    ) {
        mock.push_incoming(s2c);
    }
    if let Ok(s2c_opts) = package_data_for_bt(
        &[
            CommandType::S2cOptionsResponse as u8,
            0x05,
            0x00,
            0x00,
            0x00,
            0x01,
        ],
        DataType::DataMdr,
        0,
    ) {
        mock.push_incoming(s2c_opts);
    }
    if let Ok(nc_asm) = package_data_for_bt(
        &[
            CommandType::NcAsmRetParam as u8,
            0x02,
            0x01,
            0x01,
            0x01,
            0x01,
            0x00,
            0x00,
        ],
        DataType::DataMdr,
        0,
    ) {
        mock.push_incoming(nc_asm);
    }
    if let Ok(eq_ret) = package_data_for_bt(
        &[
            CommandType::EqRet as u8,
            0x01,
            0x00,
            0x06,
            10,
            10,
            10,
            10,
            10,
            10,
        ],
        DataType::DataMdr,
        0,
    ) {
        mock.push_incoming(eq_ret);
    }
    if let Ok(dsee_ret) = package_data_for_bt(
        &[CommandType::DseeRet as u8, 0x02, 0x00, 0x00],
        DataType::DataMdr,
        0,
    ) {
        mock.push_incoming(dsee_ret);
    }
    for _ in 0..64 {
        if let Ok(ack) = package_data_for_bt(&[], DataType::Ack, 1) {
            mock.push_incoming(ack);
        }
    }
    mock
}
