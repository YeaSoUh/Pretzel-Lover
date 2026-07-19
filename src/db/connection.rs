use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};
use turso::{Builder, Connection, Database};

static DB: OnceLock<Arc<Database>> = OnceLock::new();

pub struct Planet {
    id: String,
    star_id: isize,
    name: String,
}

pub async fn establish_database(database_url: &str) -> anyhow::Result<()> {
    let db = Builder::new_local(database_url).build().await?;
    DB.set(Arc::new(db))
        .map_err(|_| anyhow::anyhow!("DB is already initialized"))?;

    Ok(())
}

pub async fn get_planet(index: &str) -> anyhow::Result<Planet> {
    let conn = establish_connection().await?;

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

pub fn search_planets(_input: &str) {}

pub async fn edit_planet(index: &str, input: &str, _bypass: bool) -> anyhow::Result<()> {
    let conn = establish_connection().await?;

    let binding = normalize(input);

    let star_id = index
        .split('-')
        .next()
        .ok_or_else(|| anyhow::anyhow!("invalid id format"))?
        .parse::<isize>()?;

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

    conn.execute(
        // to be replaced
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
        planet.id, planet.star_id, planet.name
    )
}

async fn establish_connection() -> anyhow::Result<Connection> {
    let db = DB
        .get()
        .ok_or_else(|| anyhow::anyhow!("DB not initialized"))?;
    let db_ref = Arc::clone(db);

    let conn = db_ref.as_ref().connect()?;
    conn.busy_timeout(Duration::from_millis(1500))?; // 1.5 seconds
    conn.pragma_update("journal_mode", "'mvcc'").await?; // enables concurrency writes which is good!

    Ok(conn)
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
