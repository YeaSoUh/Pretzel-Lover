use serde::Deserialize;
use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};
use turso::{Builder, Connection, Database, Row};
use twilight_model::http::attachment::Attachment;

use crate::AppState;

static DB: OnceLock<Arc<Database>> = OnceLock::new();
static CHECKS: OnceLock<Checks> = OnceLock::new();

#[derive(Deserialize)]
struct Checks {
    #[serde(rename = "allowed_sectors")]
    sectors: Vec<String>,
    #[serde(rename = "allowed_tectonics")]
    tectonics: Vec<String>,
    #[serde(rename = "allowed_atmospheres")]
    atmospheres: Vec<String>,
    #[serde(rename = "allowed_oceans")]
    oceans: Vec<String>,
    #[serde(rename = "allowed_trees")]
    trees: Vec<String>,
}

#[derive(sea_query::Iden)]
enum Planets {
    Table,
    Id,
    StarId,
    Name,
    Radius,
    Gravity,
    Temperature,
    Sector,
    Tectonics,
    Atmosphere,
    Oceans,
    Rings,
    Trees,
    SubTrees,
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
    Gold,
    Cerussite,
    Lime,
    Saltpeter,
    Quartz,
    Ice,
    Note,
}

#[allow(dead_code)] // yes
pub struct Planet {
    id: String,
    star_id: i64,
    // default: "Unknown"
    name: String,
    // default: 0 for radius, gravity and temp
    radius: f64,
    gravity: f64,
    temperature: i64,
    sector: Option<String>,
    // Default: "Unknown" for tectonics
    tectonics: String,
    atmosphere: Option<String>,
    oceans: Option<String>,
    rings: Option<String>,
    trees: Option<String>,
    sub_trees: Option<String>,
    // default: false (only for life)
    life: bool,
    is_moon: bool,
    moons: Option<i32>,

    malachite: Option<i32>,
    hematite: Option<f64>,
    petroleum: Option<i32>,
    coal: Option<i32>,
    gummite: Option<i32>,
    tektite: Option<i32>,
    bauxite: Option<i32>,
    gold: Option<i32>,
    cerussite: Option<i32>,

    lime: Option<bool>,
    saltpeter: Option<bool>,
    quartz: Option<bool>,
    ice: Option<bool>,

    note: Option<String>,
}

pub async fn establish_database(database_url: &str) -> anyhow::Result<()> {
    let db = Builder::new_local(database_url).build().await?;
    DB.set(Arc::new(db))
        .map_err(|_| anyhow::anyhow!("DB is already initialized"))?;

    let check: Checks = {
        let content = std::fs::read("configs.json")?;
        serde_json::from_slice(&content)?
    };
    CHECKS
        .set(check)
        .map_err(|_| anyhow::anyhow!("CHECKS is already initialized"))?;

    Ok(())
}

// i will maybe replace it with search_planets later
pub async fn get_planet(index: &str, state: AppState) -> anyhow::Result<Planet> {
    if check_sql(index, state) {
        anyhow::bail!("Blacklisted sql");
    }

    let conn = establish_connection().await?;

    let mut planets = conn
        .query("SELECT * FROM planets WHERE id = ?1", [index])
        .await?;

    if let Some(row) = planets.next().await? {
        return Ok(construct_planet(row)?);
    }

    Err(anyhow::anyhow!("Planet wasn't found"))
}

pub async fn remove_planet(index: &str, state: AppState) -> anyhow::Result<()> {
    if check_sql(index, state) {
        anyhow::bail!("Blacklisted sql");
    }

    // 2nd check: parser checker idk as extra check if check_sql fails
    match index.split_once("-") {
        Some((num1, num2)) => {
            num1.parse::<i64>()
                .map_err(|_| anyhow::anyhow!("Blacklisted sql"))?;
            if let Err(_) = num2.parse::<i64>() {
                if num2.split_once("-").is_none() {
                    anyhow::bail!("Blacklisted sql")
                }
            }
        }
        None => anyhow::bail!("Blacklisted sql"),
    }

    let conn = establish_connection().await?;

    conn.execute("DELETE FROM planets WHERE id = ?1", (index,)).await?;

    Ok(())
}

