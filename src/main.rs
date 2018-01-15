#![feature(plugin)]
#![plugin(rocket_codegen)]

use std::fs;
use std::io;
use std::io::Read;
use std::path::Path;

extern crate pulldown_cmark;
extern crate rocket;
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;

use rocket_contrib::Template;

#[derive(Serialize)]
struct Note {
  title: String,
  content: String,
}

#[derive(Serialize)]
struct Notes {
  notes: Vec<Note>,
}

#[get("/")]
fn index(notes: rocket::State<Notes>) -> Template {
  Template::render("index", &notes.inner())
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
      "date" => {}
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

fn read_notes(note_dir: &str) -> io::Result<Notes> {
  let mut notes = Notes { notes: vec![] };

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
    notes.notes.push(parse_note(data)?);
  }
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
      Ok(rocket.manage(read_notes(&note_dir).unwrap()))
    }))
    .mount("/", routes![index])
    .attach(Template::fairing())
    .launch();
}
