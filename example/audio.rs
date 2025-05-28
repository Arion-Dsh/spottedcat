use std::thread;
use std::time::Duration;
use spottedcat::PlaybackCommand;
use spottedcat::Player;
use spottedcat::Track;

fn main() {
    let player = Player::new().unwrap();
    let track = Track::from_path("1.mp3").unwrap();
    let track2 = Track::from_path("2.mp3").unwrap();
    println!("Track loaded: {}", track.id);
    let track_id = player.add_track(track).unwrap();
    let track_id2 = player.add_track(track2).unwrap();

    player.send_command(track_id, PlaybackCommand::Play).unwrap();
    player.send_command(track_id2, PlaybackCommand::Play).unwrap();
    player.start_stream().unwrap();
    // Play for 10 seconds
    thread::sleep(Duration::from_secs(200));
    player.send_command(track_id, PlaybackCommand::Stop).unwrap();
    player.send_command(track_id2, PlaybackCommand::Stop).unwrap();
    
}