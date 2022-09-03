use rusqlite::{Connection, Result};
use safepass::*;

fn main() -> Result<()> {
    let conn = Connection::open(&get_database_path())?;
    create_database(&conn)?;
    display_app_intro();

    loop {
        let (_, index) = get_user_selection(&CHOICES.to_vec(), "Option");

        match index {
            0 => display_services(&conn),
            1 => create_service(&conn),
            2 => delete_services(&conn),
            _ => {
                set_clipboard("empty");
                display_message("info", "Clipboard has been erased", "green");
                break;
            }
        }
    }
    Ok(())
}
