use crate::ipc::{IpcResponse, UserCommand, IPC_SOCKET_PATH};
use crate::protocol::*;
use anyhow::{anyhow, Context, Result};
use bluer::rfcomm::{Profile, Role};
use bluer::{Adapter, Device, Uuid};
use futures::StreamExt;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{mpsc, RwLock};

pub struct BudsManager {
    pub state: Arc<RwLock<Buds4ProState>>,
    pub cmd_tx: mpsc::UnboundedSender<UserCommand>,
    pub show_gui_flag: Arc<AtomicBool>,
    pub quit_flag: Arc<AtomicBool>,
}

pub async fn find_buds4_pro(adapter: &Adapter) -> Result<Device> {
    let uuid = Uuid::from_str(BUDS4_PRO_SPP_UUID)?;
    let addrs = adapter.device_addresses().await?;

    let mut fallback_candidate: Option<Device> = None;

    for addr in addrs {
        let dev = adapter.device(addr)?;
        let is_paired = dev.is_paired().await.unwrap_or(false);
        if !is_paired {
            continue;
        }
        let name = dev.alias().await.unwrap_or_default();
        let uuids = dev.uuids().await.unwrap_or_default().unwrap_or_default();
        let is_connected = dev.is_connected().await.unwrap_or(false);

        let matches_buds = uuids.contains(&uuid)
            || name.to_lowercase().contains("buds4")
            || name.to_lowercase().contains("buds");

        if matches_buds {
            if is_connected {
                return Ok(dev);
            }
            fallback_candidate = Some(dev);
        }
    }

    fallback_candidate.ok_or_else(|| {
        anyhow!("Nenhum Galaxy Buds 4 Pro pareado foi encontrado no adaptador Bluetooth.")
    })
}

pub async fn apply_command_to_messages(
    cmd: &UserCommand,
    state: &Arc<RwLock<Buds4ProState>>,
    show_gui_flag: &Arc<AtomicBool>,
    quit_flag: &Arc<AtomicBool>,
) -> Vec<SppMessage> {
    let mut s = state.write().await;
    match *cmd {
        UserCommand::GetStatus => vec![],
        UserCommand::RefreshDebug => vec![
            encode_request_debug_all(),
            encode_request_sku(),
            encode_request_serial(),
        ],
        UserCommand::SetNoiseControl(mode) => {
            s.noise_control = mode;
            vec![encode_noise_control(mode)]
        }
        UserCommand::CycleNoiseControl => {
            let next = match s.noise_control {
                NoiseControlMode::Off => NoiseControlMode::Anc,
                NoiseControlMode::Anc => NoiseControlMode::Ambient,
                NoiseControlMode::Ambient => NoiseControlMode::Adaptive,
                NoiseControlMode::Adaptive => NoiseControlMode::Off,
            };
            s.noise_control = next;
            vec![encode_noise_control(next)]
        }
        UserCommand::SetAmbientVolume(vol) => {
            let v = vol.min(4);
            s.ambient_volume = v;
            vec![encode_ambient_volume(v)]
        }
        UserCommand::SetAncLevel(level) => {
            s.anc_level = level;
            vec![encode_anc_level(level)]
        }
        UserCommand::SetEqualizer(preset) => {
            s.equalizer = preset;
            if preset == EqualizerPreset::Custom {
                encode_custom_equalizer(&s.custom_eq_bands)
            } else {
                vec![encode_equalizer(preset)]
            }
        }
        UserCommand::SetCustomEqualizer(bands) => {
            s.custom_eq_bands = bands;
            s.equalizer = EqualizerPreset::Custom;
            encode_custom_equalizer(&bands)
        }
        UserCommand::SetTouchpadLock(lock_all) => {
            s.touchpad_locked = lock_all;
            vec![encode_lock_touchpad(&s, lock_all)]
        }
        UserCommand::SetTouchGestures {
            single,
            double,
            triple,
            hold,
            double_call,
            hold_call,
        } => {
            s.single_tap_on = single;
            s.double_tap_on = double;
            s.triple_tap_on = triple;
            s.touch_hold_on = hold;
            s.double_tap_call_on = double_call;
            s.touch_hold_call_on = hold_call;
            vec![encode_lock_touchpad(&s, s.touchpad_locked)]
        }
        UserCommand::SetVoiceDetect(enabled) => {
            s.voice_detect_enabled = enabled;
            vec![encode_voice_detect(enabled)]
        }
        UserCommand::SetVoiceDetectDuration(sec) => {
            s.voice_detect_timeout_sec = sec;
            vec![encode_voice_detect_duration(sec)]
        }
        UserCommand::SetAncWithOneEarbud(enabled) => {
            s.anc_with_one_earbud = enabled;
            vec![encode_anc_one_earbud(enabled)]
        }
        UserCommand::SetSeamlessConnection(enabled) => {
            s.seamless_connection = enabled;
            vec![encode_seamless_connection(enabled)]
        }
        UserCommand::SetSidetone(enabled) => {
            s.sidetone_enabled = enabled;
            vec![encode_sidetone(enabled)]
        }
        UserCommand::SetExtraClearCall(enabled) => {
            s.extra_clear_call = enabled;
            vec![encode_extra_clear_call(enabled)]
        }
        UserCommand::SetGameMode(enabled) => {
            s.game_mode = enabled;
            vec![encode_game_mode(enabled)]
        }
        UserCommand::SetFindMyBuds(start) => {
            s.find_my_buds_active = start;
            if !start {
                s.mute_left = false;
                s.mute_right = false;
            }
            vec![encode_find_my_buds(start)]
        }
        UserCommand::SetMuteEarbuds { left, right } => {
            s.mute_left = left;
            s.mute_right = right;
            vec![encode_mute_earbuds(left, right)]
        }
        UserCommand::ShowGui => {
            show_gui_flag.store(true, Ordering::SeqCst);
            vec![]
        }
        UserCommand::QuitDaemon => {
            quit_flag.store(true, Ordering::SeqCst);
            vec![]
        }
    }
}

