#![windows_subsystem = "windows"]

use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, SampleRate, StreamConfig};
use rodio::{Decoder, Source};
use std::io::Cursor;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use tray_item::{IconSource, TrayItem};

const BUFFER_SIZE: u32 = 8192; // 8192, 16384

fn main() -> Result<()> {
    let sound_data = include_bytes!("sound.mp3");
    let cursor = Cursor::new(sound_data);

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("no output device available");

    let config = StreamConfig {
        channels: 1,
        sample_rate: SampleRate(48000),
        buffer_size: BufferSize::Fixed(BUFFER_SIZE),
    };

    println!("Using config: {:?}", config);

    let source = Decoder::new(cursor)?;
    let channels = source.channels();

    let samples: Vec<f32> = source.convert_samples().collect();
    let samples = Arc::new(Mutex::new(samples));
    let sample_index = Arc::new(Mutex::new(0));

    let sound_on_off = Arc::new(Mutex::new(true));
    let volume_level = Arc::new(Mutex::new(0.25f32));

    let samples_clone = Arc::clone(&samples);
    let sample_index_clone = Arc::clone(&sample_index);
    let sound_on_off_clone = Arc::clone(&sound_on_off);
    let volume_level_clone = Arc::clone(&volume_level);

    let stream = device.build_output_stream(
        &config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let mut index = sample_index_clone.lock().unwrap();
            let samples = samples_clone.lock().unwrap();
            let is_on = *sound_on_off_clone.lock().unwrap();
            let volume = *volume_level_clone.lock().unwrap();

            for frame in data.chunks_mut(channels as usize) {
                for (i, sample) in frame.iter_mut().enumerate() {
                    *sample = if is_on {
                        samples[*index * channels as usize + i] * volume
                    } else {
                        0.0
                    };
                }
                *index += 1;
                if *index >= samples.len() / channels as usize {
                    *index = 0;
                }
            }
        },
        |err| eprintln!("an error occurred on stream: {}", err),
        None,
    )?;

    stream.play()?;

    // Create a tray icon
    let mut tray = TrayItem::new("sound-loop", IconSource::Resource("icon_on")).unwrap();

    // Create a channel for communication with the tray menu
    let (tx, rx) = mpsc::channel();

    // --- Volume Menu Items ---
    let tx_clone = tx.clone();
    tray.add_menu_item("10%", move || {
        tx_clone.send(0.10).unwrap();
    })?;

    let tx_clone = tx.clone();
    tray.add_menu_item("25%  <", move || {
        tx_clone.send(0.25).unwrap();
    })?;

    let tx_clone = tx.clone();
    tray.add_menu_item("100%", move || {
        tx_clone.send(1.00).unwrap();
    })?;

    // --- On/Off Menu Item ---
    let tx_clone = tx.clone();
    tray.add_menu_item("On/Off", move || {
        tx_clone.send(-1.0).unwrap();
    })?;

    tray.add_menu_item("Exit", move || {
        std::process::exit(0);
    })?;

    // Main loop to handle tray menu events
    loop {
        if let Ok(new_volume) = rx.recv() {
            if new_volume == -1.0 {
                // On/Off toggle
                let mut is_on = sound_on_off.lock().unwrap();
                *is_on = !*is_on;

                if *is_on {
                    tray.set_icon(IconSource::Resource("icon_on"))?;
                } else {
                    tray.set_icon(IconSource::Resource("icon_off"))?;
                }
            } else {
                // Volume change
                let mut vol = volume_level.lock().unwrap();
                *vol = new_volume;

                // Update volume menu labels
                tray.inner_mut()
                    .set_menu_item_label(if new_volume == 0.10 { "10%  <" } else { "10%" }, 0)?;
                tray.inner_mut()
                    .set_menu_item_label(if new_volume == 0.25 { "25%  <" } else { "25%" }, 1)?;
                tray.inner_mut()
                    .set_menu_item_label(if new_volume == 1.00 { "100% <" } else { "100%" }, 2)?;
            }
        } else {
            thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
