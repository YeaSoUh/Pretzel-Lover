use std::{sync::OnceLock, time::Duration};
use parking_lot::Mutex;
use turso::{Builder, Connection};

static DB_READ: OnceLock<Mutex<Connection>> = OnceLock::new();
static DB_WRITE: OnceLock<Connection> = OnceLock::new();

pub struct Planet {
    id: String,
    star_id: isize,
    name: String
}

pub async fn establish_connection(database_url: &str) -> anyhow::Result<()> {
    /*let conn_write = Connection::open_with_flags(
        database_url,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    ).unwrap();

    conn_write.pragma_update(None, "journal_mode", "WAL").unwrap();
    conn_write.pragma_update(None, "busy_timeout", "5000").unwrap();

    let conn_read = Connection::open_with_flags(
        database_url,
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    ).unwrap();
    conn_read.pragma_update(None, "busy_timeout", "5000").unwrap();

    DB_WRITE.set(Mutex::new(conn_write)).ok();
    DB_READ.set(Mutex::new(conn_read)).ok();
    */

    let db = Builder::new_local(database_url).build().await?;

    let conn_write = db.connect()?;
    conn_write.busy_timeout(Duration::new(3, 0))?;
    conn_write.pragma_update("journal_mode", "'mvcc'").await?; // enables concurrency writes which is good!

    DB_WRITE.set(conn_write).map_err(|_| anyhow::anyhow!("DB_WRITE already initialized"))?;

    // as for db_read seems like there is no flag for read only so db_read and db_write will be unified

    Ok(())
}

pub fn get_planet(index: &str) -> anyhow::Result<Planet> {
    let conn = DB_READ.get().ok_or_else(|| anyhow::anyhow!("Database not initialized"))?.lock();

    let planet = conn.query_row(
        "SELECT * FROM planets WHERE id = ?1",
        rusqlite::params![index],
        |row| {
            Ok(Planet {
                id: row.get(0)?,
                star_id: row.get(1)?,
                name: row.get(2)?,
            })
        },
    )?;

    Ok(planet)
}

pub fn search_planets(_input: &str) {

}

pub fn edit_planet(index: &str, input: &str, _bypass: bool) -> anyhow::Result<()> {
    let conn = DB_WRITE.get().ok_or_else(|| anyhow::anyhow!("Database not initialized"))?;

    let binding = normalize(input);

    let star_id = index.split('-').next().ok_or_else(|| anyhow::anyhow!("invalid id format"))?.parse::<isize>()?;

    let mut name: Option<&str> = None;

    for part in binding.split('|') {
        let part = part.trim();

        if let Some((k, v)) = part.split_once('=') {
            match k.trim() {
                "name" => name = Some(v.trim()),
                _ => {}
            }
        }
    }
    
    conn.execute( // to be replaced
        "
        INSERT INTO users (id, star_id, name)
        VALUES (?1, ?2, COALESCE(?3, DEFAULT))
        ON CONFLICT(id) DO UPDATE SET
            name = COALESCE(excluded.name, name)
        ",
        rusqlite::params![index, star_id, name],
    )?;

    Ok(())
}

pub fn format_response(planet: Planet) -> String {
    format!(
        "ID: {}\nStar Id: {}, Name: {}",
        planet.id,
        planet.star_id,
        planet.name
    )
}

fn normalize(input: &str) -> String {
    let mut out = String::with_capacity(input.len());

    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '&' if chars.peek() == Some(&'&') => {
                chars.next();
                out.push_str("and");
            }
            '|' if chars.peek() == Some(&'|') => {
                chars.next();
                out.push_str("or");
            }
            _ => out.push(c),
        }
    }

    out
}

/*fn validate(_input: &str) -> anyhow::Result<()> {
    Ok(())
}*/