use std::process::ExitCode;

use bluetooth_connector_rs::port::{BluetoothDevice, MockBluetoothConnector};
use bluetooth_wrapper_rs::port::BluetoothWrapper;
use byte_magic_rs::port::mac_string_to_long;
use clap::{Parser, Subcommand, ValueEnum};
use command_serializer_rs::port::{
    package_data_for_bt, serialize_multi_point_command, serialize_nc_and_asm_setting,
    serialize_vpt_setting, serialize_xm4_optimize_command, serialize_xm4_s2c_options,
    serialize_xm4_speak_to_chat, unpack_bt_message,
};
use constants_rs::port::{
    ASM_LEVEL_DISABLED, AsmId, AsmSettingType, CommandType, DataType, END_MARKER, HeadphoneModel,
    MultiPointCommand, NcAsmEffect, NcAsmSettingType, OptimizerState, S2cToggle,
    SOUND_POSITION_PRESET_ARRAY, START_MARKER, VptInquiredType,
};
use headphones_rs::port::Headphones;

#[derive(Debug, Parser)]
#[command(name = "sony-headphones-client", version)]
#[command(about = "Sony Headphones Client (WH-1000-XM3 & WH-1000-XM4)")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ModelArg {
    Xm3,
    Xm4,
}

impl From<ModelArg> for HeadphoneModel {
    fn from(m: ModelArg) -> Self {
        match m {
            ModelArg::Xm3 => Self::Wh1000Xm3,
            ModelArg::Xm4 => Self::Wh1000Xm4,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum MultiPointActionArg {
    Disconnect,
    Connect,
    Unpair,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Launch the graphical desktop interface (default when no subcommand is passed)
    Gui,
    /// List supported headset models and WH-1000-XM4 capabilities
    Models,
    /// Encode a framed NC/ASM command packet (steps 0..=19 for XM3, 0..=20 for XM4)
    EncodeNcAsm {
        #[arg(long, value_enum, default_value = "xm4")]
        model: ModelArg,
        #[arg(long, default_value_t = true)]
        enabled: bool,
        #[arg(long, default_value_t = 0)]
        level: i8,
        #[arg(long, default_value_t = false)]
        voice: bool,
        #[arg(long, default_value_t = 0)]
        seq: u8,
    },
    /// Encode a framed VPT / Sound Position command packet
    EncodeVpt {
        #[arg(long, default_value = "vpt")]
        mode: String,
        #[arg(long, default_value_t = 0)]
        preset: u8,
        #[arg(long, default_value_t = 0)]
        seq: u8,
    },
    /// Encode a WH-1000-XM4 Speak-to-Chat toggle command
    EncodeXm4S2c {
        #[arg(long)]
        active: bool,
        #[arg(long, default_value_t = 0)]
        seq: u8,
    },
    /// Encode a WH-1000-XM4 Speak-to-Chat options command
    EncodeXm4S2cOptions {
        #[arg(long, default_value_t = 0)]
        sensitivity: u8,
        #[arg(long)]
        voice: bool,
        #[arg(long, default_value_t = 0)]
        off_time: u8,
        #[arg(long, default_value_t = 0)]
        seq: u8,
    },
    /// Encode a WH-1000-XM4 ANC Optimizer command
    EncodeXm4Optimizer {
        #[arg(long)]
        optimizing: bool,
        #[arg(long, default_value_t = 0)]
        seq: u8,
    },
    /// Encode a WH-1000-XM4 Multipoint device command (DATA_MDR_NO2)
    EncodeMultipoint {
        #[arg(long, value_enum)]
        action: MultiPointActionArg,
        #[arg(long)]
        mac: String,
        #[arg(long, default_value_t = 0)]
        seq: u8,
    },
    /// Decode a hex-encoded framed packet (with or without '>'..'<' markers)
    DecodeFrame {
        #[arg(long)]
        hex: String,
    },
    /// Convert a Bluetooth MAC string into its numeric 48-bit integer value
    MacToInt {
        #[arg(long)]
        mac: String,
    },
    /// Run a deterministic offline WH-1000-XM4 session against MockBluetoothConnector
    SimulateXm4,
}

fn format_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_hex(s: &str) -> Result<Vec<u8>, String> {
    let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if !cleaned.len().is_multiple_of(2) {
        return Err("Hex input must have an even number of digits".into());
    }
    (0..cleaned.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&cleaned[i..i + 2], 16)
                .map_err(|e| format!("Invalid hex byte at offset {i}: {e}"))
        })
        .collect()
}

pub fn run_command(cmd: Commands) -> ExitCode {
    match exec_command(cmd) {
        Ok(out) => {
            if !out.is_empty() {
                println!("{out}");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn exec_command(cmd: Commands) -> Result<String, String> {
    match cmd {
        Commands::Gui => Ok(String::new()),
        Commands::Models => Ok([
            "WH-1000XM3 | max_asm_steps=19 | features=nc_asm,vpt,sound_position",
            "WH-1000XM4 | max_asm_steps=20 | features=nc_asm,speak_to_chat,anc_optimizer,multipoint",
        ]
        .join("\n")),
        Commands::EncodeNcAsm {
            model,
            enabled,
            level,
            voice,
            seq,
        } => {
            let hp_model: HeadphoneModel = model.into();
            let effect = if enabled {
                NcAsmEffect::AdjustmentCompletion
            } else {
                NcAsmEffect::Off
            };
            let asm_id = if voice { AsmId::Voice } else { AsmId::Normal };
            let asm_level = if enabled { level } else { ASM_LEVEL_DISABLED };
            let payload = serialize_nc_and_asm_setting(
                effect,
                NcAsmSettingType::LevelAdjustment,
                AsmSettingType::LevelAdjustment,
                asm_id,
                asm_level,
                hp_model.max_asm_steps(),
            )
            .map_err(|e| e.to_string())?;
            let framed =
                package_data_for_bt(&payload, DataType::DataMdr, seq).map_err(|e| e.to_string())?;
            Ok(format_hex(&framed))
        }
        Commands::EncodeVpt { mode, preset, seq } => {
            let kind = match mode.as_str() {
                "vpt" => VptInquiredType::Vpt,
                "sound-position" => {
                    let _ = SOUND_POSITION_PRESET_ARRAY.get(usize::from(preset));
                    VptInquiredType::SoundPosition
                }
                other => return Err(format!("Unsupported VPT mode: {other}")),
            };
            let payload = serialize_vpt_setting(kind, preset);
            let framed =
                package_data_for_bt(&payload, DataType::DataMdr, seq).map_err(|e| e.to_string())?;
            Ok(format_hex(&framed))
        }
        Commands::EncodeXm4S2c { active, seq } => {
            let toggle = if active {
                S2cToggle::Active
            } else {
                S2cToggle::Inactive
            };
            let payload = serialize_xm4_speak_to_chat(toggle);
            let framed =
                package_data_for_bt(&payload, DataType::DataMdr, seq).map_err(|e| e.to_string())?;
            Ok(format_hex(&framed))
        }
        Commands::EncodeXm4S2cOptions {
            sensitivity,
            voice,
            off_time,
            seq,
        } => {
            if sensitivity > 2 || off_time > 3 {
                return Err("sensitivity must be 0..=2 and off-time must be 0..=3".into());
            }
            let payload = serialize_xm4_s2c_options(sensitivity, u8::from(voice), off_time);
            let framed =
                package_data_for_bt(&payload, DataType::DataMdr, seq).map_err(|e| e.to_string())?;
            Ok(format_hex(&framed))
        }
        Commands::EncodeXm4Optimizer { optimizing, seq } => {
            let state = if optimizing {
                OptimizerState::Optimizing
            } else {
                OptimizerState::Idle
            };
            let payload = serialize_xm4_optimize_command(state);
            let framed =
                package_data_for_bt(&payload, DataType::DataMdr, seq).map_err(|e| e.to_string())?;
            Ok(format_hex(&framed))
        }
        Commands::EncodeMultipoint { action, mac, seq } => {
            mac_string_to_long(&mac).map_err(|e| e.to_string())?;
            let mp = match action {
                MultiPointActionArg::Disconnect => MultiPointCommand::Disconnect,
                MultiPointActionArg::Connect => MultiPointCommand::Connect,
                MultiPointActionArg::Unpair => MultiPointCommand::Unpair,
            };
            let payload = serialize_multi_point_command(mp, &mac);
            let framed = package_data_for_bt(&payload, DataType::DataMdrNo2, seq)
                .map_err(|e| e.to_string())?;
            Ok(format_hex(&framed))
        }
        Commands::DecodeFrame { hex } => {
            let raw = parse_hex(&hex)?;
            let inner = if raw.first() == Some(&START_MARKER) && raw.last() == Some(&END_MARKER) {
                &raw[1..raw.len() - 1]
            } else {
                &raw[..]
            };
            let msg = unpack_bt_message(inner).map_err(|e| e.to_string())?;
            Ok(format!(
                "data_type={:?} seq={} payload={}",
                msg.data_type,
                msg.seq_number,
                format_hex(&msg.message_bytes)
            ))
        }
        Commands::MacToInt { mac } => {
            let val = mac_string_to_long(&mac).map_err(|e| e.to_string())?;
            Ok(format!("0x{val:012x}"))
        }
        Commands::SimulateXm4 => simulate_xm4_session(),
    }
}

fn simulate_xm4_session() -> Result<String, String> {
    let mock = MockBluetoothConnector::new(vec![BluetoothDevice {
        name: "WH-1000XM4".into(),
        mac: "11:22:33:44:55:66".into(),
    }]);
    let recv = mock.recv_queue.clone();
    let wrapper = BluetoothWrapper::new(Box::new(mock));
    wrapper
        .connect("11:22:33:44:55:66")
        .map_err(|e| e.to_string())?;

    let mut hp = Headphones::default();
    hp.configure_for_device("WH-1000XM4");

    let cap_reply = package_data_for_bt(
        &[
            CommandType::CapabilityQueryResponse as u8,
            0x38, // DeviceManagement
            0x81, // Optimizer
            0x62, // NcAsm
            0xF5, // SmartTalkingMode
        ],
        DataType::DataMdr,
        0,
    )
    .map_err(|e| e.to_string())?;
    let mut dev_payload = vec![CommandType::DevicesQueryResponse as u8, 0x01, 0x01];
    dev_payload.extend_from_slice(b"AA:BB:CC:DD:EE:FF");
    dev_payload.push(0x01);
    dev_payload.push(6);
    dev_payload.extend_from_slice(b"Laptop");
    let dev_reply =
        package_data_for_bt(&dev_payload, DataType::DataMdrNo2, 0).map_err(|e| e.to_string())?;

    {
        let mut q = recv.lock().map_err(|_| "lock poisoned".to_string())?;
        for i in 0..3u8 {
            q.push_back(package_data_for_bt(&[], DataType::Ack, i % 2).map_err(|e| e.to_string())?);
        }
        q.push_back(cap_reply);
        q.push_back(package_data_for_bt(&[], DataType::Ack, 1).map_err(|e| e.to_string())?);
        q.push_back(dev_reply);
        for _ in 0..5 {
            q.push_back(package_data_for_bt(&[], DataType::Ack, 0).map_err(|e| e.to_string())?);
        }
    }
    hp.query_state(&wrapper).map_err(|e| e.to_string())?;

    hp.ambient_sound_control.desired = true;
    hp.asm_level.desired = 20;
    hp.focus_on_voice.desired = true;
    hp.speak_to_chat.desired = S2cToggle::Active;
    hp.set_s2c_options(1, true, 2);
    hp.optimizer_state.desired = OptimizerState::Optimizing;

    {
        let mut q = recv.lock().map_err(|_| "lock poisoned".to_string())?;
        for i in 0..4u8 {
            q.push_back(package_data_for_bt(&[], DataType::Ack, i % 2).map_err(|e| e.to_string())?);
        }
    }
    hp.set_changes(&wrapper).map_err(|e| e.to_string())?;

    let (d1, d2) = hp.get_connected_device_indices();
    Ok(format!(
        "model=WH-1000XM4 max_asm={} asm_level={} voice_focus={} s2c={:?} optimizer={:?} multipoint=({}, {})",
        hp.max_asm_steps(),
        hp.asm_level.current,
        hp.is_focus_on_voice_available(),
        hp.speak_to_chat.current,
        hp.optimizer_state.current,
        hp.saved_devices[d1].name,
        hp.saved_devices[d2].name,
    ))
}
