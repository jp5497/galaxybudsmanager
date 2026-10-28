use serde::{Deserialize, Serialize};
use std::fmt;

pub const SOM: u8 = 0xFD;
pub const EOM: u8 = 0xDD;
pub const BUDS4_PRO_SPP_UUID: &str = "2e73a4ad-332d-41fc-90e2-16bef06523f2";

const CRC16_TAB: [u16; 256] = [
    0, 4129, 8258, 12387, 16516, 20645, 24774, 28903, 33032, 37161, 41290, 45419, 49548, 53677,
    57806, 61935, 4657, 528, 12915, 8786, 21173, 17044, 29431, 25302, 37689, 33560, 45947, 41818,
    54205, 50076, 62463, 58334, 9314, 13379, 1056, 5121, 25830, 29895, 17572, 21637, 42346, 46411,
    34088, 38153, 58862, 62927, 50604, 54669, 13907, 9842, 5649, 1584, 30423, 26358, 22165, 18100,
    46939, 42874, 38681, 34616, 63455, 59390, 55197, 51132, 18628, 22757, 26758, 30887, 2112, 6241,
    10242, 14371, 51660, 55789, 59790, 63919, 35144, 39273, 43274, 47403, 23285, 19156, 31415,
    27286, 6769, 2640, 14899, 10770, 56317, 52188, 64447, 60318, 39801, 35672, 47931, 43802, 27814,
    31879, 19684, 23749, 11298, 15363, 3168, 7233, 60846, 64911, 52716, 56781, 44330, 48395,
    36200, 40265, 32407, 28342, 24277, 20212, 15891, 11826, 7761, 3696, 65439, 61374, 57309, 53244,
    48923, 44858, 40793, 36728, 37256, 33193, 45514, 41451, 53516, 49453, 61774, 57711, 4224, 161,
    12482, 8419, 20484, 16421, 28742, 24679, 33721, 37784, 41979, 46042, 49981, 54044, 58239,
    62302, 689, 4752, 8947, 13010, 16949, 21012, 25207, 29270, 46570, 42443, 38312, 34185, 62830,
    58703, 54572, 50445, 13538, 9411, 5280, 1153, 29798, 25671, 21540, 17413, 42971, 47098, 34713,
    38840, 59231, 63358, 50973, 55100, 9939, 14066, 1681, 5808, 26199, 30326, 17941, 22068, 55628,
    51565, 63758, 59695, 39368, 35305, 47498, 43435, 22596, 18533, 30726, 26663, 6336, 2273, 14466,
    10403, 52093, 56156, 60223, 64286, 35833, 39896, 43963, 48026, 19061, 23124, 27191, 31254,
    2801, 6864, 10931, 14994, 64814, 60687, 56684, 52557, 48554, 44427, 40424, 36297, 31782, 27655,
    23652, 19525, 15522, 11395, 7392, 3265, 61215, 65342, 53085, 57212, 44955, 49082, 36825, 40952,
    28183, 32310, 20053, 24180, 11923, 16050, 3793, 7920,
];

pub fn crc16_ccitt(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &b in data {
        let idx = ((crc >> 8) ^ (b as u16)) & 0xFF;
        crc = CRC16_TAB[idx as usize] ^ (crc << 8);
    }
    crc
}

