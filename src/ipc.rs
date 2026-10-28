use crate::protocol::{Buds4ProState, EqualizerPreset, NoiseControlMode};
use serde::{Deserialize, Serialize};

pub const IPC_SOCKET_PATH: &str = "/tmp/buds4pro-manager.sock";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserCommand {
    GetStatus,
    RefreshDebug,
    SetNoiseControl(NoiseControlMode),
    CycleNoiseControl,
    SetAmbientVolume(u8),
    SetAncLevel(u8),
    SetEqualizer(EqualizerPreset),
    SetCustomEqualizer([i8; 9]),
    SetTouchpadLock(bool),
    SetTouchGestures {
        single: bool,
        double: bool,
        triple: bool,
        hold: bool,
        double_call: bool,
        hold_call: bool,
    },
    SetVoiceDetect(bool),
    SetVoiceDetectDuration(u8),
    SetAncWithOneEarbud(bool),
    SetSeamlessConnection(bool),
    SetSidetone(bool),
    SetExtraClearCall(bool),
    SetGameMode(bool),
    SetFindMyBuds(bool),
    SetMuteEarbuds { left: bool, right: bool },
    ShowGui,
    QuitDaemon,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
    pub ok: bool,
    pub message: String,
    pub state: Buds4ProState,
}
