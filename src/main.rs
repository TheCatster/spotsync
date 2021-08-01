#![feature(string_remove_matches)]
use aspotify::{authorization_url, Client, ClientCredentials, PlaylistSimplified, Scope};

use fern::{log_file, Dispatch};
use log::{info, LevelFilter};

use ron::{
    de::from_reader,
    ser::{to_string_pretty, PrettyConfig},
};
use serde::{Deserialize, Serialize};

use std::{
    env,
    fs::{self, write, File},
    io::{self, Write},
};

mod tests;

#[derive(Debug, Serialize, Deserialize)]
struct PlaylistConfig {
    title: String,
    songs: Vec<Song>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Song {
    title: String,
    artists: Vec<String>,
    album: String,
}

#[tokio::main]
async fn main() {
    Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(LevelFilter::Error)
        .chain(io::stdout())
        .chain(log_file("spotsync.log").expect("No permission to write to the current directory."))
        .apply()
        .expect("Failed to dispatch Fern logger!");

    let client = authenticate_spotify().await;
    info!("Connecting to Spotify API...");

    // Fetch remote playlists
    let spotify_playlists = retrieve_spotify_users_playlists(&client, 10).await;

    // Main logic
    for playlist in &spotify_playlists {
        local_playlist_make_if_not_exists(playlist);
        let _local_songs = local_playlist_read_songs(&client, playlist).await;
        let _spotify_songs = spotify_playlist_read_songs(&client, playlist).await;
        // let needed_songs: Vec<String> = compare_playlists();
        // download_songs_and_update_ron();
    }
}

async fn authenticate_spotify() -> Client {
    dotenv::dotenv().expect("Could not read .env file!");

    match std::fs::read_to_string(".refresh_token") {
        Ok(_) => {
            info!(".refresh_token present, refreshing client.");
            Client::with_refresh(
                ClientCredentials::from_env().expect("Cannot read env vars for SpotSync!"),
                std::fs::read_to_string(".refresh_token").expect("Cannot read refresh token file!"),
            )
        }
        Err(_) => {
            info!(".refresh_token not present, creating one with proper scope.");
            let client = Client::new(
                ClientCredentials::from_env().expect("Cannot read env vars for SpotSync!"),
            );

            let (url, state) = authorization_url(
                &client.credentials.id,
                [
                    Scope::UgcImageUpload,
                    Scope::UserReadPlaybackState,
                    Scope::UserModifyPlaybackState,
                    Scope::UserReadCurrentlyPlaying,
                    Scope::Streaming,
                    Scope::AppRemoteControl,
                    Scope::UserReadEmail,
                    Scope::UserReadPrivate,
                    Scope::PlaylistReadCollaborative,
                    Scope::PlaylistModifyPublic,
                    Scope::PlaylistReadPrivate,
                    Scope::PlaylistModifyPrivate,
                    Scope::UserLibraryModify,
                    Scope::UserLibraryRead,
                    Scope::UserTopRead,
                    Scope::UserReadRecentlyPlayed,
                    Scope::UserFollowRead,
                    Scope::UserFollowModify,
                ]
                .iter()
                .copied(),
                false,
                "http://localhost:8888/callback",
            );

            println!("Go to this website: {}", url);

            print!("Enter the URL that you were redirected to: ");
            io::stdout().flush().unwrap();
            let mut redirect = String::new();
            io::stdin().read_line(&mut redirect).unwrap();

            client.redirected(&redirect, &state).await.unwrap();

            write(
                ".refresh_token",
                client
                    .refresh_token()
                    .await
                    .expect("Could not obtain refresh token from Spotify!"),
            )
            .expect("Unable to write to refresh token file, possibly no permission?");

            client
        }
    }
}

async fn spotify_playlist_read_songs(client: &Client, playlist: &PlaylistSimplified) -> Vec<Song> {
    // Why is this so ugly!? There has to be a better way.
    client
        .playlists()
        .get_playlist(&playlist.id, None)
        .await
        .expect("Unable to retrieve playlist!")
        .data
        .tracks
        .items
        .iter()
        .map(|playlist_item| {
            let song_title = match playlist_item.item.as_ref().expect("No such playlist item!") {
                aspotify::PlaylistItemType::Track(track) => &track.name,
                _ => "Track not found",
            };

            let song_artists = match playlist_item.item.as_ref().expect("No such playlist item!") {
                aspotify::PlaylistItemType::Track(track) => track
                    .artists
                    .iter()
                    .map(|x| x.name.to_string())
                    .collect::<Vec<String>>(),
                _ => vec!["Artists not found".to_string()],
            };

            let song_album = match playlist_item.item.as_ref().expect("No such playlist item!") {
                aspotify::PlaylistItemType::Track(track) => &track.album.name,
                _ => "Track not found",
            };

            Song {
                title: song_title.to_string(),
                artists: song_artists,
                album: song_album.to_string(),
            }
        })
        .collect::<Vec<Song>>()
}

async fn retrieve_spotify_users_playlists(
    client: &Client,
    limit: usize,
) -> Vec<PlaylistSimplified> {
    client
        .playlists()
        .current_users_playlists(limit, 0)
        .await
        .expect("Could not retrieve the current user's playlists!")
        .data
        .items
}

fn local_playlist_make_if_not_exists(playlist: &PlaylistSimplified) {
    // Create directory if it doesn't exist
    let mut path = env::current_dir().expect("Could not read current directory!");
    path.push(format!("./music/{}", &playlist.name));
    let metadata = fs::metadata(path);
    if metadata.is_err() {
        let mut path = env::current_dir().expect("Could not read current directory!");
        let playlist_dir = format!("./music/{}", &playlist.name);
        path.push(playlist_dir);
        std::fs::create_dir_all(path).unwrap();
    };

    // Create playlists' data directory if not available
    let mut path = env::current_dir().expect("Could not read current directory!");
    path.push("data/playlists/");
    let metadata = fs::metadata(path);
    if metadata.is_err() {
        let mut path = env::current_dir().expect("Could not read current directory!");
        path.push("data/playlists/");
        std::fs::create_dir_all(path).unwrap();
    };

    // Create RON if it doesn't exist
    let mut path = env::current_dir().expect("Could not read current directory!");
    path.push(format!("data/playlists/\"{}\".ron", &playlist.name));
    let metadata = fs::metadata(path);
    if metadata.is_err() {
        let mut path = env::current_dir().expect("Could not read current directory!");
        let playlist_ron = format!("data/playlists/\"{}\".ron", &playlist.name);
        path.push(playlist_ron);
        File::create(path).unwrap();
    };
}

async fn local_playlist_read_songs(client: &Client, playlist: &PlaylistSimplified) -> Vec<Song> {
    let input_path = format!("data/playlists/\"{}\".ron", &playlist.name);
    let f = File::open(&input_path).expect("Failed opening file");
    let playlist_ron: Result<PlaylistConfig, ron::Error> = from_reader(f);
    let playlist_struct: Vec<Song> = match playlist_ron {
        Ok(x) => x.songs,
        Err(_) => create_playlist_ron(client, playlist).await.songs,
    };

    playlist_struct
}

async fn create_playlist_ron(client: &Client, playlist: &PlaylistSimplified) -> PlaylistConfig {
    let songs: Vec<Song> = spotify_playlist_read_songs(client, playlist).await;
    let playlist_struct = PlaylistConfig {
        title: playlist.name.to_string(),
        songs,
    };

    let pretty = PrettyConfig::new()
        .with_depth_limit(4)
        .with_separate_tuple_members(true)
        .with_enumerate_arrays(true);
    let playlist_ron = to_string_pretty(&playlist_struct, pretty).expect("Serialization failed");

    let playlist_file = format!("data/playlists/\"{}\".ron", &playlist.name);
    write(playlist_file, playlist_ron).unwrap();

    playlist_struct
}
