use colored::*;
use copypasta::{ClipboardContext, ClipboardProvider};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect, Password, Select};
use home::home_dir;
use rusqlite::{params, Connection, Result};
use std::{env, fmt::Display, fs};

pub const TABLE: &str = "services";
pub const CHOICES: [&str; 4] = ["Show services", "Create service", "Delete services", "Exit"];

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

#[derive(Debug)]
pub struct Service {
    name: String,
    password: String,
}

///Set clipboard (control + v)
pub fn set_clipboard(content: &str) {
    let mut ctx = ClipboardContext::new().unwrap();
    ctx.set_contents(content.to_string().to_owned()).unwrap();
    ctx.get_contents().unwrap();
}

fn database_is_empty(conn: &Connection) -> bool {
    let services = get_services(&conn).unwrap();
    match services.len() {
        0 => {
            display_message("error", "Database is empty", "red");
            true
        }
        _ => false,
    }
}

pub fn display_services(conn: &Connection) {
    if database_is_empty(&conn) {
        return;
    }
    prompt_key_creation();
    let mut emoji = "🔐";
    let key = get_key();

    if key.is_some() {
        emoji = "🔓";
    }

    let services = get_services(&conn).unwrap();
    let service_names: Vec<String> = services
        .iter()
        .map(|x| format!("{} - {}", &x.name, &emoji))
        .collect();

    let (selected_service_name, _) = get_user_selection(&service_names, "Available services");
    if key.is_none() {
        display_message(
            "error",
            "Cannot save password to clipboard without the security key",
            "red",
        );
        return;
    }

    let selected_service = services
        .iter()
        .find(|x| x.name == selected_service_name.split_whitespace().next().unwrap())
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
                  password          VARCHAR(255) NOT NULL
                  )"
        ),
        [],
    )?;
    Ok(())
}

///Delete database records
pub fn delete_records_by_name(conn: &Connection, name: &str) -> Result<()> {
    conn.execute(
        &format!("DELETE FROM {TABLE} WHERE name = ?1"),
        params![name],
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

    if key.is_none() {
        display_message("error", "Security key is required to proceed", "red");
        return;
    }

    let name = get_user_input("Name", "sample");
    let password = get_user_input_masked("Password");
    let password = encrypt_text(&password, &key.unwrap());

    let record = Service { name, password };
    conn.execute(
        &format!("INSERT INTO {TABLE} (name, password) VALUES (?1, ?2)"),
        params![record.name, record.password,],
    )
    .unwrap();

    let msg = format!("Service created: {:?}", &record.name);
    display_message("ok", &msg, "green");
}

///Get all database connections from database
pub fn get_services(conn: &Connection) -> Result<Vec<Service>> {
    let mut records: Vec<Service> = Vec::new();
    let query = format!("SELECT * FROM {TABLE}");
    let mut stmt = conn.prepare(&query)?;

    let result_iter = stmt.query_map([], |row| {
        Ok(Service {
            name: row.get(1)?,
            password: row.get(2)?,
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

///Get text response
fn get_user_input(text: &str, default_text: &str) -> String {
    Input::with_theme(&ColorfulTheme::default())
        .with_prompt(text)
        .default(default_text.into())
        .interact_text()
        .unwrap()
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
fn get_user_multi(items: &Vec<&str>, title: &str) -> Vec<String> {
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
    let services = get_services(&conn).unwrap();
    let service_names = services.iter().map(|x| x.name.as_str()).collect();
    let selected_service_names = get_user_multi(&service_names, "Services to delete");

    let question = format!(
        "Are you sure you wish to delete {:?}",
        &selected_service_names
    );

    if !get_user_confirmation(&question) {
        return;
    }

    let mut services_to_delete: Vec<&Service> = Vec::new();

    for i in &selected_service_names {
        match services.iter().find(|x| &x.name == i) {
            Some(x) => services_to_delete.push(x),
            None => continue,
        }
    }
    for i in services_to_delete {
        delete_records_by_name(&conn, &i.name).unwrap();
    }
    let message = format!("Services deleted: {:?}", &selected_service_names);
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