pub async fn start_background_manager() -> Result<BudsManager> {
    let state = Arc::new(RwLock::new(Buds4ProState::default()));
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<UserCommand>();
    let show_gui_flag = Arc::new(AtomicBool::new(false));
    let quit_flag = Arc::new(AtomicBool::new(false));

    // Spawn IPC server so CLI commands can talk to the active daemon/GUI
    let ipc_state = state.clone();
    let ipc_tx = cmd_tx.clone();
    tokio::spawn(async move {
        let _ = std::fs::remove_file(IPC_SOCKET_PATH);
        if let Ok(listener) = UnixListener::bind(IPC_SOCKET_PATH) {
            loop {
                if let Ok((stream, _)) = listener.accept().await {
                    let st = ipc_state.clone();
                    let tx = ipc_tx.clone();
                    tokio::spawn(async move {
                        let mut reader = BufReader::new(stream);
                        let mut line = String::new();
                        if reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                            if let Ok(cmd) = serde_json::from_str::<UserCommand>(line.trim()) {
                                let is_get = matches!(cmd, UserCommand::GetStatus);
                                if !is_get {
                                    let _ = tx.send(cmd);
                                    tokio::time::sleep(Duration::from_millis(80)).await;
                                }
                                let snapshot = st.read().await.clone();
                                let resp = IpcResponse {
                                    ok: true,
                                    message: "OK".to_string(),
                                    state: snapshot,
                                };
                                if let Ok(json) = serde_json::to_string(&resp) {
                                    let mut inner = reader.into_inner();
                                    let _ = inner.write_all(format!("{json}\n").as_bytes()).await;
                                }
                            }
                        }
                    });
                }
            }
        }
    });

    // Spawn Bluetooth RFCOMM connection loop
    let bt_state = state.clone();
    let bt_show_gui = show_gui_flag.clone();
    let bt_quit = quit_flag.clone();
    tokio::spawn(async move {
        let session = match bluer::Session::new().await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Erro ao iniciar sessão BlueZ: {e}");
                return;
            }
        };
        let adapter = match session.default_adapter().await {
            Ok(a) => a,
            Err(e) => {
                eprintln!("Erro ao obter adaptador Bluetooth: {e}");
                return;
            }
        };

        let uuid = Uuid::from_str(BUDS4_PRO_SPP_UUID).expect("valid UUID");
        let profile = Profile {
            uuid,
            name: Some("Buds4ProRustManager".to_string()),
            role: Some(Role::Client),
            require_authentication: Some(false),
            require_authorization: Some(false),
            auto_connect: Some(true),
            ..Default::default()
        };

        let mut profile_handle = match session.register_profile(profile).await {
            Ok(h) => h,
            Err(e) => {
                eprintln!("Aviso ao registrar perfil RFCOMM: {e}");
                return;
            }
        };

        loop {
            if bt_quit.load(Ordering::SeqCst) {
                break;
            }

            let dev = match find_buds4_pro(&adapter).await {
                Ok(d) => d,
                Err(_) => {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    continue;
                }
            };

            let dev_name = dev
                .alias()
                .await
                .unwrap_or_else(|_| "Galaxy Buds4 Pro".to_string());
            let dev_mac = dev.address().to_string();

            {
                let mut s = bt_state.write().await;
                s.device_name = dev_name;
                s.mac_address = dev_mac;
            }

            if !dev.is_connected().await.unwrap_or(false) {
                let _ = dev.connect().await;
            }

            // Trigger profile connection in background
            let dev_clone = dev.clone();
            tokio::spawn(async move {
                let _ = dev_clone.connect_profile(&uuid).await;
            });

            // Wait for ConnectRequest from BlueZ Profile1
            let req = match tokio::time::timeout(
                Duration::from_secs(6),
                profile_handle.next(),
            )
            .await
            {
                Ok(Some(r)) => r,
                _ => {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    continue;
                }
            };

            let mut stream = match req.accept() {
                Ok(s) => s,
                Err(_) => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            {
                let mut s = bt_state.write().await;
                s.connected = true;
            }

            // Send initial handshake & info requests
            for init_msg in [
                encode_manager_info(),
                encode_request_sku(),
                encode_request_serial(),
                encode_request_debug_all(),
            ] {
                let _ = stream.write_all(&init_msg.encode()).await;
            }

            let mut read_buf = [0u8; 1024];
            let mut packet_buf = Vec::with_capacity(2048);
            let mut debug_ticker = tokio::time::interval(Duration::from_secs(12));

            loop {
                tokio::select! {
                    read_res = stream.read(&mut read_buf) => {
                        match read_res {
                            Ok(0) | Err(_) => {
                                let mut s = bt_state.write().await;
                                s.connected = false;
                                break;
                            }
                            Ok(n) => {
                                packet_buf.extend_from_slice(&read_buf[..n]);
                                let msgs = SppMessage::drain_buffer(&mut packet_buf);
                                let mut need_manager_reply = false;
                                {
                                    let mut s = bt_state.write().await;
                                    for m in msgs {
                                        if m.id == MsgId::ExtendedStatusUpdated as u8 {
                                            need_manager_reply = true;
                                        }
                                        s.apply_message(&m);
                                    }
                                }
                                if need_manager_reply {
                                    for req_msg in [
                                        encode_manager_info(),
                                        encode_request_sku(),
                                        encode_request_serial(),
                                        encode_request_debug_all(),
                                    ] {
                                        let _ = stream.write_all(&req_msg.encode()).await;
                                    }
                                }
                            }
                        }
                    }
                    Some(cmd) = cmd_rx.recv() => {
                        let out_msgs = apply_command_to_messages(&cmd, &bt_state, &bt_show_gui, &bt_quit).await;
                        for m in out_msgs {
                            let _ = stream.write_all(&m.encode()).await;
                        }
                        if bt_quit.load(Ordering::SeqCst) {
                            break;
                        }
                    }
                    _ = debug_ticker.tick() => {
                        let _ = stream.write_all(&encode_request_debug_all().encode()).await;
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });

    Ok(BudsManager {
        state,
        cmd_tx,
        show_gui_flag,
        quit_flag,
    })
}

/// Sends a command to the running daemon via Unix socket, or executes a one-shot Bluetooth session if no daemon is running.
pub async fn send_or_execute_oneshot(cmd: UserCommand) -> Result<Buds4ProState> {
    if let Ok(mut stream) = UnixStream::connect(IPC_SOCKET_PATH).await {
        let payload = format!("{}\n", serde_json::to_string(&cmd)?);
        stream.write_all(payload.as_bytes()).await?;
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        let resp: IpcResponse =
            serde_json::from_str(line.trim()).context("Resposta IPC inválida")?;
        return Ok(resp.state);
    }

    // No daemon running: run a quick direct Bluetooth connection
    let mgr = start_background_manager().await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        {
            let s = mgr.state.read().await;
            if s.connected && s.revision > 0 && !s.hardware_version.is_empty() {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(80)).await;
    }

    if !matches!(cmd, UserCommand::GetStatus) {
        let _ = mgr.cmd_tx.send(cmd);
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    let snapshot = mgr.state.read().await.clone();
    let _ = mgr.cmd_tx.send(UserCommand::QuitDaemon);
    tokio::time::sleep(Duration::from_millis(80)).await;
    let _ = std::fs::remove_file(IPC_SOCKET_PATH);
    Ok(snapshot)
}