pub async fn search_planets(input: &str, state: AppState) -> anyhow::Result<Attachment> {
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
        file_content.push_str(&format!(
            "\n{}",
            format_response(&construct_planet(row)?, false)
        ));
    }

    Ok(Attachment::from_bytes(
        "result.txt".to_owned(),
        file_content.into_bytes(),
        0,
    ))
}

pub async fn edit_planet(index: &str, input: &str, bypass: bool) -> anyhow::Result<()> {
    let conn = establish_connection().await?;

    normalize(input);

    let mut split_iter = index.split('-');
    let star_id: i64 = split_iter
        .next()
        .ok_or_else(|| anyhow::anyhow!("Invalid star id format"))?
        .parse()?;

    if split_iter.next().is_some_and(|x| x.parse::<i64>().is_err()) {
        anyhow::bail!("Invalid planet id format");
    }

    let mut is_moon = split_iter.next().is_some_and(|x| x.parse::<i64>().is_ok());

    let mut name: Option<String> = None;
    let mut radius: Option<f64> = None;
    let mut gravity: Option<f64> = None;
    let mut temperature: Option<i64> = None;
    let mut sector: Option<String> = None;
    let mut tectonics: Option<String> = None;
    let mut atmosphere: Option<String> = None;
    let mut oceans: Option<String> = None;
    let mut rings: Option<String> = None;
    let mut trees: Option<String> = None;
    let mut sub_trees: Option<String> = None;
    let mut life: Option<bool> = None;
    let mut moons: Option<i8> = None;

    let mut malachite: Option<i32> = None;
    let mut hematite: Option<f64> = None;
    let mut petroleum: Option<i32> = None;
    let mut coal: Option<i32> = None;
    let mut gummite: Option<i32> = None;
    let mut tektite: Option<i32> = None;
    let mut bauxite: Option<i32> = None;
    let mut gold: Option<i32> = None;
    let mut cerussite: Option<i32> = None;

    let mut lime: Option<bool> = None;
    let mut saltpeter: Option<bool> = None;
    let mut quartz: Option<bool> = None;
    let mut ice: Option<bool> = None;

    let mut note: Option<String> = None;

    for expr in input.split("|") {
        let expr = expr.trim();
        if expr.is_empty() {
            continue;
        }

        let (key, mut value) = expr
            .split_once("=")
            .ok_or(anyhow::anyhow!("deformed input"))?; // idk i may improve it later
        let key = key.trim().to_lowercase();
        value = value.trim();

        validate(&key, value)?;
        match key.as_str() {
            "name" => name = Some(value.to_string()),
            "radius" => radius = Some(value.parse()?),
            "gravity" => gravity = Some(value.parse()?),
            "temperature" => temperature = Some(value.parse()?),
            "sector" => sector = Some(value.to_string()),
            "tectonics" => tectonics = Some(value.to_string()),
            "atmosphere" => atmosphere = Some(value.to_string()),
            "oceans" => oceans = Some(value.to_string()),
            "rings" => rings = Some(value.to_string()),
            "trees" => trees = Some(value.to_string()),
            "sub trees" => sub_trees = Some(value.to_string()),
            "life" => life = Some(value.parse::<bool>()?),
            "moons" => moons = Some(value.parse()?),
            "malachite" => malachite = Some(value.parse()?),
            "hematite" => hematite = Some(value.parse()?),
            "petroleum" => petroleum = Some(value.parse()?),
            "coal" => coal = Some(value.parse()?),
            "gummite" => gummite = Some(value.parse()?),
            "tektite" => tektite = Some(value.parse()?),
            "bauxite" => bauxite = Some(value.parse()?),
            "gold" => gold = Some(value.parse()?),
            "cerussite" => cerussite = Some(value.parse()?),
            "lime" => lime = Some(value.parse()?),
            "saltpeter" => saltpeter = Some(value.parse()?),
            "quartz" => quartz = Some(value.parse()?),
            "ice" => ice = Some(value.parse()?),
            "note" => note = Some(value.to_string()),
            "moon" if bypass => is_moon = value.parse::<bool>()?,
            _ => continue,
        }
    }

    let (sql, query_values) = {
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
        if let Some(temperature) = temperature {
            columns.push(Planets::Temperature);
            values.push(temperature.into());
            conflict.update_column(Planets::Temperature);
        }
        if let Some(tectonics) = tectonics {
            columns.push(Planets::Tectonics);
            values.push(tectonics.into());
            conflict.update_column(Planets::Tectonics);
        }
        if let Some(sector) = sector {
            columns.push(Planets::Sector);
            values.push(sector.into());
            conflict.update_column(Planets::Sector);
        }
        if let Some(atmosphere) = atmosphere {
            columns.push(Planets::Atmosphere);
            values.push(atmosphere.into());
            conflict.update_column(Planets::Atmosphere);
        }
        if let Some(oceans) = oceans {
            columns.push(Planets::Oceans);
            values.push(oceans.into());
            conflict.update_column(Planets::Oceans);
        }
        if let Some(rings) = rings {
            columns.push(Planets::Rings);
            values.push(rings.into());
            conflict.update_column(Planets::Rings);
        }
        if let Some(trees) = trees {
            columns.push(Planets::Trees);
            values.push(trees.into());
            conflict.update_column(Planets::Trees);
        }
        if let Some(sub_trees) = sub_trees {
            columns.push(Planets::SubTrees);
            values.push(sub_trees.into());
            conflict.update_column(Planets::SubTrees);
        }
        if let Some(life) = life {
            columns.push(Planets::Life);
            values.push(life.into());
            conflict.update_column(Planets::Life);
        }
        if let Some(moons) = moons {
            columns.push(Planets::Moons);
            values.push(moons.into());
            conflict.update_column(Planets::Moons);
        }
        if let Some(malachite) = malachite {
            columns.push(Planets::Malachite);
            values.push(malachite.into());
            conflict.update_column(Planets::Malachite);
        }
        if let Some(hematite) = hematite {
            columns.push(Planets::Hematite);
            values.push(hematite.into());
            conflict.update_column(Planets::Hematite);
        }
        if let Some(petroleum) = petroleum {
            columns.push(Planets::Petroleum);
            values.push(petroleum.into());
            conflict.update_column(Planets::Petroleum);
        }
        if let Some(coal) = coal {
            columns.push(Planets::Coal);
            values.push(coal.into());
            conflict.update_column(Planets::Coal);
        }
        if let Some(gummite) = gummite {
            columns.push(Planets::Gummite);
            values.push(gummite.into());
            conflict.update_column(Planets::Gummite);
        }
        if let Some(tektite) = tektite {
            columns.push(Planets::Tektite);
            values.push(tektite.into());
            conflict.update_column(Planets::Tektite);
        }
        if let Some(bauxite) = bauxite {
            columns.push(Planets::Bauxite);
            values.push(bauxite.into());
            conflict.update_column(Planets::Bauxite);
        }
        if let Some(gold) = gold {
            columns.push(Planets::Gold);
            values.push(gold.into());
            conflict.update_column(Planets::Gold);
        }
        if let Some(cerussite) = cerussite {
            columns.push(Planets::Cerussite);
            values.push(cerussite.into());
            conflict.update_column(Planets::Cerussite);
        }

        if let Some(lime) = lime {
            columns.push(Planets::Lime);
            values.push(lime.into());
            conflict.update_column(Planets::Lime);
        }
        if let Some(saltpeter) = saltpeter {
            columns.push(Planets::Saltpeter);
            values.push(saltpeter.into());
            conflict.update_column(Planets::Saltpeter);
        }
        if let Some(quartz) = quartz {
            columns.push(Planets::Quartz);
            values.push(quartz.into());
            conflict.update_column(Planets::Quartz);
        }
        if let Some(ice) = ice {
            columns.push(Planets::Ice);
            values.push(ice.into());
            conflict.update_column(Planets::Ice);
        }

        if let Some(note) = note {
            columns.push(Planets::Note);
            values.push(note.into());
            conflict.update_column(Planets::Note);
        }

        columns.push(Planets::IsMoon);
        values.push(is_moon.into());
        conflict.update_column(Planets::IsMoon);

        sea_query::Query::insert()
            .into_table(Planets::Table)
            .columns(columns)
            .values(values)?
            .on_conflict(conflict)
            .build(sea_query::SqliteQueryBuilder)
    };

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
        sector: row.get(6)?,
        tectonics: row.get(7)?,
        atmosphere: row.get(8)?,
        oceans: row.get(9)?,
        rings: row.get(10)?,
        trees: row.get(11)?,
        sub_trees: row.get(12)?,
        life: row.get::<i64>(13)? != 0,
        is_moon: row.get::<i64>(14)? != 0,
        moons: row.get::<Option<i32>>(15)?.map(|v| v),
        malachite: row.get::<Option<i32>>(16)?.map(|v| v),
        hematite: row.get(17)?,
        petroleum: row.get::<Option<i32>>(18)?.map(|v| v),
        coal: row.get::<Option<i32>>(19)?.map(|v| v),
        gummite: row.get::<Option<i32>>(20)?.map(|v| v),
        tektite: row.get::<Option<i32>>(21)?.map(|v| v),
        bauxite: row.get::<Option<i32>>(22)?.map(|v| v),
        gold: row.get::<Option<i32>>(23)?.map(|v| v),
        cerussite: row.get::<Option<i32>>(24)?.map(|v| v),
        lime: row.get::<Option<i64>>(25)?.map(|v| v != 0),
        saltpeter: row.get::<Option<i64>>(26)?.map(|v| v != 0),
        quartz: row.get::<Option<i64>>(27)?.map(|v| v != 0),
        ice: row.get::<Option<i64>>(28)?.map(|v| v != 0),
        note: row.get(29)?,
    })
}

