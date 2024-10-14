#![windows_subsystem = "windows"] // Add this line at the beginning

use std::io::Cursor;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use rodio::{Decoder, OutputStream, Sink, Source};
use tray_item::{IconSource, TrayItem};

fn main() {
    let sound_data = include_bytes!("sound.mp3");
    let cursor = Cursor::new(sound_data);

    let sound_on_off = Arc::new(Mutex::new(true));
    let volume_level = Arc::new(Mutex::new(0.35)); // Default volume: 10%

    // Initialize audio output and decoder
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();
    let source = Decoder::new(cursor).unwrap().repeat_infinite();

    // Play the sound initially
    sink.set_volume(*volume_level.lock().unwrap());
    sink.append(source);
    sink.play();

    // Create a tray icon
    let mut tray = TrayItem::new("sound-loop", IconSource::Resource("icon_on")).unwrap();

    // Create a channel for communication with the tray menu
    let (tx, rx) = mpsc::channel();

    // --- Volume Menu Items ---
    let tx_clone = tx.clone();
    tray.add_menu_item("10%", move || {
        tx_clone.send(0.10).unwrap();
    })
    .unwrap();

    let tx_clone = tx.clone();
    tray.add_menu_item("35%  <", move || {
        tx_clone.send(0.35).unwrap();
    })
    .unwrap();

    let tx_clone = tx.clone();
    tray.add_menu_item("100%", move || {
        tx_clone.send(1.00).unwrap();
    })
    .unwrap();

    tray.inner_mut().add_separator().unwrap();

    // --- On/Off Menu Item ---
    let tx_clone = tx.clone();
    tray.add_menu_item("On/Off", move || {
        tx_clone.send(-1.0).unwrap();
    })
    .unwrap();

    tray.add_menu_item("Exit", move || {
        std::process::exit(0);
    })
    .unwrap();

    // Main loop to handle tray menu events
    loop {
        if let Ok(new_volume) = rx.recv() {
            if new_volume == -1.0 {
                // On/Off toggle
                let mut is_on = sound_on_off.lock().unwrap();
                *is_on = !*is_on;

                if *is_on {
                    sink.play();
                    tray.set_icon(IconSource::Resource("icon_on")).unwrap();
                } else {
                    sink.pause();
                    tray.set_icon(IconSource::Resource("icon_off")).unwrap();
                }
            } else {
                // Volume change
                let mut vol = volume_level.lock().unwrap();
                *vol = new_volume;
                sink.set_volume(*vol);

                // Update volume menu labels
                tray.inner_mut()
                    .set_menu_item_label(if new_volume == 0.10 { "10%  <" } else { "10%" }, 0)
                    .unwrap();
                tray.inner_mut()
                    .set_menu_item_label(if new_volume == 0.35 { "35%  <" } else { "35%" }, 1)
                    .unwrap();
                tray.inner_mut()
                    .set_menu_item_label(if new_volume == 1.00 { "100% <" } else { "100%" }, 2)
                    .unwrap();
            }
        } else {
            thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
