#![forbid(unsafe_code)]

use bluetooth_connector_rs::port::BluetoothDevice;
use bluetooth_wrapper_rs::port::BluetoothWrapper;
use command_serializer_rs::port::*;
use constants_rs::port::*;
use exceptions_rs::port::HeadphonesError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property<T> {
    pub current: T,
    pub desired: T,
}

impl<T: Clone + PartialEq> Property<T> {
    pub fn symmetric(val: T) -> Self {
        Self {
            current: val.clone(),
            desired: val,
        }
    }
    pub fn fulfill(&mut self) {
        self.current = self.desired.clone();
    }
    pub fn is_fulfilled(&self) -> bool {
        self.current == self.desired
    }
    pub fn set_state(&mut self, val: T) {
        self.current = val.clone();
        self.desired = val;
    }
}

fn send_if_dirty<T: Clone + PartialEq>(
    conn: &BluetoothWrapper,
    prop: &mut Property<T>,
    cmd: impl FnOnce(&T) -> Vec<u8>,
) -> Result<(), HeadphonesError> {
    if !prop.is_fulfilled() {
        conn.send_command(&cmd(&prop.desired))?;
        prop.fulfill();
    }
    Ok(())
}

pub struct Headphones {
    model: HeadphoneModel,
    capabilities: u32,
    pub ambient_sound_control: Property<bool>,
    pub focus_on_voice: Property<bool>,
    pub asm_level: Property<i32>,
    pub surround_position: Property<SoundPositionPreset>,
    pub vpt_type: Property<i32>,
    pub optimizer_state: Property<OptimizerState>,
    pub speak_to_chat: Property<S2cToggle>,
    pub s2c_options: Property<u32>,
    pub saved_devices: Vec<BluetoothDevice>,
    pub dev1: Property<usize>,
    pub dev2: Property<usize>,
}

impl Default for Headphones {
    fn default() -> Self {
        Self::new(HeadphoneModel::Generic)
    }
}

impl Headphones {
    pub fn new(model: HeadphoneModel) -> Self {
        let caps = model.default_capabilities();
        let surround = if caps & device_capabilities::VPT != 0 {
            Property {
                current: SoundPositionPreset::OutOfRange,
                desired: SoundPositionPreset::Off,
            }
        } else {
            Property::symmetric(SoundPositionPreset::Off)
        };
        Self {
            model,
            capabilities: caps,
            ambient_sound_control: Property::symmetric(false),
            focus_on_voice: Property::symmetric(false),
            asm_level: Property::symmetric(0),
            surround_position: surround,
            vpt_type: Property::symmetric(0),
            optimizer_state: Property::symmetric(OptimizerState::Idle),
            speak_to_chat: Property::symmetric(S2cToggle::Inactive),
            s2c_options: Property::symmetric(0),
            saved_devices: vec![BluetoothDevice {
                name: String::new(),
                mac: String::new(),
            }],
            dev1: Property::symmetric(0),
            dev2: Property::symmetric(0),
        }
    }

    pub fn configure_for_device(&mut self, name: &str) {
        *self = Self::new(HeadphoneModel::from_device_name(name));
    }
    pub fn model(&self) -> HeadphoneModel {
        self.model
    }
    pub fn capabilities(&self) -> u32 {
        self.capabilities
    }
    pub fn max_asm_steps(&self) -> i8 {
        self.model.max_asm_steps()
    }
    pub fn is_focus_on_voice_available(&self) -> bool {
        self.ambient_sound_control.current && self.asm_level.current > MINIMUM_VOICE_FOCUS_STEP
    }
    pub fn is_set_asm_level_available(&self) -> bool {
        self.ambient_sound_control.current
    }
    pub fn set_s2c_options(&mut self, sensitivity: u8, voice: bool, off_time: u8) {
        self.s2c_options.desired =
            (u32::from(sensitivity) << 16) | (u32::from(voice) << 8) | u32::from(off_time);
    }
    pub fn get_connected_device_indices(&self) -> (usize, usize) {
        (self.dev1.current, self.dev2.current)
    }
    pub fn set_multi_point_connection(&mut self, slot: usize, new_device: usize) {
        if slot == 1 {
            self.dev1.desired = new_device;
        } else {
            self.dev2.desired = new_device;
        }
    }