fn check_sql(input: &str, state: AppState) -> bool {
    state
        .configs
        .sql_blacklist
        .iter()
        .any(|sql| input.contains(sql))
}

pub fn format_response(planet: &Planet, prettier: bool) -> String {
    let mut out = String::from("");
    let mut include_space = false;

    if prettier {
        out.push_str("```");
    }

    out.push_str(&format!(
        "ID: {}\nStar Id: {}\nName: {}\nRadius: {} studs\nGravity: {:.2}g\nTemperature: {}°C",
        &planet.id,
        &planet.star_id,
        &planet.name,
        &planet.radius,
        &planet.gravity,
        &planet.temperature,
    ));
    if let Some(sector) = &planet.sector {
        out.push_str(&format!("\nSector: {}", sector));
    }
    out.push_str(&format!("\nTectonics: {}", &planet.tectonics));

    if let Some(atmosphere) = &planet.atmosphere {
        out.push_str(&format!("\nAtmosphere: {}", atmosphere))
    }
    if let Some(oceans) = &planet.oceans {
        out.push_str(&format!("\nOceans: {}", oceans))
    }
    if let Some(rings) = &planet.rings {
        out.push_str(&format!("\nRings: {}", rings))
    }
    if let Some(trees) = &planet.trees {
        out.push_str(&format!("\nTrees: {}", trees))
    }
    if let Some(sub_trees) = &planet.sub_trees {
        out.push_str(&format!("\nSub trees: {}", sub_trees))
    }
    out.push_str(&format!(
        "\nLife: {}\nIs Moon: {}",
        planet.life, planet.is_moon
    ));
    if let Some(moons) = &planet.moons {
        out.push_str(&format!("\nMoons: {}", moons))
    }
    out.push_str("\n");

    if let Some(malachite) = &planet.malachite {
        out.push_str(&format!("\nMalachite: {}", malachite));
        include_space = true;
    }
    if let Some(hematite) = &planet.hematite {
        out.push_str(&format!("\nHematite: {:.4}", hematite));
        include_space = true;
    }
    if let Some(petroleum) = &planet.petroleum {
        out.push_str(&format!("\nPetroleum: {}", petroleum));
        include_space = true;
    }
    if let Some(coal) = &planet.coal {
        out.push_str(&format!("\nCoal: {}", coal));
        include_space = true;
    }
    if let Some(gummite) = &planet.gummite {
        out.push_str(&format!("\nGummite: {}", gummite));
        include_space = true;
    }
    if let Some(tektite) = &planet.tektite {
        out.push_str(&format!("\nTektite: {}", tektite));
        include_space = true;
    }
    if let Some(bauxite) = &planet.bauxite {
        out.push_str(&format!("\nBauxite: {}", bauxite));
        include_space = true;
    }
    if let Some(gold) = &planet.gold {
        out.push_str(&format!("\nGold: {}", gold));
        include_space = true;
    }
    if let Some(cerussite) = &planet.cerussite {
        out.push_str(&format!("\nCerussite: {}", cerussite));
        include_space = true;
    }
    if include_space {
        out.push_str("\n");
        include_space = false;
    }

    if let Some(lime) = &planet.lime {
        out.push_str(&format!("\nLime: {}", lime));
        include_space = true;
    }
    if let Some(saltpeter) = &planet.saltpeter {
        out.push_str(&format!("\nSaltpeter: {}", saltpeter));
        include_space = true;
    }
    if let Some(quartz) = &planet.quartz {
        out.push_str(&format!("\nQuartz: {}", quartz));
        include_space = true;
    }
    if let Some(ice) = &planet.ice {
        out.push_str(&format!("\nIce: {}", ice));
        include_space = true;
    }

    #[allow(unused_assignments)]
    if include_space {
        out.push_str("\n");
        include_space = true;
    }

    #[allow(unused_assignments)]
    if let Some(note) = &planet.note {
        out.push_str(&format!("\nNote: {}", note));
        include_space = true;
    }
    
    if prettier {
        out.push_str("\n```");
    }

    out.push_str("\n");
    out
}

