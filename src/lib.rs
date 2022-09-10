use colored::*;
use copypasta::{ClipboardContext, ClipboardProvider};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect, Password, Select};
use home::home_dir;
use rusqlite::{params, Connection, Result};
use std::{env, fmt::Display, fs};

pub const TABLE: &str = "services";
pub const CHOICES: [&str; 5] = [
    "Show all services",
    "Create service (+)",
    "Delete services (-)",
    "Search service by name",
    "Exit",
];

#[derive(Debug)]
pub struct Service {
    name: String,
    password: String,
    username: String,
}

fn get_key_content(key_file_path: &str) -> String {
    let key = fs::read(&key_file_path).unwrap();
    let key = String::from_utf8_lossy(&key).to_string();
    key
}

pub fn get_key() -> Option<String> {
    let key_file_path = get_key_file_path();

    match fs::read(&key_file_path) {
        Ok(_) => Some(get_key_content(&key_file_path)),
        Err(_) => None,
    }
}
fn prompt_key_creation() {
    if get_key().is_some() {
        return;
    }
    let key_file_path = get_key_file_path();
    let question = format!(
        "{} was not found. Do you wish to create it? This file is a key to protect the passwords",
        &key_file_path
    );
    let should_create_key_file = get_user_confirmation(&question);

    if should_create_key_file {
        let key = create_key();
        fs::write(&key_file_path, &key).unwrap();
        display_message(
            "info",
            format!("{} was created", &key_file_path).as_str(),
            "green",
        );
    }
}

fn get_key_file_path() -> String {
    format!(
        "{}/{}.key",
        home_dir().unwrap().display(),
        env!("CARGO_PKG_NAME")
    )
}

pub fn get_database_path() -> String {
    format!(
        "{}/{}.db3",
        home_dir().unwrap().display(),
        env!("CARGO_PKG_NAME")
    )
}

///Set clipboard (control + v)
pub fn set_clipboard(content: &str) {
    let mut ctx = ClipboardContext::new().unwrap();
    ctx.set_contents(content.to_string().to_owned()).unwrap();
    ctx.get_contents().unwrap();
}

fn database_is_empty(conn: &Connection) -> bool {
    let services = get_services(&conn, false).unwrap();
    match services.len() {
        0 => {
            display_message("error", "Database is empty", "red");
            true
        }
        _ => false,
    }
}

pub fn display_services(conn: &Connection, search: bool) {
    if database_is_empty(&conn) {
        return;
    }
    prompt_key_creation();
    let mut emoji = "🔐";
    let key = get_key();

    if key.is_some() {
        emoji = "🔓";
    }

    let services = get_services(&conn, search).unwrap();

    if services.len() == 0 {
        display_message("error", "No service was found", "red");
        return;
    }

    let mut service_names: Vec<String> = services
        .iter()
        .map(|x| {
            format!(
                "{} with username/email {} has password {}",
                &x.name, &x.username, &emoji
            )
        })
        .collect();
    service_names.sort();

    let (selected_service_text, _) = get_user_selection(
        &service_names,
        format!("{} available services", service_names.len()).as_str(),
    );
    if key.is_none() {
        display_message(
            "error",
            "Cannot save password to clipboard without the security key",
            "red",
        );
        return;
    }
    let name = selected_service_text.split_whitespace().next().unwrap();
    let username = selected_service_text
        .split_ascii_whitespace()
        .nth(3)
        .unwrap();

    let selected_service = services
        .iter()
        .find(|x| x.name == name && x.username == username)
        .unwrap();

    let password = decrypt_text(&selected_service.password, &key.unwrap());

    if password.is_none() {
        display_message("error", "Invalid security key", "red");
        return;
    }
    set_clipboard(&password.unwrap());

    let message = format!(
        "Password for {:?} has been saved to your clipboard. You can use it as long as the program is running",
        selected_service.name
    );
    display_message("ok", &message, "green");
}

///Create database table if not exists
pub fn create_database(conn: &Connection) -> Result<()> {
    conn.execute(
        &format!(
            "CREATE TABLE IF NOT EXISTS {TABLE} (
                  id              INTEGER PRIMARY KEY,
                  name           VARCHAR(255) NOT NULL,
                  password          VARCHAR(255) NOT NULL,
                  username          VARCHAR(255) NOT NULL
                  )"
        ),
        [],
    )?;
    Ok(())
}

///Delete database records
pub fn delete_record(conn: &Connection, service: &Service) -> Result<()> {
    conn.execute(
        &format!("DELETE FROM {TABLE} WHERE name = ?1 AND username = ?2"),
        params![service.name, service.username],
    )?;
    Ok(())
}

///Delete all database records
pub fn purge_database(conn: &Connection) -> Result<()> {
    if get_user_confirmation("Are you sure you want to delete all services") {
        conn.execute(&format!("DELETE FROM {TABLE}"), params![])?;
    }
    Ok(())
}

///Create server connection on database
pub fn create_service(conn: &Connection) {
    let key = get_key();
    let spaces_tip_message = "Tip: replace spaces with underscores";

    if key.is_none() {
        display_message("error", "Security key is required to proceed", "red");
        return;
    }

    let name = get_user_input("Name", "sample", false);

    if name.is_none() {
        display_message("info", &spaces_tip_message, "yellow");
        return;
    }
    let username = get_user_input("Username/email", env!("USER"), false);

    if username.is_none() {
        display_message("info", &spaces_tip_message, "yellow");
        return;
    }

    let name = name.unwrap();
    let username = username.unwrap();

    let password = get_user_input_masked("Password");
    let password = encrypt_text(&password, &key.unwrap());

    let record = Service {
        name,
        password,
        username,
    };
    conn.execute(
        &format!("INSERT INTO {TABLE} (name, password, username) VALUES (?1, ?2, ?3)"),
        params![record.name, record.password, record.username],
    )
    .unwrap();

    let msg = format!("Service created: {:?}", &record.name);
    display_message("ok", &msg, "green");
}

