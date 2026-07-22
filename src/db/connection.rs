use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};
use turso::{Builder, Connection, Database, Row};
use twilight_model::http::attachment::Attachment;

use crate::AppState;

static DB: OnceLock<Arc<Database>> = OnceLock::new();

#[derive(sea_query::Iden)]
enum Planets {
    Table,
    Id,
    StarId,
    Name,
    Resources,
    Moon,
}

#[allow(dead_code)] // yes
pub struct Planet {
    id: String,
    star_id: i64,
    name: String,
    resources: String,
    moon: bool,
}

pub async fn establish_database(database_url: &str) -> anyhow::Result<()> {
    let db = Builder::new_local(database_url).build().await?;
    DB.set(Arc::new(db))
        .map_err(|_| anyhow::anyhow!("DB is already initialized"))?;

    Ok(())
}

pub async fn get_planet(index: &str) -> anyhow::Result<Planet> {
    let conn = establish_connection().await?;

    let mut planets = conn
        .query("SELECT * FROM planets WHERE id = ?1", [index])
        .await?;

    if let Some(row) = planets.next().await? {
        return Ok(construct_planet(row)?);
    }

    Err(anyhow::anyhow!("Planet wasn't found"))
}

pub async fn search_planets(input: &mut str, state: AppState) -> anyhow::Result<Attachment> {
    if check_sql(input, state) {
        anyhow::bail!("Blacklisted sql");
    }

    let conn = establish_connection().await?;
    normalize(input);

    let mut planets = conn
        .query(format!("SELECT * FROM planets WHERE {}", input), ())
        .await?;

    let mut file_content = "".to_owned();

    while let Some(row) = planets.next().await? {
        file_content.push_str(&format_response(construct_planet(row)?));
    }

    Ok(Attachment::from_bytes(
        "result.txt".to_owned(),
        file_content.into_bytes(),
        0,
    ))
}

pub async fn edit_planet(index: &str, input: &str, bypass: bool) -> anyhow::Result<()> {
    let conn = establish_connection().await?;

    let mut split_iter = index.split('-');
    let star_id = split_iter
        .next()
        .ok_or_else(|| anyhow::anyhow!("invalid star id format"))?
        .parse::<i64>()?;

    let planet_id = split_iter
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing planet id in index"))?;

    // i don't really need a moon id, just need to verify if it is a moon
    let mut is_moon = split_iter.next();

    let mut name: Option<String> = None;
    let mut resources: Option<String> = None;

    for expr in input.split("|") {
        let expr = expr.trim();
        if expr.is_empty() {
            continue;
        }

        let (mut key, mut value) = expr
            .split_once("=")
            .ok_or(anyhow::anyhow!("deformed input"))?; // idk i may improve it later
        key = key.trim();
        value = value.trim();

        match key {
            "name" => name = Some(value.to_string()),
            "resources" => resources = Some(value.to_string()),
            "moon" if bypass => is_moon = Some(value),
            _ => continue,
        }
    }

    let mut columns = vec![Planets::Id, Planets::StarId];
    let mut values: Vec<sea_query::SimpleExpr> = vec![planet_id.into(), star_id.into()];

    let mut conflict = sea_query::OnConflict::new();

    if let Some(name) = name {
        columns.push(Planets::Name);
        values.push(name.into());
        conflict.update_column(Planets::Name);
    }

    if let Some(resources) = resources {
        columns.push(Planets::Resources);
        values.push(resources.into());
        conflict.update_column(Planets::Resources);
    }

    columns.push(Planets::Moon);
    conflict.update_column(Planets::Moon);
    match is_moon {
        Some(_is_moon) => values.push(sea_query::Expr::value(true)),
        None => values.push(sea_query::Expr::value(false)),
    }

    let (sql, query_values) = sea_query::Query::insert()
        .into_table(Planets::Table)
        .columns(columns)
        .values(values)?
        .on_conflict(conflict)
        .build(sea_query::SqliteQueryBuilder);

    let turso_params: Vec<turso::Value> = query_values
        .into_iter()
        .map(|v| match v {
            sea_query::Value::Int(Some(i)) => turso::Value::Integer(i as i64),
            sea_query::Value::BigInt(Some(i)) => turso::Value::Integer(i),
            sea_query::Value::String(Some(s)) => turso::Value::Text(s),
            sea_query::Value::Bool(Some(b)) => turso::Value::Integer(if b { 1 } else { 0 }),
            _ => turso::Value::Null,
        })
        .collect();

    conn.execute(&sql, turso_params).await?;

    Ok(())
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

fn construct_planet(row: Row) -> anyhow::Result<Planet> {
    Ok(Planet {
        id: row.get(0)?,
        star_id: row.get::<i64>(1)?,
        name: row.get(2)?,
        resources: row.get(3)?,
        moon: row.get(4)?,
    })
} // i will eventually make it as impl

fn check_sql(input: &str, state: AppState) -> bool {
    state
        .configs
        .sql_blacklist
        .iter()
        .any(|sql| input.contains(sql))
}

pub fn format_response(planet: Planet) -> String {
    format!(
        "ID: {}\nStar Id: {}, Name: {}",
        planet.id, planet.star_id, planet.name
    )
}

fn normalize(input: &mut str) {
    let _ = input.replace("&&", "and");
    let _ = input.replace("||", "or");
}

/*fn validate(_input: &str) -> anyhow::Result<()> {
    Ok(())
}*/
