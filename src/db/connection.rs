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
    Radius,
    Gravity,
    Temperature,
    Tectonics,
    Atmosphere,
    Oceans,
    Rings,
    Trees,
    Life,
    IsMoon,
    Moons,
    Malachite,
    Hematite,
    Petroleum,
    Coal,
    Gummite,
    Tektite,
    Bauxite,
    Cerussite,
    Lime,
    Quartz,
}

#[allow(dead_code)] // yes
pub struct Planet {
    id: String,
    star_id: i64,
    name: String,
    // default: 0 for radius, gravity and temp
    radius: f64,
    gravity: f64,
    temperature: i64,
    tectonics: String,
    atmosphere: Option<String>,
    oceans: Option<String>,
    rings: Option<String>,
    trees: Option<String>,
    life: bool,
    is_moon: bool,
    moons: Option<i8>,

    malachite: Option<i8>,
    hematite: Option<f64>,
    petroleum: Option<i8>,
    coal: Option<i8>,
    gummite: Option<i8>,
    tektite: Option<i8>,
    bauxite: Option<i8>,
    cerussite: Option<i8>,

    lime: Option<bool>,
    quartz: Option<bool>,
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
        file_content.push_str(&format!("\n{}", format_response(&construct_planet(row)?)));
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
    let star_id: i64 = split_iter
        .next()
        .ok_or_else(|| anyhow::anyhow!("invalid star id format"))?
        .parse()?;

    split_iter
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing planet id in index"))?;

    let mut is_moon = split_iter.next().is_some();

    let mut name: Option<String> = None;
    let mut radius: Option<f64> = None;
    let mut gravity: Option<f64> = None;

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
            "radius" => radius = Some(value.parse::<f64>()?),
            "gravity" => gravity = Some(value.parse::<f64>()?),
            "moon" if bypass => is_moon = value.parse::<bool>()?,
            _ => continue,
        }
    }

    let mut columns = vec![Planets::Id, Planets::StarId];
    let mut values: Vec<sea_query::SimpleExpr> = vec![index.into(), star_id.into()];

    let mut conflict = sea_query::OnConflict::new();

    if let Some(name) = name {
        columns.push(Planets::Name);
        values.push(name.into());
        conflict.update_column(Planets::Name);
    }
    if let Some(radius) = radius {
        columns.push(Planets::Radius);
        values.push(radius.into());
        conflict.update_column(Planets::Radius);
    }
    if let Some(gravity) = gravity {
        columns.push(Planets::Gravity);
        values.push(gravity.into());
        conflict.update_column(Planets::Gravity);
    }

    columns.push(Planets::IsMoon);
    conflict.update_column(Planets::IsMoon);
    values.push(is_moon.into());

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
            sea_query::Value::Double(Some(f)) => turso::Value::Real(f),
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
        star_id: row.get(1)?,
        name: row.get(2)?,
        radius: row.get(3)?,
        gravity: row.get(4)?,
        temperature: row.get(5)?,
        tectonics: row.get(6)?,
        atmosphere: row.get(7)?,
        oceans: row.get(8)?,
        rings: row.get(9)?,
        trees: row.get(10)?,
        life: row.get(11)?,
        is_moon: row.get(12)?,
        moons: row.get::<Option<i64>>(13)?.map(|v| v as i8),
        malachite: row.get::<Option<i64>>(14)?.map(|v| v as i8),
        hematite: row.get(15)?,
        petroleum: row.get::<Option<i64>>(16)?.map(|v| v as i8),
        coal: row.get::<Option<i64>>(17)?.map(|v| v as i8),
        gummite: row.get::<Option<i64>>(18)?.map(|v| v as i8),
        tektite: row.get::<Option<i64>>(19)?.map(|v| v as i8),
        bauxite: row.get::<Option<i64>>(20)?.map(|v| v as i8),
        cerussite: row.get::<Option<i64>>(21)?.map(|v| v as i8),
        lime: row.get(22)?,
        quartz: row.get(23)?,
    })
}

fn check_sql(input: &str, state: AppState) -> bool {
    state
        .configs
        .sql_blacklist
        .iter()
        .any(|sql| input.contains(sql))
}

pub fn format_response(planet: &Planet) -> String { // will get rewritten
    let mut out = String::from("");

    out.push_str(&format!(
        "ID: {}\nStar Id: {}, Name: {}\nRadius: {}\nGravity: {}\nTemperature: {}\nTectonics: {}",
        planet.id, planet.star_id, planet.name, planet.radius, planet.gravity, planet.temperature, planet.tectonics
    ));

    if let Some(atmosphere) = &planet.atmosphere {
        out.push_str(&format!("Atmosphere: {}", atmosphere))
    }
    if let Some(oceans) = &planet.oceans {
        out.push_str(&format!("Oceans: {}", oceans))
    }
    if let Some(rings) = &planet.rings {
        out.push_str(&format!("Rings: {}", rings))
    }
    if let Some(trees) = &planet.trees {
        out.push_str(&format!("Trees: {}", trees))
    }
    out.push_str(&format!(
        "Life: {}\nIs Moon: {}",
        planet.life, planet.is_moon
    ));
    if let Some(moons) = &planet.moons {
        out.push_str(&format!("Moons: {}", moons))
    }
    out.push_str("\n");

    if let Some(malachite) = &planet.malachite {
        out.push_str(&format!("Malachite: {}", malachite))
    }
    if let Some(hematite) = &planet.hematite {
        out.push_str(&format!("Hematite: {}", hematite))
    }
    if let Some(petroleum) = &planet.petroleum {
        out.push_str(&format!("Petroleum: {}", petroleum))
    }
    if let Some(coal) = &planet.coal {
        out.push_str(&format!("Coal: {}", coal))
    }
    if let Some(gummite) = &planet.gummite {
        out.push_str(&format!("Gummite: {}", gummite))
    }
    if let Some(tektite) = &planet.tektite {
        out.push_str(&format!("Tektite: {}", tektite))
    }
    if let Some(bauxite) = &planet.bauxite {
        out.push_str(&format!("Bauxite: {}", bauxite))
    }
    if let Some(cerussite) = &planet.cerussite {
        out.push_str(&format!("Cerussite: {}", cerussite))
    }
    out.push_str("\n");

    if let Some(lime) = &planet.lime {
        out.push_str(&format!("Lime: {}", lime))
    }
    if let Some(quartz) = &planet.quartz {
        out.push_str(&format!("Quartz: {}", quartz))
    }

    out
}

fn normalize(input: &mut str) {
    let _ = input.replace("&&", "and");
    let _ = input.replace("||", "or");
}

/*fn validate(_input: &str) -> anyhow::Result<()> {
    Ok(())
}*/