///Get all database connections from database
pub fn get_services(conn: &Connection, search: bool) -> Result<Vec<Service>> {
    let mut query = format!("SELECT * FROM {TABLE}");
    let mut records: Vec<Service> = Vec::new();

    if search {
        let name = get_user_input("name", "sample", false).unwrap().to_string();
        query = format!("SELECT * FROM {TABLE} WHERE name like '%{name}%'");
    }
    let mut stmt = conn.prepare(&query)?;

    let result_iter = stmt.query_map([], |row| {
        Ok(Service {
            name: row.get(1)?,
            password: row.get(2)?,
            username: row.get(3)?,
        })
    })?;

    for i in result_iter {
        records.push(i?);
    }
    Ok(records)
}

pub fn display_message(message_type: &str, message: &str, color: &str) {
    let msg = format!("[{}] {}", message_type.to_uppercase(), message).color(color);
    println!("{msg}");
}

///Get boolean response
fn get_user_confirmation(question: &str) -> bool {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(question)
        .default(true)
        .interact()
        .unwrap()
}

///Get text response.
fn get_user_input(text: &str, default_text: &str, allow_spaces: bool) -> Option<String> {
    let res: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt(text)
        .default(default_text.into())
        .interact_text()
        .unwrap();

    if allow_spaces {
        return Some(res);
    }

    let text_parts = &res.split_ascii_whitespace().count();
    if text_parts != &1_usize {
        display_message("error", "Spaces are not allowed", "red");
        return None;
    }
    let res = res.split_ascii_whitespace().next().unwrap().to_string();
    Some(res)
}

///Get singe response from choices
pub fn get_user_selection<T>(items: &Vec<T>, title: &str) -> (String, usize)
where
    T: Display,
{
    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&items)
        .with_prompt(title)
        .default(0)
        .interact()
        .unwrap();

    (items.get(selection).unwrap().to_string(), selection)
}

///Get multiple responses
fn get_user_multi(items: &Vec<String>, title: &str) -> Vec<String> {
    let mut res = Vec::new();

    let chosen: Vec<usize> = MultiSelect::with_theme(&ColorfulTheme::default())
        .items(&items)
        .with_prompt(title)
        .interact()
        .unwrap();

    for i in chosen {
        let each = items.get(i);
        if each.is_some() {
            res.push(each.unwrap().to_string());
        }
    }
    res
}

pub fn delete_services(conn: &Connection) {
    if database_is_empty(&conn) {
        return;
    }
    let (_, index) = get_user_selection(&["All", "Selection"].to_vec(), "What to delete");

    if index == 0 {
        purge_database(&conn).unwrap();
        return;
    }
    let services = get_services(&conn, false).unwrap();
    let mut service_names: Vec<String> = services
        .iter()
        .map(|x| format!("{} with username/email {}", x.name, x.username))
        .collect();

    service_names.sort();

    let selected_service_text = get_user_multi(
        &service_names,
        "Services to delete. Hit space key to select/unselect",
    );

    if selected_service_text.len() == 0 {
        display_message("info", "No service will be deleted", "green");
        return;
    }

    let question = format!(
        "Are you sure you wish to delete {:?}",
        &selected_service_text
    );

    if !get_user_confirmation(&question) {
        return;
    }

    let mut services_to_delete: Vec<&Service> = Vec::new();

    for i in &selected_service_text {
        let name = i.split_ascii_whitespace().nth(0).unwrap();
        let username = i.split_ascii_whitespace().nth(3).unwrap();

        match services
            .iter()
            .find(|x| &x.name == &name && &x.username == &username)
        {
            Some(x) => services_to_delete.push(x),
            None => continue,
        }
    }
    for i in services_to_delete {
        delete_record(&conn, &i).unwrap();
    }
    let message = format!("Services deleted: {:?}", &selected_service_text);
    display_message("ok", &message, "green");
}

pub fn display_app_intro() {
    let key_found = match get_key() {
        Some(_) => "Yes".green(),
        None => "No".red(),
    };

    let title = format!(
        "\n{} - {} \nAuthors: {}\nVersion: {}\nLicense: {}\nCrafted with ❤️ using Rust language\nSecurity key found: {}\nDatabase: {}\n",
        env!("CARGO_PKG_NAME").to_uppercase(),
        env!("CARGO_PKG_DESCRIPTION"),
        env!("CARGO_PKG_AUTHORS"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_LICENSE"),
        key_found,
        get_database_path()
    );

    println!("{title}");
}

fn create_key() -> String {
    fernet::Fernet::generate_key()
}

fn encrypt_text(raw_text: &str, key: &str) -> String {
    let fernet = fernet::Fernet::new(&key).unwrap();
    fernet.encrypt(raw_text.as_bytes())
}

fn decrypt_text(encrypted_text: &str, key: &str) -> Option<String> {
    let fernet = fernet::Fernet::new(&key).unwrap();
    let decrypted_plaintext = fernet.decrypt(&encrypted_text);
    if decrypted_plaintext.is_err() {
        return None;
    }
    Some(format!(
        "{}",
        String::from_utf8_lossy(&decrypted_plaintext.unwrap())
    ))
}

fn get_user_input_masked(text: &str) -> String {
    let confirmartion_message = format!("Confirm {text}");
    let mismatch_message = format!("{text} is mismatching");

    Password::new()
        .with_prompt(text)
        .with_confirmation(confirmartion_message, mismatch_message)
        .interact()
        .unwrap()
}