    pub fn is_changed(&self) -> bool {
        let vpt_changed = self.capabilities & device_capabilities::VPT != 0
            && !(self.surround_position.is_fulfilled() && self.vpt_type.is_fulfilled());
        vpt_changed
            || !(self.ambient_sound_control.is_fulfilled()
                && self.asm_level.is_fulfilled()
                && self.focus_on_voice.is_fulfilled()
                && self.optimizer_state.is_fulfilled()
                && self.speak_to_chat.is_fulfilled()
                && self.s2c_options.is_fulfilled()
                && self.dev1.is_fulfilled()
                && self.dev2.is_fulfilled())
    }

    pub fn set_changes(&mut self, conn: &BluetoothWrapper) -> Result<(), HeadphonesError> {
        send_if_dirty(conn, &mut self.optimizer_state, |&s| {
            serialize_xm4_optimize_command(s)
        })?;
        send_if_dirty(conn, &mut self.speak_to_chat, |&s| {
            serialize_xm4_speak_to_chat(s)
        })?;
        send_if_dirty(conn, &mut self.s2c_options, |&o| {
            serialize_xm4_s2c_options(
                ((o >> 16) & 0xff) as u8,
                ((o >> 8) & 0xff) as u8,
                (o & 0xff) as u8,
            )
        })?;
        if !(self.ambient_sound_control.is_fulfilled()
            && self.focus_on_voice.is_fulfilled()
            && self.asm_level.is_fulfilled())
        {
            let eff = if self.ambient_sound_control.desired {
                NcAsmEffect::AdjustmentCompletion
            } else {
                NcAsmEffect::Off
            };
            let id = if self.focus_on_voice.desired {
                AsmId::Voice
            } else {
                AsmId::Normal
            };
            let lvl = if self.ambient_sound_control.desired {
                self.asm_level.desired as i8
            } else {
                ASM_LEVEL_DISABLED
            };
            conn.send_command(&serialize_nc_and_asm_setting(
                eff,
                NcAsmSettingType::LevelAdjustment,
                AsmSettingType::LevelAdjustment,
                id,
                lvl,
                self.max_asm_steps(),
            )?)?;
            self.ambient_sound_control.fulfill();
            self.asm_level.fulfill();
            self.focus_on_voice.fulfill();
        }
        if self.capabilities & device_capabilities::VPT != 0
            && !(self.vpt_type.is_fulfilled() && self.surround_position.is_fulfilled())
        {
            let (kind, preset) = if self.vpt_type.desired != 0 {
                (VptInquiredType::Vpt, self.vpt_type.desired as u8)
            } else if self.surround_position.desired != SoundPositionPreset::Off {
                (
                    VptInquiredType::SoundPosition,
                    self.surround_position.desired as u8,
                )
            } else if self.surround_position.current != SoundPositionPreset::Off {
                (
                    VptInquiredType::SoundPosition,
                    SoundPositionPreset::Off as u8,
                )
            } else if self.vpt_type.current != 0 {
                (VptInquiredType::Vpt, 0)
            } else {
                return Err(HeadphonesError::runtime("VPT and surround both zero"));
            };
            conn.send_command(&serialize_vpt_setting(kind, preset))?;
            self.vpt_type.fulfill();
            self.surround_position.fulfill();
        }
        Self::sync_slot(conn, &self.saved_devices, &mut self.dev1)?;
        Self::sync_slot(conn, &self.saved_devices, &mut self.dev2)?;
        self.apply_pending(conn);
        Ok(())
    }

    fn sync_slot(
        conn: &BluetoothWrapper,
        devices: &[BluetoothDevice],
        slot: &mut Property<usize>,
    ) -> Result<(), HeadphonesError> {
        if slot.is_fulfilled() {
            return Ok(());
        }
        if let Some(d) = (slot.current != 0)
            .then(|| devices.get(slot.current))
            .flatten()
        {
            conn.send_command_with_type(
                &serialize_multi_point_command(MultiPointCommand::Disconnect, &d.mac),
                DataType::DataMdrNo2,
            )?;
        }
        if let Some(d) = (slot.desired != 0)
            .then(|| devices.get(slot.desired))
            .flatten()
        {
            conn.send_command_with_type(
                &serialize_multi_point_command(MultiPointCommand::Connect, &d.mac),
                DataType::DataMdrNo2,
            )?;
        }
        slot.fulfill();
        Ok(())
    }

