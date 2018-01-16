#![feature(plugin)]
#![plugin(rocket_codegen)]

use std::fs;
use std::io;
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};

extern crate chrono;
extern crate notify;
extern crate pulldown_cmark;
extern crate rocket;
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;

use chrono::prelude::*;
use notify::Watcher;
use rocket_contrib::Template;
use rocket::response::NamedFile;

#[derive(Serialize)]
struct Note {
  title: String,
  date: DateTime<Utc>,
  content: String,
}

struct Notes {
  notes: Arc<Mutex<Vec<Note>>>,
  version: Arc<(Mutex<u64>, Condvar)>,
  watcher: Mutex<notify::RecommendedWatcher>,
}

#[get("/")]
fn index(notes: rocket::State<Notes>) -> Template {
  #[derive(Serialize)]
  struct TemplateData<'a> {
    notes: &'a Vec<Note>,
  };

  Template::render(
    "index",
    TemplateData {
      notes: &notes.notes.lock().unwrap(),
    },
  )
}

fn format_version(data_version: &u64) -> String {
  format!("build_{}__data_{}", env!("BUILD_TIMESTAMP"), data_version)
}

#[get("/api/version/<expected>")]
fn version(notes: rocket::State<Notes>, expected: String) -> String {
  let &(ref lock, ref cvar) = &*notes.version;
  let mut data_version = lock.lock().unwrap();
  if format_version(&*data_version) == expected {
    // Wait until the version changes.
    data_version = cvar
      .wait_timeout(data_version, std::time::Duration::from_secs(60))
      .unwrap()
      .0;
  }
  format_version(&*data_version)
}

#[get("/static/main.js")]
fn static_files() -> Option<NamedFile> {
  NamedFile::open(Path::new("generated/main.js")).ok()
}

fn parse_note(content: String) -> io::Result<Note> {
  let header_end = content.find("\n\n").ok_or(io::Error::new(
    io::ErrorKind::InvalidInput,
    "header must be separated with two newlines",
  ))?;
  let (header, content) = content.split_at(header_end);

  let mut note = Note {
    title: "".to_string(),
    content: "".to_string(),
    date: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc),
  };

  let parser = pulldown_cmark::Parser::new_ext(content, pulldown_cmark::OPTION_ENABLE_TABLES);
  pulldown_cmark::html::push_html(&mut note.content, parser);

  for line in header.split("\n") {
    let key_val: Vec<&str> = line.splitn(2, ":").collect();
    if key_val.len() != 2 {
      return Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "expected 'key: val' in header, got ".to_owned() + line,
      ));
    }
    let val = key_val[1].trim();

    match key_val[0].as_ref() {
      "name" => note.title = val.to_string(),
      "date" => {
        note.date = Utc
          .datetime_from_str(&val, "%F %R")
          .expect(&("Failed to parse '".to_owned() + val + "'"))
      }
      _ => {
        return Err(io::Error::new(
          io::ErrorKind::InvalidInput,
          "unknown header attribute: ".to_owned() + line,
        ))
      }
    }
  }
  Ok(note)
}

fn read_notes(note_dir: &str) -> io::Result<Vec<Note>> {
  let mut notes = vec![];

  for note_path in fs::read_dir(Path::new(note_dir)).expect("Unable to list note directory") {
    let note_path = note_path.expect("unable to get entry");

    let is_markdown = note_path
      .path()
      .extension()
      .map_or(false, |ext| ext == "markdown");
    if !is_markdown {
      continue;
    }

    let mut data = String::new();
    fs::File::open(note_path.path())?.read_to_string(&mut data)?;
    notes.push(parse_note(data)?);
  }

  // Put newest notes first.
  notes.sort_by(|a, b| b.date.cmp(&a.date));

  Ok(notes)
}

fn watch_notes(note_dir: &str) -> io::Result<Notes> {
  let (tx, rx) = std::sync::mpsc::channel();
  let notes = Notes {
    notes: Arc::new(Mutex::new(read_notes(note_dir)?)),
    version: Arc::new((Mutex::new(0), Condvar::new())),
    watcher: std::sync::Mutex::new(notify::watcher(tx, std::time::Duration::from_secs(1)).unwrap()),
  };

  notes
    .watcher
    .lock()
    .unwrap()
    .watch(note_dir, notify::RecursiveMode::Recursive)
    .unwrap();

  let note_dir = note_dir.to_string();
  let shared_notes = notes.notes.clone();
  let shared_version = notes.version.clone();
  std::thread::spawn(move || {
    println!("starting watcher thread ");
    loop {
      match rx.recv() {
        Ok(event) => {
          println!("{:?}", event);
          *shared_notes.lock().unwrap() = read_notes(&note_dir).unwrap();

          let &(ref lock, ref cvar) = &*shared_version;
          *lock.lock().unwrap() += 1;
          cvar.notify_all();
        }
        Err(e) => println!("watch error: {:?}", e),
      }
    }
  });
  Ok(notes)
}

fn main() {
  rocket::ignite()
    .attach(rocket::fairing::AdHoc::on_attach(|rocket| {
      let note_dir = rocket
        .config()
        .get_str("note_dir")
        .unwrap_or("notes")
        .to_owned();
      Ok(rocket.manage(watch_notes(&note_dir).unwrap()))
    }))
    .mount("/", routes![index, version, static_files])
    .attach(Template::fairing())
    .launch();
}
