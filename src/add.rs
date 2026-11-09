use crate::{
    AddArgs, AddResource, BioMusic,
    decl::{
        Resource,
        artist::{Artist, ArtistDecl},
        character::CharacterDecl,
        game::GameDecl,
        song::{SongDecl, SongSource},
        validate_resource,
    },
    ident::{Ident, IdentParseError},
    io::write_toml_resource,
    list::List,
    youtube::{domain::is_youtube_domain, id::YoutubeID},
};
use anyhow::Context;
use std::io::{Write, stdin, stdout};

pub fn add_resource(args: AddArgs, music: &mut BioMusic) -> anyhow::Result<()> {
    // only interactive cli for now
    match args.resource {
        AddResource::Song => add_song(music),
        AddResource::Artist => add_artist(music),
        AddResource::Char => add_char(music),
        AddResource::Game => add_game(music),
    }
}

fn add_game(music: &mut BioMusic) -> anyhow::Result<()> {
    let ident: Ident = clinput("Game identifier", |s| {
        let ident = Ident::try_from(s)?;
        if music.games.contains_key(&ident) {
            anyhow::bail!("Game with identifer `{ident}` already exists");
        }
        Ok(ident)
    });
    let name: String = clinput("Game name", Ok);

    let game = GameDecl { name };

    // sync to disk
    write_toml_resource(&game, &ident, &music.root_dir_path).context("Could not save created game")?;

    // sync to struct cuz yeah
    _ = music.games.insert(ident, game);

    Ok(())
}

fn add_char(music: &mut BioMusic) -> anyhow::Result<()> {
    let ident: Ident = clinput("Character identifier", |s| {
        let ident = Ident::try_from(s)?;
        if music.characters.contains_key(&ident) {
            anyhow::bail!("Character with identifer `{ident}` already exists");
        }
        Ok(ident)
    });
    let name: String = clinput("Character name", Ok);

    let character = CharacterDecl { name, aka: List::EMPTY };

    // sync to disk
    write_toml_resource(&character, &ident, &music.root_dir_path).context("Could not save created character")?;

    // sync to struct cuz yeah
    _ = music.characters.insert(ident, character);

    Ok(())
}

fn add_artist(music: &mut BioMusic) -> anyhow::Result<()> {
    let ident: Ident = clinput("Artist identifier", |s| {
        let ident = Ident::try_from(s)?;
        if music.artists.contains_key(&ident) {
            anyhow::bail!("Artist with identifer `{ident}` already exists");
        }
        Ok(ident)
    });
    let name: String = clinput("Artist name", Ok);

    let artist = ArtistDecl {
        name: name.clone(),
        ..Default::default()
    };

    // sync to disk
    write_toml_resource(&artist, &ident, &music.root_dir_path).context("Could not save created artist")?;

    // alrighty now do #stuff
    for (song_ident, song) in &mut music.songs {
        let mut sync = concrete_powder(&mut song.artist, &ident, &name);
        for feat_artist in &mut song.feat.0 {
            sync |= concrete_powder(feat_artist, &ident, &name);
        }
        if sync {
            println!("Replaced inline decleration artist with reference in song {song_ident}");
            write_toml_resource(song, song_ident, &music.root_dir_path).context("Could not update song")?;
        }
    }

    // sync to struct cuz yeah
    _ = music.artists.insert(ident, artist);

    Ok(())
}

fn concrete_powder(artist: &mut Artist, ident: &Ident, name: &str) -> bool {
    let Resource::Decl(decl) = artist else {
        return false;
    };
    let this_name = decl.name.trim();
    let name = name.trim();
    if !this_name.eq_ignore_ascii_case(ident.string()) && !this_name.eq_ignore_ascii_case(name) {
        return false;
    }
    // replace inline decl with ref
    *artist = Resource::Ref(ident.clone());
    true
}

fn add_song(music: &mut BioMusic) -> anyhow::Result<()> {
    let youtube_id: YoutubeID = clinput("YouTube source", |s| {
        let id = parse_youtube_id(s)?;
        for (ident, song) in &music.songs {
            if song.source == SongSource::Youtube(id) {
                anyhow::bail!("Song `{ident}` already has this YouTube source");
            }
        }
        Ok(id)
    });
    let ident: Ident = clinput("Song identifier", |s| {
        let ident = parse_ident(s)?;
        if let Some(song) = music.songs.get(&ident) {
            let SongSource::Youtube(id) = song.source;
            anyhow::bail!("Song with identifier `{ident}` already exists (source: `{id}`)");
        }
        Ok(ident)
    });
    let title: String = clinput("Song title", Ok);
    let artist: Option<Ident> = clinput("Artist identifier (leave blank for inline)", |s| {
        if s.is_empty() {
            return Ok(None);
        }
        let ident = Ident::try_from(s).context("Invalid artist identifier")?;
        validate_resource::<ArtistDecl>(&ident, &music.root_dir_path)?;
        Ok(Some(ident))
    });
    let artist = if let Some(a) = artist {
        Resource::Ref(a)
    } else {
        let name: String = clinput("Artist name", Ok);
        Resource::Decl(ArtistDecl {
            name,
            ..Default::default()
        })
    };

    let source = SongSource::Youtube(youtube_id);
    let song = SongDecl::new(source, title, artist);

    // sync to disk
    write_toml_resource(&song, &ident, &music.root_dir_path).context("Could not save created song")?;

    // sync to struct cuz yeah
    _ = music.songs.insert(ident, song);
    Ok(())
}

fn parse_ident(string: String) -> anyhow::Result<Ident> {
    Ident::try_from(string).map_err(IdentParseError::into)
}

#[expect(clippy::needless_pass_by_value)]
fn parse_youtube_id(string: String) -> anyhow::Result<YoutubeID> {
    if let Ok(id) = YoutubeID::try_from(string.as_str()) {
        return Ok(id);
    }
    let Ok(url) = url::Url::parse(&string) else {
        anyhow::bail!("Provide a YouTube ID or an URL");
    };
    let Some(domain) = url.domain() else {
        anyhow::bail!("Missing domain in URL")
    };
    anyhow::ensure!(is_youtube_domain(domain), "Domain is not YouTube");
    if url.path().starts_with("/watch") {
        let Some((_, id)) = url.query_pairs().find(|x| x.0 == "v") else {
            anyhow::bail!("Missing v paramater for watch page");
        };
        let id: YoutubeID = id.parse().context("Invalid ID in v parameter")?;
        return Ok(id);
    }
    url.path()
        .trim_prefix('/')
        .parse::<YoutubeID>()
        .context("Invalid YouTube URL")
}

fn clinput<T>(prompt: &'static str, f: impl Fn(String) -> anyhow::Result<T>) -> T {
    loop {
        print!("{prompt}: ");
        stdout().flush().expect("failed to flush stdout");
        let mut buf = String::new();
        let _: usize = stdin().read_line(&mut buf).expect("failed to read from stdin");
        let buf = buf.trim().to_owned();
        match f(buf) {
            Ok(x) => return x,
            Err(e) => println!("{}", crate::report_error(&e)),
        }
    }
}