fn normalize(input: &str) {
    let _ = input.replace("&&", "and");
    let _ = input.replace("||", "or");
}

fn validate(key: &str, value: &str) -> anyhow::Result<()> {
    match key {
        "malachite" | "hematite" | "petroleum" | "coal" | "gummite" | "tektite" | "bauxite"
        | "gold" | "cerussite" => {
            let concentration = value.parse::<i8>()?;
            if concentration < 0 || concentration > 3 {
                anyhow::bail!(format!("Wrong concentration information in {}", key))
            }
        }
        "life" | "lime" | "saltpeter" | "quartz" | "ice" => {
            if value != "true" && value != "false" {
                anyhow::bail!("{} is supposed to have true/false value", key);
            }
        }
        "sector" => {
            if !CHECKS
                .get()
                .ok_or(anyhow::anyhow!("CHECKS isn't initialized"))?
                .sectors
                .contains(&value.to_string())
            {
                anyhow::bail!(format!("Sector type '{}' isn't in the game", value))
            }
        }
        "tectonics" => {
            if !CHECKS
                .get()
                .ok_or(anyhow::anyhow!("CHECKS isn't initialized"))?
                .tectonics
                .contains(&value.to_string())
            {
                anyhow::bail!(format!("Tectonic type '{}' isn't in the game", value))
            }
        }
        "atmosphere" => {
            if !CHECKS
                .get()
                .ok_or(anyhow::anyhow!("CHECKS isn't initialized"))?
                .atmospheres
                .contains(&value.to_string())
            {
                anyhow::bail!(format!("Atmospheric type '{}' isn't in the game", value))
            }
        }
        "oceans" => {
            if !CHECKS
                .get()
                .ok_or(anyhow::anyhow!("CHECKS isn't initialized"))?
                .oceans
                .contains(&value.to_string())
            {
                anyhow::bail!(format!("Oceans type '{}' isn't in the game", value))
            }
        }
        "trees" | "sub trees" => {
            if !CHECKS
                .get()
                .ok_or(anyhow::anyhow!("CHECKS isn't initialized"))?
                .trees
                .contains(&value.to_string())
            {
                anyhow::bail!(format!("Tree type '{}' isn't in the game", value))
            }
        }
        _ => {}
    }

    Ok(())
}