    pub fn query_state(&mut self, conn: &BluetoothWrapper) -> Result<(), HeadphonesError> {
        let queries = [
            (DataType::DataMdr, [CommandType::InitRequest as u8, 0x00]),
            (DataType::DataMdr, [CommandType::MacQuery as u8, 0x00]),
            (
                DataType::DataMdr,
                [CommandType::CapabilityQuery as u8, 0x01],
            ),
            (
                DataType::DataMdrNo2,
                [CommandType::MultiPointDevicesQuery as u8, 0x01],
            ),
            (DataType::DataMdr, [CommandType::S2cQuery as u8, 0x05]),
            (
                DataType::DataMdr,
                [CommandType::S2cOptionsQuery as u8, 0x05],
            ),
            (DataType::DataMdr, [CommandType::NcAsmQuery as u8, 0x02]),
        ];
        for (dtype, payload) in queries {
            conn.send_command_with_type(&payload, dtype)?;
        }
        self.apply_pending(conn);
        Ok(())
    }

    pub fn apply_pending(&mut self, conn: &BluetoothWrapper) {
        for msg in conn.drain_pending() {
            self.set_state_from_reply(&msg);
        }
    }

    pub fn set_state_from_reply(&mut self, msg: &BtMessage) {
        let b = &msg.message_bytes;
        let Ok(cmd) = CommandType::try_from(*b.first().unwrap_or(&0xFF)) else {
            return;
        };
        match cmd {
            CommandType::DevicesQueryResponse | CommandType::DevicesStateResponse => {
                self.parse_devices_reply(b);
            }
            CommandType::Xm4OptimizerResponse if b.len() >= 3 => {
                self.optimizer_state.set_state(if b[2] != 0 {
                    OptimizerState::Optimizing
                } else {
                    OptimizerState::Idle
                });
            }
            CommandType::CapabilityQueryResponse => {
                let caps = b[1..].iter().fold(0u32, |acc, &x| {
                    acc | match FunctionType::from_byte(x) {
                        Some(FunctionType::DeviceManagement) => device_capabilities::MULTI_POINT,
                        Some(FunctionType::Vpt) => device_capabilities::VPT,
                        Some(FunctionType::Optimizer) => device_capabilities::OPTIMIZER,
                        Some(FunctionType::NcAsm) => device_capabilities::NC_ASM,
                        Some(FunctionType::SmartTalkingMode) => device_capabilities::SPEAK_TO_CHAT,
                        None => 0,
                    }
                });
                if caps != 0 {
                    self.capabilities = caps;
                }
            }
            CommandType::S2cQueryResponse | CommandType::S2cNotify if b.len() >= 4 => {
                self.speak_to_chat.set_state(if b[3] != 0 {
                    S2cToggle::Active
                } else {
                    S2cToggle::Inactive
                });
            }
            CommandType::S2cOptionsResponse | CommandType::S2cOptionsNotify if b.len() >= 6 => {
                self.s2c_options
                    .set_state((u32::from(b[3]) << 16) | (u32::from(b[4]) << 8) | u32::from(b[5]));
            }
            CommandType::NcAsmRetParam | CommandType::NcAsmNotify if b.len() >= 8 => {
                self.ambient_sound_control.set_state(b[2] != 0);
                self.focus_on_voice.set_state(b[6] == AsmId::Voice as u8);
                self.asm_level
                    .set_state(i32::from(b[7] as i8).clamp(0, i32::from(self.max_asm_steps())));
            }
            _ => {}
        }
    }

    fn parse_devices_reply(&mut self, b: &[u8]) {
        if b.get(1).copied() != Some(0x01) || b.len() < 3 {
            return;
        }
        let mut saved = vec![BluetoothDevice {
            name: String::new(),
            mac: String::new(),
        }];
        let (mut d1, mut d2, mut idx) = (0usize, 0usize, 3usize);
        for _ in 0..usize::from(b[2]) {
            let (Some(mac_s), Some(&slot), Some(&nlen)) = (
                b.get(idx..idx + MAC_ADDR_STR_SIZE),
                b.get(idx + MAC_ADDR_STR_SIZE),
                b.get(idx + MAC_ADDR_STR_SIZE + 1),
            ) else {
                return;
            };
            idx += MAC_ADDR_STR_SIZE + 2;
            let Some(name_s) = b.get(idx..idx + usize::from(nlen)) else {
                return;
            };
            idx += usize::from(nlen);
            saved.push(BluetoothDevice {
                name: String::from_utf8_lossy(name_s).into_owned(),
                mac: String::from_utf8_lossy(mac_s).into_owned(),
            });
            if slot == 0x01 {
                d1 = saved.len() - 1;
            } else if slot == 0x02 {
                d2 = saved.len() - 1;
            }
        }
        self.saved_devices = saved;
        self.dev1.set_state(d1);
        self.dev2.set_state(d2);
    }
}
