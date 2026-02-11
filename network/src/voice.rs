use anyhow::Result;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

pub struct VoiceManager {
    socket: Arc<UdpSocket>,
    is_recording: Arc<AtomicBool>,
    target_addr: Arc<Mutex<Option<SocketAddr>>>,
    // In a real app, we'd store the streams here to keep them alive,
    // but cpal streams rely on `std::marker::Send` which isn't always trivial.
    // For this prototype, we'll spawn a blocking thread for the audio loop.
}

impl VoiceManager {
    pub async fn new(bind_addr: &str) -> Result<Self> {
        let socket = UdpSocket::bind(bind_addr).await?;
        Ok(Self {
            socket: Arc::new(socket),
            is_recording: Arc::new(AtomicBool::new(false)),
            target_addr: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn set_target(&self, addr: SocketAddr) {
        let mut target = self.target_addr.lock().await;
        *target = Some(addr);
    }

    pub fn start_audio_loop(&self) -> Result<()> {
        if self.is_recording.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.is_recording.store(true, Ordering::SeqCst);
        let socket = self.socket.clone();
        let is_running = self.is_recording.clone();
        let target_addr_mutex = self.target_addr.clone();

        // Spawn a dedicated thread for audio input/output to avoid blocking async runtime
        std::thread::spawn(move || {
            let host = cpal::default_host();

            // Setup Input
            let input_device = match host.default_input_device() {
                Some(d) => d,
                None => {
                    eprintln!("No input device available");
                    return;
                }
            };

            let config: cpal::StreamConfig = input_device.default_input_config().unwrap().into();

            // Channel to bridge sync audio callback to async network sender
            // Use Unbounded channel to allow sending from sync code without blocking
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

            // Input Stream
            let input_stream = input_device
                .build_input_stream(
                    &config,
                    move |data: &[f32], _: &_| {
                        // Simple f32 to u8 (byte dump)
                        let mut bytes = Vec::with_capacity(data.len() * 4);
                        for sample in data {
                            bytes.extend_from_slice(&sample.to_ne_bytes());
                        }
                        let _ = tx.send(bytes);
                    },
                    |err| eprintln!("Input stream error: {}", err),
                    None,
                )
                .unwrap();

            input_stream.play().unwrap();

            // Setup Output
            let output_device = match host.default_output_device() {
                Some(d) => d,
                None => {
                    eprintln!("No output device available");
                    return;
                }
            };
            let output_config: cpal::StreamConfig =
                output_device.default_output_config().unwrap().into();

            // Channel for received audio to be played
            let (play_tx, play_rx) = std::sync::mpsc::channel::<Vec<f32>>();

            let output_stream = output_device
                .build_output_stream(
                    &output_config,
                    move |data: &mut [f32], _: &_| {
                        if let Ok(incoming) = play_rx.try_recv() {
                            let len = std::cmp::min(data.len(), incoming.len());
                            data[..len].copy_from_slice(&incoming[..len]);
                            if len < data.len() {
                                for sample in &mut data[len..] {
                                    *sample = 0.0;
                                }
                            }
                        } else {
                            for sample in data.iter_mut() {
                                *sample = 0.0;
                            }
                        }
                    },
                    |err| eprintln!("Output stream error: {}", err),
                    None,
                )
                .unwrap();

            output_stream.play().unwrap();

            // Network Sender/Receiver Loop
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            let socket_recv = socket.clone();
            rt.block_on(async {
                let mut buf = [0u8; 4096];

                loop {
                    if !is_running.load(Ordering::SeqCst) {
                        break;
                    }

                    tokio::select! {
                        // SEND: Input audio -> UDP
                        Some(data) = rx.recv() => {
                             let target = target_addr_mutex.lock().await;
                             if let Some(addr) = *target {
                                 let _ = socket.send_to(&data, addr).await;
                             }
                        }

                        // RECEIVE: UDP -> Output Audio
                        res = socket_recv.recv_from(&mut buf) => {
                            match res {
                                Ok((len, _addr)) => {
                                    let mut samples = Vec::with_capacity(len / 4);
                                    for chunk in buf[..len].chunks_exact(4) {
                                        let val = f32::from_ne_bytes(chunk.try_into().unwrap());
                                        samples.push(val);
                                    }
                                    let _ = play_tx.send(samples);
                                }
                                Err(_) => {
                                    // Ignore errors to keep loop alive
                                }
                            }
                        }
                    }
                }
            });
        });
        Ok(())
    }

    pub fn stop(&self) {
        self.is_recording.store(false, Ordering::SeqCst);
    }

    pub fn get_input_devices() -> Vec<String> {
        let host = cpal::default_host();
        match host.input_devices() {
            Ok(devices) => devices.filter_map(|d| d.name().ok()).collect(),
            Err(_) => vec!["Default Input".to_string()],
        }
    }

    pub fn get_output_devices() -> Vec<String> {
        let host = cpal::default_host();
        match host.output_devices() {
            Ok(devices) => devices.filter_map(|d| d.name().ok()).collect(),
            Err(_) => vec!["Default Output".to_string()],
        }
    }
}