pub fn crc16_for_msg(msg_id: u8, payload: &[u8]) -> [u8; 2] {
    let mut buf = Vec::with_capacity(1 + payload.len());
    buf.push(msg_id);
    buf.extend_from_slice(payload);
    crc16_ccitt(&buf).to_le_bytes()
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MsgId {
    DebugSku = 34,
    DebugGetVersion = 36,
    DebugGetAllData = 38,
    DebugBuildInfo = 40,
    DebugSerialNumber = 41,
    Acknowledgement = 66,
    ExtraClearSoundCall = 72,
    Reset = 80,
    StatusUpdated = 96,
    ExtendedStatusUpdated = 97,
    CustomEqualizeRecv = 105,
    PauseMediaWhenOneBudRemoved = 108,
    SetCallPathControl = 110,
    SetAncWithOneEarbud = 111,
    PairingMode = 114,
    NoiseControlsUpdate = 119,
    NoiseControls = 120,
    SetTouchAndHoldNoiseControls = 121,
    SetDetectConversations = 122,
    SetDetectConversationsDuration = 123,
    SetSpatialAudio = 124,
    SetSpeakSeamlessly = 125,
    CustomizeAmbientSound = 130,
    NoiseReductionLevel = 131,
    AmbientVolume = 132,
    AdjustSoundSync = 133,
    Equalizer = 134,
    GameMode = 135,
    ManagerInfo = 136,
    CustomEqualizeSend = 137,
    SetSidetone = 139,
    BudsLightingSync = 142,
    SetHearingEnhancements = 143,
    LockTouchpad = 144,
    TouchUpdated = 145,
    SetTouchpadOption = 146,
    OutsideDoubleTap = 149,
    ExtraHighAmbient = 150,
    SetVoiceWakeUp = 151,
    CheckTheFitOfEarbuds = 157,
    FindMyEarbudsStart = 160,
    FindMyEarbudsStop = 161,
    MuteEarbud = 162,
    MuteEarbudStatusUpdated = 163,
    UpdateTime = 167,
    SelfTest = 171,
    SetSeamlessConnection = 175,
    SetAdaptiveVolumeEnabled = 197,
    CradleSerialNumber = 205,
    UascSirenDetect = 222,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MsgType {
    Request = 0,
    Response = 1,
}

#[derive(Debug, Clone)]
pub struct SppMessage {
    pub id: u8,
    pub msg_type: MsgType,
    pub is_fragment: bool,
    pub payload: Vec<u8>,
}

impl SppMessage {
    pub fn new_request(id: MsgId, payload: Vec<u8>) -> Self {
        Self {
            id: id as u8,
            msg_type: MsgType::Request,
            is_fragment: false,
            payload,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let size = (1 + self.payload.len() + 2) as u16;
        let mut header = size.to_le_bytes();
        if self.is_fragment {
            header[1] |= 0x20;
        }
        if self.msg_type == MsgType::Response {
            header[1] |= 0x10;
        }
        let crc = crc16_for_msg(self.id, &self.payload);

        let mut out = Vec::with_capacity(1 + 2 + 1 + self.payload.len() + 2 + 1);
        out.push(SOM);
        out.extend_from_slice(&header);
        out.push(self.id);
        out.extend_from_slice(&self.payload);
        out.extend_from_slice(&crc);
        out.push(EOM);
        out
    }

    /// Attempts to decode one packet from the start of `buf`, returning `(SppMessage, bytes_consumed)`.
    pub fn decode_one(buf: &[u8]) -> Result<(Self, usize), PacketError> {
        if buf.len() < 7 {
            return Err(PacketError::Incomplete);
        }
        if buf[0] != SOM {
            return Err(PacketError::InvalidSom);
        }
        let header = u16::from_le_bytes([buf[1], buf[2]]);
        let is_fragment = (header & 0x2000) != 0;
        let msg_type = if (header & 0x1000) != 0 {
            MsgType::Request
        } else {
            MsgType::Response
        };
        let size = (header & 0x03FF) as usize;
        if size < 3 {
            return Err(PacketError::InvalidSize);
        }
        let total_packet_size = 1 + 2 + size + 1;
        if buf.len() < total_packet_size {
            return Err(PacketError::Incomplete);
        }
        if buf[total_packet_size - 1] != EOM {
            return Err(PacketError::InvalidEom);
        }

        let id = buf[3];
        let payload_len = size - 3;
        let payload = buf[4..4 + payload_len].to_vec();
        let crc1 = buf[4 + payload_len];
        let crc2 = buf[4 + payload_len + 1];

        let mut crc_check = Vec::with_capacity(size);
        crc_check.push(id);
        crc_check.extend_from_slice(&payload);
        // Swapped CRC bytes as per Samsung protocol
        crc_check.push(crc2);
        crc_check.push(crc1);

        if crc16_ccitt(&crc_check) != 0 {
            return Err(PacketError::ChecksumMismatch);
        }

        Ok((
            Self {
                id,
                msg_type,
                is_fragment,
                payload,
            },
            total_packet_size,
        ))
    }

    /// Consumes all complete messages from a stream buffer, discarding any corrupted bytes cleanly.
    pub fn drain_buffer(buf: &mut Vec<u8>) -> Vec<Self> {
        let mut messages = Vec::new();
        loop {
            if buf.is_empty() {
                break;
            }
            // Align to SOM (0xFD)
            if buf[0] != SOM {
                if let Some(pos) = buf.iter().position(|&b| b == SOM) {
                    buf.drain(0..pos);
                } else {
                    buf.clear();
                    break;
                }
            }
            match Self::decode_one(buf) {
                Ok((msg, consumed)) => {
                    buf.drain(0..consumed);
                    messages.push(msg);
                }
                Err(PacketError::Incomplete) => {
                    break;
                }
                Err(_) => {
                    // Corrupted frame: skip past current SOM and search for next SOM
                    if let Some(pos) = buf.iter().skip(1).position(|&b| b == SOM) {
                        buf.drain(0..pos + 1);
                    } else {
                        buf.clear();
                        break;
                    }
                }
            }
        }
        messages
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketError {
    Incomplete,
    InvalidSom,
    InvalidSize,
    InvalidEom,
    ChecksumMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NoiseControlMode {
    #[default]
    Off = 0,
    Anc = 1,
    Ambient = 2,
    Adaptive = 3,
}

impl NoiseControlMode {
    pub fn from_byte(b: u8) -> Self {
        match b {
            1 => Self::Anc,
            2 => Self::Ambient,
            3 => Self::Adaptive,
            _ => Self::Off,
        }
    }

    pub fn label_pt(&self) -> &'static str {
        match self {
            Self::Off => "Desligado",
            Self::Anc => "Cancelamento de Ruído (ANC)",
            Self::Ambient => "Som Ambiente",
            Self::Adaptive => "Adaptativo",
        }
    }

    pub fn short_pt(&self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Anc => "ANC",
            Self::Ambient => "Ambiente",
            Self::Adaptive => "Adaptativo",
        }
    }
}

impl fmt::Display for NoiseControlMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label_pt())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PlacementState {
    #[default]
    Disconnected = 0,
    Wearing = 1,
    Idle = 2,
    InCase = 3,
    ClosedCase = 4,
    Charging = 100,
}

impl PlacementState {
    pub fn from_nibble(n: u8) -> Self {
        match n {
            1 => Self::Wearing,
            2 => Self::Idle,
            3 => Self::InCase,
            4 => Self::ClosedCase,
            _ => Self::Disconnected,
        }
    }

    pub fn label_pt(&self) -> &'static str {
        match self {
            Self::Disconnected => "Desconectado",
            Self::Wearing => "No ouvido",
            Self::Idle => "Fora do ouvido",
            Self::InCase => "No estojo",
            Self::ClosedCase => "Estojo fechado",
            Self::Charging => "Carregando",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EqualizerPreset {
    #[default]
    Off = 0,
    BassBoost = 1,
    Soft = 2,
    Dynamic = 3,
    Clear = 4,
    TrebleBoost = 5,
    Custom = 7,
}

impl EqualizerPreset {
    pub fn from_byte(b: u8) -> Self {
        match b {
            0 => Self::Off,
            1 => Self::BassBoost,
            2 => Self::Soft,
            3 => Self::Dynamic,
            4 => Self::Clear,
            5 => Self::TrebleBoost,
            _ => Self::Custom,
        }
    }

    pub fn label_pt(&self) -> &'static str {
        match self {
            Self::Off => "Desligado (Padrão)",
            Self::BassBoost => "Bass Boost (Graves)",
            Self::Soft => "Suave",
            Self::Dynamic => "Dinâmico",
            Self::Clear => "Claro",
            Self::TrebleBoost => "Treble Boost (Agudos)",
            Self::Custom => "Personalizado (9 Bandas)",
        }
    }

    pub fn all_presets() -> &'static [EqualizerPreset] {
        &[
            Self::Off,
            Self::BassBoost,
            Self::Soft,
            Self::Dynamic,
            Self::Clear,
            Self::TrebleBoost,
            Self::Custom,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Buds4ProState {
    pub connected: bool,
    pub device_name: String,
    pub mac_address: String,
    pub revision: u8,
    pub battery_left: u8,
    pub battery_right: u8,
    pub battery_case: u8,
    pub is_left_charging: bool,
    pub is_right_charging: bool,
    pub is_case_charging: bool,
    pub placement_left: PlacementState,
    pub placement_right: PlacementState,
    pub noise_control: NoiseControlMode,
    pub ambient_volume: u8,
    pub anc_level: u8,
    pub equalizer: EqualizerPreset,
    pub custom_eq_bands: [i8; 9],
    pub touchpad_locked: bool,
    pub single_tap_on: bool,
    pub double_tap_on: bool,
    pub triple_tap_on: bool,
    pub touch_hold_on: bool,
    pub double_tap_call_on: bool,
    pub touch_hold_call_on: bool,
    pub voice_detect_enabled: bool,
    pub voice_detect_timeout_sec: u8,
    pub anc_with_one_earbud: bool,
    pub seamless_connection: bool,
    pub sidetone_enabled: bool,
    pub extra_clear_call: bool,
    pub spatial_audio: bool,
    pub head_tracking: bool,
    pub adaptive_volume: bool,
    pub siren_detect: bool,
    pub lighting_control: u8,
    pub game_mode: bool,
    pub color_name: String,
    pub firmware_version: String,
    pub hardware_version: String,
    pub sku_left: String,
    pub sku_right: String,
    pub serial_left: String,
    pub serial_right: String,
    pub temp_left_c: f32,
    pub temp_right_c: f32,
    pub voltage_left_v: f32,
    pub voltage_right_v: f32,
    pub current_left_ma: f32,
    pub current_right_ma: f32,
    pub find_my_buds_active: bool,
    pub mute_left: bool,
    pub mute_right: bool,
}

impl Default for Buds4ProState {
    fn default() -> Self {
        Self {
            connected: false,
            device_name: "Galaxy Buds4 Pro".to_string(),
            mac_address: String::new(),
            revision: 0,
            battery_left: 0,
            battery_right: 0,
            battery_case: 0,
            is_left_charging: false,
            is_right_charging: false,
            is_case_charging: false,
            placement_left: PlacementState::Disconnected,
            placement_right: PlacementState::Disconnected,
            noise_control: NoiseControlMode::Off,
            ambient_volume: 2,
            anc_level: 1,
            equalizer: EqualizerPreset::Dynamic,
            custom_eq_bands: [0; 9],
            touchpad_locked: false,
            single_tap_on: true,
            double_tap_on: true,
            triple_tap_on: true,
            touch_hold_on: true,
            double_tap_call_on: true,
            touch_hold_call_on: true,
            voice_detect_enabled: false,
            voice_detect_timeout_sec: 10,
            anc_with_one_earbud: false,
            seamless_connection: true,
            sidetone_enabled: false,
            extra_clear_call: true,
            spatial_audio: false,
            head_tracking: false,
            adaptive_volume: false,
            siren_detect: false,
            lighting_control: 0,
            game_mode: false,
            color_name: "Desconhecida".to_string(),
            firmware_version: String::new(),
            hardware_version: String::new(),
            sku_left: String::new(),
            sku_right: String::new(),
            serial_left: String::new(),
            serial_right: String::new(),
            temp_left_c: 0.0,
            temp_right_c: 0.0,
            voltage_left_v: 0.0,
            voltage_right_v: 0.0,
            current_left_ma: 0.0,
            current_right_ma: 0.0,
            find_my_buds_active: false,
            mute_left: false,
            mute_right: false,
        }
    }
}

impl Buds4ProState {
    pub fn apply_message(&mut self, msg: &SppMessage) -> bool {
        let p = &msg.payload;
        match msg.id {
            // STATUS_UPDATED (96)
            96 if p.len() >= 7 => {
                self.revision = p[0];
                self.battery_left = p[1];
                self.battery_right = p[2];
                let mut pl_l = PlacementState::from_nibble((p[5] & 0xF0) >> 4);
                let mut pl_r = PlacementState::from_nibble(p[5] & 0x0F);
                self.battery_case = p[6];
                if p.len() >= 8 {
                    let cs = p[7];
                    self.is_left_charging = (cs & (1 << 4)) != 0;
                    self.is_right_charging = (cs & (1 << 2)) != 0;
                    self.is_case_charging = (cs & (1 << 0)) != 0;
                    if self.is_left_charging {
                        pl_l = PlacementState::Charging;
                    }
                    if self.is_right_charging {
                        pl_r = PlacementState::Charging;
                    }
                }
                self.placement_left = pl_l;
                self.placement_right = pl_r;
                true
            }
            // EXTENDED_STATUS_UPDATED (97)
            97 if p.len() >= 34 => {
                self.revision = p[0];
                self.battery_left = p[2];
                self.battery_right = p[3];
                let mut pl_l = PlacementState::from_nibble((p[6] & 0xF0) >> 4);
                let mut pl_r = PlacementState::from_nibble(p[6] & 0x0F);
                self.battery_case = p[7];
                self.game_mode = p[8] == 1;
                self.equalizer = EqualizerPreset::from_byte(p[9]);

                // AdvancedTouchLock on Buds2..Buds4Pro
                let touch_byte = p[10];
                self.touch_hold_on = (touch_byte & (1 << 0)) != 0;
                self.triple_tap_on = (touch_byte & (1 << 1)) != 0;
                self.double_tap_on = (touch_byte & (1 << 2)) != 0;
                self.single_tap_on = (touch_byte & (1 << 3)) != 0;
                self.double_tap_call_on = (touch_byte & (1 << 4)) != 0;
                self.touch_hold_call_on = (touch_byte & (1 << 5)) != 0;
                self.touchpad_locked = (touch_byte & (1 << 7)) == 0;

                self.noise_control = NoiseControlMode::from_byte(p[12]);

                let color_l = i16::from_le_bytes([p[14], p[15]]);
                let color_r = i16::from_le_bytes([p[16], p[17]]);
                let color_id = if color_r != 0 { color_r } else { color_l };
                self.color_name = match color_id {
                    359 => "Preto (Black)".to_string(),
                    360 => "Branco (White)".to_string(),
                    361 => "Apricot".to_string(),
                    340 => "Prata (Silver)".to_string(),
                    341 => "Branco (White)".to_string(),
                    _ => format!("ID #{color_id}"),
                };

                self.seamless_connection = p[19] == 0;
                self.ambient_volume = p[23];
                self.anc_level = p[24];
                self.voice_detect_enabled = p[26] == 1;
                self.voice_detect_timeout_sec = match p[27] {
                    0 => 5,
                    1 => 10,
                    2 => 15,
                    _ => 10,
                };
                self.anc_with_one_earbud = p[28] == 1;
                self.sidetone_enabled = p[33] == 1;

                // Reader from offset 34 onwards (Buds3 / Buds3Pro / Buds4Pro layout)
                if p.len() > 35 {
                    self.spatial_audio = p[35] != 0;
                }
                if p.len() > 41 {
                    self.head_tracking = p[41] != 0;
                }
                if p.len() > 42 {
                    let cs = p[42];
                    self.is_left_charging = (cs & (1 << 4)) != 0;
                    self.is_right_charging = (cs & (1 << 2)) != 0;
                    self.is_case_charging = (cs & (1 << 0)) != 0;
                    if self.is_left_charging {
                        pl_l = PlacementState::Charging;
                    }
                    if self.is_right_charging {
                        pl_r = PlacementState::Charging;
                    }
                }
                if p.len() > 43 {
                    self.extra_clear_call = p[43] != 0;
                }
                if p.len() > 49 {
                    self.adaptive_volume = p[49] != 0;
                }
                if p.len() > 52 {
                    self.lighting_control = p[50];
                    self.siren_detect = p[52] != 0;
                }

                self.placement_left = pl_l;
                self.placement_right = pl_r;
                true
            }
            // NOISE_CONTROLS_UPDATE (119)
            119 if !p.is_empty() => {
                self.noise_control = NoiseControlMode::from_byte(p[0]);
                true
            }
            // DEBUG_GET_ALL_DATA (38)
            38 if p.len() >= 72 => {
                let hw1 = (p[1] & 0xF0) >> 4;
                let hw2 = p[1] & 0x0F;
                self.hardware_version = format!("rev{:X}.{:X}", hw1, hw2);

                // On Buds 3 / Buds 4 series, software version is a 20-byte ASCII string at offset 2
                let sw_len = 20usize;
                if p.len() >= 2 + sw_len + 50 {
                    let sw_bytes: Vec<u8> = p[2..2 + sw_len]
                        .iter()
                        .copied()
                        .filter(|&b| b != 0 && b.is_ascii_graphic())
                        .collect();
                    if let Ok(s) = String::from_utf8(sw_bytes) {
                        if !s.is_empty() {
                            self.firmware_version = s;
                        }
                    }

                    let base_idx = sw_len;
                    let temp_l = i16::from_le_bytes([p[base_idx + 35], p[base_idx + 36]]);
                    let temp_r = i16::from_le_bytes([p[base_idx + 37], p[base_idx + 38]]);
                    self.temp_left_c = (temp_l as f32) * 0.1;
                    self.temp_right_c = (temp_r as f32) * 0.1;
                    // On Buds4 Pro, the primary connected bud reports thermistor in its slot while the peer slot is 0
                    if self.temp_left_c == 0.0 && self.temp_right_c > 0.0 {
                        self.temp_left_c = self.temp_right_c;
                    } else if self.temp_right_c == 0.0 && self.temp_left_c > 0.0 {
                        self.temp_right_c = self.temp_left_c;
                    }

                    let volt_l = i16::from_le_bytes([p[base_idx + 41], p[base_idx + 42]]);
                    let curr_l = i16::from_le_bytes([p[base_idx + 43], p[base_idx + 44]]);
                    let volt_r = i16::from_le_bytes([p[base_idx + 47], p[base_idx + 48]]);
                    let curr_r = i16::from_le_bytes([p[base_idx + 49], p[base_idx + 50]]);

                    self.voltage_left_v = (volt_l as f32) * 0.01;
                    self.current_left_ma = (curr_l as f32) * -0.1;
                    self.voltage_right_v = (volt_r as f32) * 0.01;
                    self.current_right_ma = (curr_r as f32) * -0.1;
                }
                true
            }
            // DEBUG_SKU (34)
            34 if p.len() >= 14 => {
                self.sku_left = String::from_utf8_lossy(&p[0..14])
                    .trim_matches(char::from(0))
                    .trim()
                    .to_string();
                if p.len() >= 28 {
                    self.sku_right = String::from_utf8_lossy(&p[14..28])
                        .trim_matches(char::from(0))
                        .trim()
                        .to_string();
                }
                true
            }
            // DEBUG_SERIAL_NUMBER (41)
            41 if p.len() >= 22 => {
                self.serial_left = String::from_utf8_lossy(&p[0..11])
                    .trim_matches(char::from(0))
                    .trim()
                    .to_string();
                self.serial_right = String::from_utf8_lossy(&p[11..22])
                    .trim_matches(char::from(0))
                    .trim()
                    .to_string();
                true
            }
            // UNIVERSAL_MSG_ID_ACKNOWLEDGEMENT (66)
            66 if p.len() >= 2 => {
                let acked_id = p[0];
                let val = p[1];
                match acked_id {
                    120 => self.noise_control = NoiseControlMode::from_byte(val),
                    131 => self.anc_level = val,
                    132 => self.ambient_volume = val,
                    134 => self.equalizer = EqualizerPreset::from_byte(val),
                    122 => self.voice_detect_enabled = val != 0,
                    123 => {
                        self.voice_detect_timeout_sec = match val {
                            0 => 5,
                            1 => 10,
                            _ => 15,
                        }
                    }
                    111 => self.anc_with_one_earbud = val != 0,
                    139 => self.sidetone_enabled = val != 0,
                    72 => self.extra_clear_call = val != 0,
                    175 => self.seamless_connection = val == 0,
                    133 | 135 => self.game_mode = val != 0,
                    _ => {}
                }
                true
            }
            161 => {
                self.find_my_buds_active = false;
                true
            }
            _ => false,
        }
    }
}

// --- Command Encoders ---

pub fn encode_manager_info() -> SppMessage {
    // [1, ClientDeviceTypes.Samsung (1), AndroidSdkVersion (34)]
    SppMessage::new_request(MsgId::ManagerInfo, vec![1, 1, 34])
}

pub fn encode_request_debug_all() -> SppMessage {
    SppMessage::new_request(MsgId::DebugGetAllData, vec![])
}

pub fn encode_request_sku() -> SppMessage {
    SppMessage::new_request(MsgId::DebugSku, vec![])
}

pub fn encode_request_serial() -> SppMessage {
    SppMessage::new_request(MsgId::DebugSerialNumber, vec![])
}

pub fn encode_noise_control(mode: NoiseControlMode) -> SppMessage {
    SppMessage::new_request(MsgId::NoiseControls, vec![mode as u8])
}

pub fn encode_ambient_volume(vol: u8) -> SppMessage {
    SppMessage::new_request(MsgId::AmbientVolume, vec![vol.min(4)])
}

pub fn encode_anc_level(level: u8) -> SppMessage {
    SppMessage::new_request(MsgId::NoiseReductionLevel, vec![level])
}

pub fn encode_equalizer(preset: EqualizerPreset) -> SppMessage {
    SppMessage::new_request(MsgId::Equalizer, vec![preset as u8])
}

pub fn encode_custom_equalizer(bands: &[i8; 9]) -> Vec<SppMessage> {
    let mut payload = Vec::with_capacity(10);
    payload.push(9u8);
    for &b in bands {
        payload.push(b.clamp(-10, 10) as u8);
    }
    vec![
        SppMessage::new_request(MsgId::CustomEqualizeSend, payload),
        SppMessage::new_request(MsgId::Equalizer, vec![EqualizerPreset::Custom as u8]),
    ]
}

pub fn encode_lock_touchpad(state: &Buds4ProState, lock_all: bool) -> SppMessage {
    let payload = vec![
        u8::from(!lock_all),
        u8::from(state.single_tap_on),
        u8::from(state.double_tap_on),
        u8::from(state.triple_tap_on),
        u8::from(state.touch_hold_on),
        u8::from(state.double_tap_call_on),
        u8::from(state.touch_hold_call_on),
    ];
    SppMessage::new_request(MsgId::LockTouchpad, payload)
}

pub fn encode_voice_detect(enabled: bool) -> SppMessage {
    SppMessage::new_request(MsgId::SetDetectConversations, vec![u8::from(enabled)])
}

pub fn encode_voice_detect_duration(seconds: u8) -> SppMessage {
    let code = match seconds {
        0..=7 => 0u8,
        8..=12 => 1u8,
        _ => 2u8,
    };
    SppMessage::new_request(MsgId::SetDetectConversationsDuration, vec![code])
}

pub fn encode_anc_one_earbud(enabled: bool) -> SppMessage {
    SppMessage::new_request(MsgId::SetAncWithOneEarbud, vec![u8::from(enabled)])
}

pub fn encode_seamless_connection(enabled: bool) -> SppMessage {
    // Inverted in Samsung protocol: 0 = enabled, 1 = disabled
    SppMessage::new_request(MsgId::SetSeamlessConnection, vec![u8::from(!enabled)])
}

pub fn encode_sidetone(enabled: bool) -> SppMessage {
    SppMessage::new_request(MsgId::SetSidetone, vec![u8::from(enabled)])
}

pub fn encode_extra_clear_call(enabled: bool) -> SppMessage {
    SppMessage::new_request(MsgId::ExtraClearSoundCall, vec![u8::from(enabled)])
}

pub fn encode_game_mode(enabled: bool) -> SppMessage {
    SppMessage::new_request(MsgId::AdjustSoundSync, vec![u8::from(enabled)])
}

pub fn encode_find_my_buds(start: bool) -> SppMessage {
    if start {
        SppMessage::new_request(MsgId::FindMyEarbudsStart, vec![])
    } else {
        SppMessage::new_request(MsgId::FindMyEarbudsStop, vec![])
    }
}

pub fn encode_mute_earbuds(left_muted: bool, right_muted: bool) -> SppMessage {
    SppMessage::new_request(
        MsgId::MuteEarbud,
        vec![u8::from(left_muted), u8::from(right_muted)],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let msg = encode_noise_control(NoiseControlMode::Anc);
        let bytes = msg.encode();
        assert_eq!(bytes[0], SOM);
        assert_eq!(*bytes.last().unwrap(), EOM);

        let (decoded, consumed) = SppMessage::decode_one(&bytes).expect("decode failed");
        assert_eq!(consumed, bytes.len());
        assert_eq!(decoded.id, MsgId::NoiseControls as u8);
        assert_eq!(decoded.payload, vec![1]);
    }
}
